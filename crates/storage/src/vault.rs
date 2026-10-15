//! Зашифроване сховище одного профілю: файл ключа `vault.key` + база SQLCipher `data.db`.
//!
//! Ключ даних (DEK, 256 біт) випадковий і ніколи не зберігається у відкритому вигляді: його обгорнуто
//! двічі: ключем з PIN (Argon2id) і ключем з коду відновлення. Додатково шлях PIN може бути прив'язаний
//! до пристрою через [`KeyWrapHook`] (Android Keystore/iOS Keychain), а шлях відновлення від нього не залежить.
//! Стирання (`erase`) знищує обидва обгорнуті ключі й ключ пристрою: дані без них нечитабельні (крипто-стирання).
//!
//! Порядок дій при відновленні й зміні ключів завжди такий: спочатку переконатися, що ключ відкриває базу,
//! і лише потім переписувати файл ключа. Невдала спроба нічого не змінює.

use crate::crypto::{
    derive_kek, normalize_recovery, open_sealed, random_bytes, recovery_code, seal, KdfParams,
    Secret32,
};
use crate::error::StorageError;
use crate::keyfile::{unwrap_dek, wrap_dek, KeyFile};
use crate::state::AppState;
use rusqlite::Connection;
use std::fmt::Write as _;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

const SCHEMA_VERSION: i64 = 1;
const PIN_MIN_CHARS: usize = 4;
const PIN_MAX_CHARS: usize = 32;
const MAX_EXPORT_ROWS: usize = 64;
const MAX_EXPORT_FIELD: usize = 8 * 1024 * 1024;
/// Файл ключа має кілька сотень байт; усе більше за це вважаємо зіпсованим, не читаючи.
const MAX_KEYFILE_BYTES: u64 = 4096;
/// Найменша можлива зашифрована база SQLCipher (одна сторінка); менше означає, що дані втрачено.
const MIN_DB_BYTES: u64 = 4096;

/// Додатковий шар захисту ключа, прив'язаний до пристрою (наприклад, Android Keystore). Реалізує оболонка,
/// по одному екземпляру на профіль (щоб стирання профілю не зачіпало інші).
pub trait KeyWrapHook {
    fn wrap(&self, data: &[u8]) -> Result<Vec<u8>, StorageError>;
    fn unwrap(&self, data: &[u8]) -> Result<Vec<u8>, StorageError>;
    /// Знищує ключ пристрою цього профілю (викликається при стиранні).
    fn destroy(&self) -> Result<(), StorageError> {
        Ok(())
    }
}

pub struct Vault {
    dir: PathBuf,
    conn: Connection,
    dek: Secret32,
    recovery_ok: bool,
}

fn validate_pin(pin: &str) -> Result<(), StorageError> {
    let n = pin.chars().count();
    if (PIN_MIN_CHARS..=PIN_MAX_CHARS).contains(&n) {
        Ok(())
    } else {
        Err(StorageError::InvalidPin)
    }
}

#[cfg(unix)]
fn private_options() -> fs::OpenOptions {
    use std::os::unix::fs::OpenOptionsExt;
    let mut o = fs::OpenOptions::new();
    o.write(true).create(true).truncate(true).mode(0o600);
    o
}

#[cfg(not(unix))]
fn private_options() -> fs::OpenOptions {
    let mut o = fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    o
}

/// Атомарний запис: тимчасовий файл, синхронізація, перейменування, синхронізація теки.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = private_options().open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        if let Ok(d) = fs::File::open(parent) {
            let _ = d.sync_all();
        }
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> Zeroizing<String> {
    let mut s = Zeroizing::new(String::with_capacity(bytes.len() * 2));
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Відкриває базу. `create`: файл можна створити (лише при створенні сховища); інакше база мусить існувати,
/// інакше це втрата даних, а не «новий профіль».
fn open_db(path: &Path, dek: &[u8; 32], create: bool) -> Result<Connection, StorageError> {
    if create {
        if !path.exists() {
            // Файл створюємо з правами 0600 ще до SQLite: журнали WAL/SHM успадковують ці права.
            drop(private_options().open(path)?);
        }
    } else {
        let len = match fs::metadata(path) {
            Ok(m) => m.len(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(StorageError::DataLost)
            }
            Err(e) => return Err(e.into()),
        };
        if len < MIN_DB_BYTES {
            return Err(StorageError::DataLost);
        }
    }
    // Ініціалізація SQLCipher (ключ, захист пам'яті) не є потокобезпечною: одночасне відкриття кількох баз у різних
    // потоках іноді давало «PRAGMA key requires a key». У застосунку виклики й так послідовні, у тестах серіалізуємо.
    static OPEN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = OPEN_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let conn = Connection::open(path)?;
    // SQLCipher: журнал помилок вимкнено (не пишемо в системний лог), захист пам'яті ввімкнено ДО ключа.
    let _ = conn.execute_batch("PRAGMA cipher_log_level = NONE;");
    conn.execute_batch("PRAGMA cipher_memory_security = ON;")?;
    let key = hex(dek);
    let pragma = Zeroizing::new(format!("PRAGMA key = \"x'{}'\";", key.as_str()));
    conn.execute_batch(&pragma)?;
    // SQLCipher перевіряє ключ при першому читанні.
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
        r.get::<_, i64>(0)
    })
    .map_err(|_| StorageError::Corrupt("базу не відкрито ключем"))?;
    conn.execute_batch("PRAGMA temp_store = MEMORY;")?;
    let mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(StorageError::Db("журнал WAL недоступний".to_string()));
    }
    conn.execute_batch(
        "PRAGMA synchronous=FULL; PRAGMA secure_delete=ON; PRAGMA foreign_keys=ON;",
    )?;
    let check: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if check != "ok" {
        return Err(StorageError::Corrupt("перевірка цілісності бази"));
    }
    migrate(&conn)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY, v TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS state(k TEXT PRIMARY KEY, v TEXT NOT NULL);",
    )?;
    let ver: Option<String> = conn
        .query_row("SELECT v FROM meta WHERE k='schema_version'", [], |r| {
            r.get(0)
        })
        .ok();
    match ver {
        None => {
            conn.execute(
                "INSERT INTO meta(k, v) VALUES('schema_version', ?1)",
                [SCHEMA_VERSION.to_string()],
            )?;
        }
        Some(v) => {
            let n: i64 = v
                .parse()
                .map_err(|_| StorageError::Corrupt("версія схеми"))?;
            if n > SCHEMA_VERSION {
                return Err(StorageError::Corrupt("база новішої версії"));
            }
        }
    }
    Ok(())
}

fn read_keyfile(dir: &Path) -> Result<KeyFile, StorageError> {
    let path = dir.join(Vault::KEY_FILE);
    let meta = match fs::metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(StorageError::NoVault),
        Err(e) => return Err(e.into()),
    };
    if meta.len() > MAX_KEYFILE_BYTES {
        return Err(StorageError::Corrupt("файл ключа завеликий"));
    }
    KeyFile::decode(&fs::read(&path)?)
}

/// Будує обгортку PIN-шляху (з хуком, якщо він є) і записує нову сіль.
fn seal_pin_path(
    kf: &mut KeyFile,
    dek: &[u8; 32],
    pin: &str,
    hook: Option<&dyn KeyWrapHook>,
) -> Result<(), StorageError> {
    kf.salt_pin = random_bytes()?;
    kf.hooked = hook.is_some();
    let kek = derive_kek(pin.as_bytes(), &kf.salt_pin, kf.params)?;
    let inner = wrap_dek(dek, &kek, &kf.aad_pin())?;
    kf.blob_pin = match hook {
        Some(h) => h.wrap(&inner)?,
        None => inner,
    };
    Ok(())
}

fn unwrap_pin_path(
    kf: &KeyFile,
    pin: &str,
    hook: Option<&dyn KeyWrapHook>,
) -> Result<Secret32, StorageError> {
    validate_pin(pin)?;
    let kek = derive_kek(pin.as_bytes(), &kf.salt_pin, kf.params)?;
    let inner = if kf.hooked {
        match hook {
            // Помилка самого хука (ключ пристрою недійсний тощо) показується як є, а не як «невірний PIN».
            Some(h) => h.unwrap(&kf.blob_pin)?,
            None => {
                return Err(StorageError::KeyHook(
                    "потрібен захист пристрою".to_string(),
                ))
            }
        }
    } else {
        kf.blob_pin.clone()
    };
    unwrap_dek(&inner, &kek, &kf.aad_pin()).map_err(|_| StorageError::WrongPin)
}

fn unwrap_recovery_path(kf: &KeyFile, code: &str) -> Result<Secret32, StorageError> {
    let canon = normalize_recovery(code).ok_or(StorageError::WrongRecoveryCode)?;
    let kek = derive_kek(canon.as_bytes(), &kf.salt_rec, kf.params)?;
    unwrap_dek(&kf.blob_rec, &kek, &kf.aad_rec()).map_err(|_| StorageError::WrongRecoveryCode)
}

/// Нова секція відновлення для даного DEK: повертає код, який показують один раз.
fn seal_recovery_path(kf: &mut KeyFile, dek: &[u8; 32]) -> Result<String, StorageError> {
    let code = recovery_code()?;
    let canon = normalize_recovery(&code).ok_or(StorageError::Crypto("код відновлення"))?;
    kf.salt_rec = random_bytes()?;
    let kek_rec = derive_kek(canon.as_bytes(), &kf.salt_rec, kf.params)?;
    kf.blob_rec = wrap_dek(dek, &kek_rec, &kf.aad_rec())?;
    kf.set_rec_tag(dek);
    Ok(code)
}

impl Vault {
    pub const KEY_FILE: &'static str = "vault.key";
    pub const DB_FILE: &'static str = "data.db";

    pub fn exists(dir: &Path) -> bool {
        dir.join(Self::KEY_FILE).exists()
    }

    /// Створює сховище. Повертає сховище й **код відновлення**, який показують один раз.
    pub fn create(
        dir: &Path,
        pin: &str,
        params: KdfParams,
        hook: Option<&dyn KeyWrapHook>,
    ) -> Result<(Vault, String), StorageError> {
        validate_pin(pin)?;
        params.validate()?;
        // У робочій збірці надто слабкі параметри Argon2id заборонені (їх можна лише в тестах).
        if !cfg!(any(test, feature = "test-params"))
            && params.m_kib < KdfParams::MIN_M_KIB_FOR_CREATE
        {
            return Err(StorageError::BadParams);
        }
        fs::create_dir_all(dir)?;
        if Self::exists(dir) {
            return Err(StorageError::AlreadyExists);
        }
        let dek: Secret32 = Zeroizing::new(random_bytes()?);
        let mut kf = KeyFile {
            params,
            hooked: false,
            salt_pin: [0; 16],
            blob_pin: Vec::new(),
            salt_rec: [0; 16],
            blob_rec: Vec::new(),
            rec_tag: [0; 32],
        };
        let code = seal_recovery_path(&mut kf, &dek)?;
        seal_pin_path(&mut kf, &dek, pin, hook)?;

        write_atomic(&dir.join(Self::KEY_FILE), &kf.encode()?)?;
        match open_db(&dir.join(Self::DB_FILE), &dek, true) {
            Ok(conn) => Ok((
                Vault {
                    dir: dir.to_path_buf(),
                    conn,
                    dek,
                    recovery_ok: true,
                },
                code,
            )),
            Err(e) => {
                let _ = Self::erase_dir(dir);
                Err(e)
            }
        }
    }

    pub fn open(
        dir: &Path,
        pin: &str,
        hook: Option<&dyn KeyWrapHook>,
    ) -> Result<Vault, StorageError> {
        let kf = read_keyfile(dir)?;
        let dek = unwrap_pin_path(&kf, pin, hook)?;
        let conn = open_db(&dir.join(Self::DB_FILE), &dek, false)?;
        let recovery_ok = kf.recovery_section_ok(&dek);
        Ok(Vault {
            dir: dir.to_path_buf(),
            conn,
            dek,
            recovery_ok,
        })
    }

    /// Чи ціла секція відновлення. `false` означає, що код відновлення може не спрацювати: варто видати новий.
    pub fn recovery_section_ok(&self) -> bool {
        self.recovery_ok
    }

    /// Відновлення доступу кодом: задає новий PIN (і, якщо є, прив'язує до поточного пристрою).
    /// Файл ключа переписується лише після того, як знайдений ключ підтвердився відкриттям бази.
    pub fn recover(
        dir: &Path,
        code: &str,
        new_pin: &str,
        hook: Option<&dyn KeyWrapHook>,
    ) -> Result<Vault, StorageError> {
        let mut kf = read_keyfile(dir)?;
        validate_pin(new_pin)?;
        let dek = unwrap_recovery_path(&kf, code)?;
        let conn = open_db(&dir.join(Self::DB_FILE), &dek, false)?;
        seal_pin_path(&mut kf, &dek, new_pin, hook)?;
        kf.set_rec_tag(&dek); // секція відновлення щойно підтвердилась розшифруванням
        write_atomic(&dir.join(Self::KEY_FILE), &kf.encode()?)?;
        Ok(Vault {
            dir: dir.to_path_buf(),
            conn,
            dek,
            recovery_ok: true,
        })
    }

    /// Змінює PIN; старий має бути вірним.
    pub fn change_pin(
        &mut self,
        old: &str,
        new: &str,
        hook: Option<&dyn KeyWrapHook>,
    ) -> Result<(), StorageError> {
        let mut kf = read_keyfile(&self.dir)?;
        let check = unwrap_pin_path(&kf, old, hook)?;
        if *check != *self.dek {
            return Err(StorageError::WrongPin);
        }
        validate_pin(new)?;
        seal_pin_path(&mut kf, &self.dek, new, hook)?;
        write_atomic(&self.dir.join(Self::KEY_FILE), &kf.encode()?)
    }

    /// Видає новий код відновлення (старий перестає діяти). Потрібен, коли `recovery_section_ok()` дорівнює `false`.
    pub fn new_recovery_code(&mut self) -> Result<String, StorageError> {
        let mut kf = read_keyfile(&self.dir)?;
        let code = seal_recovery_path(&mut kf, &self.dek)?;
        write_atomic(&self.dir.join(Self::KEY_FILE), &kf.encode()?)?;
        self.recovery_ok = true;
        Ok(code)
    }

    /// Зберігає весь стан однією транзакцією: або все, або нічого.
    pub fn save_state(&mut self, st: &AppState) -> Result<(), StorageError> {
        let rows = st.to_rows();
        let tx = self.conn.transaction()?;
        for (k, v) in &rows {
            tx.execute(
                "INSERT INTO state(k, v) VALUES(?1, ?2) ON CONFLICT(k) DO UPDATE SET v = excluded.v",
                [k, v],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_state(&self) -> Result<Option<AppState>, StorageError> {
        let mut stmt = self.conn.prepare("SELECT k, v FROM state")?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        if rows.is_empty() {
            return Ok(None);
        }
        AppState::from_rows(&rows).map(Some)
    }

    /// PIN дорослого зберігається як `m:t:p:сіль:хеш` (Argon2id із записаними параметрами) у зашифрованій базі.
    pub fn set_adult_pin(&mut self, pin: &str) -> Result<(), StorageError> {
        validate_pin(pin)?;
        let params = read_keyfile(&self.dir)?.params;
        let salt: [u8; 16] = random_bytes()?;
        let h = derive_kek(pin.as_bytes(), &salt, params)?;
        let value = format!(
            "{}:{}:{}:{}:{}",
            params.m_kib,
            params.t,
            params.p,
            hex(&salt).as_str(),
            hex(&*h).as_str()
        );
        self.conn.execute(
            "INSERT INTO meta(k, v) VALUES('adult_pin', ?1) ON CONFLICT(k) DO UPDATE SET v = excluded.v",
            [value],
        )?;
        Ok(())
    }

    pub fn has_adult_pin(&self) -> Result<bool, StorageError> {
        Ok(self.adult_pin_row()?.is_some())
    }

    fn adult_pin_row(&self) -> Result<Option<String>, StorageError> {
        match self
            .conn
            .query_row("SELECT v FROM meta WHERE k='adult_pin'", [], |r| {
                r.get::<_, String>(0)
            }) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn verify_adult_pin(&self, pin: &str) -> Result<bool, StorageError> {
        let Some(row) = self.adult_pin_row()? else {
            return Ok(false);
        };
        let parts: Vec<&str> = row.split(':').collect();
        let (params, salt_hex, hash_hex) = match parts.as_slice() {
            [m, t, p, salt, hash] => (
                KdfParams {
                    m_kib: m
                        .parse()
                        .map_err(|_| StorageError::Corrupt("PIN дорослого"))?,
                    t: t.parse()
                        .map_err(|_| StorageError::Corrupt("PIN дорослого"))?,
                    p: p.parse()
                        .map_err(|_| StorageError::Corrupt("PIN дорослого"))?,
                },
                *salt,
                *hash,
            ),
            [salt, hash] => (read_keyfile(&self.dir)?.params, *salt, *hash),
            _ => return Err(StorageError::Corrupt("PIN дорослого")),
        };
        let salt: [u8; 16] = from_hex(salt_hex)?
            .try_into()
            .map_err(|_| StorageError::Corrupt("сіль"))?;
        let want = from_hex(hash_hex)?;
        let got = derive_kek(pin.as_bytes(), &salt, params)?;
        // Порівняння без раннього виходу.
        let diff = got
            .iter()
            .zip(want.iter())
            .fold(0u8, |a, (x, y)| a | (x ^ y))
            | u8::from(want.len() != 32);
        Ok(diff == 0)
    }

    /// Зашифрований експорт стану парольною фразою (перенесення на інший пристрій, відкрите питання Q14).
    pub fn export_encrypted(
        &self,
        passphrase: &str,
        params: KdfParams,
    ) -> Result<Vec<u8>, StorageError> {
        let st = self
            .load_state()?
            .ok_or(StorageError::Corrupt("немає стану для експорту"))?;
        let mut body = Vec::new();
        for (k, v) in st.to_rows() {
            for field in [k.as_bytes(), v.as_bytes()] {
                body.extend_from_slice(
                    &u32::try_from(field.len())
                        .map_err(|_| StorageError::Decode)?
                        .to_le_bytes(),
                );
                body.extend_from_slice(field);
            }
        }
        let body = Zeroizing::new(body);
        let salt: [u8; 16] = random_bytes()?;
        let kek = derive_kek(passphrase.as_bytes(), &salt, params)?;
        let mut head = Vec::new();
        head.extend_from_slice(b"KMTX");
        head.push(1);
        head.extend_from_slice(&params.m_kib.to_le_bytes());
        head.extend_from_slice(&params.t.to_le_bytes());
        head.extend_from_slice(&params.p.to_le_bytes());
        head.extend_from_slice(&salt);
        let (nonce, ct) = seal(&kek, &body, &head)?;
        let mut out = head;
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ct);
        Ok(out)
    }

    pub fn import_encrypted(bytes: &[u8], passphrase: &str) -> Result<AppState, StorageError> {
        const HEAD: usize = 4 + 1 + 12 + 16;
        if bytes.len() < HEAD + 24 + 16 || &bytes[..4] != b"KMTX" || bytes[4] != 1 {
            return Err(StorageError::Corrupt("не файл експорту"));
        }
        let u = |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        let params = KdfParams {
            m_kib: u(5),
            t: u(9),
            p: u(13),
        };
        // Файл приходить ззовні: параметри обмежено ще до виведення ключа.
        params.validate_for_import()?;
        let salt: [u8; 16] = bytes[17..33]
            .try_into()
            .map_err(|_| StorageError::Corrupt("сіль"))?;
        let kek = derive_kek(passphrase.as_bytes(), &salt, params)?;
        let nonce: [u8; 24] = bytes[HEAD..HEAD + 24]
            .try_into()
            .map_err(|_| StorageError::Corrupt("nonce"))?;
        let body = open_sealed(&kek, &nonce, &bytes[HEAD + 24..], &bytes[..HEAD])
            .map_err(|_| StorageError::WrongPassphrase)?;
        let mut rows = Vec::new();
        let mut i = 0;
        while i < body.len() {
            let mut fields = Vec::new();
            for _ in 0..2 {
                let len_bytes: [u8; 4] = body
                    .get(i..i + 4)
                    .and_then(|s| s.try_into().ok())
                    .ok_or(StorageError::Decode)?;
                let len = usize::try_from(u32::from_le_bytes(len_bytes))
                    .map_err(|_| StorageError::Decode)?;
                if len > MAX_EXPORT_FIELD {
                    return Err(StorageError::Decode);
                }
                let field = body.get(i + 4..i + 4 + len).ok_or(StorageError::Decode)?;
                fields.push(String::from_utf8(field.to_vec()).map_err(|_| StorageError::Decode)?);
                i += 4 + len;
            }
            rows.push((fields[0].clone(), fields[1].clone()));
            if rows.len() > MAX_EXPORT_ROWS {
                return Err(StorageError::Decode);
            }
        }
        AppState::from_rows(&rows)
    }

    /// Стирає профіль: знищує обидва обгорнуті ключі, файли бази й (якщо є) ключ пристрою.
    pub fn erase(self, hook: Option<&dyn KeyWrapHook>) -> Result<(), StorageError> {
        let dir = self.dir.clone();
        drop(self);
        Self::erase_dir(&dir)?;
        if let Some(h) = hook {
            h.destroy()?;
        }
        Ok(())
    }

    /// Стирання без PIN (кнопка «стерти все»): ключ перезаписується випадковими байтами й видаляється.
    /// На флеш-пам'яті перезапис на місці не гарантує знищення старих копій, тому реальна стійкість визначається
    /// довжиною PIN і знищенням ключа пристрою.
    pub fn erase_dir(dir: &Path) -> Result<(), StorageError> {
        let key = dir.join(Self::KEY_FILE);
        if let Ok(meta) = fs::metadata(&key) {
            let len = usize::try_from(meta.len()).unwrap_or(0).clamp(64, 4096);
            let mut junk = vec![0u8; len];
            if getrandom::getrandom(&mut junk).is_ok() {
                if let Ok(mut f) = fs::OpenOptions::new().write(true).open(&key) {
                    let _ = f.write_all(&junk);
                    let _ = f.sync_all();
                }
            }
        }
        for name in [
            Self::KEY_FILE.to_string(),
            "vault.tmp".to_string(),
            Self::DB_FILE.to_string(),
            format!("{}-wal", Self::DB_FILE),
            format!("{}-shm", Self::DB_FILE),
            format!("{}-journal", Self::DB_FILE),
        ] {
            match fs::remove_file(dir.join(&name)) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
}

fn from_hex(s: &str) -> Result<Vec<u8>, StorageError> {
    if !s.is_ascii() || s.len() % 2 != 0 {
        return Err(StorageError::Corrupt("hex"));
    }
    s.as_bytes()
        .chunks(2)
        .map(|p| {
            std::str::from_utf8(p)
                .ok()
                .and_then(|t| u8::from_str_radix(t, 16).ok())
                .ok_or(StorageError::Corrupt("hex"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kmit-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn an_interrupted_write_leaves_the_previous_state_intact() {
        let dir = dir("crash");
        let (mut v, _) = Vault::create(&dir, "4821", KdfParams::fast_for_tests(), None).unwrap();
        let st = AppState::new("Оля", 1);
        v.save_state(&st).unwrap();
        // Імітація збою посеред запису: транзакцію розпочато, змінено рядок, а коміту немає (відключення світла).
        v.conn
            .execute_batch("BEGIN; UPDATE state SET v='зіпсовано' WHERE k='learner'; INSERT INTO state(k,v) VALUES('x','y');")
            .unwrap();
        drop(v);
        let v = Vault::open(&dir, "4821", None).unwrap();
        let back = v.load_state().unwrap().unwrap();
        assert_eq!(back.profile.nickname, "Оля");
        assert_eq!(back.learner.encode(), st.learner.encode());
        drop(v);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sqlcipher_pragmas_follow_the_spec() {
        let dir = dir("pragmas");
        let (v, _) = Vault::create(&dir, "4821", KdfParams::fast_for_tests(), None).unwrap();
        let get = |p: &str| v.conn.query_row(p, [], |r| r.get::<_, i64>(0)).unwrap();
        assert_eq!(get("PRAGMA temp_store"), 2, "тимчасові дані лише в пам'яті");
        let sec: String = v
            .conn
            .query_row("PRAGMA cipher_memory_security", [], |r| r.get(0))
            .unwrap();
        assert!(
            sec == "1" || sec.eq_ignore_ascii_case("on"),
            "захист пам'яті SQLCipher: {sec}"
        );
        assert_eq!(get("PRAGMA synchronous"), 2, "FULL");
        assert_eq!(get("PRAGMA secure_delete"), 1);
        let mode: String = v
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        drop(v);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn hex_parsing_rejects_non_ascii_instead_of_panicking() {
        assert!(from_hex("aéa").is_err());
        assert!(from_hex("é").is_err());
        assert!(from_hex("abc").is_err());
        assert_eq!(from_hex("00ff").unwrap(), vec![0, 255]);
    }
}

mod common;

#[cfg(unix)]
use common::contains;
use common::TestDir;
use std::cell::Cell;
use storage::crypto::KdfParams;
use storage::state::AppState;
use storage::vault::{KeyWrapHook, Vault};
use storage::StorageError;

const FAST: KdfParams = KdfParams::fast_for_tests();

fn state() -> AppState {
    AppState::new("Мирослава", 2)
}

fn create_saved(d: &TestDir) -> String {
    let (mut v, code) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.save_state(&state()).unwrap();
    code
}

#[test]
fn a_failed_recovery_never_touches_the_key_file() {
    let d = TestDir::new();
    let code = create_saved(&d);
    let key_before = std::fs::read(d.join(Vault::KEY_FILE)).unwrap();
    // База зіпсована (той самий розмір): відновлення за кодом не може пройти й не повинно змінювати PIN.
    let db = d.join(Vault::DB_FILE);
    let len = std::fs::metadata(&db).unwrap().len();
    std::fs::write(&db, vec![0xAB; usize::try_from(len).unwrap()]).unwrap();
    assert!(Vault::recover(d.path(), &code, "9034", None).is_err());
    assert_eq!(
        std::fs::read(d.join(Vault::KEY_FILE)).unwrap(),
        key_before,
        "файл ключа не змінено"
    );
}

#[test]
fn a_missing_or_truncated_database_is_data_loss_not_a_fresh_profile() {
    let d = TestDir::new();
    let code = create_saved(&d);
    let db = d.join(Vault::DB_FILE);
    let good = std::fs::read(&db).unwrap();

    std::fs::remove_file(&db).unwrap();
    assert_eq!(
        Vault::open(d.path(), "4821", None).err(),
        Some(StorageError::DataLost)
    );
    assert_eq!(
        Vault::recover(d.path(), &code, "9034", None).err(),
        Some(StorageError::DataLost)
    );
    assert!(!db.exists(), "open не створює порожню базу");

    for len in [0usize, 1, 99, 100, 2000] {
        std::fs::write(&db, &good[..len.min(good.len())]).unwrap();
        let r = Vault::open(d.path(), "4821", None);
        assert!(
            matches!(
                r.err(),
                Some(StorageError::DataLost | StorageError::Corrupt(_))
            ),
            "базу обрізано до {len} байт"
        );
    }
    std::fs::write(&db, &good).unwrap();
    assert!(
        Vault::open(d.path(), "4821", None).is_ok(),
        "відновлена копія відкривається"
    );
}

struct FailingHook;
impl KeyWrapHook for FailingHook {
    fn wrap(&self, d: &[u8]) -> Result<Vec<u8>, StorageError> {
        Ok(d.to_vec())
    }
    fn unwrap(&self, _: &[u8]) -> Result<Vec<u8>, StorageError> {
        Err(StorageError::KeyHook("ключ пристрою недійсний".to_string()))
    }
}

#[test]
fn a_hook_failure_is_reported_as_such_and_never_as_a_wrong_pin() {
    let d = TestDir::new();
    let (v, _) = Vault::create(d.path(), "4821", FAST, Some(&FailingHook)).unwrap();
    drop(v);
    match Vault::open(d.path(), "4821", Some(&FailingHook)).err() {
        Some(StorageError::KeyHook(m)) => assert!(m.contains("недійсний")),
        other => panic!("очікувалась помилка захисту пристрою, отримано {other:?}"),
    }
}

struct CountingHook<'a>(&'a Cell<u32>);
impl KeyWrapHook for CountingHook<'_> {
    fn wrap(&self, d: &[u8]) -> Result<Vec<u8>, StorageError> {
        Ok(d.to_vec())
    }
    fn unwrap(&self, d: &[u8]) -> Result<Vec<u8>, StorageError> {
        Ok(d.to_vec())
    }
    fn destroy(&self) -> Result<(), StorageError> {
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}

#[test]
fn erasing_also_destroys_the_device_bound_key() {
    let d = TestDir::new();
    let n = Cell::new(0);
    let hook = CountingHook(&n);
    let (v, _) = Vault::create(d.path(), "4821", FAST, Some(&hook)).unwrap();
    v.erase(Some(&hook)).unwrap();
    assert_eq!(n.get(), 1, "ключ у сховищі пристрою знищено");
    assert!(!Vault::exists(d.path()));
}

#[test]
fn the_recovery_section_has_its_own_health_check_and_can_be_renewed() {
    let d = TestDir::new();
    let (mut v, code) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.save_state(&state()).unwrap();
    assert!(v.recovery_section_ok());
    drop(v);
    let path = d.join(Vault::KEY_FILE);
    let good = std::fs::read(&path).unwrap();

    // Зіпсована лише секція відновлення (хвіст файлу): за PIN доступ є, але про проблему повідомлено.
    let mut bad = good.clone();
    let in_rec_blob = bad.len() - 32 - 5;
    bad[in_rec_blob] ^= 1;
    std::fs::write(&path, &bad).unwrap();
    let mut v = Vault::open(d.path(), "4821", None).unwrap();
    assert!(
        !v.recovery_section_ok(),
        "пошкодження коду відновлення видно відразу, а не в день, коли PIN забуто"
    );
    assert!(Vault::recover(d.path(), &code, "9034", None).is_err());

    // Оновлення коду відновлення лікує секцію; старий код стає недійсним.
    let new_code = v.new_recovery_code().unwrap();
    assert!(v.recovery_section_ok());
    assert_ne!(new_code, code);
    drop(v);
    assert!(
        Vault::recover(d.path(), &code, "9034", None).is_err(),
        "старий код більше не діє"
    );
    assert!(Vault::recover(d.path(), &new_code, "9034", None).is_ok());
}

#[test]
fn every_flipped_byte_either_blocks_the_pin_or_is_reported_as_recovery_damage() {
    let d = TestDir::new();
    let _code = create_saved(&d);
    let path = d.join(Vault::KEY_FILE);
    let good = std::fs::read(&path).unwrap();
    for i in 0..good.len() {
        let mut bad = good.clone();
        bad[i] ^= 0x40;
        std::fs::write(&path, &bad).unwrap();
        match Vault::open(d.path(), "4821", None) {
            Err(_) => {}
            Ok(v) => {
                assert!(
                    !v.recovery_section_ok(),
                    "байт {i}: вхід вдався, а пошкодження не помічено"
                );
            }
        }
        std::fs::write(&path, &bad).unwrap();
    }
}

#[test]
fn spliced_key_files_cannot_corrupt_another_profile() {
    let (a, b) = (TestDir::new(), TestDir::new());
    let code_a = create_saved(&a);
    let code_b = create_saved(&b);
    // Секцію відновлення профілю B вставлено у файл ключа профілю A.
    let ka = std::fs::read(a.join(Vault::KEY_FILE)).unwrap();
    let kb = std::fs::read(b.join(Vault::KEY_FILE)).unwrap();
    let split = 18 + 16 + 2 + (24 + 32 + 16); // заголовок + сіль_pin + довжина + blob_pin
    let mut spliced = ka[..split].to_vec();
    spliced.extend_from_slice(&kb[split..]);
    std::fs::write(a.join(Vault::KEY_FILE), &spliced).unwrap();
    let v = Vault::open(a.path(), "4821", None).unwrap();
    assert!(!v.recovery_section_ok(), "підміну виявлено");
    drop(v);
    let before = std::fs::read(a.join(Vault::KEY_FILE)).unwrap();
    assert!(Vault::recover(a.path(), &code_b, "2222", None).is_err());
    assert_eq!(
        std::fs::read(a.join(Vault::KEY_FILE)).unwrap(),
        before,
        "невдале відновлення нічого не змінило"
    );
    assert!(Vault::open(a.path(), "4821", None).is_ok());
    let _ = code_a;
}

#[test]
fn an_oversized_key_file_is_refused_without_reading_it_all() {
    let d = TestDir::new();
    create_saved(&d);
    std::fs::write(d.join(Vault::KEY_FILE), vec![0u8; 5 * 1024 * 1024]).unwrap();
    assert!(matches!(
        Vault::open(d.path(), "4821", None).err(),
        Some(StorageError::Corrupt(_))
    ));
}

#[test]
fn import_parameters_from_an_outside_file_are_capped_and_a_wrong_passphrase_is_named() {
    let d = TestDir::new();
    let (mut v, _) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.save_state(&state()).unwrap();
    let blob = v.export_encrypted("фраза", FAST).unwrap();
    assert_eq!(
        Vault::import_encrypted(&blob, "інша").err(),
        Some(StorageError::WrongPassphrase)
    );
    // Файл з «величезними» параметрами Argon2id відхиляється ще до виведення ключа.
    let mut heavy = blob.clone();
    heavy[5..9].copy_from_slice(&(250 * 1024u32).to_le_bytes());
    assert_eq!(
        Vault::import_encrypted(&heavy, "фраза").err(),
        Some(StorageError::BadParams)
    );
}

#[cfg(unix)]
#[test]
fn the_database_side_files_are_private_too() {
    use std::os::unix::fs::PermissionsExt;
    let d = TestDir::new();
    let (mut v, _) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.save_state(&state()).unwrap();
    for name in ["data.db", "data.db-wal", "data.db-shm"] {
        let p = d.join(name);
        if p.exists() {
            let mode = std::fs::metadata(&p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "{name}: {mode:o}");
        }
    }
    drop(v);
    assert!(!contains(
        &std::fs::read(d.join("data.db")).unwrap(),
        "Мирослава".as_bytes()
    ));
}

#[test]
fn the_adult_pin_survives_and_non_ascii_junk_cannot_panic_the_parser() {
    let d = TestDir::new();
    let (mut v, _) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.set_adult_pin("2468").unwrap();
    assert!(v.verify_adult_pin("2468").unwrap());
    assert!(!v.verify_adult_pin("2469").unwrap());
    assert!(
        v.verify_adult_pin("ё я").is_ok(),
        "будь-який ввід перевіряється без паніки"
    );
}

mod common;

use common::{all_bytes, contains, TestDir};
use storage::crypto::KdfParams;
use storage::state::AppState;
use storage::vault::{KeyWrapHook, Vault};
use storage::StorageError;
use tutor_core::bkt::BktParams;
use tutor_core::config::Config;

const FAST: KdfParams = KdfParams::fast_for_tests();

fn state() -> AppState {
    let (p, c) = (BktParams::numeric_default(), Config::default());
    let mut st = AppState::new("Мирослава", 2);
    st.learner.begin_day(3);
    st.learner.record_task(&p, &c, "F-ADD-SAME", true, 0, None);
    st.learner
        .record_ask_adult("F-ADD-DIFF", "СекретнаУмова 7/13 + 1/13");
    st.album.award_mastery("N-ADD-MULTI");
    st
}

/// Хук, що «прив'язує» ключ до пристрою: XOR із довільним байтом (для перевірки схеми, не для захисту).
struct XorHook(u8);
impl KeyWrapHook for XorHook {
    fn wrap(&self, data: &[u8]) -> Result<Vec<u8>, StorageError> {
        Ok(data.iter().map(|b| b ^ self.0).collect())
    }
    fn unwrap(&self, data: &[u8]) -> Result<Vec<u8>, StorageError> {
        Ok(data.iter().map(|b| b ^ self.0).collect())
    }
}

#[test]
fn create_save_reopen_and_load_the_same_state() {
    let d = TestDir::new();
    let (mut v, code) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    assert_eq!(code.len(), 23);
    assert!(v.load_state().unwrap().is_none(), "новий сховище порожнє");
    v.save_state(&state()).unwrap();
    drop(v);
    let v = Vault::open(d.path(), "4821", None).unwrap();
    let back = v.load_state().unwrap().unwrap();
    assert_eq!(back.profile.nickname, "Мирослава");
    assert_eq!(back.learner.encode(), state().learner.encode());
    assert_eq!(back.album.keys(), state().album.keys());
}

#[test]
fn saving_twice_replaces_rather_than_duplicates() {
    let d = TestDir::new();
    let (mut v, _) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    let mut st = state();
    v.save_state(&st).unwrap();
    st.profile.nickname = "Інша".to_string();
    v.save_state(&st).unwrap();
    assert_eq!(v.load_state().unwrap().unwrap().profile.nickname, "Інша");
}

#[test]
fn a_wrong_pin_is_rejected_and_nothing_is_changed() {
    let d = TestDir::new();
    let (mut v, _) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.save_state(&state()).unwrap();
    drop(v);
    let before = all_bytes(d.path()).len();
    assert_eq!(
        Vault::open(d.path(), "0000", None).err(),
        Some(StorageError::WrongPin)
    );
    assert_eq!(
        Vault::open(d.path(), "", None).err(),
        Some(StorageError::InvalidPin)
    );
    assert_eq!(all_bytes(d.path()).len(), before);
    assert!(Vault::open(d.path(), "4821", None).is_ok());
}

#[test]
fn a_pin_must_have_a_sane_length() {
    let d = TestDir::new();
    assert_eq!(
        Vault::create(d.path(), "12", FAST, None).err(),
        Some(StorageError::InvalidPin)
    );
    assert_eq!(
        Vault::create(d.path(), &"9".repeat(200), FAST, None).err(),
        Some(StorageError::InvalidPin)
    );
    assert!(!Vault::exists(d.path()));
}

#[test]
fn creating_over_an_existing_vault_is_refused() {
    let d = TestDir::new();
    Vault::create(d.path(), "4821", FAST, None).unwrap();
    assert_eq!(
        Vault::create(d.path(), "1111", FAST, None).err(),
        Some(StorageError::AlreadyExists)
    );
    assert!(
        Vault::open(d.path(), "4821", None).is_ok(),
        "перший сховище не зіпсовано"
    );
}

#[test]
fn nothing_readable_is_left_on_disk() {
    let d = TestDir::new();
    let (mut v, code) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.save_state(&state()).unwrap();
    drop(v);
    let bytes = all_bytes(d.path());
    for secret in [
        "Мирослава".as_bytes(),
        "СекретнаУмова".as_bytes(),
        b"4821",
        b"N-ADD-MULTI",
        b"SQLite format 3",
        code.replace('-', "").as_bytes(),
    ] {
        assert!(
            !contains(&bytes, secret),
            "у файлах видно {:?}",
            String::from_utf8_lossy(secret)
        );
    }
}

#[test]
fn the_database_cannot_be_read_without_the_key() {
    let d = TestDir::new();
    let (mut v, _) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.save_state(&state()).unwrap();
    drop(v);
    let plain = rusqlite::Connection::open(d.join(Vault::DB_FILE)).unwrap();
    let r = plain.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
        r.get::<_, i64>(0)
    });
    assert!(r.is_err(), "база без ключа не читається");
}

#[test]
fn recovery_code_restores_access_with_a_new_pin() {
    let d = TestDir::new();
    let (mut v, code) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.save_state(&state()).unwrap();
    drop(v);
    let v = Vault::recover(d.path(), &code.to_lowercase(), "9034", None).unwrap();
    assert_eq!(
        v.load_state().unwrap().unwrap().profile.nickname,
        "Мирослава"
    );
    drop(v);
    assert_eq!(
        Vault::open(d.path(), "4821", None).err(),
        Some(StorageError::WrongPin),
        "старий PIN більше не діє"
    );
    assert!(Vault::open(d.path(), "9034", None).is_ok());
    assert!(
        Vault::recover(d.path(), &code, "5555", None).is_ok(),
        "код відновлення лишається чинним"
    );
}

#[test]
fn a_wrong_or_malformed_recovery_code_is_rejected() {
    let d = TestDir::new();
    let (v, code) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    drop(v);
    assert_eq!(
        Vault::recover(d.path(), "AAAAA-AAAAA-AAAAA-AAAAA", "9034", None).err(),
        Some(StorageError::WrongRecoveryCode)
    );
    assert_eq!(
        Vault::recover(d.path(), "short", "9034", None).err(),
        Some(StorageError::WrongRecoveryCode)
    );
    assert_eq!(
        Vault::recover(d.path(), &code, "1", None).err(),
        Some(StorageError::InvalidPin)
    );
    assert!(
        Vault::open(d.path(), "4821", None).is_ok(),
        "невдалі спроби нічого не змінили"
    );
}

#[test]
fn changing_the_pin_needs_the_old_one() {
    let d = TestDir::new();
    let (mut v, _) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.save_state(&state()).unwrap();
    assert_eq!(
        v.change_pin("0000", "7777", None).err(),
        Some(StorageError::WrongPin)
    );
    v.change_pin("4821", "7777", None).unwrap();
    drop(v);
    assert!(Vault::open(d.path(), "4821", None).is_err());
    let v = Vault::open(d.path(), "7777", None).unwrap();
    assert!(v.load_state().unwrap().is_some());
}

#[test]
fn erase_destroys_everything_and_the_data_is_unrecoverable() {
    let d = TestDir::new();
    let (mut v, code) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.save_state(&state()).unwrap();
    let db_copy = std::fs::read(d.join(Vault::DB_FILE)).unwrap();
    v.erase(None).unwrap();
    assert!(!Vault::exists(d.path()));
    assert!(
        std::fs::read_dir(d.path()).unwrap().next().is_none(),
        "у теці нічого не лишилось"
    );
    assert_eq!(
        Vault::open(d.path(), "4821", None).err(),
        Some(StorageError::NoVault)
    );
    assert_eq!(
        Vault::recover(d.path(), &code, "1234", None).err(),
        Some(StorageError::NoVault)
    );
    // Навіть із копією бази, зробленою до стирання, ключа вже немає: нове сховище в тій самій теці її не відкриє.
    let (v2, _) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    drop(v2);
    std::fs::write(d.join(Vault::DB_FILE), db_copy).unwrap();
    assert!(Vault::open(d.path(), "4821", None).is_err());
}

#[test]
fn erase_without_a_pin_works_on_a_directory() {
    let d = TestDir::new();
    let (v, _) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    drop(v);
    Vault::erase_dir(d.path()).unwrap();
    assert!(!Vault::exists(d.path()));
    Vault::erase_dir(d.path()).unwrap(); // повторно: без помилки
}

#[test]
fn a_tampered_key_file_never_gives_access_to_wrong_data_and_cannot_exhaust_memory() {
    let d = TestDir::new();
    let (v, code) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    drop(v);
    let path = d.join(Vault::KEY_FILE);
    let good = std::fs::read(&path).unwrap();

    // Зіпсований будь-який байт: або вхід за PIN не вдається, або (якщо зіпсовано лише область
    // відновлення) відновлення за кодом уже не вдається. Паніки немає.
    for i in 0..good.len() {
        let mut bad = good.clone();
        bad[i] ^= 0x40;
        std::fs::write(&path, &bad).unwrap();
        // Будь-який зіпсований байт: або вхід за PIN не вдається, або пошкодження видно (`recovery_section_ok` = false).
        let flagged = match Vault::open(d.path(), "4821", None) {
            Ok(v) => !v.recovery_section_ok(),
            Err(_) => true,
        };
        assert!(
            flagged,
            "байт {i} змінено, а вхід вдався без жодного попередження"
        );
        // Спроба відновлення нічого не ламає й не панікує (може вдатися, якщо пошкоджено лише тег).
        let _ = Vault::recover(d.path(), &code, "4821", None);
        std::fs::write(&path, &bad).unwrap();
    }
    // Обрізаний файл.
    std::fs::write(&path, &good).unwrap();
    for len in [0, 3, 10, good.len() - 1] {
        std::fs::write(&path, &good[..len]).unwrap();
        assert!(Vault::open(d.path(), "4821", None).is_err());
        assert!(Vault::recover(d.path(), &code, "4821", None).is_err());
    }
    // Підроблені параметри Argon2id (величезна пам'ять) відхиляються до виведення ключа.
    let mut huge = good.clone();
    huge[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
    std::fs::write(&path, &huge).unwrap();
    assert!(matches!(
        Vault::open(d.path(), "4821", None).err(),
        Some(StorageError::BadParams | StorageError::Corrupt(_))
    ));
    std::fs::write(&path, &good).unwrap();
    assert!(Vault::open(d.path(), "4821", None).is_ok());
}

#[test]
fn a_hardware_hook_binds_the_pin_path_but_not_the_recovery_path() {
    let d = TestDir::new();
    let hook = XorHook(0x5A);
    let (mut v, code) = Vault::create(d.path(), "4821", FAST, Some(&hook)).unwrap();
    v.save_state(&state()).unwrap();
    drop(v);
    assert!(Vault::open(d.path(), "4821", Some(&hook)).is_ok());
    assert!(
        matches!(
            Vault::open(d.path(), "4821", None).err(),
            Some(StorageError::KeyHook(_))
        ),
        "без хука не відкрити"
    );
    assert!(
        Vault::open(d.path(), "4821", Some(&XorHook(0x11))).is_err(),
        "інший ключ пристрою"
    );
    // Пристрій змінився (інший хук), але є код відновлення: доступ повертається, ключ знову прив'язано до нового пристрою.
    let new_hook = XorHook(0x11);
    let v = Vault::recover(d.path(), &code, "4821", Some(&new_hook)).unwrap();
    assert!(v.load_state().unwrap().is_some());
    drop(v);
    assert!(Vault::open(d.path(), "4821", Some(&new_hook)).is_ok());
    assert!(Vault::open(d.path(), "4821", Some(&hook)).is_err());
}

#[test]
fn the_adult_pin_is_stored_hashed_and_verified() {
    let d = TestDir::new();
    let (mut v, _) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    assert!(!v.has_adult_pin().unwrap());
    assert!(
        !v.verify_adult_pin("2468").unwrap(),
        "пін дорослого ще не задано"
    );
    v.set_adult_pin("2468").unwrap();
    assert!(v.has_adult_pin().unwrap());
    assert!(v.verify_adult_pin("2468").unwrap());
    assert!(!v.verify_adult_pin("2469").unwrap());
    assert_eq!(v.set_adult_pin("1").err(), Some(StorageError::InvalidPin));
    drop(v);
    let bytes = all_bytes(d.path());
    assert!(!contains(&bytes, b"2468"));
    let v = Vault::open(d.path(), "4821", None).unwrap();
    assert!(
        v.verify_adult_pin("2468").unwrap(),
        "переживає перевідкриття"
    );
}

#[test]
fn encrypted_export_and_import_round_trip_and_resist_tampering() {
    let d = TestDir::new();
    let (mut v, _) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.save_state(&state()).unwrap();
    let blob = v.export_encrypted("секретна фраза", FAST).unwrap();
    assert!(!contains(&blob, "Мирослава".as_bytes()));
    let st = Vault::import_encrypted(&blob, "секретна фраза").unwrap();
    assert_eq!(st.profile.nickname, "Мирослава");
    assert_eq!(st.learner.encode(), state().learner.encode());
    assert!(Vault::import_encrypted(&blob, "інша фраза").is_err());
    for i in (0..blob.len()).step_by(7) {
        let mut bad = blob.clone();
        bad[i] ^= 1;
        assert!(
            Vault::import_encrypted(&bad, "секретна фраза").is_err(),
            "байт {i}"
        );
    }
    assert!(Vault::import_encrypted(&blob[..blob.len() / 2], "секретна фраза").is_err());
    assert!(Vault::import_encrypted(&[], "x").is_err());
}

#[cfg(unix)]
#[test]
fn files_are_private_to_the_owner() {
    use std::os::unix::fs::PermissionsExt;
    let d = TestDir::new();
    let (mut v, _) = Vault::create(d.path(), "4821", FAST, None).unwrap();
    v.save_state(&state()).unwrap();
    drop(v);
    let mode = std::fs::metadata(d.join(Vault::KEY_FILE))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "файл ключа: {mode:o}");
}

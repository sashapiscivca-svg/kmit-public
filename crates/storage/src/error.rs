//! Помилки сховища. Повідомлення без персональних даних і без секретів.

use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum StorageError {
    /// Помилка файлової системи (текст без шляхів).
    Io(String),
    /// Помилка бази даних.
    Db(String),
    /// PIN не підійшов (або файл ключа підмінено).
    WrongPin,
    /// PIN поза допустимою довжиною (4-32 символи).
    InvalidPin,
    /// Код відновлення не підійшов.
    WrongRecoveryCode,
    /// Парольна фраза файлу експорту не підійшла.
    WrongPassphrase,
    /// Файл ключа є, а бази немає або вона обрізана: дані втрачено (а не «новий профіль»).
    DataLost,
    /// У теці немає сховища.
    NoVault,
    /// У теці вже є сховище.
    AlreadyExists,
    /// Файл ключа чи база зіпсовані.
    Corrupt(&'static str),
    /// Параметри виведення ключа поза безпечними межами.
    BadParams,
    /// Зайнято всі слоти профілів.
    TooManyProfiles,
    /// Слот профілю невірний чи порожній.
    NoSuchProfile,
    /// Помилка криптографії.
    Crypto(&'static str),
    /// Зіпсований текст стану.
    Decode,
    /// Забагато хибних спроб: зачекати стільки секунд.
    Locked(u32),
    /// Профіль створено новішою версією застосунку: його не відкриваємо, щоб не пошкодити.
    NewerFormat,
    /// Помилка апаратного обгортання ключа (Android Keystore).
    KeyHook(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Io(e) => write!(f, "помилка файлу: {e}"),
            StorageError::Db(e) => write!(f, "помилка бази: {e}"),
            StorageError::WrongPin => write!(f, "невірний PIN"),
            StorageError::InvalidPin => write!(f, "PIN має містити від 4 до 32 символів"),
            StorageError::WrongRecoveryCode => write!(f, "невірний код відновлення"),
            StorageError::WrongPassphrase => write!(f, "невірна парольна фраза"),
            StorageError::DataLost => write!(f, "дані профілю втрачено"),
            StorageError::NoVault => write!(f, "сховища немає"),
            StorageError::AlreadyExists => write!(f, "сховище вже існує"),
            StorageError::Corrupt(w) => write!(f, "сховище пошкоджене: {w}"),
            StorageError::BadParams => write!(f, "небезпечні параметри ключа"),
            StorageError::TooManyProfiles => write!(f, "забагато профілів"),
            StorageError::NoSuchProfile => write!(f, "такого профілю немає"),
            StorageError::Crypto(w) => write!(f, "помилка шифрування: {w}"),
            StorageError::Locked(s) => write!(f, "Забагато спроб. Зачекай {s} с"),
            StorageError::NewerFormat => write!(f, "профіль створено новішою версією Kmit"),
            StorageError::Decode => write!(f, "зіпсований стан"),
            StorageError::KeyHook(e) => write!(f, "помилка захисту ключа: {e}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::Io(format!("{:?}", e.kind()))
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self {
        StorageError::Db(e.to_string())
    }
}

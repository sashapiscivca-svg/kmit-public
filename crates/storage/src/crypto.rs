//! Примітиви: Argon2id для ключа з PIN, XChaCha20-Poly1305 для обгортання, випадкові байти, код відновлення.
//! Секрети живуть у `Zeroizing`, тож зникають із пам'яті після використання.

use crate::error::StorageError;
use argon2::{Algorithm, Argon2, Params, Version};
use blake2::digest::consts::U32;
use blake2::digest::Mac as _;
use blake2::Blake2bMac;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

pub type Secret32 = Zeroizing<[u8; 32]>;

/// Параметри Argon2id. Межі перевіряються, щоб підмінений файл не міг вичерпати пам'ять пристрою.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KdfParams {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

const MAX_M_KIB: u32 = 256 * 1024;
const MAX_T: u32 = 10;
const MAX_P: u32 = 8;

impl KdfParams {
    /// Робочі значення: 64 МіБ, 3 проходи, 1 потік (RFC 9106 і OWASP: не нижче мінімуму; калібруються на пристрої, Q24).
    pub const fn production() -> Self {
        KdfParams {
            m_kib: 64 * 1024,
            t: 3,
            p: 1,
        }
    }

    /// Дуже швидкі значення лише для тестів (у робочій збірці цієї функції немає).
    #[cfg(any(test, feature = "test-params"))]
    pub const fn fast_for_tests() -> Self {
        KdfParams {
            m_kib: 64,
            t: 1,
            p: 1,
        }
    }

    /// Межі для параметрів із зовнішнього файлу (імпорт): ближче до робочих, ніж загальні.
    pub fn validate_for_import(&self) -> Result<(), StorageError> {
        self.validate()?;
        if self.m_kib > 128 * 1024 || self.t > 4 || self.p > 2 {
            return Err(StorageError::BadParams);
        }
        Ok(())
    }

    /// Мінімум для створення сховища в робочій збірці (OWASP: 19 МіБ).
    pub const MIN_M_KIB_FOR_CREATE: u32 = 19 * 1024;

    pub fn validate(&self) -> Result<(), StorageError> {
        let ok = (8..=MAX_M_KIB).contains(&self.m_kib)
            && (1..=MAX_T).contains(&self.t)
            && (1..=MAX_P).contains(&self.p)
            && self.m_kib >= 8 * self.p;
        if ok {
            Ok(())
        } else {
            Err(StorageError::BadParams)
        }
    }
}

/// Ключ-обгортка з PIN або коду відновлення.
pub fn derive_kek(
    secret: &[u8],
    salt: &[u8; 16],
    params: KdfParams,
) -> Result<Secret32, StorageError> {
    params.validate()?;
    let p = Params::new(params.m_kib, params.t, params.p, Some(32))
        .map_err(|_| StorageError::BadParams)?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, p);
    let mut out = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(secret, salt, &mut *out)
        .map_err(|_| StorageError::Crypto("argon2"))?;
    Ok(out)
}

pub fn random_bytes<const N: usize>() -> Result<[u8; N], StorageError> {
    let mut b = [0u8; N];
    getrandom::getrandom(&mut b).map_err(|_| StorageError::Crypto("випадкові числа"))?;
    Ok(b)
}

/// Шифрує `plaintext` із прив'язкою до `aad`. Повертає (nonce, шифротекст із тегом).
pub fn seal(
    key: &[u8; 32],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<([u8; 24], Vec<u8>), StorageError> {
    let nonce: [u8; 24] = random_bytes()?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let ct = cipher
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| StorageError::Crypto("шифрування"))?;
    Ok((nonce, ct))
}

/// Розшифровує й перевіряє цілісність; будь-яка зміна даних, ключа чи `aad` дає помилку.
pub fn open_sealed(
    key: &[u8; 32],
    nonce: &[u8; 24],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Zeroizing<Vec<u8>>, StorageError> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| StorageError::Crypto("розшифрування"))
}

/// Алфавіт Crockford без `I`, `L`, `O`, `U` (щоб символи не плутались).
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Код відновлення: 20 символів (100 біт), у чотирьох групах по п'ять.
pub fn recovery_code() -> Result<String, StorageError> {
    let raw: [u8; 20] = random_bytes()?;
    let mut out = String::with_capacity(23);
    for (i, b) in raw.iter().enumerate() {
        if i > 0 && i % 5 == 0 {
            out.push('-');
        }
        out.push(ALPHABET[usize::from(b & 31)] as char);
    }
    Ok(out)
}

/// Зводить введений код до канонічного вигляду; `None`, якщо довжина чи символи невірні.
pub fn normalize_recovery(input: &str) -> Option<String> {
    let mut out = String::with_capacity(20);
    for c in input.chars() {
        if c == '-' || c.is_whitespace() {
            continue;
        }
        let c = match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        };
        if !ALPHABET.contains(&(c as u8)) || !c.is_ascii() {
            return None;
        }
        out.push(c);
    }
    (out.len() == 20).then_some(out)
}

/// Ключований MAC (BLAKE2b-256) для перевірки цілісності: `key` не використовується ні для чого іншого.
pub fn mac(key: &[u8; 32], data: &[u8]) -> [u8; 32] {
    let mut m = <Blake2bMac<U32> as blake2::digest::Mac>::new_from_slice(key)
        .expect("ключ 32 байти допустимий");
    m.update(data);
    m.finalize().into_bytes().into()
}

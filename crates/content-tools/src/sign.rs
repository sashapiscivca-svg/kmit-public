//! Ключі й підпис пакетів (Ed25519). Приватний ключ це 32 байти (hex); зберігається поза репозиторієм.

use ed25519_dalek::{Signer, SigningKey};

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn unhex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Нова пара ключів: (приватний seed hex, відкритий ключ hex).
pub fn keygen() -> (String, String) {
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed).expect("випадкові числа");
    let key = SigningKey::from_bytes(&seed);
    (hex(&seed), hex(key.verifying_key().as_bytes()))
}

pub fn public_from_private(seed_hex: &str) -> Option<[u8; 32]> {
    let seed: [u8; 32] = unhex(seed_hex)?.try_into().ok()?;
    Some(*SigningKey::from_bytes(&seed).verifying_key().as_bytes())
}

pub fn sign(seed_hex: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    let seed: [u8; 32] = unhex(seed_hex)?.try_into().ok()?;
    Some(
        SigningKey::from_bytes(&seed)
            .sign(bytes)
            .to_bytes()
            .to_vec(),
    )
}

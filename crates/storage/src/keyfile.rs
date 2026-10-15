//! Файл ключа `vault.key`: параметри Argon2id і два обгорнуті екземпляри ключа даних (DEK):
//! один відкривається PIN (і, якщо є, хуком пристрою), другий кодом відновлення.
//!
//! Розкладка (малий порядок байтів):
//! `KMTV | версія | hooked | m_kib | t | p | сіль_pin[16] | довжина_pin u16 | blob_pin | сіль_rec[16] | довжина_rec u16 | blob_rec | rec_tag[32]`
//! де кожен blob = `nonce[24] | шифротекст із тегом`, а `rec_tag` це MAC секції відновлення, ключований DEK:
//! він дозволяє після входу за PIN помітити пошкодження коду відновлення, не чекаючи дня, коли PIN забуто.

use crate::crypto::{open_sealed, seal, KdfParams, Secret32};
use crate::error::StorageError;

const MAGIC: &[u8; 4] = b"KMTV";
const VERSION: u8 = 1;
const MAX_BLOB: usize = 1024;

#[derive(Clone, Debug)]
pub struct KeyFile {
    pub params: KdfParams,
    pub hooked: bool,
    pub salt_pin: [u8; 16],
    pub blob_pin: Vec<u8>,
    pub salt_rec: [u8; 16],
    pub blob_rec: Vec<u8>,
    pub rec_tag: [u8; 32],
}

impl KeyFile {
    /// Дані, до яких «прив'язано» обгортку PIN: заголовок, параметри, ознака хука, сіль.
    pub fn aad_pin(&self) -> Vec<u8> {
        let mut a = self.header();
        a.extend_from_slice(b"pin");
        a.extend_from_slice(&self.salt_pin);
        a
    }

    /// Дані, до яких «прив'язано» обгортку коду відновлення (без ознаки хука: відновлення від нього не залежить).
    pub fn aad_rec(&self) -> Vec<u8> {
        let mut a = Vec::new();
        a.extend_from_slice(MAGIC);
        a.push(VERSION);
        a.extend_from_slice(&self.params.m_kib.to_le_bytes());
        a.extend_from_slice(&self.params.t.to_le_bytes());
        a.extend_from_slice(&self.params.p.to_le_bytes());
        a.extend_from_slice(b"rec");
        a.extend_from_slice(&self.salt_rec);
        a
    }

    /// Дані, які захищає `rec_tag`: параметри й уся секція відновлення.
    pub fn rec_tag_input(&self) -> Vec<u8> {
        let mut a = b"kmit-rec-tag-v1".to_vec();
        a.extend_from_slice(&self.params.m_kib.to_le_bytes());
        a.extend_from_slice(&self.params.t.to_le_bytes());
        a.extend_from_slice(&self.params.p.to_le_bytes());
        a.extend_from_slice(&self.salt_rec);
        a.extend_from_slice(&self.blob_rec);
        a
    }

    /// `true`, якщо секція відновлення не змінена (перевіряється ключем DEK).
    pub fn recovery_section_ok(&self, dek: &[u8; 32]) -> bool {
        let want = crate::crypto::mac(dek, &self.rec_tag_input());
        want.iter()
            .zip(self.rec_tag.iter())
            .fold(0u8, |a, (x, y)| a | (x ^ y))
            == 0
    }

    pub fn set_rec_tag(&mut self, dek: &[u8; 32]) {
        self.rec_tag = crate::crypto::mac(dek, &self.rec_tag_input());
    }

    fn header(&self) -> Vec<u8> {
        let mut h = Vec::with_capacity(18);
        h.extend_from_slice(MAGIC);
        h.push(VERSION);
        h.push(u8::from(self.hooked));
        h.extend_from_slice(&self.params.m_kib.to_le_bytes());
        h.extend_from_slice(&self.params.t.to_le_bytes());
        h.extend_from_slice(&self.params.p.to_le_bytes());
        h
    }

    pub fn encode(&self) -> Result<Vec<u8>, StorageError> {
        if self.blob_pin.len() > MAX_BLOB || self.blob_rec.len() > MAX_BLOB {
            return Err(StorageError::Corrupt("завеликий блок ключа"));
        }
        let mut out = self.header();
        out.extend_from_slice(&self.salt_pin);
        out.extend_from_slice(
            &u16::try_from(self.blob_pin.len())
                .unwrap_or(0)
                .to_le_bytes(),
        );
        out.extend_from_slice(&self.blob_pin);
        out.extend_from_slice(&self.salt_rec);
        out.extend_from_slice(
            &u16::try_from(self.blob_rec.len())
                .unwrap_or(0)
                .to_le_bytes(),
        );
        out.extend_from_slice(&self.blob_rec);
        out.extend_from_slice(&self.rec_tag);
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<KeyFile, StorageError> {
        let mut r = Reader { b: bytes, i: 0 };
        if r.take(4)? != MAGIC {
            return Err(StorageError::Corrupt("не файл ключа"));
        }
        if r.take(1)?[0] != VERSION {
            return Err(StorageError::Corrupt("невідома версія файлу ключа"));
        }
        let hooked = match r.take(1)?[0] {
            0 => false,
            1 => true,
            _ => return Err(StorageError::Corrupt("ознака хука")),
        };
        let params = KdfParams {
            m_kib: r.u32()?,
            t: r.u32()?,
            p: r.u32()?,
        };
        let salt_pin: [u8; 16] = r
            .take(16)?
            .try_into()
            .map_err(|_| StorageError::Corrupt("сіль"))?;
        let blob_pin = r.blob()?;
        let salt_rec: [u8; 16] = r
            .take(16)?
            .try_into()
            .map_err(|_| StorageError::Corrupt("сіль"))?;
        let blob_rec = r.blob()?;
        let rec_tag: [u8; 32] = r
            .take(32)?
            .try_into()
            .map_err(|_| StorageError::Corrupt("тег"))?;
        if r.i != bytes.len() {
            return Err(StorageError::Corrupt("зайві байти"));
        }
        Ok(KeyFile {
            params,
            hooked,
            salt_pin,
            blob_pin,
            salt_rec,
            blob_rec,
            rec_tag,
        })
    }
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], StorageError> {
        let end = self.i.checked_add(n).ok_or(StorageError::Corrupt("зсув"))?;
        let s = self
            .b
            .get(self.i..end)
            .ok_or(StorageError::Corrupt("обрізаний файл"))?;
        self.i = end;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32, StorageError> {
        let s: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| StorageError::Corrupt("число"))?;
        Ok(u32::from_le_bytes(s))
    }

    fn blob(&mut self) -> Result<Vec<u8>, StorageError> {
        let s: [u8; 2] = self
            .take(2)?
            .try_into()
            .map_err(|_| StorageError::Corrupt("довжина"))?;
        let len = usize::from(u16::from_le_bytes(s));
        if len > MAX_BLOB {
            return Err(StorageError::Corrupt("завеликий блок ключа"));
        }
        Ok(self.take(len)?.to_vec())
    }
}

/// Обгортає DEK ключем `kek`: `nonce | шифротекст`.
pub fn wrap_dek(dek: &[u8; 32], kek: &[u8; 32], aad: &[u8]) -> Result<Vec<u8>, StorageError> {
    let (nonce, ct) = seal(kek, dek, aad)?;
    let mut out = nonce.to_vec();
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Розгортає DEK. Будь-яка помилка (чужий ключ, зміна даних, неправильна довжина) повертається як `Crypto`.
pub fn unwrap_dek(blob: &[u8], kek: &[u8; 32], aad: &[u8]) -> Result<Secret32, StorageError> {
    if blob.len() < 24 + 16 {
        return Err(StorageError::Crypto("короткий блок"));
    }
    let nonce: [u8; 24] = blob[..24]
        .try_into()
        .map_err(|_| StorageError::Crypto("nonce"))?;
    let plain = open_sealed(kek, &nonce, &blob[24..], aad)?;
    let dek: [u8; 32] = plain
        .as_slice()
        .try_into()
        .map_err(|_| StorageError::Crypto("довжина ключа"))?;
    Ok(zeroize::Zeroizing::new(dek))
}

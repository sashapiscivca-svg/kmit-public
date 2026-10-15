//! Встановлені пакети вмісту: підписані файли в теці даних застосунку. Підпис перевіряється при встановленні і при кожному
//! запуску (файл у теці могли підмінити); пакет не новіший за вбудований ігнорується.

use crate::error::ApiError;
use std::path::Path;
use subjects::pack::{self, Pack};

fn rejected(why: &str) -> ApiError {
    ApiError::new("pack_rejected", why)
}

/// Перевіряє й записує пакет. Повертає id предмета.
pub(crate) fn install(dir: &Path, data: &[u8], sig: &[u8]) -> Result<String, ApiError> {
    install_with(dir, data, sig, &pack::PUBLISHER_PUBLIC_KEY)
}

pub(crate) fn install_with(
    dir: &Path,
    data: &[u8],
    sig: &[u8],
    key: &[u8; 32],
) -> Result<String, ApiError> {
    pack::verify_signature(data, sig, key).map_err(|_| rejected("Підпис пакета не підходить"))?;
    let p = pack::parse(data).map_err(|e| rejected(&e.to_string()))?;
    let base = subjects::base_versions();
    if base.iter().any(|(id, v)| *id == p.id && *v >= p.version) {
        return Err(rejected("Цей пакет не новіший за вбудований"));
    }
    std::fs::create_dir_all(dir).map_err(|_| rejected("Не вдалося записати пакет"))?;
    std::fs::write(dir.join(format!("{}.json", p.id)), data)
        .map_err(|_| rejected("Не вдалося записати пакет"))?;
    std::fs::write(dir.join(format!("{}.sig", p.id)), sig)
        .map_err(|_| rejected("Не вдалося записати підпис"))?;
    Ok(p.id)
}

/// Читає встановлені пакети; ті, що не проходять підпис чи розбір, мовчки пропускаються.
pub(crate) fn load_installed(dir: &Path) -> Vec<Pack> {
    load_installed_with(dir, &pack::PUBLISHER_PUBLIC_KEY)
}

pub(crate) fn load_installed_with(dir: &Path, key: &[u8; 32]) -> Vec<Pack> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let path = e.path();
        if path.extension().map_or(true, |x| x != "json") {
            continue;
        }
        let (Ok(data), Ok(sig)) = (
            std::fs::read(&path),
            std::fs::read(path.with_extension("sig")),
        ) else {
            continue;
        };
        if pack::verify_signature(&data, &sig, key).is_err() {
            continue;
        }
        if let Ok(p) = pack::parse(&data) {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn signed(version_bump: u32, key: &SigningKey) -> (Vec<u8>, Vec<u8>) {
        let mut p = subjects::current_packs()
            .into_iter()
            .find(|p| p.id == "history")
            .unwrap();
        p.version += version_bump;
        p.skills[0].title = format!("{} (оновлено)", p.skills[0].title);
        let data = serde_json::to_vec(&p).unwrap();
        let sig = key.sign(&data).to_bytes().to_vec();
        (data, sig)
    }

    fn dir() -> std::path::PathBuf {
        let mut b = [0u8; 8];
        getrandom::getrandom(&mut b).unwrap();
        let d = std::env::temp_dir().join(format!(
            "kmit-packs-{}",
            b.iter().map(|x| format!("{x:02x}")).collect::<String>()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn a_correctly_signed_newer_pack_installs_and_reloads_but_tampering_or_old_versions_do_not() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let public = *key.verifying_key().as_bytes();
        let d = dir();
        let (data, sig) = signed(1, &key);
        assert_eq!(install_with(&d, &data, &sig, &public).unwrap(), "history");
        let loaded = load_installed_with(&d, &public);
        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].skills[0].title.ends_with("(оновлено)"));
        // Підмінений після встановлення файл більше не завантажується.
        let mut tampered = data.clone();
        tampered.extend_from_slice(b" ");
        std::fs::write(d.join("history.json"), &tampered).unwrap();
        assert!(load_installed_with(&d, &public).is_empty());
        // Той самий пакет не новіший за вбудований: відхиляється; чужий ключ: відхиляється.
        let d2 = dir();
        let (old, old_sig) = signed(0, &key);
        assert!(install_with(&d2, &old, &old_sig, &public).is_err());
        let other = SigningKey::from_bytes(&[9u8; 32]);
        let (data2, sig2) = signed(1, &other);
        assert!(install_with(&d2, &data2, &sig2, &public).is_err());
        let _ = std::fs::remove_dir_all(d);
        let _ = std::fs::remove_dir_all(d2);
    }
}

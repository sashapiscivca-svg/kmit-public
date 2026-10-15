//! До кількох профілів (дітей) на одному пристрої (відкрите питання Q20). Кожен профіль це окреме сховище
//! в теці `p<слот>` зі своїм PIN, ключем і кодом відновлення. Відкрито (без PIN) видно лише номер слоту
//! й номер аватара у файлі `slot.meta`; нікнейм зашифровано.

use crate::crypto::KdfParams;
use crate::error::StorageError;
use crate::state::{ProfileKind, MAX_AVATAR};
use crate::vault::{KeyWrapHook, Vault};
use std::fs;
use std::path::{Path, PathBuf};

pub const MAX_PROFILES: usize = 4;
const META: &str = "slot.meta";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotInfo {
    pub slot: u8,
    pub avatar: u8,
    /// Тип профілю видно до введення PIN (для іншої обкладинки), це не є таємницею.
    pub kind: ProfileKind,
}

fn meta_text(avatar: u8, kind: ProfileKind) -> String {
    match kind {
        ProfileKind::Child => format!("avatar={}\n", avatar.min(MAX_AVATAR)),
        ProfileKind::Teacher => format!("avatar={}\nkind=teacher\n", avatar.min(MAX_AVATAR)),
    }
}

/// Скільки хибних спроб поспіль безкоштовні, далі зростає пауза.
pub const LOCK_AFTER: u32 = 5;
/// Найдовша пауза між спробами, секунди.
pub const MAX_LOCK_SECS: u32 = 900;
const LOCKFILE: &str = "lockout";

/// Пауза після `failures` хибних спроб: 30 с після п'ятої, далі вдвічі довша, не більше межі.
pub fn lock_delay(failures: u32) -> u32 {
    delay_for(failures)
}

fn delay_for(failures: u32) -> u32 {
    if failures < LOCK_AFTER {
        return 0;
    }
    let exp = (failures - LOCK_AFTER).min(5);
    (30u32 << exp).min(MAX_LOCK_SECS)
}

fn read_lock(dir: &Path) -> (u32, u64) {
    let Ok(t) = fs::read_to_string(dir.join(LOCKFILE)) else {
        return (0, 0);
    };
    let mut it = t.split_whitespace();
    let n = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let at = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    (n, at)
}

/// Перевіряє, чи не триває пауза. Якщо годинник відкотили назад, пауза не довша за межу.
fn check_lock(dir: &Path, now: u64) -> Result<(), StorageError> {
    let (n, last) = read_lock(dir);
    let delay = u64::from(delay_for(n));
    if delay == 0 {
        return Ok(());
    }
    let until = last.saturating_add(delay);
    if now < until {
        let left = (until - now).min(u64::from(MAX_LOCK_SECS));
        return Err(StorageError::Locked(
            u32::try_from(left).unwrap_or(MAX_LOCK_SECS),
        ));
    }
    Ok(())
}

fn note_failure(dir: &Path, now: u64) {
    let (n, _) = read_lock(dir);
    let _ = fs::write(
        dir.join(LOCKFILE),
        format!("{} {}", n.saturating_add(1), now),
    );
}

fn clear_failures(dir: &Path) {
    let _ = fs::remove_file(dir.join(LOCKFILE));
}

pub struct ProfileStore {
    root: PathBuf,
}

impl ProfileStore {
    pub fn new(root: &Path) -> Self {
        ProfileStore {
            root: root.to_path_buf(),
        }
    }

    fn slot_dir(&self, slot: u8) -> Result<PathBuf, StorageError> {
        if slot == 0 || usize::from(slot) > MAX_PROFILES {
            return Err(StorageError::NoSuchProfile);
        }
        Ok(self.root.join(format!("p{slot}")))
    }

    /// Профілі, що існують (за наявністю файлу ключа), за зростанням слота.
    pub fn list(&self) -> Result<Vec<SlotInfo>, StorageError> {
        let mut out = Vec::new();
        for slot in 1..=u8::try_from(MAX_PROFILES).unwrap_or(4) {
            let dir = self.slot_dir(slot)?;
            if !Vault::exists(&dir) {
                continue;
            }
            let avatar = fs::read_to_string(dir.join(META))
                .ok()
                .and_then(|t| {
                    t.lines()
                        .find_map(|l| l.strip_prefix("avatar=").and_then(|v| v.parse::<u8>().ok()))
                })
                .unwrap_or(0)
                .min(MAX_AVATAR);
            let kind = fs::read_to_string(dir.join(META))
                .ok()
                .and_then(|t| {
                    t.lines()
                        .find_map(|l| l.strip_prefix("kind=").map(ProfileKind::parse))
                })
                .unwrap_or_default();
            out.push(SlotInfo { slot, avatar, kind });
        }
        Ok(out)
    }

    /// Оновлює публічний код вигляду профілю (його видно в списку профілів до введення PIN).
    pub fn set_avatar(&self, slot: u8, avatar: u8) -> Result<(), StorageError> {
        let dir = self.slot_dir(slot)?;
        if !Vault::exists(&dir) {
            return Ok(());
        }
        let kind = self
            .list()?
            .into_iter()
            .find(|s| s.slot == slot)
            .map_or(ProfileKind::Child, |s| s.kind);
        fs::write(dir.join(META), meta_text(avatar, kind)).map_err(StorageError::from)
    }

    /// Перший вільний слот.
    pub fn free_slot(&self) -> Result<u8, StorageError> {
        let taken: Vec<u8> = self.list()?.iter().map(|s| s.slot).collect();
        (1..=u8::try_from(MAX_PROFILES).unwrap_or(4))
            .find(|s| !taken.contains(s))
            .ok_or(StorageError::TooManyProfiles)
    }

    /// Створює профіль у першому вільному слоті; повертає слот, відкрите сховище й код відновлення.
    pub fn create(
        &self,
        pin: &str,
        avatar: u8,
        params: KdfParams,
        hook: Option<&dyn KeyWrapHook>,
    ) -> Result<(u8, Vault, String), StorageError> {
        let slot = self.free_slot()?;
        let (v, code) = self.create_in_slot(slot, pin, avatar, params, hook)?;
        Ok((slot, v, code))
    }

    /// Створює профіль у заданому вільному слоті (оболонка спершу готує хук саме для цього слота).
    pub fn create_in_slot(
        &self,
        slot: u8,
        pin: &str,
        avatar: u8,
        params: KdfParams,
        hook: Option<&dyn KeyWrapHook>,
    ) -> Result<(Vault, String), StorageError> {
        self.create_in_slot_kind(slot, pin, avatar, ProfileKind::Child, params, hook)
    }

    /// Те саме із зазначенням типу профілю (учень чи вчитель).
    pub fn create_in_slot_kind(
        &self,
        slot: u8,
        pin: &str,
        avatar: u8,
        kind: ProfileKind,
        params: KdfParams,
        hook: Option<&dyn KeyWrapHook>,
    ) -> Result<(Vault, String), StorageError> {
        let dir = self.slot_dir(slot)?;
        if Vault::exists(&dir) {
            return Err(StorageError::AlreadyExists);
        }
        // Залишки від перерваного створення прибираємо: вільний слот це слот без файлу ключа.
        Vault::erase_dir(&dir)?;
        let (vault, code) = Vault::create(&dir, pin, params, hook)?;
        let _ = fs::write(dir.join(META), meta_text(avatar, kind));
        Ok((vault, code))
    }

    pub fn open(
        &self,
        slot: u8,
        pin: &str,
        hook: Option<&dyn KeyWrapHook>,
    ) -> Result<Vault, StorageError> {
        let dir = self.slot_dir(slot)?;
        if !Vault::exists(&dir) {
            return Err(StorageError::NoSuchProfile);
        }
        Vault::open(&dir, pin, hook)
    }

    pub fn recover(
        &self,
        slot: u8,
        code: &str,
        new_pin: &str,
        hook: Option<&dyn KeyWrapHook>,
    ) -> Result<Vault, StorageError> {
        let dir = self.slot_dir(slot)?;
        if !Vault::exists(&dir) {
            return Err(StorageError::NoSuchProfile);
        }
        Vault::recover(&dir, code, new_pin, hook)
    }

    /// Як `open`, але з обмеженням спроб: після `LOCK_AFTER` хибних PIN між спробами зростає пауза (до `MAX_LOCK_SECS`).
    /// `now`: секунди годинника пристрою.
    pub fn open_at(
        &self,
        slot: u8,
        pin: &str,
        hook: Option<&dyn KeyWrapHook>,
        now: u64,
    ) -> Result<Vault, StorageError> {
        let dir = self.slot_dir(slot)?;
        if !Vault::exists(&dir) {
            return Err(StorageError::NoSuchProfile);
        }
        check_lock(&dir, now)?;
        match Vault::open(&dir, pin, hook) {
            Err(StorageError::WrongPin) => {
                note_failure(&dir, now);
                Err(StorageError::WrongPin)
            }
            other => {
                if other.is_ok() {
                    clear_failures(&dir);
                }
                other
            }
        }
    }

    /// Як `recover`, з тим самим лічильником спроб.
    pub fn recover_at(
        &self,
        slot: u8,
        code: &str,
        new_pin: &str,
        hook: Option<&dyn KeyWrapHook>,
        now: u64,
    ) -> Result<Vault, StorageError> {
        let dir = self.slot_dir(slot)?;
        if !Vault::exists(&dir) {
            return Err(StorageError::NoSuchProfile);
        }
        check_lock(&dir, now)?;
        match Vault::recover(&dir, code, new_pin, hook) {
            Err(StorageError::WrongRecoveryCode) => {
                note_failure(&dir, now);
                Err(StorageError::WrongRecoveryCode)
            }
            other => {
                if other.is_ok() {
                    clear_failures(&dir);
                }
                other
            }
        }
    }

    /// Стирає профіль без PIN (кнопка «стерти»): ключі знищено, слот вільний.
    pub fn erase(&self, slot: u8, hook: Option<&dyn KeyWrapHook>) -> Result<(), StorageError> {
        let dir = match self.slot_dir(slot) {
            Ok(d) => d,
            Err(_) => return Ok(()),
        };
        if !dir.exists() {
            return Ok(());
        }
        Vault::erase_dir(&dir)?;
        let _ = fs::remove_file(dir.join(META));
        let _ = fs::remove_dir(&dir);
        if let Some(h) = hook {
            h.destroy()?;
        }
        Ok(())
    }
}

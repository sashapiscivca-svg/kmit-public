//! Кабінет вчителя: окремий тип профілю з власними командами. Жодних даних дітей: команди `teacher_*` не читають
//! дитячих сховищ, а дитячі команди недоступні вчительському профілю (і навпаки). Рішення A31.

use storage::state::ProfileKind;

/// Команди, які може виконувати вчительський профіль, крім `teacher_*`: налаштування, захист, копія профілю, пакети.
const TEACHER_SHARED: [&str; 11] = [
    "settings_get",
    "settings_set",
    "adult_status",
    "adult_set_pin",
    "adult_unlock",
    "adult_lock",
    "recovery_new",
    "export_profile",
    "packs_list",
    "pack_install",
    "about",
];

/// Чи дозволено команду цьому типу профілю.
pub(crate) fn allowed(kind: ProfileKind, name: &str) -> bool {
    match kind {
        // Збірка методиста: лише вичитка (`rv_*`) і новий код відновлення; усе інше (дитячі й вчительські команди) заборонено.
        _ if cfg!(feature = "reviewer") => {
            kind == ProfileKind::Teacher && (name.starts_with("rv_") || name == "recovery_new")
        }
        _ if name.starts_with("rv_") => false,
        ProfileKind::Child => !name.starts_with("teacher_"),
        ProfileKind::Teacher => name.starts_with("teacher_") || TEACHER_SHARED.contains(&name),
    }
}

mod celebrate;
mod lessons;
mod work;

pub(crate) use celebrate::{pending as celebration_pending, seen as celebration_seen};
pub(crate) use lessons::*;
pub(crate) use work::*;

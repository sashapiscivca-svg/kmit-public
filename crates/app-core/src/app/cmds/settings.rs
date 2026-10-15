//! Налаштування, режим дорослого, відновлення й перенесення профілю.

use super::*;

pub(super) fn handle(
    env: &Env,
    open: &mut Open,
    name: &str,
    args: &Value,
    ctx: &Ctx,
) -> Result<Option<Reply>, ApiError> {
    let out = match name {
        "settings_get" => settings_get(open, ctx),
        "settings_set" => settings_set(open, args, ctx)?,
        "adult_status" => {
            json!({"pin_set": open.vault.has_adult_pin()?, "unlocked": adult_unlocked(open, ctx)})
        }
        "adult_set_pin" => {
            let pin = str_arg(args, "pin")?;
            require_adult(open, ctx)?;
            open.vault.set_adult_pin(pin)?;
            open.adult_until = ctx.wall.saturating_add(ADULT_UNLOCK_SECS);
            json!({"ok": true})
        }
        "adult_unlock" => {
            // Пауза після серії хибних спроб (як для PIN профілю).
            let (n, last) = open.adult_fail;
            let delay = u64::from(storage::profiles::lock_delay(n));
            if delay > 0 && ctx.wall < last.saturating_add(delay) {
                let left =
                    (last + delay - ctx.wall).min(u64::from(storage::profiles::MAX_LOCK_SECS));
                return Err(ApiError::from(storage::StorageError::Locked(
                    u32::try_from(left).unwrap_or(storage::profiles::MAX_LOCK_SECS),
                )));
            }
            if open.vault.verify_adult_pin(str_arg(args, "pin")?)? {
                open.adult_fail = (0, 0);
                open.adult_until = ctx.wall.saturating_add(ADULT_UNLOCK_SECS);
                json!({"ok": true})
            } else {
                open.adult_fail = (n.saturating_add(1), ctx.wall);
                return Err(ApiError::new("wrong_adult_pin", "Невірний PIN дорослого"));
            }
        }
        "adult_lock" => {
            open.adult_until = 0;
            json!({"ok": true})
        }
        "adult_items" => {
            require_adult(open, ctx)?;
            json!({"items": open.state.learner.ask_adult_items().iter().map(|(skill, statement)| json!({
                "skill": skill, "title": skill_title(env, skill), "statement": statement,
            })).collect::<Vec<_>>()})
        }
        "adult_resolve" => {
            require_adult(open, ctx)?;
            let removed = open
                .state
                .learner
                .resolve_ask_adult(str_arg(args, "skill")?, str_arg(args, "statement")?);
            json!({"removed": removed})
        }
        "recovery_new" => {
            require_adult(open, ctx)?;
            let code = open.vault.new_recovery_code()?;
            json!({"recovery_code": code})
        }
        "export_profile" => {
            require_adult(open, ctx)?;
            let blob = open
                .vault
                .export_encrypted(str_arg(args, "passphrase")?, KdfParams::production())?;
            open.state.last_export_day = ctx.day.max(1);
            json!({"data": base64::encode(&blob), "filename": "profil.kmit"})
        }
        _ => return Ok(None),
    };
    Ok(Some(Reply::Done(out)))
}

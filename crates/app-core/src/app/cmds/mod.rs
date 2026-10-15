//! Команди відкритого профілю за предметною областю. Кожен модуль повертає `None`, якщо команда не його.

use super::*;

mod docs;
mod home;
mod learn;
#[cfg(feature = "reviewer")]
mod review;
mod session;
mod settings;
mod teacher;

/// Відповідь команди: `Done` проходить спільну післяобробку (відзнаки, збереження), `Early` повертається як є.
pub(super) enum Reply {
    Done(Value),
    Early(Value),
}

fn early(v: Value) -> Result<Option<Reply>, ApiError> {
    Ok(Some(Reply::Early(v)))
}

type Handler = fn(&Env, &mut Open, &str, &Value, &Ctx) -> Result<Option<Reply>, ApiError>;

#[cfg(feature = "reviewer")]
const HANDLERS: [Handler; 7] = [
    review::handle,
    teacher::handle,
    home::handle,
    learn::handle,
    session::handle,
    docs::handle,
    settings::handle,
];
#[cfg(not(feature = "reviewer"))]
const HANDLERS: [Handler; 6] = [
    teacher::handle,
    home::handle,
    learn::handle,
    session::handle,
    docs::handle,
    settings::handle,
];

pub(super) fn dispatch(
    env: &Env,
    open: &mut Open,
    name: &str,
    args: &Value,
    ctx: &Ctx,
) -> Result<Reply, ApiError> {
    for h in HANDLERS {
        if let Some(v) = h(env, open, name, args, ctx)? {
            return Ok(v);
        }
    }
    Err(ApiError::new(
        "unknown_command",
        &format!("невідома команда {name}"),
    ))
}

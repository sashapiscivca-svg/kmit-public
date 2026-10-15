//! Вичитка для методиста (лише збірка `reviewer`).

use super::*;

pub(super) fn handle(
    _env: &Env,
    open: &mut Open,
    name: &str,
    args: &Value,
    ctx: &Ctx,
) -> Result<Option<Reply>, ApiError> {
    let out = match name {
        "rv_overview" => crate::review::overview_json(&open.state),
        "rv_topic" => {
            let id = args.get("skill").and_then(Value::as_str).unwrap_or("");
            crate::review::topic_json(&open.state, id)?
        }
        "rv_note" => crate::review::note_set(&mut open.state, args)?,
        "rv_clear" => crate::review::note_clear(&mut open.state, args)?,
        "rv_topic_ok" => crate::review::topic_ok(&mut open.state, args)?,
        "rv_export" => {
            let only = args.get("skill").and_then(Value::as_str);
            let (csv, filename) = crate::review::export(&open.state, only, ctx)?;
            json!({"data": base64::encode(csv.as_bytes()), "filename": filename})
        }
        _ => return Ok(None),
    };
    Ok(Some(Reply::Done(out)))
}

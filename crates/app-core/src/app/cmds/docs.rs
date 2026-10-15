//! Аркуші, подяка вчителю, конспекти.

use super::*;

pub(super) fn handle(
    env: &Env,
    open: &mut Open,
    name: &str,
    args: &Value,
    ctx: &Ctx,
) -> Result<Option<Reply>, ApiError> {
    let out = match name {
        "sheet" => {
            let subject = args
                .get("subject")
                .and_then(Value::as_str)
                .unwrap_or("math");
            let sheet = crate::repeat::subject_sheet(env, open, subject, ctx)
                .ok_or_else(|| ApiError::bad_request("невідомий предмет"))?;
            sheet_json(&sheet)
        }
        "sheet_pdf" => {
            let subject = args
                .get("subject")
                .and_then(Value::as_str)
                .unwrap_or("math");
            let sheet = crate::repeat::subject_sheet(env, open, subject, ctx)
                .ok_or_else(|| ApiError::bad_request("невідомий предмет"))?;
            let hero = tutor_core::hero::compose(&open.state.hero);
            let pdf = report::styled::styled_sheet_pdf(
                &sheet,
                crate::catalog::subject_color(subject),
                &hero,
            )
            .map_err(|_| ApiError::new("pdf", "Не вдалося створити аркуш"))?;
            json!({"data": base64::encode(&pdf), "filename": format!("arkush-dlya-vchytelya-{subject}.pdf")})
        }
        "thanks_pdf" | "thanks_text" => {
            let clean = |key: &str, max: usize| -> String {
                args.get(key)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .chars()
                    .filter(|c| *c == '\n' || !c.is_control())
                    .take(max)
                    .collect::<String>()
                    .trim()
                    .to_string()
            };
            let (to, text, from) = (clean("to", 60), clean("text", 600), clean("from", 40));
            if !crate::lesson_prog::is_real_text(&text, 4, 15) {
                return Err(ApiError::bad_request("Напиши кілька слів власними словами"));
            }
            if name == "thanks_pdf" {
                let hero = tutor_core::hero::compose(&open.state.hero);
                let pdf = report::styled::thanks_card_pdf(&to, &text, &from, &hero)
                    .map_err(|_| ApiError::new("pdf", "Не вдалося створити листівку"))?;
                json!({"data": base64::encode(&pdf), "filename": "lyst-podyaky.pdf"})
            } else {
                let mut body = String::new();
                if !to.is_empty() {
                    body.push_str(&format!("{to}\n\n"));
                }
                body.push_str(&text);
                if !from.is_empty() {
                    body.push_str(&format!("\n\nЗ повагою, {from}"));
                }
                json!({"data": base64::encode(body.as_bytes()), "filename": "lyst-podyaky.txt"})
            }
        }
        "note_get" => {
            let note = open
                .state
                .notes
                .get(skill_arg(env, args)?)
                .cloned()
                .unwrap_or_default();
            let fields: Vec<Value> = crate::lesson_prog::note_fields(&note)
                .into_iter()
                .map(|(l, t)| json!({"label": l, "text": t}))
                .collect();
            json!({"text": crate::lesson_prog::note_readable(&note), "fields": fields})
        }
        "note_set" => {
            let skill = skill_arg(env, args)?.to_string();
            let clean = |raw: &str, max: usize| -> String {
                raw.chars()
                    .filter(|c| *c == '\n' || !c.is_control())
                    .filter(|c| *c != '§')
                    .take(max)
                    .collect::<String>()
                    .trim()
                    .to_string()
            };
            let fields: Vec<(String, String)> = if let Some(arr) =
                args.get("fields").and_then(Value::as_array)
            {
                arr.iter()
                    .take(8)
                    .filter_map(|f| {
                        let label = clean(f.get("label").and_then(Value::as_str).unwrap_or(""), 80)
                            .replace('\n', " ");
                        let text = clean(
                            f.get("text").and_then(Value::as_str).unwrap_or(""),
                            MAX_NOTE_CHARS,
                        );
                        (!text.is_empty()).then_some((label, text))
                    })
                    .collect()
            } else {
                let t = clean(
                    args.get("text").and_then(Value::as_str).unwrap_or(""),
                    MAX_NOTE_CHARS,
                );
                if t.is_empty() {
                    Vec::new()
                } else {
                    vec![(String::new(), t)]
                }
            };
            let mut stored = crate::lesson_prog::note_join(&fields);
            if stored.chars().count() > MAX_NOTE_CHARS {
                stored = stored.chars().take(MAX_NOTE_CHARS).collect();
            }
            if fields.is_empty() {
                open.state.notes.remove(&skill);
            } else {
                open.state.notes.insert(skill, stored.clone());
            }
            json!({"text": crate::lesson_prog::note_readable(&stored)})
        }
        "notes_list" => {
            json!({"notes": notes_entries(env, open).iter().map(|(id, title, text)| json!({
            "skill": id, "title": title, "chars": text.chars().count(),
        })).collect::<Vec<_>>()})
        }
        "notes_pdf" => {
            let entries: Vec<(String, String)> = notes_entries(env, open)
                .into_iter()
                .map(|(_, title, text)| (title, text))
                .collect();
            let pdf = report::notes::notes_pdf(&open.state.profile.nickname, &entries)
                .map_err(|_| ApiError::new("pdf", "Не вдалося створити конспект"))?;
            json!({"data": base64::encode(&pdf), "filename": "moj-konspekt.pdf"})
        }
        _ => return Ok(None),
    };
    Ok(Some(Reply::Done(out)))
}

//! Вичитка для методиста (лише збірка `reviewer`): огляд тем, зауваження до окремих текстів, позначка «усе гаразд»,
//! експорт CSV для `content-tools apply`. Дані дітей не використовуються. Підпис і підвищення версії пакета за видавцем.
//!
//! Одиниця вичитки (`unit`): одне завдання (`bank:N`, `typed:N`), один блок уроку (`lesson:N`), назва уроку
//! (`lesson:title`), запитання для нотаток (`lesson:notes`). Позначки зберігаються як `ReviewNote`:
//! - «усе гаразд»: статус `ok`, поле `*` (для назви й нотаток `*:title`, `*:notes`), у `text` відбиток змісту одиниці;
//! - виправлення: статус `fix`, поле тексту (`prompt`, `hint:0`, …), у `text` початковий текст;
//! - «прибрати завдання»: статус `remove`, поле `remove`, у `text` умова завдання (у CSV це поле `prompt`).
//!
//! Позначка діє, лише поки текст у пакеті той самий (відбиток чи текст збігається): після оновлення вмісту
//! (видалені завдання зсувають індекси) застарілі позначки ігноруються й не потрапляють у файл.

use crate::ctx::Ctx;
use crate::datefmt::date_of;
use crate::error::ApiError;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use storage::state::{AppState, ReviewNote, MAX_REVIEW_NOTES};

/// Рядок звіту теми: `(тип, індекс, поле, текст)`.
type Row = (String, u32, String, String);

/// Ідентифікатор одиниці за типом, індексом і назвою поля рядка звіту.
fn unit_id(kind: &str, index: u32, field: &str) -> String {
    if kind == "lesson" {
        if field == "title" || field == "*:title" {
            return "lesson:title".to_string();
        }
        if field.starts_with("note_prompt") || field == "*:notes" {
            return "lesson:notes".to_string();
        }
    }
    format!("{kind}:{index}")
}

fn split_field(n: &ReviewNote) -> (&str, &str) {
    n.field.split_once(':').unwrap_or(("", n.field.as_str()))
}

/// Одиниця, до якої належить зауваження.
fn note_unit(n: &ReviewNote) -> String {
    let (kind, field) = split_field(n);
    unit_id(kind, n.index, field)
}

/// Рядки звіту теми (без назви теми). Вміст незмінний, тому читаємо з пакета один раз на тему.
fn rows_of(skill: &str) -> Option<Arc<Vec<Row>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Vec<Row>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(r) = cache.lock().ok()?.get(skill) {
        return Some(r.clone());
    }
    let pack = serde_json::to_value(subjects::pack_for_skill(skill)?).ok()?;
    let rows: Vec<Row> = content_tools::review::rows_for(&pack)
        .into_iter()
        .filter(|r| r[1] == skill && r[2] != "skill")
        .map(|r| {
            (
                r[2].clone(),
                r[3].parse().unwrap_or(0),
                r[4].clone(),
                r[5].clone(),
            )
        })
        .collect();
    let rows = Arc::new(rows);
    cache.lock().ok()?.insert(skill.to_string(), rows.clone());
    Some(rows)
}

/// Усі одиниці кожної теми в порядку показу.
fn units_by_skill() -> &'static HashMap<String, Vec<String>> {
    static CACHE: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut out: HashMap<String, Vec<String>> = HashMap::new();
        for pack in subjects::current_packs() {
            let Ok(value) = serde_json::to_value(&pack) else {
                continue;
            };
            for r in content_tools::review::rows_for(&value) {
                if r[2] == "skill" {
                    continue;
                }
                let id = unit_id(&r[2], r[3].parse().unwrap_or(0), &r[4]);
                let list = out.entry(r[1].clone()).or_default();
                if !list.contains(&id) {
                    list.push(id);
                }
            }
        }
        out
    })
}

/// Відбиток змісту одиниці (FNV-1a): позначка «усе гаразд» діє, лише поки зміст той самий.
fn unit_fp(rows: &[Row], unit: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (kind, index, field, text) in rows {
        if unit_id(kind, *index, field) != unit {
            continue;
        }
        for b in field.bytes().chain([0]).chain(text.bytes()).chain([0]) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{h:016x}")
}

/// Чи позначка відповідає чинному змісту пакета.
fn is_current(n: &ReviewNote, rows: &[Row]) -> bool {
    let (kind, field) = split_field(n);
    if field.starts_with('*') {
        return unit_fp(rows, &unit_id(kind, n.index, field)) == n.text;
    }
    let want = if field == "remove" { "prompt" } else { field };
    rows.iter()
        .find(|r| r.0 == kind && r.1 == n.index && r.2 == want)
        .is_some_and(|r| r.3 == n.text)
}

/// Позначки теми, що відповідають чинному змісту, згруповані за одиницями; друге число це кількість застарілих.
fn current_by_unit<'a>(
    notes: &[&'a ReviewNote],
    skill: &str,
) -> (HashMap<String, Vec<&'a ReviewNote>>, usize) {
    let mut m: HashMap<String, Vec<&ReviewNote>> = HashMap::new();
    if notes.is_empty() {
        return (m, 0);
    }
    let Some(rows) = rows_of(skill) else {
        return (m, notes.len());
    };
    let mut stale = 0;
    for n in notes {
        if is_current(n, &rows) {
            m.entry(note_unit(n)).or_default().push(n);
        } else {
            stale += 1;
        }
    }
    (m, stale)
}

fn notes_of<'a>(state: &'a AppState, skill: &str) -> Vec<&'a ReviewNote> {
    state
        .teacher
        .notes
        .iter()
        .filter(|n| n.skill == skill)
        .collect()
}

/// Стан одиниці за зауваженнями: `remove` > `fix` > `ok` > `none`.
fn unit_status(notes: &[&ReviewNote]) -> &'static str {
    if notes
        .iter()
        .any(|n| n.status == "remove" && n.field.ends_with(":remove"))
    {
        "remove"
    } else if notes.iter().any(|n| n.status == "fix") {
        "fix"
    } else if notes.iter().any(|n| n.status == "ok") {
        "ok"
    } else {
        "none"
    }
}

/// Огляд: пакети за класами, у кожної теми лічильники одиниць і підсумок для головного екрана.
pub(crate) fn overview_json(state: &AppState) -> Value {
    let versions = subjects::pack_versions();
    let units = units_by_skill();
    let mut by_skill: HashMap<&str, Vec<&ReviewNote>> = HashMap::new();
    for n in &state.teacher.notes {
        by_skill.entry(n.skill.as_str()).or_default().push(n);
    }
    let (mut t_total, mut t_done, mut t_fix, mut t_remove, mut t_stale) =
        (0u64, 0u64, 0u64, 0u64, 0usize);
    let mut packs: Vec<Value> = subjects::subjects()
        .into_iter()
        .map(|s| {
            let version = versions
                .iter()
                .find(|(id, _)| id == s.pack)
                .map_or(1, |(_, v)| *v);
            let topics: Vec<Value> = s
                .skills
                .iter()
                .map(|k| {
                    let all = units.get(k.id).map_or(&[][..], Vec::as_slice);
                    let own = by_skill.get(k.id).map_or(&[][..], Vec::as_slice);
                    let (by, stale) = current_by_unit(own, k.id);
                    t_stale += stale;
                    let (mut done, mut fix, mut remove) = (0u64, 0u64, 0u64);
                    for u in all {
                        match by.get(u).map_or("none", |n| unit_status(n)) {
                            "ok" => done += 1,
                            "fix" => {
                                done += 1;
                                fix += 1;
                            }
                            "remove" => {
                                done += 1;
                                remove += 1;
                            }
                            _ => {}
                        }
                    }
                    t_total += all.len() as u64;
                    t_done += done;
                    t_fix += fix;
                    t_remove += remove;
                    json!({"id": k.id, "title": k.title, "status": subjects::review_status(k.id),
                           "total": all.len(), "done": done, "fix": fix, "remove": remove})
                })
                .collect();
            json!({"pack": s.pack, "subject": s.id, "grade": s.grade, "title": s.title,
                   "version": version, "topics": topics})
        })
        .collect();
    packs.sort_by_key(|p| {
        (
            p["grade"].as_u64().unwrap_or(0),
            p["title"].as_str().unwrap_or("").to_string(),
        )
    });
    json!({"packs": packs, "totals": {"total": t_total, "done": t_done, "fix": t_fix,
           "remove": t_remove, "notes": state.teacher.notes.len() - t_stale, "stale": t_stale,
           "name": state.profile.nickname}})
}

/// Усі тексти теми, згруповані за одиницями, разом зі станом і зауваженнями.
pub(crate) fn topic_json(state: &AppState, skill: &str) -> Result<Value, ApiError> {
    let rows = rows_of(skill).ok_or_else(|| ApiError::bad_request("Невідома тема"))?;
    let own = notes_of(state, skill);
    let (by, _) = current_by_unit(&own, skill);
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, (String, u32, Vec<Value>)> = HashMap::new();
    for (kind, index, field, text) in rows.iter() {
        let id = unit_id(kind, *index, field);
        let note = by.get(&id).and_then(|v| {
            v.iter()
                .find(|n| n.field == format!("{kind}:{field}") && n.index == *index)
        });
        let entry = groups.entry(id.clone()).or_insert_with(|| {
            order.push(id.clone());
            (kind.clone(), *index, Vec::new())
        });
        entry.2.push(json!({
            "field": field, "text": text,
            "note": note.map(|n| json!({"status": n.status, "correction": n.correction, "comment": n.comment})),
        }));
    }
    let units: Vec<Value> = order
        .iter()
        .map(|id| {
            let (kind, index, fields) = &groups[id];
            let notes = by.get(id).map_or(&[][..], Vec::as_slice);
            let removed = notes.iter().find(|n| n.status == "remove");
            let ok_mark = notes
                .iter()
                .any(|x| x.status == "ok" && split_field(x).1.starts_with('*'));
            json!({"unit": id, "kind": kind, "index": index, "status": unit_status(notes), "ok_mark": ok_mark,
                   "remove_comment": removed.map(|n| n.comment.clone()), "fields": fields})
        })
        .collect();
    let title = subjects::subjects()
        .into_iter()
        .flat_map(|s| s.skills)
        .find(|k| k.id == skill)
        .map_or(String::new(), |k| k.title.to_string());
    Ok(
        json!({"skill": skill, "title": title, "review": subjects::review_status(skill), "units": units}),
    )
}

fn field_index(args: &Value) -> u32 {
    args.get("index")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(0)
}

const FULL: &str = "Забагато позначок: надішліть файл розробникові й почніть із наступних тем";

fn store_note(state: &mut AppState, note: ReviewNote) -> Result<(), ApiError> {
    let notes = &mut state.teacher.notes;
    if let Some(n) = notes
        .iter_mut()
        .find(|n| n.skill == note.skill && n.field == note.field && n.index == note.index)
    {
        *n = note;
        return Ok(());
    }
    if notes.len() >= MAX_REVIEW_NOTES {
        return Err(ApiError::bad_request(FULL));
    }
    notes.push(note);
    Ok(())
}

/// Прибирає застарілі позначки теми (зміст у пакеті вже інший).
fn prune_stale(state: &mut AppState, skill: &str) {
    let Some(rows) = rows_of(skill) else {
        return;
    };
    state
        .teacher
        .notes
        .retain(|n| n.skill != skill || is_current(n, &rows));
}

/// Зберігає (або замінює) зауваження до тексту чи позначку: «усе гаразд» (`field` = `*`, `*:title`, `*:notes`),
/// виправлення (`field` = поле тексту) або «прибрати завдання» (`field` = `prompt`, зберігається як `remove`).
/// Текст береться з пакета, а не від користувача.
pub(crate) fn note_set(state: &mut AppState, args: &Value) -> Result<Value, ApiError> {
    let get = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("");
    let (skill, kind, field) = (get("skill"), get("kind"), get("field"));
    let index = field_index(args);
    let status = get("status");
    if !["ok", "fix", "remove"].contains(&status) {
        return Err(ApiError::bad_request("Статус має бути ok, fix або remove"));
    }
    let rows = rows_of(skill).ok_or_else(|| ApiError::bad_request("Невідома тема"))?;
    let missing = || ApiError::bad_request("Такого тексту в темі немає");
    let (store_field, text) = match status {
        "ok" => {
            if !field.starts_with('*') {
                return Err(ApiError::bad_request(
                    "Позначку «усе гаразд» ставлять на всю одиницю",
                ));
            }
            let unit = unit_id(kind, index, field);
            if !rows.iter().any(|r| unit_id(&r.0, r.1, &r.2) == unit) {
                return Err(missing());
            }
            (field.to_string(), unit_fp(&rows, &unit))
        }
        "remove" => {
            if !(["bank", "typed"].contains(&kind) && field == "prompt") {
                return Err(ApiError::bad_request("Прибрати можна лише завдання"));
            }
            let prompt = rows
                .iter()
                .find(|r| r.0 == kind && r.1 == index && r.2 == "prompt")
                .ok_or_else(missing)?;
            // У темі має лишитися хоча б одне завдання: інакше `apply` однаково залишить останнє.
            let tasks: std::collections::BTreeSet<(&str, u32)> = rows
                .iter()
                .filter(|r| r.0 == "bank" || r.0 == "typed")
                .map(|r| (r.0.as_str(), r.1))
                .collect();
            let own = notes_of(state, skill);
            let (by, _) = current_by_unit(&own, skill);
            let already = by
                .iter()
                .filter(|(u, n)| *u != &format!("{kind}:{index}") && unit_status(n) == "remove")
                .count();
            if already + 1 >= tasks.len() {
                return Err(ApiError::bad_request("Залиште в темі хоча б одне завдання"));
            }
            ("remove".to_string(), prompt.3.clone())
        }
        _ => {
            let row = rows
                .iter()
                .find(|r| r.0 == kind && r.1 == index && r.2 == field)
                .ok_or_else(missing)?;
            (field.to_string(), row.3.clone())
        }
    };
    let correction: String = get("correction").chars().take(2000).collect();
    if status == "fix" && correction.trim().is_empty() {
        return Err(ApiError::bad_request("Напишіть виправлений текст"));
    }
    prune_stale(state, skill);
    store_note(
        state,
        ReviewNote {
            skill: skill.to_string(),
            field: format!("{kind}:{store_field}"),
            index,
            text,
            status: status.to_string(),
            correction: if status == "fix" {
                correction
            } else {
                String::new()
            },
            comment: get("comment").chars().take(500).collect(),
        },
    )?;
    Ok(json!({"ok": true, "total": state.teacher.notes.len()}))
}

/// Знімає позначку (повертає текст у стан «без позначки»). Для «прибрати завдання» поле `prompt` означає `remove`.
pub(crate) fn note_clear(state: &mut AppState, args: &Value) -> Result<Value, ApiError> {
    let get = |k: &str| args.get(k).and_then(Value::as_str).unwrap_or("");
    let (kind, field) = (get("kind"), get("field"));
    let field = if field == "remove" { "remove" } else { field };
    let key = format!("{kind}:{field}");
    let (skill, index) = (get("skill"), field_index(args));
    let before = state.teacher.notes.len();
    state
        .teacher
        .notes
        .retain(|n| !(n.skill == skill && n.field == key && n.index == index));
    Ok(json!({"ok": true, "removed": before - state.teacher.notes.len()}))
}

/// Усі одиниці теми без жодної чинної позначки вважає такими, що не мають зауважень.
pub(crate) fn topic_ok(state: &mut AppState, args: &Value) -> Result<Value, ApiError> {
    let skill = args.get("skill").and_then(Value::as_str).unwrap_or("");
    let rows = rows_of(skill).ok_or_else(|| ApiError::bad_request("Невідома тема"))?;
    prune_stale(state, skill);
    let own = notes_of(state, skill);
    let (by, _) = current_by_unit(&own, skill);
    let mut todo: Vec<(String, u32, String, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (kind, index, field, _) in rows.iter() {
        let id = unit_id(kind, *index, field);
        if by.contains_key(&id) || !seen.insert(id.clone()) {
            continue;
        }
        let star = match id.as_str() {
            "lesson:title" => "*:title",
            "lesson:notes" => "*:notes",
            _ => "*",
        };
        todo.push((kind.clone(), *index, star.to_string(), unit_fp(&rows, &id)));
    }
    // Перевіряємо місце наперед, щоб не лишити частково додані позначки.
    if state.teacher.notes.len() + todo.len() > MAX_REVIEW_NOTES {
        return Err(ApiError::bad_request(FULL));
    }
    let count = todo.len();
    for (kind, index, field, fp) in todo {
        store_note(
            state,
            ReviewNote {
                skill: skill.to_string(),
                field: format!("{kind}:{field}"),
                index,
                text: fp,
                status: "ok".to_string(),
                correction: String::new(),
                comment: String::new(),
            },
        )?;
    }
    Ok(json!({"ok": true, "marked": count}))
}

/// Транслітерація імені для назви файлу (проста, лише для файлової системи).
fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.to_lowercase().chars() {
        let t = match c {
            'а' => "a",
            'б' => "b",
            'в' => "v",
            'г' => "h",
            'ґ' => "g",
            'д' => "d",
            'е' => "e",
            'є' => "ie",
            'ж' => "zh",
            'з' => "z",
            'и' => "y",
            'і' => "i",
            'ї' => "i",
            'й' => "i",
            'к' => "k",
            'л' => "l",
            'м' => "m",
            'н' => "n",
            'о' => "o",
            'п' => "p",
            'р' => "r",
            'с' => "s",
            'т' => "t",
            'у' => "u",
            'ф' => "f",
            'х' => "kh",
            'ц' => "ts",
            'ч' => "ch",
            'ш' => "sh",
            'щ' => "shch",
            'ю' => "iu",
            'я' => "ia",
            'ь' | '\'' | '’' | 'ʼ' => "",
            c if c.is_ascii_alphanumeric() => {
                s.push(c);
                continue;
            }
            _ => "-",
        };
        s.push_str(t);
    }
    let s = s
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if s.is_empty() {
        "metodyst".to_string()
    } else {
        s.chars().take(24).collect()
    }
}

/// CSV для `content-tools apply` (`fix` і `remove` застосовуються, `ok` це підтвердження перевірки) і ім'я файлу.
pub(crate) fn export(
    state: &AppState,
    only_skill: Option<&str>,
    ctx: &Ctx,
) -> Result<(String, String), ApiError> {
    let notes: Vec<&ReviewNote> = state
        .teacher
        .notes
        .iter()
        .filter(|n| only_skill.map_or(true, |s| n.skill == s))
        .collect();
    let pack_of: HashMap<&str, &str> = subjects::subjects()
        .into_iter()
        .flat_map(|s| {
            s.skills
                .iter()
                .map(move |k| (k.id, s.pack))
                .collect::<Vec<_>>()
        })
        .collect();
    let mut rows = Vec::new();
    for n in notes {
        let (Some(pack), Some(cur)) = (pack_of.get(n.skill.as_str()), rows_of(&n.skill)) else {
            continue;
        };
        if !is_current(n, &cur) {
            continue; // зміст уже інший: застаріла позначка у файл не потрапляє
        }
        let (kind, field) = split_field(n);
        // «Прибрати завдання» у файлі це `remove` на полі `prompt`; «усе гаразд» не має тексту.
        let (field, text) = match field {
            "remove" => ("prompt", n.text.clone()),
            f if f.starts_with('*') => (f, String::new()),
            f => (f, n.text.clone()),
        };
        rows.push(vec![
            (*pack).to_string(),
            n.skill.clone(),
            kind.to_string(),
            n.index.to_string(),
            field.to_string(),
            text,
            n.status.clone(),
            n.correction.clone(),
            n.comment.clone(),
        ]);
    }
    if rows.is_empty() {
        return Err(ApiError::bad_request("Ще немає позначок"));
    }
    let scope = only_skill.map_or(String::new(), |s| format!("-{}", s.to_lowercase()));
    let filename = format!(
        "vychytka-{}{}-{}.csv",
        slug(&state.profile.nickname),
        scope,
        date_of(ctx.day)
    );
    Ok((content_tools::review::to_csv(rows), filename))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_slugs_are_right() {
        assert_eq!(slug("Оксана Петрівна"), "oksana-petrivna");
        assert_eq!(slug("  Ґанна-Їжак! "), "ganna-izhak");
        assert_eq!(slug("???"), "metodyst");
    }

    #[test]
    fn unit_ids_separate_the_lesson_title_notes_and_blocks() {
        assert_eq!(unit_id("lesson", 0, "title"), "lesson:title");
        assert_eq!(unit_id("lesson", 0, "note_prompt:2"), "lesson:notes");
        assert_eq!(unit_id("lesson", 0, "text"), "lesson:0");
        assert_eq!(unit_id("bank", 7, "wrong:1"), "bank:7");
        assert_eq!(unit_id("lesson", 0, "*:title"), "lesson:title");
        assert_eq!(unit_id("bank", 2, "remove"), "bank:2");
    }
}

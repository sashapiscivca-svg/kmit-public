//! Звіт для методиста й застосування його виправлень.
//!
//! Кожен текстовий елемент пакета це один рядок CSV: `pack, skill, kind, index, field, text, status, correction, comment`.
//! Методист заповнює `status` (`ok` / `fix` / `remove`), `correction` (новий текст) і `comment`. `apply` застосовує
//! лише рядки `fix` (і `remove` для завдань), перевіряючи, що `text` досі збігається з поточним (захист від застарілого звіту).

use crate::csv;
use serde_json::{json, Value};

pub const HEADER: [&str; 9] = [
    "pack",
    "skill",
    "kind",
    "index",
    "field",
    "text",
    "status",
    "correction",
    "comment",
];

fn row(pack: &str, skill: &str, kind: &str, index: usize, field: &str, text: &str) -> Vec<String> {
    vec![
        pack.into(),
        skill.into(),
        kind.into(),
        index.to_string(),
        field.into(),
        text.into(),
        String::new(),
        String::new(),
        String::new(),
    ]
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Усі текстові поля пакета (як JSON `Value`) у вигляді рядків звіту.
pub fn rows_for(pack: &Value) -> Vec<Vec<String>> {
    let id = pack["id"].as_str().unwrap_or("");
    let mut out = Vec::new();
    for s in pack["skills"].as_array().into_iter().flatten() {
        out.push(row(
            id,
            s["id"].as_str().unwrap_or(""),
            "skill",
            0,
            "title",
            s["title"].as_str().unwrap_or(""),
        ));
    }
    for (kind, key) in [("bank", "banks"), ("typed", "typed")] {
        for b in pack[key].as_array().into_iter().flatten() {
            let skill = b["skill"].as_str().unwrap_or("");
            for (i, it) in b["items"].as_array().into_iter().flatten().enumerate() {
                out.push(row(
                    id,
                    skill,
                    kind,
                    i,
                    "prompt",
                    it["prompt"].as_str().unwrap_or(""),
                ));
                if kind == "bank" {
                    out.push(row(
                        id,
                        skill,
                        kind,
                        i,
                        "correct",
                        it["correct"].as_str().unwrap_or(""),
                    ));
                    for (k, w) in strings(&it["wrong"]).iter().enumerate() {
                        out.push(row(id, skill, kind, i, &format!("wrong:{k}"), w));
                    }
                } else {
                    for (k, a) in strings(&it["accepted"]).iter().enumerate() {
                        out.push(row(id, skill, kind, i, &format!("accepted:{k}"), a));
                    }
                }
                out.push(row(
                    id,
                    skill,
                    kind,
                    i,
                    "explanation",
                    it["explanation"].as_str().unwrap_or(""),
                ));
                for (k, h) in strings(&it["hints"]).iter().enumerate() {
                    out.push(row(id, skill, kind, i, &format!("hint:{k}"), h));
                }
            }
        }
    }
    for l in pack["lessons"].as_array().into_iter().flatten() {
        let skill = l["skill"].as_str().unwrap_or("");
        out.push(row(
            id,
            skill,
            "lesson",
            0,
            "title",
            l["title"].as_str().unwrap_or(""),
        ));
        for (i, b) in l["blocks"].as_array().into_iter().flatten().enumerate() {
            for field in [
                "text",
                "statement",
                "result",
                "prompt",
                "answer",
                "explanation",
                "caption",
            ] {
                if let Some(t) = b[field].as_str() {
                    out.push(row(id, skill, "lesson", i, field, t));
                }
            }
            for (k, s) in strings(&b["steps"]).iter().enumerate() {
                out.push(row(id, skill, "lesson", i, &format!("step:{k}"), s));
            }
            for (k, s) in strings(&b["options"]).iter().enumerate() {
                out.push(row(id, skill, "lesson", i, &format!("option:{k}"), s));
            }
        }
        for (k, p) in strings(&l["note_prompts"]).iter().enumerate() {
            out.push(row(id, skill, "lesson", 0, &format!("note_prompt:{k}"), p));
        }
    }
    out
}

pub fn to_csv(rows: Vec<Vec<String>>) -> String {
    let mut all = vec![HEADER.iter().map(|s| (*s).to_string()).collect::<Vec<_>>()];
    all.extend(rows);
    csv::write(&all)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Зручний для читання й друку HTML-звіт (без скриптів): за темами, із позначкою правильної відповіді.
pub fn to_html(pack: &Value) -> String {
    let mut h = String::new();
    h.push_str("<!doctype html><html lang=\"uk\"><meta charset=\"utf-8\"><title>Звіт для методиста</title>");
    h.push_str("<style>body{font:15px/1.5 system-ui,sans-serif;max-width:60rem;margin:2rem auto;padding:0 1rem}h2{border-bottom:2px solid #333;margin-top:2.5rem}h3{margin-bottom:.2rem}.q{margin:.8rem 0;padding:.5rem .8rem;border:1px solid #bbb;border-radius:6px;break-inside:avoid}.ok{color:#0a6b3b;font-weight:700}li{margin:.1rem 0}.m{color:#666;font-size:.85em}</style>");
    h.push_str(&format!("<h1>{}</h1><p class=\"m\">id: {}, версія {}. Позначай кожне завдання: ✓ добре / ✎ виправити (напиши) / ✗ видалити.</p>", esc(pack["title"].as_str().unwrap_or("")), esc(pack["id"].as_str().unwrap_or("")), pack["version"]));
    for s in pack["skills"].as_array().into_iter().flatten() {
        let sid = s["id"].as_str().unwrap_or("");
        h.push_str(&format!(
            "<h2>{} <span class=\"m\">{}</span></h2>",
            esc(s["title"].as_str().unwrap_or("")),
            esc(sid)
        ));
        for l in pack["lessons"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|l| l["skill"] == sid)
        {
            h.push_str("<h3>Урок</h3>");
            for b in l["blocks"].as_array().into_iter().flatten() {
                match b["type"].as_str().unwrap_or("") {
                    "text" | "remember" => h.push_str(&format!("<p>{}</p>", esc(b["text"].as_str().unwrap_or("")))),
                    "example" => h.push_str(&format!("<div class=\"q\"><b>{}</b><ol>{}</ol>= {}</div>", esc(b["statement"].as_str().unwrap_or("")), strings(&b["steps"]).iter().map(|x| format!("<li>{}</li>", esc(x))).collect::<String>(), esc(b["result"].as_str().unwrap_or("")))),
                    "quiz" => {
                        let c = b["correct"].as_u64().unwrap_or(0) as usize;
                        h.push_str(&format!("<div class=\"q\"><b>{}</b><ul>{}</ul><i>{}</i></div>", esc(b["prompt"].as_str().unwrap_or("")), strings(&b["options"]).iter().enumerate().map(|(i, x)| format!("<li{}>{}</li>", if i == c { " class=\"ok\"" } else { "" }, esc(x))).collect::<String>(), esc(b["explanation"].as_str().unwrap_or(""))));
                    }
                    "selfcheck" => h.push_str(&format!("<div class=\"q\"><b>{}</b> <span class=\"ok\">{}</span><br><i>{}</i></div>", esc(b["prompt"].as_str().unwrap_or("")), esc(b["answer"].as_str().unwrap_or("")), esc(b["explanation"].as_str().unwrap_or("")))),
                    _ => {}
                }
            }
        }
        for b in pack["banks"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["skill"] == sid)
        {
            h.push_str("<h3>Завдання з вибором</h3>");
            for (i, it) in b["items"].as_array().into_iter().flatten().enumerate() {
                h.push_str(&format!("<div class=\"q\"><b>{}. {}</b><ul><li class=\"ok\">{}</li>{}</ul><i>{}</i><div class=\"m\">підказки: {}</div></div>", i + 1, esc(it["prompt"].as_str().unwrap_or("")), esc(it["correct"].as_str().unwrap_or("")), strings(&it["wrong"]).iter().map(|x| format!("<li>{}</li>", esc(x))).collect::<String>(), esc(it["explanation"].as_str().unwrap_or("")), esc(&strings(&it["hints"]).join(" / "))));
            }
        }
        for b in pack["typed"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|b| b["skill"] == sid)
        {
            h.push_str("<h3>Завдання з введенням слова</h3>");
            for (i, it) in b["items"].as_array().into_iter().flatten().enumerate() {
                h.push_str(&format!("<div class=\"q\"><b>{}. {}</b><br>приймається: <span class=\"ok\">{}</span><br><i>{}</i><div class=\"m\">підказки: {}</div></div>", i + 1, esc(it["prompt"].as_str().unwrap_or("")), esc(&strings(&it["accepted"]).join(" | ")), esc(it["explanation"].as_str().unwrap_or("")), esc(&strings(&it["hints"]).join(" / "))));
            }
        }
    }
    h
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub fixed: usize,
    pub removed: usize,
    pub skipped: Vec<String>,
}

fn target<'a>(
    pack: &'a mut Value,
    skill: &str,
    kind: &str,
    index: usize,
    field: &str,
) -> Option<&'a mut Value> {
    let (name, sub) = field
        .split_once(':')
        .map_or((field, None), |(a, b)| (a, b.parse::<usize>().ok()));
    match kind {
        "skill" => pack["skills"]
            .as_array_mut()?
            .iter_mut()
            .find(|s| s["id"] == skill)?
            .get_mut(name),
        "bank" | "typed" => {
            let key = if kind == "bank" { "banks" } else { "typed" };
            let item = pack[key]
                .as_array_mut()?
                .iter_mut()
                .find(|b| b["skill"] == skill)?["items"]
                .as_array_mut()?
                .get_mut(index)?;
            // У звіті поле підказки зветься `hint:k`, а в пакеті масив `hints`.
            let key = if name == "hint" { "hints" } else { name };
            match sub {
                Some(k) => item.get_mut(key)?.as_array_mut()?.get_mut(k),
                None => item.get_mut(key),
            }
        }
        "lesson" => {
            let lesson = pack["lessons"]
                .as_array_mut()?
                .iter_mut()
                .find(|l| l["skill"] == skill)?;
            if name == "title" {
                return lesson.get_mut("title");
            }
            if name == "note_prompt" {
                return lesson
                    .get_mut("note_prompts")?
                    .as_array_mut()?
                    .get_mut(sub?);
            }
            let block = lesson["blocks"].as_array_mut()?.get_mut(index)?;
            match sub {
                Some(k) => block
                    .get_mut(if name == "step" { "steps" } else { "options" })?
                    .as_array_mut()?
                    .get_mut(k),
                None => block.get_mut(name),
            }
        }
        _ => None,
    }
}

/// Застосовує виправлення до пакетів (за id). Повертає звіт; змінені пакети мають збільшену версію.
pub fn apply(packs: &mut [Value], csv_text: &str) -> Applied {
    let rows = csv::read(csv_text);
    let mut res = Applied::default();
    let mut changed = std::collections::BTreeSet::new();
    let mut removals: Vec<(String, String, String, usize)> = Vec::new();
    for (n, r) in rows.iter().enumerate().skip(1) {
        if r.len() < 8 {
            continue;
        }
        let (pack, skill, kind, idx, field, text, status, corr) = (
            &r[0],
            &r[1],
            &r[2],
            &r[3],
            &r[4],
            &r[5],
            r[6].trim().to_lowercase(),
            &r[7],
        );
        let line = n + 1;
        let Ok(index) = idx.parse::<usize>() else {
            continue;
        };
        match status.as_str() {
            "fix" if !corr.trim().is_empty() => {
                let Some(p) = packs.iter_mut().find(|p| p["id"] == pack.as_str()) else {
                    res.skipped
                        .push(format!("рядок {line}: невідомий пакет {pack}"));
                    continue;
                };
                match target(p, skill, kind, index, field) {
                    Some(v) if v.as_str() == Some(text.as_str()) => {
                        *v = json!(corr.trim());
                        res.fixed += 1;
                        changed.insert(pack.clone());
                    }
                    Some(_) => res.skipped.push(format!(
                        "рядок {line}: текст у пакеті вже інший, звіт застарів ({skill} {field})"
                    )),
                    None => res.skipped.push(format!(
                        "рядок {line}: поле не знайдено ({skill} {kind} {index} {field})"
                    )),
                }
            }
            "remove" if (kind == "bank" || kind == "typed") && field == "prompt" => {
                removals.push((pack.clone(), skill.clone(), kind.clone(), index));
            }
            _ => {}
        }
    }
    // Видалення з кінця, щоб не збивати індекси; перевіряємо, що умова збігається з тим, що бачив методист.
    removals.sort_by_key(|b| std::cmp::Reverse(b.3));
    for (pack, skill, kind, index) in removals {
        let Some(p) = packs.iter_mut().find(|p| p["id"] == pack.as_str()) else {
            continue;
        };
        let key = if kind == "bank" { "banks" } else { "typed" };
        let Some(items) = p[key]
            .as_array_mut()
            .and_then(|b| b.iter_mut().find(|b| b["skill"] == skill.as_str()))
            .and_then(|b| b["items"].as_array_mut())
        else {
            continue;
        };
        let expected = rows
            .iter()
            .find(|r| {
                r.len() >= 6
                    && r[0] == pack
                    && r[1] == skill
                    && r[2] == kind
                    && r[3] == index.to_string()
                    && r[4] == "prompt"
            })
            .map(|r| r[5].clone());
        if index < items.len()
            && items.len() > 1
            && expected.as_deref() == items[index]["prompt"].as_str()
        {
            items.remove(index);
            res.removed += 1;
            changed.insert(pack);
        } else {
            res.skipped.push(format!(
                "видалення {skill} №{index}: не збігається умова або остання в темі"
            ));
        }
    }
    for p in packs.iter_mut() {
        if changed.contains(p["id"].as_str().unwrap_or("")) {
            let v = p["version"].as_u64().unwrap_or(1) + 1;
            p["version"] = json!(v);
        }
    }
    res
}

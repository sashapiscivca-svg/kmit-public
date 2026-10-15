//! Серверний прогрес уроку: UI не може його обійти. Ключі в `AppState.lessons`:
//! `<тема>` маска пройдених контрольних питань, `<тема>#ex` маска зрозумілих прикладів, `<тема>#xp` маска зданих пояснень.

use serde_json::{json, Value};
use storage::state::AppState;
use tutor_core::lesson::{Lesson, LessonBlock};

/// (контрольні питання, приклади, пояснення «своїми словами»).
pub(crate) fn counts(l: &Lesson) -> (u32, u32, u32) {
    let mut c = (0, 0, 0);
    for b in &l.blocks {
        match b {
            LessonBlock::SelfCheck { .. } | LessonBlock::Quiz { .. } => c.0 += 1,
            LessonBlock::Example { .. } => c.1 += 1,
            LessonBlock::Explain { .. } => c.2 += 1,
            _ => {}
        }
    }
    c
}

pub(crate) fn full_mask(n: u32) -> u32 {
    if n >= 32 {
        u32::MAX
    } else {
        (1u32 << n) - 1
    }
}

fn mask(st: &AppState, key: &str) -> u32 {
    st.lessons.get(key).copied().unwrap_or(0)
}

/// Урок пройдено, лише якщо всі приклади зрозумілі, всі контрольні питання розв'язані самостійно й усі пояснення здано.
pub(crate) fn is_complete(l: &Lesson, id: &str, st: &AppState) -> bool {
    let (c, e, x) = counts(l);
    if c + e + x == 0 {
        return false;
    }
    let ok = |n: u32, key: &str| {
        let f = full_mask(n);
        mask(st, key) & f == f
    };
    ok(c, id) && ok(e, &format!("{id}#ex")) && ok(x, &format!("{id}#xp"))
}

pub(crate) fn progress_json(l: &Lesson, id: &str, st: &AppState) -> Value {
    let (c, e, x) = counts(l);
    json!({
        "checks": mask(st, id) & full_mask(c),
        "examples": mask(st, &format!("{id}#ex")) & full_mask(e),
        "explains": mask(st, &format!("{id}#xp")) & full_mask(x),
        "complete": is_complete(l, id, st),
    })
}

/// Справжній текст, а не набір літер: не менше `words` різних слів і `chars` символів із літерами.
pub(crate) fn is_real_text(text: &str, words: usize, chars: usize) -> bool {
    let mut set = std::collections::BTreeSet::new();
    let mut letters = 0;
    for w in text.split(|c: char| !c.is_alphabetic() && c != '\'' && c != '’') {
        let w = w.to_lowercase();
        let n = w.chars().count();
        if n >= 2 && w.chars().collect::<std::collections::BTreeSet<_>>().len() >= 2 {
            letters += n;
            set.insert(w);
        }
    }
    set.len() >= words && letters >= chars
}

/// Конспект зберігається як розділи: `§Мітка\nтекст`, розділи розділені порожнім рядком. Старі конспекти без `§` це один розділ.
pub(crate) fn note_fields(note: &str) -> Vec<(String, String)> {
    if !note.starts_with('§') {
        return if note.trim().is_empty() {
            Vec::new()
        } else {
            vec![(String::new(), note.trim().to_string())]
        };
    }
    note.split("\n§")
        .map(|p| p.trim_start_matches('§'))
        .filter_map(|p| {
            let (label, text) = p.split_once('\n').unwrap_or((p, ""));
            let text = text.trim();
            (!text.is_empty()).then(|| (label.trim().to_string(), text.to_string()))
        })
        .collect()
}

pub(crate) fn note_join(fields: &[(String, String)]) -> String {
    if fields.len() == 1 && fields[0].0.is_empty() {
        return fields[0].1.clone();
    }
    fields
        .iter()
        .map(|(l, t)| format!("§{l}\n{t}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Лише слова самого учня (без міток розділів): за ним рахується відзнака.
pub(crate) fn note_body(note: &str) -> String {
    note_fields(note)
        .into_iter()
        .map(|(_, t)| t)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Для PDF та перегляду: мітка на окремому рядку.
pub(crate) fn note_readable(note: &str) -> String {
    note_fields(note)
        .into_iter()
        .map(|(l, t)| if l.is_empty() { t } else { format!("{l}\n{t}") })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Конспект «справжній»: достатньо різних слів власного тексту.
pub(crate) fn note_is_real(note: &str) -> bool {
    let body = note_body(note);
    body.chars().count() >= 80 && is_real_text(&body, 12, 60)
}

//! «Згадай і поясни»: рознесене повторення вивчених тем через пригадування. Дитина спершу пише своїми словами,
//! лише потім бачить ключові ідеї, звіряє й оцінює себе. Правильної чи хибної відповіді застосунок не оцінює
//! (відповідь дитини нікуди не зберігається), а розкладає повторення за інтервалами, як це роблять із завданнями.
//! Дані: лише рівень і день наступного повторення на тему (ключ `<тема>#rc` у `AppState.lessons`).

use crate::app::Env;
use crate::lesson_prog;
use serde_json::{json, Value};
use storage::state::AppState;
use tutor_core::lesson::{Lesson, LessonBlock, LessonSource};

/// Дні до наступного повторення за рівнем: 1, 3, 7, 14, 30.
const GAPS: [u32; 5] = [1, 3, 7, 14, 30];
const COUNT_KEY: &str = "#recalls";

fn key(id: &str) -> String {
    format!("{id}#rc")
}

fn pack(level: u32, due: u32) -> u32 {
    (due << 3) | level.min(4)
}

fn unpack(v: u32) -> (u32, u32) {
    (v & 7, v >> 3)
}

/// Картка: запитання на пригадування й ключові ідеї, які дитина побачить після власної спроби.
pub(crate) struct Card {
    pub prompt: String,
    pub ideas: Vec<String>,
}

pub(crate) fn card_for(env: &Env, id: &str) -> Option<Card> {
    let l: Lesson = env.lessons.lesson(id)?;
    let explain = l.blocks.iter().find_map(|b| match b {
        LessonBlock::Explain { prompt, key_ideas } => Some(Card {
            prompt: prompt.clone(),
            ideas: key_ideas.clone(),
        }),
        _ => None,
    });
    explain.or_else(|| {
        l.blocks.iter().find_map(|b| match b {
            LessonBlock::Remember(t) => Some(Card {
                prompt: format!(
                    "Згадай головне з теми «{}» і поясни своїми словами.",
                    l.title
                ),
                ideas: vec![t.clone()],
            }),
            _ => None,
        })
    })
}

fn eligible(env: &Env, st: &AppState, id: &str) -> bool {
    let done = env
        .lessons
        .lesson(id)
        .is_some_and(|l| lesson_prog::is_complete(&l, id, st));
    (done || st.learner.is_mastered(id)) && card_for(env, id).is_some()
}

/// Заводить повторення для вивчених тем, яких ще немає в черзі: перший раз завтра.
pub(crate) fn seed(env: &Env, st: &mut AppState, today: u32) {
    for id in env.graph.topological_order().unwrap_or_default() {
        if !st.lessons.contains_key(&key(id)) && eligible(env, st, id) {
            st.lessons.insert(key(id), pack(0, today + GAPS[0]));
        }
    }
}

pub(crate) fn due_skills(env: &Env, st: &AppState, today: u32) -> Vec<String> {
    env.graph
        .topological_order()
        .unwrap_or_default()
        .into_iter()
        .filter(|id| {
            st.lessons
                .get(&key(id))
                .is_some_and(|v| unpack(*v).1 <= today)
        })
        .map(String::from)
        .collect()
}

pub(crate) fn queue_json(env: &Env, st: &mut AppState, today: u32) -> Value {
    seed(env, st, today);
    let cards: Vec<Value> = due_skills(env, st, today)
        .into_iter()
        .filter_map(|id| {
            let c = card_for(env, &id)?;
            let level = st.lessons.get(&key(&id)).map_or(0, |v| unpack(*v).0);
            Some(json!({
                "skill": id, "title": crate::views::skill_title(env, &id),
                "prompt": c.prompt, "level": level,
                "subject": crate::catalog::subject_of(&id),
            }))
        })
        .take(10)
        .collect();
    json!({"cards": cards, "done_total": st.lessons.get(COUNT_KEY).copied().unwrap_or(0)})
}

/// Крок 2: після власної спроби (справжній текст) показуються ключові ідеї.
pub(crate) fn show(env: &Env, id: &str, text: &str) -> Value {
    let Some(c) = card_for(env, id) else {
        return json!({"accepted": false, "message": "Для цієї теми ще немає картки."});
    };
    if !lesson_prog::is_real_text(text, 5, 25) {
        return json!({"accepted": false,
            "message": "Спробуй спочатку написати своїми словами хоча б кілька слів, потім я покажу головні ідеї."});
    }
    json!({"accepted": true, "key_ideas": c.ideas})
}

/// Крок 3: самооцінка «згадав(ла) / частково / ні» розкладає наступне повторення.
/// Повертає (нова відзнака можлива) = кількість успішних повторень.
pub(crate) fn rate(st: &mut AppState, id: &str, rating: &str, today: u32) -> Option<u32> {
    let (level, _) = st.lessons.get(&key(id)).map_or((0, 0), |v| unpack(*v));
    let (nl, gap) = match rating {
        "yes" => ((level + 1).min(4), GAPS[(level as usize + 1).min(4)]),
        "part" => (level, GAPS[level as usize].max(2)),
        "no" => (0, 1),
        _ => return None,
    };
    st.lessons.insert(key(id), pack(nl, today + gap));
    if rating != "no" {
        let n = st.lessons.entry(COUNT_KEY.to_string()).or_insert(0);
        *n += 1;
    }
    Some(st.lessons.get(COUNT_KEY).copied().unwrap_or(0))
}

pub(crate) fn recall_count(st: &AppState) -> u32 {
    st.lessons.get(COUNT_KEY).copied().unwrap_or(0)
}

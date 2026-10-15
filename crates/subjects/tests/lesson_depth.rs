//! Глибина уроків: достатньо пояснень, прикладів, контрольних питань, понять, схем і завдання «поясни своїми словами».
//! Теми зі списку `lesson_depth_pending.txt` ще чекають збагачення (агенти й автори прибирають їх зі списку, коли урок готовий).
//! `DEPTH_LIST=1 cargo test -p subjects --test lesson_depth -- --nocapture` друкує теми, що не проходять.

use std::collections::BTreeSet;
use subjects::SubjectLessons;
use tutor_core::lesson::{Lesson, LessonBlock, LessonSource};

fn pending() -> BTreeSet<String> {
    include_str!("lesson_depth_pending.txt")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

/// Перелік вад уроку (порожній, якщо урок достатньо глибокий).
fn flaws(l: &Lesson) -> Vec<String> {
    let mut texts = 0;
    let (mut ex, mut checks, mut explains, mut hooks, mut terms, mut figs, mut rem) =
        (0, 0, 0, 0, 0, 0, 0);
    let mut words = 0;
    for b in &l.blocks {
        match b {
            LessonBlock::Text(t) => {
                texts += 1;
                words += t.split_whitespace().count();
            }
            LessonBlock::Example { steps, .. } if steps.len() >= 2 => ex += 1,
            LessonBlock::Example { .. } => {}
            LessonBlock::SelfCheck { .. } | LessonBlock::Quiz { .. } => checks += 1,
            LessonBlock::Explain { key_ideas, .. } if (2..=6).contains(&key_ideas.len()) => {
                explains += 1
            }
            LessonBlock::Explain { .. } => {}
            LessonBlock::Hook(_) => hooks += 1,
            LessonBlock::Term { .. } => terms += 1,
            LessonBlock::Figure { .. } => figs += 1,
            LessonBlock::Remember(_) => rem += 1,
        }
    }
    let mut v = Vec::new();
    let mut need = |ok: bool, what: &str| {
        if !ok {
            v.push(what.to_string());
        }
    };
    need(texts >= 5, "потрібно ≥ 5 абзаців пояснення");
    need(words >= 150, "потрібно ≥ 150 слів пояснення");
    need(ex >= 2, "потрібно ≥ 2 розібраних приклади");
    need(checks >= 3, "потрібно ≥ 3 контрольні питання");
    need(explains >= 1, "потрібно ≥ 1 «поясни своїми словами»");
    need(hooks >= 1, "потрібно ≥ 1 запитання «Подумай» до пояснення");
    need(terms >= 2, "потрібно ≥ 2 поняття зі словника");
    need(figs >= 1, "потрібна ≥ 1 схема чи малюнок");
    need(rem >= 1, "потрібен підсумок «Запам'ятай»");
    need(
        l.note_prompts.len() >= 3,
        "потрібно ≥ 3 запитання для конспекту",
    );
    v
}

fn all_lessons() -> Vec<Lesson> {
    let mut ids: Vec<String> = course_ids();
    ids.sort();
    ids.into_iter()
        .filter_map(|id| SubjectLessons.lesson(&id))
        .collect()
}

fn course_ids() -> Vec<String> {
    // Теми предметів і математики: уроки математики лежать у пакеті `math`, їхні теми не входять до `subjects()`.
    let mut ids: Vec<String> = subjects::subjects()
        .iter()
        .flat_map(|s| s.skills.iter().map(|k| k.id.to_string()))
        .collect();
    ids.extend(subjects::lesson_skill_ids().into_iter().map(String::from));
    ids.sort();
    ids.dedup();
    ids
}

#[test]
fn every_lesson_that_is_not_pending_is_deep_enough() {
    let pend = pending();
    let list = std::env::var("DEPTH_LIST").is_ok();
    let mut bad = Vec::new();
    for l in all_lessons() {
        let f = flaws(&l);
        if list && !f.is_empty() {
            println!("PENDING {}", l.skill);
        }
        if !f.is_empty() && !pend.contains(l.skill) {
            bad.push(format!("{}: {}", l.skill, f.join("; ")));
        }
        if f.is_empty() && pend.contains(l.skill) {
            bad.push(format!(
                "{}: урок уже достатньо глибокий, приберіть його з lesson_depth_pending.txt",
                l.skill
            ));
        }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

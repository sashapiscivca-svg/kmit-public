//! Структурні перевірки предмета «Українська мова» (те, чого немає в `content_tests.rs`).

use std::collections::{BTreeMap, BTreeSet};
use subjects::{all_banks, generator_for, subjects, SubjectLessons};
use tutor_core::generator::{ScriptedLevel, SkillGenerator};
use tutor_core::lesson::{LessonBlock, LessonSource};

/// Теми з вибором відповіді (теми із введенням слова `UA-T-*` перевіряє `typed_tests.rs`).
fn ukrainian() -> subjects::Subject {
    let mut s = subjects()
        .into_iter()
        .find(|s| s.id == "ukrainian")
        .expect("предмет ukrainian");
    s.skills.retain(|k| !k.id.starts_with("UA-T-"));
    s
}

#[test]
fn ukrainian_has_ten_to_fourteen_topics_with_ua_prefix_and_acyclic_prerequisites() {
    let s = ukrainian();
    assert!(
        (10..=14).contains(&s.skills.len()),
        "тем: {}",
        s.skills.len()
    );
    let mut order: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, k) in s.skills.iter().enumerate() {
        assert!(k.id.starts_with("UA-"), "{}", k.id);
        assert!(!k.title.trim().is_empty(), "{}", k.id);
        assert!(order.insert(k.id, i).is_none(), "дубль {}", k.id);
    }
    // Передумова має стояти в списку раніше за тему: цикл неможливий.
    for k in &s.skills {
        for p in k.prerequisites {
            assert!(order[p] < order[k.id], "{} залежить від пізнішої {p}", k.id);
        }
    }
    let roots = s
        .skills
        .iter()
        .filter(|k| k.prerequisites.is_empty())
        .count();
    assert!(roots >= 1);
}

#[test]
fn every_ukrainian_topic_has_at_least_25_items_and_three_distinct_hints_each() {
    for k in ukrainian().skills {
        let g = generator_for(k.id).unwrap();
        let bank = all_banks()
            .into_iter()
            .find(|b| b.skill_id() == k.id)
            .unwrap();
        assert!(bank.len() >= 25, "{}: лише {} завдань", k.id, bank.len());
        for v in 0..bank.len() as u64 {
            let t = g.generate(v);
            let hints: BTreeSet<String> = [ScriptedLevel::R1, ScriptedLevel::R2, ScriptedLevel::R3]
                .iter()
                .map(|l| g.hint(&t, *l))
                .collect();
            assert_eq!(
                hints.len(),
                3,
                "{}: підказки мають бути різні: «{}»",
                k.id,
                t.statement
            );
            assert!(
                hints.iter().all(|h| h.trim().chars().count() >= 10),
                "{}",
                k.id
            );
        }
    }
}

#[test]
fn lesson_examples_do_not_repeat_bank_questions_and_quizzes_are_not_bank_clones() {
    for k in ukrainian().skills {
        let g = generator_for(k.id).unwrap();
        let bank = all_banks()
            .into_iter()
            .find(|b| b.skill_id() == k.id)
            .unwrap();
        let prompts: BTreeSet<String> = (0..bank.len() as u64)
            .map(|v| g.generate(v).statement.to_lowercase())
            .collect();
        // Пари «умова + правильна відповідь» із банку.
        let pairs: BTreeSet<(String, String)> = (0..bank.len() as u64)
            .map(|v| {
                let t = g.generate(v);
                let a = g.reveal(&t).unwrap().answer_text.to_lowercase();
                (t.statement.to_lowercase(), a)
            })
            .collect();
        let lesson = SubjectLessons.lesson(k.id).unwrap();
        for b in &lesson.blocks {
            match b {
                LessonBlock::Example {
                    statement,
                    steps,
                    result,
                } => {
                    assert!(
                        !prompts.contains(&statement.to_lowercase()),
                        "{}: приклад збігається із завданням",
                        k.id
                    );
                    assert!(steps.len() >= 2 && !result.trim().is_empty(), "{}", k.id);
                }
                LessonBlock::Quiz {
                    prompt,
                    options,
                    correct,
                    ..
                } => {
                    let key = (prompt.to_lowercase(), options[*correct].to_lowercase());
                    assert!(
                        !pairs.contains(&key),
                        "{}: самоперевірка «{prompt}» дублює завдання банку",
                        k.id
                    );
                }
                _ => {}
            }
        }
        assert!(lesson.title.trim().chars().count() >= 3, "{}", k.id);
    }
}

#[test]
fn the_correct_choice_is_spread_over_all_positions() {
    for k in ukrainian().skills {
        let g = generator_for(k.id).unwrap();
        let bank = all_banks()
            .into_iter()
            .find(|b| b.skill_id() == k.id)
            .unwrap();
        let mut seen = BTreeSet::new();
        for v in 0..bank.len() as u64 {
            seen.insert(g.generate(v).answer.to_string());
        }
        assert!(
            seen.len() >= 3,
            "{}: правильна відповідь займає лише позиції {seen:?}",
            k.id
        );
    }
}

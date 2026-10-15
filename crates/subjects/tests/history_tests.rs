//! Структурні перевірки предмета «Вступ до історії України» (поверх загальних content_tests).

use std::collections::BTreeSet;
use subjects::{all_banks, subjects, SubjectLessons};
use tutor_core::generator::{ScriptedLevel, SkillGenerator};
use tutor_core::lesson::{LessonBlock, LessonSource};

fn history_skills() -> Vec<subjects::SubjectSkill> {
    subjects()
        .into_iter()
        .find(|s| s.id == "history" && s.grade == 5)
        .expect("предмет history")
        .skills
}

#[test]
fn history_grade_5_has_a_full_course_with_hi_prefix_and_an_acyclic_graph() {
    let skills = history_skills();
    assert!((20..=40).contains(&skills.len()), "тем: {}", skills.len());
    let ids: BTreeSet<&str> = skills.iter().map(|s| s.id).collect();
    assert_eq!(ids.len(), skills.len(), "ідентифікатори тем повторюються");
    for s in &skills {
        assert!(s.id.starts_with("HI-"), "{}", s.id);
        for p in s.prerequisites {
            assert!(
                ids.contains(p),
                "{}: передумова {p} з іншого предмета",
                s.id
            );
        }
    }
    // Топологічний обхід: на кожному кроці є тема, всі передумови якої вже пройдено.
    let mut done: BTreeSet<&str> = BTreeSet::new();
    while done.len() < skills.len() {
        let next: Vec<&str> = skills
            .iter()
            .filter(|s| !done.contains(s.id) && s.prerequisites.iter().all(|p| done.contains(p)))
            .map(|s| s.id)
            .collect();
        assert!(!next.is_empty(), "цикл у передумовах");
        done.extend(next);
    }
}

#[test]
fn history_banks_have_25_items_three_distinct_hints_and_ascii_apostrophes() {
    let ids: BTreeSet<&str> = history_skills().iter().map(|s| s.id).collect();
    let banks: Vec<_> = all_banks()
        .into_iter()
        .filter(|b| ids.contains(b.skill_id()))
        .collect();
    assert_eq!(banks.len(), ids.len());
    for b in &banks {
        let id = b.skill_id();
        assert!(b.len() >= 25, "{id}: лише {} завдань", b.len());
        for v in 0..b.len() as u64 {
            let t = b.generate(v);
            let choices = b.choices(&t);
            assert!(
                (3..=5).contains(&choices.len()),
                "{id}: {} варіантів",
                choices.len()
            );
            let hints: BTreeSet<String> = [ScriptedLevel::R1, ScriptedLevel::R2, ScriptedLevel::R3]
                .iter()
                .map(|l| b.hint(&t, *l))
                .collect();
            assert_eq!(
                hints.len(),
                3,
                "{id}: «{}» має менше трьох різних підказок",
                t.statement
            );
            let mut texts = vec![t.statement.clone(), b.reveal(&t).unwrap().explanation];
            texts.extend(choices);
            texts.extend(hints);
            for s in texts {
                assert!(
                    !s.contains('’') && !s.contains('ʼ') && !s.contains('"'),
                    "{id}: символ у «{s}»"
                );
                assert!(!s.contains("  "), "{id}: подвійний пробіл у «{s}»");
            }
        }
    }
}

#[test]
fn history_lessons_have_three_step_examples_and_varied_quiz_positions() {
    let mut positions = BTreeSet::new();
    for s in history_skills() {
        let l = SubjectLessons.lesson(s.id).unwrap();
        for b in &l.blocks {
            match b {
                LessonBlock::Example { steps, result, .. } => {
                    assert!(steps.len() >= 3, "{}: у прикладі менше трьох кроків", s.id);
                    assert!(!result.trim().is_empty());
                }
                LessonBlock::Quiz { correct, .. } => {
                    positions.insert(*correct);
                }
                _ => {}
            }
        }
    }
    assert!(
        positions.len() >= 3,
        "правильні варіанти уроків стоять майже завжди на одному місці"
    );
}

use std::collections::BTreeSet;
use subjects::{
    all_banks, all_generators, all_typed_banks, generator_for, subject_of, subjects, SubjectLessons,
};
use tutor_core::generator::{AnswerKind, ScriptedLevel, SkillGenerator};
use tutor_core::lesson::{LessonBlock, LessonSource};

fn all_skill_ids() -> BTreeSet<&'static str> {
    subjects()
        .iter()
        .flat_map(|s| s.skills.iter().map(|k| k.id))
        .collect()
}

#[test]
fn there_are_five_or_six_typed_topics_per_subject() {
    for (prefix, subject) in [("UA-T-", "ukrainian"), ("EN-T-", "english")] {
        let n = all_typed_banks()
            .iter()
            .filter(|b| b.skill_id().starts_with(prefix))
            .count();
        assert!((5..=6).contains(&n), "{prefix}: {n} тем");
        for b in all_typed_banks()
            .iter()
            .filter(|b| b.skill_id().starts_with(prefix))
        {
            assert_eq!(subject_of(b.skill_id()), Some(subject), "{}", b.skill_id());
        }
    }
}

#[test]
fn typed_banks_have_no_authoring_problems_and_enough_items() {
    for b in all_typed_banks() {
        let p = b.lint();
        assert!(p.is_empty(), "{}: {p:?}", b.skill_id());
        assert!(b.len() >= 25, "{}: лише {} завдань", b.skill_id(), b.len());
    }
}

#[test]
fn every_task_accepts_its_main_answer_and_rejects_misspellings() {
    for b in all_typed_banks() {
        let id = b.skill_id();
        assert_eq!(b.answer_kind(), AnswerKind::Text, "{id}");
        for v in 0..b.len() as u64 {
            let t = b.generate(v);
            let r = b.reveal(&t).unwrap();
            let main = r.answer_text.clone();
            assert!(
                r.explanation.chars().count() >= 15,
                "{id}: коротке пояснення"
            );
            assert_eq!(
                b.check_text(&t, &main),
                Some(true),
                "{id}: «{}» не приймає «{main}»",
                t.statement
            );
            // Регістр і зайві пробіли не є помилкою.
            assert_eq!(
                b.check_text(&t, &format!("  {}  ", main.to_uppercase())),
                Some(true),
                "{id}: {main}"
            );
            // Додана зайва літера не приймається.
            assert_ne!(
                b.check_text(&t, &format!("{main}q")),
                Some(true),
                "{id}: {main}"
            );
            // Вилучена літера (по черзі кожна) не приймається, якщо результат не є іншим прийнятним варіантом.
            let chars: Vec<char> = main.chars().collect();
            for i in 0..chars.len() {
                let broken: String = chars
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, c)| *c)
                    .collect();
                // Відомі пари двох правильних написань (обидва в `accepted`).
                if broken.trim().is_empty() || broken == "слабий" {
                    continue;
                }
                assert_ne!(
                    b.check_text(&t, &broken),
                    Some(true),
                    "{id}: «{broken}» приймається замість «{main}»"
                );
            }
            // Порожня відповідь не приймається.
            assert_ne!(b.check_text(&t, ""), Some(true), "{id}");
            // Підказки навідні: не більше трьох, не видають відповіді.
            for l in [ScriptedLevel::R1, ScriptedLevel::R2, ScriptedLevel::R3] {
                let h = b.hint(&t, l);
                assert!(!h.trim().is_empty(), "{id}");
                assert!(
                    !h.to_lowercase().contains(&main.to_lowercase()),
                    "{id}: підказка видає відповідь «{main}»"
                );
            }
            assert!(t.statement.chars().count() <= 220, "{id}: {}", t.statement);
        }
    }
}

#[test]
fn typed_skills_have_lessons_known_prerequisites_and_unique_ids() {
    let all = all_skill_ids();
    let typed_ids: Vec<&str> = all_typed_banks().iter().map(|b| b.skill_id()).collect();
    let unique: BTreeSet<&str> = typed_ids.iter().copied().collect();
    assert_eq!(unique.len(), typed_ids.len(), "ідентифікатори повторюються");
    let others: BTreeSet<&str> = all_banks().iter().map(|b| b.skill_id()).collect();
    for id in &typed_ids {
        assert!(!others.contains(id), "{id} збігається з іншою темою");
        assert!(all.contains(id), "{id}: немає в subjects()");
        assert!(generator_for(id).is_some(), "{id}");
        let skill = subjects()
            .iter()
            .flat_map(|s| s.skills.clone())
            .find(|k| k.id == *id)
            .unwrap();
        assert!(!skill.prerequisites.is_empty(), "{id}: немає передумов");
        for p in skill.prerequisites {
            assert!(all.contains(p), "{id}: невідома передумова {p}");
            assert!(
                !p.contains("-T-"),
                "{id}: передумова має бути звичайною темою"
            );
        }
        let l = SubjectLessons
            .lesson(id)
            .unwrap_or_else(|| panic!("немає уроку для {id}"));
        let count = |f: fn(&LessonBlock) -> bool| l.blocks.iter().filter(|b| f(b)).count();
        assert!(count(|b| matches!(b, LessonBlock::Text(_))) >= 2, "{id}");
        assert!(
            count(|b| matches!(b, LessonBlock::Example { .. })) >= 2,
            "{id}"
        );
        assert!(
            count(|b| matches!(b, LessonBlock::Quiz { .. })) >= 2,
            "{id}"
        );
        assert!(
            count(|b| matches!(b, LessonBlock::Remember(_))) >= 1,
            "{id}"
        );
        assert!(l.note_prompts.len() >= 3, "{id}");
        for b in &l.blocks {
            if let LessonBlock::Quiz {
                options, correct, ..
            } = b
            {
                assert!(options.len() >= 3 && *correct < options.len(), "{id}");
                let set: BTreeSet<_> = options.iter().collect();
                assert_eq!(set.len(), options.len(), "{id}");
            }
        }
    }
    assert!(all_generators().len() >= typed_ids.len());
}

/// Умови typed-завдань не повторюють умов наявних завдань з вибором, а вміст уроку не збігається з умовами банку.
#[test]
fn typed_prompts_do_not_repeat_choice_prompts_or_lesson_examples() {
    let mut choice: BTreeSet<String> = BTreeSet::new();
    for b in all_banks() {
        for v in 0..b.len() as u64 {
            choice.insert(b.generate(v).statement.trim().to_lowercase());
        }
    }
    for b in all_typed_banks() {
        let id = b.skill_id();
        let prompts: BTreeSet<String> = (0..b.len() as u64)
            .map(|v| b.generate(v).statement.trim().to_lowercase())
            .collect();
        assert_eq!(prompts.len(), b.len(), "{id}: умови повторюються");
        for p in &prompts {
            assert!(!choice.contains(p), "{id}: збігається з вибором: {p}");
        }
        let l = SubjectLessons.lesson(id).unwrap();
        for blk in &l.blocks {
            if let LessonBlock::Example { statement, .. } = blk {
                assert!(
                    !prompts.contains(&statement.trim().to_lowercase()),
                    "{id}: приклад уроку збігається з умовою банку"
                );
            }
        }
    }
}

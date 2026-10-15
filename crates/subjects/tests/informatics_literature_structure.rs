//! Структурні вимоги до предметів «Інформатика» (IT-) та «Українська література» (LT-), яких немає в загальних тестах:
//! префікси ідентифікаторів, кількість тем, обсяг банку (≥ 25) і зв'язність графа передумов.

use subjects::{all_banks, subjects};
use tutor_core::generator::SkillGenerator;

fn skills_of(subject: &str) -> Vec<(&'static str, &'static [&'static str])> {
    subjects()
        .into_iter()
        .find(|s| s.id == subject)
        .map(|s| s.skills.iter().map(|k| (k.id, k.prerequisites)).collect())
        .unwrap_or_default()
}

fn check(subject: &str, prefix: &str, min_topics: usize, max_topics: usize) {
    let skills = skills_of(subject);
    assert!(
        (min_topics..=max_topics).contains(&skills.len()),
        "{subject}: тем {}, очікується {min_topics}..={max_topics}",
        skills.len()
    );
    let ids: Vec<&str> = skills.iter().map(|s| s.0).collect();
    for (id, prereqs) in &skills {
        assert!(id.starts_with(prefix), "{id}: префікс має бути {prefix}");
        for p in *prereqs {
            // Передумови лише з цього ж предмета й раніше в переліку: граф без циклів.
            let pos = ids.iter().position(|x| x == p);
            let me = ids.iter().position(|x| x == id);
            assert!(
                pos.is_some() && pos < me,
                "{id}: передумова {p} має бути раніше"
            );
        }
    }
    for b in all_banks()
        .iter()
        .filter(|b| b.skill_id().starts_with(prefix))
    {
        assert!(
            b.len() >= 25,
            "{}: лише {} завдань, потрібно ≥ 25",
            b.skill_id(),
            b.len()
        );
    }
}

#[test]
fn informatics_is_structurally_complete() {
    check("informatics", "IT-", 8, 10);
}

#[test]
fn literature_is_structurally_complete() {
    check("literature", "LT-", 6, 8);
}

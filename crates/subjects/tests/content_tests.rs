use std::collections::BTreeSet;
use subjects::{all_banks, all_generators, generator_for, subject_of, subjects, SubjectLessons};
use tutor_core::generator::{AnswerKind, ScriptedLevel, SkillGenerator};
use tutor_core::lesson::{LessonBlock, LessonSource};
use tutor_core::task::Verdict;

#[test]
fn every_subject_skill_has_a_bank_a_lesson_and_known_prerequisites() {
    let all: BTreeSet<&str> = subjects()
        .iter()
        .flat_map(|s| s.skills.iter().map(|k| k.id))
        .collect();
    for s in subjects() {
        assert!(!s.skills.is_empty(), "{}", s.id);
        for k in &s.skills {
            assert!(generator_for(k.id).is_some(), "немає банку для {}", k.id);
            assert!(
                SubjectLessons.lesson(k.id).is_some(),
                "немає уроку для {}",
                k.id
            );
            assert_eq!(subject_of(k.id), Some(s.id));
            for p in k.prerequisites {
                assert!(all.contains(p), "{}: невідома передумова {p}", k.id);
            }
        }
    }
    assert_eq!(
        all.len(),
        all_generators().len(),
        "банк без теми чи тема без банку"
    );
}

#[test]
fn banks_have_no_authoring_problems() {
    for b in all_banks() {
        let p = b.lint();
        assert!(p.is_empty(), "{}: {p:?}", b.skill_id());
    }
}

/// Якість вмісту: обсяг, довжина текстів, і чи не видає правильну відповідь довжина варіанта (типова авторська помилка).
#[test]
fn banks_have_enough_items_sane_text_lengths_and_no_length_bias() {
    for b in all_banks() {
        let id = b.skill_id();
        let n = b.len();
        assert!(
            n >= 15,
            "{id}: лише {n} завдань, потрібно щонайменше 15 (ціль 25+)"
        );
        let mut longest = 0usize;
        for v in 1..=n as u64 {
            let t = b.generate(v);
            assert!(
                t.statement.chars().count() <= 220,
                "{id}: задовге питання «{}»",
                t.statement
            );
            let r = b.reveal(&t).unwrap();
            assert!(
                r.explanation.chars().count() >= 15,
                "{id}: надто коротке пояснення"
            );
            let ch = b.choices(&t);
            for o in &ch {
                assert!(o.chars().count() <= 90, "{id}: задовгий варіант «{o}»");
            }
            let max = ch.iter().map(|o| o.chars().count()).max().unwrap();
            let correct_len = r.answer_text.chars().count();
            if correct_len == max && ch.iter().filter(|o| o.chars().count() == max).count() == 1 {
                longest += 1;
            }
        }
        // Якщо правильний варіант майже завжди найдовший, учень вгадує за довжиною.
        assert!(
            longest * 100 <= n * 55 + 100,
            "{id}: правильна відповідь найдовша у {longest} із {n}"
        );
    }
}

#[test]
fn banks_pass_the_authoring_lint_have_enough_items_and_behave() {
    for g in all_generators()
        .into_iter()
        .filter(|g| g.answer_kind() == AnswerKind::Choice)
    {
        assert_eq!(g.answer_kind(), AnswerKind::Choice);
        let id = g.skill_id();
        let mut distinct = BTreeSet::new();
        for v in 1..=300u64 {
            let t = g.generate(v);
            distinct.insert(t.statement.clone() + &g.reveal(&t).unwrap().answer_text);
            let ch = g.choices(&t);
            assert!(ch.len() >= 3, "{id}");
            let pos = t.answer.to_string();
            assert_eq!(t.check(&pos), Verdict::Correct);
            let r = g.reveal(&t).unwrap();
            assert_eq!(ch[pos.parse::<usize>().unwrap() - 1], r.answer_text);
            for l in [ScriptedLevel::R1, ScriptedLevel::R2, ScriptedLevel::R3] {
                let h = g.hint(&t, l);
                assert!(
                    !tutor_core::bank::hint_leaks(&h, &r.answer_text),
                    "{id}: підказка видає відповідь"
                );
            }
        }
        assert!(
            distinct.len() >= 12,
            "{id}: замало різних завдань: {}",
            distinct.len()
        );
    }
}

#[test]
fn lessons_are_well_formed_and_their_quizzes_are_consistent() {
    for s in subjects() {
        for k in &s.skills {
            let l = SubjectLessons.lesson(k.id).unwrap();
            let texts = l
                .blocks
                .iter()
                .filter(|b| matches!(b, LessonBlock::Text(_)))
                .count();
            let ex = l
                .blocks
                .iter()
                .filter(|b| matches!(b, LessonBlock::Example { .. }))
                .count();
            let qz: Vec<_> = l
                .blocks
                .iter()
                .filter_map(|b| {
                    if let LessonBlock::Quiz {
                        options,
                        correct,
                        explanation,
                        prompt,
                    } = b
                    {
                        Some((options, correct, explanation, prompt))
                    } else {
                        None
                    }
                })
                .collect();
            assert!(texts >= 2 && ex >= 2 && qz.len() >= 2, "{}", k.id);
            assert!(l
                .blocks
                .iter()
                .any(|b| matches!(b, LessonBlock::Remember(_))));
            assert!(l.note_prompts.len() >= 3);
            for (options, correct, explanation, prompt) in qz {
                assert!(options.len() >= 3 && *correct < options.len());
                assert!(!explanation.trim().is_empty() && !prompt.trim().is_empty());
                let set: BTreeSet<_> = options.iter().collect();
                assert_eq!(set.len(), options.len());
            }
        }
    }
}

/// Тексти для учня не мають гендерних форм минулого часу (рішення A12): без «(-ла)», «(а)» тощо.
#[test]
fn texts_have_no_gendered_forms() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/packs");
    let mut files = 0;
    for f in std::fs::read_dir(dir).unwrap().flatten() {
        let t = std::fs::read_to_string(f.path()).unwrap();
        files += 1;
        for bad in [
            "(-ла)",
            "(-ла",
            "(ла)",
            "(лась)",
            "(-лась)",
            "(-а)",
            "(а)",
            "(ка)",
            "(на)",
            "(ий)",
        ] {
            assert!(
                !t.contains(bad),
                "{:?} містить гендерну форму {bad}",
                f.file_name()
            );
        }
    }
    assert!(files >= 6, "перевірено лише {files} пакетів");
}

#[test]
fn the_interface_strings_have_no_gendered_forms_either() {
    let t = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/ui/uk.js"))
        .unwrap();
    for bad in ["(-ла)", "(ла)", "(-а)", "(а)", "(ка)"] {
        assert!(!t.contains(bad), "uk.js містить гендерну форму {bad}");
    }
}

use tutor_core::bank::{lint_items, BankGenerator, BankItem};
use tutor_core::generator::{AnswerKind, ScriptedLevel, SkillGenerator};
use tutor_core::task::Verdict;

fn items() -> Vec<BankItem> {
    (0..6)
        .map(|i| BankItem {
            prompt: format!("Питання про тему номер {i}?"),
            correct: format!("правильна відповідь {i}"),
            wrong: vec![
                format!("хибна перша {i}"),
                format!("хибна друга {i}"),
                format!("хибна третя {i}"),
            ],
            explanation: format!("Тому що правило {i} саме таке."),
            hints: vec![
                "Пригадай, про що ми читали в уроці.".to_string(),
                "Подумай, яка ознака тут головна.".to_string(),
                "Виключи варіанти, які суперечать правилу.".to_string(),
            ],
        })
        .collect()
}

fn generator() -> BankGenerator {
    BankGenerator::new("TEST-BANK", items())
}

#[test]
fn a_bank_task_is_deterministic_and_the_answer_is_the_position_of_the_correct_option() {
    let g = generator();
    assert_eq!(g.answer_kind(), AnswerKind::Choice);
    for v in 1..200u64 {
        let (a, b) = (g.generate(v), g.generate(v));
        assert_eq!(a.statement, b.statement);
        assert_eq!(a.answer, b.answer);
        let ch = g.choices(&a);
        assert_eq!(ch.len(), 4);
        let pos = usize::try_from(a.answer.to_string().parse::<i64>().unwrap()).unwrap();
        let reveal = g.reveal(&a).unwrap();
        assert_eq!(
            ch[pos - 1],
            reveal.answer_text,
            "позиція вказує на правильний варіант"
        );
        assert_eq!(a.check(&pos.to_string()), Verdict::Correct);
        let other = if pos == 1 { 2 } else { 1 };
        assert_eq!(a.check(&other.to_string()), Verdict::Wrong);
    }
}

#[test]
fn the_order_of_options_varies_across_variants_and_covers_every_position() {
    let g = generator();
    let mut seen = std::collections::BTreeSet::new();
    for v in 1..400u64 {
        seen.insert(g.generate(v).answer.to_string());
    }
    assert_eq!(
        seen.len(),
        4,
        "правильна відповідь не завжди на одному місці: {seen:?}"
    );
}

#[test]
fn every_item_is_reachable_and_statements_are_distinct() {
    let g = generator();
    let s: std::collections::BTreeSet<String> =
        (1..=100u64).map(|v| g.generate(v).statement).collect();
    assert_eq!(s.len(), 6);
}

#[test]
fn a_bank_task_has_nothing_secret_to_protect_numerically() {
    let t = generator().generate(3);
    assert!(t.protected_values().is_empty());
}

#[test]
fn scripted_hints_come_from_the_item_and_never_contain_the_answer() {
    let g = generator();
    let t = g.generate(7);
    let r = g.reveal(&t).unwrap();
    for l in [ScriptedLevel::R1, ScriptedLevel::R2, ScriptedLevel::R3] {
        let h = g.hint(&t, l);
        assert!(!h.is_empty());
        assert!(!h.to_lowercase().contains(&r.answer_text.to_lowercase()));
    }
    assert!(
        g.analog(&t, &[]).is_none(),
        "для фактів аналога немає: після спроби показується відгук"
    );
}

#[test]
fn the_lint_catches_authoring_mistakes() {
    assert!(lint_items(&items()).is_empty());
    let mut bad = items();
    bad[0].hints[1] = "Це правильна відповідь 0, до речі".to_string();
    bad[1].wrong.truncate(1);
    bad[2].wrong[0] = bad[2].correct.clone();
    bad[3].explanation.clear();
    bad[4].prompt = bad[5].prompt.clone();
    let problems = lint_items(&bad);
    assert!(problems.len() >= 5, "{problems:?}");
}

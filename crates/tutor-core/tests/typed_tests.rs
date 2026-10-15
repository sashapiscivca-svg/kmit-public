use tutor_core::bank::{lint_typed, normalize_word, TypedBankGenerator, TypedItem};
use tutor_core::figure::Figure;
use tutor_core::generator::{AnswerKind, SkillGenerator};

fn items() -> Vec<TypedItem> {
    ["м'ята", "сім'я", "п'ять"]
        .iter()
        .enumerate()
        .map(|(i, w)| TypedItem {
            prompt: format!("Запиши слово, що означає значення номер {i}."),
            accepted: vec![w.to_string()],
            explanation: format!("Тут потрібен апостроф, правило номер {i}."),
            hints: vec!["Подумай про правило.".into()],
        })
        .collect()
}

#[test]
fn typed_answers_ignore_case_spaces_and_apostrophe_variants() {
    let g = TypedBankGenerator::new("T", items());
    assert_eq!(g.answer_kind(), AnswerKind::Text);
    let t = g.generate(1); // «м'ята» не перше: variant 1 -> item 1 = «сім'я»
    assert_eq!(g.check_text(&t, "  СІМ’Я "), Some(true));
    assert_eq!(g.check_text(&t, "сімя"), Some(false));
    assert_eq!(g.reveal(&t).unwrap().answer_text, "сім'я");
    assert!(t.protected_values().is_empty());
    assert_eq!(normalize_word("  М’Я   Та "), "м'я та");
}

#[test]
fn typed_lint_catches_leaks_duplicates_and_empties() {
    assert!(lint_typed(&items()).is_empty());
    let mut bad = items();
    bad[0].hints = vec!["Це слово м'ята, до речі".into()];
    bad[1].accepted.clear();
    bad[2].prompt = bad[0].prompt.clone();
    assert!(lint_typed(&bad).len() >= 3);
}

#[test]
fn figures_report_what_they_show_and_reject_absurd_sizes() {
    assert_eq!(Figure::FractionBar { num: 3, den: 4 }.values().len(), 1);
    assert!(Figure::FractionBar { num: 3, den: 4 }.is_sane());
    assert!(!Figure::FractionBar { num: 3, den: 400 }.is_sane());
    assert!(!Figure::Angle {
        deg: 400,
        label: "a".into()
    }
    .is_sane());
    let nl = Figure::NumberLine {
        min: 0,
        max: 2,
        parts: 4,
        marks: vec![(3, "A".into())],
    };
    assert!(nl.is_sane());
    assert_eq!(nl.values()[0].to_string(), "3/4");
    assert_eq!(
        Figure::Rect {
            w: "5 см".into(),
            h: "3 см".into()
        }
        .texts()
        .len(),
        2
    );
}

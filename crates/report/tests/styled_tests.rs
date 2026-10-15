use report::styled::{styled_sheet_pdf, thanks_card_pdf};
use tutor_core::hero::{compose, Look};
use tutor_core::sheet::{AskAdultItem, Gap, SkillLine, TeacherSheet};

fn sheet(n: usize) -> TeacherSheet {
    let line = |i: usize| SkillLine {
        skill: format!("S{i}"),
        title: format!("Тема номер {i} з довгою назвою для перевірки переносу рядків у картці"),
    };
    TeacherSheet {
        subject: "Українська мова".into(),
        to_repeat: (0..2).map(line).collect(),
        nickname: "Оля".into(),
        mastered: (0..n).map(line).collect(),
        in_progress: (0..3).map(line).collect(),
        gaps: vec![Gap {
            skill: "S1".into(),
            title: "Апостроф".into(),
            hypotheses: vec!["Ставить апостроф після будь-якого приголосного".into()],
            ideas: vec!["Складіть п'ять слів із апострофом і поясніть, чому він там".into()],
        }],
        ask_adult: vec![AskAdultItem {
            skill: "S2".into(),
            title: "Коми".into(),
            statement: "Постав коми: Я купив хліб молоко і сир".into(),
        }],
        stickers: 12,
        sticker_capacity: 80,
    }
}

#[test]
fn the_styled_sheet_is_a_valid_pdf_and_paginates_long_lists() {
    let hero = compose(&Look::default());
    let short = styled_sheet_pdf(&sheet(3), "#f2b705", &hero).unwrap();
    assert!(short.starts_with(b"%PDF-1.4") && short.ends_with(b"%%EOF\n"));
    assert!(String::from_utf8_lossy(&short).contains("/Count 1"));
    let long = styled_sheet_pdf(&sheet(80), "#2f5bea", &hero).unwrap();
    assert!(
        !String::from_utf8_lossy(&long).contains("/Count 1 "),
        "довгий список має кілька сторінок"
    );
    // Порожній аркуш теж коректний.
    let empty = TeacherSheet {
        subject: String::new(),
        to_repeat: vec![],
        nickname: String::new(),
        mastered: vec![],
        in_progress: vec![],
        gaps: vec![],
        ask_adult: vec![],
        stickers: 0,
        sticker_capacity: 0,
    };
    assert!(styled_sheet_pdf(&empty, "#2f5bea", &[])
        .unwrap()
        .starts_with(b"%PDF"));
}

#[test]
fn the_thanks_card_handles_short_long_and_empty_messages() {
    let hero = compose(&Look::default());
    for text in [
        "Дякую за уроки!",
        &"Мені дуже приємно, що ви так терпляче пояснюєте. ".repeat(12),
        "",
    ] {
        let pdf = thanks_card_pdf("Вчительці математики", text, "Оля", &hero).unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4"));
    }
    assert!(thanks_card_pdf("", "x", "", &[]).is_ok());
}

#[test]
fn the_recovery_sheet_is_a_pdf_for_normal_and_long_codes() {
    use report::styled::recovery_sheet_pdf;
    let pdf = recovery_sheet_pdf("Оля", "ABCDE-12345-FGHIJ-67890").unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(recovery_sheet_pdf("", &"ABCDE-".repeat(7)).is_ok());
}

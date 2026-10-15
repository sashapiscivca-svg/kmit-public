use report::pdf::Block;
use report::sheet::{sheet_document, sheet_pdf};
use tutor_core::sheet::{AskAdultItem, Gap, SkillLine, TeacherSheet};

fn sample() -> TeacherSheet {
    TeacherSheet {
        subject: "Математика".to_string(),
        to_repeat: vec![SkillLine {
            skill: "N-ADD-MULTI".into(),
            title: "Додавання багатоцифрових чисел".into(),
        }],
        nickname: "Мирослава".to_string(),
        mastered: vec![SkillLine {
            skill: "N-ADD-MULTI".into(),
            title: "Додавання багатоцифрових чисел".into(),
        }],
        in_progress: vec![SkillLine {
            skill: "F-ADD-SAME".into(),
            title: "Додавання дробів з однаковими знаменниками".into(),
        }],
        gaps: vec![Gap {
            skill: "F-ADD-SAME".into(),
            title: "Додавання дробів з однаковими знаменниками".into(),
            hypotheses: vec!["Може додавати не лише чисельники, а й знаменники".into()],
            ideas: vec!["Покажіть піцу, розрізану на однакові частини.".into()],
        }],
        ask_adult: vec![AskAdultItem {
            skill: "F-ADD-DIFF".into(),
            title: "Додавання дробів з різними знаменниками".into(),
            statement: "Обчисли: 1/6 + 5/12".into(),
        }],
        stickers: 5,
        sticker_capacity: 16,
    }
}

fn texts(blocks: &[Block]) -> String {
    blocks
        .iter()
        .map(|b| match b {
            Block::Title(t)
            | Block::Heading(t)
            | Block::Para(t)
            | Block::Bullet(t)
            | Block::SubBullet(t) => t.clone(),
            Block::Rule | Block::Space(_) => String::new(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_document_follows_the_agreed_content_and_tone() {
    let d = sheet_document(&sample());
    let t = texts(&d.blocks);
    for want in [
        "Аркуш для вчителя",
        "а не оцінка",
        "Нікнейм у застосунку: Мирослава",
        "Стікерів в альбомі: 5 із 16",
        "Додавання багатоцифрових чисел",
        "Може додавати не лише чисельники, а й знаменники",
        "Ідея на 10 хвилин",
        "Обчисли: 1/6 + 5/12",
        "не містить відповідей",
    ] {
        assert!(t.contains(want), "немає «{want}»:\n{t}");
    }
    assert!(
        t.contains("Ім") && t.contains("____"),
        "місце, де дитина допише ім'я від руки"
    );
    for bad in ["оцінка:", "бал", "слабк", "не вміє", "лінив"] {
        assert!(
            !t.to_lowercase().contains(bad),
            "оцінне формулювання «{bad}»"
        );
    }
}

#[test]
fn an_empty_sheet_says_there_is_nothing_yet_instead_of_leaving_blanks() {
    let s = TeacherSheet {
        subject: String::new(),
        to_repeat: vec![],
        nickname: String::new(),
        mastered: vec![],
        in_progress: vec![],
        gaps: vec![],
        ask_adult: vec![],
        stickers: 0,
        sticker_capacity: 16,
    };
    let t = texts(&sheet_document(&s).blocks);
    assert!(t.matches("поки немає").count() >= 3, "{t}");
    assert!(
        !t.contains("Нікнейм у застосунку"),
        "порожній нікнейм не друкується"
    );
    assert!(sheet_pdf(&s).unwrap().starts_with(b"%PDF-1.4"));
}

#[test]
fn the_pdf_is_produced_for_a_realistic_sheet() {
    let pdf = sheet_pdf(&sample()).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4") && pdf.len() > report::FONT_BYTES.len());
}

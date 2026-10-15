use report::notes::{notes_document, notes_pdf};
use report::pdf::Block;

fn entries() -> Vec<(String, String)> {
    vec![
        (
            "Додавання дробів з однаковими знаменниками".to_string(),
            "Додаю чисельники.\n\nЗнаменник не змінюю, бо частини однакові.".to_string(),
        ),
        ("Порядок дій".to_string(), "Спочатку множення.".to_string()),
    ]
}

#[test]
fn the_document_has_a_heading_per_topic_and_one_paragraph_per_line() {
    let d = notes_document("Мирослава", &entries());
    let headings: Vec<_> = d
        .blocks
        .iter()
        .filter_map(|b| {
            if let Block::Heading(t) = b {
                Some(t.as_str())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        headings,
        ["Додавання дробів з однаковими знаменниками", "Порядок дій"]
    );
    let paras = d
        .blocks
        .iter()
        .filter(|b| matches!(b, Block::Para(_)))
        .count();
    assert!(paras >= 4, "рядки конспекту стали абзацами: {paras}");
}

#[test]
fn an_empty_notebook_still_makes_a_valid_pdf() {
    let pdf = notes_pdf("Оля", &[]).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4"));
}

#[test]
fn the_pdf_renders_and_survives_hostile_text() {
    let mut e = entries();
    e.push((
        "Дивне".to_string(),
        "\u{0}\u{1b}[31m 😀 \t мішанина ()\\ \r\n".repeat(200),
    ));
    let pdf = notes_pdf("Оля", &e).unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(pdf.len() > 50_000);
}

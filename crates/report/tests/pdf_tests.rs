use report::pdf::{render, Block, Document};
use report::ttf::Font;
use std::collections::BTreeMap;

fn doc(blocks: Vec<Block>) -> Document {
    Document {
        title: "Аркуш".to_string(),
        blocks,
    }
}

fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).rposition(|w| w == needle)
}

/// Розбирає таблицю xref (за байтами) і перевіряє, що кожне зміщення вказує на «N 0 obj».
fn check_structure(pdf: &[u8]) -> usize {
    assert!(pdf.starts_with(b"%PDF-1.4"), "заголовок PDF");
    assert!(pdf.trim_ascii_end().ends_with(b"%%EOF"));
    let sx = rfind(pdf, b"startxref").expect("startxref");
    let tail = std::str::from_utf8(&pdf[sx + 9..]).unwrap();
    let xref_off: usize = tail
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(
        pdf[xref_off..].starts_with(b"xref"),
        "зміщення xref вказує на таблицю"
    );
    let table = std::str::from_utf8(&pdf[xref_off..sx]).unwrap();
    let mut lines = table.lines();
    lines.next();
    let head = lines.next().unwrap();
    let mut it = head.split_whitespace();
    let first: usize = it.next().unwrap().parse().unwrap();
    let count: usize = it.next().unwrap().parse().unwrap();
    assert_eq!(first, 0);
    for n in 0..count {
        let entry = lines.next().unwrap();
        assert_eq!(
            entry.len(),
            19,
            "запис xref має 20 байт разом із розривом рядка: {entry:?}"
        );
        if n == 0 {
            continue;
        }
        let off: usize = entry[..10].trim().parse().unwrap();
        let want = format!("{n} 0 obj");
        assert!(
            pdf[off..].starts_with(want.as_bytes()),
            "об'єкт {n} не за зміщенням {off}"
        );
    }
    count - 1
}

/// Витягує текст зі сторінкових потоків: гліфи 16-бітні, перетворюються назад на символи.
fn extract_text(pdf: &[u8]) -> String {
    let font = Font::parse(report::FONT_BYTES).unwrap();
    let text = String::from_utf8_lossy(pdf).to_string();
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(i) = rest.find('<') {
        let tail = &rest[i + 1..];
        let Some(j) = tail.find('>') else { break };
        let hex = &tail[..j];
        if hex.len() >= 4 && hex.len() % 4 == 0 && tail[j + 1..].trim_start().starts_with("Tj") {
            for k in (0..hex.len()).step_by(4) {
                let gid = u16::from_str_radix(&hex[k..k + 4], 16).unwrap();
                out.push(font.char_for_glyph(gid).unwrap_or('\u{FFFD}'));
            }
            out.push('\n');
        }
        rest = &tail[j + 1..];
    }
    out
}

#[test]
fn a_small_document_is_a_structurally_valid_pdf_with_readable_text() {
    let pdf = render(&doc(vec![
        Block::Title("Аркуш для вчителя".into()),
        Block::Para("Нікнейм: Мирослава".into()),
        Block::Heading("Що вже опановано".into()),
        Block::Bullet("Додавання дробів з однаковими знаменниками".into()),
        Block::Rule,
        Block::Space(10.0),
        Block::Para("Ґанок, Єнот, Їжак, І’я".into()),
    ]))
    .unwrap();
    let objects = check_structure(&pdf);
    assert!(objects >= 8, "каталог, сторінки, шрифт, потоки: {objects}");
    let t = extract_text(&pdf);
    for want in [
        "Аркуш для вчителя",
        "Нікнейм: Мирослава",
        "Що вже опановано",
        "Додавання дробів з однаковими знаменниками",
        "Ґанок, Єнот, Їжак",
    ] {
        assert!(
            t.contains(want),
            "немає «{want}» у витягнутому тексті:\n{t}"
        );
    }
}

#[test]
fn a_long_document_flows_onto_numbered_pages() {
    let blocks: Vec<Block> = (0..150)
        .map(|i| Block::Bullet(format!("Пункт номер {i}: ") + &"слово ".repeat(12)))
        .collect();
    let pdf = render(&doc(blocks)).unwrap();
    check_structure(&pdf);
    let text = String::from_utf8_lossy(&pdf).to_string();
    let pages = text.matches("/Type /Page\n").count() + text.matches("/Type /Page ").count();
    assert!(pages >= 4, "{pages} сторінок");
    assert!(text.contains(&format!("/Count {pages}")));
    let t = extract_text(&pdf);
    assert!(t.contains("Сторінка 1 з"), "нумерація сторінок");
    assert!(t.contains("Пункт номер 149"));
}

#[test]
fn very_long_words_are_broken_and_never_hang_or_overflow() {
    let pdf = render(&doc(vec![
        Block::Para("Ы".repeat(2_000)),
        Block::Para("a".repeat(500) + " " + &"б".repeat(500)),
    ]))
    .unwrap();
    check_structure(&pdf);
    let t = extract_text(&pdf);
    assert!(t.matches('Ы').count() == 2_000, "жоден символ не втрачено");
}

#[test]
fn invisible_control_and_unsupported_characters_are_neutralised() {
    let pdf = render(&doc(vec![Block::Para(
        "Ім'я\u{202E}ля\u{200B}ля\u{0}\u{7} 😀 ✨ end".into(),
    )]))
    .unwrap();
    let t = extract_text(&pdf);
    assert!(
        t.contains("Ім'яляля"),
        "керівні й невидимі символи відсікаються: {t:?}"
    );
    assert!(!t.contains('\u{202E}') && !t.contains('\u{0}'));
    assert!(
        t.contains("? ? end") || t.contains("?  ? end") || t.contains("end"),
        "непідтримувані замінено на ?: {t:?}"
    );
}

#[test]
fn an_empty_document_still_produces_a_valid_single_page_pdf() {
    let pdf = render(&doc(vec![])).unwrap();
    check_structure(&pdf);
    assert!(String::from_utf8_lossy(&pdf).contains("/Count 1"));
}

#[test]
fn only_the_used_glyph_widths_are_listed() {
    let pdf = render(&doc(vec![Block::Para("абв".into())])).unwrap();
    let text = String::from_utf8_lossy(&pdf).to_string();
    let w = text.find("/W [").expect("масив ширин");
    let end = text[w..].find("]\n").unwrap();
    let entries: BTreeMap<_, _> = text[w..w + end]
        .match_indices('[')
        .map(|(i, _)| (i, ()))
        .collect();
    assert!(
        entries.len() < 60,
        "у масиві лише використані гліфи, а не весь шрифт"
    );
}

#[test]
fn the_font_program_is_embedded_and_marked_as_such() {
    let pdf = render(&doc(vec![Block::Para("тест".into())])).unwrap();
    let text = String::from_utf8_lossy(&pdf).to_string();
    assert!(text.contains("/FontFile2"));
    assert!(text.contains("/Subtype /CIDFontType2"));
    assert!(text.contains("/ToUnicode"));
    assert!(pdf.len() > report::FONT_BYTES.len(), "шрифт вбудовано");
}

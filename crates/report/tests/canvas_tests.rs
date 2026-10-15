use report::canvas::{Align, Canvas, FontId, Rgb, A4};

#[test]
fn a_styled_page_renders_a_valid_pdf_with_three_fonts_and_cyrillic_text() {
    let mut c = Canvas::new(A4).unwrap();
    c.rect(
        0.0,
        700.0,
        A4.0,
        142.0,
        0.0,
        Some(Rgb::hex("#2f5bea")),
        None,
    );
    c.rect(
        40.0,
        400.0,
        200.0,
        100.0,
        14.0,
        Some(Rgb::hex("#ffd23f")),
        Some((Rgb(27, 42, 107), 2.0)),
    );
    c.text(
        FontId::Bold,
        30.0,
        40.0,
        780.0,
        Align::Left,
        Rgb(255, 255, 255),
        "Аркуш для вчителя",
    );
    c.text(
        FontId::Hand,
        26.0,
        300.0,
        600.0,
        Align::Center,
        Rgb(0, 0, 0),
        "Дякую, вчителю!",
    );
    let n = c.paragraph(FontId::Body, 11.0, 40.0, 380.0, 200.0, 1.4, Align::Left, Rgb(0, 0, 0), "Довгий абзац українською мовою, який має перенестися на кілька рядків у межах заданої ширини.");
    assert!(n >= 3, "{n}");
    c.line(40.0, 300.0, 500.0, 300.0, Rgb(0, 0, 0), 1.0, true);
    c.new_page();
    c.text(
        FontId::Body,
        12.0,
        40.0,
        700.0,
        Align::Right,
        Rgb(0, 0, 0),
        "друга сторінка",
    );
    assert_eq!(c.page_count(), 2);
    let pdf = c.finish("Тест").unwrap();
    assert!(pdf.starts_with(b"%PDF-1.4"));
    assert!(pdf.ends_with(b"%%EOF\n"));
    let s = String::from_utf8_lossy(&pdf);
    assert!(s.contains("/Count 2"));
    assert_eq!(s.matches("/Subtype /CIDFontType2").count(), 3);
    assert!(s.contains("/F3 "), "рукописний шрифт підключено");
    assert!(pdf.len() > 100_000, "шрифти вбудовано: {}", pdf.len());
}

#[test]
fn alignment_and_width_use_the_real_font_metrics() {
    let c = Canvas::new(A4).unwrap();
    let narrow = c.text_width(FontId::Body, 12.0, "іі");
    let wide = c.text_width(FontId::Body, 12.0, "ЖЖ");
    assert!(wide > narrow);
    assert_eq!(c.lines_needed(FontId::Body, 12.0, 1000.0, "один рядок"), 1);
}

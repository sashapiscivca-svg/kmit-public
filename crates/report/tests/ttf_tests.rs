use report::ttf::Font;

fn font() -> Font<'static> {
    Font::parse(report::FONT_BYTES).expect("вбудований шрифт розбирається")
}

#[test]
fn the_embedded_font_covers_ukrainian_digits_and_punctuation() {
    let f = font();
    for c in "АБВГҐДЕЄЖЗИІЇЙКЛМНОПРСТУФХЦЧШЩЬЮЯабвгґдеєжзиіїйклмнопрстуфхцчшщьюя'ʼ’0123456789,.:;-—()/?!«»№ ".chars() {
        assert_ne!(f.glyph(c), 0, "немає гліфа для {c:?}");
    }
}

#[test]
fn unsupported_characters_map_to_the_missing_glyph() {
    let f = font();
    assert_eq!(f.glyph('😀'), 0);
    assert_eq!(f.glyph('✨'), 0);
    assert_eq!(f.glyph('\u{FEFF}'), 0);
}

#[test]
fn advances_are_sane_and_proportional() {
    let f = font();
    assert_eq!(f.units_per_em(), 2048);
    let w = f.advance(f.glyph('W'));
    let i = f.advance(f.glyph('i'));
    let space = f.advance(f.glyph(' '));
    assert!(w > i && i > 0 && space > 0, "W={w} i={i} space={space}");
    assert!(f.text_width("Привіт, світе", 12.0) > f.text_width("Привіт", 12.0));
    assert!((f.text_width("", 12.0)).abs() < f32::EPSILON);
}

#[test]
fn glyphs_round_trip_for_text_extraction() {
    let f = font();
    for c in "Ґрета".chars() {
        assert_eq!(f.char_for_glyph(f.glyph(c)), Some(c));
    }
}

#[test]
fn broken_font_data_is_an_error_not_a_panic() {
    assert!(Font::parse(&[]).is_none());
    assert!(Font::parse(b"not a font at all").is_none());
    let bytes = report::FONT_BYTES;
    for len in [4, 12, 100, 1000, bytes.len() / 2] {
        let _ = Font::parse(&bytes[..len]); // не панікує
    }
    let mut corrupted = bytes.to_vec();
    for i in (0..2000).step_by(3) {
        corrupted[i] ^= 0xFF;
    }
    let _ = Font::parse(&corrupted);
}

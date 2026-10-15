use tutor_core::frac::Frac;
use tutor_core::numtok::numeric_tokens;

fn f(n: i64, d: i64) -> Frac {
    Frac::new(n, d).unwrap()
}

#[test]
fn finds_fractions() {
    assert_eq!(numeric_tokens("Обчисли: 3/8 + 2/8"), vec![f(3, 8), f(2, 8)]);
}

#[test]
fn finds_decimals_with_comma_and_dot() {
    assert_eq!(numeric_tokens("12,5 і 7.25"), vec![f(25, 2), f(29, 4)]);
}

#[test]
fn sentence_punctuation_is_not_a_decimal() {
    assert_eq!(numeric_tokens("Відповідь: 12."), vec![Frac::int(12)]);
    assert_eq!(
        numeric_tokens("Спершу 5, потім 7."),
        vec![Frac::int(5), Frac::int(7)]
    );
}

#[test]
fn comma_after_fraction_starts_new_token() {
    assert_eq!(numeric_tokens("5/8,1/8"), vec![f(5, 8), f(1, 8)]);
}

#[test]
fn no_numbers_gives_empty() {
    assert!(numeric_tokens("Жодних чисел тут немає").is_empty());
}

#[test]
fn spaced_thousands_are_separate_tokens() {
    assert_eq!(numeric_tokens("1 234"), vec![Frac::int(1), Frac::int(234)]);
}

#[test]
fn number_words_are_a_known_blind_spot() {
    // Зафіксовано свідомо: числа словами не розпізнаються (див. документацію модуля).
    assert!(numeric_tokens("п'ять восьмих").is_empty());
}

#[test]
fn spaces_around_the_slash_make_one_fraction() {
    assert_eq!(numeric_tokens("Вийде 1 / 2"), vec![f(1, 2)]);
    assert_eq!(numeric_tokens("Вийде 1 /2 і 3/ 4"), vec![f(1, 2), f(3, 4)]);
    assert_eq!(
        numeric_tokens("Вийде 1\u{a0}/\u{a0}2"),
        vec![f(1, 2)],
        "нерозривний пробіл"
    );
}

#[test]
fn non_ascii_digits_are_normalised() {
    assert_eq!(
        numeric_tokens("١/٢"),
        vec![f(1, 2)],
        "арабсько-індійські цифри"
    );
    assert_eq!(numeric_tokens("１/２"), vec![f(1, 2)], "повноширинні цифри");
    assert_eq!(
        numeric_tokens("۱۲"),
        vec![Frac::int(12)],
        "розширені арабсько-індійські"
    );
    assert_eq!(numeric_tokens("٠,٦٢٥"), vec![f(5, 8)]);
}

#[test]
fn padding_zeros_do_not_hide_a_value() {
    assert_eq!(numeric_tokens("0000000000000000,5"), vec![f(1, 2)]);
    assert_eq!(numeric_tokens("0,5000000000000"), vec![f(1, 2)]);
    assert_eq!(numeric_tokens("00012"), vec![Frac::int(12)]);
    assert_eq!(numeric_tokens("12,000"), vec![Frac::int(12)]);
}

#[test]
fn unrepresentable_numbers_are_skipped_without_panicking() {
    assert!(numeric_tokens("99999999999999999999").is_empty());
    assert!(numeric_tokens(&"9".repeat(10_000)).is_empty());
    assert!(numeric_tokens("0,1234567890123").is_empty());
}

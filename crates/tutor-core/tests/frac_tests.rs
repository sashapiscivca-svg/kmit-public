use proptest::prelude::*;
use tutor_core::frac::{parse_answer, Frac};

fn f(n: i64, d: i64) -> Frac {
    Frac::new(n, d).unwrap()
}

#[test]
fn new_normalizes_sign_and_gcd() {
    assert_eq!(f(2, 4), f(1, 2));
    assert_eq!((f(1, -2).num(), f(1, -2).den()), (-1, 2));
    assert_eq!(f(0, 5), Frac::int(0));
    assert_eq!(Frac::new(1, 0), None);
}

#[test]
fn arithmetic_is_exact() {
    assert_eq!(f(1, 2).checked_add(f(1, 3)), Some(f(5, 6)));
    assert_eq!(f(1, 2).checked_sub(f(1, 3)), Some(f(1, 6)));
    assert_eq!(f(2, 3).checked_mul(f(3, 4)), Some(f(1, 2)));
    assert_eq!(f(1, 2).checked_div(f(1, 4)), Some(Frac::int(2)));
    assert_eq!(f(1, 2).checked_div(Frac::int(0)), None);
}

#[test]
fn ordering_compares_values() {
    assert!(f(1, 3) < f(1, 2));
    assert!(f(2, 4) == f(1, 2));
    assert!(f(-1, 2) < f(0, 1));
}

#[test]
fn display_is_plain() {
    assert_eq!(f(3, 4).to_string(), "3/4");
    assert_eq!(Frac::int(5).to_string(), "5");
}

#[test]
fn parse_accepts_integers_fractions_and_decimals() {
    assert_eq!(parse_answer("12"), Some(Frac::int(12)));
    assert_eq!(parse_answer("3/4"), Some(f(3, 4)));
    assert_eq!(parse_answer(" 3 / 4 "), Some(f(3, 4)));
    assert_eq!(parse_answer("3,5"), Some(f(7, 2)));
    assert_eq!(parse_answer("3.5"), Some(f(7, 2)));
    assert_eq!(parse_answer("-2"), Some(Frac::int(-2)));
    assert_eq!(parse_answer("0,25"), Some(f(1, 4)));
}

#[test]
fn parse_handles_child_typing_habits() {
    assert_eq!(parse_answer("0,50"), Some(f(1, 2)), "кінцевий нуль");
    assert_eq!(
        parse_answer("\t7 \n"),
        Some(Frac::int(7)),
        "пробіли навколо"
    );
    assert_eq!(parse_answer("007"), Some(Frac::int(7)), "ведучі нулі");
}

#[test]
fn parse_rejects_hostile_input_without_panicking() {
    for bad in [
        "١٢",
        "1/2/3",
        "99999999999999999999",
        "3,99999999999999",
        "0,0000000001",
        "١/٢",
        "½",
        "1 2/3",
    ] {
        assert_eq!(parse_answer(bad), None, "мало бути None для {bad:?}");
    }
    let huge = "9".repeat(10_000);
    assert_eq!(parse_answer(&huge), None);
}

#[test]
fn parse_rejects_garbage() {
    for bad in [
        "",
        "   ",
        "abc",
        "1/0",
        "3,",
        ",5",
        "1,2,3",
        "3,1234567890",
        "1/",
        "/2",
        "--1",
        "1e3",
    ] {
        assert_eq!(parse_answer(bad), None, "мало бути None для {bad:?}");
    }
}

proptest! {
    #[test]
    fn normalization_is_idempotent(n in -1000i64..1000, d in 1i64..1000) {
        let a = Frac::new(n, d).unwrap();
        prop_assert_eq!(Frac::new(a.num(), a.den()), Some(a));
        prop_assert!(a.den() > 0);
    }

    #[test]
    fn addition_commutes(n in -500i64..500, d in 1i64..500, m in -500i64..500, e in 1i64..500) {
        let (a, b) = (Frac::new(n, d).unwrap(), Frac::new(m, e).unwrap());
        prop_assert_eq!(a.checked_add(b), b.checked_add(a));
    }

    #[test]
    fn add_then_sub_roundtrips(n in -500i64..500, d in 1i64..500, m in -500i64..500, e in 1i64..500) {
        let (a, b) = (Frac::new(n, d).unwrap(), Frac::new(m, e).unwrap());
        prop_assert_eq!(a.checked_add(b).unwrap().checked_sub(b), Some(a));
    }
}

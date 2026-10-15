use tutor_core::rng::{seed_for, SeededRng};

#[test]
fn same_seed_same_sequence() {
    let mut a = SeededRng::new(42);
    let mut b = SeededRng::new(42);
    for _ in 0..100 {
        assert_eq!(a.range(0, 1_000_000), b.range(0, 1_000_000));
    }
}

#[test]
fn different_seeds_differ() {
    let mut a = SeededRng::new(1);
    let mut b = SeededRng::new(2);
    let xs: Vec<i64> = (0..20).map(|_| a.range(0, 1_000_000)).collect();
    let ys: Vec<i64> = (0..20).map(|_| b.range(0, 1_000_000)).collect();
    assert_ne!(xs, ys);
}

#[test]
fn range_respects_bounds_and_hits_both_ends() {
    let mut r = SeededRng::new(7);
    let mut seen = [false; 5];
    for _ in 0..500 {
        let x = r.range(3, 7);
        assert!((3..=7).contains(&x));
        seen[(x - 3) as usize] = true;
    }
    assert!(seen.iter().all(|&s| s), "усі п'ять значень мали з'явитися");
}

#[test]
fn single_value_range_is_constant() {
    let mut r = SeededRng::new(9);
    for _ in 0..10 {
        assert_eq!(r.range(4, 4), 4);
    }
}

#[test]
fn unit_is_in_half_open_interval() {
    let mut r = SeededRng::new(3);
    for _ in 0..1000 {
        let u = r.unit();
        assert!((0.0..1.0).contains(&u));
    }
}

#[test]
fn seed_for_depends_on_skill_and_variant() {
    assert_ne!(seed_for("F-ADD-SAME", 1), seed_for("F-ADD-SAME", 2));
    assert_ne!(seed_for("F-ADD-SAME", 1), seed_for("F-ADD-DIFF", 1));
    assert_eq!(seed_for("F-ADD-SAME", 1), seed_for("F-ADD-SAME", 1));
}

/// Золоті значення: фіксують поведінку так, щоб будь-яка платформа в CI підтвердила ту саму послідовність.
#[test]
fn golden_values_are_stable() {
    assert_eq!(seed_for("F-ADD-SAME", 1), GOLDEN_SEED);
    let mut r = SeededRng::new(42);
    let got: Vec<i64> = (0..8).map(|_| r.range(0, 99)).collect();
    assert_eq!(got, GOLDEN_SEQUENCE);
}

const GOLDEN_SEED: u64 = 691_608_069_768_904_852;
const GOLDEN_SEQUENCE: [i64; 8] = [37, 12, 20, 66, 44, 38, 8, 12];

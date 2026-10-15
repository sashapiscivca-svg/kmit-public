use tutor_core::bkt::{BktError, BktParams, SkillState, P_MAX, P_MIN};
use tutor_core::config::Config;

#[test]
fn constructor_enforces_bounds() {
    assert!(BktParams::new(0.3, 0.1, 0.05, 0.1).is_ok());
    assert_eq!(
        BktParams::new(0.0, 0.1, 0.05, 0.1),
        Err(BktError::OutOfRange)
    );
    assert_eq!(
        BktParams::new(0.3, 1.0, 0.05, 0.1),
        Err(BktError::OutOfRange)
    );
    assert_eq!(
        BktParams::new(0.3, 0.1, 0.5, 0.1),
        Err(BktError::GuessTooHigh)
    );
    assert_eq!(
        BktParams::new(0.3, 0.1, 0.05, 0.5),
        Err(BktError::SlipTooHigh)
    );
    assert!(
        BktParams::new(0.3, 0.1, 0.49, 0.49).is_ok(),
        "сума 0.98 < 1 допустима"
    );
    assert_eq!(
        BktParams::new(f64::NAN, 0.1, 0.05, 0.1),
        Err(BktError::OutOfRange)
    );
}

#[test]
fn update_matches_hand_computed_value() {
    // p=0.3, slip=0.1, guess=0.05, learn=0.12:
    // posterior = 0.27 / 0.305 = 0.885245901639...; p' = posterior + (1 - posterior) * 0.12
    let p = BktParams::numeric_default();
    let got = p.update(0.3, true);
    assert!((got - 0.899_016_393_443).abs() < 1e-9, "отримано {got}");
}

#[test]
fn correct_raises_and_incorrect_lowers_relative_to_start() {
    let p = BktParams::numeric_default();
    let after_correct = p.update(0.5, true);
    let after_wrong = p.update(0.5, false);
    assert!(after_correct > 0.5);
    assert!(after_wrong < after_correct);
}

#[test]
fn probability_stays_inside_open_unit_interval() {
    let p = BktParams::numeric_default();
    let mut x = p.p_init();
    for i in 0..200 {
        x = p.update(x, i % 3 != 0);
        assert!(x > 0.0 && x < 1.0, "вийшло за межі: {x}");
    }
}

#[test]
fn certainty_is_never_absorbing() {
    let p = BktParams::numeric_default();
    let mut x = p.p_init();
    for _ in 0..1000 {
        x = p.update(x, true);
    }
    assert!(x <= P_MAX, "стеля досягнута: {x}");
    let after_wrong = p.update(x, false);
    assert!(after_wrong < x, "після помилки ймовірність має знизитися");
    let mut y = p.p_init();
    for _ in 0..1000 {
        y = p.update(y, false);
    }
    assert!(y >= P_MIN, "підлога: {y}");
}

#[test]
fn mastery_needs_probability_streak_and_two_days() {
    let p = BktParams::numeric_default();
    let cfg = Config::default();

    // Багато успіхів в один день: P(L) високе, але днів лише 1.
    let mut one_day = SkillState::new(&p);
    for _ in 0..10 {
        one_day.observe(&p, 0, true, 0);
    }
    assert!(one_day.p_known >= cfg.mastery_p);
    assert!(!one_day.is_mastered(&cfg));

    // Той самий результат у два дні: опановано.
    let mut two_days = SkillState::new(&p);
    for day in [0, 0, 0, 1, 1, 1, 1, 1] {
        two_days.observe(&p, day, true, 0);
    }
    assert!(two_days.is_mastered(&cfg));
}

#[test]
fn hint_or_error_resets_streak_and_blocks_mastery() {
    let p = BktParams::numeric_default();
    let cfg = Config::default();
    let mut s = SkillState::new(&p);
    for day in [0, 1, 1, 1, 1, 1] {
        s.observe(&p, day, true, 0);
    }
    assert!(s.is_mastered(&cfg));
    s.observe(&p, 2, true, 1); // правильно, але з підказкою: не «чистий» успіх
    assert_eq!(s.streak, 0);
    assert!(!s.is_mastered(&cfg));
}

#[test]
fn predicted_success_blends_knowing_and_guessing() {
    // p=0.3, slip=0.1, guess=0.05: 0.3 * 0.9 + 0.7 * 0.05 = 0.305
    let p = BktParams::numeric_default();
    assert!((p.predicted_success(0.3) - 0.305).abs() < 1e-12);
    assert!(p.predicted_success(0.9) > p.predicted_success(0.5));
}

#[test]
fn attempts_count_every_observation() {
    let p = BktParams::numeric_default();
    let mut s = SkillState::new(&p);
    assert_eq!(s.attempts, 0);
    s.observe(&p, 0, true, 0);
    s.observe(&p, 0, false, 2);
    s.observe(&p, 1, true, 1);
    assert_eq!(s.attempts, 3);
}

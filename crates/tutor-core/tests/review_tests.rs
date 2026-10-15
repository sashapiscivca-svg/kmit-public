use tutor_core::config::Config;
use tutor_core::review::ReviewSchedule;

fn cfg() -> Config {
    Config::default()
}

#[test]
fn first_review_is_due_after_the_first_interval() {
    let s = ReviewSchedule::after_mastery(10, &cfg());
    assert_eq!((s.level(), s.due_day()), (0, 11));
    assert!(!s.is_due(10));
    assert!(s.is_due(11));
    assert!(s.is_due(40));
}

#[test]
fn clean_reviews_climb_the_intervals() {
    let c = cfg();
    let mut s = ReviewSchedule::after_mastery(0, &c); // due 1
    let mut day = 1;
    let mut expected_gaps = vec![3, 7, 14, 30, 30, 30].into_iter();
    for _ in 0..6 {
        s = s.after_review(true, day, &c);
        let gap = expected_gaps.next().unwrap();
        assert_eq!(
            s.due_day(),
            day + gap,
            "після чистого повторення в день {day}"
        );
        day = s.due_day();
    }
    assert_eq!(s.level(), 4, "найдовший інтервал є стелею");
}

#[test]
fn a_failed_review_resets_to_the_first_interval() {
    let c = cfg();
    let mut s = ReviewSchedule::after_mastery(0, &c);
    s = s.after_review(true, 1, &c);
    s = s.after_review(true, 4, &c);
    assert_eq!(s.level(), 2);
    s = s.after_review(false, 11, &c);
    assert_eq!((s.level(), s.due_day()), (0, 12));
}

#[test]
fn overdue_and_microtest_thresholds() {
    let c = cfg();
    let s = ReviewSchedule::after_mastery(0, &c); // due 1
    assert_eq!(s.overdue_days(0), 0);
    assert_eq!(s.overdue_days(11), 10);
    assert!(!s.needs_microtest(30, &c));
    assert!(s.needs_microtest(31, &c), "30 днів прострочення");
}

#[test]
fn day_arithmetic_saturates_instead_of_overflowing() {
    let c = cfg();
    let s = ReviewSchedule::after_mastery(u32::MAX, &c);
    assert_eq!(s.due_day(), u32::MAX);
    let t = s.after_review(true, u32::MAX, &c);
    assert_eq!(t.due_day(), u32::MAX);
}

use tutor_core::config::Config;
use tutor_core::sessionctl::{
    BreakReason, DayUsage, EndReason, Session, SessionStatus, StartDenied,
};

fn cfg() -> Config {
    Config {
        enforce_limits: true,
        ..Config::default()
    } // 20 хв ціль, 25 хв жорстка межа, пауза 60 с, простій 300 с, перерва 240 с, 2 сесії
}

fn start(now: u64) -> (DayUsage, Session) {
    let mut d = DayUsage::new();
    d.begin_day(1);
    let s = d.start(now, false, false, &cfg()).unwrap();
    (d, s)
}

#[test]
fn the_daily_session_limit_and_the_break_between_sessions() {
    let c = cfg();
    let mut d = DayUsage::new();
    d.begin_day(1);
    let first = d.start(1_000, false, false, &c).unwrap();
    drop(first);
    d.finish(2_000);
    assert_eq!(
        d.can_start(2_100, false, false, &c),
        Err(StartDenied::BreakNotOver { secs: 140 })
    );
    assert!(
        d.start(2_240, false, false, &c).is_ok(),
        "перерва 240 с минула"
    );
    d.finish(3_500);
    assert_eq!(
        d.can_start(9_999, false, false, &c),
        Err(StartDenied::MaxSessionsToday)
    );
    // Дорослий може дозволити третю сесію, але не четверту.
    assert!(d.start(9_999, false, true, &c).is_ok());
    assert_eq!(
        d.can_start(99_999, false, true, &c),
        Err(StartDenied::MaxSessionsToday)
    );
}

#[test]
fn a_new_day_resets_the_counters_but_not_the_break() {
    let c = cfg();
    let mut d = DayUsage::new();
    d.begin_day(1);
    d.start(100, false, false, &c).unwrap();
    d.start(100, false, false, &c).unwrap();
    d.finish(10_000);
    d.begin_day(2);
    assert_eq!(d.sessions_started(), 0);
    assert!(d.can_start(10_000 + 240, false, false, &c).is_ok());
    // Перерва зберігається й через опівніч: рано-вранці не можна стартувати негайно після пізньої сесії.
    assert_eq!(
        d.can_start(10_100, false, false, &c),
        Err(StartDenied::BreakNotOver { secs: 140 })
    );
}

#[test]
fn quiet_hours_deny_a_start() {
    let d = DayUsage::new();
    assert_eq!(
        d.can_start(0, true, false, &cfg()),
        Err(StartDenied::QuietHours)
    );
}

#[test]
fn active_time_accumulates_and_the_target_leads_to_wrap_up() {
    let c = cfg();
    let (_d, mut s) = start(0);
    let mut now = 0;
    while s.active_secs() + 30 < 1_200 {
        now += 30;
        assert_eq!(s.activity(now, &c), SessionStatus::Active);
    }
    now += 30;
    assert_eq!(
        s.activity(now, &c),
        SessionStatus::WrapUp,
        "ціль 20 хв: дороби задачу й завершуй"
    );
    s.end_time_up();
    assert_eq!(s.status(&c), SessionStatus::Ended(EndReason::TimeUp));
}

#[test]
fn the_hard_cap_ends_the_session_even_in_wrap_up() {
    let c = cfg();
    let (_d, mut s) = start(0);
    let mut now = 0;
    loop {
        now += 30;
        if let SessionStatus::Ended(r) = s.activity(now, &c) {
            assert_eq!(r, EndReason::HardCap);
            break;
        }
        assert!(now < 3_000, "жорстка межа мала спрацювати");
    }
    assert!(s.active_secs() >= 1_500);
}

#[test]
fn a_pause_counts_only_its_first_minute_and_does_not_punish() {
    let c = cfg();
    let (_d, mut s) = start(0);
    s.activity(30, &c);
    assert_eq!(s.active_secs(), 30);
    // Пауза 200 с (між 60 і 300): перша хвилина паузи це час біля екрана, решта ні; сесія триває.
    assert_eq!(s.activity(230, &c), SessionStatus::Active);
    assert_eq!(s.active_secs(), 90);
    assert_eq!(s.activity(250, &c), SessionStatus::Active);
    assert_eq!(s.active_secs(), 110);
}

#[test]
fn a_child_who_answers_every_ninety_seconds_still_reaches_the_limits() {
    let c = cfg();
    let (_d, mut s) = start(0);
    let mut now = 0;
    let mut status = SessionStatus::Active;
    for _ in 0..40 {
        now += 90;
        status = s.activity(now, &c);
        if matches!(status, SessionStatus::Ended(_)) {
            break;
        }
    }
    assert_eq!(
        status,
        SessionStatus::Ended(EndReason::HardCap),
        "50 хв календарного часу не безмежні"
    );
    assert!(s.active_secs() >= 1_500);
}

#[test]
fn the_wall_time_cap_stops_a_session_even_when_activity_is_sparse() {
    let c = cfg();
    let (_d, mut s) = start(0);
    let mut now = 0;
    loop {
        now += 250; // кожна пауза зараховує лише першу хвилину
        if let SessionStatus::Ended(r) = s.activity(now, &c) {
            assert_eq!(r, EndReason::HardCap);
            break;
        }
        assert!(now < 20_000, "календарна межа мала спрацювати");
    }
    assert!(now <= 2 * u64::from(c.session_hard_cap_secs) + 250);
}

#[test]
fn a_long_idle_ends_the_session_on_tick_or_on_return() {
    let c = cfg();
    let (_d, mut s) = start(0);
    s.activity(10, &c);
    assert_eq!(s.tick(200, &c), SessionStatus::Active);
    assert_eq!(s.tick(400, &c), SessionStatus::Ended(EndReason::Idle));

    let (_d2, mut s2) = start(0);
    s2.activity(10, &c);
    assert_eq!(
        s2.activity(1_000, &c),
        SessionStatus::Ended(EndReason::Idle)
    );
    assert_eq!(s2.active_secs(), 10, "час простою не додано");
}

#[test]
fn the_end_reasons_are_sticky() {
    let c = cfg();
    let (_d, mut s) = start(0);
    s.end_by_user();
    s.end_nothing_left();
    s.end_time_up();
    assert_eq!(s.status(&c), SessionStatus::Ended(EndReason::UserEnded));
    assert_eq!(
        s.activity(10, &c),
        SessionStatus::Ended(EndReason::UserEnded)
    );
}

#[test]
fn time_going_backwards_adds_no_time_but_resynchronises() {
    let c = cfg();
    let (_d, mut s) = start(1_000);
    s.activity(1_030, &c);
    assert_eq!(s.activity(500, &c), SessionStatus::Active);
    assert_eq!(s.active_secs(), 30, "стрибок назад часу не додає");
    s.activity(530, &c);
    assert_eq!(
        s.active_secs(),
        60,
        "далі години знову йдуть: нічого не заморожено"
    );
    assert_eq!(
        s.tick(500 + 400, &c),
        SessionStatus::Ended(EndReason::Idle),
        "простій після відкату теж виявляється"
    );
    let mut big = start(u64::MAX - 5).1;
    assert_eq!(big.activity(u64::MAX, &c), SessionStatus::Active);
}

#[test]
fn a_device_clock_reset_never_locks_the_child_out() {
    let c = cfg();
    let mut d = DayUsage::new();
    d.begin_day(1);
    d.start(1_700_000_000, false, false, &c).unwrap();
    d.finish(1_700_001_000);
    // Годинник скинувся (розряджений телефон без мережі): перерва вважається завершеною.
    assert!(d.can_start(1_000_000_000, false, false, &c).is_ok());
    // Навіть якщо «залишок» був би величезним, він не перевищує звичайної перерви.
    d.finish(u64::MAX);
    assert!(
        matches!(
            d.can_start(10, false, true, &c),
            Ok(()) | Err(StartDenied::BreakNotOver { secs: 1..=240 })
        ),
        "залишок перерви обмежено її тривалістю"
    );
}

#[test]
fn a_very_short_session_does_not_use_up_a_daily_slot() {
    let c = cfg();
    let mut d = DayUsage::new();
    d.begin_day(1);
    let mut s = d.start(1_000, false, false, &c).unwrap();
    s.activity(1_030, &c); // зникло світло через 30 с
    d.finish_session(1_030, &s, &c);
    assert_eq!(
        d.sessions_started(),
        0,
        "сесію коротшу за мінімум не зараховано"
    );
    assert!(
        d.can_start(1_031, false, false, &c).is_ok(),
        "і перерви після неї немає"
    );
    // Звичайна сесія зараховується й дає перерву.
    let mut s = d.start(2_000, false, false, &c).unwrap();
    let mut now = 2_000;
    for _ in 0..10 {
        now += 30;
        s.activity(now, &c);
    }
    d.finish_session(now, &s, &c);
    assert_eq!(d.sessions_started(), 1);
    assert!(matches!(
        d.can_start(now + 1, false, false, &c),
        Err(StartDenied::BreakNotOver { .. })
    ));
}

#[test]
fn an_unfinished_session_survives_a_restart() {
    let c = cfg();
    let (_d, mut s) = start(0);
    s.activity(30, &c);
    s.record_task(25, true, 0);
    s.record_task(40, false, 2);
    let text = s.encode();
    let mut back = Session::decode(&text).unwrap();
    assert_eq!(back.encode(), text);
    assert_eq!(back.summary(), s.summary());
    // Продовження з того ж місця: час тече далі від збереженого моменту.
    assert_eq!(back.activity(60, &c), SessionStatus::Active);
    assert_eq!(back.active_secs(), 60);
    assert!(Session::decode("зіпсовано").is_err());
    assert!(Session::decode("").is_err());
}

fn feed(s: &mut Session, items: &[(u32, bool)]) {
    for (lat, ok) in items {
        s.record_task(*lat, *ok, 0);
    }
}

#[test]
fn fatigue_needs_slow_and_failing_recent_tasks() {
    let c = cfg();
    let (_d, mut s) = start(0);
    // 5 нормальних задач (≈20 с), потім 3 повільні (40 с) і невдалі.
    feed(
        &mut s,
        &[(20, true), (18, true), (22, true), (20, true), (21, true)],
    );
    assert_eq!(s.suggest_break(&c), None, "мало задач");
    feed(&mut s, &[(40, false), (45, true), (50, false)]);
    assert_eq!(s.suggest_break(&c), Some(BreakReason::Fatigue));
}

#[test]
fn slow_but_correct_or_fast_but_wrong_is_not_fatigue() {
    let c = cfg();
    let (_d, mut slow_ok) = start(0);
    feed(
        &mut slow_ok,
        &[
            (20, true),
            (18, true),
            (22, true),
            (20, true),
            (21, true),
            (40, true),
            (45, true),
            (50, true),
        ],
    );
    assert_eq!(slow_ok.suggest_break(&c), None);
    let (_d, mut fast_wrong) = start(0);
    feed(
        &mut fast_wrong,
        &[
            (20, true),
            (18, true),
            (22, true),
            (20, true),
            (21, true),
            (20, false),
            (19, false),
            (21, false),
        ],
    );
    assert_eq!(fast_wrong.suggest_break(&c), None);
}

#[test]
fn the_summary_reports_what_happened() {
    let c = cfg();
    let (_d, mut s) = start(0);
    s.activity(30, &c);
    s.record_task(20, true, 0);
    s.record_task(40, false, 2);
    let sum = s.summary();
    assert_eq!(
        (sum.active_secs, sum.tasks, sum.solved, sum.hints),
        (30, 2, 1, 2)
    );
}

#[test]
fn zero_median_latency_does_not_trigger_fatigue() {
    let c = cfg();
    let (_d, mut s) = start(0);
    feed(
        &mut s,
        &[
            (0, true),
            (0, true),
            (0, true),
            (0, true),
            (0, true),
            (1, false),
            (1, false),
            (1, true),
        ],
    );
    assert_eq!(
        s.suggest_break(&c),
        None,
        "швидкі відповіді за нульовою медіаною не є втомою"
    );
}

#[test]
fn a_saved_session_with_repeated_or_missing_keys_is_rejected() {
    let c = cfg();
    let (_d, mut s) = start(0);
    s.activity(30, &c);
    s.record_task(20, true, 0);
    let good = s.encode();
    assert!(Session::decode(&good).is_ok());
    // Повторений ключ замість відсутніх: раніше такий текст проходив із нулями за замовчуванням.
    let forged = "KMT-SESSION 1\nstarted_at 1\nstarted_at 1\nstarted_at 1\nstarted_at 1\nstarted_at 1\nstarted_at 1\nstarted_at 1\nlatencies -\ncorrect -\n";
    assert!(Session::decode(forged).is_err());
    // Кількість задач має збігатися з довжиною запису затримок.
    let mismatch = good.replace("tasks 1", "tasks 5");
    assert!(Session::decode(&mismatch).is_err());
    // Відсутній ключ.
    let missing: String = good
        .lines()
        .filter(|l| !l.starts_with("hints "))
        .map(|l| format!("{l}\n"))
        .collect();
    assert!(Session::decode(&missing).is_err());
}

/// Рішення A9: за замовчуванням ліміти не діють; час лише радиться (`WrapUp`), сесія не обривається.
#[test]
fn by_default_limits_do_not_block_and_the_session_is_never_cut() {
    let c = Config::default();
    assert!(!c.enforce_limits);
    let mut d = DayUsage::new();
    d.begin_day(1);
    for i in 0..5u64 {
        let s = d.start(1_000 + i, true, false, &c).expect("жодних відмов");
        d.finish_session(2_000 + i, &s, &c);
    }
    let mut s = d.start(10_000, false, false, &c).unwrap();
    let mut now = 10_000;
    for _ in 0..200 {
        now += 30;
        s.activity(now, &c);
    }
    assert!(s.active_secs() > u64::from(c.session_hard_cap_secs));
    assert_eq!(s.status(&c), SessionStatus::WrapUp);
}

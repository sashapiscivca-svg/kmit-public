use tutor_core::config::Config;
use tutor_core::gate::{GateDecision, HintGate};

fn cfg() -> Config {
    // Затримка 12 с, 3 швидкі запити = блокування на 60 с. Вимога «спершу власна спроба» перевіряється окремо.
    Config {
        require_attempt_before_hint: false,
        ..Config::default()
    }
}

#[test]
fn the_first_hint_is_allowed_immediately_by_default() {
    let mut g = HintGate::new();
    assert_eq!(g.request(100, &cfg()), GateDecision::Allow);
}

#[test]
fn the_next_level_waits_for_the_minimum_delay() {
    let mut g = HintGate::new();
    let c = cfg();
    assert_eq!(g.request(100, &c), GateDecision::Allow);
    assert_eq!(g.request(103, &c), GateDecision::Wait { secs: 9 });
    assert_eq!(
        g.request(112, &c),
        GateDecision::Allow,
        "рівно через 12 с від попереднього показу"
    );
}

#[test]
fn impatient_tapping_during_the_wait_is_harmless() {
    let mut g = HintGate::new();
    let c = cfg();
    assert_eq!(g.request(100, &c), GateDecision::Allow);
    for t in 101..=108 {
        assert!(
            matches!(g.request(t, &c), GateDecision::Wait { .. }),
            "{t}: лише очікування, без блоку"
        );
    }
    assert_eq!(g.request(112, &c), GateDecision::Allow);
}

#[test]
fn clicking_through_all_levels_without_trying_is_blocked_and_the_block_expires() {
    let mut g = HintGate::new();
    let c = cfg();
    assert_eq!(g.request(100, &c), GateDecision::Allow);
    assert_eq!(g.request(112, &c), GateDecision::Allow);
    assert_eq!(g.request(124, &c), GateDecision::Allow);
    assert_eq!(
        g.request(136, &c),
        GateDecision::Blocked { secs: 60 },
        "три рівні поспіль без жодної спроби"
    );
    assert_eq!(g.request(160, &c), GateDecision::Blocked { secs: 36 });
    assert_eq!(g.request(196, &c), GateDecision::Allow, "блок минув");
}

#[test]
fn an_attempt_between_hints_resets_the_streak() {
    let mut g = HintGate::new();
    let c = cfg();
    assert_eq!(g.request(100, &c), GateDecision::Allow);
    assert_eq!(g.request(112, &c), GateDecision::Allow);
    g.note_attempt();
    assert_eq!(g.request(124, &c), GateDecision::Allow);
    assert_eq!(
        g.request(136, &c),
        GateDecision::Allow,
        "після спроби лічильник почався знову"
    );
}

#[test]
fn a_long_think_between_hints_also_resets_the_streak() {
    let mut g = HintGate::new();
    let c = cfg();
    assert_eq!(g.request(100, &c), GateDecision::Allow);
    assert_eq!(g.request(112, &c), GateDecision::Allow);
    assert_eq!(
        g.request(400, &c),
        GateDecision::Allow,
        "дитина довго думала сама"
    );
    assert_eq!(g.request(412, &c), GateDecision::Allow);
    assert_eq!(g.request(424, &c), GateDecision::Allow);
}

#[test]
fn a_child_who_tries_between_hints_is_never_blocked() {
    let mut g = HintGate::new();
    let c = cfg();
    for i in 0..10 {
        assert_eq!(g.request(100 + i * 20, &c), GateDecision::Allow);
        g.note_attempt();
    }
}

#[test]
fn requiring_an_attempt_first_is_a_setting() {
    let c = Config {
        require_attempt_before_hint: true,
        ..cfg()
    };
    let mut g = HintGate::new();
    assert_eq!(g.request(10, &c), GateDecision::NeedAttempt);
    g.note_attempt();
    assert_eq!(g.request(11, &c), GateDecision::Allow);
}

#[test]
fn a_clock_that_moves_backwards_resynchronises_instead_of_locking_the_hints() {
    let mut g = HintGate::new();
    let c = cfg();
    assert_eq!(g.request(1000, &c), GateDecision::Allow);
    assert_eq!(
        g.request(500, &c),
        GateDecision::Wait { secs: 12 },
        "після відкату відлік іде від нового часу"
    );
    assert_eq!(
        g.request(512, &c),
        GateDecision::Allow,
        "а не чекає, поки годинник «наздожене»"
    );
}

#[test]
fn huge_times_do_not_overflow() {
    let mut g = HintGate::new();
    let c = cfg();
    assert_eq!(g.request(u64::MAX - 1, &c), GateDecision::Allow);
    assert_eq!(g.request(u64::MAX, &c), GateDecision::Wait { secs: 11 });
    for _ in 0..10 {
        let _ = g.request(u64::MAX, &c);
    }
}

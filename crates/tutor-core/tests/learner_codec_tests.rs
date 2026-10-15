use std::collections::BTreeSet;
use tutor_core::bkt::{BktParams, SkillState};
use tutor_core::config::Config;
use tutor_core::learner::{CodecError, LearnerState};
use tutor_core::sessionctl::DayUsage;

fn cut_at_a_char_boundary(s: &str, mut end: usize) -> String {
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

fn rich_learner() -> (LearnerState, Config) {
    let (p, c) = (BktParams::numeric_default(), Config::default());
    let mut l = LearnerState::new();
    // A опановано (дні 0 і 1), далі повторення з мікротестом у процесі.
    l.begin_day(0);
    for _ in 0..6 {
        l.record_task(&p, &c, "A", true, 0, None);
    }
    l.begin_day(1);
    for _ in 0..6 {
        l.record_task(&p, &c, "A", true, 0, None);
    }
    l.begin_day(60);
    l.record_task(&p, &c, "A", true, 0, None); // мікротест: 1 із 3
                                               // B розпочато, помилки, підказки.
    l.record_task(&p, &c, "B", false, 1, Some("add_denominators"));
    l.record_task(&p, &c, "B", false, 0, Some("add_denominators"));
    l.record_task(&p, &c, "B", true, 2, None);
    l.mark_seen("B", "Обчисли: 1/5 + 2/5");
    l.mark_seen("B", "Умова з пробілами,\nновим рядком, % і \\ та «лапками»");
    l.next_variant("B");
    l.next_variant("B");
    l.mark_exhausted("C");
    l.record_ask_adult("B", "Обчисли: 1/3 + 1/6");
    (l, c)
}

#[test]
fn an_empty_learner_round_trips() {
    let l = LearnerState::new();
    let text = l.encode();
    let back = LearnerState::decode(&text).unwrap();
    assert_eq!(back.encode(), text);
    assert_eq!(back.today(), 0);
}

#[test]
fn a_rich_learner_round_trips_with_identical_behaviour() {
    let (l, c) = rich_learner();
    let text = l.encode();
    let back = LearnerState::decode(&text).unwrap();
    assert_eq!(back.encode(), text, "кодування стабільне");

    assert_eq!(back.today(), l.today());
    assert_eq!(back.mastered_skills(), l.mastered_skills());
    assert_eq!(back.tasks_today("A"), l.tasks_today("A"));
    assert_eq!(back.tasks_today("B"), l.tasks_today("B"));
    assert_eq!(back.due_reviews(&c), l.due_reviews(&c));
    assert_eq!(back.bug_count("B", "add_denominators"), 2);
    assert_eq!(back.bug_hypotheses("B", 2), l.bug_hypotheses("B", 2));
    assert_eq!(back.ask_adult_items(), l.ask_adult_items());
    assert!(back.has_seen("B", "Обчисли: 1/5 + 2/5"));
    assert!(back.has_seen("B", "Умова з пробілами,\nновим рядком, % і \\ та «лапками»"));
    assert!(back.is_exhausted("C") && !back.is_exhausted("B"));
    for skill in ["A", "B"] {
        let (x, y) = (l.skill(skill).unwrap(), back.skill(skill).unwrap());
        assert_eq!(
            x.p_known.to_bits(),
            y.p_known.to_bits(),
            "точне значення f64 для {skill}"
        );
        assert_eq!((x.streak, x.attempts), (y.streak, y.attempts));
        assert_eq!(x.clean_days(), y.clean_days());
    }
    // Мікротест у процесі переживає збереження: після ще двох задач він завершується так само.
    let (mut l, mut back) = (l, back);
    let p = BktParams::numeric_default();
    let a: Vec<_> = (0..2)
        .map(|_| l.record_task(&p, &c, "A", true, 0, None))
        .collect();
    let b: Vec<_> = (0..2)
        .map(|_| back.record_task(&p, &c, "A", true, 0, None))
        .collect();
    assert_eq!(a, b);
    assert_eq!(l.encode(), back.encode());
}

#[test]
fn the_order_of_seen_statements_survives_so_recycling_behaves_the_same() {
    let mut l = LearnerState::new();
    for n in 0..30 {
        l.mark_seen("S", &format!("умова {n}"));
    }
    let mut back = LearnerState::decode(&l.encode()).unwrap();
    assert!(l.recycle_seen("S", 5) && back.recycle_seen("S", 5));
    for n in 0..30 {
        let t = format!("умова {n}");
        assert_eq!(l.has_seen("S", &t), back.has_seen("S", &t), "{t}");
    }
    assert!(back.has_seen("S", "умова 29") && !back.has_seen("S", "умова 0"));
}

#[test]
fn skill_state_parts_round_trip() {
    let p = BktParams::numeric_default();
    let mut s = SkillState::new(&p);
    s.observe(&p, 3, true, 0);
    s.observe(&p, 7, true, 0);
    let t = SkillState::from_parts(s.p_known, s.streak, s.attempts, s.clean_days().clone());
    assert_eq!(t.p_known.to_bits(), s.p_known.to_bits());
    assert_eq!(t.clean_days(), &BTreeSet::from([3, 7]));
}

#[test]
fn day_usage_parts_round_trip() {
    let c = Config::default();
    let mut d = DayUsage::new();
    d.begin_day(5);
    d.start(1_000, false, false, &c).unwrap();
    d.finish(2_500);
    let (day, started, last_end) = d.parts();
    let e = DayUsage::from_parts(day, started, last_end);
    assert_eq!(e.parts(), d.parts());
    assert_eq!(e.sessions_started(), 1);
}

#[test]
fn hostile_or_broken_input_is_an_error_not_a_panic() {
    let (l, _) = rich_learner();
    let good = l.encode();
    let cases: Vec<String> = vec![
        String::new(),
        "garbage".into(),
        "KMT-LEARNER 999\n".into(),
        "KMT-LEARNER 1\nunknown thing\n".into(),
        "KMT-LEARNER 1\nskill A zzzz 1 1 \n".into(),
        "KMT-LEARNER 1\nskill A 3ff0000000000000 1 1 \n".into(), // p_known = 1.0, поза (0, 1)
        "KMT-LEARNER 1\nskill A 7ff8000000000000 1 1 \n".into(), // NaN
        "KMT-LEARNER 1\nseen A bad%zzescape\n".into(),
        "KMT-LEARNER 1\nschedule A 9 5\n".into(), // рівень поза межами
        "KMT-LEARNER 1\ntoday notanumber\n".into(),
        "KMT-LEARNER 1\nskill A\n".into(),
        cut_at_a_char_boundary(&good, good.len() / 2), // обрізаний посередині
    ];
    for (i, c) in cases.iter().enumerate() {
        let r = LearnerState::decode(c);
        // Деякі «обрізані» тексти можуть бути коректними; головне, що паніки немає.
        if i < 11 {
            assert!(r.is_err(), "випадок {i} мав бути помилкою: {c:?}");
        }
    }
    let huge = "x".repeat(9 * 1024 * 1024);
    assert!(matches!(
        LearnerState::decode(&huge),
        Err(CodecError::TooLarge)
    ));
}

#[test]
fn every_prefix_of_a_valid_text_decodes_or_errors_without_panicking() {
    let (l, _) = rich_learner();
    let good = l.encode();
    for end in 0..good.len() {
        if good.is_char_boundary(end) {
            let _ = LearnerState::decode(&good[..end]);
        }
    }
}

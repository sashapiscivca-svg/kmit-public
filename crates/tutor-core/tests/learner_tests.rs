use tutor_core::bkt::BktParams;
use tutor_core::config::Config;
use tutor_core::learner::{LearnerState, MicrotestOutcome};

fn setup() -> (BktParams, Config, LearnerState) {
    (
        BktParams::numeric_default(),
        Config::default(),
        LearnerState::new(),
    )
}

/// Доводить навичку до опанування: чисті успіхи у два різні дні.
fn master(l: &mut LearnerState, p: &BktParams, c: &Config, skill: &str, day0: u32) {
    l.begin_day(day0);
    for _ in 0..6 {
        l.record_task(p, c, skill, true, 0, None);
    }
    l.begin_day(day0 + 1);
    for _ in 0..6 {
        l.record_task(p, c, skill, true, 0, None);
    }
}

#[test]
fn counts_tasks_per_day_and_resets_only_when_the_day_changes() {
    let (p, c, mut l) = setup();
    l.begin_day(3);
    l.record_task(&p, &c, "A", true, 0, None);
    l.record_task(&p, &c, "A", false, 1, None);
    l.record_task(&p, &c, "B", true, 0, None);
    assert_eq!(
        (l.tasks_today("A"), l.tasks_today("B"), l.tasks_today("C")),
        (2, 1, 0)
    );
    l.begin_day(3);
    assert_eq!(l.tasks_today("A"), 2, "той самий день: лічильник лишається");
    l.begin_day(4);
    assert_eq!(l.tasks_today("A"), 0);
    assert_eq!(l.today(), 4);
}

#[test]
fn mastery_needs_two_days_and_schedules_the_first_review() {
    let (p, c, mut l) = setup();
    l.begin_day(0);
    for _ in 0..8 {
        l.record_task(&p, &c, "A", true, 0, None);
    }
    assert!(!l.is_mastered("A"), "один день не дає опанування");
    l.begin_day(1);
    let mut newly = 0;
    for _ in 0..4 {
        if l.record_task(&p, &c, "A", true, 0, None).newly_mastered {
            newly += 1;
        }
    }
    assert_eq!(newly, 1, "опанування фіксується рівно один раз");
    assert!(l.is_mastered("A"));
    let s = l.schedule("A").unwrap();
    assert_eq!(s.due_day(), 2, "перше повторення на наступний день");
}

#[test]
fn a_hint_or_mistake_never_counts_toward_mastery_days() {
    let (p, c, mut l) = setup();
    l.begin_day(0);
    for _ in 0..8 {
        l.record_task(&p, &c, "A", true, 0, None);
    }
    l.begin_day(1);
    for _ in 0..8 {
        l.record_task(&p, &c, "A", true, 1, None); // правильно, але з підказкою
    }
    assert!(!l.is_mastered("A"));
}

#[test]
fn mastery_is_sticky_and_reviews_update_the_schedule() {
    let (p, c, mut l) = setup();
    master(&mut l, &p, &c, "A", 0); // опановано в день 1, повторення в день 2
    assert!(l.is_mastered("A"));

    // Практика в день, коли повторення ще не настало, розклад не змінює.
    let before = *l.schedule("A").unwrap();
    let r = l.record_task(&p, &c, "A", true, 0, None);
    assert!(!r.was_review);
    assert_eq!(*l.schedule("A").unwrap(), before);

    // День 2: повторення настало, чистий успіх піднімає рівень.
    l.begin_day(2);
    assert_eq!(l.due_reviews(&c).len(), 1);
    let r = l.record_task(&p, &c, "A", true, 0, None);
    assert!(r.was_review && !r.newly_mastered);
    assert_eq!(l.schedule("A").unwrap().level(), 1);
    assert_eq!(l.schedule("A").unwrap().due_day(), 2 + 3);

    // День 5: невдале повторення скидає рівень, але навичка лишається опанованою.
    l.begin_day(5);
    let r = l.record_task(&p, &c, "A", false, 0, None);
    assert!(r.was_review);
    assert_eq!(l.schedule("A").unwrap().level(), 0);
    assert!(l.is_mastered("A"));
}

#[test]
fn due_reviews_are_capped_and_oldest_first() {
    let (p, c, mut l) = setup();
    for (i, s) in ["S1", "S2", "S3", "S4", "S5"].iter().enumerate() {
        master(&mut l, &p, &c, s, u32::try_from(i).unwrap() * 2);
    }
    l.begin_day(100);
    let due = l.due_reviews(&c);
    assert_eq!(due.len(), 3, "без «лавини боргу»");
    let overdue: Vec<u32> = due.iter().map(|d| d.overdue_days).collect();
    let mut sorted = overdue.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    assert_eq!(overdue, sorted, "найстарші першими");
    assert!(
        due.iter().all(|d| d.microtest),
        "після такого пропуску пропонується мікротест"
    );
}

#[test]
fn microtest_flag_appears_only_after_a_long_gap() {
    let (p, c, mut l) = setup();
    master(&mut l, &p, &c, "A", 0); // повторення в день 2
    l.begin_day(10);
    assert!(!l.due_reviews(&c)[0].microtest);
    l.begin_day(40);
    assert!(l.due_reviews(&c)[0].microtest);
}

#[test]
fn a_single_bug_hit_is_not_a_hypothesis_and_different_bugs_are_counted_apart() {
    let (p, c, mut l) = setup();
    l.begin_day(0);
    l.record_task(&p, &c, "A", false, 0, Some("b1"));
    assert_eq!(l.bug_count("A", "b1"), 1);
    assert!(
        l.bug_hypotheses("A", 2).is_empty(),
        "одиничний збіг не є гіпотезою"
    );
    l.record_task(&p, &c, "A", false, 1, Some("b1"));
    l.record_task(&p, &c, "A", false, 0, Some("b2"));
    l.record_task(&p, &c, "A", true, 0, None);
    assert_eq!(l.bug_count("A", "b1"), 2);
    assert_eq!(l.bug_count("A", "b2"), 1);
    assert_eq!(l.bug_hypotheses("A", 2), vec!["b1".to_string()]);
    assert!(l.bug_hypotheses("B", 1).is_empty());
}

#[test]
fn seen_statements_variant_cursor_and_exhaustion() {
    let mut l = LearnerState::new();
    assert!(!l.has_seen("A", "2+2"));
    assert!(l.mark_seen("A", "2+2"));
    assert!(!l.mark_seen("A", "2+2"), "повторна умова вже бачена");
    assert!(
        l.mark_seen("B", "2+2"),
        "для іншої навички та сама умова нова"
    );
    assert_eq!(
        (
            l.next_variant("A"),
            l.next_variant("A"),
            l.next_variant("B")
        ),
        (1, 2, 1)
    );
    assert!(!l.is_exhausted("A"));
    l.mark_exhausted("A");
    assert!(l.is_exhausted("A") && !l.is_exhausted("B"));
}

#[test]
fn the_review_cap_is_per_day_not_per_request() {
    let (p, c, mut l) = setup();
    for (i, s) in ["S1", "S2", "S3", "S4", "S5"].iter().enumerate() {
        master(&mut l, &p, &c, s, u32::try_from(i).unwrap() * 2);
    }
    l.begin_day(100);
    let mut done = 0;
    while let Some(d) = l.due_reviews(&c).first().cloned() {
        l.record_task(&p, &c, &d.skill, true, 0, None);
        done += 1;
        assert!(done <= 3, "більше трьох повторень за день");
    }
    assert_eq!(done, 3);
    assert!(
        l.due_reviews(&c).is_empty(),
        "ліміт дня вичерпано, хоча прострочені лишились"
    );
    l.begin_day(101);
    assert!(
        !l.due_reviews(&c).is_empty(),
        "наступного дня ліміт оновлюється"
    );
}

#[test]
fn a_device_clock_moved_backwards_or_far_ahead_does_not_panic_or_create_debt() {
    let (p, c, mut l) = setup();
    master(&mut l, &p, &c, "A", 100); // опановано в день 101, повторення в день 102

    // Годинник відкотили назад: нічого не прострочено, лічильники дня обнуляються, паніки немає.
    l.begin_day(50);
    assert_eq!(l.today(), 50);
    assert!(l.due_reviews(&c).is_empty());
    l.record_task(&p, &c, "A", true, 0, None);

    // Годинник стрибнув у далеке майбутнє: одне повторення з мікротестом, без переповнення.
    l.begin_day(u32::MAX);
    let due = l.due_reviews(&c);
    assert_eq!(due.len(), 1);
    assert!(due[0].microtest);
    // Мікротест складається з кількох задач; після його проходження арифметика днів насичується.
    let mut last = None;
    for _ in 0..c.microtest_tasks {
        let r = l.record_task(&p, &c, "A", true, 0, None);
        assert!(r.was_review);
        last = r.microtest;
    }
    assert_eq!(last, Some(MicrotestOutcome::Passed));
    assert_eq!(
        l.schedule("A").unwrap().due_day(),
        u32::MAX,
        "арифметика днів насичується"
    );
}

#[test]
fn due_reviews_skip_exhausted_skills_without_hiding_the_others() {
    let (p, c, mut l) = setup();
    for (i, s) in ["S1", "S2", "S3", "S4"].iter().enumerate() {
        master(&mut l, &p, &c, s, u32::try_from(i).unwrap() * 2);
    }
    l.begin_day(100);
    l.mark_exhausted("S1"); // найстарше прострочення
    let due = l.due_reviews(&c);
    assert_eq!(due.len(), 3, "ліміт заповнюють придатні навички");
    assert!(due.iter().all(|d| d.skill != "S1"));
}

#[test]
fn bug_evidence_counts_even_when_the_task_was_finally_solved() {
    // Типовий шлях з драбиною: хибна відповідь, підказка, далі правильно. Помилка все одно була.
    let (p, c, mut l) = setup();
    l.begin_day(0);
    l.record_task(&p, &c, "A", true, 1, Some("add_denominators"));
    l.record_task(&p, &c, "A", true, 2, Some("add_denominators"));
    assert_eq!(l.bug_count("A", "add_denominators"), 2);
    assert_eq!(
        l.bug_hypotheses("A", 2),
        vec!["add_denominators".to_string()]
    );
}

#[test]
fn recycling_forgets_all_but_the_most_recent_statements() {
    let mut l = LearnerState::new();
    for n in 0..5 {
        l.mark_seen("A", &format!("s{n}"));
    }
    assert!(
        !l.recycle_seen("A", 5),
        "нічого забувати: бачених не більше, ніж треба тримати"
    );
    assert!(l.recycle_seen("A", 2));
    assert!(!l.has_seen("A", "s0") && !l.has_seen("A", "s1") && !l.has_seen("A", "s2"));
    assert!(l.has_seen("A", "s3") && l.has_seen("A", "s4"));
    assert!(l.mark_seen("A", "s0"), "забуті умови знову нові");
    assert!(!l.recycle_seen("B", 2), "інша навичка порожня");
}

// ---- відкат опанування (spec 5.3: відкат при P(L) < 0.6) і мікротест (spec 5.5)

fn fail_until_below(
    l: &mut LearnerState,
    p: &BktParams,
    c: &Config,
    skill: &str,
    limit: f64,
) -> u32 {
    let mut n = 0;
    while l.skill(skill).unwrap().p_known >= limit && n < 50 {
        l.record_task(p, c, skill, false, 0, None);
        n += 1;
    }
    n
}

#[test]
fn a_slip_does_not_undo_mastery() {
    let (p, c, mut l) = setup();
    master(&mut l, &p, &c, "A", 0);
    let r = l.record_task(&p, &c, "A", false, 0, None);
    assert!(!r.rolled_back);
    assert!(l.is_mastered("A"));
}

#[test]
fn mastery_is_rolled_back_when_the_estimate_falls_below_the_threshold_and_can_be_regained() {
    let (p, c, mut l) = setup();
    master(&mut l, &p, &c, "A", 0);
    let mut rolled = 0;
    for _ in 0..12 {
        let r = l.record_task(&p, &c, "A", false, 0, None);
        if r.rolled_back {
            rolled += 1;
        }
    }
    assert_eq!(rolled, 1, "відкат фіксується один раз");
    assert!(!l.is_mastered("A"));
    assert!(
        l.schedule("A").is_none(),
        "розклад повторень знято до нового опанування"
    );
    assert!(l.skill("A").unwrap().p_known < c.rollback_p);

    // Знову опановує чистими успіхами: «нове опанування» фіксується ще раз.
    l.begin_day(5);
    let mut again = 0;
    for _ in 0..12 {
        if l.record_task(&p, &c, "A", true, 0, None).newly_mastered {
            again += 1;
        }
    }
    assert_eq!(again, 1);
    assert!(l.is_mastered("A") && l.schedule("A").is_some());
}

#[test]
fn a_weak_but_not_rolled_back_skill_stays_mastered() {
    let (p, c, mut l) = setup();
    master(&mut l, &p, &c, "A", 0);
    fail_until_below(&mut l, &p, &c, "A", c.readiness_p);
    let st = l.skill("A").unwrap().p_known;
    assert!(
        st < c.readiness_p && st >= c.rollback_p,
        "оцінка {st} між відкатом і готовністю"
    );
    assert!(l.is_mastered("A"));
}

#[test]
fn after_a_long_gap_the_review_is_a_microtest_that_must_be_passed() {
    let (p, c, mut l) = setup();
    master(&mut l, &p, &c, "A", 0); // повторення настало б у день 2
    l.begin_day(60); // понад 30 днів прострочення
    let first = l.record_task(&p, &c, "A", true, 0, None);
    assert_eq!(first.microtest, Some(MicrotestOutcome::InProgress));
    assert!(first.was_review);
    assert_eq!(
        l.schedule("A").unwrap().due_day(),
        2,
        "розклад не змінюється, доки мікротест не завершено"
    );
    l.record_task(&p, &c, "A", true, 0, None);
    let last = l.record_task(&p, &c, "A", true, 0, None);
    assert_eq!(last.microtest, Some(MicrotestOutcome::Passed));
    assert!(l.is_mastered("A"));
    let s = l.schedule("A").unwrap();
    assert_eq!(
        (s.level(), s.due_day()),
        (0, 61),
        "після мікротесту інтервали починаються спочатку"
    );
}

#[test]
fn a_failed_microtest_rolls_the_skill_back_to_relearning() {
    let (p, c, mut l) = setup();
    master(&mut l, &p, &c, "A", 0);
    l.begin_day(60);
    l.record_task(&p, &c, "A", true, 0, None);
    l.record_task(&p, &c, "A", false, 0, None);
    let last = l.record_task(&p, &c, "A", false, 2, None);
    assert_eq!(last.microtest, Some(MicrotestOutcome::Failed));
    assert!(last.rolled_back);
    assert!(!l.is_mastered("A"));
    assert!(l.schedule("A").is_none());
}

#[test]
fn a_microtest_uses_up_the_daily_review_allowance() {
    let (p, c, mut l) = setup();
    master(&mut l, &p, &c, "A", 0);
    master(&mut l, &p, &c, "B", 2);
    l.begin_day(60);
    for _ in 0..3 {
        l.record_task(&p, &c, "A", true, 0, None);
    }
    assert!(
        l.due_reviews(&c).is_empty(),
        "три задачі мікротесту вичерпали денний ліміт повторень"
    );
    l.begin_day(61);
    let due = l.due_reviews(&c);
    assert!(
        due.iter().any(|d| d.skill == "B" && d.microtest),
        "наступного дня черга B"
    );
}

#[test]
fn the_ask_adult_list_holds_skill_and_statement_only_and_can_be_resolved() {
    let mut l = LearnerState::new();
    l.record_ask_adult("F-ADD-DIFF", "Обчисли: 1/3 + 1/6");
    l.record_ask_adult("F-ADD-DIFF", "Обчисли: 1/3 + 1/6"); // дубль не множиться
    l.record_ask_adult("D-ADD", "Обчисли: 8,9 + 8,8");
    assert_eq!(
        l.ask_adult_items(),
        vec![
            ("F-ADD-DIFF".to_string(), "Обчисли: 1/3 + 1/6".to_string()),
            ("D-ADD".to_string(), "Обчисли: 8,9 + 8,8".to_string()),
        ]
    );
    assert!(l.resolve_ask_adult("D-ADD", "Обчисли: 8,9 + 8,8"));
    assert!(
        !l.resolve_ask_adult("D-ADD", "Обчисли: 8,9 + 8,8"),
        "вже знято"
    );
    assert_eq!(l.ask_adult_items().len(), 1);
}

#[test]
fn bug_evidence_is_cleared_when_the_skill_is_mastered() {
    let (p, c, mut l) = setup();
    l.begin_day(0);
    l.record_task(&p, &c, "A", false, 0, Some("b1"));
    l.record_task(&p, &c, "A", false, 1, Some("b1"));
    assert_eq!(l.bug_hypotheses("A", 2), vec!["b1".to_string()]);
    for _ in 0..8 {
        l.record_task(&p, &c, "A", true, 0, None);
    }
    l.begin_day(1);
    for _ in 0..8 {
        l.record_task(&p, &c, "A", true, 0, None);
    }
    assert!(l.is_mastered("A"));
    assert_eq!(
        l.bug_count("A", "b1"),
        0,
        "опановану навичку старі помилки більше не характеризують"
    );
    assert!(l.bug_hypotheses("A", 1).is_empty());
}

#[test]
fn the_ask_adult_list_keeps_the_order_in_which_tasks_were_added() {
    let mut l = LearnerState::new();
    l.record_ask_adult("Z-SKILL", "умова 1");
    l.record_ask_adult("A-SKILL", "умова 2");
    l.record_ask_adult("Z-SKILL", "умова 1"); // дубль
    l.record_ask_adult("M-SKILL", "умова 3");
    assert_eq!(
        l.ask_adult_items(),
        vec![
            ("Z-SKILL".to_string(), "умова 1".to_string()),
            ("A-SKILL".to_string(), "умова 2".to_string()),
            ("M-SKILL".to_string(), "умова 3".to_string()),
        ],
        "порядок додавання, не алфавіт"
    );
    let back = LearnerState::decode(&l.encode()).unwrap();
    assert_eq!(
        back.ask_adult_items(),
        l.ask_adult_items(),
        "порядок переживає збереження"
    );
}

#[test]
fn the_daily_total_survives_a_restart_and_resets_with_the_day() {
    let (p, c, mut l) = setup();
    l.begin_day(5);
    l.record_task(&p, &c, "A", true, 0, None);
    l.record_task(&p, &c, "B", false, 1, None);
    l.record_task(&p, &c, "A", true, 0, None);
    assert_eq!(l.tasks_today_total(), 3);
    let back = LearnerState::decode(&l.encode()).unwrap();
    assert_eq!(
        back.tasks_today_total(),
        3,
        "слот перемежування переживає перезапуск застосунку"
    );
    let mut next = back;
    next.begin_day(6);
    assert_eq!(next.tasks_today_total(), 0);
}

#[test]
fn a_passed_test_out_grants_mastery_with_a_review_schedule_and_is_idempotent() {
    let (p, c) = (BktParams::numeric_default(), Config::default());
    let mut l = LearnerState::new();
    l.begin_day(10);
    assert!(!l.is_mastered("N-ADD-MULTI"));
    assert!(
        l.grant_mastery(&p, &c, "N-ADD-MULTI"),
        "перший раз: нове опанування"
    );
    assert!(l.is_mastered("N-ADD-MULTI"));
    assert!(l.skill("N-ADD-MULTI").unwrap().p_known >= c.mastery_p);
    assert_eq!(
        l.schedule("N-ADD-MULTI").unwrap().due_day(),
        11,
        "перше повторення за інтервалом"
    );
    assert!(
        !l.grant_mastery(&p, &c, "N-ADD-MULTI"),
        "повторно нічого не змінює"
    );
    // Подальші задачі не знімають опанування одразу.
    let r = l.record_task(&p, &c, "N-ADD-MULTI", true, 0, None);
    assert!(!r.rolled_back && !r.newly_mastered);
    assert!(l.is_mastered("N-ADD-MULTI"));
}

#[test]
fn memory_strength_fades_between_reviews_and_all_due_ignores_the_daily_cap() {
    use tutor_core::bkt::BktParams;
    let cfg = tutor_core::config::Config::default();
    let params = BktParams::numeric_default();
    let mut l = tutor_core::learner::LearnerState::new();
    l.begin_day(1);
    for s in ["A", "B", "C", "D", "E"] {
        l.grant_mastery(&params, &cfg, s);
    }
    // Одразу після опанування пам'ять свіжа, у день повторення вже 33%, а через півтора інтервалу 0%.
    assert_eq!(l.memory_pct("A", &cfg), Some(100));
    l.begin_day(2);
    assert_eq!(l.memory_pct("A", &cfg), Some(34));
    l.begin_day(3);
    assert_eq!(l.memory_pct("A", &cfg), Some(0));
    assert_eq!(l.memory_pct("NOPE", &cfg), None);
    // Звичайний вибір обмежений трьома повтореннями за день, режим «Повторення» бачить усе.
    assert!(l.due_reviews(&cfg).len() <= cfg.max_reviews_per_session);
    assert_eq!(l.all_due_reviews(&cfg).len(), 5);
}

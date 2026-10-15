use course_pack::grade5_m0_graph;
use tutor_core::bkt::BktParams;
use tutor_core::config::Config;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::learner::LearnerState;
use tutor_core::task::Task;
use tutor_engine::{note_analog_shown, pick_task, select_skill, Pick, Reason, Selection};

fn env() -> (BktParams, Config, LearnerState) {
    (
        BktParams::numeric_default(),
        Config::default(),
        LearnerState::new(),
    )
}

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

/// Опановує всі навички курсу в порядку передумов (кожна наступна через `step` днів); повертає день після останньої.
fn master_whole_course(l: &mut LearnerState, p: &BktParams, c: &Config, step: u32) -> u32 {
    let order = grade5_m0_graph().topological_order().unwrap();
    let mut day = 0;
    for skill in order {
        master(l, p, c, skill, day);
        day += step;
    }
    day
}

fn pick(l: &LearnerState, idx: u32) -> Option<Selection> {
    let (p, c, _) = env();
    select_skill(&grade5_m0_graph(), l, &p, &c, idx)
}

fn pick_enforced(l: &LearnerState, idx: u32) -> Option<Selection> {
    let (p, mut c, _) = env();
    c.enforce_limits = true;
    select_skill(&grade5_m0_graph(), l, &p, &c, idx)
}

#[test]
fn a_new_learner_starts_with_the_only_available_skill() {
    let (_, _, l) = env();
    assert_eq!(
        pick(&l, 0),
        Some(Selection {
            skill: "N-ADD-MULTI".into(),
            reason: Reason::New
        })
    );
}

#[test]
fn a_started_skill_is_continued() {
    let (p, c, mut l) = env();
    l.begin_day(0);
    l.record_task(&p, &c, "N-ADD-MULTI", true, 0, None);
    assert_eq!(
        pick(&l, 1),
        Some(Selection {
            skill: "N-ADD-MULTI".into(),
            reason: Reason::Continue
        })
    );
}

#[test]
fn prerequisites_gate_new_skills() {
    let (p, c, mut l) = env();
    l.begin_day(0);
    for _ in 0..6 {
        l.record_task(&p, &c, "N-ADD-MULTI", true, 0, None);
    }
    // Один день: ще не опановано, тож залежні навички недоступні.
    assert_eq!(pick(&l, 6).unwrap().skill, "N-ADD-MULTI");
    master(&mut l, &p, &c, "N-ADD-MULTI", 1);
    l.begin_day(3);
    // Тепер доступні лише ті, чиї передумови опановано; залежні від неопанованих (F-ADD-DIFF, N-DIV-MULTI, A-ANGLE, P-PCT) ще ні.
    let s = pick(&l, 0).unwrap();
    assert_eq!(s.reason, Reason::New);
    assert!(
        [
            "D-ADD",
            "F-ADD-SAME",
            "N-SUB-MULTI",
            "N-MUL-MULTI",
            "N-ROUND"
        ]
        .contains(&s.skill.as_str()),
        "{s:?}"
    );
    for locked in ["F-ADD-DIFF", "N-DIV-MULTI", "A-ANGLE", "P-PCT"] {
        assert_ne!(s.skill, locked);
    }
}

#[test]
fn the_most_reachable_skill_wins_over_a_brand_new_one() {
    let (p, c, mut l) = env();
    master(&mut l, &p, &c, "N-ADD-MULTI", 0);
    l.begin_day(3);
    // Один чистий успіх: P(L) ≈ 0.90, прогноз успіху ≈ 0.81, поруч із ціллю 0.8.
    l.record_task(&p, &c, "F-ADD-SAME", true, 0, None);
    assert_eq!(
        pick(&l, 0),
        Some(Selection {
            skill: "F-ADD-SAME".into(),
            reason: Reason::Continue
        })
    );
}

#[test]
fn the_daily_cap_moves_the_learner_on_and_then_ends_the_day() {
    let (p, c, mut l) = env();
    l.begin_day(0);
    for _ in 0..c.max_tasks_per_skill_per_day {
        l.record_task(&p, &c, "N-ADD-MULTI", true, 0, None);
    }
    assert_eq!(
        pick_enforced(&l, 8),
        None,
        "єдина доступна навичка вичерпала денний ліміт"
    );
    l.begin_day(1);
    assert_eq!(pick_enforced(&l, 0).unwrap().skill, "N-ADD-MULTI");
}

#[test]
fn every_nth_task_is_a_review_when_one_is_due() {
    let (p, c, mut l) = env();
    master(&mut l, &p, &c, "N-ADD-MULTI", 0); // повторення настає в день 2
    l.begin_day(2);
    let first = pick(&l, 0).unwrap();
    assert_ne!(
        first.reason,
        Reason::Review { microtest: false },
        "слот ще не настав"
    );
    assert_eq!(
        pick(&l, 2),
        Some(Selection {
            skill: "N-ADD-MULTI".into(),
            reason: Reason::Review { microtest: false }
        })
    );
}

#[test]
fn off_slot_reviews_are_offered_when_nothing_new_remains() {
    let (p, c, mut l) = env();
    master_whole_course(&mut l, &p, &c, 10);
    l.begin_day(300);
    // Усе опановано; прострочені повторення є, а нового немає.
    let s = pick(&l, 0).unwrap();
    assert!(
        matches!(s.reason, Reason::Review { microtest: true }),
        "після довгого пропуску мікротест: {s:?}"
    );
}

#[test]
fn nothing_to_do_means_the_day_is_over() {
    let (p, c, mut l) = env();
    let last_day = master_whole_course(&mut l, &p, &c, 2);
    l.begin_day(last_day + 2);
    // Усе опановано. Виконуємо всі прострочені повторення чистими успіхами.
    let mut guard = 0;
    while let Some(d) = l.due_reviews(&c).first().cloned() {
        l.record_task(&p, &c, &d.skill, true, 0, None);
        guard += 1;
        assert!(guard < 20, "повторення мали закінчитися");
    }
    assert!(l.due_reviews(&c).is_empty());
    assert_eq!(
        pick(&l, 0),
        None,
        "нового немає й повторень немає: на сьогодні все"
    );
    assert_eq!(pick(&l, 2), None, "навіть у слоті перемежування");
}

#[test]
fn exhausted_skills_are_skipped() {
    let (_, _, mut l) = env();
    l.mark_exhausted("N-ADD-MULTI");
    assert_eq!(pick(&l, 0), None, "єдину доступну навичку вичерпано");
}

#[test]
fn selection_is_deterministic() {
    let (p, c, mut l) = env();
    master(&mut l, &p, &c, "N-ADD-MULTI", 0);
    l.begin_day(3);
    let a: Vec<_> = (0..10).map(|i| pick(&l, i)).collect();
    let b: Vec<_> = (0..10).map(|i| pick(&l, i)).collect();
    assert_eq!(a, b);
}

// ---- вибір задач

struct TinyGen; // лише три різні умови, решта повторюється
impl SkillGenerator for TinyGen {
    fn skill_id(&self) -> &'static str {
        "TINY"
    }
    fn generate(&self, variant: u64) -> Task {
        let n = i64::try_from(variant % 3).unwrap();
        Task {
            skill: "TINY",
            variant,
            statement: format!("Обчисли: {} + 1", n + 10),
            operands: vec![],
            operand_labels: vec![],
            params: vec![],
            answer: tutor_core::frac::Frac::int(n + 11),
            steps: vec![],
            bugs: vec![],
        }
    }
    fn hint(&self, _t: &Task, _l: ScriptedLevel) -> String {
        String::new()
    }
    fn analog(&self, _t: &Task, _a: &[tutor_core::frac::Frac]) -> Option<WorkedExample> {
        None
    }
}

#[test]
fn picked_tasks_never_repeat_a_statement_and_exhaustion_is_reported() {
    let mut l = LearnerState::new();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..3 {
        match pick_task(&TinyGen, &mut l, &Config::default()) {
            Pick::Task(t) => assert!(seen.insert(t.statement.clone()), "умова повторилась"),
            Pick::Exhausted => panic!("рано вичерпано"),
        }
    }
    assert!(matches!(
        pick_task(&TinyGen, &mut l, &Config::default()),
        Pick::Exhausted
    ));
    assert!(l.is_exhausted("TINY"));
}

#[test]
fn real_generators_give_distinct_tasks_and_analogs_count_as_seen() {
    for g in generators_math::all_generators() {
        let mut l = LearnerState::new();
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..60 {
            let Pick::Task(t) = pick_task(g.as_ref(), &mut l, &Config::default()) else {
                panic!("{}: рано вичерпано", g.skill_id())
            };
            assert!(
                seen.insert(t.statement.clone()),
                "{}: повтор «{}»",
                g.skill_id(),
                t.statement
            );
            let ex = tutor_core::guard::guarded_analog(g.as_ref(), &t, &[]).expect("аналог є");
            note_analog_shown(&mut l, &ex);
            assert!(l.has_seen(g.skill_id(), &ex.task.statement));
        }
    }
}

#[test]
fn an_exhausted_mastered_skill_is_not_offered_for_review() {
    let (p, c, mut l) = env();
    master(&mut l, &p, &c, "N-ADD-MULTI", 0);
    l.mark_exhausted("N-ADD-MULTI");
    l.begin_day(2); // повторення настало, але умов для цієї навички більше немає
                    // Слот перемежування і запасний шлях не повинні повертати те, що неможливо видати.
    for idx in 0..6 {
        let s = pick(&l, idx);
        assert!(
            s.as_ref().is_none_or(|s| s.skill != "N-ADD-MULTI"),
            "idx {idx}: запропоновано вичерпану навичку {s:?}"
        );
    }
}

#[test]
fn a_started_skill_keeps_priority_over_new_ones_after_a_hinted_first_task() {
    let (p, c, mut l) = env();
    master(&mut l, &p, &c, "N-ADD-MULTI", 0);
    l.begin_day(3);
    let first = pick(&l, 0).unwrap();
    assert_eq!(first.reason, Reason::New);
    // Яку саме нову навичку обрано, залежить від порядку графа; ключове лише те, що її не кидають.
    let started = first.skill.clone();
    // Розв'язала з підказкою: не «чистий» успіх, оцінка знання падає нижче стартової.
    l.record_task(&p, &c, &started, true, 1, None);
    for idx in [1, 3, 4] {
        assert_eq!(
            pick(&l, idx),
            Some(Selection {
                skill: started.clone(),
                reason: Reason::Continue
            }),
            "idx {idx}: навичку кинули після однієї підказки"
        );
    }
}

#[test]
fn a_started_skill_that_hit_the_daily_cap_hands_over_to_a_new_one() {
    let (p, c, mut l) = env();
    master(&mut l, &p, &c, "N-ADD-MULTI", 0);
    l.begin_day(3);
    for _ in 0..c.max_tasks_per_skill_per_day {
        l.record_task(&p, &c, "D-ADD", false, 0, None);
    }
    // Доступні нові навички: F-ADD-SAME, N-SUB-MULTI, N-MUL-MULTI, N-ROUND (D-ADD уже вичерпала денний ліміт).
    let s = pick_enforced(&l, 1).unwrap();
    assert_eq!(s.reason, Reason::New);
    assert!(
        ["F-ADD-SAME", "N-SUB-MULTI", "N-MUL-MULTI", "N-ROUND"].contains(&s.skill.as_str()),
        "{s:?}"
    );
}

struct ThirtyGen; // тридцять різних умов: більше за вікно повторів, але скінченний простір
impl SkillGenerator for ThirtyGen {
    fn skill_id(&self) -> &'static str {
        "THIRTY"
    }
    fn generate(&self, variant: u64) -> Task {
        let n = i64::try_from(variant % 30).unwrap();
        Task {
            skill: "THIRTY",
            variant,
            statement: format!("Обчисли: {n} + 1"),
            operands: vec![],
            operand_labels: vec![],
            params: vec![],
            answer: tutor_core::frac::Frac::int(n + 1),
            steps: vec![],
            bugs: vec![],
        }
    }
    fn hint(&self, _t: &Task, _l: ScriptedLevel) -> String {
        String::new()
    }
    fn analog(&self, _t: &Task, _a: &[tutor_core::frac::Frac]) -> Option<WorkedExample> {
        None
    }
}

#[test]
fn a_finite_pool_is_recycled_in_new_rounds_without_close_repeats() {
    let cfg = Config::default();
    let mut l = LearnerState::new();
    let mut order = Vec::new();
    for _ in 0..120 {
        match pick_task(&ThirtyGen, &mut l, &cfg) {
            Pick::Task(t) => order.push(t.statement.clone()),
            Pick::Exhausted => {
                panic!("пул у 30 умов не має вичерпуватися: умови можна повторювати в новому колі")
            }
        }
    }
    assert!(!l.is_exhausted("THIRTY"));
    for i in 0..order.len() {
        for j in (i + 1)..order.len() {
            if order[i] == order[j] {
                assert!(
                    j - i > cfg.repeat_guard_recent,
                    "умова «{}» повторилась через {} задач (вікно {})",
                    order[i],
                    j - i,
                    cfg.repeat_guard_recent
                );
            }
        }
    }
}

#[test]
fn a_mastered_but_shaky_prerequisite_does_not_unlock_the_next_skills() {
    let (p, c, mut l) = setup_env();
    master(&mut l, &p, &c, "N-ADD-MULTI", 0);
    l.begin_day(3);
    // Падіння оцінки нижче готовності, але вище відкату: опанована, проте ще не надійна.
    while l.skill("N-ADD-MULTI").unwrap().p_known >= c.readiness_p {
        l.record_task(&p, &c, "N-ADD-MULTI", false, 0, None);
    }
    assert!(l.is_mastered("N-ADD-MULTI"));
    let s = pick(&l, 1).unwrap();
    assert_eq!(s.skill, "N-ADD-MULTI", "спершу зміцнюємо передумову");
    assert_eq!(s.reason, Reason::Continue);
}

fn setup_env() -> (BktParams, Config, LearnerState) {
    env()
}

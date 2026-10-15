//! Потік сесії: вибір задачі, відповіді, підказки, завершення. Стан задачі тримається лише тут;
//! назовні виходять лише подання без відповідей.

use crate::app::{Env, Open};
use crate::catalog::{self, generator_for};
use crate::ctx::Ctx;
use crate::error::ApiError;
use crate::views::{skill_title, sticker_json};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use storage::state::{AppState, ItemSrs, MAX_ITEM_SRS};
use tutor_core::album::{StickerId, TaskFacts};
use tutor_core::frac::{parse_answer, Frac};
use tutor_core::gate::{GateDecision, HintGate};
use tutor_core::generator::AnswerKind;
use tutor_core::generator::{SkillGenerator, WorkedExample};
use tutor_core::ladder::{Ladder, LadderAction};
use tutor_core::lesson::LessonSource;
use tutor_core::session::next_hint;
use tutor_core::sessionctl::{EndReason, Session, SessionStatus};
use tutor_core::task::{Task, Verdict};
use tutor_core::view::{hint_view, task_view, ChildHint};
use tutor_engine::{note_analog_shown, pick_task, select_skill, Pick, Reason};

/// Найдовша відповідь, яку розбираємо (довша це сміття).
const MAX_INPUT_CHARS: usize = 40;

#[derive(Clone, Copy)]
pub(crate) enum Kind {
    New,
    Practice,
    Review,
    Microtest,
}

impl Kind {
    fn code(self) -> &'static str {
        match self {
            Kind::New => "new",
            Kind::Practice => "practice",
            Kind::Review => "review",
            Kind::Microtest => "microtest",
        }
    }
}

pub(crate) struct Current {
    pub skill: String,
    pub kind: Kind,
    pub gen: Box<dyn SkillGenerator>,
    pub task: Task,
    pub ladder: Ladder,
    pub gate: HintGate,
    pub rejected: Vec<Frac>,
    pub wrong: u32,
    pub bug: Option<&'static str>,
    pub started_at: u64,
    pub analog: Option<WorkedExample>,
    pub number: u32,
    /// Перевірка «я це вже вмію»: підказок немає.
    pub no_hints: bool,
    /// Це приклад, що раніше дався важко й тепер повернувся (прогалина пам'яті).
    pub hard_return: bool,
}

/// Перевірка «я це вже вмію»: кілька задач підряд без підказок і без помилок.
pub(crate) struct Testout {
    pub skill: String,
    pub total: u32,
    pub asked: u32,
    pub clean: u32,
    pub failed: bool,
}

pub(crate) struct FeedbackInfo {
    pub skill: String,
    /// Для повідомлення про проблему із завданням: (ключ, умова).
    pub flag_key: (String, String),
    /// Порада після серії помилок: `lesson` (перечитати пояснення) або `break` (відпочити).
    pub support: Option<&'static str>,
    /// Передумови, які варто повторити (для поради `prereq`).
    pub weak_prereqs: Vec<(String, String)>,
    /// Для завдань із вибором: (правильний варіант, пояснення) показуються після спроби.
    pub reveal: Option<(String, String)>,
    pub outcome: &'static str,
    pub stickers: Vec<StickerId>,
    pub mastered: Option<String>,
    pub suggest_break: bool,
    /// Що сталося з пам'яттю про цей приклад: `opened` (запам'ятали як складний), `closed` (подолано), `still` (ще не вийшло).
    pub gap: Option<&'static str>,
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum Phase {
    /// Нова тема: пропозиція прочитати пояснення до першої задачі (задачі ще немає).
    Intro {
        skill: String,
    },
    Task(Box<Current>),
    Analog(Box<Current>),
    Feedback(FeedbackInfo),
    Ended(EndReason),
}

pub(crate) struct Run {
    pub session: Session,
    pub phase: Phase,
    pub new_stickers: Vec<StickerId>,
    pub number: u32,
    /// Вільна практика: учень сам обрав навичку (будь-яку, навіть закриту за графом); розклад не втручається.
    pub focus: Option<String>,
    pub subject: String,
    /// Теми, для яких вступ уже показано в цій сесії.
    pub intro_done: BTreeSet<String>,
    pub testout: Option<Testout>,
    pub testout_result: Option<bool>,
    /// Скільки задач поспіль без правильної відповіді (усі теми) і за темами.
    pub miss_streak: u32,
    pub skill_miss: BTreeMap<String, u32>,
    /// Номер задачі, на якій пропущено завдання банку: повертається не раніше ніж через кілька інших.
    pub item_gap: BTreeMap<(String, u32), u32>,
    /// Теми, з яких були задачі в цій сесії (для запитання «що було складним»).
    pub practiced: BTreeSet<String>,
    /// Режим «Повторення»: лише теми, які пора повторити (розклад і пропущені завдання), чергуючись.
    pub review: Option<ReviewRun>,
}

/// Налаштування режиму «Повторення».
#[derive(Clone, Debug)]
pub(crate) struct ReviewRun {
    /// Скільки задач найбільше в одному повторенні.
    pub cap: u32,
    /// Скільки задач кожної теми вже було сьогодні на початку: щоб рахувати лише задачі цього повторення.
    pub base: BTreeMap<String, u32>,
}

/// Задач на одне повторення й на тему за раз.
const REVIEW_CAP: u32 = 10;
const REVIEW_PER_SKILL: u32 = 2;

/// Через скільки інших задач у сесії може повернутися щойно пропущене завдання банку.
const ITEM_GAP: u32 = 3;
/// Інтервали повторення пропущених завдань банку, дні (після помилки, 1, 3, 7).
const ITEM_DAYS: [u32; 5] = [0, 1, 3, 7, 14];
/// Скільки задач у перевірці «я це вже вмію».
const TESTOUT_TASKS: u32 = 3;
const TESTOUT_TASKS_RECALL: u32 = 5;

/// Допоміжні дані для вибору наступної задачі.
struct Aux<'a> {
    number: u32,
    focus: Option<&'a str>,
    subject: &'a str,
    intro_done: &'a BTreeSet<String>,
    item_gap: &'a BTreeMap<(String, u32), u32>,
    /// Відновлена після перезапуску сесія: вступ до теми не повторюємо.
    no_intro: bool,
    review: Option<&'a ReviewRun>,
}

/// Для завдань із вибором підказка доступна й до першої спроби (інакше помилка одразу розкриває відповідь).
fn gate_cfg(env: &Env, kind: AnswerKind) -> tutor_core::config::Config {
    let mut c = env.cfg.clone();
    if kind != AnswerKind::Number {
        c.require_attempt_before_hint = false;
    }
    c
}

/// Ключ завдання для повідомлень про проблеми: номер завдання банку чи варіант генератора.
fn item_key(task: &Task) -> String {
    task.params
        .first()
        .filter(|_| task.operands.is_empty())
        .map_or_else(|| task.variant.to_string(), ToString::to_string)
}

/// Повідомлення дитини про проблему із завданням, що зараз на екрані (задача чи відгук).
pub(crate) fn flag_item(env: &Env, open: &mut Open, args: &Value) -> Result<Value, ApiError> {
    let _ = env;
    let reason = args
        .get("reason")
        .and_then(Value::as_str)
        .filter(|r| storage::state::FLAG_REASONS.contains(r))
        .ok_or_else(|| ApiError::bad_request("невідома причина"))?
        .to_string();
    let run = open.run.as_ref().ok_or_else(ApiError::wrong_phase)?;
    let (skill, key, statement) = match &run.phase {
        Phase::Task(c) | Phase::Analog(c) => {
            (c.skill.clone(), item_key(&c.task), c.task.statement.clone())
        }
        Phase::Feedback(f) => (f.skill.clone(), f.flag_key.0.clone(), f.flag_key.1.clone()),
        _ => return Err(ApiError::wrong_phase()),
    };
    let flags = &mut open.state.flags;
    if let Some(f) = flags.iter_mut().find(|f| f.skill == skill && f.key == key) {
        f.reason = reason;
    } else if flags.len() < storage::state::MAX_FLAGS {
        flags.push(storage::state::ItemFlag {
            skill,
            key,
            reason,
            statement: statement.chars().take(200).collect(),
        });
    }
    Ok(json!({"count": flags.len()}))
}

fn placeholder() -> Phase {
    Phase::Ended(EndReason::UserEnded)
}

/// Вибирає наступну задачу або повідомляє, що на сьогодні все.
fn advance(env: &Env, st: &mut AppState, session: &Session, wall: u64, aux: &Aux<'_>) -> Phase {
    let graph = catalog::sub_graph(&env.graph, aux.subject, st.settings.grade);
    match session.status(&env.cfg) {
        SessionStatus::Active => {}
        // Без обов'язкових лімітів час лише радиться: сесія триває, інтерфейс м'яко пропонує перерву.
        SessionStatus::WrapUp if env.cfg.enforce_limits => return Phase::Ended(EndReason::TimeUp),
        SessionStatus::WrapUp => {}
        SessionStatus::Ended(r) => return Phase::Ended(r),
    }
    for _ in 0..30 {
        let idx = st.learner.tasks_today_total();
        let chosen = match (aux.review, aux.focus) {
            (Some(rv), _) => review_pick(env, st, &graph, rv),
            (None, Some(skill)) => Some(tutor_engine::Selection {
                skill: skill.to_string(),
                reason: Reason::Continue,
            }),
            (None, None) => select_skill(&graph, &st.learner, &env.bkt, &env.cfg, idx),
        };
        let Some(sel) = chosen else {
            return Phase::Ended(EndReason::NothingLeft);
        };
        let Some(gen) = generator_for(&sel.skill) else {
            st.learner.mark_exhausted(&sel.skill);
            continue;
        };
        // Нова тема: спершу пропозиція прочитати пояснення (за бажанням учня, налаштування «lesson_first»).
        let first_time = st
            .learner
            .skill(&sel.skill)
            .map_or(true, |s| s.attempts == 0);
        if st.settings.lesson_first
            && aux.focus.is_none()
            && matches!(sel.reason, Reason::New)
            && first_time
            && !aux.intro_done.contains(&sel.skill)
            && !aux.no_intro
            && env.lessons.lesson(&sel.skill).is_some()
        {
            return Phase::Intro { skill: sel.skill };
        }
        let kind = match sel.reason {
            Reason::New => Kind::New,
            Reason::Continue => Kind::Practice,
            Reason::Review { microtest: false } => Kind::Review,
            Reason::Review { microtest: true } => Kind::Microtest,
        };
        // Пропущені раніше завдання банків повертаються за інтервалами.
        let mut returned = false;
        let task = match due_item(st, aux, gen.as_ref(), &sel.skill) {
            Some(task) => {
                returned = true;
                Some(task)
            }
            None => match pick_task(gen.as_ref(), &mut st.learner, &env.cfg) {
                Pick::Task(task) => Some(*task),
                Pick::Exhausted => None,
            },
        };
        if let Some(task) = task {
            return Phase::Task(Box::new(Current {
                skill: sel.skill,
                kind,
                gen,
                task,
                ladder: Ladder::new(&env.cfg),
                gate: HintGate::new(),
                rejected: Vec::new(),
                wrong: 0,
                bug: None,
                started_at: wall,
                analog: None,
                number: aux.number,
                no_hints: false,
                hard_return: returned,
            }));
        }
    }
    Phase::Ended(EndReason::NothingLeft)
}

/// Приклад, який раніше дався важко (прогалина) і який пора повторити, якщо він є. Для завдань банків це те саме
/// завдання, для обчислювальних тем той самий варіант генератора.
fn due_item(st: &AppState, aux: &Aux<'_>, gen: &dyn SkillGenerator, skill: &str) -> Option<Task> {
    let today = st.learner.today();
    let entry = st.item_srs.iter().find(|i| {
        i.skill == skill
            && i.due_day <= today
            && aux
                .item_gap
                .get(&(i.skill.clone(), i.item))
                .map_or(true, |n| aux.number >= n.saturating_add(ITEM_GAP))
    })?;
    let task = gen.generate(u64::from(entry.item));
    // Завдання має справді відповідати збереженому номеру (банк міг змінитися).
    let same = if gen.answer_kind() == AnswerKind::Number {
        task.variant == u64::from(entry.item)
    } else {
        task.params.first().copied() == Some(i64::from(entry.item))
    };
    same.then_some(task)
}

/// Підсумок задачі: стан учня, сесія, стікери.
fn finalize(
    env: &Env,
    st: &mut AppState,
    run: &mut Run,
    cur: Current,
    solved: bool,
    adult: bool,
    wall: u64,
) {
    let hints = cur.ladder.hints_used();
    let rec = st.learner.record_task(
        env.bkt_for(&cur.skill),
        &env.cfg,
        &cur.skill,
        solved,
        hints,
        cur.bug,
    );
    let recall = cur.gen.answer_kind() != AnswerKind::Number;
    let reveal = recall
        .then(|| cur.gen.reveal(&cur.task))
        .flatten()
        .map(|r| (r.answer_text, r.explanation));
    let latency = u32::try_from(wall.saturating_sub(cur.started_at)).unwrap_or(u32::MAX);
    run.session.record_task(latency, solved, hints);
    let facts = TaskFacts {
        solved,
        wrong_attempts: cur.wrong,
        hints_used: hints,
        asked_adult: adult,
    };
    let mut stickers = st.album.award_for_task(&cur.skill, facts);
    if rec.newly_mastered {
        stickers.extend(st.album.award_mastery(&cur.skill));
    }
    if adult {
        st.learner.record_ask_adult(&cur.skill, &cur.task.statement);
    }
    run.new_stickers.extend(stickers.clone());

    // Пам'ять про приклад: статистика (банки) і «прогалини» для всіх тем. Приклад, що дався важко (помилка, кілька
    // хибних спроб чи підказок, показ дорослому), повертатиметься за інтервалами, доки не вийде чисто.
    let variant = if recall {
        cur.task.params.first().and_then(|p| u32::try_from(*p).ok())
    } else {
        u32::try_from(cur.task.variant).ok()
    };
    let clean = solved && !adult && cur.wrong == 0 && hints <= 1;
    // Старанність тижня: задача, над якою справді думали; завершене повторення; подолана складна задача.
    if latency >= crate::weekly::ENGAGED_SECS {
        crate::weekly::note_task(st, st.learner.today());
    }
    if run.review.is_some() && solved && !adult {
        crate::weekly::note_review(st, st.learner.today());
    }
    let mut gap_event = None;
    if let Some(item) = variant {
        if recall {
            note_stat(st, &cur.skill, item, solved && !adult);
        }
        let kind = if adult {
            2
        } else if !solved {
            0
        } else {
            1
        };
        // Перевірка «я це вже вмію» не створює «прогалин»: це вибір дитини, а не складний приклад.
        if run.testout.is_none() || kind == 1 {
            gap_event = note_gap(st, run, &cur.skill, item, clean, kind);
        }
        if gap_event == Some("closed") {
            crate::weekly::note_gap(st, st.learner.today());
        }
    }
    {
        let today = st.learner.today();
        let e = st.activity.entry(today).or_insert((0, 0));
        e.0 = e.0.saturating_add(1);
        if solved && !adult {
            e.1 = e.1.saturating_add(1);
        }
        // Зберігаємо лише останні 60 днів.
        let cutoff = today.saturating_sub(60);
        st.activity.retain(|d, _| *d >= cutoff);
    }
    run.practiced.insert(cur.skill.clone());
    // Серії помилок: порада перечитати пояснення чи відпочити (завжди необов'язкова).
    if solved {
        run.miss_streak = 0;
        run.skill_miss.remove(&cur.skill);
    } else if run.testout.is_some() {
        // Не рахуємо як серію помилок: перевірка добровільна.
    } else {
        run.miss_streak = run.miss_streak.saturating_add(1);
        let n = run.skill_miss.entry(cur.skill.clone()).or_insert(0);
        *n = n.saturating_add(1);
    }
    // Слабкі передумови: не опановані або з низькою оцінкою знання.
    let weak_prereqs: Vec<(String, String)> = env
        .graph
        .get(&cur.skill)
        .map(|n| {
            n.prerequisites
                .iter()
                .filter(|p| {
                    !st.learner.is_mastered(p)
                        || st
                            .learner
                            .skill(p)
                            .is_some_and(|s| s.p_known < env.cfg.rollback_p)
                })
                .take(3)
                .map(|p| (p.to_string(), skill_title(env, p)))
                .collect()
        })
        .unwrap_or_default();
    let struggling = !solved && run.skill_miss.get(&cur.skill).copied().unwrap_or(0) >= 3;
    let support = if solved || run.testout.is_some() {
        None
    } else if struggling && !weak_prereqs.is_empty() {
        Some("prereq")
    } else if struggling && env.lessons.lesson(&cur.skill).is_some() {
        Some("lesson")
    } else if run.miss_streak >= 4 {
        Some("break")
    } else {
        None
    };
    // Перевірка «я це вже вмію»: потрібні чисті відповіді без підказок і помилок.
    if let Some(t) = run.testout.as_mut() {
        t.asked = t.asked.saturating_add(1);
        if solved && !adult && hints == 0 && cur.wrong == 0 {
            t.clean = t.clean.saturating_add(1);
        } else {
            t.failed = true;
        }
    }
    run.phase = Phase::Feedback(FeedbackInfo {
        weak_prereqs: if support == Some("prereq") {
            weak_prereqs
        } else {
            Vec::new()
        },
        skill: cur.skill.clone(),
        flag_key: (item_key(&cur.task), cur.task.statement.clone()),
        support,
        reveal,
        outcome: if adult {
            "adult"
        } else if solved {
            "solved"
        } else {
            "missed"
        },
        stickers,
        mastered: rec.newly_mastered.then(|| skill_title(env, &cur.skill)),
        suggest_break: run.session.suggest_break(&env.cfg).is_some(),
        gap: gap_event,
    });
}

/// Статистика завдання банку (анонімні лічильники для якості завдань).
fn note_stat(st: &mut AppState, skill: &str, item: u32, correct: bool) {
    let stat = st
        .item_stats
        .entry((skill.to_string(), item))
        .or_insert((0, 0));
    stat.0 = stat.0.saturating_add(1);
    if correct {
        stat.1 = stat.1.saturating_add(1);
    }
}

/// Пам'ять про приклад. Не вийшло чисто: прогалина відкривається (або лишається) і приклад повернеться; показаний
/// дорослому повертається не раніше завтра. Вийшло чисто: етап зростає, а після останнього прогалина закривається.
fn note_gap(
    st: &mut AppState,
    run: &mut Run,
    skill: &str,
    item: u32,
    clean: bool,
    kind: u8,
) -> Option<&'static str> {
    let today = st.learner.today();
    let pos = st
        .item_srs
        .iter()
        .position(|i| i.skill == skill && i.item == item);
    if !clean {
        run.item_gap.insert((skill.to_string(), item), run.number);
    }
    let first_due = if kind == 2 {
        today.saturating_add(1)
    } else {
        today
    };
    match (clean, pos) {
        (false, Some(p)) => {
            let g = &mut st.item_srs[p];
            g.stage = 0;
            g.due_day = first_due;
            g.kind = g.kind.max(kind);
            g.misses = g.misses.saturating_add(1);
            Some("still")
        }
        (false, None) if st.item_srs.len() < MAX_ITEM_SRS => {
            st.item_srs.push(ItemSrs {
                skill: skill.to_string(),
                item,
                due_day: first_due,
                stage: 0,
                kind,
                opened: today,
                misses: 0,
            });
            Some("opened")
        }
        (true, Some(p)) => {
            let next = st.item_srs[p].stage + 1;
            if usize::from(next) >= ITEM_DAYS.len() {
                let g = st.item_srs.remove(p);
                st.gaps_closed_total = st.gaps_closed_total.saturating_add(1);
                st.closed_gaps.insert(0, (g.skill, g.opened, today));
                st.closed_gaps.truncate(storage::state::MAX_CLOSED_GAPS);
                Some("closed")
            } else {
                st.item_srs[p].stage = next;
                st.item_srs[p].due_day = today.saturating_add(ITEM_DAYS[usize::from(next)]);
                Some("better")
            }
        }
        _ => None,
    }
}

fn end_run(env: &Env, open: &mut Open, reason: EndReason, wall: u64) {
    let Open { state, run, .. } = open;
    if let Some(run) = run.as_mut() {
        match reason {
            EndReason::UserEnded => run.session.end_by_user(),
            EndReason::NothingLeft => run.session.end_nothing_left(),
            EndReason::TimeUp => run.session.end_time_up(),
            EndReason::Idle | EndReason::HardCap => {}
        }
        state.usage.finish_session(wall, &run.session, &env.cfg);
        run.phase = Phase::Ended(reason);
    }
    state.session = None;
}

fn denied(d: tutor_core::sessionctl::StartDenied) -> Value {
    json!({ "denied": crate::views::denied_json(d) })
}

/// Початок (або продовження) сесії.
pub(crate) fn session_start(
    env: &Env,
    open: &mut Open,
    args: &Value,
    ctx: &Ctx,
) -> Result<Value, ApiError> {
    let saved = open.state.session_subject.clone();
    let subject = args
        .get("subject")
        .and_then(Value::as_str)
        .or(saved.as_deref())
        .unwrap_or_else(|| catalog::default_subject(open.state.settings.grade));
    let known = catalog::subject_list(open.state.settings.grade)
        .into_iter()
        .any(|(id, _)| id == subject);
    if !known {
        return Err(ApiError::bad_request("невідомий предмет"));
    }
    start_run(env, open, ctx, None, subject.to_string(), None, None)
}

/// Вільна практика обраної учнем навички.
pub(crate) fn practice_start(
    env: &Env,
    open: &mut Open,
    args: &Value,
    ctx: &Ctx,
) -> Result<Value, ApiError> {
    let skill = args
        .get("skill")
        .and_then(Value::as_str)
        .filter(|s| {
            env.graph.get(s).is_some()
                && generator_for(s).is_some()
                && catalog::grade_of(s) == open.state.settings.grade
        })
        .ok_or_else(|| ApiError::bad_request("невідома навичка"))?;
    let subject = catalog::subject_of(skill).to_string();
    start_run(env, open, ctx, Some(skill.to_string()), subject, None, None)
}

/// Перевірка «я це вже вмію» для обраної теми: кілька задач без підказок; усі чисті — тема опановується.
pub(crate) fn testout_start(
    env: &Env,
    open: &mut Open,
    args: &Value,
    ctx: &Ctx,
) -> Result<Value, ApiError> {
    let skill = args
        .get("skill")
        .and_then(Value::as_str)
        .filter(|s| env.graph.get(s).is_some() && catalog::grade_of(s) == open.state.settings.grade)
        .ok_or_else(|| ApiError::bad_request("невідома навичка"))?;
    let gen = generator_for(skill).ok_or_else(|| ApiError::bad_request("невідома навичка"))?;
    let total = if gen.answer_kind() == AnswerKind::Number {
        TESTOUT_TASKS
    } else {
        TESTOUT_TASKS_RECALL
    };
    let subject = catalog::subject_of(skill).to_string();
    let t = Testout {
        skill: skill.to_string(),
        total,
        asked: 0,
        clean: 0,
        failed: false,
    };
    start_run(
        env,
        open,
        ctx,
        Some(skill.to_string()),
        subject,
        Some(t),
        None,
    )
}

/// Перехід від пропозиції пояснення до задач (після читання уроку або одразу).
pub(crate) fn intro_continue(env: &Env, open: &mut Open, ctx: &Ctx) -> Result<Value, ApiError> {
    let Open { state, run, .. } = open;
    let r = run.as_mut().ok_or_else(ApiError::wrong_phase)?;
    let Phase::Intro { skill } = &r.phase else {
        return Err(ApiError::wrong_phase());
    };
    r.intro_done.insert(skill.clone());
    r.session.activity(ctx.wall, &env.cfg);
    let phase = advance(
        env,
        state,
        &r.session,
        ctx.wall,
        &Aux {
            number: r.number,
            focus: r.focus.as_deref(),
            subject: &r.subject,
            intro_done: &r.intro_done,
            item_gap: &r.item_gap,
            no_intro: false,
            review: r.review.as_ref(),
        },
    );
    match phase {
        Phase::Ended(reason) => end_run(env, open, reason, ctx.wall),
        p => r.phase = p,
    }
    Ok(json!({}))
}

/// Під час перевірки «я це вже вмію» підказок немає.
fn mark_no_hints(phase: Phase, on: bool) -> Phase {
    match phase {
        Phase::Task(mut c) if on => {
            c.no_hints = true;
            Phase::Task(c)
        }
        other => other,
    }
}

fn start_run(
    env: &Env,
    open: &mut Open,
    ctx: &Ctx,
    focus: Option<String>,
    subject: String,
    testout: Option<Testout>,
    review: Option<ReviewRun>,
) -> Result<Value, ApiError> {
    if matches!(open.run.as_ref().map(|r| &r.phase), Some(Phase::Ended(_))) {
        open.run = None;
    }
    if open.run.is_some() {
        return Ok(json!({}));
    }
    let Open { state, run, .. } = open;
    state.usage.begin_day(ctx.day);
    state.learner.begin_day(ctx.day);

    // Незавершена сесія після збою: продовжуємо, якщо вона не прострочена.
    let mut session = None;
    let mut resumed = false;
    if let Some(mut saved) = state.session.take() {
        match saved.tick(ctx.wall, &env.cfg) {
            SessionStatus::Ended(_) => state.usage.finish_session(ctx.wall, &saved, &env.cfg),
            _ => {
                session = Some(saved);
                resumed = true;
            }
        }
    }
    let mut session = match session {
        Some(s) => s,
        None => match state.usage.start(
            ctx.wall,
            state.settings.is_quiet(ctx.minute),
            state.settings.adult_extra_sessions,
            &env.cfg,
        ) {
            Ok(s) => s,
            Err(d) => return Ok(denied(d)),
        },
    };
    session.activity(ctx.wall, &env.cfg);
    let intro_done = BTreeSet::new();
    let item_gap = BTreeMap::new();
    let phase = advance(
        env,
        state,
        &session,
        ctx.wall,
        &Aux {
            number: 1,
            focus: focus.as_deref(),
            subject: &subject,
            intro_done: &intro_done,
            item_gap: &item_gap,
            no_intro: resumed,
            review: review.as_ref(),
        },
    );
    let phase = mark_no_hints(phase, testout.is_some());
    let ended = if let Phase::Ended(r) = &phase {
        Some(*r)
    } else {
        None
    };
    *run = Some(Run {
        session,
        phase,
        new_stickers: Vec::new(),
        number: 1,
        focus,
        subject,
        intro_done,
        testout,
        testout_result: None,
        miss_streak: 0,
        skill_miss: BTreeMap::new(),
        item_gap,
        practiced: BTreeSet::new(),
        review,
    });
    if let Some(r) = ended {
        end_run(env, open, r, ctx.wall);
    }
    Ok(json!({}))
}

pub(crate) fn answer(
    env: &Env,
    open: &mut Open,
    args: &Value,
    ctx: &Ctx,
) -> Result<Value, ApiError> {
    let input = args
        .get("input")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let Open { state, run, .. } = open;
    let run = run.as_mut().ok_or_else(ApiError::wrong_phase)?;
    run.session.activity(ctx.wall, &env.cfg);
    let unreadable = input.is_empty() || input.chars().count() > MAX_INPUT_CHARS;
    let phase = std::mem::replace(&mut run.phase, placeholder());
    match phase {
        Phase::Task(mut cur) => {
            if unreadable {
                run.phase = Phase::Task(cur);
                return Ok(json!({"result": "unreadable"}));
            }
            if cur.gen.answer_kind() == AnswerKind::Text {
                // Слово, яке учень вводить сам: одна повторна спроба (описка буває), потім відгук із правильним словом.
                return match cur.gen.check_text(&cur.task, &input) {
                    Some(true) => {
                        cur.gate.note_attempt();
                        finalize(env, state, run, *cur, true, false, ctx.wall);
                        Ok(json!({"result": "correct"}))
                    }
                    Some(false) => {
                        cur.gate.note_attempt();
                        cur.wrong += 1;
                        if cur.wrong >= 2 || run.testout.is_some() {
                            finalize(env, state, run, *cur, false, false, ctx.wall);
                            Ok(json!({"result": "missed"}))
                        } else {
                            let letters = cur
                                .gen
                                .reveal(&cur.task)
                                .map_or(0, |r| r.answer_text.chars().count());
                            run.phase = Phase::Task(cur);
                            Ok(
                                json!({"result": "try_again", "text_retry": true, "letters": letters}),
                            )
                        }
                    }
                    None => {
                        run.phase = Phase::Task(cur);
                        Ok(json!({"result": "unreadable"}))
                    }
                };
            }
            match cur.task.check(&input) {
                Verdict::Unparseable => {
                    run.phase = Phase::Task(cur);
                    Ok(json!({"result": "unreadable"}))
                }
                Verdict::Correct => {
                    cur.gate.note_attempt();
                    finalize(env, state, run, *cur, true, false, ctx.wall);
                    Ok(json!({"result": "correct"}))
                }
                _ if cur.gen.answer_kind() == AnswerKind::Choice || run.testout.is_some() => {
                    // Завдання з вибором: після першої хибної спроби показуємо правильну відповідь і пояснення
                    // (рішення C42), а не вгадуємо далі.
                    cur.gate.note_attempt();
                    cur.wrong += 1;
                    finalize(env, state, run, *cur, false, false, ctx.wall);
                    Ok(json!({"result": "missed"}))
                }
                verdict => {
                    cur.gate.note_attempt();
                    let mut diagnosis = None;
                    if let Verdict::Bug(id) = verdict {
                        cur.bug = Some(id);
                        diagnosis = catalog::bug_message(&cur.skill, id);
                    }
                    cur.wrong += 1;
                    if let Some(v) = parse_answer(&input) {
                        cur.rejected.push(v);
                    }
                    match cur.ladder.record_wrong() {
                        LadderAction::AskAdult => {
                            finalize(env, state, run, *cur, false, true, ctx.wall);
                            Ok(json!({"result": "try_again", "ask_adult": true}))
                        }
                        action => {
                            let offer = action == LadderAction::OfferHint;
                            run.phase = Phase::Task(cur);
                            Ok(
                                json!({"result": "try_again", "offer_hint": offer, "diagnosis": diagnosis}),
                            )
                        }
                    }
                }
            }
        }
        Phase::Analog(mut cur) => {
            if unreadable {
                run.phase = Phase::Analog(cur);
                return Ok(json!({"result": "unreadable"}));
            }
            let verdict = cur
                .analog
                .as_ref()
                .map_or(Verdict::Wrong, |a| a.check_final(&input));
            match verdict {
                Verdict::Unparseable => {
                    run.phase = Phase::Analog(cur);
                    Ok(json!({"result": "unreadable"}))
                }
                Verdict::Correct => {
                    cur.analog = None;
                    run.phase = Phase::Task(cur);
                    Ok(json!({"result": "analog_done"}))
                }
                _ => {
                    run.phase = Phase::Analog(cur);
                    Ok(json!({"result": "analog_try_again"}))
                }
            }
        }
        other => {
            run.phase = other;
            Err(ApiError::wrong_phase())
        }
    }
}

pub(crate) fn hint(env: &Env, open: &mut Open, ctx: &Ctx) -> Result<Value, ApiError> {
    let Open { state, run, .. } = open;
    let run = run.as_mut().ok_or_else(ApiError::wrong_phase)?;
    run.session.activity(ctx.wall, &env.cfg);
    let phase = std::mem::replace(&mut run.phase, placeholder());
    match phase {
        Phase::Task(cur) if cur.no_hints => {
            run.phase = Phase::Task(cur);
            Ok(json!({"hint": {"kind": "none"}, "disabled": true}))
        }
        Phase::Task(mut cur) => match cur
            .gate
            .request(ctx.wall, &gate_cfg(env, cur.gen.answer_kind()))
        {
            GateDecision::Wait { secs } => {
                run.phase = Phase::Task(cur);
                Ok(json!({"wait": {"secs": secs}}))
            }
            GateDecision::NeedAttempt => {
                run.phase = Phase::Task(cur);
                Ok(json!({"need_attempt": true}))
            }
            GateDecision::Blocked { secs } => {
                run.phase = Phase::Task(cur);
                Ok(json!({"blocked": {"secs": secs}}))
            }
            GateDecision::Allow
                if cur.gen.answer_kind() != AnswerKind::Number && cur.ladder.hints_used() >= 3 =>
            {
                // Для завдань із вибором є лише три навідні підказки, далі відгук після спроби.
                run.phase = Phase::Task(cur);
                Ok(json!({"hint": {"kind": "none"}}))
            }
            GateDecision::Allow => {
                let out = next_hint(&mut cur.ladder, cur.gen.as_ref(), &cur.task, &cur.rejected);
                let analog = match &out {
                    tutor_core::session::HintOutput::Analog(ex) => Some((**ex).clone()),
                    _ => None,
                };
                match hint_view(out) {
                    ChildHint::Text(text) => {
                        let level = cur.ladder.hints_used();
                        run.phase = Phase::Task(cur);
                        Ok(json!({"hint": {"kind": "text", "text": text, "level": level}}))
                    }
                    ChildHint::Analog(a) => {
                        if let Some(ex) = analog {
                            note_analog_shown(&mut state.learner, &ex);
                            cur.analog = Some(ex);
                        }
                        run.phase = Phase::Analog(cur);
                        Ok(json!({"hint": {
                            "kind": "analog",
                            "statement": a.statement,
                            "steps": a.shown_steps,
                            "prompt": a.final_prompt,
                        }}))
                    }
                    ChildHint::ReturnToTask | ChildHint::OfferHint => {
                        run.phase = Phase::Task(cur);
                        Ok(json!({"hint": {"kind": "none"}}))
                    }
                    ChildHint::AskAdult => {
                        finalize(env, state, run, *cur, false, true, ctx.wall);
                        Ok(json!({"hint": {"kind": "ask_adult"}}))
                    }
                }
            }
        },
        Phase::Analog(cur) => {
            run.phase = Phase::Analog(cur);
            Ok(json!({"hint": {"kind": "in_analog"}}))
        }
        other => {
            run.phase = other;
            Err(ApiError::wrong_phase())
        }
    }
}

/// Дитина сама передає задачу дорослому («Не зараз»).
pub(crate) fn hand_to_adult(env: &Env, open: &mut Open, ctx: &Ctx) -> Result<Value, ApiError> {
    let Open { state, run, .. } = open;
    let run = run.as_mut().ok_or_else(ApiError::wrong_phase)?;
    run.session.activity(ctx.wall, &env.cfg);
    match std::mem::replace(&mut run.phase, placeholder()) {
        Phase::Task(cur) | Phase::Analog(cur) => {
            finalize(env, state, run, *cur, false, true, ctx.wall);
            Ok(json!({}))
        }
        other => {
            run.phase = other;
            Err(ApiError::wrong_phase())
        }
    }
}

/// З аналога назад до своєї задачі (без дописування).
pub(crate) fn back_to_task(open: &mut Open) -> Result<Value, ApiError> {
    let run = open.run.as_mut().ok_or_else(ApiError::wrong_phase)?;
    match std::mem::replace(&mut run.phase, placeholder()) {
        Phase::Analog(mut cur) => {
            cur.analog = None;
            run.phase = Phase::Task(cur);
            Ok(json!({}))
        }
        other => {
            run.phase = other;
            Err(ApiError::wrong_phase())
        }
    }
}

/// Після відгуку до наступної задачі або до завершення.
pub(crate) fn continue_(env: &Env, open: &mut Open, ctx: &Ctx) -> Result<Value, ApiError> {
    let Open { state, run, .. } = open;
    let r = run.as_mut().ok_or_else(ApiError::wrong_phase)?;
    if !matches!(r.phase, Phase::Feedback(_)) {
        return Err(ApiError::wrong_phase());
    }
    r.session.activity(ctx.wall, &env.cfg);
    // Перевірка «я це вже вмію» закінчується після останньої задачі або першої недосконалої відповіді.
    if let Some(t) = &r.testout {
        if t.failed || t.asked >= t.total {
            let passed = !t.failed && t.clean >= t.total;
            let skill = t.skill.clone();
            if passed
                && state
                    .learner
                    .grant_mastery(env.bkt_for(&skill), &env.cfg, &skill)
            {
                let got = state.album.award_mastery(&skill);
                r.new_stickers.extend(got);
            }
            r.testout_result = Some(passed);
            end_run(env, open, EndReason::UserEnded, ctx.wall);
            return Ok(json!({}));
        }
    }
    r.number = r.number.saturating_add(1);
    let phase = advance(
        env,
        state,
        &r.session,
        ctx.wall,
        &Aux {
            number: r.number,
            focus: r.focus.as_deref(),
            subject: &r.subject,
            intro_done: &r.intro_done,
            item_gap: &r.item_gap,
            no_intro: false,
            review: r.review.as_ref(),
        },
    );
    let phase = mark_no_hints(phase, r.testout.is_some());
    match phase {
        Phase::Ended(reason) => end_run(env, open, reason, ctx.wall),
        p => r.phase = p,
    }
    Ok(json!({}))
}

pub(crate) fn end_session(env: &Env, open: &mut Open, ctx: &Ctx) -> Result<Value, ApiError> {
    match open.run.as_ref().map(|r| &r.phase) {
        None => Err(ApiError::wrong_phase()),
        Some(Phase::Ended(_)) => Ok(json!({})),
        Some(_) => {
            end_run(env, open, EndReason::UserEnded, ctx.wall);
            Ok(json!({}))
        }
    }
}

/// Поточний екран сесії.
pub(crate) fn model(env: &Env, open: &Open) -> Value {
    let Some(run) = &open.run else {
        return json!({"phase": "none"});
    };
    let wrap_up = matches!(run.session.status(&env.cfg), SessionStatus::WrapUp);
    match &run.phase {
        Phase::Task(cur) => {
            let v = task_view(&cur.task);
            json!({
                "phase": "task",
                "kind": cur.kind.code(),
                "skill_id": cur.skill,
                "skill_title": skill_title(env, &cur.skill),
                "number": cur.number,
                "statement": v.statement,
                "choices": cur.gen.choices(&cur.task),
                "answer_kind": match cur.gen.answer_kind() {
                    AnswerKind::Number => "number",
                    AnswerKind::Choice => "choice",
                    AnswerKind::Text => "text",
                },
                "figures": crate::views::figures_json(&cur.gen.figures(&cur.task)),
                "hint_level": cur.ladder.hints_used(),
                "hints_allowed": !cur.no_hints,
                "hard_return": cur.hard_return,
                "testout": run.testout.as_ref().map(|t| json!({"asked": t.asked, "total": t.total})),
                "wrap_up": wrap_up,
            })
        }
        Phase::Intro { skill } => json!({
            "phase": "intro",
            "skill_id": skill,
            "skill_title": skill_title(env, skill),
            "has_lesson": env.lessons.lesson(skill).is_some(),
            "wrap_up": wrap_up,
        }),
        Phase::Analog(cur) => {
            let v = task_view(&cur.task);
            let (statement, steps, prompt) = cur.analog.as_ref().map_or_else(
                || (String::new(), Vec::new(), String::new()),
                |a| {
                    (
                        a.task.statement.clone(),
                        a.shown_steps.clone(),
                        a.final_prompt.clone(),
                    )
                },
            );
            json!({
                "phase": "analog",
                "skill_id": cur.skill,
                "skill_title": skill_title(env, &cur.skill),
                "task_statement": v.statement,
                "statement": statement,
                "steps": steps,
                "prompt": prompt,
                "figures": cur
                    .analog
                    .as_ref()
                    .map(|a| crate::views::figures_json(&cur.gen.figures(&a.task)))
                    .unwrap_or_default(),
                "wrap_up": wrap_up,
            })
        }
        Phase::Feedback(f) => json!({
            "phase": "feedback",
            "outcome": f.outcome,
            "skill_id": f.skill,
            "support": f.support,
            "weak_prerequisites": f.weak_prereqs.iter().map(|(id, t)| json!({"id": id, "title": t})).collect::<Vec<_>>(),
            "reveal": f.reveal.as_ref().map(|(a, e)| json!({"answer": a, "explanation": e})),
            "stickers": f.stickers.iter().map(|s| sticker_json(env, s)).collect::<Vec<_>>(),
            "mastered": f.mastered,
            "suggest_break": f.suggest_break,
            "gap": f.gap,
            "wrap_up": wrap_up,
        }),
        Phase::Ended(reason) => {
            let s = run.session.summary();
            json!({
                "phase": "ended",
                "reason": match reason {
                    EndReason::TimeUp => "time_up",
                    EndReason::HardCap => "hard_cap",
                    EndReason::Idle => "idle",
                    EndReason::UserEnded => "user",
                    EndReason::NothingLeft => "nothing_left",
                },
                "summary": {"tasks": s.tasks, "solved": s.solved, "minutes": s.active_secs / 60},
                "stickers": run.new_stickers.iter().map(|x| sticker_json(env, x)).collect::<Vec<_>>(),
                "practiced": run.practiced.iter().take(8).map(|s| json!({"id": s, "title": skill_title(env, s)})).collect::<Vec<_>>(),
                "testout": run.testout_result.map(|p| json!({"passed": p, "skill_title": run.testout.as_ref().map(|t| skill_title(env, &t.skill))})),
            })
        }
    }
}

/// Для тестів: правильне слово поточного завдання з введенням слова (лише з `test-support`).
#[cfg(feature = "test-support")]
pub(crate) fn peek_text_answer(open: &Open) -> Option<String> {
    match open.run.as_ref().map(|r| &r.phase) {
        Some(Phase::Task(c)) => c.gen.reveal(&c.task).map(|r| r.answer_text),
        _ => None,
    }
}

/// Для тестів: відповідь поточної задачі (лише з `test-support`).
#[cfg(feature = "test-support")]
pub(crate) fn peek_answer(open: &Open) -> Option<String> {
    match open.run.as_ref().map(|r| &r.phase) {
        Some(Phase::Task(c) | Phase::Analog(c)) => Some(c.task.answer.to_string()),
        _ => None,
    }
}

#[cfg(feature = "test-support")]
pub(crate) fn peek_analog_answer(open: &Open) -> Option<String> {
    match open.run.as_ref().map(|r| &r.phase) {
        Some(Phase::Analog(c)) => c.analog.as_ref().map(|a| a.task.answer.to_string()),
        _ => None,
    }
}

#[cfg(feature = "test-support")]
pub(crate) fn peek_protected(open: &Open) -> Vec<Frac> {
    match open.run.as_ref().map(|r| &r.phase) {
        Some(Phase::Task(c)) => c.task.protected_values(),
        Some(Phase::Analog(c)) => {
            let mut v = c.task.protected_values();
            if let Some(a) = &c.analog {
                v.push(a.task.answer);
            }
            v
        }
        _ => Vec::new(),
    }
}

/// Режим «Повторення»: коротка серія задач лише з тем, які пора повторити. `subject` необов'язковий.
pub(crate) fn review_start(
    env: &Env,
    open: &mut Open,
    args: &Value,
    ctx: &Ctx,
) -> Result<Value, ApiError> {
    let subject = args
        .get("subject")
        .and_then(Value::as_str)
        .map(str::to_string);
    if let Some(sid) = &subject {
        if !catalog::subject_list(open.state.settings.grade)
            .iter()
            .any(|(id, _)| id == sid)
        {
            return Err(ApiError::bad_request("невідомий предмет"));
        }
    }
    open.state.learner.begin_day(ctx.day);
    let base: BTreeMap<String, u32> = env
        .graph
        .ids()
        .into_iter()
        .map(|s| (s.to_string(), open.state.learner.tasks_today(s)))
        .collect();
    let rv = ReviewRun {
        cap: REVIEW_CAP,
        base,
    };
    // Сесії ведуться в межах одного предмета; без явного вибору береться перший предмет, де є що повторювати.
    let subject = subject.unwrap_or_else(|| {
        let today_due: Vec<String> = open
            .state
            .learner
            .all_due_reviews(&env.cfg)
            .into_iter()
            .map(|d| d.skill)
            .collect();
        let grade = open.state.settings.grade;
        catalog::subject_list(grade)
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| today_due.iter().any(|s| catalog::subject_of(s) == *id))
            .unwrap_or_else(|| catalog::default_subject(grade))
            .to_string()
    });
    start_run(env, open, ctx, None, subject, None, Some(rv))
}

/// Наступна тема повторення: найменше задач у цьому повторенні, потім найстаріше прострочення; кожна тема не більше
/// `REVIEW_PER_SKILL` задач, щоб теми чергувалися.
fn review_pick(
    env: &Env,
    st: &AppState,
    graph: &course_pack::SkillGraph,
    rv: &ReviewRun,
) -> Option<tutor_engine::Selection> {
    let today = st.learner.today();
    let taken = |s: &str| {
        st.learner
            .tasks_today(s)
            .saturating_sub(rv.base.get(s).copied().unwrap_or(0))
    };
    let total: u32 = graph.ids().iter().map(|s| taken(s)).sum();
    if total >= rv.cap {
        return None;
    }
    // (задач у цьому повторенні, порядок за прострочення, тема, мікротест)
    let mut cands: Vec<(u32, usize, String, Option<bool>)> = Vec::new();
    for (pos, d) in st.learner.all_due_reviews(&env.cfg).into_iter().enumerate() {
        if graph.get(&d.skill).is_some() && taken(&d.skill) < REVIEW_PER_SKILL {
            cands.push((taken(&d.skill), pos, d.skill, Some(d.microtest)));
        }
    }
    for i in &st.item_srs {
        if i.due_day <= today
            && graph.get(&i.skill).is_some()
            && taken(&i.skill) < REVIEW_PER_SKILL
            && !cands.iter().any(|c| c.2 == i.skill)
        {
            cands.push((taken(&i.skill), usize::MAX, i.skill.clone(), None));
        }
    }
    cands.sort_by(|a, b| (a.0, a.1, &a.2).cmp(&(b.0, b.1, &b.2)));
    let (_, _, skill, micro) = cands.into_iter().next()?;
    Some(tutor_engine::Selection {
        skill,
        reason: micro.map_or(Reason::Continue, |m| Reason::Review { microtest: m }),
    })
}

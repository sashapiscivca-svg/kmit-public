//! Диспетчер команд і стан застосунку: профілі, налаштування, режим дорослого, сторінка «додому».

use crate::badges;
use crate::base64;
use crate::ctx::Ctx;
use crate::error::ApiError;
use crate::flow::{self, Phase, Run};
use crate::views::{album_json, denied_json, settings_json, sheet_json, skill_title};
use course_pack::SkillGraph;
use serde_json::{json, Value};
use std::path::Path;
use storage::crypto::KdfParams;
use storage::profiles::{ProfileStore, MAX_PROFILES};
use storage::state::{AppState, Settings, MAX_NOTE_CHARS};
use storage::vault::{KeyWrapHook, Vault};

mod cmds;
use tutor_core::bkt::BktParams;
use tutor_core::config::Config;
use tutor_core::frac::parse_answer;
use tutor_core::lesson::{LessonBlock, LessonSource};
use tutor_core::sheet::sanitize_nickname;

/// Скільки секунд режим дорослого лишається розблокованим.
const ADULT_UNLOCK_SECS: u64 = 300;

pub(crate) struct Env {
    pub lessons: crate::catalog::Lessons,
    pub graph: SkillGraph,
    pub cfg: Config,
    pub bkt: BktParams,
    pub bkt_choice: BktParams,
    /// Тека, де лежать встановлені (підписані) пакети вмісту.
    pub packs_dir: std::path::PathBuf,
}

impl Env {
    /// Параметри оцінки знання: для завдань із вибором вгадування можливе (P(G) = 0,25).
    pub fn bkt_for(&self, skill: &str) -> &BktParams {
        let choice = crate::catalog::generator_for(skill)
            .is_some_and(|g| g.answer_kind() == tutor_core::generator::AnswerKind::Choice);
        if choice {
            &self.bkt_choice
        } else {
            &self.bkt
        }
    }
}

pub(crate) struct Open {
    pub slot: u8,
    pub vault: Vault,
    pub state: AppState,
    pub run: Option<Run>,
    pub adult_until: u64,
    /// Хибні спроби PIN дорослого (лічильник, момент останньої): пауза, як для PIN профілю.
    pub adult_fail: (u32, u64),
    /// Хибні спроби контрольних питань уроку в цій сесії: (тема, номер) → кількість. Після двох спроб питання «спалене»:
    /// відповідь показано, відзнака за нього не нараховується.
    pub lesson_wrong: std::collections::HashMap<(String, u32), u8>,
    /// День, за який зібрано `lesson_wrong`: наступного дня спроби починаються заново.
    pub lesson_wrong_day: u32,
    /// Рівень підказки до питань уроку в цей день: (тема, номер) → 0..=2.
    pub lesson_hints: std::collections::HashMap<(String, u32), u8>,
    /// Теми, для яких дитина вже написала пригадування (без цього самооцінка не приймається).
    pub recall_shown: std::collections::HashSet<String>,
}

/// Фабрика хуків захисту ключа пристроєм: по одному на профіль (слот).
pub type HookFactory = Box<dyn Fn(u8) -> Box<dyn KeyWrapHook> + Send>;

pub struct App {
    store: ProfileStore,
    kdf: KdfParams,
    hook: Option<HookFactory>,
    env: Env,
    open: Option<Open>,
}

/// Команди, після яких стан записується у сховище.
const MUTATING: &[&str] = &[
    "teacher_work_build",
    "teacher_celebrate_seen",
    "rv_note",
    "rv_clear",
    "rv_topic_ok",
    "teacher_bookmark",
    "session_start",
    "practice_start",
    "testout_start",
    "review_start",
    "intro_continue",
    "reflect",
    "flag_item",
    "backup_snooze",
    "note_set",
    "week_claim",
    "ceremony_do",
    "hero_set",
    "recall_queue",
    "recall_rate",
    "answer",
    "hint",
    "hand_to_adult",
    "back_to_task",
    "continue",
    "end_session",
    "settings_set",
    "adult_set_pin",
    "adult_resolve",
];

/// Команди, відповідь яких доповнюється поточним екраном сесії (`model`).
const SESSION_CMDS: &[&str] = &[
    "session_start",
    "practice_start",
    "testout_start",
    "review_start",
    "intro_continue",
    "session_state",
    "answer",
    "hint",
    "hand_to_adult",
    "back_to_task",
    "continue",
    "end_session",
];

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, ApiError> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request(&format!("немає поля {key}")))
}

fn u8_arg(args: &Value, key: &str) -> Result<u8, ApiError> {
    args.get(key)
        .and_then(Value::as_u64)
        .and_then(|v| u8::try_from(v).ok())
        .ok_or_else(|| ApiError::bad_request(&format!("немає числа {key}")))
}

fn persist(open: &mut Open) -> Result<(), ApiError> {
    let live = open
        .run
        .as_ref()
        .filter(|r| !matches!(r.phase, Phase::Ended(_)));
    open.state.session = live.map(|r| r.session.clone());
    open.state.session_subject = live.map(|r| r.subject.clone());
    open.vault.save_state(&open.state)?;
    Ok(())
}

impl App {
    /// `root` — тека даних застосунку. `hook` — захист ключа пристроєм (Android Keystore), якщо він є.
    pub fn new(root: &Path, kdf: KdfParams, hook: Option<HookFactory>) -> App {
        // Встановлені пакети вмісту додаються до вбудованих до першого звернення до вмісту (діє один раз за процес).
        subjects::install_extra(crate::packs::load_installed(&root.join("packs")));
        App {
            store: ProfileStore::new(root),
            kdf,
            hook,
            env: Env {
                lessons: crate::catalog::Lessons,
                graph: crate::catalog::merged_graph(),
                cfg: Config::default(),
                bkt: BktParams::numeric_default(),
                bkt_choice: BktParams::choice_default(),
                packs_dir: root.join("packs"),
            },
            open: None,
        }
    }

    /// Єдина точка входу. Повертає JSON-результат або помилку з кодом.
    pub fn call(&mut self, name: &str, args: &Value, ctx: &Ctx) -> Result<Value, ApiError> {
        match name {
            "profiles" => self.profiles(),
            "hero_options" => Ok(crate::hero_api::options_json()),
            "hero_preview" => Ok(crate::hero_api::preview_json(args)),
            "create_profile" => self.create_profile(args),
            // У збірці методиста немає дитячих профілів і перенесення профілів.
            "import_profile" if cfg!(feature = "reviewer") => Err(ApiError::new(
                "wrong_profile_kind",
                "Ця дія недоступна в застосунку методиста",
            )),
            "open_profile" => self.open_profile(args, ctx),
            "recover_profile" => self.recover_profile(args, ctx),
            "erase_profile" => self.erase_profile(args),
            "recovery_sheet" => Self::recovery_sheet(args),
            "import_profile" => self.import_profile(args),
            "lock" => self.lock(),
            "about" => Ok(json!({
                "name": "Kmit",
                "version": env!("CARGO_PKG_VERSION"),
                "data": "Усі дані лишаються на цьому пристрої й зашифровані.",
                "reviewer": cfg!(feature = "reviewer"),
            })),
            _ => {
                let App {
                    env, open, store, ..
                } = self;
                let open = open.as_mut().ok_or_else(ApiError::no_profile_open)?;
                if !crate::teacher::allowed(open.state.kind, name) {
                    return Err(ApiError::new(
                        "wrong_profile_kind",
                        "Ця дія недоступна в цьому профілі",
                    ));
                }
                env.cfg.enforce_limits = open.state.settings.enforce_limits;
                let out = handle_open(env, open, name, args, ctx)?;
                // Зміна вигляду героя оновлює й публічний код у списку профілів.
                if name == "hero_set" {
                    let _ = store.set_avatar(open.slot, open.state.profile.avatar);
                }
                Ok(out)
            }
        }
    }

    fn make_hook(&self, slot: u8) -> Option<Box<dyn KeyWrapHook>> {
        self.hook.as_ref().map(|f| f(slot))
    }

    fn profiles(&self) -> Result<Value, ApiError> {
        let list = self.store.list()?;
        Ok(json!({
            "profiles": list.iter().map(|p| json!({"slot": p.slot, "avatar": p.avatar, "kind": p.kind.as_str(), "px": crate::hero_api::avatar_px(p.avatar)})).collect::<Vec<_>>(),
            "max": MAX_PROFILES,
            "can_create": list.len() < if cfg!(feature = "reviewer") { 1 } else { MAX_PROFILES },
        }))
    }

    fn close_current(&mut self) -> Result<(), ApiError> {
        if let Some(mut open) = self.open.take() {
            persist(&mut open)?;
        }
        Ok(())
    }

    fn create_profile(&mut self, args: &Value) -> Result<Value, ApiError> {
        let nickname = sanitize_nickname(str_arg(args, "nickname")?);
        if nickname.is_empty() {
            return Err(ApiError::new(
                "invalid_nickname",
                "Напиши ім'я або прізвисько літерами",
            ));
        }
        // Герой дитини: колір шкіри, колір волосся й зачіска (з вільних). Старий виклик із одним кодом теж працює.
        let skin = u8_arg(args, "skin")
            .ok()
            .filter(|s| usize::from(*s) < tutor_core::hero::SKINS.len());
        let hair = u8_arg(args, "hair_color")
            .ok()
            .filter(|h| usize::from(*h) < tutor_core::hero::HAIR_COLORS.len());
        let style = args.get("hair_style").and_then(Value::as_str).filter(|id| {
            tutor_core::hero::item(id)
                .is_some_and(|i| i.free && i.slot == tutor_core::hero::Slot::Hair)
        });
        let avatar = match (skin, hair) {
            (Some(s), Some(h)) => crate::hero_api::avatar_code(s, h),
            _ => u8_arg(args, "avatar").unwrap_or(9).min(47),
        };
        let pin = str_arg(args, "pin")?;
        let kind = args.get("kind").and_then(Value::as_str).map_or(
            storage::state::ProfileKind::Child,
            storage::state::ProfileKind::parse,
        );
        if cfg!(feature = "reviewer")
            && (kind != storage::state::ProfileKind::Teacher || !self.store.list()?.is_empty())
        {
            return Err(ApiError::new(
                "wrong_profile_kind",
                "Ця дія недоступна в застосунку методиста",
            ));
        }
        self.close_current()?;
        let slot = self.store.free_slot()?;
        let hook = self.make_hook(slot);
        let (mut vault, code) =
            self.store
                .create_in_slot_kind(slot, pin, avatar, kind, self.kdf, hook.as_deref())?;
        let mut state = AppState::new(&nickname, avatar);
        state.kind = kind;
        state.hero.skin = avatar / 8;
        state.hero.hair_color = avatar % 8;
        if let Some(id) = style {
            state
                .hero
                .equipped
                .insert(tutor_core::hero::Slot::Hair, id.to_string());
        }
        vault.save_state(&state)?;
        self.open = Some(Open {
            slot,
            vault,
            state,
            run: None,
            adult_until: 0,
            adult_fail: (0, 0),
            lesson_wrong: Default::default(),
            lesson_wrong_day: 0,
            lesson_hints: Default::default(),
            recall_shown: Default::default(),
        });
        Ok(json!({"slot": slot, "recovery_code": code}))
    }

    fn open_profile(&mut self, args: &Value, ctx: &Ctx) -> Result<Value, ApiError> {
        let slot = u8_arg(args, "slot")?;
        let pin = str_arg(args, "pin")?;
        self.close_current()?;
        let hook = self.make_hook(slot);
        let vault = self.store.open_at(slot, pin, hook.as_deref(), ctx.wall)?;
        let state = vault
            .load_state()?
            .ok_or_else(|| ApiError::new("storage", "Профіль порожній"))?;
        let kind_code = state.kind.as_str();
        self.open = Some(Open {
            slot,
            vault,
            state,
            run: None,
            adult_until: 0,
            adult_fail: (0, 0),
            lesson_wrong: Default::default(),
            lesson_wrong_day: 0,
            lesson_hints: Default::default(),
            recall_shown: Default::default(),
        });
        Ok(json!({"ok": true, "kind": kind_code}))
    }

    fn recover_profile(&mut self, args: &Value, ctx: &Ctx) -> Result<Value, ApiError> {
        let slot = u8_arg(args, "slot")?;
        let code = str_arg(args, "code")?;
        let new_pin = str_arg(args, "new_pin")?;
        self.close_current()?;
        let hook = self.make_hook(slot);
        let vault = self
            .store
            .recover_at(slot, code, new_pin, hook.as_deref(), ctx.wall)?;
        let state = vault
            .load_state()?
            .ok_or_else(|| ApiError::new("storage", "Профіль порожній"))?;
        let kind_code = state.kind.as_str();
        self.open = Some(Open {
            slot,
            vault,
            state,
            run: None,
            adult_until: 0,
            adult_fail: (0, 0),
            lesson_wrong: Default::default(),
            lesson_wrong_day: 0,
            lesson_hints: Default::default(),
            recall_shown: Default::default(),
        });
        Ok(json!({"ok": true, "kind": kind_code}))
    }

    /// Аркуш із кодом відновлення для друку чи збереження. Нічого не читає зі сховища: код приходить із екрана.
    fn recovery_sheet(args: &Value) -> Result<Value, ApiError> {
        let code = str_arg(args, "code")?.trim().to_string();
        let valid = !code.is_empty()
            && code.chars().count() <= 40
            && code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        if !valid {
            return Err(ApiError::bad_request("Це не схоже на код відновлення"));
        }
        let nick: String = args
            .get("nickname")
            .and_then(Value::as_str)
            .unwrap_or("")
            .chars()
            .filter(|c| !c.is_control())
            .take(40)
            .collect();
        let pdf = report::styled::recovery_sheet_pdf(&nick, &code)
            .map_err(|_| ApiError::new("pdf", "Не вдалося створити аркуш"))?;
        Ok(json!({"data": base64::encode(&pdf), "filename": "kod-vidnovlennya.pdf"}))
    }

    fn erase_profile(&mut self, args: &Value) -> Result<Value, ApiError> {
        let slot = u8_arg(args, "slot")?;
        if self.open.as_ref().is_some_and(|o| o.slot == slot) {
            self.open = None; // без запису: профіль зникає
        }
        let hook = self.make_hook(slot);
        self.store.erase(slot, hook.as_deref())?;
        Ok(json!({"ok": true}))
    }

    fn import_profile(&mut self, args: &Value) -> Result<Value, ApiError> {
        let data = base64::decode(str_arg(args, "data")?)
            .ok_or_else(|| ApiError::bad_request("файл пошкоджено"))?;
        let passphrase = str_arg(args, "passphrase")?;
        let pin = str_arg(args, "pin")?;
        let state = Vault::import_encrypted(&data, passphrase)?;
        self.close_current()?;
        let slot = self.store.free_slot()?;
        let hook = self.make_hook(slot);
        let (mut vault, code) = self.store.create_in_slot_kind(
            slot,
            pin,
            state.profile.avatar,
            state.kind,
            self.kdf,
            hook.as_deref(),
        )?;
        vault.save_state(&state)?;
        let kind_code = state.kind.as_str();
        self.open = Some(Open {
            slot,
            vault,
            state,
            run: None,
            adult_until: 0,
            adult_fail: (0, 0),
            lesson_wrong: Default::default(),
            lesson_wrong_day: 0,
            lesson_hints: Default::default(),
            recall_shown: Default::default(),
        });
        Ok(json!({"slot": slot, "recovery_code": code, "kind": kind_code}))
    }

    fn lock(&mut self) -> Result<Value, ApiError> {
        self.close_current()?;
        Ok(json!({"ok": true}))
    }

    /// Для тестів: відповідь поточної задачі.
    /// Для тестів: правильне слово поточного завдання з введенням слова.
    #[cfg(feature = "test-support")]
    pub fn peek_text_answer(&self) -> Option<String> {
        self.open.as_ref().and_then(flow::peek_text_answer)
    }

    #[cfg(feature = "test-support")]
    pub fn peek_answer(&self) -> Option<String> {
        self.open.as_ref().and_then(flow::peek_answer)
    }

    #[cfg(feature = "test-support")]
    pub fn peek_analog_answer(&self) -> Option<String> {
        self.open.as_ref().and_then(flow::peek_analog_answer)
    }

    /// Лише для тестів збірки методиста: змінює збережені позначки (імітація оновлення вмісту).
    #[cfg(all(feature = "test-support", feature = "reviewer"))]
    pub fn tamper_review_notes_for_test(
        &mut self,
        f: impl FnOnce(&mut Vec<storage::state::ReviewNote>),
    ) {
        if let Some(o) = self.open.as_mut() {
            f(&mut o.state.teacher.notes);
        }
    }

    /// Лише для тестів збірки методиста: заповнює сховище позначок до `n` штук.
    #[cfg(all(feature = "test-support", feature = "reviewer"))]
    pub fn fill_review_notes_for_test(&mut self, n: usize) {
        if let Some(o) = self.open.as_mut() {
            o.state.teacher.notes = (0..n)
                .map(|i| storage::state::ReviewNote {
                    skill: "FILL".to_string(),
                    field: "bank:*".to_string(),
                    index: i as u32,
                    text: String::new(),
                    status: "ok".to_string(),
                    correction: String::new(),
                    comment: String::new(),
                })
                .collect();
        }
    }

    /// Лише для тестів: позначає опанованими всі теми класу (для перевірки пропозиції свята).
    #[cfg(feature = "test-support")]
    pub fn master_grade_for_test(&mut self, grade: u8, percent: usize) {
        let ids: Vec<&str> = self
            .env
            .graph
            .ids()
            .into_iter()
            .filter(|id| crate::catalog::grade_of(id) == grade)
            .collect();
        let n = (ids.len() * percent).div_ceil(100);
        if let Some(o) = self.open.as_mut() {
            for id in ids.into_iter().take(n) {
                o.state.learner.force_mastered_for_test(id);
            }
        }
    }

    #[cfg(feature = "test-support")]
    pub fn peek_protected_values(&self) -> Vec<tutor_core::frac::Frac> {
        self.open
            .as_ref()
            .map_or_else(Vec::new, flow::peek_protected)
    }
}

fn sync_day(open: &mut Open, ctx: &Ctx) {
    open.state.usage.begin_day(ctx.day);
    open.state.learner.begin_day(ctx.day);
}

/// PIN дорослого (чи свій PIN налаштувань) необов'язковий: поки його не задано, налаштування доступні всім.
/// Задають його лише ті, хто хоче захистити налаштування (наприклад, родина, що ввімкнула ліміти часу).
fn adult_unlocked(open: &Open, ctx: &Ctx) -> bool {
    open.adult_until > ctx.wall || open.vault.has_adult_pin().is_ok_and(|has| !has)
}

fn require_adult(open: &Open, ctx: &Ctx) -> Result<(), ApiError> {
    if adult_unlocked(open, ctx) {
        Ok(())
    } else {
        Err(ApiError::new(
            "adult_required",
            "Для цього потрібен дорослий",
        ))
    }
}

fn handle_open(
    env: &Env,
    open: &mut Open,
    name: &str,
    args: &Value,
    ctx: &Ctx,
) -> Result<Value, ApiError> {
    // Урок пройдено до кінця саме цією дією: зараховується в старанність тижня.
    let lesson_before = matches!(name, "lesson_check" | "lesson_example" | "lesson_explain")
        .then(|| args.get("skill").and_then(Value::as_str))
        .flatten()
        .and_then(|id| {
            env.lessons.lesson(id).map(|l| {
                (
                    id.to_string(),
                    crate::lesson_prog::is_complete(&l, id, &open.state),
                )
            })
        });
    let mut out = match cmds::dispatch(env, open, name, args, ctx)? {
        cmds::Reply::Done(v) => v,
        // Ранній вихід команди обходить спільну післяобробку, як і раніше.
        cmds::Reply::Early(v) => return Ok(v),
    };
    if let Some((id, was)) = lesson_before {
        if let Some(l) = env.lessons.lesson(&id) {
            if !was && crate::lesson_prog::is_complete(&l, &id, &open.state) {
                crate::weekly::note_lesson(&mut open.state, ctx.day);
            }
        }
    }
    let fresh = badges::award_new(env, &mut open.state);
    if MUTATING.contains(&name)
        || !fresh.is_empty()
        || matches!(name, "lesson_check" | "lesson_example" | "lesson_explain")
    {
        persist(open)?;
    }
    if !fresh.is_empty() {
        out["new_badges"] = json!(fresh.iter().map(|b| b.to_json(true)).collect::<Vec<_>>());
    }
    if SESSION_CMDS.contains(&name) {
        out["model"] = flow::model(env, open);
        if !fresh.is_empty() && out["model"]["phase"] == "feedback" {
            out["model"]["badges"] = out["new_badges"].clone();
        }
    }
    Ok(out)
}

pub(crate) fn skill_state(
    env: &Env,
    l: &tutor_core::learner::LearnerState,
    id: &str,
) -> &'static str {
    let prereqs_ok = env
        .graph
        .get(id)
        .is_some_and(|n| n.prerequisites.iter().all(|p| l.is_mastered(p)));
    if l.is_mastered(id) {
        "mastered"
    } else if l.skill(id).is_some_and(|s| s.attempts > 0) {
        "in_progress"
    } else if prereqs_ok {
        "available"
    } else {
        "locked"
    }
}

/// Уроки містять розв'язані приклади, тож поки відкрита задача (чи аналог), вони закриті: числа задачі не можуть
/// опинитися в тексті, який показується учню (інваріант І1).
fn ensure_no_open_task(open: &Open) -> Result<(), ApiError> {
    match open.run.as_ref().map(|r| &r.phase) {
        Some(Phase::Task(_) | Phase::Analog(_)) => Err(ApiError::new(
            "task_active",
            "Спершу закінчи задачу: урок відкриється після неї",
        )),
        _ => Ok(()),
    }
}

fn skill_arg<'a>(env: &Env, args: &'a Value) -> Result<&'a str, ApiError> {
    args.get("skill")
        .and_then(Value::as_str)
        .filter(|s| env.graph.get(s).is_some())
        .ok_or_else(|| ApiError::bad_request("невідома навичка"))
}

/// Конспекти за порядком курсу: (id, назва теми, текст).
fn notes_entries(env: &Env, open: &Open) -> Vec<(String, String, String)> {
    env.graph
        .topological_order()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| {
            open.state.notes.get(id).map(|t| {
                (
                    id.to_string(),
                    skill_title(env, id),
                    crate::lesson_prog::note_readable(t),
                )
            })
        })
        .collect()
}

/// Пропозиція «що зробити сьогодні»: повторення, продовження розпочатої теми чи нова тема. Лише порада.
pub(crate) fn day_suggestion(env: &Env, open: &Open) -> Value {
    let idx = open.state.learner.tasks_today_total();
    let mut best: Option<(u8, Value)> = None;
    for (sid, title) in crate::catalog::subject_list(open.state.settings.grade) {
        let sg = crate::catalog::sub_graph(&env.graph, sid, open.state.settings.grade);
        let Some(sel) =
            tutor_engine::select_skill(&sg, &open.state.learner, &env.bkt, &env.cfg, idx)
        else {
            continue;
        };
        let (rank, reason) = match sel.reason {
            tutor_engine::Reason::Review { .. } => (0, "review"),
            tutor_engine::Reason::Continue => (1, "continue"),
            tutor_engine::Reason::New => (2, "new"),
        };
        if best.as_ref().map_or(true, |(r, _)| rank < *r) {
            best = Some((
                rank,
                json!({"subject": sid, "subject_title": title, "skill": sel.skill,
                       "skill_title": skill_title(env, &sel.skill), "reason": reason}),
            ));
        }
    }
    best.map_or(Value::Null, |(_, v)| v)
}

fn home(env: &Env, open: &mut Open, ctx: &Ctx) -> Value {
    sync_day(open, ctx);
    crate::recall::seed(env, &mut open.state, ctx.day);
    if matches!(open.run.as_ref().map(|r| &r.phase), Some(Phase::Ended(_))) {
        open.run = None;
    }
    let due: Vec<String> = open
        .state
        .learner
        .due_reviews(&env.cfg)
        .into_iter()
        .map(|d| d.skill)
        .collect();
    let skills: Vec<Value> = env
        .graph
        .topological_order()
        .unwrap_or_default()
        .into_iter()
        .filter(|id| crate::catalog::grade_of(id) == open.state.settings.grade)
        .map(|id| {
            let state = skill_state(env, &open.state.learner, id);
            json!({"id": id, "title": skill_title(env, id), "state": state, "subject": crate::catalog::subject_of(id), "review_due": due.iter().any(|d| d == id)})
        })
        .collect();
    let idx = open.state.learner.tasks_today_total();
    let subjects_json: Vec<Value> = crate::catalog::subject_list(open.state.settings.grade)
        .into_iter()
        .map(|(sid, title)| {
            let sg = crate::catalog::sub_graph(&env.graph, sid, open.state.settings.grade);
            let ids = sg.ids();
            let mastered = ids.iter().filter(|i| open.state.learner.is_mastered(i)).count();
            let nothing = tutor_engine::select_skill(&sg, &open.state.learner, &env.bkt, &env.cfg, idx).is_none();
            json!({"id": sid, "title": title, "total": ids.len(), "mastered": mastered, "nothing_left": nothing})
        })
        .collect();
    let suggestion = day_suggestion(env, open);
    // Нагадування про копію профілю: лише після справжнього користування (30+ задач за тиждень чи загалом), м'яко.
    let total_tasks: u32 = open.state.activity.values().map(|(t, _)| *t).sum();
    let today_n = open.state.learner.today();
    let backup_hint = total_tasks >= 30
        && today_n >= open.state.backup_snooze_day
        && (open.state.last_export_day == 0
            || today_n.saturating_sub(open.state.last_export_day) >= 14);
    let wk = crate::weekly::status(&open.state, ctx.day);
    let week_claimable = wk["pending"].as_array().map_or(0, Vec::len)
        + usize::from(wk["current"]["can_claim"] == true);
    let week_now = wk["current"].clone();
    let all_nothing = subjects_json.iter().all(|s| s["nothing_left"] == true);
    // Одне число «тем до повторення» для головної й екрана повторення.
    let due_total = crate::repeat::overview(env, open, ctx)["total_due"]
        .as_u64()
        .unwrap_or(0);
    let can_start = if open.run.is_some() || open.state.session.is_some() {
        json!({"ok": true, "resume": true})
    } else {
        match open.state.usage.can_start(
            ctx.wall,
            open.state.settings.is_quiet(ctx.minute),
            open.state.settings.adult_extra_sessions,
            &env.cfg,
        ) {
            Ok(()) if all_nothing => json!({"ok": false, "reason": "nothing_left"}),
            Ok(()) => json!({"ok": true}),
            Err(d) => denied_json(d),
        }
    };
    json!({
        "nickname": open.state.profile.nickname,
        "avatar": open.state.profile.avatar,
        "stickers": {
            "count": open.state.album.count(),
            "capacity": tutor_core::album::Album::capacity(
                env.graph.ids().into_iter().filter(|id| crate::catalog::grade_of(id) == open.state.settings.grade).count()
            ),
        },
        "skills": skills,
        "subjects": subjects_json,
        "suggestion": suggestion,
        "backup_hint": backup_hint,
        "hero": crate::hero_api::look_json(&open.state),
        "week_claimable": week_claimable,
        "week": week_now,
        "grade": open.state.settings.grade,
        "grade_ready": crate::grades::is_ready(open.state.settings.grade),
        "promote": crate::ceremony::offer(env, &open.state),
        "world": {"id": crate::catalog::world_of(open.state.settings.grade).0, "name": crate::catalog::world_of(open.state.settings.grade).1},
        "review_due_total": due_total,
        "recall_due": crate::recall::due_skills(env, &open.state, ctx.day).len(),
        "can_start": can_start,
        "settings": settings_json(&open.state.settings),
        "recovery_ok": open.vault.recovery_section_ok(),
    })
}

fn settings_get(open: &Open, ctx: &Ctx) -> Value {
    let mut v = settings_json(&open.state.settings);
    v["adult_unlocked"] = json!(adult_unlocked(open, ctx));
    v
}

fn settings_set(open: &mut Open, args: &Value, ctx: &Ctx) -> Result<Value, ApiError> {
    let s: &mut Settings = &mut open.state.settings;
    if let Some(v) = args.get("font_scale_percent").and_then(Value::as_u64) {
        s.font_scale_percent = u16::try_from(v.clamp(80, 200)).unwrap_or(100);
    }
    if let Some(v) = args.get("reduce_motion").and_then(Value::as_bool) {
        s.reduce_motion = v;
    }
    if let Some(v) = args.get("sound").and_then(Value::as_bool) {
        s.sound = v;
    }
    if let Some(v) = args
        .get("grade")
        .and_then(Value::as_u64)
        .and_then(|g| u8::try_from(g).ok())
        .filter(|g| crate::grades::valid(*g))
    {
        s.grade = v;
    }
    if let Some(v) = args.get("weekly_goal_days").and_then(Value::as_u64) {
        s.weekly_goal_days = u8::try_from(v.min(7)).unwrap_or(0);
    }
    for (key, field) in [
        ("lesson_first", &mut s.lesson_first),
        ("ask_confidence", &mut s.ask_confidence),
        ("reading_mode", &mut s.reading_mode),
        ("onboarded", &mut s.onboarded),
        ("allow_screenshots", &mut s.allow_screenshots),
        ("adult_notice_ack", &mut s.adult_notice_ack),
    ] {
        if let Some(v) = args.get(key).and_then(Value::as_bool) {
            *field = v;
        }
    }
    let touches_adult = args.get("quiet_hours").is_some()
        || args.get("adult_extra_sessions").is_some()
        || args.get("enforce_limits").is_some();
    if touches_adult {
        if !adult_unlocked(open, ctx) {
            return Err(ApiError::new(
                "adult_required",
                "Для цього потрібен дорослий",
            ));
        }
        let s = &mut open.state.settings;
        if let Some(q) = args.get("quiet_hours") {
            s.quiet_hours = if q.is_null() {
                None
            } else {
                let from = q
                    .get("from")
                    .and_then(Value::as_u64)
                    .and_then(|x| u16::try_from(x).ok());
                let to = q
                    .get("to")
                    .and_then(Value::as_u64)
                    .and_then(|x| u16::try_from(x).ok());
                match (from, to) {
                    (Some(a), Some(b)) if a < 1440 && b < 1440 => Some((a, b)),
                    _ => return Err(ApiError::bad_request("тихі години вказано хибно")),
                }
            };
        }
        if let Some(v) = args.get("adult_extra_sessions").and_then(Value::as_bool) {
            s.adult_extra_sessions = v;
        }
        if let Some(v) = args.get("enforce_limits").and_then(Value::as_bool) {
            s.enforce_limits = v;
        }
    }
    Ok(settings_json(&open.state.settings))
}

fn small_index(args: &Value) -> Result<u32, ApiError> {
    args.get("index")
        .and_then(Value::as_u64)
        .and_then(|i| u32::try_from(i).ok())
        .filter(|i| *i < 32)
        .ok_or_else(|| ApiError::bad_request("немає номера"))
}

/// Урок для екрана: відповіді й ключові ідеї не надсилаються, поки учень не спробує сам.
fn lesson_json(id: &str, l: &tutor_core::lesson::Lesson, st: &AppState) -> Value {
    let (mut chk, mut ex, mut xp) = (0u32, 0u32, 0u32);
    let blocks: Vec<Value> = l
        .blocks
        .iter()
        .map(|b| match b {
            LessonBlock::Text(t) => json!({"type": "text", "text": t}),
            LessonBlock::Hook(t) => json!({"type": "hook", "prompt": t}),
            LessonBlock::Term { term, definition } => {
                json!({"type": "term", "term": term, "definition": definition})
            }
            LessonBlock::Example {
                statement,
                steps,
                result,
            } => {
                ex += 1;
                json!({"type": "example", "index": ex - 1, "statement": statement, "steps": steps, "result": result})
            }
            LessonBlock::SelfCheck { prompt, .. } => {
                chk += 1;
                json!({"type": "selfcheck", "index": chk - 1, "prompt": prompt})
            }
            LessonBlock::Quiz { prompt, options, .. } => {
                chk += 1;
                json!({"type": "quiz", "index": chk - 1, "prompt": prompt, "options": options})
            }
            LessonBlock::Explain { prompt, .. } => {
                xp += 1;
                json!({"type": "explain", "index": xp - 1, "prompt": prompt})
            }
            LessonBlock::Figure { figure, caption } => json!({
                "type": "figure", "figure": crate::views::figure_json(figure), "caption": caption,
            }),
            LessonBlock::Remember(t) => json!({"type": "remember", "text": t}),
        })
        .collect();
    json!({"id": id, "title": l.title, "blocks": blocks, "note_prompts": l.note_prompts,
           "progress": crate::lesson_prog::progress_json(l, id, st)})
}

/// Для методиста: відповіді, пояснення й ключові ідеї в порядку блоків (лише в режимі методиста).
fn reviewer_answers(l: &tutor_core::lesson::Lesson) -> Value {
    let (mut chk, mut xp) = (0u32, 0u32);
    let mut checks = Vec::new();
    let mut explains = Vec::new();
    for b in &l.blocks {
        match b {
            LessonBlock::SelfCheck {
                answer,
                explanation,
                ..
            } => {
                checks.push(json!({"index": chk, "answer": answer, "explanation": explanation}));
                chk += 1;
            }
            LessonBlock::Quiz {
                options,
                correct,
                explanation,
                ..
            } => {
                checks.push(
                    json!({"index": chk, "answer": options[*correct], "explanation": explanation}),
                );
                chk += 1;
            }
            LessonBlock::Explain { key_ideas, .. } => {
                explains.push(json!({"index": xp, "key_ideas": key_ideas}));
                xp += 1;
            }
            _ => {}
        }
    }
    json!({"checks": checks, "explains": explains})
}

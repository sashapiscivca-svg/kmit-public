//! Стан застосунку, який зберігається: профіль, налаштування, стан учня, альбом, лічильники сесій.
//! Подається як набір рядків «ключ-значення» для таблиці в зашифрованій базі.

use crate::error::StorageError;
use tutor_core::album::Album;
use tutor_core::learner::LearnerState;
use tutor_core::sessionctl::{DayUsage, Session};
use tutor_core::sheet::sanitize_nickname;

/// Найбільший номер аватара (аватари підбирає застосунок; профіль зберігає лише номер).
/// Публічний код вигляду героя в списку профілів: колір шкіри × 8 + колір волосся (до 6 × 8 = 48 варіантів).
pub const MAX_AVATAR: u8 = 63;
const MIN_FONT: u16 = 80;
const MAX_FONT: u16 = 200;
const MINUTES_PER_DAY: u16 = 24 * 60;

/// Профіль: лише нікнейм, який обрала дитина, і номер аватара. Прізвища, школи, дати народження немає.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    pub nickname: String,
    pub avatar: u8,
}

/// Налаштування доступності та родини. Без категорій потреб і діагнозів (рішення R2 #70).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Масштаб тексту, % (80-200).
    pub font_scale_percent: u16,
    pub reduce_motion: bool,
    /// Звук (за замовчуванням вимкнено).
    pub sound: bool,
    /// Тихі години як хвилини від півночі (початок, кінець); може переходити через північ (Q4).
    pub quiet_hours: Option<(u16, u16)>,
    /// Дорослий дозволив додаткову сесію на день (до `max_sessions_per_day_adult_cap`).
    pub adult_extra_sessions: bool,
    /// Обов'язкові ліміти часу (за замовчуванням вимкнено: час лише радиться; вмикає сам учень чи родина).
    pub enforce_limits: bool,
    /// Нова тема починається з пропозиції прочитати пояснення (за замовчуванням так).
    pub lesson_first: bool,
    /// Перед відповіддю запитувати, наскільки учень довіряє своїй відповіді (за бажанням).
    pub ask_confidence: bool,
    /// Читабельний режим: більший міжрядковий інтервал і відстань між літерами.
    pub reading_mode: bool,
    /// Учень пройшов вступ при першому запуску.
    pub onboarded: bool,
    /// Дозволити знімки екрана (за замовчуванням ні).
    pub allow_screenshots: bool,
    /// Ціль на тиждень, яку ставить сама дитина: скільки днів позайматися (0 = без цілі, 1..=7).
    pub weekly_goal_days: u8,
    /// Клас (поки лише 5).
    pub grade: u8,
    /// Режим методиста: в уроках видно правильні відповіді й ключові ідеї (для перевірки вмісту, не для дитини).
    /// Дорослий ознайомився зі сторінкою «Для дорослого» (що зберігається й як видалити).
    pub adult_notice_ack: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            font_scale_percent: 100,
            reduce_motion: false,
            sound: false,
            quiet_hours: None,
            adult_extra_sessions: false,
            enforce_limits: false,
            lesson_first: true,
            ask_confidence: false,
            reading_mode: false,
            onboarded: false,
            allow_screenshots: false,
            weekly_goal_days: 0,
            grade: 5,
            adult_notice_ack: false,
        }
    }
}

impl Settings {
    /// Чи хвилина доби (0..1439) потрапляє в тихі години.
    pub fn is_quiet(&self, minute_of_day: u16) -> bool {
        let Some((from, to)) = self.quiet_hours else {
            return false;
        };
        if from <= to {
            minute_of_day >= from && minute_of_day < to
        } else {
            minute_of_day >= from || minute_of_day < to
        }
    }

    fn encode(&self) -> String {
        let quiet = self
            .quiet_hours
            .map_or("-".to_string(), |(a, b)| format!("{a}-{b}"));
        format!(
            "font_scale_percent={}\nreduce_motion={}\nsound={}\nquiet={}\nadult_extra={}\nlimits={}\nlesson_first={}\nconfidence={}\nreading={}\nonboarded={}\nscreenshots={}\ngoal={}\ngrade={}\nadult_ack={}\n",
            self.font_scale_percent,
            u8::from(self.reduce_motion),
            u8::from(self.sound),
            quiet,
            u8::from(self.adult_extra_sessions),
            u8::from(self.enforce_limits),
            u8::from(self.lesson_first),
            u8::from(self.ask_confidence),
            u8::from(self.reading_mode),
            u8::from(self.onboarded),
            u8::from(self.allow_screenshots),
            self.weekly_goal_days,
            self.grade,
            u8::from(self.adult_notice_ack)
        )
    }

    fn decode(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k {
                "font_scale_percent" => {
                    if let Ok(n) = v.parse::<u32>() {
                        s.font_scale_percent =
                            u16::try_from(n.clamp(u32::from(MIN_FONT), u32::from(MAX_FONT)))
                                .unwrap_or(100);
                    }
                }
                "reduce_motion" => s.reduce_motion = v == "1",
                "sound" => s.sound = v == "1",
                "adult_extra" => s.adult_extra_sessions = v == "1",
                "limits" => s.enforce_limits = v == "1",
                "lesson_first" => s.lesson_first = v != "0",
                "confidence" => s.ask_confidence = v == "1",
                "reading" => s.reading_mode = v == "1",
                "onboarded" => s.onboarded = v == "1",
                "screenshots" => s.allow_screenshots = v == "1",
                "adult_ack" => s.adult_notice_ack = v == "1",
                "goal" => s.weekly_goal_days = v.parse().ok().filter(|g| *g <= 7).unwrap_or(0),
                "grade" => s.grade = v.parse().ok().filter(|g| (5..=12).contains(g)).unwrap_or(5),
                "quiet" => {
                    s.quiet_hours = v.split_once('-').and_then(|(a, b)| {
                        let (a, b) = (a.parse::<u16>().ok()?, b.parse::<u16>().ok()?);
                        (a < MINUTES_PER_DAY && b < MINUTES_PER_DAY).then_some((a, b))
                    });
                }
                _ => {}
            }
        }
        s
    }
}

/// Найдовший власний конспект однієї теми (символів).
pub const MAX_NOTE_CHARS: usize = 4000;

/// «Прогалина» у пам'яті: конкретний приклад, що дався важко (помилка, кілька хибних спроб, показ дорослому).
/// Знеособлений запис: лише тема, номер завдання чи варіант генератора й лічильники, без тексту дитини.
/// Приклад повертається за інтервалами 0/1/3/7/14 днів, доки не вийде чисто; тоді прогалина закривається.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemSrs {
    pub skill: String,
    /// Номер завдання банку або варіант генератора (щоб відтворити саме цей приклад).
    pub item: u32,
    pub due_day: u32,
    pub stage: u8,
    /// Чому виникла прогалина: 0 помилка, 1 багато спроб чи підказок, 2 показано дорослому.
    pub kind: u8,
    /// День, коли прогалину відкрито.
    pub opened: u32,
    /// Скільки разів приклад ще не вийшов чисто (після відкриття).
    pub misses: u8,
}

/// Старанність за один календарний тиждень (лише лічильники за днями; тиждень починається з понеділка).
/// `tasks` це задачі, над якими дитина справді думала; `lessons` пройдені уроки; `reviews` завершені повторення.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WeekRec {
    pub tasks: [u8; 7],
    pub lessons: [u8; 7],
    pub reviews: [u8; 7],
    /// Подолані складні приклади за тиждень.
    pub gaps: u8,
    /// Предмет, який дитина обрала нагородою за цей тиждень.
    pub claimed: Option<String>,
}

/// Скільки тижнів зберігаємо.
pub const MAX_WEEKS: usize = 20;

/// Закрита прогалина: (тема, день відкриття, день закриття).
pub type ClosedGap = (String, u32, u32);

/// Скільки закритих прогалин пам'ятаємо поіменно (решта лише в лічильнику).
pub const MAX_CLOSED_GAPS: usize = 60;
/// Найвищий етап повторення прогалини (інтервали 0, 1, 3, 7, 14).
pub const MAX_GAP_STAGE: u8 = 4;

/// Скільки завдань-повторень зберігаємо щонайбільше.
pub const MAX_ITEM_SRS: usize = 300;

/// Поточна версія формату профілю. Збільшується лише разом із кроком міграції в `MIGRATIONS`.
pub const CURRENT_SCHEMA: u32 = 1;

type Rows = Vec<(String, String)>;

/// Кроки міграції: `MIGRATIONS[n]` переводить формат версії `n` у версію `n + 1`. Профілі, збережені до появи версій,
/// мають версію 0; їхній формат сумісний із 1 (нові рядки необов'язкові), тож крок 0 нічого не змінює.
const MIGRATIONS: &[fn(&mut Rows)] = &[|_| {}];

/// Оновлює рядки до поточної версії. Новіший формат відхиляється.
pub fn migrate_rows(rows: &mut Rows) -> Result<(), StorageError> {
    migrate_rows_with(rows, MIGRATIONS)
}

/// Те саме з довільним списком кроків (для перевірки порядку виконання). Поточна версія = кількість кроків.
pub fn migrate_rows_with(rows: &mut Rows, steps: &[fn(&mut Rows)]) -> Result<(), StorageError> {
    let current = u32::try_from(steps.len()).unwrap_or(u32::MAX);
    let stored: u32 = rows
        .iter()
        .find(|(k, _)| k == "schema")
        .and_then(|(_, v)| v.trim().parse().ok())
        .unwrap_or(0);
    if stored > current {
        return Err(StorageError::NewerFormat);
    }
    for step in &steps[stored as usize..] {
        step(rows);
    }
    rows.retain(|(k, _)| k != "schema");
    rows.push(("schema".to_string(), current.to_string()));
    Ok(())
}

/// Повідомлення дитини про проблему із завданням (для авторів вмісту).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemFlag {
    pub skill: String,
    /// Номер завдання банку чи варіант генератора.
    pub key: String,
    /// `wrong_answer`, `unclear` або `other`.
    pub reason: String,
    pub statement: String,
}

pub const MAX_FLAGS: usize = 200;
pub const FLAG_REASONS: [&str; 3] = ["wrong_answer", "unclear", "other"];

pub struct AppState {
    /// Тип профілю (за замовчуванням дитячий).
    pub kind: ProfileKind,
    /// Дані вчителя (порожні в дитячому профілі).
    pub teacher: TeacherData,
    pub profile: Profile,
    pub learner: LearnerState,
    pub album: Album,
    pub usage: DayUsage,
    pub settings: Settings,
    /// Незавершена сесія (щоб відключення світла не з'їдало слот і сесію можна було продовжити).
    pub session: Option<Session>,
    /// Власні конспекти учня за темами (вільний текст самого учня; лише в зашифрованому профілі).
    pub notes: std::collections::BTreeMap<String, String>,
    /// Предмет незавершеної сесії (щоб після перезапуску вона продовжилась у тому ж предметі).
    pub session_subject: Option<String>,
    /// Які самоперевірки уроків учень розв'язав правильно: навичка → бітова маска за номером самоперевірки.
    pub lessons: std::collections::BTreeMap<String, u32>,
    /// Завдання банків, що чекають повторення.
    pub flags: Vec<ItemFlag>,
    /// Скільки задач і правильних за день (лише лічильники): для екрана «Мій тиждень». Останні 60 днів.
    pub activity: std::collections::BTreeMap<u32, (u32, u32)>,
    /// День останньої копії профілю (0 = не було) і день, до якого нагадування відкладено.
    pub last_export_day: u32,
    /// Класи, перехід із яких відсвятковано (свято переходу, один раз на клас).
    pub ceremonies: std::collections::BTreeSet<u8>,
    pub backup_snooze_day: u32,
    pub item_srs: Vec<ItemSrs>,
    /// Закриті прогалини (останні) і загальна кількість подоланих складних прикладів.
    pub closed_gaps: Vec<ClosedGap>,
    /// Піксельний герой: вигляд і предмети, що вже здобуті (вільні є завжди).
    pub hero: tutor_core::hero::Look,
    pub wardrobe: std::collections::BTreeSet<String>,
    /// Старанність за тижнями (номер тижня від понеділка епохи).
    pub weeks: std::collections::BTreeMap<u32, WeekRec>,
    pub gaps_closed_total: u32,
    /// Спроби й правильні відповіді за (навичка, номер завдання): для анонімної статистики якості завдань.
    pub item_stats: std::collections::BTreeMap<(String, u32), (u32, u32)>,
    /// Здобуті відзнаки (ідентифікатори); ніколи не відбираються.
    pub badges: std::collections::BTreeSet<String>,
}

fn esc(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\n', "%0A")
        .replace('\r', "%0D")
        .replace('|', "%7C")
}

fn unesc(s: &str) -> String {
    // Порядок важливий: %25 останнім, щоб «%257C» лишилося буквальним «%7C».
    s.replace("%0A", "\n")
        .replace("%0D", "\r")
        .replace("%7C", "|")
        .replace("%25", "%")
}

/// Тип профілю: учень або вчитель. Вчительський профіль не містить учнівських даних і навпаки.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProfileKind {
    #[default]
    Child,
    Teacher,
}

impl ProfileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ProfileKind::Child => "child",
            ProfileKind::Teacher => "teacher",
        }
    }

    /// Невідоме значення читається як дитячий профіль (безпечніше й сумісно зі старими).
    pub fn parse(s: &str) -> Self {
        if s.trim() == "teacher" {
            ProfileKind::Teacher
        } else {
            ProfileKind::Child
        }
    }
}

/// Запис про створену роботу (без жодних даних учнів): для відтворення варіантів за зерном.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkRecord {
    pub day: u32,
    pub grade: u8,
    pub kind: String,
    pub skills: Vec<String>,
    pub variants: u8,
    pub seed: u64,
}

/// Зауваження методиста до тексту пакета (локально, зашифровано).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewNote {
    pub skill: String,
    pub field: String,
    pub index: u32,
    pub text: String,
    pub status: String,
    pub correction: String,
    pub comment: String,
}

/// Дані кабінету вчителя.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TeacherData {
    pub works: Vec<WorkRecord>,
    pub notes: Vec<ReviewNote>,
    /// Закладки: теми, до яких вчитель любить повертатися.
    pub bookmarks: Vec<String>,
    /// Скільки робіт створено за весь час (список робіт обрізається, лічильник ні).
    pub works_made: u32,
    /// Свята кабінету, які вчитель уже бачив (`works:10`, `td:2026`, …): кожне показується один раз.
    pub celebrated: std::collections::BTreeSet<String>,
}

pub const MAX_BOOKMARKS: usize = 100;

pub const MAX_TEACHER_WORKS: usize = 200;
pub const MAX_REVIEW_NOTES: usize = 30_000;

impl AppState {
    pub fn new(nickname: &str, avatar: u8) -> Self {
        AppState {
            kind: ProfileKind::Child,
            teacher: TeacherData::default(),
            profile: Profile {
                nickname: sanitize_nickname(nickname),
                avatar: avatar.min(MAX_AVATAR),
            },
            learner: LearnerState::new(),
            album: Album::new(),
            usage: DayUsage::new(),
            settings: Settings::default(),
            session: None,
            notes: std::collections::BTreeMap::new(),
            session_subject: None,
            lessons: std::collections::BTreeMap::new(),
            flags: Vec::new(),
            activity: std::collections::BTreeMap::new(),
            last_export_day: 0,
            ceremonies: Default::default(),
            backup_snooze_day: 0,
            item_srs: Vec::new(),
            closed_gaps: Vec::new(),
            hero: tutor_core::hero::Look::default(),
            wardrobe: std::collections::BTreeSet::new(),
            weeks: std::collections::BTreeMap::new(),
            gaps_closed_total: 0,
            item_stats: std::collections::BTreeMap::new(),
            badges: std::collections::BTreeSet::new(),
        }
    }

    /// Рядки для збереження.
    pub fn to_rows(&self) -> Vec<(String, String)> {
        let (day, started, last_end) = self.usage.parts();
        let last = last_end.map_or("-".to_string(), |v| v.to_string());
        vec![
            ("schema".to_string(), CURRENT_SCHEMA.to_string()),
            ("kind".to_string(), self.kind.as_str().to_string()),
            (
                "teacher_works".to_string(),
                self.teacher
                    .works
                    .iter()
                    .map(|w| {
                        format!(
                            "{}|{}|{}|{}|{}|{}",
                            w.day,
                            w.grade,
                            esc(&w.kind),
                            w.skills
                                .iter()
                                .map(|s| esc(s))
                                .collect::<Vec<_>>()
                                .join(","),
                            w.variants,
                            w.seed
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                "teacher_bookmarks".to_string(),
                self.teacher.bookmarks.join("\n"),
            ),
            (
                "teacher_works_made".to_string(),
                self.teacher.works_made.to_string(),
            ),
            (
                "teacher_celebrated".to_string(),
                self.teacher
                    .celebrated
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                "teacher_notes".to_string(),
                self.teacher
                    .notes
                    .iter()
                    .map(|n| {
                        format!(
                            "{}|{}|{}|{}|{}|{}|{}",
                            esc(&n.skill),
                            esc(&n.field),
                            n.index,
                            esc(&n.text),
                            esc(&n.status),
                            esc(&n.correction),
                            esc(&n.comment)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                "profile".to_string(),
                format!(
                    "avatar={}\nnickname={}\n",
                    self.profile.avatar,
                    esc(&self.profile.nickname)
                ),
            ),
            ("settings".to_string(), self.settings.encode()),
            ("learner".to_string(), self.learner.encode()),
            ("album".to_string(), self.album.keys().join("\n")),
            ("usage".to_string(), format!("{day} {started} {last}")),
            (
                "session".to_string(),
                self.session
                    .as_ref()
                    .map_or_else(|| "-".to_string(), Session::encode),
            ),
            (
                "notes".to_string(),
                self.notes
                    .iter()
                    .map(|(k, v)| format!("{k}={}", esc(v)))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                "session_subject".to_string(),
                self.session_subject.clone().unwrap_or_default(),
            ),
            (
                "lessons".to_string(),
                self.lessons
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                "activity".to_string(),
                self.activity
                    .iter()
                    .map(|(d, (t, s))| format!("{d} {t} {s}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                "ceremonies".to_string(),
                self.ceremonies
                    .iter()
                    .map(u8::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            (
                "backup".to_string(),
                format!("{} {}", self.last_export_day, self.backup_snooze_day),
            ),
            (
                "flags".to_string(),
                self.flags
                    .iter()
                    .map(|f| format!("{}|{}|{}|{}", f.skill, f.key, f.reason, esc(&f.statement)))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                "item_srs".to_string(),
                self.item_srs
                    .iter()
                    .map(|i| {
                        format!(
                            "{}|{}|{}|{}|{}|{}|{}",
                            i.skill, i.item, i.due_day, i.stage, i.kind, i.opened, i.misses
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            ("hero".to_string(), encode_hero(&self.hero, &self.wardrobe)),
            (
                "weeks".to_string(),
                self.weeks
                    .iter()
                    .map(|(w, r)| {
                        let j =
                            |a: &[u8; 7]| a.iter().map(u8::to_string).collect::<Vec<_>>().join(",");
                        format!(
                            "{w}|{}|{}|{}|{}|{}",
                            j(&r.tasks),
                            j(&r.lessons),
                            j(&r.reviews),
                            r.gaps,
                            r.claimed.clone().unwrap_or_default()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                "closed_gaps".to_string(),
                format!(
                    "{}\n{}",
                    self.gaps_closed_total,
                    self.closed_gaps
                        .iter()
                        .map(|(s, a, b)| format!("{s}|{a}|{b}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
            ),
            (
                "item_stats".to_string(),
                self.item_stats
                    .iter()
                    .map(|((sk, it), (a, c))| format!("{sk}|{it}|{a}|{c}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                "badges".to_string(),
                self.badges.iter().cloned().collect::<Vec<_>>().join("\n"),
            ),
        ]
    }

    /// Відновлення з рядків. Бракує обов'язкового чи зіпсовано, то помилка.
    pub fn from_rows(rows: &[(String, String)]) -> Result<AppState, StorageError> {
        let mut migrated = rows.to_vec();
        migrate_rows(&mut migrated)?;
        let rows = &migrated[..];
        let get = |k: &str| {
            rows.iter()
                .find(|(key, _)| key == k)
                .map(|(_, v)| v.as_str())
        };

        let profile_text = get("profile").ok_or(StorageError::Decode)?;
        let mut nickname = None;
        let mut avatar = None;
        for line in profile_text.lines() {
            match line.split_once('=') {
                Some(("nickname", v)) => nickname = Some(sanitize_nickname(&unesc(v))),
                Some(("avatar", v)) => {
                    avatar = v.parse::<u8>().ok().map(|a| a.min(MAX_AVATAR)).or(Some(0))
                }
                _ => {}
            }
        }
        let profile = Profile {
            nickname: nickname.ok_or(StorageError::Decode)?,
            avatar: avatar.unwrap_or(0),
        };

        let learner = LearnerState::decode(get("learner").ok_or(StorageError::Decode)?)
            .map_err(|_| StorageError::Decode)?;
        let album = Album::from_keys(get("album").ok_or(StorageError::Decode)?.lines());

        let usage_text = get("usage").ok_or(StorageError::Decode)?;
        let parts: Vec<&str> = usage_text.split(' ').collect();
        if parts.len() != 3 {
            return Err(StorageError::Decode);
        }
        let usage = DayUsage::from_parts(
            parts[0].parse().map_err(|_| StorageError::Decode)?,
            parts[1].parse().map_err(|_| StorageError::Decode)?,
            if parts[2] == "-" {
                None
            } else {
                Some(parts[2].parse().map_err(|_| StorageError::Decode)?)
            },
        );

        let settings = get("settings").map_or_else(Settings::default, Settings::decode);
        let session = match get("session") {
            None | Some("-") => None,
            Some(text) => Some(Session::decode(text).map_err(|_| StorageError::Decode)?),
        };
        // Зіпсований рядок конспекту не повинен позбавляти прогресу: непридатні записи просто пропускаються.
        let notes = get("notes")
            .map(|t| {
                t.lines()
                    .filter_map(|l| l.split_once('='))
                    .filter(|(k, _)| !k.is_empty())
                    .map(|(k, v)| {
                        (
                            k.to_string(),
                            unesc(v).chars().take(MAX_NOTE_CHARS).collect(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let lessons = get("lessons")
            .map(|t| {
                t.lines()
                    .filter_map(|l| l.split_once('='))
                    .filter(|(k, _)| !k.is_empty())
                    .filter_map(|(k, v)| Some((k.to_string(), v.parse::<u32>().ok()?)))
                    .collect()
            })
            .unwrap_or_default();
        let activity: std::collections::BTreeMap<u32, (u32, u32)> = get("activity")
            .map(|t| {
                t.lines()
                    .filter_map(|l| {
                        let p: Vec<u32> = l.split(' ').filter_map(|x| x.parse().ok()).collect();
                        (p.len() == 3 && p[2] <= p[1]).then(|| (p[0], (p[1], p[2])))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let ceremonies: std::collections::BTreeSet<u8> = get("ceremonies")
            .map(|t| {
                t.split(',')
                    .filter_map(|x| x.parse::<u8>().ok())
                    .filter(|g| (5..=12).contains(g))
                    .collect()
            })
            .unwrap_or_default();
        let (last_export_day, backup_snooze_day) = get("backup")
            .and_then(|t| {
                let mut it = t.split(' ').filter_map(|x| x.parse::<u32>().ok());
                Some((it.next()?, it.next()?))
            })
            .unwrap_or((0, 0));
        let flags: Vec<ItemFlag> = get("flags")
            .map(|t| {
                t.lines()
                    .filter_map(|l| {
                        let p: Vec<&str> = l.splitn(4, '|').collect();
                        if p.len() != 4 || p[0].is_empty() || !FLAG_REASONS.contains(&p[2]) {
                            return None;
                        }
                        Some(ItemFlag {
                            skill: p[0].to_string(),
                            key: p[1].to_string(),
                            reason: p[2].to_string(),
                            statement: unesc(p[3]).chars().take(200).collect(),
                        })
                    })
                    .take(MAX_FLAGS)
                    .collect()
            })
            .unwrap_or_default();
        let item_srs: Vec<ItemSrs> = get("item_srs")
            .map(|t| {
                t.lines()
                    .filter_map(|l| {
                        let p: Vec<&str> = l.split('|').collect();
                        // Старий формат мав 4 поля; нові поля (причина, день відкриття, невдачі) необов'язкові.
                        if !(p.len() == 4 || p.len() == 7) || p[0].is_empty() {
                            return None;
                        }
                        let extra = |i: usize| p.get(i).and_then(|v| v.parse::<u32>().ok());
                        Some(ItemSrs {
                            skill: p[0].to_string(),
                            item: p[1].parse().ok()?,
                            due_day: p[2].parse().ok()?,
                            stage: p[3].parse().ok().filter(|s| *s <= MAX_GAP_STAGE)?,
                            kind: u8::try_from(extra(4).unwrap_or(0))
                                .ok()
                                .filter(|k| *k <= 2)?,
                            opened: extra(5).unwrap_or(0),
                            misses: u8::try_from(extra(6).unwrap_or(0).min(250)).unwrap_or(0),
                        })
                    })
                    .take(MAX_ITEM_SRS)
                    .collect()
            })
            .unwrap_or_default();
        let (hero, wardrobe) = get("hero").map_or_else(
            || (tutor_core::hero::Look::default(), Default::default()),
            decode_hero,
        );
        let weeks: std::collections::BTreeMap<u32, WeekRec> = get("weeks")
            .map(|t| {
                t.lines()
                    .filter_map(|l| {
                        let p: Vec<&str> = l.split('|').collect();
                        if p.len() != 6 {
                            return None;
                        }
                        let arr = |s: &str| -> Option<[u8; 7]> {
                            let v: Vec<u8> = s.split(',').filter_map(|x| x.parse().ok()).collect();
                            <[u8; 7]>::try_from(v).ok()
                        };
                        let claimed =
                            (!p[5].is_empty() && p[5].len() <= 40).then(|| p[5].to_string());
                        Some((
                            p[0].parse().ok()?,
                            WeekRec {
                                tasks: arr(p[1])?,
                                lessons: arr(p[2])?,
                                reviews: arr(p[3])?,
                                gaps: p[4].parse().ok()?,
                                claimed,
                            },
                        ))
                    })
                    .take(MAX_WEEKS)
                    .collect()
            })
            .unwrap_or_default();
        let (gaps_closed_total, closed_gaps) = get("closed_gaps")
            .map(|t| {
                let mut lines = t.lines();
                let total: u32 = lines.next().and_then(|l| l.parse().ok()).unwrap_or(0);
                let list: Vec<ClosedGap> = lines
                    .filter_map(|l| {
                        let p: Vec<&str> = l.split('|').collect();
                        if p.len() != 3 || p[0].is_empty() {
                            return None;
                        }
                        Some((p[0].to_string(), p[1].parse().ok()?, p[2].parse().ok()?))
                    })
                    .take(MAX_CLOSED_GAPS)
                    .collect();
                (total, list)
            })
            .unwrap_or_default();
        let item_stats = get("item_stats")
            .map(|t| {
                t.lines()
                    .filter_map(|l| {
                        let p: Vec<&str> = l.split('|').collect();
                        if p.len() != 4 || p[0].is_empty() {
                            return None;
                        }
                        let (a, c): (u32, u32) = (p[2].parse().ok()?, p[3].parse().ok()?);
                        let item: u32 = p[1].parse().ok()?;
                        (c <= a).then(|| ((p[0].to_string(), item), (a, c)))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let badges = get("badges")
            .map(|t| {
                t.lines()
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let kind = get("kind").map_or(ProfileKind::Child, ProfileKind::parse);
        let works: Vec<WorkRecord> = get("teacher_works")
            .map(|t| {
                t.lines()
                    .filter_map(|l| {
                        let p: Vec<&str> = l.split('|').collect();
                        if p.len() != 6 {
                            return None;
                        }
                        Some(WorkRecord {
                            day: p[0].parse().ok()?,
                            grade: p[1].parse().ok()?,
                            kind: unesc(p[2]),
                            skills: p[3]
                                .split(',')
                                .filter(|s| !s.is_empty())
                                .map(unesc)
                                .collect(),
                            variants: p[4].parse().ok()?,
                            seed: p[5].parse().ok()?,
                        })
                    })
                    .take(MAX_TEACHER_WORKS)
                    .collect()
            })
            .unwrap_or_default();
        let review_notes: Vec<ReviewNote> = get("teacher_notes")
            .map(|t| {
                t.lines()
                    .filter_map(|l| {
                        let p: Vec<&str> = l.split('|').collect();
                        if p.len() != 7 {
                            return None;
                        }
                        Some(ReviewNote {
                            skill: unesc(p[0]),
                            field: unesc(p[1]),
                            index: p[2].parse().ok()?,
                            text: unesc(p[3]),
                            status: unesc(p[4]),
                            correction: unesc(p[5]),
                            comment: unesc(p[6]),
                        })
                    })
                    .take(MAX_REVIEW_NOTES)
                    .collect()
            })
            .unwrap_or_default();
        Ok(AppState {
            kind,
            teacher: TeacherData {
                works_made: get("teacher_works_made")
                    .and_then(|t| t.trim().parse::<u32>().ok())
                    .unwrap_or(0)
                    .max(u32::try_from(works.len()).unwrap_or(0)),
                celebrated: get("teacher_celebrated")
                    .map(|t| {
                        t.lines()
                            .filter(|l| !l.is_empty() && l.len() <= 24)
                            .map(String::from)
                            .take(64)
                            .collect()
                    })
                    .unwrap_or_default(),
                works,
                notes: review_notes,
                bookmarks: get("teacher_bookmarks")
                    .map(|t| {
                        t.lines()
                            .filter(|l| !l.is_empty() && l.len() <= 40)
                            .map(String::from)
                            .take(MAX_BOOKMARKS)
                            .collect()
                    })
                    .unwrap_or_default(),
            },
            profile,
            learner,
            album,
            usage,
            settings,
            session,
            notes,
            session_subject: get("session_subject")
                .filter(|t| !t.is_empty() && t.len() <= 40)
                .map(str::to_string),
            lessons,
            activity,
            last_export_day,
            ceremonies,
            backup_snooze_day,
            flags,
            item_srs,
            closed_gaps,
            hero,
            wardrobe,
            weeks,
            gaps_closed_total,
            item_stats,
            badges,
        })
    }
}

fn encode_hero(
    look: &tutor_core::hero::Look,
    wardrobe: &std::collections::BTreeSet<String>,
) -> String {
    let eq: Vec<String> = look
        .equipped
        .iter()
        .map(|(s, id)| format!("{}:{id}", s.code()))
        .collect();
    format!(
        "skin={}\nhair={}\neq={}\nwardrobe={}\n",
        look.skin,
        look.hair_color,
        eq.join(","),
        wardrobe.iter().cloned().collect::<Vec<_>>().join(",")
    )
}

/// Герой читається поблажливо: хибні чи невідомі предмети відкидаються, обов'язкові шари беруться за замовчуванням.
fn decode_hero(text: &str) -> (tutor_core::hero::Look, std::collections::BTreeSet<String>) {
    use tutor_core::hero::{item, Look, Slot, HAIR_COLORS, SKINS};
    let mut look = Look::default();
    let mut wardrobe = std::collections::BTreeSet::new();
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        match k {
            "skin" => {
                if let Some(n) = v
                    .parse::<u8>()
                    .ok()
                    .filter(|n| usize::from(*n) < SKINS.len())
                {
                    look.skin = n;
                }
            }
            "hair" => {
                if let Some(n) = v
                    .parse::<u8>()
                    .ok()
                    .filter(|n| usize::from(*n) < HAIR_COLORS.len())
                {
                    look.hair_color = n;
                }
            }
            "eq" => {
                look.equipped.clear();
                for pair in v.split(',') {
                    if let Some((slot, id)) = pair.split_once(':') {
                        if let (Some(s), Some(it)) = (Slot::from_code(slot), item(id)) {
                            if it.slot == s {
                                look.equipped.insert(s, id.to_string());
                            }
                        }
                    }
                }
            }
            "wardrobe" => {
                wardrobe = v
                    .split(',')
                    .filter(|id| item(id).is_some_and(|it| !it.free))
                    .map(str::to_string)
                    .collect();
            }
            _ => {}
        }
    }
    // Обов'язкові шари.
    let base = Look::default();
    for (slot, id) in base.equipped {
        look.equipped.entry(slot).or_insert(id);
    }
    // Одягнене має належати дитині: вільне або здобуте.
    look.equipped
        .retain(|_, id| item(id).is_some_and(|it| it.free || wardrobe.contains(id)));
    for (slot, id) in Look::default().equipped {
        look.equipped.entry(slot).or_insert(id);
    }
    (look, wardrobe)
}

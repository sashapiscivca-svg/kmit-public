//! Сесії й ліміти (spec розділ 8): тривалість, перерви між сесіями, добовий ліміт, простій, втома.
//! Усі моменти часу подає оболонка в секундах; ядро годинник не читає.

use crate::config::Config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartDenied {
    /// Добовий ліміт сесій вичерпано.
    MaxSessionsToday,
    /// Ще триває перерва між сесіями.
    BreakNotOver { secs: u32 },
    /// Тихі години, заданi сім'єю (відкрите питання Q4).
    QuietHours,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    /// Час сесії вийшов (після завершення поточної задачі).
    TimeUp,
    /// Жорстка межа досягнута.
    HardCap,
    /// Довгий простій.
    Idle,
    /// Дитина сама завершила.
    UserEnded,
    /// Нічого посильного не лишилося.
    NothingLeft,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    /// Час цілі вийшов: дороби поточну задачу й завершуй, нової не давай.
    WrapUp,
    Ended(EndReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakReason {
    Fatigue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionSummary {
    pub active_secs: u64,
    pub tasks: u32,
    pub solved: u32,
    pub hints: u32,
}

/// Лічильники одного дня для сесій.
#[derive(Clone, Debug, Default)]
pub struct DayUsage {
    day: u32,
    sessions_started: u32,
    last_end: Option<u64>,
}

impl DayUsage {
    pub fn new() -> Self {
        Self::default()
    }

    /// (день, скільки сесій розпочато, момент завершення останньої): для збереження.
    pub fn parts(&self) -> (u32, u32, Option<u64>) {
        (self.day, self.sessions_started, self.last_end)
    }

    pub fn from_parts(day: u32, sessions_started: u32, last_end: Option<u64>) -> Self {
        DayUsage {
            day,
            sessions_started,
            last_end,
        }
    }

    pub fn begin_day(&mut self, day: u32) {
        if day != self.day {
            *self = DayUsage {
                day,
                sessions_started: 0,
                last_end: self.last_end,
            };
        }
    }

    pub fn sessions_started(&self) -> u32 {
        self.sessions_started
    }

    /// `adult_extra`: дорослий дозволив ще сесію понад стандартний ліміт (до `max_sessions_per_day_adult_cap`).
    pub fn can_start(
        &self,
        now: u64,
        quiet_hours: bool,
        adult_extra: bool,
        cfg: &Config,
    ) -> Result<(), StartDenied> {
        if !cfg.enforce_limits {
            return Ok(());
        }
        let limit = if adult_extra {
            cfg.max_sessions_per_day_adult_cap
        } else {
            cfg.max_sessions_per_day
        };
        if self.sessions_started >= limit {
            return Err(StartDenied::MaxSessionsToday);
        }
        if quiet_hours {
            return Err(StartDenied::QuietHours);
        }
        if let Some(end) = self.last_end {
            // Якщо годинник відкотився (`now < end`), перерва вважається завершеною: вона турбота про самопочуття,
            // а не захист, тож помилка годинника не має замикати дитину.
            if now >= end {
                let over_at = end.saturating_add(u64::from(cfg.break_between_sessions_secs));
                if now < over_at {
                    let left = u32::try_from(over_at - now).unwrap_or(u32::MAX);
                    return Err(StartDenied::BreakNotOver {
                        secs: left.min(cfg.break_between_sessions_secs),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn start(
        &mut self,
        now: u64,
        quiet_hours: bool,
        adult_extra: bool,
        cfg: &Config,
    ) -> Result<Session, StartDenied> {
        self.can_start(now, quiet_hours, adult_extra, cfg)?;
        self.sessions_started = self.sessions_started.saturating_add(1);
        Ok(Session::new(now))
    }

    /// Фіксує кінець сесії (момент `now`), від якого рахується перерва.
    pub fn finish(&mut self, now: u64) {
        self.last_end = Some(now);
    }

    /// Завершує сесію з урахуванням її тривалості: надто коротка (збій, зникло світло, випадковий запуск)
    /// не витрачає добовий слот і не дає перерви.
    pub fn finish_session(&mut self, now: u64, session: &Session, cfg: &Config) {
        if session.active_secs() < u64::from(cfg.min_counted_session_secs) {
            self.sessions_started = self.sessions_started.saturating_sub(1);
        } else {
            self.finish(now);
        }
    }
}

#[derive(Clone, Debug)]
pub struct Session {
    started_at: u64,
    last_activity: u64,
    active_secs: u64,
    tasks: u32,
    solved: u32,
    hints: u32,
    ended: Option<EndReason>,
    latencies: Vec<u32>,
    correct: Vec<bool>,
}

impl Session {
    fn new(now: u64) -> Self {
        Session {
            started_at: now,
            last_activity: now,
            active_secs: 0,
            tasks: 0,
            solved: 0,
            hints: 0,
            ended: None,
            latencies: Vec::new(),
            correct: Vec::new(),
        }
    }

    pub fn active_secs(&self) -> u64 {
        self.active_secs
    }

    /// Будь-яка дія дитини в момент `now`. З кожної паузи зараховується її перша хвилина (час біля екрана),
    /// решта до `idle_end_secs` не зараховується (без штрафу), довша пауза завершує сесію. Абсолютна межа за
    /// календарним часом (двічі жорстка межа) зупиняє сесію, навіть якщо дії рідкісні. Час, що йде назад,
    /// нічого не додає, але відлік синхронізується, а не заморожується.
    pub fn activity(&mut self, now: u64, cfg: &Config) -> SessionStatus {
        if self.ended.is_none() {
            if self.past_wall_cap(now, cfg) {
                self.ended = Some(EndReason::HardCap);
            } else {
                let gap = now.saturating_sub(self.last_activity);
                if gap > u64::from(cfg.idle_end_secs) {
                    self.ended = Some(EndReason::Idle);
                } else {
                    self.active_secs = self
                        .active_secs
                        .saturating_add(gap.min(u64::from(cfg.idle_pause_secs)));
                }
                self.last_activity = now;
            }
        }
        self.status(cfg)
    }

    fn past_wall_cap(&self, now: u64, cfg: &Config) -> bool {
        if !cfg.enforce_limits {
            return false;
        }
        now.saturating_sub(self.started_at) > u64::from(cfg.session_hard_cap_secs).saturating_mul(2)
    }

    /// Перевірка без дії дитини (наприклад, таймер оболонки): довгий простій завершує сесію.
    pub fn tick(&mut self, now: u64, cfg: &Config) -> SessionStatus {
        if self.ended.is_none() {
            if self.past_wall_cap(now, cfg) {
                self.ended = Some(EndReason::HardCap);
            } else if now.saturating_sub(self.last_activity) > u64::from(cfg.idle_end_secs) {
                self.ended = Some(EndReason::Idle);
            }
        }
        self.status(cfg)
    }

    pub fn status(&self, cfg: &Config) -> SessionStatus {
        if let Some(r) = self.ended {
            return SessionStatus::Ended(r);
        }
        if cfg.enforce_limits && self.active_secs >= u64::from(cfg.session_hard_cap_secs) {
            return SessionStatus::Ended(EndReason::HardCap);
        }
        if self.active_secs >= u64::from(cfg.session_secs) {
            return SessionStatus::WrapUp;
        }
        SessionStatus::Active
    }

    /// Підсумок задачі: скільки секунд думала дитина, чи правильно, скільки підказок.
    pub fn record_task(&mut self, latency_secs: u32, correct: bool, hints: u8) {
        self.tasks = self.tasks.saturating_add(1);
        if correct {
            self.solved = self.solved.saturating_add(1);
        }
        self.hints = self.hints.saturating_add(u32::from(hints));
        self.latencies.push(latency_secs);
        self.correct.push(correct);
    }

    /// Втома: останні три задачі довші за півтора медіани попередніх і щонайменше дві з трьох невдалі.
    /// Пропонується перерва як вибір, не як покарання.
    pub fn suggest_break(&self, cfg: &Config) -> Option<BreakReason> {
        let n = self.latencies.len();
        if cfg.fatigue_window < 5 || n < cfg.fatigue_window {
            return None;
        }
        let recent = &self.latencies[n - 3..];
        let earlier_start = n.saturating_sub(cfg.fatigue_window);
        let mut earlier: Vec<u32> = self.latencies[earlier_start..n - 3].to_vec();
        earlier.sort_unstable();
        let median = earlier[earlier.len() / 2];
        if median == 0 {
            return None;
        }
        let slow = recent
            .iter()
            .all(|l| u64::from(*l) * 2 > u64::from(median) * 3);
        let wrong = self.correct[n - 3..].iter().filter(|c| !**c).count();
        (slow && wrong >= 2).then_some(BreakReason::Fatigue)
    }

    /// Дитина сама натиснула «закінчити».
    pub fn end_by_user(&mut self) {
        if self.ended.is_none() {
            self.ended = Some(EndReason::UserEnded);
        }
    }

    /// Нічого посильного не лишилося.
    pub fn end_nothing_left(&mut self) {
        if self.ended.is_none() {
            self.ended = Some(EndReason::NothingLeft);
        }
    }

    /// Час цілі вийшов, поточну задачу дороблено: сесія завершується.
    pub fn end_time_up(&mut self) {
        if self.ended.is_none() {
            self.ended = Some(EndReason::TimeUp);
        }
    }

    pub fn summary(&self) -> SessionSummary {
        SessionSummary {
            active_secs: self.active_secs,
            tasks: self.tasks,
            solved: self.solved,
            hints: self.hints,
        }
    }
}

fn end_code(r: EndReason) -> &'static str {
    match r {
        EndReason::TimeUp => "time_up",
        EndReason::HardCap => "hard_cap",
        EndReason::Idle => "idle",
        EndReason::UserEnded => "user",
        EndReason::NothingLeft => "nothing",
    }
}

fn end_from_code(c: &str) -> Option<EndReason> {
    [
        EndReason::TimeUp,
        EndReason::HardCap,
        EndReason::Idle,
        EndReason::UserEnded,
        EndReason::NothingLeft,
    ]
    .into_iter()
    .find(|r| end_code(*r) == c)
}

const SESSION_HEADER: &str = "KMT-SESSION 1";
const SESSION_MAX_TASKS: usize = 10_000;

impl Session {
    /// Текст для збереження: сесію, яку перервало вимкнення світла, можна продовжити з того ж місця.
    pub fn encode(&self) -> String {
        let nums = |v: &[u32]| {
            if v.is_empty() {
                "-".to_string()
            } else {
                v.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
            }
        };
        let flags = if self.correct.is_empty() {
            "-".to_string()
        } else {
            self.correct
                .iter()
                .map(|c| if *c { "1" } else { "0" })
                .collect::<Vec<_>>()
                .join(",")
        };
        format!(
            "{SESSION_HEADER}\nstarted_at {}\nlast_activity {}\nactive {}\ntasks {}\nsolved {}\nhints {}\nended {}\nlatencies {}\ncorrect {}\n",
            self.started_at,
            self.last_activity,
            self.active_secs,
            self.tasks,
            self.solved,
            self.hints,
            self.ended.map_or("-", end_code),
            nums(&self.latencies),
            flags
        )
    }

    pub fn decode(text: &str) -> Result<Session, crate::learner::CodecError> {
        use crate::learner::CodecError;
        let mut lines = text.lines();
        if lines.next() != Some(SESSION_HEADER) {
            return Err(CodecError::Header);
        }
        let mut s = Session::new(0);
        let mut seen = 0u16;
        for (idx, line) in lines.enumerate() {
            let n = idx + 2;
            let bad = CodecError::Line(n);
            let (k, v) = line.split_once(' ').ok_or(CodecError::Line(n))?;
            let u64v = || v.parse::<u64>().map_err(|_| CodecError::Line(n));
            let u32v = || v.parse::<u32>().map_err(|_| CodecError::Line(n));
            let bit: u16 = match k {
                "started_at" => 1,
                "last_activity" => 1 << 1,
                "active" => 1 << 2,
                "tasks" => 1 << 3,
                "solved" => 1 << 4,
                "hints" => 1 << 5,
                "ended" => 1 << 6,
                "latencies" => 1 << 7,
                "correct" => 1 << 8,
                _ => return Err(bad),
            };
            if seen & bit != 0 {
                return Err(bad);
            }
            seen |= bit;
            match k {
                "started_at" => s.started_at = u64v()?,
                "last_activity" => s.last_activity = u64v()?,
                "active" => s.active_secs = u64v()?,
                "tasks" => s.tasks = u32v()?,
                "solved" => s.solved = u32v()?,
                "hints" => s.hints = u32v()?,
                "ended" => {
                    s.ended = if v == "-" {
                        None
                    } else {
                        Some(end_from_code(v).ok_or(CodecError::Line(n))?)
                    };
                }
                "latencies" => {
                    s.latencies = if v == "-" {
                        Vec::new()
                    } else {
                        v.split(',')
                            .map(|x| x.parse::<u32>().map_err(|_| CodecError::Line(n)))
                            .collect::<Result<_, _>>()?
                    };
                    if s.latencies.len() > SESSION_MAX_TASKS {
                        return Err(bad);
                    }
                }
                "correct" => {
                    s.correct = if v == "-" {
                        Vec::new()
                    } else {
                        v.split(',')
                            .map(|x| match x {
                                "1" => Ok(true),
                                "0" => Ok(false),
                                _ => Err(CodecError::Line(n)),
                            })
                            .collect::<Result<_, _>>()?
                    };
                    if s.correct.len() > SESSION_MAX_TASKS {
                        return Err(bad);
                    }
                }
                _ => return Err(bad),
            }
        }
        if seen != 0b1_1111_1111
            || s.latencies.len() != s.correct.len()
            || usize::try_from(s.tasks).ok() != Some(s.latencies.len())
        {
            return Err(CodecError::Line(0));
        }
        Ok(s)
    }
}

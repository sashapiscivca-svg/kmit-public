//! Усі пороги в одному місці. Це інженерні дефолти, які калібруються на пілоті (рішення C6).

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Чи примусово діють ліміти часу (сесії на день, перерва, тихі години, жорстка межа сесії, ліміт задач навички за день).
    /// За замовчуванням ні: навчання не блокується, час і перерви лише радяться (рішення A9). Родина чи сам учень
    /// можуть увімкнути обмеження в налаштуваннях профілю.
    pub enforce_limits: bool,
    /// Поріг ймовірності опанування P(L).
    pub mastery_p: f64,
    /// Скільки правильних поспіль без підказок потрібно.
    pub mastery_streak: u32,
    /// Скільки різних днів з чистим успіхом потрібно.
    pub mastery_days: usize,
    /// Після скількох помилок поспіль пропонується підказка.
    pub offer_hint_after_wrong: u8,
    /// Скільки невдалих спроб після аналога R4 до «запитай дорослого».
    pub max_wrong_after_analog: u8,
    /// Інтервали повторення в днях після опанування (1, 3, 7, 14, 30).
    pub review_intervals_days: [u32; 5],
    /// Скільки повторень максимум пропонується за сесію (без «лавини боргу» після пропусків).
    pub max_reviews_per_session: usize,
    /// Після скількох днів прострочення замість повторення пропонується короткий діагностичний мікротест.
    pub microtest_after_overdue_days: u32,
    /// Ліміт задач однієї навички за день.
    pub max_tasks_per_skill_per_day: u32,
    /// Цільова ймовірність успіху для вибору посильної навички.
    pub target_success: f64,
    /// Кожна n-та задача за день є повторенням, якщо є прострочені (перемежування).
    pub interleave_every: u32,
    /// Скільки останніх показаних умов навички не повторюється, коли пул умов вичерпано й починається нове коло.
    pub repeat_guard_recent: usize,
    /// Готовність: передумова відкриває наступні навички, лише якщо оцінка її знання не нижча за це (spec 5.3).
    pub readiness_p: f64,
    /// Відкат: опанована навичка знову вважається такою, що потребує навчання, якщо оцінка впала нижче за це (spec 5.3).
    pub rollback_p: f64,
    /// Скільки задач у діагностичному мікротесті після довгого пропуску (spec 5.5).
    pub microtest_tasks: u8,
    /// Скільки з них мають бути розв'язані чисто, щоб навичку було збережено.
    pub microtest_pass_clean: u8,
    /// Мінімальна затримка між рівнями підказок, секунди (відкрите питання Q2: калібрується на пілоті).
    pub hint_min_delay_secs: u32,
    /// Скільки рівнів підказки поспіль без жодної спроби відповіді вважаються «клацанням до відповіді» (gaming).
    pub gaming_rapid_requests: u8,
    /// На скільки секунд блокуються підказки після спрацювання gaming-тригера.
    pub gaming_block_secs: u32,
    /// Чи вимагати хоч одну спробу відповіді до першої підказки (Q1, рішення власника: так, спершу власна спроба).
    pub require_attempt_before_hint: bool,
    /// Цільова тривалість сесії, секунди (активний час без пауз); після неї сесія завершується після поточної задачі.
    pub session_secs: u32,
    /// Жорстка межа сесії, секунди: після неї навіть поточну задачу не продовжують.
    pub session_hard_cap_secs: u32,
    /// Пауза без дій довша за це не зараховується в тривалість сесії (без штрафу), секунди.
    pub idle_pause_secs: u32,
    /// Простій довший за це завершує сесію, секунди.
    pub idle_end_secs: u32,
    /// Перерва між сесіями, секунди.
    pub break_between_sessions_secs: u32,
    /// Сесій на день за замовчуванням.
    pub max_sessions_per_day: u32,
    /// Скільки сесій на день може дозволити дорослий максимум (відкрите питання Q4).
    pub max_sessions_per_day_adult_cap: u32,
    /// Скільки останніх задач потрібно для оцінки втоми.
    pub fatigue_window: usize,
    /// Сесія коротша за це (збій, випадковий запуск) не витрачає добовий слот і не дає перерви, секунди.
    pub min_counted_session_secs: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enforce_limits: false,
            mastery_p: 0.95,
            mastery_streak: 3,
            mastery_days: 2,
            offer_hint_after_wrong: 2,
            max_wrong_after_analog: 2,
            review_intervals_days: [1, 3, 7, 14, 30],
            max_reviews_per_session: 3,
            microtest_after_overdue_days: 30,
            max_tasks_per_skill_per_day: 8,
            target_success: 0.8,
            interleave_every: 3,
            repeat_guard_recent: 20,
            readiness_p: 0.8,
            rollback_p: 0.6,
            microtest_tasks: 3,
            microtest_pass_clean: 2,
            hint_min_delay_secs: 12,
            gaming_rapid_requests: 3,
            gaming_block_secs: 60,
            require_attempt_before_hint: true,
            session_secs: 20 * 60,
            session_hard_cap_secs: 25 * 60,
            idle_pause_secs: 60,
            idle_end_secs: 5 * 60,
            break_between_sessions_secs: 4 * 60,
            max_sessions_per_day: 2,
            max_sessions_per_day_adult_cap: 3,
            fatigue_window: 8,
            min_counted_session_secs: 120,
        }
    }
}

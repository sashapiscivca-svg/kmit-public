//! Затримка між рівнями підказок і виявлення «клацання до відповіді» (gaming the system).
//! Час подає оболонка (секунди з монотонного годинника); ядро годинник не читає.
//!
//! Нетерпляче натискання під час очікування безпечне (завжди лише `Wait`, без штрафу). «Клацанням до відповіді»
//! вважається отримання кількох рівнів поспіль без жодної спроби відповісти між ними.

use crate::config::Config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateDecision {
    /// Можна показувати наступний рівень.
    Allow,
    /// Зачекай `secs` секунд. Інтерфейс показує запитання «що ти вже знаєш?», а не зворотний відлік.
    Wait { secs: u32 },
    /// Спершу спробуй відповісти (вмикається налаштуванням `require_attempt_before_hint`).
    NeedAttempt,
    /// Підказки тимчасово недоступні: пропонується спробувати відповісти, перерва або інше завдання.
    Blocked { secs: u32 },
}

#[derive(Clone, Debug, Default)]
pub struct HintGate {
    last_hint_at: Option<u64>,
    attempts_made: u32,
    /// Скільки рівнів видано поспіль без спроби відповіді.
    grants_in_a_row: u8,
    blocked_until: Option<u64>,
}

impl HintGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Записує спробу відповіді (правильну чи ні): серія «клацання» починається знову.
    pub fn note_attempt(&mut self) {
        self.attempts_made = self.attempts_made.saturating_add(1);
        self.grants_in_a_row = 0;
    }

    /// Дитина просить підказку в момент `now`. Лише `Allow` означає, що рівень можна показувати.
    pub fn request(&mut self, now: u64, cfg: &Config) -> GateDecision {
        if let Some(until) = self.blocked_until {
            if now < until {
                return GateDecision::Blocked {
                    secs: u32::try_from(until - now).unwrap_or(u32::MAX),
                };
            }
            self.blocked_until = None;
            self.grants_in_a_row = 0;
        }
        if cfg.require_attempt_before_hint && self.attempts_made == 0 {
            return GateDecision::NeedAttempt;
        }
        let min_delay = u64::from(cfg.hint_min_delay_secs);
        if let Some(last) = self.last_hint_at {
            // Годинник відкотився: відлік іде від нового часу, а не чекає, поки він «наздожене».
            if now < last {
                self.last_hint_at = Some(now);
            }
            let elapsed = now.saturating_sub(self.last_hint_at.unwrap_or(now));
            if elapsed < min_delay {
                let secs = u32::try_from(min_delay - elapsed).unwrap_or(u32::MAX);
                return GateDecision::Wait { secs };
            }
            // Довга власна думка між підказками теж скидає серію.
            if elapsed > min_delay.saturating_mul(3) {
                self.grants_in_a_row = 0;
            }
        }
        if self.grants_in_a_row >= cfg.gaming_rapid_requests {
            self.blocked_until = Some(now.saturating_add(u64::from(cfg.gaming_block_secs)));
            return GateDecision::Blocked {
                secs: cfg.gaming_block_secs,
            };
        }
        self.last_hint_at = Some(now);
        self.grants_in_a_row = self.grants_in_a_row.saturating_add(1);
        GateDecision::Allow
    }
}

//! Драбина підказок R1 → R2 → R3 → R4 (аналог) → повернення → «запитай дорослого».
//! Стан не містить жодних відповідей: тільки лічильники.

use crate::config::Config;
use crate::generator::ScriptedLevel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LadderAction {
    ShowScripted(ScriptedLevel),
    ShowAnalog,
    ReturnToTask,
    OfferHint,
    AskAdult,
}

#[derive(Clone, Debug)]
pub struct Ladder {
    stage: u8,
    wrong_streak: u8,
    wrong_after_analog: u8,
    ask_adult: bool,
    offer_after: u8,
    max_after_analog: u8,
}

impl Ladder {
    pub fn new(cfg: &Config) -> Self {
        Ladder {
            stage: 0,
            wrong_streak: 0,
            wrong_after_analog: 0,
            ask_adult: false,
            offer_after: cfg.offer_hint_after_wrong,
            max_after_analog: cfg.max_wrong_after_analog,
        }
    }

    /// Скільки рівнів підказки вже показано (0..=4).
    pub fn hints_used(&self) -> u8 {
        self.stage
    }

    /// Дитина просить підказку.
    pub fn request_hint(&mut self) -> LadderAction {
        if self.ask_adult || (self.stage >= 4 && self.wrong_after_analog >= self.max_after_analog) {
            self.ask_adult = true;
            return LadderAction::AskAdult;
        }
        match self.stage {
            0 => {
                self.stage = 1;
                LadderAction::ShowScripted(ScriptedLevel::R1)
            }
            1 => {
                self.stage = 2;
                LadderAction::ShowScripted(ScriptedLevel::R2)
            }
            2 => {
                self.stage = 3;
                LadderAction::ShowScripted(ScriptedLevel::R3)
            }
            3 => {
                self.stage = 4;
                LadderAction::ShowAnalog
            }
            _ => LadderAction::ReturnToTask,
        }
    }

    /// Дитина відповіла неправильно.
    pub fn record_wrong(&mut self) -> LadderAction {
        if self.ask_adult {
            return LadderAction::AskAdult;
        }
        if self.stage >= 4 {
            self.wrong_after_analog = self.wrong_after_analog.saturating_add(1);
            if self.wrong_after_analog >= self.max_after_analog {
                self.ask_adult = true;
                return LadderAction::AskAdult;
            }
            return LadderAction::ReturnToTask;
        }
        self.wrong_streak = self.wrong_streak.saturating_add(1);
        if self.stage == 0 && self.wrong_streak >= self.offer_after {
            LadderAction::OfferHint
        } else {
            LadderAction::ReturnToTask
        }
    }

    /// І3: безпечного аналога немає. Розв'язку не показуємо, переходимо до дорослого.
    pub fn analog_unavailable(&mut self) {
        self.ask_adult = true;
    }
}

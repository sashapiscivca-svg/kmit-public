//! Модель знань BKT з фіксованими параметрами. Лише +, -, *, / над f64: без libm, тож результат
//! однаковий на всіх платформах.

use crate::config::Config;
use std::collections::BTreeSet;

/// Межі ймовірності: без них P(L) може дійти до 1.0 і більше ніколи не знизитися навіть після помилок.
pub const P_MIN: f64 = 0.001;
pub const P_MAX: f64 = 0.999;

#[derive(Debug, PartialEq, Eq)]
pub enum BktError {
    OutOfRange,
    GuessTooHigh,
    SlipTooHigh,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BktParams {
    p_init: f64,
    p_learn: f64,
    p_guess: f64,
    p_slip: f64,
}

impl BktParams {
    /// Обмеження за Baker та ін.: P(G) < 0.5 і P(S) < 0.5 (звідси P(G) + P(S) < 1); усі значення в (0, 1).
    pub fn new(p_init: f64, p_learn: f64, p_guess: f64, p_slip: f64) -> Result<Self, BktError> {
        for v in [p_init, p_learn, p_guess, p_slip] {
            if !(v > 0.0 && v < 1.0) {
                return Err(BktError::OutOfRange);
            }
        }
        if p_guess >= 0.5 {
            return Err(BktError::GuessTooHigh);
        }
        if p_slip >= 0.5 {
            return Err(BktError::SlipTooHigh);
        }
        Ok(BktParams {
            p_init,
            p_learn,
            p_guess,
            p_slip,
        })
    }

    /// Стартові значення для навичок з числовою відповіддю (дефолт, калібрується пілотом).
    pub fn numeric_default() -> Self {
        BktParams {
            p_init: 0.3,
            p_learn: 0.12,
            p_guess: 0.05,
            p_slip: 0.1,
        }
    }

    /// Для завдань із вибором із ≥ 4 варіантів: вгадати можна з імовірністю до чверті (P(G) = 0,25).
    pub fn choice_default() -> Self {
        BktParams {
            p_init: 0.25,
            p_learn: 0.15,
            p_guess: 0.25,
            p_slip: 0.1,
        }
    }

    pub fn p_init(&self) -> f64 {
        self.p_init
    }

    /// Очікувана ймовірність правильної відповіді за оцінки P(L): знає й не схибив, або не знає й вгадав.
    pub fn predicted_success(&self, p_known: f64) -> f64 {
        p_known * (1.0 - self.p_slip) + (1.0 - p_known) * self.p_guess
    }

    /// Одне оновлення після спостереження. `correct` = правильно й без підказок.
    pub fn update(&self, p_known: f64, correct: bool) -> f64 {
        let (num, den) = if correct {
            let hit = p_known * (1.0 - self.p_slip);
            (hit, hit + (1.0 - p_known) * self.p_guess)
        } else {
            let miss = p_known * self.p_slip;
            (miss, miss + (1.0 - p_known) * (1.0 - self.p_guess))
        };
        let posterior = num / den;
        (posterior + (1.0 - posterior) * self.p_learn).clamp(P_MIN, P_MAX)
    }
}

#[derive(Clone, Debug)]
pub struct SkillState {
    pub p_known: f64,
    pub streak: u32,
    /// Скільки задач цієї навички вже зараховано (з підказками чи без).
    pub attempts: u32,
    clean_days: BTreeSet<u32>,
}

impl SkillState {
    pub fn new(params: &BktParams) -> Self {
        SkillState {
            p_known: params.p_init(),
            streak: 0,
            attempts: 0,
            clean_days: BTreeSet::new(),
        }
    }

    /// Відновлення зі збережених частин (з сховища). Значення не перевіряються тут: це робить кодек.
    pub fn from_parts(p_known: f64, streak: u32, attempts: u32, clean_days: BTreeSet<u32>) -> Self {
        SkillState {
            p_known,
            streak,
            attempts,
            clean_days,
        }
    }

    /// Дні, у які були чисті успіхи (для збереження й перевірки опанування).
    pub fn clean_days(&self) -> &BTreeSet<u32> {
        &self.clean_days
    }

    /// Записати результат задачі. `day` — номер дня (ціле), `hints_used` — скільки підказок взято.
    pub fn observe(&mut self, params: &BktParams, day: u32, correct: bool, hints_used: u8) {
        let clean = correct && hints_used == 0;
        self.attempts = self.attempts.saturating_add(1);
        self.p_known = params.update(self.p_known, clean);
        if clean {
            self.streak = self.streak.saturating_add(1);
            self.clean_days.insert(day);
        } else {
            self.streak = 0;
        }
    }

    /// Опанування: P(L) ≥ поріг, достатня серія й достатньо різних днів.
    pub fn is_mastered(&self, cfg: &Config) -> bool {
        self.p_known >= cfg.mastery_p
            && self.streak >= cfg.mastery_streak
            && self.clean_days.len() >= cfg.mastery_days
    }
}

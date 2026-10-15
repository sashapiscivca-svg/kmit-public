//! Повторення за зростаючими інтервалами на цілих числах (рішення C3). Без FSRS і без дробових розрахунків.

use crate::config::Config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReviewSchedule {
    level: u8,
    due_day: u32,
}

/// Найвищий рівень розкладу (індекс останнього інтервалу).
pub const MAX_LEVEL: u8 = 4;

impl ReviewSchedule {
    /// Відновлення зі збережених частин. `None`, якщо рівень поза межами.
    pub fn from_parts(level: u8, due_day: u32) -> Option<Self> {
        (level <= MAX_LEVEL).then_some(ReviewSchedule { level, due_day })
    }

    /// Розклад одразу після опанування навички в день `day`.
    pub fn after_mastery(day: u32, cfg: &Config) -> Self {
        ReviewSchedule {
            level: 0,
            due_day: day.saturating_add(cfg.review_intervals_days[0]),
        }
    }

    /// Розклад після повторення. Чистий успіх піднімає рівень (довший інтервал), будь-що інше скидає на перший.
    pub fn after_review(self, clean: bool, day: u32, cfg: &Config) -> Self {
        let last = cfg.review_intervals_days.len() - 1;
        let next = if clean {
            (usize::from(self.level) + 1).min(last)
        } else {
            0
        };
        ReviewSchedule {
            level: u8::try_from(next).unwrap_or(0),
            due_day: day.saturating_add(cfg.review_intervals_days[next]),
        }
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn due_day(&self) -> u32 {
        self.due_day
    }

    pub fn is_due(&self, day: u32) -> bool {
        day >= self.due_day
    }

    pub fn overdue_days(&self, day: u32) -> u32 {
        day.saturating_sub(self.due_day)
    }

    /// Після довгого пропуску замість «боргу» пропонується короткий діагностичний мікротест.
    pub fn needs_microtest(&self, day: u32, cfg: &Config) -> bool {
        self.overdue_days(day) >= cfg.microtest_after_overdue_days
    }
}

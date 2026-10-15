//! Задача: умова, правильна відповідь, захищені значення, типові помилки.

use crate::frac::{parse_answer, Frac};
use crate::numtok::numeric_tokens;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BugAnswer {
    pub id: &'static str,
    pub value: Frac,
}

#[derive(Clone, Debug)]
pub struct Task {
    pub skill: &'static str,
    pub variant: u64,
    /// Текст умови, який бачить дитина.
    pub statement: String,
    /// Числа з умови (нормалізовані).
    pub operands: Vec<Frac>,
    /// Підписи операндів так, як вони записані в умові (наприклад `2/8`).
    pub operand_labels: Vec<String>,
    /// Приватні параметри генератора (потрібні, щоб побудувати аналог).
    pub params: Vec<i64>,
    pub answer: Frac,
    /// Проміжні значення розв'язку.
    pub steps: Vec<Frac>,
    pub bugs: Vec<BugAnswer>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Correct,
    Bug(&'static str),
    Wrong,
    Unparseable,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TaskError {
    OracleMismatch,
    ProtectedOverlapsOperand,
    BugEqualsAnswer,
    BugEqualsOperand,
    BugsNotDistinct,
    StatementNumbersMismatch,
}

impl Task {
    /// Значення, які підказка не має права показувати: відповідь і проміжні результати.
    pub fn protected_values(&self) -> Vec<Frac> {
        // Завдання з вибором (без операндів): відповідь це лише номер варіанта, числового секрету немає.
        if self.operands.is_empty() {
            return Vec::new();
        }
        let mut set: BTreeSet<Frac> = self.steps.iter().copied().collect();
        set.insert(self.answer);
        set.into_iter().collect()
    }

    pub fn check(&self, input: &str) -> Verdict {
        let Some(v) = parse_answer(input) else {
            return Verdict::Unparseable;
        };
        if v == self.answer {
            return Verdict::Correct;
        }
        if let Some(b) = self.bugs.iter().find(|b| b.value == v) {
            return Verdict::Bug(b.id);
        }
        Verdict::Wrong
    }
}

/// Перевірка якості згенерованої задачі. `oracle` обчислено іншим шляхом, ніж `task.answer`.
pub fn validate_task(task: &Task, oracle: Frac) -> Result<(), TaskError> {
    if task.answer != oracle {
        return Err(TaskError::OracleMismatch);
    }
    let operands: BTreeSet<Frac> = task.operands.iter().copied().collect();
    if task.protected_values().iter().any(|p| operands.contains(p)) {
        return Err(TaskError::ProtectedOverlapsOperand);
    }
    let mut seen = BTreeSet::new();
    for b in &task.bugs {
        if b.value == task.answer {
            return Err(TaskError::BugEqualsAnswer);
        }
        if operands.contains(&b.value) {
            return Err(TaskError::BugEqualsOperand);
        }
        if !seen.insert(b.value) {
            return Err(TaskError::BugsNotDistinct);
        }
    }
    let in_statement: BTreeSet<Frac> = numeric_tokens(&task.statement).into_iter().collect();
    if in_statement != operands {
        return Err(TaskError::StatementNumbersMismatch);
    }
    Ok(())
}

//! Контракт генератора навички та типи підказок.

use crate::frac::Frac;
use crate::task::Task;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptedLevel {
    R1,
    R2,
    R3,
}

/// R4: аналогічний приклад з іншими числами. Фінальну відповідь аналога дописує дитина.
#[derive(Clone, Debug)]
pub struct WorkedExample {
    pub task: Task,
    /// Показані кроки розв'язання аналога (без фінальної відповіді).
    pub shown_steps: Vec<String>,
    /// Прохання до дитини завершити аналог.
    pub final_prompt: String,
}

impl WorkedExample {
    /// Перевірка того, що дописала дитина в аналозі (фінальну відповідь аналога знає лише рушій).
    pub fn check_final(&self, input: &str) -> crate::task::Verdict {
        self.task.check(input)
    }

    /// Усі тексти, які побачить дитина.
    pub fn texts(&self) -> Vec<&str> {
        let mut v = vec![self.task.statement.as_str()];
        v.extend(self.shown_steps.iter().map(String::as_str));
        v.push(self.final_prompt.as_str());
        v
    }
}

/// Яка відповідь очікується від учня.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnswerKind {
    /// Число, дріб чи десятковий дріб (обчислювальні задачі).
    Number,
    /// Вибір одного з варіантів; відповідь учня це номер варіанта з 1 (завдання з банку).
    Choice,
    /// Слово чи коротка фраза, яку учень вводить сам (правопис, переклад слова).
    Text,
}

/// Відгук після спроби для завдань із вибором: правильна відповідь і чому (рішення C42).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reveal {
    pub answer_text: String,
    pub explanation: String,
}

pub trait SkillGenerator: Send {
    fn skill_id(&self) -> &'static str;
    /// Детермінована задача за номером варіанта.
    fn generate(&self, variant: u64) -> Task;
    /// Скриптована підказка рівнів R1-R3. Тексти без відповіді та проміжних результатів задачі.
    fn hint(&self, task: &Task, level: ScriptedLevel) -> String;
    /// Аналогічний приклад (R4). `avoid` — відповіді, які дитина вже ввела й почула «неправильно»:
    /// відповідь аналога не повинна з ними збігатися. `None`, якщо безпечного аналога не знайдено.
    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample>;

    /// Вид відповіді (за замовчуванням число).
    fn answer_kind(&self) -> AnswerKind {
        AnswerKind::Number
    }

    /// Варіанти відповідей у показаному порядку (лише для `Choice`).
    fn choices(&self, _task: &Task) -> Vec<String> {
        Vec::new()
    }

    /// Відгук після спроби (для `Choice` і `Text`).
    fn reveal(&self, _task: &Task) -> Option<Reveal> {
        None
    }

    /// Перевірка введеного слова (лише для `Text`): `Some(true/false)`; `None`, якщо навичка цього не вміє.
    fn check_text(&self, _task: &Task, _input: &str) -> Option<bool> {
        None
    }

    /// Малюнки до умови (смужки, кут, прямокутник…): лише дані умови, не відповідь (І1).
    fn figures(&self, _task: &Task) -> Vec<crate::figure::Figure> {
        Vec::new()
    }
}

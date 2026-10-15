//! Банк завдань із вибором відповіді для предметів, де знання фактичні (мова, історія, природознавство тощо).
//! Це не обчислювальні задачі, тож тут діє інший принцип (рішення C42): після власної спроби показується правильна
//! відповідь із коротким поясненням, а повторення розкладається за інтервалами. Підказки до спроби лише навідні.

use crate::frac::Frac;
use crate::generator::{AnswerKind, Reveal, ScriptedLevel, SkillGenerator, WorkedExample};
use crate::rng::{seed_for, SeededRng};
use crate::task::Task;

/// Одне авторське завдання. Правильна відповідь лише одна; неправильні мають бути правдоподібними (типові хибні уявлення).
#[derive(Clone, Debug)]
pub struct BankItem {
    pub prompt: String,
    pub correct: String,
    pub wrong: Vec<String>,
    /// Чому саме так (показується після спроби, і коли відповідь вірна).
    pub explanation: String,
    /// Навідні запитання R1-R3 (не містять відповіді; порядок від загального до конкретного).
    pub hints: Vec<String>,
}

/// Генератор навички з банку завдань. Варіант `v` відповідає завданню `v mod N`; порядок варіантів відповідей
/// перемішується детерміновано від (навичка, варіант).
#[derive(Clone)]
pub struct BankGenerator {
    skill: &'static str,
    items: std::sync::Arc<Vec<BankItem>>,
}

impl BankGenerator {
    pub fn new(skill: &'static str, items: Vec<BankItem>) -> Self {
        assert!(!items.is_empty(), "порожній банк навички {skill}");
        BankGenerator {
            skill,
            items: std::sync::Arc::new(items),
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Усі завдання (для експорту вмісту й огляду).
    pub fn items(&self) -> &[BankItem] {
        &self.items
    }

    /// Перевірка авторських помилок цього банку.
    pub fn lint(&self) -> Vec<String> {
        lint_items(&self.items)
    }

    fn item_of(&self, task: &Task) -> &BankItem {
        let idx = usize::try_from(task.params[0]).unwrap_or(0) % self.items.len();
        &self.items[idx]
    }

    /// Показаний порядок варіантів: (тексти, позиція правильного з нуля).
    fn arrange(&self, item_idx: usize, variant: u64) -> (Vec<String>, usize) {
        let item = &self.items[item_idx];
        let mut opts: Vec<(bool, &String)> = std::iter::once((true, &item.correct))
            .chain(item.wrong.iter().map(|w| (false, w)))
            .collect();
        let mut rng = SeededRng::new(seed_for(self.skill, variant));
        // Фішер-Йейтс на детермінованому генераторі.
        for i in (1..opts.len()).rev() {
            let j = usize::try_from(rng.range(0, i as i64)).unwrap_or(0);
            opts.swap(i, j);
        }
        let pos = opts.iter().position(|(c, _)| *c).unwrap_or(0);
        (opts.into_iter().map(|(_, t)| t.clone()).collect(), pos)
    }
}

impl SkillGenerator for BankGenerator {
    fn skill_id(&self) -> &'static str {
        self.skill
    }

    fn generate(&self, variant: u64) -> Task {
        let item_idx = usize::try_from(variant % self.items.len() as u64).unwrap_or(0);
        let (_, pos) = self.arrange(item_idx, variant);
        Task {
            skill: self.skill,
            variant,
            statement: self.items[item_idx].prompt.clone(),
            // Порожні операнди: у завданні з вибором немає числового секрету (відповідь це номер варіанта).
            operands: vec![],
            operand_labels: vec![],
            params: vec![item_idx as i64],
            answer: Frac::int(pos as i64 + 1),
            steps: vec![],
            bugs: vec![],
        }
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        let item = self.item_of(task);
        let i = match level {
            ScriptedLevel::R1 => 0,
            ScriptedLevel::R2 => 1,
            ScriptedLevel::R3 => 2,
        };
        item.hints
            .get(i)
            .or_else(|| item.hints.last())
            .cloned()
            .unwrap_or_else(|| {
                "Пригадай урок і спробуй виключити варіанти, які точно не підходять.".to_string()
            })
    }

    fn analog(&self, _task: &Task, _avoid: &[Frac]) -> Option<WorkedExample> {
        None
    }

    fn answer_kind(&self) -> AnswerKind {
        AnswerKind::Choice
    }

    fn choices(&self, task: &Task) -> Vec<String> {
        let idx = usize::try_from(task.params[0]).unwrap_or(0) % self.items.len();
        self.arrange(idx, task.variant).0
    }

    fn reveal(&self, task: &Task) -> Option<Reveal> {
        let item = self.item_of(task);
        Some(Reveal {
            answer_text: item.correct.clone(),
            explanation: item.explanation.clone(),
        })
    }
}

/// Чи видає підказка правильну відповідь. Для довгих відповідей це входження підрядка; для коротких (до трьох
/// символів, як «at», «a», «ф») лише збіг із цілим словом: інакше будь-яка підказка містить таку літеру.
pub fn hint_leaks(hint: &str, correct: &str) -> bool {
    let hint = hint.to_lowercase();
    let correct = correct.trim().to_lowercase();
    if correct.is_empty() {
        return false;
    }
    if correct.chars().count() <= 3 {
        return hint
            .split(|c: char| !(c.is_alphanumeric() || c == '\''))
            .any(|w| w == correct);
    }
    hint.contains(&correct)
}

/// Перевірка авторських помилок: повертає перелік проблем (порожній, якщо все гаразд).
pub fn lint_items(items: &[BankItem]) -> Vec<String> {
    let mut out = Vec::new();
    let mut prompts = std::collections::BTreeSet::new();
    for (i, it) in items.iter().enumerate() {
        let at = |m: &str| format!("#{i} «{}»: {m}", it.prompt);
        if it.prompt.trim().is_empty() {
            out.push(at("порожнє питання"));
        }
        if !prompts.insert(it.prompt.trim().to_lowercase()) {
            out.push(at("питання повторюється"));
        }
        if it.wrong.len() < 2 {
            out.push(at("потрібно щонайменше 2 хибні варіанти (разом ≥ 3)"));
        }
        let mut all: Vec<String> = it.wrong.iter().map(|w| w.trim().to_lowercase()).collect();
        all.push(it.correct.trim().to_lowercase());
        let n = all.len();
        all.sort();
        all.dedup();
        if all.len() != n {
            out.push(at(
                "варіанти повторюються або хибний збігається з правильним",
            ));
        }
        if all.iter().any(String::is_empty) {
            out.push(at("порожній варіант"));
        }
        if it.explanation.trim().is_empty() {
            out.push(at("немає пояснення"));
        }
        if it.hints.is_empty() {
            out.push(at("немає підказок"));
        }
        for h in &it.hints {
            if hint_leaks(h, &it.correct) {
                out.push(at("підказка містить правильну відповідь"));
            }
        }
    }
    out
}

/// Завдання з введенням слова: приймається будь-яка з `accepted` (без урахування регістру, зайвих пробілів,
/// різновидів апострофа). Правильною для відгуку вважається перша.
#[derive(Clone, Debug)]
pub struct TypedItem {
    pub prompt: String,
    pub accepted: Vec<String>,
    pub explanation: String,
    /// Навідні підказки без відповіді (до трьох).
    pub hints: Vec<String>,
}

/// Нормалізація введеного слова для порівняння.
pub fn normalize_word(s: &str) -> String {
    s.trim()
        .chars()
        .map(|c| match c {
            '’' | 'ʼ' | '`' | '′' => '\'',
            c => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Генератор навички з введенням слова. Варіант `v` відповідає завданню `v mod N`.
#[derive(Clone)]
pub struct TypedBankGenerator {
    skill: &'static str,
    items: std::sync::Arc<Vec<TypedItem>>,
}

impl TypedBankGenerator {
    pub fn new(skill: &'static str, items: Vec<TypedItem>) -> Self {
        assert!(!items.is_empty(), "порожній банк навички {skill}");
        TypedBankGenerator {
            skill,
            items: std::sync::Arc::new(items),
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn lint(&self) -> Vec<String> {
        lint_typed(&self.items)
    }

    /// Усі завдання (для експорту вмісту й огляду).
    pub fn items(&self) -> &[TypedItem] {
        &self.items
    }

    fn item_of(&self, task: &Task) -> &TypedItem {
        let idx = usize::try_from(task.params[0]).unwrap_or(0) % self.items.len();
        &self.items[idx]
    }
}

impl SkillGenerator for TypedBankGenerator {
    fn skill_id(&self) -> &'static str {
        self.skill
    }

    fn generate(&self, variant: u64) -> Task {
        let idx = usize::try_from(variant % self.items.len() as u64).unwrap_or(0);
        Task {
            skill: self.skill,
            variant,
            statement: self.items[idx].prompt.clone(),
            operands: vec![],
            operand_labels: vec![],
            params: vec![idx as i64],
            answer: Frac::int(0),
            steps: vec![],
            bugs: vec![],
        }
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        let item = self.item_of(task);
        let i = match level {
            ScriptedLevel::R1 => 0,
            ScriptedLevel::R2 => 1,
            ScriptedLevel::R3 => 2,
        };
        item.hints
            .get(i)
            .or_else(|| item.hints.last())
            .cloned()
            .unwrap_or_else(|| "Пригадай урок і подумай ще раз.".to_string())
    }

    fn analog(&self, _task: &Task, _avoid: &[Frac]) -> Option<WorkedExample> {
        None
    }

    fn answer_kind(&self) -> AnswerKind {
        AnswerKind::Text
    }

    fn reveal(&self, task: &Task) -> Option<Reveal> {
        let item = self.item_of(task);
        Some(Reveal {
            answer_text: item.accepted.first().cloned().unwrap_or_default(),
            explanation: item.explanation.clone(),
        })
    }

    fn check_text(&self, task: &Task, input: &str) -> Option<bool> {
        let given = normalize_word(input);
        Some(
            self.item_of(task)
                .accepted
                .iter()
                .any(|a| normalize_word(a) == given),
        )
    }
}

/// Перевірка авторських помилок у завданнях з введенням слова.
pub fn lint_typed(items: &[TypedItem]) -> Vec<String> {
    let mut out = Vec::new();
    let mut prompts = std::collections::BTreeSet::new();
    for (i, it) in items.iter().enumerate() {
        let at = |m: &str| format!("#{i} «{}»: {m}", it.prompt);
        if it.prompt.trim().is_empty() {
            out.push(at("порожнє питання"));
        }
        if !prompts.insert(it.prompt.trim().to_lowercase()) {
            out.push(at("питання повторюється"));
        }
        if it.accepted.is_empty() || it.accepted.iter().any(|a| normalize_word(a).is_empty()) {
            out.push(at("немає прийнятної відповіді або вона порожня"));
        }
        if it.explanation.trim().chars().count() < 15 {
            out.push(at("немає пояснення"));
        }
        if it.hints.is_empty() {
            out.push(at("немає підказок"));
        }
        for h in &it.hints {
            for a in &it.accepted {
                let a = normalize_word(a);
                if !a.is_empty() && normalize_word(h).contains(&a) {
                    out.push(at("підказка містить відповідь"));
                }
            }
        }
    }
    out
}

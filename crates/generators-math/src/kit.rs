//! Спільні інструменти для навичок розширеного курсу: форми іменників, десяткові записи,
//! конструктор задачі з цілими операндами та `Spec`: генератор, заданий набором функцій.

use crate::common::{build, find_analog};
use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::rng::SeededRng;
use tutor_core::task::{BugAnswer, Task};

/// Три форми іменника для числівників: називний однини, називний множини, родовий множини
/// (наприклад «книжка», «книжки», «книжок»). Числа в умовах стоять у називному чи знахідному відмінку.
pub type Forms = [&'static str; 3];

/// Форма іменника, що узгоджується з числом `n` (1 книжка, 2 книжки, 5 книжок, 11 книжок, 21 книжка).
pub fn form(n: i64, f: &Forms) -> &'static str {
    let (m10, m100) = (n % 10, n % 100);
    if (11..=14).contains(&m100) {
        f[2]
    } else if m10 == 1 {
        f[0]
    } else if (2..=4).contains(&m10) {
        f[1]
    } else {
        f[2]
    }
}

/// Число з іменником: «21 книжка».
pub fn pl(n: i64, f: &Forms) -> String {
    format!("{n} {}", form(n, f))
}

/// Десятковий запис `units / 10^digits` з комою, без зайвих нулів у кінці.
pub fn dec_str(units: i64, digits: u32) -> String {
    let scale = 10_i64.pow(digits);
    let (int, frac) = (units / scale, units % scale);
    if frac == 0 {
        return int.to_string();
    }
    let mut f = format!("{frac:0width$}", width = digits as usize);
    while f.ends_with('0') {
        f.pop();
    }
    format!("{int},{f}")
}

/// Дріб `n / d` для значень відповідей і помилок. Знаменник у наших викликах ніколи не нуль.
pub fn fr(n: i64, d: i64) -> Frac {
    Frac::new(n, d).expect("знаменник не нуль")
}

/// Складові задачі; `build` перетворює їх на `Task`. Операнди задано парами (значення, запис в умові).
pub struct Make {
    pub skill: &'static str,
    pub variant: u64,
    pub statement: String,
    pub operands: Vec<(Frac, String)>,
    pub params: Vec<i64>,
    pub answer: Frac,
    pub steps: Vec<Frac>,
    pub bugs: Vec<(&'static str, Frac)>,
}

/// Цілі операнди з їхнім записом.
pub fn ints(values: &[i64]) -> Vec<(Frac, String)> {
    values
        .iter()
        .map(|&n| (Frac::int(n), n.to_string()))
        .collect()
}

impl Make {
    pub fn build(self) -> Task {
        let (operands, operand_labels): (Vec<Frac>, Vec<String>) =
            self.operands.into_iter().unzip();
        Task {
            skill: self.skill,
            variant: self.variant,
            statement: self.statement,
            operands,
            operand_labels,
            params: self.params,
            answer: self.answer,
            steps: self.steps,
            bugs: self
                .bugs
                .into_iter()
                .map(|(id, value)| BugAnswer { id, value })
                .collect(),
        }
    }
}

/// Генератор, заданий набором функцій. `make` отримує генератор випадкових чисел і номер варіанта
/// та повертає задачу разом з оракулом (відповіддю, обчисленою іншим способом, ніж `Task::answer`).
pub struct Spec {
    pub id: &'static str,
    pub make: fn(&mut SeededRng, u64) -> Option<(Task, Frac)>,
    pub hint: fn(&Task, ScriptedLevel) -> String,
    /// Показані кроки аналога: тексти без чисел (окрім чисел умови самого аналога), щоб не було витоку.
    pub steps: fn(&Task) -> Vec<String>,
    pub prompt: &'static str,
}

impl SkillGenerator for Spec {
    fn skill_id(&self) -> &'static str {
        self.id
    }

    fn generate(&self, variant: u64) -> Task {
        build(self.id, variant, |rng| (self.make)(rng, variant))
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        (self.hint)(task, level)
    }

    fn figures(&self, task: &Task) -> Vec<tutor_core::figure::Figure> {
        crate::figures::for_task(task)
    }

    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample> {
        find_analog(self, task, avoid, |c| WorkedExample {
            task: c.clone(),
            shown_steps: (self.steps)(c),
            final_prompt: self.prompt.to_string(),
        })
    }
}

/// Вибір тексту за рівнем підказки.
pub fn by_level(level: ScriptedLevel, r1: String, r2: String, r3: String) -> String {
    match level {
        ScriptedLevel::R1 => r1,
        ScriptedLevel::R2 => r2,
        ScriptedLevel::R3 => r3,
    }
}

/// Випадковий елемент зрізу.
pub fn pick<'a, T>(rng: &mut SeededRng, items: &'a [T]) -> &'a T {
    &items[rng.range(0, items.len() as i64 - 1) as usize]
}

/// Найбільший спільний дільник (алгоритм Евкліда).
pub fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

/// Найменший простий дільник числа `n >= 2`.
pub fn smallest_prime_factor(n: i64) -> i64 {
    (2..=n).find(|p| n % p == 0).unwrap_or(n)
}

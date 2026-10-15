//! Уроки-пояснення: короткий текст з прикладами й питаннями для самоперевірки. Навчання, а не лише тести (рішення A10).
//!
//! Правила безпеки (перевіряються тестами курс-пакета): умови прикладів не збігаються з жодною задачею генератора
//! (учень не бачить розв'язку майбутньої задачі), а урок показується лише поза активною задачею.

/// Блок уроку. Усі тексти українською, нейтральні щодо роду.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LessonBlock {
    /// Пояснення: абзац тексту (ідея, правило, чому це так).
    Text(String),
    /// Розв'язаний приклад: умова в тому ж вигляді, що й у генератора («Обчисли: …»), кроки й результат.
    Example {
        statement: String,
        steps: Vec<String>,
        result: String,
    },
    /// Малюнок до пояснення (наочна модель) з підписом.
    Figure {
        figure: crate::figure::Figure,
        caption: String,
    },
    /// Самоперевірка: учень відповідає сам (`answer` розбирається як відповідь у задачах), потім бачить пояснення.
    SelfCheck {
        prompt: String,
        answer: String,
        explanation: String,
    },
    /// Самоперевірка з вибором (для предметів без обчислень): `correct` — номер правильного варіанта з нуля.
    Quiz {
        prompt: String,
        options: Vec<String>,
        correct: usize,
        explanation: String,
    },
    /// Коротко: що запам'ятати (основа для власного конспекту).
    Remember(String),
    /// Прогноз до пояснення: «Як ти думаєш?». Відповідь не оцінюється (лише активує попередні знання).
    Hook(String),
    /// Поняття з визначенням: збирається в словник конспекту.
    Term { term: String, definition: String },
    /// «Поясни своїми словами»: після відповіді учень звіряє її з ключовими ідеями й відмічає, які згадав.
    Explain {
        prompt: String,
        key_ideas: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lesson {
    pub skill: &'static str,
    pub title: String,
    pub blocks: Vec<LessonBlock>,
    /// Підказки для власного конспекту (учень пише своїми словами): запитання-каркас.
    pub note_prompts: Vec<String>,
}

/// Джерело уроків курс-пакета.
pub trait LessonSource {
    fn lesson(&self, skill: &str) -> Option<Lesson>;
}

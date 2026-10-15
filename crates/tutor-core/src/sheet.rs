//! Модель аркуша для вчителя (spec розділ 10). Типи навмисно не мають полів для вільного тексту дитини,
//! часових міток, діагнозів, оцінок чи порівнянь: цього в аркуші просто немає чим записати.
//! Ім'я дитина дописує від руки; тут лише нікнейм, який вона сама обрала.

/// Навичка з назвою (назва береться з курс-пакета).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillLine {
    pub skill: String,
    pub title: String,
}

/// Прогалина: навичка, гіпотези про помилки (нейтральні описи) й ідеї для вчителя.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    pub skill: String,
    pub title: String,
    pub hypotheses: Vec<String>,
    pub ideas: Vec<String>,
}

/// Задача, з якою потрібна людина: умова без розв'язку.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskAdultItem {
    pub skill: String,
    pub title: String,
    pub statement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeacherSheet {
    /// Предмет аркуша (порожній, якщо аркуш не прив'язаний до предмета).
    pub subject: String,
    pub nickname: String,
    /// Теми, які пора повторити (за розкладом повторень).
    pub to_repeat: Vec<SkillLine>,
    pub mastered: Vec<SkillLine>,
    pub in_progress: Vec<SkillLine>,
    pub gaps: Vec<Gap>,
    pub ask_adult: Vec<AskAdultItem>,
    pub stickers: usize,
    pub sticker_capacity: usize,
}

/// Тексти курс-пакета для аркуша: опис гіпотези про помилку й ідеї на 10 хвилин. Перегляд методистом обов'язковий.
pub trait SheetContent {
    /// Нейтральний опис типової помилки («Додає і чисельники, і знаменники»). `None`, якщо опису немає.
    fn describe_bug(&self, skill: &str, bug: &str) -> Option<String>;
    /// Ідея для вчителя на 10 хвилин під цю помилку.
    fn idea_for_bug(&self, skill: &str, bug: &str) -> Option<String>;
    /// Загальна ідея для навички, яка ще не засвоєна.
    fn idea_for_skill(&self, skill: &str) -> Option<String>;
}

/// Найбільша довжина нікнейма на аркуші (символів).
pub const NICKNAME_MAX_CHARS: usize = 24;

/// Нікнейм для аркуша: без керівних символів, без зайвих пробілів, не довший за ліміт.
pub fn sanitize_nickname(raw: &str) -> String {
    // Білий список: літери, цифри, пробіл, дефіс і апостроф. Керівні, невидимі й двоспрямовані символи
    // (U+202E, U+200B, U+FEFF тощо) та емодзі відсікаються: вони могли б зіпсувати друк аркуша.
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '\'' | '\u{2019}'))
        .collect();
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
        .chars()
        .take(NICKNAME_MAX_CHARS)
        .collect::<String>()
        .trim()
        .to_string()
}

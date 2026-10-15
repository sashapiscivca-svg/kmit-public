//! Каталог курсу: об'єднаний граф тем усіх предметів, генератори, уроки. Математика має обчислювальні генератори,
//! решта предметів банки завдань з вибором (крейт `subjects`).

use course_pack::{grade5_graph, SkillGraph, SkillNode};
use tutor_core::generator::SkillGenerator;
use tutor_core::lesson::{Lesson, LessonSource};

pub(crate) const MATH: &str = "math";
pub(crate) const MATH_TITLE: &str = "Математика";

pub(crate) fn generator_for(skill: &str) -> Option<Box<dyn SkillGenerator>> {
    generators_math::generator_for(skill).or_else(|| subjects::generator_for(skill))
}

/// Усі теми всіх предметів в одному графі (між предметами залежностей немає).
pub(crate) fn merged_graph() -> SkillGraph {
    let math = grade5_graph();
    let mut nodes: Vec<SkillNode> = math
        .ids()
        .into_iter()
        .filter_map(|id| math.get(id).cloned())
        .collect();
    for s in subjects::subjects() {
        for k in s.skills {
            nodes.push(SkillNode {
                id: k.id,
                title: k.title,
                prerequisites: k.prerequisites.to_vec(),
            });
        }
    }
    SkillGraph::new(nodes).expect("об'єднаний граф курсу коректний")
}

/// Дитяче пояснення типової помилки (лише для математики).
pub(crate) fn bug_message(skill: &str, bug: &str) -> Option<&'static str> {
    generators_math::bug_message(skill, bug)
}

pub(crate) fn subject_of(skill: &str) -> &'static str {
    subjects::subject_of(skill).unwrap_or(MATH)
}

/// Клас, до якого належить тема: математика поки лише 5 клас.
pub(crate) fn grade_of(skill: &str) -> u8 {
    subjects::grade_of(skill).unwrap_or(5)
}

/// Класи, для яких є вміст.
pub(crate) fn content_grades() -> Vec<u8> {
    let mut g = subjects::grades();
    if !g.contains(&5) {
        g.push(5);
        g.sort_unstable();
    }
    g
}

/// Предмет за замовчуванням для класу: перший у списку (математика в 5 класі).
pub(crate) fn default_subject(grade: u8) -> &'static str {
    subject_list(grade).first().map_or(MATH, |(id, _)| id)
}

/// Предмети класу в порядку показу: математика (лише 5 клас), далі решта.
pub(crate) fn subject_list(grade: u8) -> Vec<(&'static str, String)> {
    let mut v = Vec::new();
    if grade == 5 {
        v.push((MATH, MATH_TITLE.to_string()));
    }
    v.extend(
        subjects::subjects()
            .into_iter()
            .filter(|s| s.grade == grade)
            .map(|s| (s.id, s.title.to_string())),
    );
    v
}

/// Колір предмета (як обкладинка зошита в інтерфейсі).
pub(crate) fn subject_color(subject: &str) -> &'static str {
    match subject {
        "math" => "#2f5bea",
        "ukrainian" => "#f2b705",
        "english" => "#e0453a",
        "history" => "#c8742b",
        "nature" => "#2e9b5f",
        "informatics" => "#1b9aaa",
        "literature" => "#7b4fc9",
        "biology" => "#4c9a2a",
        "geography" => "#2a7fba",
        "physics" => "#5a4fcf",
        "chemistry" => "#c2410c",
        _ => "#2f5bea",
    }
}

/// Підграф одного предмета.
pub(crate) fn sub_graph(full: &SkillGraph, subject: &str, grade: u8) -> SkillGraph {
    let nodes: Vec<SkillNode> = full
        .ids()
        .into_iter()
        .filter(|id| subject_of(id) == subject && grade_of(id) == grade)
        .filter_map(|id| full.get(id).cloned())
        .collect();
    SkillGraph::new(nodes).unwrap_or_else(|_| full.clone())
}

/// Уроки всіх предметів.
pub(crate) struct Lessons;

impl LessonSource for Lessons {
    fn lesson(&self, skill: &str) -> Option<Lesson> {
        subjects::SubjectLessons
            .lesson(skill)
            .or_else(|| generators_math::MathLessons.lesson(skill))
    }
}

/// Світ істот класу: у кожного класу своя колекція. Нові класи додають рядок сюди й малюнок у `app/ui/worlds.js`
/// (`docs/worlds.md`). Усі класи 5–12 мають свій світ; для класів поза цим діапазоном (на майбутнє) світи повторюються по колу.
const WORLDS: [(u8, &str, &str); 8] = [
    (5, "forest", "Лісова галявина"),
    (6, "sea", "Море й берег"),
    (7, "sky", "Гори й небо"),
    (8, "cave", "Печери й кристали"),
    (9, "desert", "Пустеля й оазис"),
    (10, "space", "Космічна подорож"),
    (11, "ice", "Полярне сяйво"),
    (12, "summit", "Вершина"),
];

/// Ідентифікатор і назва світу істот для класу.
pub(crate) fn world_of(grade: u8) -> (&'static str, &'static str) {
    WORLDS
        .iter()
        .find(|w| w.0 == grade)
        .map(|w| (w.1, w.2))
        .unwrap_or_else(|| {
            let i = usize::from(grade.saturating_sub(5)) % WORLDS.len();
            (WORLDS[i].1, WORLDS[i].2)
        })
}

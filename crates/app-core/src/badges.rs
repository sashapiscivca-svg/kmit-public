//! Відзнаки за різні здібності (за зразком скаутських і пластових): за урок, конспект, розділ і курс.
//! Лише додаються, ніколи не відбираються; нічого не залежить від швидкості чи кількості спроб (рішення A9, Q3).

use crate::app::Env;
use crate::catalog;
use serde_json::{json, Value};
use storage::state::AppState;
use tutor_core::lesson::{LessonBlock, LessonSource};

pub(crate) struct Badge {
    pub id: String,
    pub group: &'static str,
    pub title: String,
    pub description: &'static str,
}

impl Badge {
    pub fn to_json(&self, earned: bool) -> Value {
        json!({"id": self.id, "group": self.group, "title": self.title,
               "description": self.description, "earned": earned})
    }
}

pub(crate) fn family_name(prefix: &str) -> &'static str {
    match prefix {
        "N" => "Натуральні числа",
        "D" => "Десяткові дроби",
        "F" => "Звичайні дроби",
        "EQ" => "Рівняння",
        "WORD" => "Текстові задачі",
        "U" => "Величини",
        "G" => "Геометрія",
        _ => "Розділ",
    }
}

pub(crate) fn selfcheck_count(env: &Env, skill: &str) -> usize {
    env.lessons.lesson(skill).map_or(0, |l| {
        l.blocks
            .iter()
            .filter(|b| {
                matches!(
                    b,
                    LessonBlock::SelfCheck { .. }
                        | LessonBlock::Quiz { .. }
                        | LessonBlock::Example { .. }
                        | LessonBlock::Explain { .. }
                )
            })
            .count()
    })
}

/// Увесь каталог відзнак у порядку курсу.
pub(crate) fn catalog(env: &Env) -> Vec<Badge> {
    let order = env.graph.topological_order().unwrap_or_default();
    let title = |id: &str| crate::views::skill_title(env, id);
    let mut out = Vec::new();
    for id in &order {
        if selfcheck_count(env, id) > 0 {
            out.push(Badge {
                id: format!("lesson:{id}"),
                group: "lesson",
                title: format!("Урок: {}", title(id)),
                description: "Усі питання для самоперевірки розв’язано правильно.",
            });
        }
        out.push(Badge {
            id: format!("note:{id}"),
            group: "note",
            title: format!("Конспект: {}", title(id)),
            description: "Записано власний конспект теми своїми словами.",
        });
    }
    // Розділи (N, D, F, EQ, WORD, U, G) є лише в математиці.
    let mut prefixes: Vec<&str> = order
        .iter()
        .filter(|s| catalog::subject_of(s) == catalog::MATH)
        .filter_map(|s| s.split('-').next())
        .collect();
    prefixes.dedup();
    let mut seen = Vec::new();
    let count_of = |p: &str| {
        order
            .iter()
            .filter(|s| catalog::subject_of(s) == catalog::MATH && s.split('-').next() == Some(p))
            .count()
    };
    for p in prefixes {
        // Розділ з однієї теми не дає окремої відзнаки: її вже дає опанування теми.
        if !seen.contains(&p) && count_of(p) >= 2 {
            seen.push(p);
            out.push(Badge {
                id: format!("family:{p}"),
                group: "family",
                title: format!("Розділ: {}", family_name(p)),
                description: "Опановано всі теми розділу.",
            });
        }
    }
    for grade in catalog::content_grades() {
        for (subject, title) in catalog::subject_list(grade) {
            let id = match (subject == catalog::MATH, grade) {
                (true, _) => "course:math5".to_string(),
                (false, 5) => format!("course:{subject}"),
                (false, g) => format!("course:{subject}:{g}"),
            };
            out.push(Badge {
                id,
                group: "course",
                title: if subject == catalog::MATH {
                    "Математика 5 клас: усі теми опановано".to_string()
                } else {
                    format!("{title} {grade} клас: усі теми опановано")
                },
                description: "Опановано кожну тему курсу.",
            });
        }
    }
    for n in [1u32, 5, 15] {
        out.push(Badge {
            id: format!("gap:{n}"),
            group: "gap",
            title: if n == 1 {
                "Подолано перший складний приклад".to_string()
            } else {
                format!("Подолано складних прикладів: {n}")
            },
            description: "Приклад, що був складним, повернувся за кілька днів і тепер виходить без підказок.",
        });
    }
    for n in [5u32, 15, 40] {
        out.push(Badge {
            id: format!("recall:{n}"),
            group: "recall",
            title: format!("Пригадано {n} разів"),
            description: "Спершу пояснення своїми словами, потім звірка з ідеями: так знання тримаються довго.",
        });
    }
    out
}

fn earned_now(env: &Env, st: &AppState, id: &str) -> bool {
    let Some((kind, rest)) = id.split_once(':') else {
        return false;
    };
    let mastered = |s: &str| st.learner.is_mastered(s);
    match kind {
        "lesson" => env
            .lessons
            .lesson(rest)
            .is_some_and(|l| crate::lesson_prog::is_complete(&l, rest, st)),
        "note" => {
            let lesson_ok = env
                .lessons
                .lesson(rest)
                .map_or(true, |l| crate::lesson_prog::is_complete(&l, rest, st));
            lesson_ok
                && st
                    .notes
                    .get(rest)
                    .is_some_and(|t| crate::lesson_prog::note_is_real(t))
        }
        "family" => {
            let ids: Vec<&str> = env
                .graph
                .ids()
                .into_iter()
                .filter(|s| {
                    catalog::subject_of(s) == catalog::MATH && s.split('-').next() == Some(rest)
                })
                .collect();
            !ids.is_empty() && ids.iter().all(|s| mastered(s))
        }
        "course" => {
            let (name, grade) = match rest.split_once(':') {
                Some((n, g)) => (n, g.parse::<u8>().unwrap_or(5)),
                None => (rest, 5),
            };
            let subject = if name == "math5" { catalog::MATH } else { name };
            let ids: Vec<&str> = env
                .graph
                .ids()
                .into_iter()
                .filter(|s| catalog::subject_of(s) == subject && catalog::grade_of(s) == grade)
                .collect();
            !ids.is_empty() && ids.iter().all(|s| mastered(s))
        }
        "gap" => rest.parse::<u32>().is_ok_and(|n| st.gaps_closed_total >= n),
        "recall" => rest
            .parse::<u32>()
            .is_ok_and(|n| crate::recall::recall_count(st) >= n),
        _ => false,
    }
}

/// Видає нові відзнаки, яких ще немає, і повертає їх. Раз видану відзнаку не відбирають.
pub(crate) fn award_new(env: &Env, st: &mut AppState) -> Vec<Badge> {
    let mut fresh = Vec::new();
    for b in catalog(env) {
        if !st.badges.contains(&b.id) && earned_now(env, st, &b.id) {
            st.badges.insert(b.id.clone());
            fresh.push(b);
        }
    }
    fresh
}

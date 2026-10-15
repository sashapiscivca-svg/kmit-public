//! Бібліотека уроків для вчителя: пошук, закладки, кабінет, план і пам'ятка.

use super::work::*;
use crate::catalog;
use crate::error::ApiError;
use serde_json::{json, Value};

/// Пошук по всіх уроках (назви тем, поняття, абзаци пояснення): вчитель знаходить, де про щось сказано.
pub(crate) fn search_json(env: &crate::app::Env, query: &str) -> Value {
    use tutor_core::lesson::{LessonBlock, LessonSource};
    let q: String = query
        .trim()
        .chars()
        .take(60)
        .collect::<String>()
        .to_lowercase();
    if q.chars().count() < 2 {
        return json!({"hits": [], "query": q});
    }
    let mut hits: Vec<Value> = Vec::new();
    for id in env.graph.topological_order().unwrap_or_default() {
        if hits.len() >= 40 {
            break;
        }
        let title = crate::views::skill_title(env, id);
        let mut snippet: Option<String> = None;
        let by_title = title.to_lowercase().contains(&q);
        if let Some(l) = env.lessons.lesson(id) {
            for b in &l.blocks {
                let text: &str = match b {
                    LessonBlock::Text(t) | LessonBlock::Remember(t) | LessonBlock::Hook(t) => t,
                    LessonBlock::Term { definition, .. } => definition,
                    LessonBlock::Example { statement, .. } => statement,
                    _ => continue,
                };
                let low = text.to_lowercase();
                if let Some(pos) = low.find(&q) {
                    // Фрагмент навколо збігу (за символами, щоб не різати літеру навпіл).
                    let chars: Vec<char> = text.chars().collect();
                    let at = low[..pos].chars().count();
                    let from = at.saturating_sub(50);
                    let to = (at + 90).min(chars.len());
                    snippet = Some(chars[from..to].iter().collect::<String>());
                    break;
                }
            }
        }
        if by_title || snippet.is_some() {
            hits.push(json!({
                "id": id, "title": title, "grade": catalog::grade_of(id),
                "subject": catalog::subject_of(id), "snippet": snippet,
            }));
        }
    }
    json!({"hits": hits, "query": q})
}

/// Увімкнути/вимкнути закладку на тему.
pub(crate) fn bookmark_set(
    state: &mut storage::state::AppState,
    skill: &str,
    on: bool,
) -> Result<Value, ApiError> {
    let b = &mut state.teacher.bookmarks;
    b.retain(|x| x != skill);
    if on {
        if b.len() >= storage::state::MAX_BOOKMARKS {
            return Err(ApiError::bad_request("Забагато закладок: прибери зайві"));
        }
        b.insert(0, skill.to_string());
    }
    Ok(json!({"ok": true, "bookmarks": b.len()}))
}

/// Головна кабінету: ім'я й останні створені роботи (без жодних даних учнів).
pub(crate) fn home_json(
    env: &crate::app::Env,
    state: &storage::state::AppState,
    day: u32,
) -> Value {
    let bookmarks: Vec<Value> = state
        .teacher
        .bookmarks
        .iter()
        .filter(|id| env.graph.get(id).is_some())
        .map(|id| json!({"id": id, "title": crate::views::skill_title(env, id), "grade": catalog::grade_of(id)}))
        .collect();
    let works: Vec<Value> = state
        .teacher
        .works
        .iter()
        .rev()
        .take(5)
        .map(|w| {
            json!({
                "day": w.day, "grade": w.grade, "kind": w.kind, "skills": w.skills,
                "variants": w.variants, "seed": w.seed,
            })
        })
        .collect();
    json!({"nickname": state.profile.nickname, "works_total": state.teacher.works.len(), "works": works, "bookmarks": bookmarks,
           "celebration": crate::teacher::celebration_pending(state, day)})
}

/// План уроку на 45 хвилин із блоків уроку теми та кількох завдань на закріплення.
pub(crate) fn lesson_plan_doc(
    env: &crate::app::Env,
    skill: &str,
    grade: u8,
    seed: u64,
) -> Result<report::work::PlanDoc, ApiError> {
    use tutor_core::lesson::{LessonBlock, LessonSource};
    let l = env
        .lessons
        .lesson(skill)
        .ok_or_else(|| ApiError::bad_request("Для цієї теми ще немає уроку"))?;
    let (mut intro, mut explain, mut examples, mut checks, mut wrap) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for b in &l.blocks {
        match b {
            LessonBlock::Hook(t) => intro.push(format!("• Запитання класу: {t}")),
            LessonBlock::Text(t) => explain.push(t.clone()),
            LessonBlock::Term { term, definition } => {
                explain.push(format!("• Поняття «{term}»: {definition}"));
            }
            LessonBlock::Figure { caption, .. } => {
                explain.push(format!("• Схема на дошці: {caption}"));
            }
            LessonBlock::Example {
                statement,
                steps,
                result,
            } => {
                examples.push(format!("Приклад: {statement}"));
                for (i, st) in steps.iter().enumerate() {
                    examples.push(format!("• Крок {}: {st}", i + 1));
                }
                examples.push(format!("Результат: {result}"));
            }
            LessonBlock::SelfCheck { prompt, answer, .. } => {
                checks.push(format!("• {prompt} Відповідь: {answer}"));
            }
            LessonBlock::Quiz {
                prompt,
                options,
                correct,
                ..
            } => {
                checks.push(format!(
                    "• {prompt} Варіанти: {}. Правильно: {}",
                    options.join("; "),
                    options[*correct]
                ));
            }
            LessonBlock::Explain { prompt, key_ideas } => {
                wrap.push(format!("{prompt} Ключові ідеї для звірки:"));
                for k in key_ideas {
                    wrap.push(format!("• {k}"));
                }
            }
            LessonBlock::Remember(t) => wrap.push(format!("Запам'ятай: {t}")),
        }
    }
    if !l.note_prompts.is_empty() {
        wrap.push("Запитання для конспекту дитини:".to_string());
        for q in &l.note_prompts {
            wrap.push(format!("• {q}"));
        }
    }
    let mut sections = Vec::new();
    for (h, lines) in [
        ("Вступ, 5 хв", intro),
        ("Пояснення, 15 хв", explain),
        ("Розібрані приклади, 10 хв", examples),
        ("Перевірка розуміння, 10 хв", checks),
        ("Підсумок, 5 хв", wrap),
    ] {
        if !lines.is_empty() {
            sections.push(report::work::PlanSection {
                heading: h.to_string(),
                lines,
            });
        }
    }
    // Кілька завдань на закріплення з банку чи генератора теми (один варіант, з ключем).
    let spec = WorkSpec {
        level: Level::Any,
        large: false,
        grade,
        skills: vec![skill.to_string()],
        kind: WorkKind::Quiz,
        variants: 1,
        seed,
    };
    let practice = build_work(env, &spec)
        .map(|w| {
            w.variants[0]
                .items
                .iter()
                .map(|i| (i.statement.clone(), i.answer.clone()))
                .collect()
        })
        .unwrap_or_default();
    Ok(report::work::PlanDoc {
        title: l.title.clone(),
        subtitle: format!("{} клас · орієнтовно 45 хвилин", grade),
        sections,
        practice,
    })
}

/// Пам'ятка для учнів (одна сторінка для друку): поняття, головне правило й запитання для конспекту.
/// Без правильних відповідей до контрольних питань і без жодних даних учнів.
pub(crate) fn handout_doc(
    env: &crate::app::Env,
    skill: &str,
    grade: u8,
) -> Result<report::work::PlanDoc, ApiError> {
    use tutor_core::lesson::{LessonBlock, LessonSource};
    let l = env
        .lessons
        .lesson(skill)
        .ok_or_else(|| ApiError::bad_request("Для цієї теми ще немає уроку"))?;
    let (mut terms, mut remember) = (Vec::new(), Vec::new());
    for b in &l.blocks {
        match b {
            LessonBlock::Term { term, definition } => terms.push(format!("• {term}: {definition}")),
            LessonBlock::Remember(t) => remember.push(t.clone()),
            _ => {}
        }
    }
    let mut sections = Vec::new();
    if !remember.is_empty() {
        sections.push(report::work::PlanSection {
            heading: "Запам'ятай".to_string(),
            lines: remember,
        });
    }
    if !terms.is_empty() {
        sections.push(report::work::PlanSection {
            heading: "Поняття".to_string(),
            lines: terms,
        });
    }
    if !l.note_prompts.is_empty() {
        sections.push(report::work::PlanSection {
            heading: "Запиши своїми словами".to_string(),
            lines: l.note_prompts.iter().map(|q| format!("• {q}")).collect(),
        });
    }
    if sections.is_empty() {
        return Err(ApiError::bad_request(
            "У цій темі немає чого друкувати в пам'ятці",
        ));
    }
    Ok(report::work::PlanDoc {
        title: l.title.clone(),
        subtitle: format!("{grade} клас · пам'ятка для учнів"),
        sections,
        practice: Vec::new(),
    })
}

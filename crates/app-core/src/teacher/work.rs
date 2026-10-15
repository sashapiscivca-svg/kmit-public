//! Роботи для друку: детерміновані варіанти з ключем, теми, тематичний план.

use crate::catalog;
use crate::error::ApiError;
use serde_json::{json, Value};
use tutor_core::generator::AnswerKind;
use tutor_core::rng::{seed_for, SeededRng};

// ---------------------------------------------------------------------------------------------------------------
// Роботи: детерміновані варіанти з ключем. Лише для вчительського профілю; даних учнів тут немає.
// ---------------------------------------------------------------------------------------------------------------

/// Вид роботи й кількість завдань у варіанті.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkKind {
    Quiz,
    Test,
    Homework,
    Nmt,
    /// Завдання з введенням слова (пропуски).
    Gaps,
    /// Словниковий диктант: умови читає вчитель.
    Dictation,
}

impl WorkKind {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "quiz" => Some(WorkKind::Quiz),
            "test" => Some(WorkKind::Test),
            "homework" => Some(WorkKind::Homework),
            "nmt" => Some(WorkKind::Nmt),
            "gaps" => Some(WorkKind::Gaps),
            "dictation" => Some(WorkKind::Dictation),
            _ => None,
        }
    }

    pub(crate) fn code(self) -> &'static str {
        match self {
            WorkKind::Quiz => "quiz",
            WorkKind::Test => "test",
            WorkKind::Homework => "homework",
            WorkKind::Nmt => "nmt",
            WorkKind::Gaps => "gaps",
            WorkKind::Dictation => "dictation",
        }
    }

    fn text_only(self) -> bool {
        matches!(self, WorkKind::Gaps | WorkKind::Dictation)
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            WorkKind::Quiz => "Самостійна робота",
            WorkKind::Test => "Контрольна робота",
            WorkKind::Homework => "Домашня робота",
            WorkKind::Nmt => "Тренування у форматі НМТ",
            WorkKind::Gaps => "Завдання з пропусками",
            WorkKind::Dictation => "Словниковий диктант",
        }
    }

    fn per_variant(self) -> usize {
        match self {
            WorkKind::Quiz => 5,
            WorkKind::Test => 10,
            WorkKind::Homework => 8,
            WorkKind::Nmt => 12,
            WorkKind::Gaps => 8,
            WorkKind::Dictation => 10,
        }
    }
}

/// Орієнтовна складність завдання за формою умови (позначок складності в банках поки немає).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Any,
    Recall,
    Apply,
    Nmt,
}

impl Level {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "" | "any" => Some(Level::Any),
            "recall" => Some(Level::Recall),
            "apply" => Some(Level::Apply),
            "nmt" => Some(Level::Nmt),
            _ => None,
        }
    }
}

/// Рівень за формою: короткі прямі запитання це пригадування, довгі умови й формулювання в стилі НМТ вищі.
pub(crate) fn level_of(statement: &str) -> Level {
    let s = statement.to_lowercase();
    let len = statement.chars().count();
    let nmt = [
        "укажіть",
        "укажи рядок",
        "яке твердження",
        "не є ",
        "не належить",
        "послідовн",
        "у якому рядку",
        "установи відповідність",
        "що не ",
        "причин",
        "наслідк",
    ];
    if len > 140 || nmt.iter().any(|m| s.contains(m)) {
        Level::Nmt
    } else if len <= 70 {
        Level::Recall
    } else {
        Level::Apply
    }
}

pub(crate) struct WorkSpec {
    pub level: Level,
    pub large: bool,
    pub grade: u8,
    pub skills: Vec<String>,
    pub kind: WorkKind,
    pub variants: u8,
    pub seed: u64,
}

pub(crate) struct WorkItem {
    pub skill: String,
    pub skill_title: String,
    pub statement: String,
    pub choices: Vec<String>,
    pub answer: String,
}

pub(crate) struct Variant {
    pub label: char,
    pub items: Vec<WorkItem>,
}

pub(crate) struct Work {
    pub title: String,
    pub variants: Vec<Variant>,
}

pub(crate) const MAX_VARIANTS: u8 = 4;
pub(crate) const MAX_SKILLS: usize = 8;

/// Відповідь числової задачі так, як її записує учень: десятковий дріб з комою, якщо умова з комою.
fn number_key(task: &tutor_core::task::Task) -> String {
    let decimal_in_statement = task.operand_labels.iter().any(|l| l.contains(','));
    let a = task.answer;
    if decimal_in_statement && a.den() != 1 {
        // Десятковий запис можливий, лише якщо знаменник має дільники 2 і 5.
        let mut d = a.den();
        let mut places = 0u32;
        while d % 10 == 0 {
            d /= 10;
            places += 1;
        }
        let (mut twos, mut fives) = (0u32, 0u32);
        while d % 2 == 0 {
            d /= 2;
            twos += 1;
        }
        while d % 5 == 0 {
            d /= 5;
            fives += 1;
        }
        if d == 1 {
            let scale = places + twos.max(fives);
            let den = 10i128.pow(scale);
            let num = i128::from(a.num()) * den / i128::from(a.den());
            let neg = num < 0;
            let abs = num.abs();
            let int = abs / den;
            let frac = abs % den;
            let digits = format!("{frac:0>width$}", width = scale as usize);
            let digits = digits.trim_end_matches('0');
            let sign = if neg { "-" } else { "" };
            return if digits.is_empty() {
                format!("{sign}{int}")
            } else {
                format!("{sign}{int},{digits}")
            };
        }
    }
    a.to_string()
}

/// Будує роботу. Помилка (а не порожня робота), якщо теми немає, вона з іншого класу чи завдань замало.
pub(crate) fn build_work(env: &crate::app::Env, spec: &WorkSpec) -> Result<Work, ApiError> {
    if spec.skills.is_empty() || spec.skills.len() > MAX_SKILLS {
        return Err(ApiError::bad_request("Обери від 1 до 8 тем"));
    }
    if spec.variants == 0 || spec.variants > MAX_VARIANTS {
        return Err(ApiError::bad_request("Варіантів може бути від 1 до 4"));
    }
    let mut gens = Vec::new();
    for s in &spec.skills {
        let g = catalog::generator_for(s)
            .filter(|_| (5..=spec.grade).contains(&catalog::grade_of(s)))
            .ok_or_else(|| ApiError::bad_request("Невідома тема для цього класу"))?;
        gens.push((s.clone(), g));
    }
    if spec.kind.text_only()
        && gens
            .iter()
            .any(|(_, g)| g.answer_kind() != AnswerKind::Text)
    {
        return Err(ApiError::bad_request(
            "Для цього виду роботи потрібні теми із введенням слова (наприклад, словник, множина, апостроф)",
        ));
    }
    let total = spec.kind.per_variant();
    let skills = gens.len();
    if skills > total {
        return Err(ApiError::bad_request(
            "Тем більше, ніж завдань у цьому виді роботи: обери менше тем або інший вид",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut variants = Vec::new();
    for v in 0..spec.variants {
        let mut items = Vec::new();
        for (si, (skill, gen)) in gens.iter().enumerate() {
            let want = total / skills + usize::from(si < total % skills);
            let mut rng = SeededRng::new(seed_for(skill, spec.seed ^ (u64::from(v) << 56)));
            let mut got = 0usize;
            let mut attempts = 0;
            while got < want {
                attempts += 1;
                if attempts > 600 {
                    return Err(ApiError::bad_request(
                        "У темі замало різних завдань для такої роботи: зменш кількість варіантів чи обери більше тем",
                    ));
                }
                let n = u64::try_from(rng.range(1, 1_000_000)).unwrap_or(1);
                let t = gen.generate(n);
                // Малюнки до умови (смужки, кути) у роботі не відтворюємо.
                if !gen.figures(&t).is_empty() {
                    continue;
                }
                if spec.level != Level::Any
                    && gen.answer_kind() == AnswerKind::Choice
                    && level_of(&t.statement) != spec.level
                {
                    continue;
                }
                if !seen.insert(t.statement.trim().to_lowercase()) {
                    continue;
                }
                let (choices, answer) = match gen.answer_kind() {
                    AnswerKind::Number => (Vec::new(), number_key(&t)),
                    AnswerKind::Choice => {
                        let r = gen.reveal(&t).ok_or_else(|| {
                            ApiError::new("content", "Тема без відповіді для ключа")
                        })?;
                        (gen.choices(&t), r.answer_text)
                    }
                    AnswerKind::Text => {
                        let r = gen.reveal(&t).ok_or_else(|| {
                            ApiError::new("content", "Тема без відповіді для ключа")
                        })?;
                        (Vec::new(), r.answer_text)
                    }
                };
                items.push(WorkItem {
                    skill: skill.clone(),
                    skill_title: crate::views::skill_title(env, skill),
                    statement: t.statement.clone(),
                    choices,
                    answer,
                });
                got += 1;
            }
        }
        variants.push(Variant { label: 'А', items });
    }
    // Літери варіантів: А, Б, В, Г.
    for (i, v) in variants.iter_mut().enumerate() {
        v.label = ['А', 'Б', 'В', 'Г'][i];
    }
    Ok(Work {
        title: spec.kind.title().to_string(),
        variants,
    })
}

pub(crate) fn work_json(w: &Work) -> Value {
    json!({
        "title": w.title,
        "variants": w.variants.iter().map(|v| json!({
            "label": v.label.to_string(),
            "items": v.items.iter().map(|i| json!({
                "skill": i.skill, "skill_title": i.skill_title, "statement": i.statement,
                "choices": i.choices, "answer": i.answer,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// Розбір аргументів `teacher_work_*`.
pub(crate) fn spec_from(args: &Value, today: u32) -> Result<WorkSpec, ApiError> {
    let grade = args
        .get("grade")
        .and_then(Value::as_u64)
        .and_then(|g| u8::try_from(g).ok())
        .ok_or_else(|| ApiError::bad_request("Не вказано клас"))?;
    let skills: Vec<String> = args
        .get("skills")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let kind = args
        .get("kind")
        .and_then(Value::as_str)
        .and_then(WorkKind::parse)
        .ok_or_else(|| ApiError::bad_request("Невідомий вид роботи"))?;
    let variants = args
        .get("variants")
        .and_then(Value::as_u64)
        .map_or(Ok(1), u8::try_from)
        .map_err(|_| ApiError::bad_request("Варіантів може бути від 1 до 4"))?;
    // Без явного зерна беремо день: той самий день дає ту саму роботу, інший день іншу.
    let seed = args
        .get("seed")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| u64::from(today) * 7919 + 13);
    let level = args
        .get("level")
        .and_then(Value::as_str)
        .map_or(Some(Level::Any), Level::parse)
        .ok_or_else(|| ApiError::bad_request("Невідомий рівень складності"))?;
    let flag = |k: &str| args.get(k).and_then(Value::as_bool).unwrap_or(false);
    Ok(WorkSpec {
        level,
        large: flag("large"),
        grade,
        skills,
        kind,
        variants,
        seed,
    })
}

/// Теми класу, з яких можна скласти роботу: за предметами, у порядку курсу.
fn subjects_of_grade(env: &crate::app::Env, grade: u8) -> Vec<Value> {
    catalog::subject_list(grade)
        .into_iter()
        .map(|(sid, title)| {
            let sg = catalog::sub_graph(&env.graph, sid, grade);
            let skills: Vec<Value> = sg
                .topological_order()
                .unwrap_or_default()
                .into_iter()
                .filter(|id| catalog::generator_for(id).is_some())
                .map(|id| json!({"id": id, "title": crate::views::skill_title(env, id)}))
                .collect();
            json!({"id": sid, "title": title, "skills": skills})
        })
        .filter(|s| !s["skills"].as_array().is_some_and(Vec::is_empty))
        .collect()
}

pub(crate) fn topics_json(env: &crate::app::Env, grade: u8) -> Value {
    let grades = catalog::content_grades();
    let grade = if grades.contains(&grade) { grade } else { 5 };
    let earlier: Vec<Value> = grades
        .iter()
        .copied()
        .filter(|g| *g < grade)
        .map(|g| json!({"grade": g, "subjects": subjects_of_grade(env, g)}))
        .collect();
    json!({"grade": grade, "grades": grades, "subjects": subjects_of_grade(env, grade), "earlier": earlier})
}

/// Тематичний план предмета класу: теми в порядку курсу з кількістю уроків (орієнтовно) для друку.
pub(crate) fn topic_plan_doc(
    env: &crate::app::Env,
    grade: u8,
    subject: &str,
    lessons: u8,
) -> Result<report::work::TopicPlanDoc, ApiError> {
    let (sid, title) = catalog::subject_list(grade)
        .into_iter()
        .find(|(id, _)| *id == subject)
        .ok_or_else(|| ApiError::bad_request("Невідомий предмет цього класу"))?;
    let sg = catalog::sub_graph(&env.graph, sid, grade);
    let rows: Vec<report::work::TopicRow> = sg
        .topological_order()
        .unwrap_or_default()
        .into_iter()
        .map(|id| report::work::TopicRow {
            title: crate::views::skill_title(env, id),
            lessons,
        })
        .collect();
    if rows.is_empty() {
        return Err(ApiError::bad_request("У цьому предметі ще немає тем"));
    }
    Ok(report::work::TopicPlanDoc {
        title: format!("Тематичний план: {title}"),
        subtitle: format!(
            "{grade} клас · порядок тем курсу Kmit; уроків на тему: орієнтовно {lessons}"
        ),
        rows,
    })
}

/// Перетворює роботу на документ для друку.
pub(crate) fn work_doc(spec: &WorkSpec, w: &Work) -> report::work::WorkDoc {
    let mut titles: Vec<String> = Vec::new();
    for v in &w.variants {
        for i in &v.items {
            if !titles.contains(&i.skill_title) {
                titles.push(i.skill_title.clone());
            }
        }
    }
    report::work::WorkDoc {
        scale: if spec.large { 1.4 } else { 1.0 },
        dictation: spec.kind == WorkKind::Dictation,
        title: w.title.clone(),
        subtitle: titles.join(", "),
        grade: spec.grade,
        seed: spec.seed,
        variants: w
            .variants
            .iter()
            .map(|v| report::work::VariantDoc {
                label: v.label,
                items: v
                    .items
                    .iter()
                    .map(|i| report::work::ItemDoc {
                        text: i.statement.clone(),
                        choices: i.choices.clone(),
                        answer: i.answer.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tutor_core::frac::parse_answer;

    /// Ключ має читатися назад у те саме число, яке вважає правильним застосунок.
    #[test]
    fn number_keys_parse_back_to_the_task_answer() {
        for skill in [
            "D-ADD",
            "D-SUB",
            "F-ADD-SAME",
            "F-SUB-DIFF",
            "N-ADD-MULTI",
            "P-PCT",
            "N-DIVREM",
        ] {
            let Some(g) = catalog::generator_for(skill) else {
                continue;
            };
            if g.answer_kind() != AnswerKind::Number {
                continue;
            }
            for v in 1..=300u64 {
                let t = g.generate(v);
                let key = number_key(&t);
                assert_eq!(
                    parse_answer(&key),
                    Some(t.answer),
                    "{skill} #{v}: ключ «{key}»"
                );
            }
        }
    }
}

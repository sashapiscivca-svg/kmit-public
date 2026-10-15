//! Уроки: покриття навичок, структура, правильність математики й безпека умов щодо генераторів.

mod lessons_solver;

use generators_math::generator_for;
use std::collections::BTreeSet;
use subjects::SubjectLessons as MathLessons;
use tutor_core::frac::{parse_answer, Frac};
use tutor_core::lesson::{Lesson, LessonBlock, LessonSource};

const SKILLS: [&str; 11] = [
    "N-ADD-MULTI",
    "N-SUB-MULTI",
    "N-MUL-MULTI",
    "N-DIV-MULTI",
    "N-ORDER",
    "D-ADD",
    "D-SUB",
    "F-ADD-SAME",
    "F-ADD-DIFF",
    "F-SUB-SAME",
    "F-SUB-DIFF",
];

fn all_lessons() -> Vec<Lesson> {
    course_pack::grade5_graph()
        .ids()
        .into_iter()
        .map(|id| MathLessons.lesson(id).expect("урок є"))
        .collect()
}

/// Незалежне обчислення виразу «Обчисли: a ∘ b [∘ c]»: спочатку × і :, потім + і -.
fn eval(statement: &str) -> Frac {
    let body = statement
        .strip_prefix("Обчисли: ")
        .expect("умова починається з «Обчисли: »");
    let tokens: Vec<&str> = body.split(' ').collect();
    assert!(tokens.len() % 2 == 1, "хибна умова: {statement}");
    let mut terms: Vec<Frac> = vec![parse_answer(tokens[0]).expect("операнд")];
    let mut signs: Vec<&str> = Vec::new();
    for pair in tokens[1..].chunks(2) {
        let rhs = parse_answer(pair[1]).expect("операнд");
        match pair[0] {
            "×" => {
                let last = terms.pop().unwrap();
                terms.push(last.checked_mul(rhs).unwrap());
            }
            ":" => {
                let last = terms.pop().unwrap();
                terms.push(last.checked_div(rhs).unwrap());
            }
            op @ ("+" | "-") => {
                signs.push(op);
                terms.push(rhs);
            }
            other => panic!("невідома дія {other}"),
        }
    }
    let mut acc = terms[0];
    for (sign, t) in signs.iter().zip(&terms[1..]) {
        acc = if *sign == "+" {
            acc.checked_add(*t).unwrap()
        } else {
            acc.checked_sub(*t).unwrap()
        };
    }
    acc
}

fn texts(lesson: &Lesson) -> Vec<&str> {
    let mut v: Vec<&str> = vec![lesson.title.as_str()];
    for b in &lesson.blocks {
        match b {
            LessonBlock::Text(t) | LessonBlock::Remember(t) | LessonBlock::Hook(t) => v.push(t),
            LessonBlock::Term { term, definition } => {
                v.push(term);
                v.push(definition);
            }
            LessonBlock::Explain { prompt, key_ideas } => {
                v.push(prompt);
                v.extend(key_ideas.iter().map(String::as_str));
            }
            LessonBlock::Example {
                statement,
                steps,
                result,
            } => {
                v.push(statement);
                v.extend(steps.iter().map(String::as_str));
                v.push(result);
            }
            LessonBlock::SelfCheck {
                prompt,
                answer,
                explanation,
            } => {
                v.push(prompt);
                v.push(answer);
                v.push(explanation);
            }
            LessonBlock::Figure { caption, .. } => v.push(caption),
            LessonBlock::Quiz {
                prompt,
                options,
                explanation,
                ..
            } => {
                v.push(prompt);
                v.extend(options.iter().map(String::as_str));
                v.push(explanation);
            }
        }
    }
    v.extend(lesson.note_prompts.iter().map(String::as_str));
    v
}

#[test]
fn lesson_exists_for_every_graph_skill_and_no_foreign() {
    let ids = course_pack::grade5_graph().ids();
    assert_eq!(ids.len(), SKILLS.len() + lessons_solver::NEW_SKILLS.len());
    for s in lessons_solver::NEW_SKILLS {
        assert!(ids.contains(&s), "{s} немає в графі");
    }
    for id in &ids {
        let l = MathLessons
            .lesson(id)
            .unwrap_or_else(|| panic!("немає уроку {id}"));
        assert_eq!(l.skill, *id);
    }
    for s in SKILLS {
        assert!(ids.contains(&s), "{s} немає в графі");
    }
    assert!(MathLessons.lesson("X-FOREIGN").is_none());
    assert!(MathLessons.lesson("").is_none());
}

#[test]
fn lesson_structure() {
    for l in all_lessons() {
        let n = |f: fn(&LessonBlock) -> bool| l.blocks.iter().filter(|b| f(b)).count();
        assert!(n(|b| matches!(b, LessonBlock::Text(_))) >= 1, "{}", l.skill);
        assert!(
            n(|b| matches!(b, LessonBlock::Example { .. })) >= 2,
            "{}",
            l.skill
        );
        assert!(
            n(|b| matches!(b, LessonBlock::SelfCheck { .. })) >= 2,
            "{}",
            l.skill
        );
        assert!(
            n(|b| matches!(b, LessonBlock::Remember(_))) >= 1,
            "{}",
            l.skill
        );
        assert!(l.note_prompts.len() >= 3, "{}", l.skill);
        for b in &l.blocks {
            if let LessonBlock::Example { steps, .. } = b {
                assert!(steps.len() >= 3, "{}: замало кроків", l.skill);
            }
        }
    }
}

#[test]
fn examples_and_checks_are_mathematically_correct() {
    for l in all_lessons() {
        for b in &l.blocks {
            match b {
                LessonBlock::Example {
                    statement, result, ..
                } => {
                    let want = parse_answer(result).expect("результат розбирається");
                    let got = if lessons_solver::NEW_SKILLS.contains(&l.skill) {
                        lessons_solver::solve(l.skill, statement).unwrap_or_else(|| {
                            panic!("{}: умову не розпізнано: {statement}", l.skill)
                        })
                    } else {
                        eval(statement)
                    };
                    assert_eq!(got, want, "{}: {statement} = {result}", l.skill);
                }
                LessonBlock::SelfCheck { prompt, answer, .. } => {
                    let want = parse_answer(answer).expect("відповідь розбирається");
                    if lessons_solver::NEW_SKILLS.contains(&l.skill) {
                        let got = lessons_solver::solve(l.skill, prompt).unwrap_or_else(|| {
                            panic!("{}: умову не розпізнано: {prompt}", l.skill)
                        });
                        assert_eq!(got, want, "{}: {prompt}", l.skill);
                    } else if prompt.starts_with("Обчисли: ") {
                        assert_eq!(eval(prompt), want, "{}: {prompt}", l.skill);
                    } else {
                        assert_eq!(independent_word_answer(prompt), want, "{prompt}");
                    }
                }
                _ => {}
            }
        }
    }
}

/// Незалежні обчислення для словесних питань самоперевірки (за ключовою фразою).
fn independent_word_answer(prompt: &str) -> Frac {
    let int = Frac::int;
    let frac = |n, d| Frac::new(n, d).unwrap();
    if prompt.starts_with("Цифри одиниць двох доданків 8 і 7") {
        return int((8 + 7) / 10);
    }
    if prompt.starts_with("Різниця дорівнює 38") {
        return int(38 + 27);
    }
    if prompt.starts_with("Скільки буде 40 × 30") {
        return int(40 * 30);
    }
    if prompt.starts_with("Відомо, що 15 × 6 = 90") {
        return int(90 / 6);
    }
    if prompt.starts_with("Оля порахувала 5 + 2 × 3") {
        return int(5 + 2 * 3);
    }
    if prompt.starts_with("Скільки десятих міститься в числі 1,3") {
        return int(13);
    }
    if prompt.starts_with("Скільки десятих міститься в числі 3?") {
        return int(3 * 10);
    }
    if prompt.starts_with("Ділимо 812 на 4") {
        return int(812 / 4);
    }
    if prompt.starts_with("Піцу розрізали на 8 рівних шматків. Спершу з'їли 3")
    {
        return frac(3 + 2, 8);
    }
    if prompt.starts_with("Що більше: 1/3 чи 1/4") {
        return frac(1, 3).max(frac(1, 4));
    }
    if prompt.starts_with("Піцу розрізали на 8 рівних шматків. Залишилось 7")
    {
        return frac(7 - 3, 8);
    }
    if prompt.starts_with("Який найменший спільний знаменник") {
        return int((1..).find(|n| n % 4 == 0 && n % 6 == 0).unwrap());
    }
    panic!("немає незалежної перевірки для: {prompt}");
}

#[test]
fn lesson_statements_never_match_generated_tasks() {
    for l in all_lessons() {
        let mut shown: BTreeSet<&str> = BTreeSet::new();
        for b in &l.blocks {
            match b {
                LessonBlock::Example { statement, .. } => {
                    shown.insert(statement);
                }
                LessonBlock::SelfCheck { prompt, .. }
                    if prompt.starts_with("Обчисли: ")
                        || lessons_solver::NEW_SKILLS.contains(&l.skill) =>
                {
                    shown.insert(prompt);
                }
                _ => {}
            }
        }
        let g = generator_for(l.skill).expect("генератор");
        for v in 1..=20_000 {
            let t = g.generate(v);
            assert!(
                !shown.contains(t.statement.as_str()),
                "{}: умова «{}» уроку збігається з варіантом {v}",
                l.skill,
                t.statement
            );
        }
    }
}

#[test]
fn texts_are_clean() {
    for l in all_lessons() {
        for t in texts(&l) {
            assert!(!t.trim().is_empty(), "{}: порожній текст", l.skill);
            assert_eq!(t, t.trim(), "{}: зайві пробіли по краях: {t}", l.skill);
            assert!(!t.contains("  "), "{}: подвійний пробіл: {t}", l.skill);
            assert!(
                !t.chars().any(char::is_control),
                "{}: символ керування: {t}",
                l.skill
            );
        }
    }
}

#[test]
fn all_answers_parse() {
    for l in all_lessons() {
        for b in &l.blocks {
            match b {
                LessonBlock::SelfCheck { answer, .. } => {
                    assert!(parse_answer(answer).is_some(), "{}: {answer}", l.skill)
                }
                LessonBlock::Example { result, .. } => {
                    assert!(parse_answer(result).is_some(), "{}: {result}", l.skill)
                }
                _ => {}
            }
        }
    }
}

#[test]
fn lesson_figures_are_sane_and_captioned() {
    for l in all_lessons() {
        for b in &l.blocks {
            if let LessonBlock::Figure { figure, caption } = b {
                assert!(figure.is_sane(), "{}: непридатний малюнок", l.skill);
                assert!(
                    caption.chars().count() >= 20,
                    "{}: короткий підпис",
                    l.skill
                );
                // Схеми (flow, compare, mindmap, sets) мають порожню назву: перевіряємо лише їхні рядки.
                let skip =
                    usize::from(matches!(figure, tutor_core::figure::Figure::Diagram { .. }));
                for t in figure.texts().into_iter().skip(skip) {
                    assert!(!t.trim().is_empty(), "{}: порожній підпис", l.skill);
                }
            }
        }
    }
}

#[test]
fn key_lessons_have_figures() {
    for skill in [
        "F-ADD-SAME",
        "F-ADD-DIFF",
        "D-ADD",
        "G-AREA",
        "G-PERIM",
        "A-ANGLE",
        "N-ROUND",
    ] {
        let l = MathLessons.lesson(skill).expect("урок є");
        assert!(
            l.blocks
                .iter()
                .any(|b| matches!(b, LessonBlock::Figure { .. })),
            "{skill}: немає малюнка в уроці"
        );
    }
}

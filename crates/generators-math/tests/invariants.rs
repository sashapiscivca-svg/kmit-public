//! Інваріанти І1-І3 на всіх навичках курсу: 5 000 варіантів на навичку (із 35 навичками 10 000 робить прогін надто довгим;
//! одноразовий прогін на 10 000 варіантів пройдено при додаванні навичок розширення).

use generators_math::all_generators;
use tutor_core::guard::{analog_is_safe, find_leak, guarded_analog};
use tutor_core::numtok::numeric_tokens;
use tutor_core::task::Verdict;

const VARIANTS: u64 = 5_000;

/// Одинадцять початкових навичок (нижча межа різноманітності збережена з M2).
fn all_original() -> [&'static str; 11] {
    [
        "N-ADD-MULTI",
        "D-ADD",
        "F-ADD-SAME",
        "F-ADD-DIFF",
        "N-SUB-MULTI",
        "N-MUL-MULTI",
        "N-DIV-MULTI",
        "N-ORDER",
        "D-SUB",
        "F-SUB-SAME",
        "F-SUB-DIFF",
    ]
}

#[test]
fn generation_is_deterministic() {
    for g in all_generators() {
        for v in [1, 2, 3, 77, 9_999] {
            let (a, b) = (g.generate(v), g.generate(v));
            assert_eq!(a.statement, b.statement, "{} варіант {v}", g.skill_id());
            assert_eq!(a.answer, b.answer);
        }
    }
}

#[test]
fn different_variants_give_variety() {
    for g in all_generators() {
        let statements: std::collections::BTreeSet<String> =
            (1..=200).map(|v| g.generate(v).statement).collect();
        // Простір задач скінченний (у F-ADD-SAME приблизно 170 різних умов): вибір задач у M1 має не повторювати умови.
        // Навички розширення курсу мають давати щонайменше 80 різних умов із 200 варіантів.
        let need = if all_original().contains(&g.skill_id()) {
            50
        } else {
            80
        };
        assert!(
            statements.len() > need,
            "{}: лише {} різних задач із 200",
            g.skill_id(),
            statements.len()
        );
    }
}

#[test]
fn correct_answer_and_every_bug_are_recognised() {
    for g in all_generators() {
        for v in 1..=VARIANTS {
            let t = g.generate(v);
            assert_eq!(
                t.check(&t.answer.to_string()),
                Verdict::Correct,
                "{} #{v}",
                g.skill_id()
            );
            for b in &t.bugs {
                assert_eq!(
                    t.check(&b.value.to_string()),
                    Verdict::Bug(b.id),
                    "{} #{v}",
                    g.skill_id()
                );
            }
        }
    }
}

#[test]
fn statement_numbers_are_exactly_the_operands() {
    for g in all_generators() {
        for v in 1..=1_000 {
            let t = g.generate(v);
            let mut tokens = numeric_tokens(&t.statement);
            tokens.sort();
            tokens.dedup();
            let mut ops = t.operands.clone();
            ops.sort();
            ops.dedup();
            assert_eq!(tokens, ops, "{} #{v}: {}", g.skill_id(), t.statement);
        }
    }
}

/// І1: самі шаблони R1-R3 не містять відповіді чи проміжних результатів (запасний текст не потрібен).
#[test]
fn i1_scripted_hints_never_leak() {
    use tutor_core::generator::ScriptedLevel::{R1, R2, R3};
    for g in all_generators() {
        for v in 1..=VARIANTS {
            let t = g.generate(v);
            for level in [R1, R2, R3] {
                let text = g.hint(&t, level);
                assert!(
                    find_leak(&text, &t).is_none(),
                    "{} #{v} {level:?} витік у «{text}»; задача «{}»",
                    g.skill_id(),
                    t.statement
                );
            }
        }
    }
}

/// І2 і І3: безпечний аналог існує для кожного варіанта, а його тексти не містять нічого забороненого.
#[test]
fn i2_i3_analog_exists_and_is_safe_for_every_variant() {
    for g in all_generators() {
        for v in 1..=VARIANTS {
            let t = g.generate(v);
            let ex = guarded_analog(g.as_ref(), &t, &[])
                .unwrap_or_else(|| panic!("{} #{v}: безпечного аналога немає", g.skill_id()));
            assert!(analog_is_safe(&t, &ex, &[]));
            assert_ne!(ex.task.answer, t.answer);
            assert_ne!(ex.task.statement, t.statement);
            assert!(ex.texts().iter().all(|s| find_leak(s, &t).is_none()));
            assert!(
                !ex.texts()
                    .iter()
                    .any(|s| numeric_tokens(s).contains(&ex.task.answer)),
                "{} #{v}: показано фінальну відповідь аналога",
                g.skill_id()
            );
        }
    }
}

/// Краєві номери варіантів не повинні ламати генерацію чи пошук аналога.
#[test]
fn edge_variants_work() {
    for g in all_generators() {
        for v in [0, 1, u64::MAX - 1, u64::MAX] {
            let t = g.generate(v);
            assert_eq!(t.variant, v);
            assert!(
                guarded_analog(g.as_ref(), &t, &[]).is_some(),
                "{} #{v}: немає аналога",
                g.skill_id()
            );
        }
    }
}

/// Типова помилка не може збігатися з числом з умови: інакше дитину, яка просто повторила число, діагностують хибно.
#[test]
fn no_bug_value_equals_an_operand() {
    for g in all_generators() {
        for v in 1..=VARIANTS {
            let t = g.generate(v);
            for b in &t.bugs {
                assert!(
                    !t.operands.contains(&b.value),
                    "{} #{v}: помилка «{}» збігається з операндом у «{}»",
                    g.skill_id(),
                    b.id,
                    t.statement
                );
            }
        }
    }
}

/// І2: відповідь аналога не збігається ні з числом з умови, ні з відомою типовою помилкою цієї задачі.
#[test]
fn analog_answer_never_equals_an_operand_or_a_bug_value() {
    for g in all_generators() {
        for v in 1..=VARIANTS {
            let t = g.generate(v);
            let ex = guarded_analog(g.as_ref(), &t, &[]).expect("аналог має існувати");
            assert!(
                !t.operands.contains(&ex.task.answer),
                "{} #{v}: аналог закінчується числом з умови",
                g.skill_id()
            );
            assert!(
                !t.bugs.iter().any(|b| b.value == ex.task.answer),
                "{} #{v}: аналог закінчується типовою помилкою",
                g.skill_id()
            );
        }
    }
}

/// І2: коли дитина вже ввела цю відповідь, генератор шукає інший аналог.
#[test]
fn analog_avoids_answers_the_child_already_typed() {
    for g in all_generators() {
        for v in 1..=2_000 {
            let t = g.generate(v);
            let first = guarded_analog(g.as_ref(), &t, &[]).unwrap();
            let second = guarded_analog(g.as_ref(), &t, &[first.task.answer])
                .unwrap_or_else(|| panic!("{} #{v}: немає альтернативного аналога", g.skill_id()));
            assert_ne!(second.task.answer, first.task.answer);
        }
    }
}

/// Нижня межа різноманітності: простір умов не має мовчки скорочуватися (діти з підказками витрачають по дві умови на задачу).
#[test]
fn every_skill_has_a_large_enough_pool_of_distinct_statements() {
    let lower_bounds = [
        ("N-ADD-MULTI", 2_000),
        ("D-ADD", 1_000),
        ("F-ADD-SAME", 400),
        ("F-ADD-DIFF", 250),
        ("N-SUB-MULTI", 2_000),
        ("N-MUL-MULTI", 2_000),
        ("N-DIV-MULTI", 1_500),
        ("N-ORDER", 2_000),
        ("D-SUB", 2_000),
        ("F-SUB-SAME", 400),
        ("F-SUB-DIFF", 350),
        // Розширення курсу (рішення A20).
        ("N-ROUND", 2_000),
        ("N-DIVREM", 1_000),
        ("N-DIVTEST", 3_000),
        ("N-GCD", 350),
        ("N-LCM", 250),
        ("N-POWER", 900),
        ("EQ-ADD-SUB", 2_500),
        ("EQ-MUL", 1_000),
        ("WORD-1", 2_500),
        ("WORD-2", 2_500),
        ("MOTION", 2_000),
        ("U-LEN", 1_000),
        ("U-MASS", 900),
        ("U-TIME", 700),
        ("G-PERIM", 2_000),
        ("G-AREA", 1_700),
        ("G-VOL", 1_300),
        ("A-ANGLE", 700),
        ("F-MIXED", 800),
        ("F-REDUCE", 800),
        ("F-OF", 1_000),
        ("P-PCT", 1_700),
        ("D-MUL10", 2_500),
        ("AVG", 2_500),
    ];
    for g in all_generators() {
        let distinct: std::collections::BTreeSet<String> =
            (1..=5_000).map(|v| g.generate(v).statement).collect();
        let need = lower_bounds
            .iter()
            .find(|(id, _)| *id == g.skill_id())
            .map(|(_, n)| *n)
            .expect("для навички є межа");
        assert!(
            distinct.len() >= need,
            "{}: лише {} різних умов, потрібно щонайменше {need}",
            g.skill_id(),
            distinct.len()
        );
    }
}

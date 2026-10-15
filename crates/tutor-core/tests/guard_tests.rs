mod common;

use common::{base_task, f, good_example, AnalogMode, FakeGen};
use tutor_core::frac::Frac;
use tutor_core::generator::ScriptedLevel;
use tutor_core::guard::{
    analog_is_safe, find_leak, guarded_analog, safe_scripted_hint, Leak, SAFE_FALLBACK,
};

#[test]
fn i1_detects_answer_and_intermediate_values() {
    let t = base_task();
    assert_eq!(find_leak("Вийде 5/8", &t), Some(Leak { value: f(5, 8) }));
    assert_eq!(
        find_leak("Рівний дріб 10/16", &t),
        Some(Leak { value: f(5, 8) })
    );
    assert_eq!(
        find_leak("Чисельники дають 5", &t),
        Some(Leak {
            value: Frac::int(5)
        })
    );
    assert_eq!(
        find_leak("Це 0,625", &t),
        Some(Leak { value: f(5, 8) }),
        "десятковий запис тієї ж відповіді"
    );
    assert_eq!(find_leak("Це 0.625", &t), Some(Leak { value: f(5, 8) }));
    assert_eq!(
        find_leak("Додай 3/8 і 2/8", &t),
        None,
        "операнди показувати можна"
    );
    assert_eq!(find_leak("Подумай про знаменники", &t), None);
}

#[test]
fn i1_leaky_scripted_hint_is_replaced_by_safe_fallback() {
    let leaky = FakeGen {
        leaky_hint: true,
        analog_mode: AnalogMode::Good,
    };
    let clean = FakeGen {
        leaky_hint: false,
        analog_mode: AnalogMode::Good,
    };
    let t = base_task();
    for level in [ScriptedLevel::R1, ScriptedLevel::R2, ScriptedLevel::R3] {
        assert_eq!(safe_scripted_hint(&leaky, &t, level), SAFE_FALLBACK);
        assert_eq!(
            safe_scripted_hint(&clean, &t, level),
            "Подумай про знаменники."
        );
    }
    assert_eq!(find_leak(SAFE_FALLBACK, &t), None);
}

#[test]
fn i2_good_analog_is_safe() {
    assert!(analog_is_safe(&base_task(), &good_example(), &[]));
}

#[test]
fn i2_unsafe_analogs_are_rejected() {
    let t = base_task();
    for mode in [
        AnalogMode::SameAnswerAsTask,
        AnalogMode::OverlappingOperands,
        AnalogMode::ShowsTaskAnswer,
        AnalogMode::ShowsOwnAnswer,
    ] {
        let gen = FakeGen {
            leaky_hint: false,
            analog_mode: mode,
        };
        assert!(guarded_analog(&gen, &t, &[]).is_none());
    }
}

#[test]
fn i2_analog_of_another_skill_is_rejected() {
    let mut ex = good_example();
    ex.task.skill = "OTHER";
    assert!(!analog_is_safe(&base_task(), &ex, &[]));
}

#[test]
fn i3_missing_analog_gives_none_not_a_solution() {
    let gen = FakeGen {
        leaky_hint: false,
        analog_mode: AnalogMode::Missing,
    };
    assert!(guarded_analog(&gen, &base_task(), &[]).is_none());
}

#[test]
fn guarded_analog_returns_good_example() {
    let gen = FakeGen {
        leaky_hint: false,
        analog_mode: AnalogMode::Good,
    };
    assert!(guarded_analog(&gen, &base_task(), &[]).is_some());
}

#[test]
fn i2_analog_answer_equal_to_a_task_operand_is_rejected() {
    let mut ex = good_example();
    ex.task.answer = f(3, 8); // число з умови вихідної задачі
    assert!(!analog_is_safe(&base_task(), &ex, &[]));
}

#[test]
fn i2_analog_answer_equal_to_a_known_bug_value_of_the_task_is_rejected() {
    let mut ex = good_example();
    ex.task.answer = f(5, 16); // типова помилка вихідної задачі
    assert!(!analog_is_safe(&base_task(), &ex, &[]));
}

#[test]
fn i2_analog_answer_equal_to_what_the_child_just_typed_is_rejected() {
    // Дитина ввела 3/5 і почула «неправильно»; аналог не має закінчуватися тим самим числом.
    assert!(!analog_is_safe(&base_task(), &good_example(), &[f(3, 5)]));
    assert!(analog_is_safe(&base_task(), &good_example(), &[f(1, 2)]));
}

#[test]
fn guarded_analog_passes_the_childs_rejected_answers_through() {
    let gen = FakeGen {
        leaky_hint: false,
        analog_mode: AnalogMode::Good,
    };
    assert!(guarded_analog(&gen, &base_task(), &[f(3, 5)]).is_none());
    assert!(guarded_analog(&gen, &base_task(), &[f(1, 2)]).is_some());
}

#[test]
fn worked_example_final_step_is_checked_by_the_engine() {
    use tutor_core::task::Verdict;
    let ex = good_example(); // аналог: 1/5 + 2/5 = 3/5
    assert_eq!(ex.check_final("3/5"), Verdict::Correct);
    assert_eq!(ex.check_final("3/10"), Verdict::Bug("add_denominators"));
    assert_eq!(ex.check_final("1/2"), Verdict::Wrong);
}

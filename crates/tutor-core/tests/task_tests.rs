use tutor_core::frac::Frac;
use tutor_core::task::{validate_task, BugAnswer, Task, TaskError, Verdict};

fn f(n: i64, d: i64) -> Frac {
    Frac::new(n, d).unwrap()
}

/// 3/8 + 2/8 = 5/8; проміжний результат 5; типова помилка 5/16.
fn fixture() -> Task {
    Task {
        skill: "T",
        variant: 1,
        statement: "Обчисли: 3/8 + 2/8".to_string(),
        operands: vec![f(3, 8), f(2, 8)],
        operand_labels: vec!["3/8".into(), "2/8".into()],
        params: vec![3, 2, 8],
        answer: f(5, 8),
        steps: vec![Frac::int(5)],
        bugs: vec![BugAnswer {
            id: "add_denominators",
            value: f(5, 16),
        }],
    }
}

#[test]
fn check_distinguishes_all_verdicts() {
    let t = fixture();
    assert_eq!(t.check("5/8"), Verdict::Correct);
    assert_eq!(t.check(" 5 / 8 "), Verdict::Correct);
    assert_eq!(
        t.check("10/16"),
        Verdict::Correct,
        "рівний дріб теж правильний"
    );
    assert_eq!(t.check("5/16"), Verdict::Bug("add_denominators"));
    assert_eq!(t.check("1/2"), Verdict::Wrong);
    assert_eq!(t.check("п'ять восьмих"), Verdict::Unparseable);
}

#[test]
fn protected_values_are_answer_plus_steps_sorted_unique() {
    let mut t = fixture();
    t.steps = vec![Frac::int(5), Frac::int(5), f(5, 8)];
    assert_eq!(t.protected_values(), vec![f(5, 8), Frac::int(5)]);
}

#[test]
fn valid_task_passes() {
    assert_eq!(validate_task(&fixture(), f(5, 8)), Ok(()));
}

#[test]
fn oracle_mismatch_is_rejected() {
    assert_eq!(
        validate_task(&fixture(), f(1, 2)),
        Err(TaskError::OracleMismatch)
    );
}

#[test]
fn protected_value_equal_to_operand_is_rejected() {
    let mut t = fixture();
    t.steps = vec![f(3, 8)];
    assert_eq!(
        validate_task(&t, f(5, 8)),
        Err(TaskError::ProtectedOverlapsOperand)
    );
}

#[test]
fn bug_equal_to_answer_is_rejected() {
    let mut t = fixture();
    t.bugs = vec![BugAnswer {
        id: "x",
        value: f(5, 8),
    }];
    assert_eq!(validate_task(&t, f(5, 8)), Err(TaskError::BugEqualsAnswer));
}

#[test]
fn bug_equal_to_operand_is_rejected() {
    let mut t = fixture();
    t.bugs = vec![BugAnswer {
        id: "x",
        value: f(3, 8),
    }];
    assert_eq!(validate_task(&t, f(5, 8)), Err(TaskError::BugEqualsOperand));
}

#[test]
fn duplicate_bug_values_are_rejected() {
    let mut t = fixture();
    t.bugs = vec![
        BugAnswer {
            id: "a",
            value: f(5, 16),
        },
        BugAnswer {
            id: "b",
            value: f(5, 16),
        },
    ];
    assert_eq!(validate_task(&t, f(5, 8)), Err(TaskError::BugsNotDistinct));
}

#[test]
fn statement_numbers_must_match_operands() {
    let mut t = fixture();
    t.statement = "Обчисли: 3/8 + 4/8".to_string();
    assert_eq!(
        validate_task(&t, f(5, 8)),
        Err(TaskError::StatementNumbersMismatch)
    );
}

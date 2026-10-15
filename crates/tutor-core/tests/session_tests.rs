mod common;

use common::{base_task, AnalogMode, FakeGen};
use tutor_core::config::Config;
use tutor_core::guard::{find_leak, SAFE_FALLBACK};
use tutor_core::ladder::Ladder;
use tutor_core::session::{next_hint, HintOutput};

fn ladder() -> Ladder {
    Ladder::new(&Config::default())
}

#[test]
fn full_path_with_good_analog() {
    let gen = FakeGen {
        leaky_hint: false,
        analog_mode: AnalogMode::Good,
    };
    let task = base_task();
    let mut l = ladder();
    for _ in 0..3 {
        assert!(matches!(
            next_hint(&mut l, &gen, &task, &[]),
            HintOutput::Text(_)
        ));
    }
    assert!(matches!(
        next_hint(&mut l, &gen, &task, &[]),
        HintOutput::Analog(_)
    ));
    assert!(matches!(
        next_hint(&mut l, &gen, &task, &[]),
        HintOutput::ReturnToTask
    ));
}

#[test]
fn leaky_generator_never_shows_the_answer() {
    let gen = FakeGen {
        leaky_hint: true,
        analog_mode: AnalogMode::Good,
    };
    let task = base_task();
    let mut l = ladder();
    for _ in 0..3 {
        match next_hint(&mut l, &gen, &task, &[]) {
            HintOutput::Text(t) => {
                assert_eq!(t, SAFE_FALLBACK);
                assert!(find_leak(&t, &task).is_none());
            }
            other => panic!("очікувався текст, отримано {other:?}"),
        }
    }
}

#[test]
fn i3_unsafe_or_missing_analog_ends_in_ask_adult_without_solution() {
    for mode in [
        AnalogMode::Missing,
        AnalogMode::SameAnswerAsTask,
        AnalogMode::ShowsTaskAnswer,
        AnalogMode::ShowsOwnAnswer,
        AnalogMode::OverlappingOperands,
    ] {
        let gen = FakeGen {
            leaky_hint: false,
            analog_mode: mode,
        };
        let task = base_task();
        let mut l = ladder();
        for _ in 0..3 {
            next_hint(&mut l, &gen, &task, &[]);
        }
        assert!(matches!(
            next_hint(&mut l, &gen, &task, &[]),
            HintOutput::AskAdult
        ));
        assert!(matches!(
            next_hint(&mut l, &gen, &task, &[]),
            HintOutput::AskAdult
        ));
    }
}

#[test]
fn analog_ending_in_a_rejected_answer_leads_to_ask_adult() {
    use common::f;
    let gen = FakeGen {
        leaky_hint: false,
        analog_mode: AnalogMode::Good,
    };
    let task = base_task();
    let rejected = [f(3, 5)]; // збігається з відповіддю аналога
    let mut l = ladder();
    for _ in 0..3 {
        next_hint(&mut l, &gen, &task, &rejected);
    }
    assert!(matches!(
        next_hint(&mut l, &gen, &task, &rejected),
        HintOutput::AskAdult
    ));
}

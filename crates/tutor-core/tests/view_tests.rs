mod common;

use common::{base_task, f, good_example, AnalogMode, FakeGen};
use tutor_core::config::Config;
use tutor_core::guard::find_leak;
use tutor_core::ladder::Ladder;
use tutor_core::session::{next_hint, HintOutput};
use tutor_core::view::{hint_view, task_view, ChildAnalogView, ChildHint};

#[test]
fn task_view_carries_only_the_statement() {
    let t = base_task();
    let v = task_view(&t);
    assert_eq!(v.skill, "T");
    assert_eq!(v.variant, 1);
    assert_eq!(v.statement, "Обчисли: 3/8 + 2/8");
    assert!(
        find_leak(&format!("{v:?}"), &t).is_none(),
        "подання не містить відповіді: {v:?}"
    );
}

#[test]
fn every_hint_output_converts_without_answer_fields() {
    let t = base_task();
    assert_eq!(
        hint_view(HintOutput::Text("Подумай.".into())),
        ChildHint::Text("Подумай.".into())
    );
    assert_eq!(hint_view(HintOutput::ReturnToTask), ChildHint::ReturnToTask);
    assert_eq!(hint_view(HintOutput::OfferHint), ChildHint::OfferHint);
    assert_eq!(hint_view(HintOutput::AskAdult), ChildHint::AskAdult);

    let ex = good_example();
    let v = hint_view(HintOutput::Analog(Box::new(ex.clone())));
    assert_eq!(
        v,
        ChildHint::Analog(ChildAnalogView {
            statement: ex.task.statement.clone(),
            shown_steps: ex.shown_steps.clone(),
            final_prompt: ex.final_prompt.clone(),
        })
    );
    let shown = format!("{v:?}");
    assert!(
        find_leak(&shown, &t).is_none(),
        "відповідь вихідної задачі в поданні: {shown}"
    );
    assert!(
        !tutor_core::numtok::numeric_tokens(&shown).contains(&ex.task.answer),
        "фінальна відповідь аналога в поданні: {shown}"
    );
}

#[test]
fn whole_ladder_run_produces_leak_free_views() {
    let gen = FakeGen {
        leaky_hint: true,
        analog_mode: AnalogMode::Good,
    };
    let t = base_task();
    let mut ladder = Ladder::new(&Config::default());
    for _ in 0..6 {
        let view = hint_view(next_hint(&mut ladder, &gen, &t, &[]));
        let shown = format!("{view:?}");
        assert!(find_leak(&shown, &t).is_none(), "{shown}");
    }
}

#[test]
fn the_final_answer_of_the_analog_stays_with_the_engine() {
    let ex = good_example();
    // Дитина дописує аналог; перевіряє рушій.
    assert_eq!(ex.check_final("3/5"), tutor_core::task::Verdict::Correct);
    assert_ne!(ex.task.answer, f(5, 8));
}

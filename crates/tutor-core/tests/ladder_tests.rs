use tutor_core::config::Config;
use tutor_core::generator::ScriptedLevel;
use tutor_core::ladder::{Ladder, LadderAction};

fn ladder() -> Ladder {
    Ladder::new(&Config::default())
}

#[test]
fn hint_requests_walk_the_ladder_in_order() {
    let mut l = ladder();
    assert_eq!(
        l.request_hint(),
        LadderAction::ShowScripted(ScriptedLevel::R1)
    );
    assert_eq!(
        l.request_hint(),
        LadderAction::ShowScripted(ScriptedLevel::R2)
    );
    assert_eq!(
        l.request_hint(),
        LadderAction::ShowScripted(ScriptedLevel::R3)
    );
    assert_eq!(l.request_hint(), LadderAction::ShowAnalog);
    assert_eq!(l.hints_used(), 4);
}

#[test]
fn after_analog_further_requests_only_return_to_task() {
    let mut l = ladder();
    for _ in 0..4 {
        l.request_hint();
    }
    assert_eq!(l.request_hint(), LadderAction::ReturnToTask);
    assert_eq!(l.request_hint(), LadderAction::ReturnToTask);
    assert_eq!(l.hints_used(), 4, "рівнів не більше чотирьох");
}

#[test]
fn two_wrong_answers_after_analog_lead_to_ask_adult() {
    let mut l = ladder();
    for _ in 0..4 {
        l.request_hint();
    }
    assert_eq!(l.record_wrong(), LadderAction::ReturnToTask);
    assert_eq!(l.record_wrong(), LadderAction::AskAdult);
    assert_eq!(l.request_hint(), LadderAction::AskAdult);
    assert_eq!(l.record_wrong(), LadderAction::AskAdult);
}

#[test]
fn hint_is_offered_after_two_wrong_answers_before_any_hint() {
    let mut l = ladder();
    assert_eq!(l.record_wrong(), LadderAction::ReturnToTask);
    assert_eq!(l.record_wrong(), LadderAction::OfferHint);
}

#[test]
fn no_offer_once_hints_have_started() {
    let mut l = ladder();
    l.request_hint();
    assert_eq!(l.record_wrong(), LadderAction::ReturnToTask);
    assert_eq!(l.record_wrong(), LadderAction::ReturnToTask);
}

#[test]
fn analog_unavailable_goes_straight_to_ask_adult() {
    let mut l = ladder();
    for _ in 0..3 {
        l.request_hint();
    }
    l.analog_unavailable();
    assert_eq!(l.request_hint(), LadderAction::AskAdult);
}

#[test]
fn thresholds_come_from_config() {
    let cfg = Config {
        max_wrong_after_analog: 1,
        offer_hint_after_wrong: 1,
        ..Config::default()
    };
    let mut l = Ladder::new(&cfg);
    assert_eq!(l.record_wrong(), LadderAction::OfferHint);
    for _ in 0..4 {
        l.request_hint();
    }
    assert_eq!(l.record_wrong(), LadderAction::AskAdult);
}

#[test]
fn many_wrong_answers_without_hints_never_overflow() {
    let mut l = ladder();
    for _ in 0..1000 {
        l.record_wrong();
    }
    // Дитина все ще може попросити підказку, і драбина починається з R1.
    assert_eq!(
        l.request_hint(),
        LadderAction::ShowScripted(ScriptedLevel::R1)
    );
}

#![cfg(not(feature = "reviewer"))]
mod common;

use common::{app, call, create_and_open, ctx, ok, TestDir};
use serde_json::json;

#[test]
fn profile_lifecycle_through_the_api() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    assert_eq!(
        ok(&mut a, "profiles", json!({}), &c)["profiles"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let (slot, code) = create_and_open(&mut a, &c);
    assert_eq!(code.len(), 23);
    let home = ok(&mut a, "home", json!({}), &c);
    assert_eq!(home["nickname"], "Мирослава");
    assert_eq!(home["avatar"], 2);
    ok(&mut a, "lock", json!({}), &c);
    assert_eq!(
        call(&mut a, "home", json!({}), &c).unwrap_err().code,
        "no_profile_open"
    );
    assert_eq!(
        call(
            &mut a,
            "open_profile",
            json!({"slot": slot, "pin": "0000"}),
            &c
        )
        .unwrap_err()
        .code,
        "wrong_pin"
    );
    assert_eq!(
        call(
            &mut a,
            "open_profile",
            json!({"slot": slot, "pin": "12"}),
            &c
        )
        .unwrap_err()
        .code,
        "invalid_pin"
    );
    ok(
        &mut a,
        "open_profile",
        json!({"slot": slot, "pin": "4821"}),
        &c,
    );
    assert_eq!(ok(&mut a, "home", json!({}), &c)["nickname"], "Мирослава");
    ok(&mut a, "lock", json!({}), &c);
    ok(
        &mut a,
        "recover_profile",
        json!({"slot": slot, "code": code, "new_pin": "9090"}),
        &c,
    );
    ok(&mut a, "lock", json!({}), &c);
    assert!(call(
        &mut a,
        "open_profile",
        json!({"slot": slot, "pin": "4821"}),
        &c
    )
    .is_err());
    ok(
        &mut a,
        "open_profile",
        json!({"slot": slot, "pin": "9090"}),
        &c,
    );
    ok(&mut a, "lock", json!({}), &c);
    ok(&mut a, "erase_profile", json!({"slot": slot}), &c);
    assert_eq!(
        ok(&mut a, "profiles", json!({}), &c)["profiles"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn a_new_child_sees_a_map_with_one_available_skill_and_can_start() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_and_open(&mut a, &c);
    let home = ok(&mut a, "home", json!({}), &c);
    let skills = home["skills"].as_array().unwrap();
    let states: Vec<_> = skills
        .iter()
        .filter(|s| s["subject"] == "math")
        .map(|s| (s["id"].as_str().unwrap(), s["state"].as_str().unwrap()))
        .collect();
    assert!(states.contains(&("N-ADD-MULTI", "available")), "{states:?}");
    assert!(states.contains(&("F-ADD-DIFF", "locked")));
    assert_eq!(home["stickers"]["count"], 0);
    assert_eq!(
        home["stickers"]["capacity"].as_u64().unwrap(),
        4 * skills.len() as u64
    );
    assert_eq!(home["can_start"]["ok"], true);
    assert!(
        !home.to_string().contains("p_known"),
        "числових оцінок знання дитині не показуємо"
    );
}

#[test]
fn the_first_task_never_exposes_the_answer_and_a_correct_answer_finishes_it() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_and_open(&mut a, &c);
    let m = ok(&mut a, "session_start", json!({}), &c)["model"].clone();
    assert_eq!(m["phase"], "task");
    assert!(m["statement"].as_str().unwrap().starts_with("Обчисли:"));
    assert_eq!(m["kind"], "new");
    let answer = a.peek_answer().unwrap();
    let r = ok(&mut a, "answer", json!({"input": answer}), &ctx(1_010, 1));
    assert_eq!(r["result"], "correct");
    assert_eq!(r["model"]["phase"], "feedback");
    assert_eq!(r["model"]["outcome"], "solved");
    let next = ok(&mut a, "continue", json!({}), &ctx(1_012, 1))["model"].clone();
    assert_eq!(next["phase"], "task");
    assert_eq!(next["number"], 2);
}

#[test]
fn wrong_unreadable_and_hint_paths() {
    let d = TestDir::new();
    let mut a = app(&d);
    create_and_open(&mut a, &ctx(1_000, 1));
    ok(&mut a, "session_start", json!({}), &ctx(1_000, 1));

    let r = ok(&mut a, "answer", json!({"input": "ой"}), &ctx(1_005, 1));
    assert_eq!(
        r["result"], "unreadable",
        "нерозбірливе не вважається помилкою"
    );

    let r = ok(&mut a, "answer", json!({"input": "0"}), &ctx(1_010, 1));
    assert_eq!(r["result"], "try_again");
    assert_eq!(r["offer_hint"], false);
    let r = ok(&mut a, "answer", json!({"input": "1"}), &ctx(1_020, 1));
    assert_eq!(
        r["offer_hint"], true,
        "після двох помилок пропонується підказка"
    );

    let h = ok(&mut a, "hint", json!({}), &ctx(1_030, 1));
    assert_eq!(h["hint"]["kind"], "text");
    let wait = ok(&mut a, "hint", json!({}), &ctx(1_032, 1));
    assert_eq!(wait["wait"]["secs"], 10);
    let h2 = ok(&mut a, "hint", json!({}), &ctx(1_050, 1));
    assert_eq!(h2["hint"]["kind"], "text");
}

#[test]
fn the_full_ladder_ends_with_an_analog_the_child_completes_then_returns() {
    let d = TestDir::new();
    let mut a = app(&d);
    create_and_open(&mut a, &ctx(1_000, 1));
    ok(&mut a, "session_start", json!({}), &ctx(1_000, 1));
    let mut now = 1_000;
    // Власна спроба передує першій підказці (рішення власника, Q1).
    ok(&mut a, "answer", json!({"input": "0"}), &ctx(now + 1, 1));
    let mut analog = None;
    for _ in 0..4 {
        now += 100; // повільно й спокійно: затримки проходять
        let h = ok(&mut a, "hint", json!({}), &ctx(now, 1));
        if h["hint"]["kind"] == "analog" {
            analog = Some(h);
            break;
        }
        // Між підказками дитина пробує відповісти (інакше це «клацання до відповіді»).
        ok(&mut a, "answer", json!({"input": "0"}), &ctx(now + 5, 1));
    }
    let analog = analog.expect("четвертий рівень: аналог");
    assert!(analog["hint"]["statement"]
        .as_str()
        .unwrap()
        .starts_with("Обчисли:"));
    assert!(analog["hint"]["prompt"]
        .as_str()
        .unwrap()
        .contains("Тепер ти"));
    assert_eq!(analog["model"]["phase"], "analog");
    // Дитина дописує аналог: помилка не штрафується, правильна відповідь повертає до своєї задачі.
    let wrong = ok(&mut a, "answer", json!({"input": "0"}), &ctx(now + 120, 1));
    assert_eq!(wrong["result"], "analog_try_again");
    let right = a.peek_analog_answer().unwrap();
    let done = ok(
        &mut a,
        "answer",
        json!({"input": right}),
        &ctx(now + 130, 1),
    );
    assert_eq!(done["result"], "analog_done");
    assert_eq!(done["model"]["phase"], "task");
    // Власну відповідь дитина досі має знайти сама.
    let own = a.peek_answer().unwrap();
    let fin = ok(&mut a, "answer", json!({"input": own}), &ctx(now + 150, 1));
    assert_eq!(fin["result"], "correct");
}

#[test]
fn giving_up_hands_the_task_to_an_adult_without_a_solution_and_without_stickers() {
    let d = TestDir::new();
    let mut a = app(&d);
    create_and_open(&mut a, &ctx(1_000, 1));
    ok(&mut a, "session_start", json!({}), &ctx(1_000, 1));
    let r = ok(&mut a, "hand_to_adult", json!({}), &ctx(1_050, 1))["model"].clone();
    assert_eq!(r["phase"], "feedback");
    assert_eq!(r["outcome"], "adult");
    assert!(r["stickers"].as_array().unwrap().is_empty());
    let sheet = ok(&mut a, "sheet", json!({}), &ctx(1_060, 1));
    assert_eq!(sheet["ask_adult"].as_array().unwrap().len(), 1);
    let item = &sheet["ask_adult"][0];
    assert!(item["statement"].as_str().unwrap().starts_with("Обчисли:"));
}

#[test]
fn the_recovery_sheet_is_made_from_a_valid_code_only() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    let r = ok(
        &mut a,
        "recovery_sheet",
        json!({"code": "ABCDE-12345-FGHIJ-67890", "nickname": "Оля"}),
        &c,
    );
    assert_eq!(r["filename"], "kod-vidnovlennya.pdf");
    assert!(r["data"].as_str().is_some_and(|s| s.starts_with("JVBER")));
    for bad in ["", "AB CD", "<script>", &"A".repeat(60)] {
        assert!(
            call(&mut a, "recovery_sheet", json!({"code": bad}), &c).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn the_adult_notice_acknowledgement_is_a_saved_setting_that_defaults_to_false() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_and_open(&mut a, &c);
    assert_eq!(
        ok(&mut a, "settings_get", json!({}), &c)["adult_notice_ack"],
        false
    );
    ok(
        &mut a,
        "settings_set",
        json!({"adult_notice_ack": true}),
        &c,
    );
    assert_eq!(
        ok(&mut a, "settings_get", json!({}), &c)["adult_notice_ack"],
        true
    );
}

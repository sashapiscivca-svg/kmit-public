#![cfg(not(feature = "reviewer"))]
mod common;
use common::*;
use serde_json::json;

fn open_child(a: &mut app_core::App, c: &app_core::Ctx) {
    create_and_open(a, c);
}

#[test]
fn every_grade_has_its_own_world_and_the_collection_can_be_read_per_grade() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    open_child(&mut a, &c);
    let five = ok(&mut a, "album", json!({}), &c);
    assert_eq!(five["grade"], 5);
    assert_eq!(five["world"]["id"], "forest");
    let six = ok(&mut a, "album", json!({"grade": 6}), &c);
    assert_eq!(six["world"]["id"], "sea");
    // Світ є в кожного класу, де є колекція.
    for (g, w) in [
        (8, "cave"),
        (9, "desert"),
        (10, "space"),
        (11, "ice"),
        (12, "summit"),
    ] {
        assert_eq!(
            ok(&mut a, "album", json!({"grade": g}), &c)["world"]["id"],
            w
        );
    }
    assert_eq!(six["grade"], 6);
    // Нічого не відбирається й не змішується: клітини лише того класу.
    assert!(six["cells"]
        .as_array()
        .unwrap()
        .iter()
        .all(|x| x["skill"].as_str().is_some()));
    let bad = ok(&mut a, "album", json!({"grade": 99}), &c);
    assert_eq!(bad["grade"], 5, "хибний клас замінюється поточним");
    let worlds = five["worlds"].as_array().unwrap();
    assert_eq!(
        worlds.len(),
        1,
        "у п'ятикласника лише його світ: {worlds:?}"
    );
}

#[test]
fn the_celebration_is_offered_from_eighty_percent_and_never_blocks_the_move() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    open_child(&mut a, &c);
    let s = ok(&mut a, "ceremony_status", json!({}), &c);
    assert_eq!(s["from"], 5);
    assert_eq!(s["to"], 6);
    assert_eq!(s["can"], true);
    assert_eq!(s["mastered"], 0);
    assert!(
        ok(&mut a, "home", json!({}), &c)["promote"].is_null(),
        "з 0 % свято не пропонується"
    );
    a.master_grade_for_test(5, 79);
    assert!(
        ok(&mut a, "home", json!({}), &c)["promote"].is_null(),
        "79 % ще мало"
    );
    a.master_grade_for_test(5, 80);
    let h = ok(&mut a, "home", json!({}), &c);
    assert_eq!(h["promote"]["from"], 5);
    assert_eq!(h["promote"]["to"], 6);
    assert!(h["promote"]["percent"].as_u64().unwrap() >= 80);
}

#[test]
fn moving_up_records_the_celebration_once_switches_the_grade_and_keeps_the_old_collection() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    open_child(&mut a, &c);
    // Без жодної опанованої теми перейти можна: перехід нічим не блокується.
    let summary = ok(&mut a, "ceremony_do", json!({}), &c);
    assert_eq!(summary["from"], 5);
    assert_eq!(summary["to"], 6);
    assert_eq!(summary["next_world"]["id"], "sea");
    assert_eq!(ok(&mut a, "home", json!({}), &c)["grade"], 6);
    let s = ok(&mut a, "ceremony_status", json!({}), &c);
    assert_eq!(s["from"], 6);
    assert_eq!(s["done_before"], false);
    // Свідоцтво лише після свята цього класу.
    let pdf = ok(&mut a, "ceremony_pdf", json!({"from": 5}), &c);
    assert!(pdf["data"].as_str().unwrap().starts_with("JVBER"));
    assert!(call(&mut a, "ceremony_pdf", json!({"from": 6}), &c).is_err());
    // Нагород свято не видає: колекція 5 класу та сама.
    assert_eq!(ok(&mut a, "album", json!({"grade": 5}), &c)["count"], 0);
    // Повернутися назад можна, і свято повторно не фіксується.
    ok(&mut a, "settings_set", json!({"grade": 5}), &c);
    assert_eq!(
        ok(&mut a, "ceremony_status", json!({}), &c)["done_before"],
        true
    );
    // 6 -> 7 можна, 7 -> 8 ні (8 клас ще без вмісту).
    ok(&mut a, "settings_set", json!({"grade": 6}), &c);
    ok(&mut a, "ceremony_do", json!({}), &c);
    assert_eq!(ok(&mut a, "home", json!({}), &c)["grade"], 7);
    assert!(call(&mut a, "ceremony_do", json!({}), &c).is_err());
    let s = ok(&mut a, "ceremony_status", json!({}), &c);
    assert_eq!(s["can"], false);
    // Світи пройдених класів видно в колекціях.
    let worlds = ok(&mut a, "album", json!({}), &c)["worlds"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(worlds, 3);
}

#[test]
fn the_celebration_survives_a_restart() {
    let d = TestDir::new();
    let c = ctx(1_000, 1);
    let slot;
    {
        let mut a = app(&d);
        slot = create_and_open(&mut a, &c).0;
        ok(&mut a, "ceremony_do", json!({}), &c);
    }
    let mut a = app(&d);
    ok(
        &mut a,
        "open_profile",
        json!({"slot": slot, "pin": "4821"}),
        &c,
    );
    assert!(ok(&mut a, "ceremony_pdf", json!({"from": 5}), &c)["data"].is_string());
}

#[test]
fn the_teacher_profile_has_no_celebration() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    assert_eq!(
        call(&mut a, "ceremony_do", json!({}), &c).unwrap_err().code,
        "wrong_profile_kind"
    );
}

#[test]
fn the_last_grade_has_its_own_final_celebration_without_a_new_world() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    open_child(&mut a, &c);
    ok(&mut a, "settings_set", json!({"grade": 12}), &c);
    let s = ok(&mut a, "ceremony_status", json!({}), &c);
    assert_eq!(s["final"], true);
    assert_eq!(s["can"], true);
    assert!(s["to"].is_null());
    assert!(s["overall"]["total"].as_u64().unwrap() > 0);
    assert!(call(&mut a, "ceremony_pdf", json!({"from": 12}), &c).is_err());
    let done = ok(&mut a, "ceremony_do", json!({}), &c);
    assert_eq!(done["final"], true);
    assert_eq!(
        ok(&mut a, "home", json!({}), &c)["grade"],
        12,
        "клас не змінюється"
    );
    assert!(ok(&mut a, "ceremony_pdf", json!({"from": 12}), &c)["data"]
        .as_str()
        .unwrap()
        .starts_with("JVBER"));
    // Повторно свято не пропонується.
    assert!(ok(&mut a, "home", json!({}), &c)["promote"].is_null());
}

#![cfg(not(feature = "reviewer"))]
//! Окремі профілі вчителя й учня: тип видно в списку, команди розведені, PIN і сховища різні.
mod common;
use common::*;
use serde_json::json;

fn create_kind(a: &mut app_core::App, kind: &str, pin: &str, c: &app_core::Ctx) -> u64 {
    let r = ok(
        a,
        "create_profile",
        json!({"nickname": "Оксана", "pin": pin, "kind": kind}),
        c,
    );
    r["slot"].as_u64().unwrap()
}

#[test]
fn the_profile_list_tells_a_teacher_from_a_child() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    let child = create_kind(&mut a, "child", "4821", &c);
    let teacher = create_kind(&mut a, "teacher", "1357", &c);
    let list = ok(&mut a, "profiles", json!({}), &c);
    let kind_of = |slot: u64| {
        list["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["slot"] == slot)
            .unwrap()["kind"]
            .clone()
    };
    assert_eq!(kind_of(child), "child");
    assert_eq!(kind_of(teacher), "teacher");
}

#[test]
fn a_teacher_profile_cannot_run_child_commands_and_a_child_cannot_run_teacher_ones() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_kind(&mut a, "teacher", "1357", &c);
    for cmd in [
        "home",
        "session_start",
        "album",
        "week_status",
        "hero_catalog",
        "notes_list",
        "sheet",
    ] {
        let e = call(&mut a, cmd, json!({}), &c).unwrap_err();
        assert_eq!(e.code, "wrong_profile_kind", "{cmd}");
    }
    ok(&mut a, "settings_get", json!({}), &c);
    ok(&mut a, "lock", json!({}), &c);

    create_kind(&mut a, "child", "4821", &c);
    let e = call(&mut a, "teacher_topics", json!({"grade": 5}), &c).unwrap_err();
    assert_eq!(e.code, "wrong_profile_kind");
    ok(&mut a, "home", json!({}), &c);
}

#[test]
fn another_profiles_pin_does_not_open_a_teacher_slot() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    let child = create_kind(&mut a, "child", "4821", &c);
    let teacher = create_kind(&mut a, "teacher", "1357", &c);
    ok(&mut a, "lock", json!({}), &c);
    assert!(call(
        &mut a,
        "open_profile",
        json!({"slot": teacher, "pin": "4821"}),
        &c
    )
    .is_err());
    assert!(call(
        &mut a,
        "open_profile",
        json!({"slot": child, "pin": "1357"}),
        &c
    )
    .is_err());
    ok(
        &mut a,
        "open_profile",
        json!({"slot": teacher, "pin": "1357"}),
        &c,
    );
    let e = call(&mut a, "home", json!({}), &c).unwrap_err();
    assert_eq!(e.code, "wrong_profile_kind");
}

#[test]
fn opening_and_importing_report_the_profile_kind_and_a_teacher_profile_survives_a_backup() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    let slot = create_kind(&mut a, "teacher", "1357", &c);
    ok(&mut a, "lock", json!({}), &c);
    let opened = ok(
        &mut a,
        "open_profile",
        json!({"slot": slot, "pin": "1357"}),
        &c,
    );
    assert_eq!(opened["kind"], "teacher");

    let exp = ok(
        &mut a,
        "export_profile",
        json!({"passphrase": "фраза для перенесення"}),
        &c,
    );
    let data = exp["data"].as_str().unwrap().to_string();
    let d2 = TestDir::new();
    let mut b = app(&d2);
    let r = ok(
        &mut b,
        "import_profile",
        json!({"data": data, "passphrase": "фраза для перенесення", "pin": "7777"}),
        &c,
    );
    assert_eq!(r["kind"], "teacher", "тип профілю переноситься");
    let list = ok(&mut b, "profiles", json!({}), &c);
    assert_eq!(
        list["profiles"][0]["kind"], "teacher",
        "у списку імпортований профіль вчительський"
    );
    ok(&mut b, "teacher_home", json!({}), &c);
}

#[test]
fn the_reviewer_commands_do_not_exist_in_the_child_and_teacher_app() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    assert_eq!(
        call(&mut a, "rv_overview", json!({}), &c).unwrap_err().code,
        "wrong_profile_kind"
    );
    assert_eq!(ok(&mut a, "about", json!({}), &c)["reviewer"], false);
}

#[test]
fn the_teacher_gets_small_celebrations_once_for_milestones_and_the_calendar() {
    let d = TestDir::new();
    let mut a = app(&d);
    let june = ctx(1_000, 20_605); // 2026-06-01, звичайний день
    create_teacher(&mut a, &june);
    assert!(ok(&mut a, "teacher_home", json!({}), &june)["celebration"].is_null());
    ok(
        &mut a,
        "teacher_work_build",
        json!({"grade": 5, "skills": ["D-ADD"], "kind": "quiz", "seed": 1}),
        &june,
    );
    let h = ok(&mut a, "teacher_home", json!({}), &june);
    assert_eq!(h["celebration"]["id"], "works:1");
    assert!(h["celebration"]["title"]
        .as_str()
        .unwrap()
        .contains("Перша"));
    ok(
        &mut a,
        "teacher_celebrate_seen",
        json!({"id": "works:1"}),
        &june,
    );
    assert!(
        ok(&mut a, "teacher_home", json!({}), &june)["celebration"].is_null(),
        "лише один раз"
    );
    assert!(call(
        &mut a,
        "teacher_celebrate_seen",
        json!({"id": "../x"}),
        &june
    )
    .is_err());
    // День працівників освіти: перша неділя жовтня 2026 (4-го).
    let td = ctx(2_000, 20_730);
    let h = ok(&mut a, "teacher_home", json!({}), &td);
    assert_eq!(h["celebration"]["id"], "td:2026");
    assert!(h["celebration"]["title"].as_str().unwrap().contains("Дн"));
}

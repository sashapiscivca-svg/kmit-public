//! Збірка методиста (`--features reviewer,test-support`): лише вичитка, без дитячих профілів, робіт і планів.
#![cfg(all(feature = "reviewer", feature = "test-support"))]
mod common;
use common::*;
use serde_json::{json, Value};

const SKILL: &str = "UA-APOSTROPHE";

fn topic(a: &mut app_core::App, c: &app_core::Ctx) -> Value {
    ok(a, "rv_topic", json!({"skill": SKILL}), c)
}

fn unit<'a>(t: &'a Value, id: &str) -> &'a Value {
    t["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["unit"] == id)
        .unwrap_or_else(|| panic!("немає одиниці {id}"))
}

fn note(
    a: &mut app_core::App,
    c: &app_core::Ctx,
    args: Value,
) -> Result<Value, app_core::ApiError> {
    call(a, "rv_note", args, c)
}

#[test]
fn the_reviewer_build_says_so_allows_one_profile_and_refuses_child_profiles_and_imports() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    assert_eq!(ok(&mut a, "about", json!({}), &c)["reviewer"], true);
    let child = call(
        &mut a,
        "create_profile",
        json!({"nickname": "Дитина", "pin": "1234"}),
        &c,
    );
    assert_eq!(child.unwrap_err().code, "wrong_profile_kind");
    let imp = call(
        &mut a,
        "import_profile",
        json!({"data": "", "passphrase": "x", "pin": "1234"}),
        &c,
    );
    assert_eq!(imp.unwrap_err().code, "wrong_profile_kind");
    assert_eq!(ok(&mut a, "profiles", json!({}), &c)["can_create"], true);
    create_teacher(&mut a, &c);
    assert_eq!(ok(&mut a, "profiles", json!({}), &c)["can_create"], false);
}

#[test]
fn a_reviewer_cannot_use_teacher_or_child_commands() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    for name in [
        "teacher_work_build",
        "teacher_work_pdf",
        "teacher_plan_pdf",
        "teacher_topics",
        "teacher_home",
        "teacher_search",
        "teacher_bookmark",
        "teacher_lesson",
        "home",
        "session_start",
        "settings_get",
        "export_profile",
        "pack_install",
        "adult_status",
    ] {
        assert_eq!(
            call(&mut a, name, json!({}), &c).unwrap_err().code,
            "wrong_profile_kind",
            "{name}"
        );
    }
}

#[test]
fn the_overview_counts_units_per_topic_and_totals() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let o = ok(&mut a, "rv_overview", json!({}), &c);
    let packs = o["packs"].as_array().unwrap();
    assert!(packs
        .iter()
        .any(|p| p["pack"] == "ukrainian" && p["grade"] == 5));
    let grades: Vec<u64> = packs.iter().map(|p| p["grade"].as_u64().unwrap()).collect();
    assert!(grades.windows(2).all(|w| w[0] <= w[1]), "пакети за класами");
    let t = packs
        .iter()
        .flat_map(|p| p["topics"].as_array().unwrap())
        .find(|t| t["id"] == SKILL)
        .unwrap();
    assert!(t["total"].as_u64().unwrap() >= 20, "{t}");
    assert_eq!(t["done"], 0);
    assert_eq!(o["totals"]["notes"], 0);
    assert_eq!(o["totals"]["name"], "Оксана");
}

#[test]
fn units_separate_tasks_lesson_blocks_title_and_note_prompts() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let t = topic(&mut a, &c);
    let units: Vec<&str> = t["units"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["unit"].as_str().unwrap())
        .collect();
    assert!(units.contains(&"bank:0") && units.contains(&"lesson:0"));
    let mut sorted = units.clone();
    sorted.dedup();
    assert_eq!(sorted.len(), units.len(), "одиниці не повторюються");
    if units.contains(&"lesson:title") {
        let title = unit(&t, "lesson:title");
        assert!(title["fields"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f["field"] == "title"));
    }
    let b0 = unit(&t, "bank:0");
    let fields: Vec<&str> = b0["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields[0], "prompt");
    assert!(
        fields.contains(&"correct")
            && fields.contains(&"explanation")
            && fields.contains(&"hint:2")
    );
    assert_eq!(b0["status"], "none");
}

#[test]
fn marks_have_precedence_clear_and_topic_ok_only_touches_unmarked_units() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let ok_mark = json!({"skill": SKILL, "kind": "bank", "index": 0, "field": "*", "status": "ok"});
    note(&mut a, &c, ok_mark.clone()).unwrap();
    assert_eq!(unit(&topic(&mut a, &c), "bank:0")["status"], "ok");

    let fix = json!({"skill": SKILL, "kind": "bank", "index": 0, "field": "hint:0", "status": "fix", "correction": "Нова підказка", "comment": "ясніше"});
    note(&mut a, &c, fix).unwrap();
    assert_eq!(
        unit(&topic(&mut a, &c), "bank:0")["status"],
        "fix",
        "виправлення сильніше за «усе гаразд»"
    );
    let remove =
        json!({"skill": SKILL, "kind": "bank", "index": 0, "field": "prompt", "status": "remove"});
    note(&mut a, &c, remove).unwrap();
    assert_eq!(unit(&topic(&mut a, &c), "bank:0")["status"], "remove");

    // Знімаємо «прибрати», потім виправлення: лишається «усе гаразд».
    ok(
        &mut a,
        "rv_clear",
        json!({"skill": SKILL, "kind": "bank", "index": 0, "field": "remove"}),
        &c,
    );
    ok(
        &mut a,
        "rv_clear",
        json!({"skill": SKILL, "kind": "bank", "index": 0, "field": "hint:0"}),
        &c,
    );
    assert_eq!(unit(&topic(&mut a, &c), "bank:0")["status"], "ok");

    let before = ok(&mut a, "rv_overview", json!({}), &c)["totals"]["notes"]
        .as_u64()
        .unwrap();
    let r = ok(&mut a, "rv_topic_ok", json!({"skill": SKILL}), &c);
    let total = unit_count(&mut a, &c);
    assert_eq!(
        r["marked"].as_u64().unwrap() as usize,
        total - 1,
        "bank:0 уже позначено"
    );
    let after = ok(&mut a, "rv_overview", json!({}), &c)["totals"]["notes"]
        .as_u64()
        .unwrap();
    assert_eq!(after, before + r["marked"].as_u64().unwrap());
    let again = ok(&mut a, "rv_topic_ok", json!({"skill": SKILL}), &c);
    assert_eq!(again["marked"], 0, "повторно нічого не додає");
    let o = ok(&mut a, "rv_overview", json!({}), &c);
    let t = o["packs"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["topics"].as_array().unwrap())
        .find(|t| t["id"] == SKILL)
        .unwrap();
    assert_eq!(t["done"], t["total"]);
}

fn unit_count(a: &mut app_core::App, c: &app_core::Ctx) -> usize {
    topic(a, c)["units"].as_array().unwrap().len()
}

#[test]
fn invalid_marks_are_rejected() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    for bad in [
        json!({"skill": SKILL, "kind": "bank", "index": 0, "field": "prompt", "status": "fix"}), // без тексту
        json!({"skill": SKILL, "kind": "bank", "index": 0, "field": "prompt", "status": "maybe"}),
        json!({"skill": SKILL, "kind": "bank", "index": 9999, "field": "prompt", "status": "ok"}),
        json!({"skill": SKILL, "kind": "bank", "index": 9999, "field": "*", "status": "ok"}),
        json!({"skill": SKILL, "kind": "bank", "index": 0, "field": "correct", "status": "remove"}), // прибрати можна лише завдання
        json!({"skill": SKILL, "kind": "lesson", "index": 0, "field": "prompt", "status": "remove"}),
        json!({"skill": "NO-SUCH", "kind": "bank", "index": 0, "field": "prompt", "status": "ok"}),
    ] {
        assert!(note(&mut a, &c, bad.clone()).is_err(), "{bad}");
    }
    assert_eq!(
        ok(&mut a, "rv_overview", json!({}), &c)["totals"]["notes"],
        0
    );
}

#[test]
fn the_export_is_named_after_the_reviewer_and_date_and_applies_back_to_the_pack() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 20_646);
    create_teacher(&mut a, &c);
    assert!(
        call(&mut a, "rv_export", json!({}), &c).is_err(),
        "без позначок експорту немає"
    );

    let t = topic(&mut a, &c);
    let prompt = unit(&t, "bank:1")["fields"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();
    note(&mut a, &c, json!({"skill": SKILL, "kind": "bank", "index": 1, "field": "prompt", "status": "fix", "correction": "Виправлена умова", "comment": ""})).unwrap();
    note(
        &mut a,
        &c,
        json!({"skill": SKILL, "kind": "bank", "index": 2, "field": "*", "status": "ok"}),
    )
    .unwrap();
    note(
        &mut a,
        &c,
        json!({"skill": SKILL, "kind": "bank", "index": 3, "field": "prompt", "status": "remove"}),
    )
    .unwrap();
    note(
        &mut a,
        &c,
        json!({"skill": SKILL, "kind": "bank", "index": 4, "field": "hint:1", "status": "fix", "correction": "Нова підказка", "comment": ""}),
    )
    .unwrap();

    let ex = ok(&mut a, "rv_export", json!({}), &c);
    assert_eq!(ex["filename"], "vychytka-oksana-2026-07-12.csv");
    let only = ok(&mut a, "rv_export", json!({"skill": SKILL}), &c);
    assert_eq!(
        only["filename"],
        "vychytka-oksana-ua-apostrophe-2026-07-12.csv"
    );
    let csv =
        String::from_utf8(app_core::base64::decode(ex["data"].as_str().unwrap()).unwrap()).unwrap();
    let rows = content_tools::csv::read(&csv);
    assert_eq!(
        rows[0].join(","),
        "pack,skill,kind,index,field,text,status,correction,comment"
    );
    assert_eq!(rows.len(), 5);
    let fix = rows.iter().find(|r| r[6] == "fix").unwrap();
    assert_eq!(fix[5], prompt, "текст у файлі береться з пакета");

    // Той самий файл застосовується інструментом розробника: виправлення на місці, «усе гаразд» нічого не змінює, завдання прибрано.
    let mut packs: Vec<Value> = subjects::current_packs()
        .iter()
        .map(|p| serde_json::to_value(p).unwrap())
        .collect();
    let items_before = packs.iter().find(|p| p["id"] == fix[0].as_str()).unwrap()["banks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["skill"] == SKILL)
        .unwrap()["items"]
        .as_array()
        .unwrap()
        .len();
    let r = content_tools::review::apply(&mut packs, &csv);
    assert_eq!((r.fixed, r.removed), (2, 1), "{:?}", r.skipped);
    let bank = packs.iter().find(|p| p["id"] == fix[0].as_str()).unwrap()["banks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["skill"] == SKILL)
        .unwrap()
        .clone();
    assert_eq!(bank["items"][1]["prompt"], "Виправлена умова");
    assert_eq!(bank["items"].as_array().unwrap().len(), items_before - 1);
    // Індекси після видалення завдання №3 зсунулися: підказка завдання №4 тепер у №3.
    assert_eq!(
        bank["items"][3]["hints"][1], "Нова підказка",
        "виправлення підказки застосовується"
    );
}

#[test]
fn notes_survive_a_restart_and_stay_encrypted_per_profile() {
    let d = TestDir::new();
    let c = ctx(1_000, 1);
    {
        let mut a = app(&d);
        create_teacher(&mut a, &c);
        note(
            &mut a,
            &c,
            json!({"skill": SKILL, "kind": "bank", "index": 4, "field": "*", "status": "ok"}),
        )
        .unwrap();
    }
    let mut a = app(&d);
    let slot = ok(&mut a, "profiles", json!({}), &c)["profiles"][0]["slot"].clone();
    ok(
        &mut a,
        "open_profile",
        json!({"slot": slot, "pin": "1357"}),
        &c,
    );
    assert_eq!(unit(&topic(&mut a, &c), "bank:4")["status"], "ok");
}

#[test]
#[ignore = "вимірювання часу; запуск вручну: cargo test --release --features reviewer,test-support perf -- --ignored --nocapture"]
fn perf_overview_and_topic() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let t0 = std::time::Instant::now();
    ok(&mut a, "rv_overview", json!({}), &c);
    let first = t0.elapsed();
    let t1 = std::time::Instant::now();
    for _ in 0..5 {
        ok(&mut a, "rv_overview", json!({}), &c);
    }
    let overview = t1.elapsed() / 5;
    let t2 = std::time::Instant::now();
    for _ in 0..5 {
        ok(&mut a, "rv_topic", json!({"skill": SKILL}), &c);
    }
    let topic = t2.elapsed() / 5;
    let t3 = std::time::Instant::now();
    for i in 0..5u32 {
        note(
            &mut a,
            &c,
            json!({"skill": SKILL, "kind": "bank", "index": i, "field": "*", "status": "ok"}),
        )
        .unwrap();
    }
    let note_t = t3.elapsed() / 5;
    a.fill_review_notes_for_test(20_000);
    let t4 = std::time::Instant::now();
    note(
        &mut a,
        &c,
        json!({"skill": SKILL, "kind": "bank", "index": 9, "field": "*", "status": "ok"}),
    )
    .unwrap();
    let heavy = t4.elapsed();
    let t5 = std::time::Instant::now();
    ok(&mut a, "rv_overview", json!({}), &c);
    let heavy_overview = t5.elapsed();
    println!("перший огляд {first:?}, огляд {overview:?}, тема {topic:?}, позначка {note_t:?}; при 20000 позначок: позначка {heavy:?}, огляд {heavy_overview:?}");
}

#[test]
fn the_last_task_of_a_topic_cannot_be_removed_and_remove_does_not_erase_a_fix() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let t = topic(&mut a, &c);
    let tasks: Vec<u64> = t["units"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|u| u["kind"] == "bank")
        .map(|u| u["index"].as_u64().unwrap())
        .collect();
    // Виправлення умови й «прибрати» не затирають одне одного.
    note(&mut a, &c, json!({"skill": SKILL, "kind": "bank", "index": 0, "field": "prompt", "status": "fix", "correction": "Інша умова"})).unwrap();
    note(
        &mut a,
        &c,
        json!({"skill": SKILL, "kind": "bank", "index": 0, "field": "prompt", "status": "remove"}),
    )
    .unwrap();
    let u0 = topic(&mut a, &c);
    let prompt = &unit(&u0, "bank:0")["fields"][0];
    assert_eq!(prompt["note"]["status"], "fix", "виправлення лишилося");
    assert_eq!(unit(&u0, "bank:0")["status"], "remove");
    ok(
        &mut a,
        "rv_clear",
        json!({"skill": SKILL, "kind": "bank", "index": 0, "field": "remove"}),
        &c,
    );
    assert_eq!(unit(&topic(&mut a, &c), "bank:0")["status"], "fix");
    // Позначаємо всі завдання, крім одного, а останнє відхиляється.
    for i in &tasks[..tasks.len() - 1] {
        note(&mut a, &c, json!({"skill": SKILL, "kind": "bank", "index": i, "field": "prompt", "status": "remove"})).unwrap();
    }
    let last = tasks[tasks.len() - 1];
    let e = note(&mut a, &c, json!({"skill": SKILL, "kind": "bank", "index": last, "field": "prompt", "status": "remove"})).unwrap_err();
    assert!(e.message.contains("хоча б одне"), "{}", e.message);
}

#[test]
fn marks_on_changed_content_are_stale_and_not_exported() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    note(
        &mut a,
        &c,
        json!({"skill": SKILL, "kind": "bank", "index": 2, "field": "*", "status": "ok"}),
    )
    .unwrap();
    note(&mut a, &c, json!({"skill": SKILL, "kind": "bank", "index": 1, "field": "prompt", "status": "fix", "correction": "Інша умова"})).unwrap();
    // Імітуємо оновлення вмісту: у збереженій позначці інший відбиток і інший початковий текст.
    {
        let o = ok(&mut a, "rv_overview", json!({}), &c);
        assert_eq!(o["totals"]["stale"], 0);
    }
    a.tamper_review_notes_for_test(|n| {
        for x in n.iter_mut() {
            x.text = format!("{}-змінено", x.text);
        }
    });
    let o = ok(&mut a, "rv_overview", json!({}), &c);
    assert_eq!(o["totals"]["stale"], 2);
    assert_eq!(o["totals"]["notes"], 0);
    assert_eq!(unit(&topic(&mut a, &c), "bank:2")["status"], "none");
    assert!(
        call(&mut a, "rv_export", json!({}), &c).is_err(),
        "застарілі позначки не експортуються"
    );
}

#[test]
fn topic_ok_refuses_when_there_is_no_room_and_changes_nothing() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    a.fill_review_notes_for_test(storage::state::MAX_REVIEW_NOTES - 3);
    let before = ok(&mut a, "rv_overview", json!({}), &c)["totals"]["notes"].clone();
    assert!(call(&mut a, "rv_topic_ok", json!({"skill": SKILL}), &c).is_err());
    assert_eq!(
        ok(&mut a, "rv_overview", json!({}), &c)["totals"]["notes"],
        before,
        "нічого не додано частково"
    );
}

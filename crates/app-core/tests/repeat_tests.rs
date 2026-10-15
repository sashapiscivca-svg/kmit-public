#![cfg(not(feature = "reviewer"))]
mod common;

use common::{app, create_and_open, ctx, ok, TestDir};
use serde_json::{json, Value};

fn answer_right(a: &mut app_core::App, t: u64, day: u32) -> Value {
    let right = a.peek_answer().unwrap();
    ok(a, "answer", json!({"input": right}), &ctx(t, day))
}

/// Опановує тему за один сеанс «я це вже вмію» у день `day` (три чисті відповіді).
fn master_by_testout(a: &mut app_core::App, skill: &str, day: u32) -> u64 {
    let mut now = u64::from(day) * 86_400 + 9 * 3_600;
    let r = ok(a, "testout_start", json!({"skill": skill}), &ctx(now, day));
    assert_eq!(r["model"]["phase"], "task");
    for _ in 0..3 {
        now += 15;
        answer_right(a, now, day);
        now += 2;
        ok(a, "continue", json!({}), &ctx(now, day));
    }
    now
}

fn tasks_due(o: &Value) -> u64 {
    o["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["tasks_due"].as_u64().unwrap())
        .sum()
}

#[test]
fn review_mode_brings_back_a_mastered_topic_when_due_and_spaces_it_further_after_a_clean_review() {
    let d = TestDir::new();
    let mut a = app(&d);
    create_and_open(&mut a, &ctx(86_400 + 9 * 3_600, 1));
    master_by_testout(&mut a, "D-ADD", 1);
    // Того ж дня повторювати ще нічого.
    let o = ok(
        &mut a,
        "review_overview",
        json!({}),
        &ctx(86_400 + 12 * 3_600, 1),
    );
    assert_eq!(o["total_due"], 0);
    assert_eq!(o["tracked"], 1);
    // Наступного дня тема в боргу, пам'ять частково згасла.
    let day2 = 2 * 86_400 + 9 * 3_600;
    let o = ok(&mut a, "review_overview", json!({}), &ctx(day2, 2));
    let math = o["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "math")
        .unwrap()
        .clone();
    assert_eq!(math["scheduled"][0]["skill"], "D-ADD");
    assert!(math["scheduled"][0]["memory"].as_u64().unwrap() < 50);
    // План на день починається з повторення.
    let p = ok(&mut a, "plan", json!({}), &ctx(day2, 2));
    assert_eq!(p["blocks"][0]["kind"], "review");
    assert_eq!(p["blocks"][0]["subject"], "math");
    // Режим повторення дає задачі саме з цієї теми.
    let r = ok(
        &mut a,
        "review_start",
        json!({"subject": "math"}),
        &ctx(day2 + 5, 2),
    );
    assert_eq!(r["model"]["phase"], "task");
    let mut now = day2 + 5;
    let mut asked = 0;
    loop {
        now += 15;
        let r = answer_right(&mut a, now, 2);
        assert_eq!(r["model"]["phase"], "feedback");
        asked += 1;
        now += 2;
        let c = ok(&mut a, "continue", json!({}), &ctx(now, 2));
        if c["model"]["phase"] != "task" {
            break;
        }
        assert!(asked < 5, "повторення не зупиняється");
    }
    assert_eq!(
        asked, 1,
        "одна чиста відповідь закриває борг теми, повторення завершується"
    );
    // Після чистого повторення борг закрито, наступне через кілька днів.
    let o = ok(&mut a, "review_overview", json!({}), &ctx(now + 60, 2));
    assert_eq!(tasks_due(&o), 0);
    let o = ok(
        &mut a,
        "review_overview",
        json!({}),
        &ctx(3 * 86_400 + 9 * 3_600, 3),
    );
    assert_eq!(tasks_due(&o), 0, "наступний інтервал довший за день");
    let o = ok(
        &mut a,
        "review_overview",
        json!({}),
        &ctx(6 * 86_400 + 9 * 3_600, 6),
    );
    assert_eq!(tasks_due(&o), 1, "через 3 дні тема знову в плані");
    // Головна й екран повторення показують одне й те саме число тем.
    let home = ok(&mut a, "home", json!({}), &ctx(6 * 86_400 + 9 * 3_600, 6));
    assert_eq!(home["review_due_total"], o["total_due"]);
    assert!(o["total_due"].as_u64().unwrap() >= 1);
}

#[test]
fn the_profile_labels_topics_and_the_plan_holds_back_new_material_after_bad_days() {
    let d = TestDir::new();
    let mut a = app(&d);
    create_and_open(&mut a, &ctx(86_400 + 9 * 3_600, 1));
    master_by_testout(&mut a, "D-ADD", 1);
    let day3 = 3 * 86_400 + 9 * 3_600;
    let p = ok(&mut a, "profile", json!({"subject": "math"}), &ctx(day3, 3));
    let t = p["topics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["skill"] == "D-ADD")
        .unwrap()
        .clone();
    assert_eq!(t["label"], "repeat");
    assert_eq!(p["summary"]["repeat"], 1);
    assert!(
        ok(&mut a, "profile", json!({"subject": "nope"}), &ctx(day3, 3))["topics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn the_teacher_sheet_is_per_subject_and_lists_what_is_due_for_repetition() {
    let d = TestDir::new();
    let mut a = app(&d);
    create_and_open(&mut a, &ctx(86_400 + 9 * 3_600, 1));
    master_by_testout(&mut a, "D-ADD", 1);
    let day3 = 3 * 86_400 + 9 * 3_600;
    let m = ok(&mut a, "sheet", json!({"subject": "math"}), &ctx(day3, 3));
    assert_eq!(m["subject"], "Математика");
    assert_eq!(m["mastered"][0]["skill"], "D-ADD");
    assert_eq!(m["to_repeat"][0]["skill"], "D-ADD");
    // Аркуш з іншого предмета не містить тем математики.
    let u = ok(
        &mut a,
        "sheet",
        json!({"subject": "ukrainian"}),
        &ctx(day3, 3),
    );
    assert_eq!(u["subject"], "Українська мова");
    assert!(u["mastered"].as_array().unwrap().is_empty());
    assert!(u["to_repeat"].as_array().unwrap().is_empty());
    let pdf = ok(
        &mut a,
        "sheet_pdf",
        json!({"subject": "ukrainian"}),
        &ctx(day3, 3),
    );
    assert!(pdf["filename"].as_str().unwrap().contains("ukrainian"));
    assert!(common::call(&mut a, "sheet", json!({"subject": "nope"}), &ctx(day3, 3)).is_err());
}

/// Складний приклад, показаний дорослому, повертається завтра, потім за інтервалами, доки не вийде чисто п'ять разів.
#[test]
fn an_example_shown_to_an_adult_comes_back_on_a_schedule_until_it_is_conquered() {
    let d = TestDir::new();
    let mut a = app(&d);
    let day1 = 86_400 + 9 * 3_600;
    create_and_open(&mut a, &ctx(day1, 1));
    let r = ok(
        &mut a,
        "practice_start",
        json!({"skill": "D-ADD"}),
        &ctx(day1, 1),
    );
    let statement = r["model"]["statement"].as_str().unwrap().to_string();
    ok(&mut a, "hand_to_adult", json!({}), &ctx(day1 + 20, 1));
    ok(&mut a, "end_session", json!({}), &ctx(day1 + 30, 1));
    // Складний приклад збережено знеособлено: це лише тема, номер варіанта й лічильники.
    let g = ok(
        &mut a,
        "memory_graph",
        json!({"subject": "math"}),
        &ctx(day1 + 40, 1),
    );
    assert_eq!(g["gaps"][0]["statement"], statement);
    assert_eq!(g["gaps"][0]["kind"], "asked_adult");
    assert_eq!(g["closed_total"], 0);
    // Показаний дорослому приклад повертається не раніше завтра, дні повернення: +1, +1, +3, +7, +14.
    let mut day = 1u32;
    for (i, gap) in [1u32, 1, 3, 7, 14].into_iter().enumerate() {
        day += gap;
        let now = u64::from(day) * 86_400 + 9 * 3_600;
        let o = ok(&mut a, "review_overview", json!({}), &ctx(now, day));
        let math = o["subjects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == "math")
            .unwrap()
            .clone();
        assert_eq!(math["tasks_due"], 1, "день {day}");
        let r = ok(
            &mut a,
            "review_start",
            json!({"subject": "math"}),
            &ctx(now + 5, day),
        );
        assert_eq!(
            r["model"]["statement"], statement,
            "повертається саме цей приклад"
        );
        assert_eq!(r["model"]["hard_return"], true);
        let r = answer_right(&mut a, now + 20, day);
        assert_eq!(r["model"]["phase"], "feedback");
        let want = if i == 4 { "closed" } else { "better" };
        assert_eq!(r["model"]["gap"], want, "крок {i}");
        ok(&mut a, "end_session", json!({}), &ctx(now + 30, day));
    }
    let g = ok(
        &mut a,
        "memory_graph",
        json!({"subject": "math"}),
        &ctx(day as u64 * 86_400 + 12 * 3_600, day),
    );
    assert!(g["gaps"].as_array().unwrap().is_empty());
    assert_eq!(g["closed_total"], 1);
    assert_eq!(g["closed"][0]["skill"], "D-ADD");
}

/// Приклад, який вдруге не вийшов, не губиться: прогалина лишається й повернеться знову.
#[test]
fn a_miss_on_a_returning_example_keeps_the_gap_open() {
    let d = TestDir::new();
    let mut a = app(&d);
    let day1 = 86_400 + 9 * 3_600;
    create_and_open(&mut a, &ctx(day1, 1));
    ok(
        &mut a,
        "practice_start",
        json!({"skill": "D-ADD"}),
        &ctx(day1, 1),
    );
    ok(&mut a, "hand_to_adult", json!({}), &ctx(day1 + 20, 1));
    ok(&mut a, "end_session", json!({}), &ctx(day1 + 30, 1));
    let now = 2 * 86_400 + 9 * 3_600;
    let r = ok(
        &mut a,
        "review_start",
        json!({"subject": "math"}),
        &ctx(now, 2),
    );
    assert_eq!(r["model"]["hard_return"], true);
    let r = ok(&mut a, "answer", json!({"input": "0"}), &ctx(now + 10, 2));
    assert_ne!(r["model"]["gap"], "closed");
    ok(&mut a, "end_session", json!({}), &ctx(now + 60, 2));
    let g = ok(
        &mut a,
        "memory_graph",
        json!({"subject": "math"}),
        &ctx(now + 90, 2),
    );
    assert_eq!(g["gaps"].as_array().unwrap().len(), 1);
}

/// Кілька хибних спроб, хоч і з правильним фіналом, теж роблять приклад «складним» і він повертатиметься.
#[test]
fn a_task_solved_only_after_wrong_attempts_is_remembered_as_struggled() {
    let d = TestDir::new();
    let mut a = app(&d);
    let day1 = 86_400 + 9 * 3_600;
    create_and_open(&mut a, &ctx(day1, 1));
    ok(
        &mut a,
        "practice_start",
        json!({"skill": "D-ADD"}),
        &ctx(day1, 1),
    );
    ok(
        &mut a,
        "answer",
        json!({"input": "999999"}),
        &ctx(day1 + 10, 1),
    );
    let r = answer_right(&mut a, day1 + 20, 1);
    assert_eq!(r["model"]["phase"], "feedback");
    assert_eq!(r["model"]["gap"], "opened");
    ok(&mut a, "end_session", json!({}), &ctx(day1 + 30, 1));
    let g = ok(
        &mut a,
        "memory_graph",
        json!({"subject": "math"}),
        &ctx(day1 + 40, 1),
    );
    assert_eq!(g["gaps"][0]["kind"], "struggled");
    // Чистий прохід першого ж разу не створює прогалини.
    ok(
        &mut a,
        "practice_start",
        json!({"skill": "D-SUB"}),
        &ctx(day1 + 100, 1),
    );
    let r = answer_right(&mut a, day1 + 110, 1);
    assert!(r["model"]["gap"].is_null());
}

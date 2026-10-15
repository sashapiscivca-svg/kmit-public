#![cfg(not(feature = "reviewer"))]
mod common;

use app_core::App;
use common::{app, call, create_and_open, create_and_open_raw, ctx, ok, TestDir};
use serde_json::{json, Value};

const DAY: u64 = 86_400;

fn answer_right(a: &mut App, now: u64, day: u32) -> Value {
    let right = a.peek_answer().unwrap();
    ok(a, "answer", json!({"input": right}), &ctx(now, day))
}

fn answer_wrong_choice(a: &mut App, now: u64, day: u32) -> Value {
    let right: usize = a.peek_answer().unwrap().parse().unwrap();
    let wrong = if right == 1 { 2 } else { 1 };
    ok(
        a,
        "answer",
        json!({"input": wrong.to_string()}),
        &ctx(now, day),
    )
}

#[test]
fn a_new_topic_offers_the_explanation_first_and_only_once() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open_raw(&mut a, &ctx(t, 1));
    let m = ok(&mut a, "session_start", json!({}), &ctx(t, 1))["model"].clone();
    assert_eq!(m["phase"], "intro");
    assert_eq!(m["skill_id"], "N-ADD-MULTI");
    assert_eq!(m["has_lesson"], true);
    assert!(m.get("statement").is_none(), "задачі ще немає");
    // Поки задачі немає, урок відкритий.
    ok(
        &mut a,
        "lesson",
        json!({"skill": "N-ADD-MULTI"}),
        &ctx(t + 5, 1),
    );
    let m = ok(&mut a, "intro_continue", json!({}), &ctx(t + 20, 1))["model"].clone();
    assert_eq!(m["phase"], "task");
    answer_right(&mut a, t + 30, 1);
    // Далі в цій темі пояснення вже не пропонується.
    let m = ok(&mut a, "continue", json!({}), &ctx(t + 40, 1))["model"].clone();
    assert_eq!(m["phase"], "task");
    // Команда поза фазою вступу — помилка.
    assert_eq!(
        call(&mut a, "intro_continue", json!({}), &ctx(t + 50, 1))
            .unwrap_err()
            .code,
        "wrong_phase"
    );
}

#[test]
fn the_offer_can_be_switched_off_and_free_practice_never_shows_it() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open_raw(&mut a, &ctx(t, 1));
    let m = ok(
        &mut a,
        "practice_start",
        json!({"skill": "D-ADD"}),
        &ctx(t, 1),
    )["model"]
        .clone();
    assert_eq!(m["phase"], "task", "вільна практика без вступу");
    ok(&mut a, "end_session", json!({}), &ctx(t + 5, 1));
    ok(
        &mut a,
        "settings_set",
        json!({"lesson_first": false}),
        &ctx(t + 6, 1),
    );
    let m = ok(&mut a, "session_start", json!({}), &ctx(t + 7, 1))["model"].clone();
    assert_eq!(m["phase"], "task");
}

#[test]
fn passing_a_test_out_masters_the_topic_without_hints_and_failing_does_not() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 3 * DAY + 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 3));
    let r = ok(
        &mut a,
        "testout_start",
        json!({"skill": "N-ADD-MULTI"}),
        &ctx(t, 3),
    );
    let m = &r["model"];
    assert_eq!(m["phase"], "task");
    assert_eq!(m["hints_allowed"], false);
    assert_eq!(m["testout"]["total"], 3);
    let h = ok(&mut a, "hint", json!({}), &ctx(t + 100, 3));
    assert_eq!(h["hint"]["kind"], "none");
    let mut now = t;
    for i in 0..3 {
        now += 15;
        let r = answer_right(&mut a, now, 3);
        assert_eq!(r["model"]["phase"], "feedback", "крок {i}");
        now += 2;
        let c = ok(&mut a, "continue", json!({}), &ctx(now, 3));
        if i < 2 {
            assert_eq!(c["model"]["phase"], "task");
            assert_eq!(c["model"]["testout"]["asked"], i + 1);
        } else {
            assert_eq!(c["model"]["phase"], "ended");
            assert_eq!(c["model"]["testout"]["passed"], true);
        }
    }
    let home = ok(&mut a, "home", json!({}), &ctx(now + 5, 3));
    let st = home["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "N-ADD-MULTI")
        .unwrap()["state"]
        .clone();
    assert_eq!(st, "mastered");
    let album = ok(&mut a, "album", json!({}), &ctx(now + 6, 3));
    assert!(album["cells"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["skill"] == "N-ADD-MULTI" && c["kind"] == "mastery" && c["earned"] == true));

    // Невдача: помилка одразу завершує перевірку, тема не опановується.
    let r = ok(
        &mut a,
        "testout_start",
        json!({"skill": "N-SUB-MULTI"}),
        &ctx(now + 100, 3),
    );
    assert_eq!(r["model"]["phase"], "task");
    let r = ok(&mut a, "answer", json!({"input": "0"}), &ctx(now + 110, 3));
    assert_eq!(r["model"]["phase"], "feedback");
    // Хибна відповідь у перевірці «я це вже вмію» не створює «прогалини» (правильне число, за І1, не показуємо).
    let g = ok(
        &mut a,
        "memory_graph",
        json!({"subject": "math"}),
        &ctx(now + 112, 3),
    );
    assert!(g["gaps"].as_array().unwrap().is_empty());
    let c = ok(&mut a, "continue", json!({}), &ctx(now + 115, 3));
    assert_eq!(c["model"]["phase"], "ended");
    assert_eq!(c["model"]["testout"]["passed"], false);
    let home = ok(&mut a, "home", json!({}), &ctx(now + 120, 3));
    let st = home["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "N-SUB-MULTI")
        .unwrap()["state"]
        .clone();
    assert_ne!(st, "mastered");
}

#[test]
fn placement_proposes_the_first_available_unmastered_topic() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    let p = ok(
        &mut a,
        "placement_next",
        json!({"subject": "math"}),
        &ctx(t, 1),
    );
    assert_eq!(p["skill"], "N-ADD-MULTI");
    assert_eq!(p["title"], "Додавання багатоцифрових чисел");
    assert_eq!(
        call(
            &mut a,
            "placement_next",
            json!({"subject": "nope"}),
            &ctx(t, 1)
        )
        .unwrap_err()
        .code,
        "bad_request"
    );
}

#[test]
fn repeated_misses_in_one_topic_suggest_the_lesson_and_a_long_streak_suggests_a_break() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    ok(
        &mut a,
        "session_start",
        json!({"subject": "ukrainian"}),
        &ctx(t, 1),
    );
    let mut now = t;
    let mut supports = Vec::new();
    for _ in 0..5 {
        now += 10;
        let r = answer_wrong_choice(&mut a, now, 1);
        supports.push(r["model"]["support"].clone());
        now += 2;
        ok(&mut a, "continue", json!({}), &ctx(now, 1));
    }
    assert_eq!(supports[0], Value::Null);
    assert_eq!(supports[1], Value::Null);
    assert_eq!(supports[2], "lesson", "{supports:?}");
    assert_eq!(supports[3], "lesson");
    // Правильна відповідь обнуляє серію.
    now += 10;
    let r = answer_right(&mut a, now, 1);
    assert_eq!(r["model"]["support"], Value::Null);
}

#[test]
fn a_missed_bank_item_comes_back_after_a_few_other_tasks() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    let first = ok(
        &mut a,
        "session_start",
        json!({"subject": "ukrainian"}),
        &ctx(t, 1),
    )["model"]
        .clone();
    let missed_statement = first["statement"].as_str().unwrap().to_string();
    let mut now = t + 10;
    answer_wrong_choice(&mut a, now, 1);
    let mut seen_again = None;
    for i in 0..14 {
        now += 5;
        let m = ok(&mut a, "continue", json!({}), &ctx(now, 1))["model"].clone();
        if m["statement"].as_str() == Some(missed_statement.as_str()) {
            seen_again = Some(i);
            break;
        }
        now += 5;
        answer_right(&mut a, now, 1);
    }
    let i = seen_again.expect("пропущене завдання не повернулося");
    assert!(
        i >= 2,
        "повертається не одразу, а після кількох інших (було {i})"
    );
}

#[test]
fn anonymous_item_statistics_carry_no_identity() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    ok(
        &mut a,
        "session_start",
        json!({"subject": "ukrainian"}),
        &ctx(t, 1),
    );
    answer_right(&mut a, t + 10, 1);
    ok(&mut a, "continue", json!({}), &ctx(t + 12, 1));
    answer_wrong_choice(&mut a, t + 20, 1);
    let r = ok(&mut a, "stats_export", json!({}), &ctx(t + 30, 1));
    assert_eq!(r["filename"], "kmit-statystyka.json");
    let raw = r["data"].as_str().unwrap();
    assert!(raw.len() > 10);
    let items = r["preview"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    let text = r.to_string();
    assert!(!text.contains("Мирослава"), "нікнейма немає");
    assert!(items
        .iter()
        .all(|i| i["attempts"].as_u64().unwrap() >= i["correct"].as_u64().unwrap()));
}

#[test]
fn home_suggests_what_to_do_today_without_forcing_it() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    let h = ok(&mut a, "home", json!({}), &ctx(t, 1));
    let s = &h["suggestion"];
    assert_eq!(s["reason"], "new");
    assert!(s["subject"].is_string() && s["skill_title"].is_string());
}

#[test]
fn a_child_can_flag_a_topic_as_hard_and_it_reaches_the_teacher_sheet() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    ok(
        &mut a,
        "reflect",
        json!({"skill": "F-ADD-SAME"}),
        &ctx(t, 1),
    );
    ok(
        &mut a,
        "reflect",
        json!({"skill": "F-ADD-SAME"}),
        &ctx(t + 1, 1),
    );
    let sheet = ok(&mut a, "sheet", json!({}), &ctx(t + 2, 1));
    let asks = sheet["ask_adult"].as_array().unwrap();
    assert_eq!(asks.len(), 1, "повторне позначення не дублюється");
    assert!(asks[0]["statement"].as_str().unwrap().contains("складною"));
    assert_eq!(
        call(&mut a, "reflect", json!({"skill": "NOPE"}), &ctx(t, 1))
            .unwrap_err()
            .code,
        "bad_request"
    );
}

#[test]
fn a_typed_answer_gets_one_retry_with_a_length_hint_then_the_right_word_is_shown() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    let m = ok(
        &mut a,
        "practice_start",
        json!({"skill": "UA-T-APOSTROPHE"}),
        &ctx(t, 1),
    )["model"]
        .clone();
    assert_eq!(m["answer_kind"], "text");
    assert_eq!(m["phase"], "task");
    // Хибне слово: перша спроба повторна, з кількістю літер (без самого слова).
    let r = ok(&mut a, "answer", json!({"input": "ххх"}), &ctx(t + 5, 1));
    assert_eq!(r["result"], "try_again");
    assert_eq!(r["text_retry"], true);
    assert!(r["letters"].as_u64().unwrap() >= 3);
    // Друга хибна: відгук із правильним словом і поясненням.
    let r = ok(&mut a, "answer", json!({"input": "ууу"}), &ctx(t + 8, 1));
    assert_eq!(r["result"], "missed");
    assert_eq!(r["model"]["phase"], "feedback");
    assert!(
        r["model"]["reveal"]["answer"]
            .as_str()
            .unwrap()
            .chars()
            .count()
            >= 3
    );
    // Наступне: правильне слово з іншим регістром і зайвим пробілом приймається.
    ok(&mut a, "continue", json!({}), &ctx(t + 10, 1));
    let right = a.peek_text_answer().unwrap();
    let r = ok(
        &mut a,
        "answer",
        json!({"input": format!("  {}  ", right.to_uppercase())}),
        &ctx(t + 20, 1),
    );
    assert_eq!(r["result"], "correct");
    assert_eq!(r["model"]["outcome"], "solved");
}

#[test]
fn a_child_can_report_a_broken_task_and_the_list_exports_without_identity() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    let m = ok(
        &mut a,
        "session_start",
        json!({"subject": "ukrainian"}),
        &ctx(t, 1),
    )["model"]
        .clone();
    let statement = m["statement"].as_str().unwrap().to_string();
    // Під час задачі: «умова незрозуміла».
    let r = ok(
        &mut a,
        "flag_item",
        json!({"reason": "unclear"}),
        &ctx(t + 3, 1),
    );
    assert_eq!(r["count"], 1);
    // Повторне повідомлення про те саме завдання не дублюється.
    let r = ok(
        &mut a,
        "flag_item",
        json!({"reason": "unclear"}),
        &ctx(t + 4, 1),
    );
    assert_eq!(r["count"], 1);
    // Після відповіді (відгук): «відповідь неправильна».
    answer_wrong_choice(&mut a, t + 10, 1);
    let r = ok(
        &mut a,
        "flag_item",
        json!({"reason": "wrong_answer"}),
        &ctx(t + 12, 1),
    );
    assert_eq!(
        r["count"], 1,
        "те саме завдання, інша причина: оновлюється, не множиться"
    );
    assert_eq!(
        call(
            &mut a,
            "flag_item",
            json!({"reason": "???"}),
            &ctx(t + 13, 1)
        )
        .unwrap_err()
        .code,
        "bad_request"
    );
    let e = ok(&mut a, "flags_export", json!({}), &ctx(t + 20, 1));
    assert_eq!(e["filename"], "kmit-pomylky.json");
    let items = e["preview"]["flags"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["reason"], "wrong_answer");
    assert_eq!(items[0]["statement"], statement.as_str());
    assert!(!e.to_string().contains("Мирослава"));
    // Поза задачею повідомляти нема про що.
    ok(&mut a, "end_session", json!({}), &ctx(t + 30, 1));
    assert_eq!(
        call(
            &mut a,
            "flag_item",
            json!({"reason": "unclear"}),
            &ctx(t + 31, 1)
        )
        .unwrap_err()
        .code,
        "wrong_phase"
    );
}

#[test]
fn repeated_struggle_points_to_the_weak_prerequisites_first() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    // F-ADD-DIFF вимагає F-ADD-SAME, а вона ще не опанована.
    ok(
        &mut a,
        "practice_start",
        json!({"skill": "F-ADD-DIFF"}),
        &ctx(t, 1),
    );
    let mut support = Value::Null;
    let mut weak = Value::Null;
    for i in 0..3u64 {
        let r = ok(&mut a, "hand_to_adult", json!({}), &ctx(t + 10 + i * 10, 1));
        support = r["model"]["support"].clone();
        weak = r["model"]["weak_prerequisites"].clone();
        ok(&mut a, "continue", json!({}), &ctx(t + 15 + i * 10, 1));
    }
    assert_eq!(support, "prereq", "{support:?}");
    let ids: Vec<&str> = weak
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"F-ADD-SAME"), "{ids:?}");
    assert!(weak[0]["title"].is_string());
    // Опанована передумова в список не потрапляє.
    ok(&mut a, "end_session", json!({}), &ctx(t + 60, 1));
    ok(
        &mut a,
        "testout_start",
        json!({"skill": "F-ADD-SAME"}),
        &ctx(t + 70, 1),
    );
    let mut now = t + 70;
    for _ in 0..3 {
        now += 10;
        answer_right(&mut a, now, 1);
        now += 2;
        ok(&mut a, "continue", json!({}), &ctx(now, 1));
    }
    ok(
        &mut a,
        "practice_start",
        json!({"skill": "F-ADD-DIFF"}),
        &ctx(now + 5, 1),
    );
    let mut weak_after = Value::Null;
    for i in 0..3u64 {
        let r = ok(
            &mut a,
            "hand_to_adult",
            json!({}),
            &ctx(now + 10 + i * 10, 1),
        );
        weak_after = r["model"]["weak_prerequisites"].clone();
        ok(&mut a, "continue", json!({}), &ctx(now + 15 + i * 10, 1));
    }
    let ids: Vec<&str> = weak_after.as_array().map_or(vec![], |v| {
        v.iter().map(|w| w["id"].as_str().unwrap()).collect()
    });
    assert!(!ids.contains(&"F-ADD-SAME"), "{ids:?}");
}

#[test]
fn my_week_counts_activity_by_day_and_shows_progress_to_the_childs_own_goal() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t + 10 * DAY, 10));
    ok(
        &mut a,
        "settings_set",
        json!({"weekly_goal_days": 3}),
        &ctx(t + 10 * DAY, 10),
    );
    for (day, n) in [(8u32, 2u32), (10, 3)] {
        let base = t + u64::from(day) * DAY;
        ok(
            &mut a,
            "practice_start",
            json!({"skill": "N-ADD-MULTI"}),
            &ctx(base, day),
        );
        for i in 0..n {
            answer_right(&mut a, base + 10 + u64::from(i) * 5, day);
            ok(
                &mut a,
                "continue",
                json!({}),
                &ctx(base + 12 + u64::from(i) * 5, day),
            );
        }
        ok(&mut a, "end_session", json!({}), &ctx(base + 100, day));
    }
    let w = ok(&mut a, "week", json!({}), &ctx(t + 10 * DAY + 500, 10));
    let days = w["days"].as_array().unwrap();
    assert_eq!(days.len(), 7);
    assert_eq!(days[6]["tasks"], 3, "сьогодні");
    assert_eq!(days[4]["tasks"], 2, "позавчора");
    assert_eq!(w["active_days"], 2);
    assert_eq!(w["goal"], 3);
    assert_eq!(w["tasks_total"], 5);
    // Ціль лише порада: обмежується 1..=7, 0 вимикає.
    let s = ok(
        &mut a,
        "settings_set",
        json!({"weekly_goal_days": 99}),
        &ctx(t, 10),
    );
    assert_eq!(s["weekly_goal_days"], 7);
}

#[test]
fn a_gentle_reminder_to_back_up_appears_after_real_use_and_can_be_snoozed() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    assert_eq!(
        ok(&mut a, "home", json!({}), &ctx(t, 1))["backup_hint"],
        false,
        "нового профілю не лякаємо"
    );
    ok(
        &mut a,
        "practice_start",
        json!({"skill": "N-ADD-MULTI"}),
        &ctx(t, 1),
    );
    for i in 0..30u64 {
        answer_right(&mut a, t + 10 + i * 5, 1);
        ok(&mut a, "continue", json!({}), &ctx(t + 12 + i * 5, 1));
    }
    ok(&mut a, "end_session", json!({}), &ctx(t + 400, 1));
    assert_eq!(
        ok(&mut a, "home", json!({}), &ctx(t + 401, 1))["backup_hint"],
        true
    );
    ok(&mut a, "backup_snooze", json!({}), &ctx(t + 402, 1));
    assert_eq!(
        ok(&mut a, "home", json!({}), &ctx(t + 403, 1))["backup_hint"],
        false
    );
    // Через тиждень знову нагадується, поки копії немає.
    let later = t + 9 * DAY;
    assert_eq!(
        ok(&mut a, "home", json!({}), &ctx(later, 10))["backup_hint"],
        true
    );
    // Копія профілю знімає нагадування на два тижні.
    ok(
        &mut a,
        "adult_set_pin",
        json!({"pin": "2468"}),
        &ctx(later, 10),
    );
    ok(
        &mut a,
        "export_profile",
        json!({"passphrase": "фраза для копії"}),
        &ctx(later + 1, 10),
    );
    assert_eq!(
        ok(&mut a, "home", json!({}), &ctx(later + 2, 10))["backup_hint"],
        false
    );
    assert_eq!(
        ok(&mut a, "home", json!({}), &ctx(later + 14 * DAY + 5, 24))["backup_hint"],
        true
    );
}

#[test]
fn a_signed_content_pack_installs_and_a_forged_or_older_one_does_not() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    // Пакет без підпису чи з чужим підписом не встановлюється.
    let pack = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../subjects/packs/history.json"
    ))
    .unwrap();
    let b64 = |b: &[u8]| app_core::base64::encode(b);
    let e = call(
        &mut a,
        "pack_install",
        json!({"data": b64(&pack), "sig": b64(&[0u8; 64])}),
        &ctx(t, 1),
    )
    .unwrap_err();
    assert_eq!(e.code, "pack_rejected");
    let e = call(
        &mut a,
        "pack_install",
        json!({"data": "%%%", "sig": "x"}),
        &ctx(t, 1),
    )
    .unwrap_err();
    assert_eq!(e.code, "bad_request");
    // Список пакетів показує вбудовані версії.
    let l = ok(&mut a, "packs_list", json!({}), &ctx(t, 1));
    assert!(l["packs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["id"] == "history" && p["version"].as_u64().unwrap() >= 1));
}

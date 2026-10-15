#![cfg(not(feature = "reviewer"))]
mod common;

use common::{app, call, create_and_open, ctx, ok, TestDir};
use serde_json::{json, Value};

const DAY: u64 = 86_400;

fn start_ukrainian(a: &mut app_core::App, now: u64, day: u32) -> Value {
    ok(
        a,
        "session_start",
        json!({"subject": "ukrainian"}),
        &ctx(now, day),
    )["model"]
        .clone()
}

#[test]
fn the_home_screen_lists_subjects_with_progress_and_the_new_subject_is_available() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(9 * 3_600, 1);
    create_and_open(&mut a, &c);
    let home = ok(&mut a, "home", json!({}), &c);
    let subs = home["subjects"].as_array().unwrap();
    assert_eq!(subs[0]["id"], "math");
    let ua = subs.iter().find(|s| s["id"] == "ukrainian").unwrap();
    assert_eq!(ua["title"], "Українська мова");
    assert_eq!(ua["nothing_left"], false);
    assert!(home["skills"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["id"] == "UA-APOSTROPHE" && s["subject"] == "ukrainian"));
    assert_eq!(
        call(&mut a, "session_start", json!({"subject": "nope"}), &c)
            .unwrap_err()
            .code,
        "bad_request"
    );
}

#[test]
fn a_choice_task_shows_options_and_a_miss_reveals_the_answer_with_an_explanation() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    let m = start_ukrainian(&mut a, t, 1);
    assert_eq!(m["phase"], "task");
    let choices = m["choices"].as_array().unwrap();
    assert!(choices.len() >= 3);
    assert!(m.get("reveal").is_none());
    // Хибна відповідь: одразу відгук із правильним варіантом і поясненням, без вгадування далі.
    let right: usize = a.peek_answer().unwrap().parse().unwrap();
    let wrong = if right == 1 { 2 } else { 1 };
    let r = ok(
        &mut a,
        "answer",
        json!({"input": wrong.to_string()}),
        &ctx(t + 10, 1),
    );
    assert_eq!(r["result"], "missed");
    let m = &r["model"];
    assert_eq!(m["phase"], "feedback");
    assert_eq!(m["outcome"], "missed");
    assert_eq!(m["reveal"]["answer"], choices[right - 1]);
    assert!(!m["reveal"]["explanation"].as_str().unwrap().is_empty());
}

#[test]
fn a_correct_choice_also_gets_the_explanation_and_hints_work_before_an_attempt_up_to_three() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    create_and_open(&mut a, &ctx(t, 1));
    start_ukrainian(&mut a, t, 1);
    // Підказки до першої спроби дозволені (для завдань із вибором), лише три навідні.
    let mut now = t;
    let mut texts = 0;
    for _ in 0..3 {
        now += 100;
        let h = ok(&mut a, "hint", json!({}), &ctx(now, 1));
        assert_eq!(h["hint"]["kind"], "text");
        texts += 1;
    }
    assert_eq!(texts, 3);
    now += 100;
    assert_eq!(
        ok(&mut a, "hint", json!({}), &ctx(now, 1))["hint"]["kind"],
        "none"
    );
    let right = a.peek_answer().unwrap();
    let r = ok(&mut a, "answer", json!({"input": right}), &ctx(now + 5, 1));
    assert_eq!(r["result"], "correct");
    assert_eq!(r["model"]["outcome"], "solved");
    assert!(!r["model"]["reveal"]["explanation"]
        .as_str()
        .unwrap()
        .is_empty());
}

#[test]
fn quizzes_in_a_subject_lesson_work_and_mastering_the_subject_earns_its_course_badge() {
    let d = TestDir::new();
    let mut a = app(&d);
    create_and_open(&mut a, &ctx(DAY, 1));
    // Урок: питання без відповіді до спроби; правильні відповіді дають відзнаку за урок.
    let l = ok(
        &mut a,
        "lesson",
        json!({"skill": "UA-APOSTROPHE"}),
        &ctx(DAY, 1),
    );
    let quizzes: Vec<&Value> = l["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["type"] == "quiz")
        .collect();
    assert!(quizzes.len() >= 2);
    for q in &quizzes {
        assert!(q.get("correct").is_none() && q.get("explanation").is_none());
    }
    let mut badge = 0;
    for q in &quizzes {
        let i = q["index"].as_u64().unwrap();
        let w = ok(
            &mut a,
            "lesson_check",
            json!({"skill": "UA-APOSTROPHE", "index": i, "input": "99"}),
            &ctx(DAY, 1),
        );
        assert_eq!(w["unreadable"], true);
        // Учень знає відповідь: правильна відповідь з першої спроби зараховується (вгадування перебором ні).
        let src = {
            use tutor_core::lesson::{LessonBlock, LessonSource};
            let l = subjects::SubjectLessons.lesson("UA-APOSTROPHE").unwrap();
            let k = l
                .blocks
                .iter()
                .filter_map(|b| match b {
                    LessonBlock::Quiz { correct, .. } => Some(*correct),
                    _ => None,
                })
                .nth(i as usize)
                .unwrap();
            k + 1
        };
        let r = ok(
            &mut a,
            "lesson_check",
            json!({"skill": "UA-APOSTROPHE", "index": i, "input": src.to_string()}),
            &ctx(DAY, 1),
        );
        assert_eq!(r["correct"], true);
        badge += r["new_badges"].as_array().map_or(0, Vec::len);
    }
    // Лише питання без розібраних прикладів: відзнаки ще немає; розбираємо приклади, і вона з'являється.
    assert_eq!(badge, 0, "без прикладів відзнаки за урок немає");
    let n_ex = l["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["type"] == "example")
        .count() as u64;
    for e in 0..n_ex {
        let r = ok(
            &mut a,
            "lesson_example",
            json!({"skill": "UA-APOSTROPHE", "index": e}),
            &ctx(DAY, 1),
        );
        let _ = r;
    }
    {
        use tutor_core::lesson::{LessonBlock, LessonSource};
        let text =
            "Я поясню це своїми словами так: спочатку згадую правило, потім перевіряю на слові.";
        let mut xp = 0u64;
        for b in &subjects::SubjectLessons
            .lesson("UA-APOSTROPHE")
            .unwrap()
            .blocks
        {
            if let LessonBlock::Explain { key_ideas, .. } = b {
                let rec: Vec<bool> = key_ideas.iter().map(|_| true).collect();
                ok(
                    &mut a,
                    "lesson_explain",
                    json!({"skill": "UA-APOSTROPHE", "index": xp, "text": text}),
                    &ctx(DAY, 1),
                );
                ok(
                    &mut a,
                    "lesson_explain",
                    json!({"skill": "UA-APOSTROPHE", "index": xp, "text": text, "recalled": rec}),
                    &ctx(DAY, 1),
                );
                xp += 1;
            }
        }
    }
    let album = ok(&mut a, "album", json!({}), &ctx(DAY, 1));
    let earned = album["badges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["id"] == "lesson:UA-APOSTROPHE" && b["earned"] == true)
        .count();
    badge += earned;
    assert_eq!(badge, 1);
    // Опанування однієї теми: правильні відповіді у два різні дні (вільна практика цієї теми).
    for day in 2..=3u32 {
        let mut now = u64::from(day) * DAY + 9 * 3_600;
        let mut model = ok(
            &mut a,
            "practice_start",
            json!({"skill": "UA-APOSTROPHE"}),
            &ctx(now, day),
        )["model"]
            .clone();
        for _ in 0..8 {
            if model["phase"] != "task" {
                break;
            }
            now += 15;
            let right = a.peek_answer().unwrap();
            let r = ok(&mut a, "answer", json!({"input": right}), &ctx(now, day));
            model = r["model"].clone();
            if model["phase"] == "feedback" {
                now += 2;
                model = ok(&mut a, "continue", json!({}), &ctx(now, day))["model"].clone();
            }
        }
        if model["phase"] != "ended" {
            ok(&mut a, "end_session", json!({}), &ctx(now, day));
        }
    }
    let home = ok(&mut a, "home", json!({}), &ctx(4 * DAY, 4));
    let state = home["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "UA-APOSTROPHE")
        .unwrap()["state"]
        .clone();
    assert_eq!(state, "mastered");
    let ua = home["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "ukrainian")
        .unwrap()
        .clone();
    assert!(
        ua["mastered"].as_u64().unwrap() >= 1
            && ua["mastered"].as_u64().unwrap() < ua["total"].as_u64().unwrap()
    );
}

#[test]
fn an_interrupted_subject_session_resumes_in_the_same_subject_after_a_restart() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t = 9 * 3_600;
    let (slot, _) = create_and_open(&mut a, &ctx(t, 1));
    start_ukrainian(&mut a, t, 1);
    ok(&mut a, "lock", json!({}), &ctx(t + 5, 1));
    let mut b = app(&d);
    ok(
        &mut b,
        "open_profile",
        json!({"slot": slot, "pin": "4821"}),
        &ctx(t + 10, 1),
    );
    let home = ok(&mut b, "home", json!({}), &ctx(t + 11, 1));
    assert_eq!(home["can_start"]["resume"], true);
    let m = ok(&mut b, "session_start", json!({}), &ctx(t + 12, 1))["model"].clone();
    assert_eq!(m["phase"], "task");
    assert!(
        m["choices"].as_array().is_some_and(|c| c.len() >= 3),
        "це завдання з вибором із мови"
    );
}

#![cfg(not(feature = "reviewer"))]
mod common;

use common::{app, call, create_and_open, ctx, ok, TestDir};
use serde_json::{json, Value};

const SKILLS: [&str; 35] = [
    "N-ADD-MULTI",
    "N-SUB-MULTI",
    "N-MUL-MULTI",
    "N-DIV-MULTI",
    "N-ORDER",
    "D-ADD",
    "D-SUB",
    "F-ADD-SAME",
    "F-ADD-DIFF",
    "F-SUB-SAME",
    "F-SUB-DIFF",
    "N-ROUND",
    "N-DIVREM",
    "N-DIVTEST",
    "N-GCD",
    "N-LCM",
    "N-POWER",
    "EQ-ADD-SUB",
    "EQ-MUL",
    "WORD-1",
    "WORD-2",
    "MOTION",
    "U-LEN",
    "U-MASS",
    "U-TIME",
    "G-PERIM",
    "G-AREA",
    "G-VOL",
    "A-ANGLE",
    "F-MIXED",
    "F-REDUCE",
    "F-OF",
    "P-PCT",
    "D-MUL10",
    "AVG",
];

fn selfchecks(lesson: &Value) -> Vec<u64> {
    lesson["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["type"] == "selfcheck")
        .map(|b| b["index"].as_u64().unwrap())
        .collect()
}

#[test]
fn every_skill_has_a_lesson_and_a_page_and_no_answer_is_sent_before_the_attempt() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(9 * 3_600, 1);
    create_and_open(&mut a, &c);
    for s in SKILLS {
        let page = ok(&mut a, "skill_page", json!({"skill": s}), &c);
        assert_eq!(page["has_lesson"], true, "{s}");
        assert!(!page["note_prompts"].as_array().unwrap().is_empty());
        let l = ok(&mut a, "lesson", json!({"skill": s}), &c);
        assert!(selfchecks(&l).len() >= 2, "{s}");
        for b in l["blocks"].as_array().unwrap() {
            if b["type"] == "selfcheck" {
                assert!(
                    b.get("answer").is_none() && b.get("explanation").is_none(),
                    "{s}: відповідь до спроби"
                );
            }
        }
    }
    assert_eq!(
        call(&mut a, "lesson", json!({"skill": "NOPE"}), &c)
            .unwrap_err()
            .code,
        "bad_request"
    );
}

/// Проходить урок чесно: розкриває приклади й розв'язує контрольні питання (відповіді з джерела уроків).
fn complete_lesson(a: &mut app_core::App, skill: &str, c: &app_core::Ctx) -> Vec<Value> {
    use tutor_core::lesson::{LessonBlock, LessonSource};
    let src = subjects::SubjectLessons.lesson(skill).unwrap();
    let mut badges = Vec::new();
    let (mut ex, mut ck, mut xp) = (0u64, 0u64, 0u64);
    for b in &src.blocks {
        let mut check = |input: String, ck: u64| {
            let r = ok(
                a,
                "lesson_check",
                json!({"skill": skill, "index": ck, "input": input}),
                c,
            );
            assert_eq!(r["correct"], true, "{skill} #{ck}");
            badges.extend(r["new_badges"].as_array().cloned().unwrap_or_default());
        };
        match b {
            LessonBlock::Example { .. } => {
                ok(a, "lesson_example", json!({"skill": skill, "index": ex}), c);
                ex += 1;
            }
            LessonBlock::SelfCheck { answer, .. } => {
                check(answer.clone(), ck);
                ck += 1;
            }
            LessonBlock::Quiz { correct, .. } => {
                check((correct + 1).to_string(), ck);
                ck += 1;
            }
            LessonBlock::Explain { key_ideas, .. } => {
                let text = "Я поясню це своїми словами так: спочатку розумію ідею, потім перевіряю на прикладі.";
                ok(
                    a,
                    "lesson_explain",
                    json!({"skill": skill, "index": xp, "text": text}),
                    c,
                );
                let rec: Vec<bool> = key_ideas.iter().map(|_| true).collect();
                let r = ok(
                    a,
                    "lesson_explain",
                    json!({"skill": skill, "index": xp, "text": text, "recalled": rec}),
                    c,
                );
                badges.extend(r["new_badges"].as_array().cloned().unwrap_or_default());
                xp += 1;
            }
            _ => {}
        }
    }
    badges
}

#[test]
fn a_wrong_lesson_answer_first_gets_a_nudge_then_the_solution_but_no_credit() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(9 * 3_600, 1);
    create_and_open(&mut a, &c);
    let l = ok(&mut a, "lesson", json!({"skill": "D-ADD"}), &c);
    let i = selfchecks(&l)[0];
    let r = ok(
        &mut a,
        "lesson_check",
        json!({"skill": "D-ADD", "index": i, "input": "999999"}),
        &c,
    );
    assert_eq!(r["correct"], false);
    assert_eq!(
        r["explanation"], "",
        "після першої помилки пояснення ще немає"
    );
    assert_eq!(
        r["answer"], "",
        "після першої помилки відповідь ще не показується"
    );
    assert_eq!(r["unlocked"], false);
    let r = ok(
        &mut a,
        "lesson_check",
        json!({"skill": "D-ADD", "index": i, "input": "ой"}),
        &c,
    );
    assert_eq!(r["unreadable"], true);
    let r = ok(
        &mut a,
        "lesson_check",
        json!({"skill": "D-ADD", "index": i, "input": "888888"}),
        &c,
    );
    let right = r["answer"].as_str().unwrap().to_string();
    assert!(!right.is_empty(), "після другої помилки показується розбір");
    assert!(!r["explanation"].as_str().unwrap().is_empty());
    assert_eq!(r["unlocked"], true);
    // Відповісти правильно після розбору можна, але кредиту за це питання немає.
    let r = ok(
        &mut a,
        "lesson_check",
        json!({"skill": "D-ADD", "index": i, "input": right}),
        &c,
    );
    assert_eq!(r["correct"], true);
    assert_eq!(r["credited"], false);
    let l = ok(&mut a, "lesson", json!({"skill": "D-ADD"}), &c);
    assert_eq!(l["progress"]["checks"], 0);
    // Наступного дня спроби починаються заново: правильна самостійна відповідь зараховується.
    let c2 = ctx(86_400 + 9 * 3_600, 2);
    let r = ok(
        &mut a,
        "lesson_check",
        json!({"skill": "D-ADD", "index": i, "input": right}),
        &c2,
    );
    assert_eq!(r["credited"], true);
}

#[test]
fn the_lesson_badge_needs_every_example_understood_and_every_check_solved_and_comes_once() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(9 * 3_600, 1);
    create_and_open(&mut a, &c);
    let l = ok(&mut a, "lesson", json!({"skill": "D-ADD"}), &c);
    assert_eq!(l["progress"]["complete"], false);
    // Лише контрольні питання без прикладів: уроку не завершено, відзнаки немає.
    use tutor_core::lesson::{LessonBlock, LessonSource};
    let mut n = 0u64;
    for b in &subjects::SubjectLessons.lesson("D-ADD").unwrap().blocks {
        let input = match b {
            LessonBlock::SelfCheck { answer, .. } => answer.clone(),
            LessonBlock::Quiz { correct, .. } => (correct + 1).to_string(),
            _ => continue,
        };
        let r = ok(
            &mut a,
            "lesson_check",
            json!({"skill": "D-ADD", "index": n, "input": input}),
            &c,
        );
        assert!(
            r["new_badges"].as_array().map_or(true, Vec::is_empty),
            "без прикладів відзнаки немає"
        );
        n += 1;
    }
    complete_lesson(&mut a, "D-ADD", &c);
    let album = ok(&mut a, "album", json!({}), &c);
    let earned: Vec<&str> = album["badges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|b| b["earned"] == true)
        .map(|b| b["id"].as_str().unwrap())
        .collect();
    assert_eq!(earned, ["lesson:D-ADD"]);
    let l = ok(&mut a, "lesson", json!({"skill": "D-ADD"}), &c);
    assert_eq!(l["progress"]["complete"], true);
}

#[test]
fn a_note_earns_a_badge_only_for_real_own_words_after_the_lesson_and_labels_do_not_count() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(9 * 3_600, 1);
    create_and_open(&mut a, &c);
    let text = "Додаю чисельники, а знаменник залишається, бо частини однакового розміру. Наприкінці скорочую дріб, якщо можна.";
    // Підписи розділів і одна літера, повторена багато разів, не є конспектом, навіть після уроку.
    ok(
        &mut a,
        "note_set",
        json!({"skill": "F-ADD-SAME", "fields": [
            {"label": "Головне своїми словами про додавання дробів з однаковими знаменниками", "text": "ааааааааааааааааааааааааааааааааааааааааааааааааааааааааааааааааааааааааааааааааааа"}]}),
        &c,
    );
    let got: Vec<String> = complete_lesson(&mut a, "F-ADD-SAME", &c)
        .iter()
        .map(|b| b["id"].as_str().unwrap().to_string())
        .collect();
    assert!(!got.contains(&"note:F-ADD-SAME".to_string()), "{got:?}");
    let r = ok(
        &mut a,
        "note_set",
        json!({"skill": "F-ADD-SAME", "fields": [
            {"label": "Головне", "text": text}, {"label": "Мій приклад", "text": "одна друга плюс одна друга"}]}),
        &c,
    );
    assert_eq!(r["new_badges"][0]["id"], "note:F-ADD-SAME");
    let got = ok(&mut a, "note_get", json!({"skill": "F-ADD-SAME"}), &c);
    assert_eq!(got["fields"][1]["label"], "Мій приклад");
    let r = ok(
        &mut a,
        "note_set",
        json!({"skill": "F-ADD-SAME", "text": text}),
        &c,
    );
    assert!(
        r["new_badges"].as_array().map_or(true, Vec::is_empty),
        "вдруге не видається"
    );
    // Стирання конспекту нічого не відбирає.
    ok(
        &mut a,
        "note_set",
        json!({"skill": "F-ADD-SAME", "text": ""}),
        &c,
    );
    let album = ok(&mut a, "album", json!({}), &c);
    assert!(album["badges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b["id"] == "note:F-ADD-SAME" && b["earned"] == true));
}

#[test]
fn mastering_every_skill_earns_family_and_course_badges_and_the_feedback_announces_them() {
    let d = TestDir::new();
    let mut a = app(&d);
    let mut announced = Vec::new();
    create_and_open(&mut a, &ctx(1, 1));
    'days: for day in 1..=4u32 {
        let mut now = u64::from(day) * 86_400 + 9 * 3_600;
        for s in SKILLS {
            let r = ok(
                &mut a,
                "practice_start",
                json!({"skill": s}),
                &ctx(now, day),
            );
            let mut model = r["model"].clone();
            for _ in 0..10 {
                if model["phase"] != "task" {
                    break;
                }
                now += 20;
                let right = a.peek_answer().unwrap();
                let r = ok(&mut a, "answer", json!({"input": right}), &ctx(now, day));
                model = r["model"].clone();
                if let Some(b) = model["badges"].as_array() {
                    announced.extend(b.iter().map(|x| x["id"].as_str().unwrap().to_string()));
                }
                if model["phase"] == "feedback" {
                    now += 3;
                    model = ok(&mut a, "continue", json!({}), &ctx(now, day))["model"].clone();
                }
            }
            if model["phase"] != "ended" {
                ok(&mut a, "end_session", json!({}), &ctx(now, day));
            }
        }
        let home = ok(&mut a, "home", json!({}), &ctx(now, day));
        if home["skills"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["state"] == "mastered")
        {
            break 'days;
        }
    }
    for want in ["family:N", "family:D", "family:F", "course:math5"] {
        assert!(
            announced.iter().any(|x| x == want),
            "{want} не оголошено: {announced:?}"
        );
    }
}

#[test]
fn lessons_are_closed_while_a_task_is_open_so_they_can_never_leak_its_numbers() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(9 * 3_600, 1);
    create_and_open(&mut a, &c);
    ok(&mut a, "session_start", json!({}), &c);
    for cmd in [
        ("lesson", json!({"skill": "D-ADD"})),
        (
            "lesson_check",
            json!({"skill": "D-ADD", "index": 0, "input": "1"}),
        ),
    ] {
        assert_eq!(
            call(&mut a, cmd.0, cmd.1, &c).unwrap_err().code,
            "task_active"
        );
    }
    ok(&mut a, "end_session", json!({}), &c);
    ok(&mut a, "lesson", json!({"skill": "D-ADD"}), &c);
}

#[test]
fn recall_comes_back_after_a_day_needs_own_words_first_and_spaces_by_self_rating() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c1 = ctx(9 * 3_600, 1);
    create_and_open(&mut a, &c1);
    // Нічого не вивчено: повторювати нічого.
    let q = ok(&mut a, "recall_queue", json!({}), &c1);
    assert_eq!(q["cards"].as_array().unwrap().len(), 0);
    complete_lesson(&mut a, "D-ADD", &c1);
    // У день проходження картки ще немає, наступного дня вона з'являється.
    let q = ok(&mut a, "recall_queue", json!({}), &c1);
    assert_eq!(q["cards"].as_array().unwrap().len(), 0);
    let c2 = ctx(86_400 + 9 * 3_600, 2);
    let home = ok(&mut a, "home", json!({}), &c2);
    assert_eq!(home["recall_due"], 1);
    let q = ok(&mut a, "recall_queue", json!({}), &c2);
    assert_eq!(q["cards"][0]["skill"], "D-ADD");
    // Самооцінка без власної спроби не приймається.
    assert!(call(
        &mut a,
        "recall_rate",
        json!({"skill": "D-ADD", "rating": "yes"}),
        &c2
    )
    .is_err());
    // Нечесна «спроба» (одна літера багато разів) не відкриває ідеї.
    let r = ok(
        &mut a,
        "recall_show",
        json!({"skill": "D-ADD", "text": "ааааааааааааааааааааааааааааа"}),
        &c2,
    );
    assert_eq!(r["accepted"], false);
    let r = ok(
        &mut a,
        "recall_show",
        json!({"skill": "D-ADD", "text": "Додаю числа під комою, кому ставлю на місце, потім перевіряю."}),
        &c2,
    );
    assert_eq!(r["accepted"], true);
    // Нова відповідь у дві зайві спроби не потрібна: оцінюємо, інтервал зростає (3 дні).
    let r = ok(
        &mut a,
        "recall_rate",
        json!({"skill": "D-ADD", "rating": "yes"}),
        &c2,
    );
    assert_eq!(r["recalls"], 1);
    let q = ok(&mut a, "recall_queue", json!({}), &c2);
    assert_eq!(
        q["cards"].as_array().unwrap().len(),
        0,
        "картка пішла на наступні дні"
    );
    let c5 = ctx(4 * 86_400 + 9 * 3_600, 5);
    let q = ok(&mut a, "recall_queue", json!({}), &c5);
    assert_eq!(
        q["cards"].as_array().unwrap().len(),
        1,
        "через 3 дні картка повертається"
    );
}

#[test]
fn a_lesson_question_has_two_levels_of_hints_and_a_way_to_see_the_solution_without_credit() {
    use tutor_core::lesson::{LessonBlock, LessonSource};
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(9 * 3_600, 1);
    create_and_open(&mut a, &c);
    let l = subjects::SubjectLessons.lesson("UA-APOSTROPHE").unwrap();
    let correct = l
        .blocks
        .iter()
        .find_map(|b| match b {
            LessonBlock::Quiz { correct, .. } => Some(*correct),
            _ => None,
        })
        .unwrap();
    // Розбір без жодної спроби чи підказки не показується.
    assert!(call(
        &mut a,
        "lesson_check",
        json!({"skill": "UA-APOSTROPHE", "index": 0, "give_up": true}),
        &c
    )
    .is_err());
    let h1 = ok(
        &mut a,
        "lesson_hint",
        json!({"skill": "UA-APOSTROPHE", "index": 0}),
        &c,
    );
    assert_eq!(
        (h1["level"].as_u64(), h1["kind"].as_str()),
        (Some(1), Some("reread"))
    );
    let h2 = ok(
        &mut a,
        "lesson_hint",
        json!({"skill": "UA-APOSTROPHE", "index": 0}),
        &c,
    );
    assert_eq!(h2["kind"], "eliminate");
    let removed = h2["remove"].as_u64().unwrap() as usize;
    assert_ne!(
        removed, correct,
        "підказка ніколи не прибирає правильний варіант"
    );
    // Третя підказка лишається другою (далі підказувати нічого).
    assert_eq!(
        ok(
            &mut a,
            "lesson_hint",
            json!({"skill": "UA-APOSTROPHE", "index": 0}),
            &c
        )["level"],
        2
    );
    // Розбір після підказки: відповідь і пояснення, крок розблоковано, відзнаки за нього немає.
    let r = ok(
        &mut a,
        "lesson_check",
        json!({"skill": "UA-APOSTROPHE", "index": 0, "give_up": true}),
        &c,
    );
    assert_eq!(r["gave_up"], true);
    assert_eq!(r["unlocked"], true);
    assert_eq!(r["credited"], false);
    assert!(!r["answer"].as_str().unwrap().is_empty());
    // Навіть правильна відповідь після розбору не дає кредиту.
    let r = ok(
        &mut a,
        "lesson_check",
        json!({"skill": "UA-APOSTROPHE", "index": 0, "input": (correct + 1).to_string()}),
        &c,
    );
    assert_eq!(r["credited"], false);
    // «Не знаю, покажи ідеї» показує ідеї, але урок від цього не стає зарахованим.
    let e = ok(
        &mut a,
        "lesson_explain",
        json!({"skill": "UA-APOSTROPHE", "index": 0, "skip": true}),
        &c,
    );
    assert_eq!(e["skipped"], true);
    assert_eq!(e["unlocked"], true);
    assert!(!e["key_ideas"].as_array().unwrap().is_empty());
    let l = ok(&mut a, "lesson", json!({"skill": "UA-APOSTROPHE"}), &c);
    assert_eq!(l["progress"]["complete"], false);
    assert_eq!(l["progress"]["explains"], 0);
}

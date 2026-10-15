#![cfg(not(feature = "reviewer"))]
mod common;

use app_core::App;
use common::{app, call, create_and_open, ctx, enforce_limits, ok, TestDir};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const DAY: u64 = 86_400;

/// Верхня межа днів для проходження курсу математики через API (цикл завершується, щойно все опановано).
const COURSE_DAYS: u32 = 120;

/// Проходить поточну сесію до кінця: усі відповіді правильні, інколи після помилки й підказки.
fn play_session(a: &mut App, start: u64, day: u32, mistakes: bool) -> (u64, Value) {
    let mut now = start;
    let mut model = ok(a, "session_start", json!({}), &ctx(now, day))["model"].clone();
    assert_ne!(model["phase"], "none", "сесія мала початися");
    let mut guard = 0;
    let mut tasks = 0;
    loop {
        guard += 1;
        assert!(guard < 400, "сесія не завершується");
        match model["phase"].as_str().unwrap() {
            "ended" => return (now, model),
            "feedback" => {
                now += 3;
                model = ok(a, "continue", json!({}), &ctx(now, day))["model"].clone();
            }
            "analog" => {
                now += 10;
                let right = a.peek_analog_answer().unwrap();
                model = ok(a, "answer", json!({"input": right}), &ctx(now, day))["model"].clone();
            }
            "task" => {
                now += 20;
                tasks += 1;
                if mistakes && tasks % 5 == 0 {
                    // Помилка, потім підказка (після паузи), потім правильно.
                    ok(a, "answer", json!({"input": "0"}), &ctx(now, day));
                    now += 20;
                    ok(a, "hint", json!({}), &ctx(now, day));
                    now += 20;
                }
                let right = a.peek_answer().unwrap();
                model = ok(a, "answer", json!({"input": right}), &ctx(now, day))["model"].clone();
            }
            other => panic!("невідома фаза {other}"),
        }
    }
}

#[test]
fn a_whole_course_through_the_api_masters_every_skill_without_a_single_leak() {
    let d = TestDir::new();
    let mut a = app(&d);
    create_and_open(&mut a, &ctx(1, 1));
    enforce_limits(&mut a, &ctx(1, 1));
    let mut all_stickers = BTreeSet::new();
    let math_mastered = |home: &Value| {
        home["skills"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["subject"] == "math")
            .all(|s| s["state"] == "mastered")
    };
    // Курс математики великий (десятки тем): граф відкриває теми поступово, тож дні рахуємо до опанування всього.
    let mut last_day = 1;
    for day in 1..=COURSE_DAYS {
        last_day = day;
        let mut now = u64::from(day) * DAY + 9 * 3_600;
        for _ in 0..2 {
            let home = ok(&mut a, "home", json!({}), &ctx(now, day));
            if home["can_start"]["ok"] != true {
                break;
            }
            let (end, ended) = play_session(&mut a, now, day, true);
            for s in ended["stickers"].as_array().unwrap() {
                all_stickers.insert(s["key"].as_str().unwrap().to_string());
            }
            now = end + 600; // перерва між сесіями минула
        }
        if math_mastered(&ok(&mut a, "home", json!({}), &ctx(now, day))) {
            break;
        }
    }
    let home = ok(
        &mut a,
        "home",
        json!({}),
        &ctx(last_day as u64 * DAY + 20_000, last_day),
    );
    let states: Vec<_> = home["skills"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["subject"] == "math")
        .map(|s| s["state"].as_str().unwrap().to_string())
        .collect();
    assert!(
        states.iter().all(|s| s == "mastered"),
        "не все опановано: {states:?}"
    );
    let topics = home["skills"].as_array().unwrap().len();
    let math_topics = home["skills"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["subject"] == "math")
        .count();
    assert!(
        all_stickers
            .iter()
            .filter(|k| k.starts_with("mastery:"))
            .count()
            >= math_topics,
        "{all_stickers:?}"
    );
    assert!(all_stickers.len() <= 4 * topics);
    let album = ok(
        &mut a,
        "album",
        json!({}),
        &ctx(last_day as u64 * DAY + 20_000, last_day),
    );
    assert_eq!(album["capacity"], 4 * topics);
    assert_eq!(album["cells"].as_array().unwrap().len(), 4 * topics);
    assert_eq!(
        album["count"].as_u64().unwrap() as usize,
        album["cells"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["earned"] == true)
            .count()
    );
}

#[test]
fn daily_limit_break_and_quiet_hours() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t0 = 5 * DAY + 9 * 3_600;
    create_and_open(&mut a, &ctx(t0, 5));
    enforce_limits(&mut a, &ctx(t0, 5));
    let (end1, _) = play_session(&mut a, t0, 5, false);
    // Одразу після сесії діє перерва.
    let home = ok(&mut a, "home", json!({}), &ctx(end1 + 30, 5));
    assert_eq!(home["can_start"]["ok"], false);
    assert_eq!(home["can_start"]["reason"], "break");
    let denied = ok(&mut a, "session_start", json!({}), &ctx(end1 + 30, 5));
    assert!(denied["denied"]["secs"].as_u64().unwrap() <= 240);
    assert_eq!(denied["model"]["phase"], "none");
    // Після перерви друга сесія дозволена, третя ні.
    // Після перерви: єдина доступна навичка вже вичерпала ліміт задач на день, тож пропонувати нічого.
    let home = ok(&mut a, "home", json!({}), &ctx(end1 + 300, 5));
    let math = home["subjects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "math")
        .unwrap();
    assert_eq!(math["nothing_left"], true, "а не порожня сесія");
    // Наступного дня знову можна.
    let home = ok(&mut a, "home", json!({}), &ctx(t0 + DAY, 6));
    assert_eq!(home["can_start"]["ok"], true);
}

#[test]
fn quiet_hours_need_an_adult_and_then_block_starting() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(9 * 3_600, 1);
    create_and_open(&mut a, &c);
    enforce_limits(&mut a, &c);
    ok(&mut a, "adult_set_pin", json!({"pin": "2468"}), &c);
    ok(&mut a, "adult_lock", json!({}), &c);
    let err = call(
        &mut a,
        "settings_set",
        json!({"quiet_hours": {"from": 0, "to": 1439}}),
        &c,
    )
    .unwrap_err();
    assert_eq!(err.code, "adult_required");
    ok(&mut a, "adult_unlock", json!({"pin": "2468"}), &c);
    ok(
        &mut a,
        "settings_set",
        json!({"quiet_hours": {"from": 0, "to": 1439}}),
        &c,
    );
    let home = ok(&mut a, "home", json!({}), &c);
    assert_eq!(home["can_start"]["reason"], "quiet");
    ok(&mut a, "settings_set", json!({"quiet_hours": null}), &c);
    assert_eq!(ok(&mut a, "home", json!({}), &c)["can_start"]["ok"], true);
    // Хибні тихі години відхиляються.
    assert_eq!(
        call(
            &mut a,
            "settings_set",
            json!({"quiet_hours": {"from": 5000, "to": 1}}),
            &c
        )
        .unwrap_err()
        .code,
        "bad_request"
    );
}

#[test]
fn an_interrupted_session_resumes_after_a_restart_and_does_not_use_a_second_slot() {
    let d = TestDir::new();
    let t0 = 3 * DAY + 9 * 3_600;
    let slot;
    {
        let mut a = app(&d);
        slot = create_and_open(&mut a, &ctx(t0, 3)).0;
        let m = ok(&mut a, "session_start", json!({}), &ctx(t0, 3))["model"].clone();
        assert_eq!(m["phase"], "task");
        for k in 1..=3 {
            let right = a.peek_answer().unwrap();
            ok(
                &mut a,
                "answer",
                json!({"input": right}),
                &ctx(t0 + k * 20, 3),
            );
            ok(&mut a, "continue", json!({}), &ctx(t0 + k * 20 + 5, 3));
        }
        // Світло зникло: застосунок закрито без завершення сесії.
        ok(&mut a, "lock", json!({}), &ctx(t0 + 300, 3));
    }
    let mut a = app(&d);
    ok(
        &mut a,
        "open_profile",
        json!({"slot": slot, "pin": "4821"}),
        &ctx(t0 + 330, 3),
    );
    let home = ok(&mut a, "home", json!({}), &ctx(t0 + 330, 3));
    assert_eq!(home["can_start"]["resume"], true, "є що продовжити");
    let m = ok(&mut a, "session_start", json!({}), &ctx(t0 + 340, 3))["model"].clone();
    assert_eq!(m["phase"], "task", "продовжили з нової задачі");
    ok(&mut a, "end_session", json!({}), &ctx(t0 + 400, 3));
    // Друга сесія того ж дня доступна після перерви: перша була лише одна.
    let home = ok(&mut a, "home", json!({}), &ctx(t0 + 1_000, 3));
    assert_eq!(home["can_start"]["ok"], true);
}

#[test]
fn a_very_short_accidental_session_does_not_cost_a_slot() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t0 = 2 * DAY + 9 * 3_600;
    create_and_open(&mut a, &ctx(t0, 2));
    ok(&mut a, "session_start", json!({}), &ctx(t0, 2));
    ok(&mut a, "end_session", json!({}), &ctx(t0 + 20, 2));
    ok(&mut a, "home", json!({}), &ctx(t0 + 25, 2));
    let r = ok(&mut a, "session_start", json!({}), &ctx(t0 + 30, 2));
    assert_eq!(
        r["model"]["phase"], "task",
        "випадковий запуск не вимагає перерви"
    );
}

#[test]
fn hostile_and_malformed_calls_are_errors_not_panics() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    assert_eq!(
        call(&mut a, "answer", json!({"input": "1"}), &c)
            .unwrap_err()
            .code,
        "no_profile_open"
    );
    create_and_open(&mut a, &c);
    assert_eq!(
        call(&mut a, "nope", json!({}), &c).unwrap_err().code,
        "unknown_command"
    );
    assert_eq!(
        call(&mut a, "answer", json!({"input": "1"}), &c)
            .unwrap_err()
            .code,
        "wrong_phase"
    );
    assert_eq!(
        call(&mut a, "hint", json!({}), &c).unwrap_err().code,
        "wrong_phase"
    );
    assert_eq!(
        call(&mut a, "continue", json!({}), &c).unwrap_err().code,
        "wrong_phase"
    );
    assert_eq!(
        call(&mut a, "create_profile", json!({}), &c)
            .unwrap_err()
            .code,
        "bad_request"
    );
    assert_eq!(
        call(
            &mut a,
            "create_profile",
            json!({"nickname": "\u{202E}\u{200B}", "avatar": 1, "pin": "4821"}),
            &c
        )
        .unwrap_err()
        .code,
        "invalid_nickname"
    );
    ok(&mut a, "session_start", json!({}), &c);
    for bad in [
        "",
        "   ",
        &"9".repeat(10_000),
        "ой",
        "1/0",
        "--1",
        "\u{0}",
        "٣",
    ] {
        let r = ok(&mut a, "answer", json!({"input": bad}), &ctx(1_010, 1));
        assert!(
            ["unreadable", "try_again"].contains(&r["result"].as_str().unwrap()),
            "{bad:?} -> {r}"
        );
    }
    assert_eq!(
        call(&mut a, "answer", json!({"input": 5}), &ctx(1_020, 1))
            .map(|r| r["result"].clone())
            .ok(),
        Some(json!("unreadable"))
    );
    assert_eq!(
        call(
            &mut a,
            "open_profile",
            json!({"slot": 99, "pin": "4821"}),
            &c
        )
        .unwrap_err()
        .code,
        "no_such_profile"
    );
    assert_eq!(
        call(&mut a, "erase_profile", json!({"slot": 99}), &c)
            .map(|_| ())
            .ok(),
        Some(())
    );
}

#[test]
fn adult_mode_gates_settings_items_and_export() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(10_000, 1);
    create_and_open(&mut a, &c);
    ok(&mut a, "session_start", json!({}), &c);
    ok(&mut a, "hand_to_adult", json!({}), &ctx(10_050, 1));
    ok(&mut a, "end_session", json!({}), &ctx(10_060, 1));
    // Поки PIN не задано, нічого не закрито: налаштування й список доступні самому учню.
    assert_eq!(
        ok(&mut a, "adult_items", json!({}), &ctx(10_100, 1))["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(ok(&mut a, "adult_status", json!({}), &c)["pin_set"], false);
    ok(
        &mut a,
        "adult_set_pin",
        json!({"pin": "2468"}),
        &ctx(10_100, 1),
    );
    ok(&mut a, "adult_lock", json!({}), &ctx(10_100, 1));
    assert_eq!(
        call(
            &mut a,
            "adult_unlock",
            json!({"pin": "0000"}),
            &ctx(10_100, 1)
        )
        .unwrap_err()
        .code,
        "wrong_adult_pin"
    );
    assert_eq!(
        call(
            &mut a,
            "adult_set_pin",
            json!({"pin": "1357"}),
            &ctx(10_100, 1)
        )
        .unwrap_err()
        .code,
        "adult_required"
    );
    ok(
        &mut a,
        "adult_unlock",
        json!({"pin": "2468"}),
        &ctx(10_100, 1),
    );
    let items = ok(&mut a, "adult_items", json!({}), &ctx(10_110, 1));
    assert_eq!(items["items"].as_array().unwrap().len(), 1);
    let it = &items["items"][0];
    let r = ok(
        &mut a,
        "adult_resolve",
        json!({"skill": it["skill"], "statement": it["statement"]}),
        &ctx(10_120, 1),
    );
    assert_eq!(r["removed"], true);
    assert_eq!(
        ok(&mut a, "adult_items", json!({}), &ctx(10_130, 1))["items"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    // Через 5 хвилин режим дорослого сам вимикається.
    assert_eq!(
        call(&mut a, "adult_items", json!({}), &ctx(10_100 + 301, 1))
            .unwrap_err()
            .code,
        "adult_required"
    );
}

#[test]
fn encrypted_export_and_import_move_the_progress_to_a_new_profile() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t0 = 4 * DAY + 9 * 3_600;
    create_and_open(&mut a, &ctx(t0, 4));
    enforce_limits(&mut a, &ctx(t0, 4));
    ok(
        &mut a,
        "note_set",
        json!({"skill": "N-ADD-MULTI", "text": "мій конспект переїхав"}),
        &ctx(t0, 4),
    );
    let (end, _) = play_session(&mut a, t0, 4, false);
    ok(
        &mut a,
        "adult_set_pin",
        json!({"pin": "2468"}),
        &ctx(end, 4),
    );
    // Експорт 64 МіБ-Argon2 у продакшені дорогий, але в тестах використовується продакшен-параметр експорту: це один виклик.
    let exp = ok(
        &mut a,
        "export_profile",
        json!({"passphrase": "фраза для перенесення"}),
        &ctx(end + 1, 4),
    );
    let data = exp["data"].as_str().unwrap().to_string();
    let before = ok(&mut a, "home", json!({}), &ctx(end + 2, 4))["skills"].clone();
    let d2 = TestDir::new();
    let mut b = app(&d2);
    let r = ok(
        &mut b,
        "import_profile",
        json!({"data": data.clone(), "passphrase": "фраза для перенесення", "pin": "7777"}),
        &ctx(end + 3, 4),
    );
    assert_eq!(r["recovery_code"].as_str().unwrap().len(), 23);
    let after = ok(&mut b, "home", json!({}), &ctx(end + 4, 4))["skills"].clone();
    assert_eq!(before, after, "стан учня перенесено");
    assert_eq!(
        ok(
            &mut b,
            "note_get",
            json!({"skill": "N-ADD-MULTI"}),
            &ctx(end + 5, 4)
        )["text"],
        "мій конспект переїхав",
        "конспект переноситься разом із профілем"
    );
    assert!(call(
        &mut b,
        "import_profile",
        json!({"data": data.clone(), "passphrase": "хибна", "pin": "7777"}),
        &ctx(1, 1)
    )
    .is_err());
    assert_eq!(
        call(
            &mut b,
            "import_profile",
            json!({"data": "це не base64!", "passphrase": "x", "pin": "7777"}),
            &ctx(1, 1)
        )
        .unwrap_err()
        .code,
        "bad_request"
    );
}

#[test]
fn the_teacher_sheet_has_no_answers_and_lists_the_handed_over_task() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t0 = 6 * DAY + 9 * 3_600;
    create_and_open(&mut a, &ctx(t0, 6));
    ok(&mut a, "session_start", json!({}), &ctx(t0, 6));
    let answer = a.peek_answer().unwrap();
    ok(&mut a, "hand_to_adult", json!({}), &ctx(t0 + 30, 6));
    let sheet = ok(&mut a, "sheet", json!({}), &ctx(t0 + 40, 6));
    assert_eq!(sheet["nickname"], "Мирослава");
    assert_eq!(sheet["ask_adult"].as_array().unwrap().len(), 1);
    assert!(!sheet.to_string().contains(&format!("\"{answer}\"")));
    // Місткість аркуша це стікери лише математики: 35 тем × 4 види.
    assert_eq!(sheet["sticker_capacity"], 140);
    assert_eq!(sheet["subject"], "Математика");
}

#[test]
fn settings_are_clamped_and_persist_across_restarts() {
    let d = TestDir::new();
    let c = ctx(1_000, 1);
    let slot;
    {
        let mut a = app(&d);
        slot = create_and_open(&mut a, &c).0;
        let s = ok(
            &mut a,
            "settings_set",
            json!({"font_scale_percent": 9999, "reduce_motion": true, "sound": true}),
            &c,
        );
        assert_eq!(s["font_scale_percent"], 200);
        ok(&mut a, "lock", json!({}), &c);
    }
    let mut a = app(&d);
    ok(
        &mut a,
        "open_profile",
        json!({"slot": slot, "pin": "4821"}),
        &c,
    );
    let s = ok(&mut a, "settings_get", json!({}), &c);
    assert_eq!(
        (
            s["font_scale_percent"].clone(),
            s["reduce_motion"].clone(),
            s["sound"].clone()
        ),
        (json!(200), json!(true), json!(true))
    );
}

#[test]
fn the_sheet_can_be_downloaded_as_a_pdf_without_answers() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t0 = 8 * DAY + 9 * 3_600;
    create_and_open(&mut a, &ctx(t0, 8));
    ok(&mut a, "session_start", json!({}), &ctx(t0, 8));
    let answer = a.peek_answer().unwrap();
    ok(&mut a, "hand_to_adult", json!({}), &ctx(t0 + 30, 8));
    let r = ok(&mut a, "sheet_pdf", json!({}), &ctx(t0 + 40, 8));
    assert_eq!(r["filename"], "arkush-dlya-vchytelya-math.pdf");
    let bytes = app_core_test_decode(r["data"].as_str().unwrap());
    assert!(bytes.starts_with(b"%PDF-1.4"));
    assert!(bytes.len() > 100_000, "шрифт вбудовано");
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains(&format!("({answer})")));
}

/// Невеликий декодер base64 для тесту (бібліотечний `base64` приватний).
fn app_core_test_decode(s: &str) -> Vec<u8> {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = s.bytes().filter(|b| *b != b'=').collect();
    let mut out = Vec::new();
    let (mut buf, mut bits) = (0u32, 0u32);
    for b in bytes {
        buf = (buf << 6) | A.iter().position(|a| *a == b).unwrap() as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    out
}

/// Рішення A9: за замовчуванням навчання нічим не блокується: немає денного ліміту, перерви, жорсткої межі
/// сесії, а налаштування не потребують PIN дорослого. Час лише радиться (`wrap_up`).
#[test]
fn by_default_nothing_blocks_learning_and_time_is_only_advice() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t0 = 3 * DAY + 9 * 3_600;
    create_and_open(&mut a, &ctx(t0, 3));
    // Налаштування змінюються без PIN дорослого.
    let s = ok(
        &mut a,
        "settings_set",
        json!({"quiet_hours": {"from": 0, "to": 1439}}),
        &ctx(t0, 3),
    );
    assert_eq!(s["quiet_hours"]["to"], 1439);
    // Тихі години без обов'язкових лімітів нічого не забороняють.
    let home = ok(&mut a, "home", json!({}), &ctx(t0, 3));
    assert_eq!(home["can_start"]["ok"], true);
    // Довга сесія: після цільових 20 і навіть 25 хвилин вона не обривається, лише з'являється порада про перерву.
    let mut now = t0;
    let mut model = ok(&mut a, "session_start", json!({}), &ctx(now, 3))["model"].clone();
    let mut advised = false;
    for _ in 0..200 {
        match model["phase"].as_str().unwrap() {
            "feedback" => {
                now += 3;
                model = ok(&mut a, "continue", json!({}), &ctx(now, 3))["model"].clone();
            }
            "task" => {
                advised |= model["wrap_up"] == true;
                now += 20;
                let right = a.peek_answer().unwrap();
                model =
                    ok(&mut a, "answer", json!({"input": right}), &ctx(now, 3))["model"].clone();
            }
            other => panic!("сесія обірвалася (фаза {other}) після {} с", now - t0),
        }
    }
    assert!(now - t0 > 25 * 60, "перевірено понад жорстку межу");
    assert!(
        advised,
        "після 20 хвилин інтерфейс отримує пораду про перерву"
    );
    ok(&mut a, "end_session", json!({}), &ctx(now, 3));
    // Одразу можна почати знову: перерва й денний ліміт не діють.
    let home = ok(&mut a, "home", json!({}), &ctx(now + 1, 3));
    assert_eq!(home["can_start"]["ok"], true);
}

/// Рішення A10/A9: учень сам обирає, що вчити: вільна практика будь-якої навички (навіть «ще рано» за графом),
/// без залежності від розкладу; задачі лише цієї навички.
#[test]
fn free_practice_works_on_any_skill_even_a_locked_one() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t0 = 2 * DAY + 9 * 3_600;
    create_and_open(&mut a, &ctx(t0, 2));
    let home = ok(&mut a, "home", json!({}), &ctx(t0, 2));
    let locked = home["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["state"] == "locked")
        .expect("у новому профілі є закриті навички")["id"]
        .as_str()
        .unwrap()
        .to_string();
    let r = ok(
        &mut a,
        "practice_start",
        json!({"skill": locked}),
        &ctx(t0, 2),
    );
    let title = home["skills"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == locked.as_str())
        .unwrap()["title"]
        .clone();
    let mut model = r["model"].clone();
    for i in 0..6u64 {
        assert_eq!(model["phase"], "task", "крок {i}");
        assert_eq!(model["skill_title"], title);
        let right = a.peek_answer().unwrap();
        let r = ok(
            &mut a,
            "answer",
            json!({"input": right}),
            &ctx(t0 + 30 * (i + 1), 2),
        );
        assert_eq!(r["model"]["phase"], "feedback");
        let c = ok(
            &mut a,
            "continue",
            json!({}),
            &ctx(t0 + 30 * (i + 1) + 5, 2),
        );
        model = c["model"].clone();
    }
    assert_eq!(
        call(
            &mut a,
            "practice_start",
            json!({"skill": "NOPE"}),
            &ctx(t0, 2)
        )
        .unwrap_err()
        .code,
        "bad_request"
    );
}

#[test]
fn notes_are_saved_cleaned_limited_and_survive_a_restart() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(9 * 3_600, 1);
    let (slot, _) = create_and_open(&mut a, &c);
    assert_eq!(
        ok(&mut a, "note_get", json!({"skill": "D-ADD"}), &c)["text"],
        ""
    );
    let text = "Кома під комою.\n\u{0}Рахую як цілі числа\u{1b}";
    let r = ok(
        &mut a,
        "note_set",
        json!({"skill": "D-ADD", "text": text}),
        &c,
    );
    assert_eq!(r["text"], "Кома під комою.\nРахую як цілі числа");
    // Чужа навичка й надмірно довгий текст.
    assert_eq!(
        call(
            &mut a,
            "note_set",
            json!({"skill": "NOPE", "text": "x"}),
            &c
        )
        .unwrap_err()
        .code,
        "bad_request"
    );
    let long = "а".repeat(10_000);
    let r = ok(
        &mut a,
        "note_set",
        json!({"skill": "N-ORDER", "text": long}),
        &c,
    );
    assert_eq!(r["text"].as_str().unwrap().chars().count(), 4_000);
    // Порожній текст видаляє конспект.
    ok(
        &mut a,
        "note_set",
        json!({"skill": "N-ORDER", "text": "  \n "}),
        &c,
    );
    assert_eq!(
        ok(&mut a, "note_get", json!({"skill": "N-ORDER"}), &c)["text"],
        ""
    );
    // Перезапуск застосунку: конспект на місці.
    ok(&mut a, "lock", json!({}), &c);
    let mut b = app(&d);
    ok(
        &mut b,
        "open_profile",
        json!({"slot": slot, "pin": "4821"}),
        &c,
    );
    assert_eq!(
        ok(&mut b, "note_get", json!({"skill": "D-ADD"}), &c)["text"],
        "Кома під комою.\nРахую як цілі числа"
    );
    // PDF конспекту й перелік у відповіді.
    let pdf = ok(&mut b, "notes_pdf", json!({}), &c);
    assert_eq!(pdf["filename"], "moj-konspekt.pdf");
    assert!(pdf["data"].as_str().unwrap().len() > 1000);
    let list = ok(&mut b, "notes_list", json!({}), &c);
    assert_eq!(list["notes"].as_array().unwrap().len(), 1);
}

#[test]
fn too_many_wrong_pins_pause_further_attempts_and_the_right_pin_works_after_the_pause() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t0 = 9 * 3_600;
    let (slot, _) = create_and_open(&mut a, &ctx(t0, 1));
    ok(&mut a, "lock", json!({}), &ctx(t0, 1));
    for i in 0..5u64 {
        let e = call(
            &mut a,
            "open_profile",
            json!({"slot": slot, "pin": "0000"}),
            &ctx(t0 + i, 1),
        )
        .unwrap_err();
        assert_eq!(e.code, "wrong_pin");
    }
    let e = call(
        &mut a,
        "open_profile",
        json!({"slot": slot, "pin": "4821"}),
        &ctx(t0 + 6, 1),
    )
    .unwrap_err();
    assert_eq!(e.code, "pin_locked");
    assert!(e.message.contains("Зачекай"), "{}", e.message);
    ok(
        &mut a,
        "open_profile",
        json!({"slot": slot, "pin": "4821"}),
        &ctx(t0 + 100, 1),
    );
}

#[test]
fn the_adult_pin_is_also_rate_limited() {
    let d = TestDir::new();
    let mut a = app(&d);
    let t0 = 9 * 3_600;
    create_and_open(&mut a, &ctx(t0, 1));
    ok(&mut a, "adult_set_pin", json!({"pin": "2468"}), &ctx(t0, 1));
    ok(&mut a, "adult_lock", json!({}), &ctx(t0, 1));
    for i in 0..5u64 {
        let e = call(
            &mut a,
            "adult_unlock",
            json!({"pin": "0000"}),
            &ctx(t0 + i, 1),
        )
        .unwrap_err();
        assert_eq!(e.code, "wrong_adult_pin");
    }
    let e = call(
        &mut a,
        "adult_unlock",
        json!({"pin": "2468"}),
        &ctx(t0 + 6, 1),
    )
    .unwrap_err();
    assert_eq!(e.code, "pin_locked");
    ok(
        &mut a,
        "adult_unlock",
        json!({"pin": "2468"}),
        &ctx(t0 + 200, 1),
    );
}

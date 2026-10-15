#![cfg(not(feature = "reviewer"))]
mod common;

use common::{app, call, create_and_open, ctx, ok, TestDir};
use serde_json::{json, Value};
use tutor_core::lesson::{LessonBlock, LessonSource};

fn answer_right(a: &mut app_core::App, t: u64, day: u32) -> Value {
    let right = a.peek_answer().unwrap();
    ok(a, "answer", json!({"input": right}), &ctx(t, day))
}

/// Проходить урок чесно (приклади, самостійні відповіді, пояснення).
fn complete_lesson(a: &mut app_core::App, skill: &str, now: u64, day: u32) {
    let c = ctx(now, day);
    let (mut ex, mut ck, mut xp) = (0u64, 0u64, 0u64);
    for b in &subjects::SubjectLessons.lesson(skill).unwrap().blocks {
        match b {
            LessonBlock::Example { .. } => {
                ok(
                    a,
                    "lesson_example",
                    json!({"skill": skill, "index": ex}),
                    &c,
                );
                ex += 1;
            }
            LessonBlock::SelfCheck { answer, .. } => {
                ok(
                    a,
                    "lesson_check",
                    json!({"skill": skill, "index": ck, "input": answer}),
                    &c,
                );
                ck += 1;
            }
            LessonBlock::Quiz { correct, .. } => {
                ok(
                    a,
                    "lesson_check",
                    json!({"skill": skill, "index": ck, "input": (correct + 1).to_string()}),
                    &c,
                );
                ck += 1;
            }
            LessonBlock::Explain { key_ideas, .. } => {
                let text = "Я поясню це своїми словами так: спочатку розумію ідею, потім перевіряю на прикладі.";
                ok(
                    a,
                    "lesson_explain",
                    json!({"skill": skill, "index": xp, "text": text}),
                    &c,
                );
                let rec: Vec<bool> = key_ideas.iter().map(|_| true).collect();
                ok(
                    a,
                    "lesson_explain",
                    json!({"skill": skill, "index": xp, "text": text, "recalled": rec}),
                    &c,
                );
                xp += 1;
            }
            _ => {}
        }
    }
}

/// День старанності: п'ять справжніх задач і (за бажанням) урок.
fn study_day(a: &mut app_core::App, day: u32, lesson: Option<&str>, think_secs: u64) {
    let mut now = u64::from(day) * 86_400 + 9 * 3_600;
    ok(
        a,
        "practice_start",
        json!({"skill": "D-ADD"}),
        &ctx(now, day),
    );
    for _ in 0..5 {
        now += think_secs;
        answer_right(a, now, day);
        now += 2;
        ok(a, "continue", json!({}), &ctx(now, day));
    }
    ok(a, "end_session", json!({}), &ctx(now + 5, day));
    if let Some(l) = lesson {
        complete_lesson(a, l, now + 60, day);
    }
}

fn status(a: &mut app_core::App, day: u32) -> Value {
    ok(
        a,
        "week_status",
        json!({}),
        &ctx(u64::from(day) * 86_400 + 20 * 3_600, day),
    )
}

#[test]
fn the_ruler_counts_effort_and_a_sufficient_week_lets_the_child_choose_one_of_three_items() {
    let d = TestDir::new();
    let mut a = app(&d);
    create_and_open(&mut a, &ctx(4 * 86_400 + 9 * 3_600, 4));
    // Понеділок: лише п'ять задач: одна одиниця, нагороди ще немає.
    study_day(&mut a, 4, None, 15);
    let s = status(&mut a, 4);
    assert_eq!(s["current"]["units"], 1);
    assert_eq!(s["current"]["tier"], 0);
    assert_eq!(s["current"]["can_claim"], false);
    // Ще чотири дні із задачами й уроками: 5 + 4 = 9 одиниць.
    for (day, lesson) in [
        (5, "D-ADD"),
        (6, "D-SUB"),
        (7, "F-ADD-SAME"),
        (8, "F-SUB-SAME"),
    ] {
        study_day(&mut a, day, Some(lesson), 15);
    }
    let s = status(&mut a, 8);
    assert_eq!(s["current"]["units"], 9, "{s}");
    assert_eq!(s["current"]["tier"], 1);
    assert_eq!(s["current"]["breakdown"]["lessons"], 4);
    let offer = s["current"]["offer"].as_array().unwrap().clone();
    assert_eq!(offer.len(), 3);
    assert!(
        offer.iter().all(|i| i["rarity"] == "common"),
        "звичайний рівень пропонує звичайне"
    );
    // Вибір: лише з пропозиції.
    let week = s["current"]["week"].as_u64().unwrap();
    let c = ctx(8 * 86_400 + 21 * 3_600, 8);
    assert!(call(
        &mut a,
        "week_claim",
        json!({"week": week, "item": "hat_crown"}),
        &c
    )
    .is_err());
    let pick = offer[1]["id"].as_str().unwrap().to_string();
    let r = ok(
        &mut a,
        "week_claim",
        json!({"week": week, "item": pick}),
        &c,
    );
    assert_eq!(r["claimed"]["id"], pick.as_str());
    assert_eq!(r["status"]["collected"], 1);
    // Раз на тиждень: другий вибір неможливий.
    assert!(call(
        &mut a,
        "week_claim",
        json!({"week": week, "item": offer[0]["id"].as_str().unwrap()}),
        &c
    )
    .is_err());
    // Предмет у шафі й одягнений; його можна зняти (якщо це не обов'язковий шар) і одягнути знову.
    let cat = ok(&mut a, "hero_catalog", json!({}), &c);
    assert_eq!(cat["collected"], 1);
    let it = offer[1].clone();
    let slot = it["slot"].as_str().unwrap();
    let removable = cat["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["slot"] == slot)
        .unwrap()["removable"]
        == true;
    if removable {
        ok(&mut a, "hero_set", json!({"slot": slot, "item": ""}), &c);
        ok(&mut a, "hero_set", json!({"slot": slot, "item": pick}), &c);
    }
}

#[test]
fn answering_in_a_blink_does_not_count_as_effort_and_unowned_items_cannot_be_worn() {
    let d = TestDir::new();
    let mut a = app(&d);
    create_and_open(&mut a, &ctx(4 * 86_400 + 9 * 3_600, 4));
    study_day(&mut a, 4, None, 1);
    assert_eq!(
        status(&mut a, 4)["current"]["units"],
        0,
        "задачі за секунду не рахуються"
    );
    let c = ctx(4 * 86_400 + 20 * 3_600, 4);
    assert!(call(
        &mut a,
        "hero_set",
        json!({"slot": "hat", "item": "hat_crown"}),
        &c
    )
    .is_err());
    assert!(call(
        &mut a,
        "hero_set",
        json!({"slot": "outfit", "item": ""}),
        &c
    )
    .is_err());
    assert!(call(
        &mut a,
        "hero_set",
        json!({"slot": "hat", "item": "outfit_space"}),
        &c
    )
    .is_err());
    // Базовий вигляд вільний.
    let r = ok(
        &mut a,
        "hero_set",
        json!({"skin": 4, "hair_color": 5, "slot": "hair", "item": "hair_curly"}),
        &c,
    );
    assert_eq!(r["skin"], 4);
    assert_eq!(r["equipped"]["hair"], "hair_curly");
    assert!(call(&mut a, "hero_set", json!({"skin": 99}), &c).is_err());
    // Герой потрапляє на головну й у PDF-постер.
    let home = ok(&mut a, "home", json!({}), &c);
    assert!(home["hero"]["px"].as_array().unwrap().len() > 20);
    let pdf = ok(&mut a, "hero_pdf", json!({}), &c);
    assert_eq!(pdf["filename"], "mij-heroj.pdf");
    assert!(pdf["data"].as_str().unwrap().len() > 1000);
}

#[test]
fn an_unclaimed_reward_waits_for_the_child_in_the_following_weeks() {
    let d = TestDir::new();
    let mut a = app(&d);
    create_and_open(&mut a, &ctx(4 * 86_400 + 9 * 3_600, 4));
    for (day, lesson) in [
        (4, "D-ADD"),
        (5, "D-SUB"),
        (6, "F-ADD-SAME"),
        (7, "F-SUB-SAME"),
        (8, "F-OF"),
    ] {
        study_day(&mut a, day, Some(lesson), 15);
    }
    // Наступного понеділка нова лінійка порожня, а нагорода за минулий тиждень чекає.
    let s = status(&mut a, 11);
    assert_eq!(s["current"]["units"], 0);
    let pending = s["pending"].as_array().unwrap();
    assert_eq!(pending.len(), 1);
    assert!(pending[0]["can_claim"] == true);
    let home = ok(&mut a, "home", json!({}), &ctx(11 * 86_400 + 9 * 3_600, 11));
    assert_eq!(home["week_claimable"], 1);
    // Через дев'ять тижнів вона вже згорає (її можна забрати протягом восьми тижнів).
    let s = status(&mut a, 11 + 9 * 7);
    assert!(s["pending"].as_array().unwrap().is_empty());
}

#[test]
fn the_thanks_card_needs_the_childs_own_words_and_produces_a_pdf_and_a_text_file() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(4 * 86_400 + 9 * 3_600, 4);
    create_and_open(&mut a, &c);
    let args = json!({"to": "Вчительці математики", "text": "Дякую, що ви терпляче пояснюєте дроби, мені стало легше.", "from": "Оля"});
    let pdf = ok(&mut a, "thanks_pdf", args.clone(), &c);
    assert_eq!(pdf["filename"], "lyst-podyaky.pdf");
    assert!(pdf["data"].as_str().unwrap().len() > 1000);
    let txt = ok(&mut a, "thanks_text", args, &c);
    assert_eq!(txt["filename"], "lyst-podyaky.txt");
    assert!(txt["data"].as_str().unwrap().len() > 40);
    // Без справжніх слів листівки немає (нічого не створюється з порожнього чи безглуздого тексту).
    assert!(call(&mut a, "thanks_pdf", json!({"text": ""}), &c).is_err());
    assert!(call(
        &mut a,
        "thanks_pdf",
        json!({"text": "ааааааааааааааааааааааааа"}),
        &c
    )
    .is_err());
    // Аркуш для вчителя тепер оформлений PDF.
    let sheet = ok(&mut a, "sheet_pdf", json!({"subject": "ukrainian"}), &c);
    assert!(sheet["filename"].as_str().unwrap().contains("ukrainian"));
}

#[test]
fn a_new_profile_starts_with_the_childs_own_hero_and_the_profile_list_shows_it() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(4 * 86_400 + 9 * 3_600, 4);
    ok(
        &mut a,
        "create_profile",
        json!({"nickname": "Оля", "pin": "1234", "skin": 4, "hair_color": 6, "hair_style": "hair_curly"}),
        &c,
    );
    let cat = ok(&mut a, "hero_catalog", json!({}), &c);
    assert_eq!(cat["look"]["skin"], 4);
    assert_eq!(cat["look"]["hair_color"], 6);
    assert_eq!(cat["look"]["equipped"]["hair"], "hair_curly");
    // У списку профілів (до входу) видно героя з обраними кольорами, а не тваринку.
    let list = ok(&mut a, "profiles", json!({}), &c);
    assert_eq!(list["profiles"][0]["avatar"], 4 * 8 + 6);
    assert!(list["profiles"][0]["px"].as_array().unwrap().len() > 15);
    // Зміна вигляду оновлює й публічний код.
    ok(&mut a, "hero_set", json!({"skin": 1, "hair_color": 2}), &c);
    let list = ok(&mut a, "profiles", json!({}), &c);
    assert_eq!(list["profiles"][0]["avatar"], 8 + 2);
    // Чужа зачіска (не вільна) чи неіснуюча при створенні ігнорується.
    ok(&mut a, "lock", json!({}), &c);
    ok(
        &mut a,
        "create_profile",
        json!({"nickname": "Іра", "pin": "4321", "skin": 0, "hair_color": 0, "hair_style": "hat_crown"}),
        &c,
    );
    let cat = ok(&mut a, "hero_catalog", json!({}), &c);
    assert_eq!(cat["look"]["equipped"]["hair"], "hair_short");
}

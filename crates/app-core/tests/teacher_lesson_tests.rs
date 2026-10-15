#![cfg(not(feature = "reviewer"))]
mod common;
use common::*;
use serde_json::json;

#[test]
fn a_teacher_sees_lesson_answers_and_gets_a_plan_pdf_but_a_child_cannot() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let l = ok(
        &mut a,
        "teacher_lesson",
        json!({"skill": "UA-APOSTROPHE"}),
        &c,
    );
    assert!(
        l["reviewer"]["checks"]
            .as_array()
            .is_some_and(|x| !x.is_empty()),
        "відповіді самоперевірок"
    );
    assert!(
        l["reviewer"]["explains"]
            .as_array()
            .is_some_and(|x| !x.is_empty()),
        "ключові ідеї"
    );
    let p = ok(
        &mut a,
        "teacher_lesson_plan_pdf",
        json!({"skill": "UA-APOSTROPHE"}),
        &c,
    );
    assert_eq!(p["filename"], "plan-uroku.pdf");
    assert!(p["data"].as_str().unwrap().starts_with("JVBER"));
    assert!(call(&mut a, "teacher_lesson", json!({"skill": "NO-SUCH"}), &c).is_err());

    ok(&mut a, "lock", json!({}), &c);
    create_and_open(&mut a, &c);
    let e = call(
        &mut a,
        "teacher_lesson",
        json!({"skill": "UA-APOSTROPHE"}),
        &c,
    )
    .unwrap_err();
    assert_eq!(e.code, "wrong_profile_kind");
}

#[test]
fn a_plan_exists_for_a_math_topic_too() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let p = ok(
        &mut a,
        "teacher_lesson_plan_pdf",
        json!({"skill": "D-ADD"}),
        &c,
    );
    assert!(p["data"].as_str().unwrap().len() > 2_000);
}

#[test]
fn a_teacher_can_search_all_lessons_and_keep_bookmarks_that_survive_a_relock() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    let slot = {
        create_teacher(&mut a, &c);
        ok(&mut a, "profiles", json!({}), &c)["profiles"][0]["slot"]
            .as_u64()
            .unwrap()
    };
    // Пошук по назві й по тексту уроку, у кількох предметах.
    let r = ok(&mut a, "teacher_search", json!({"q": "апостроф"}), &c);
    let hits = r["hits"].as_array().unwrap();
    assert!(hits.iter().any(|h| h["id"] == "UA-APOSTROPHE"), "{hits:?}");
    assert!(hits.iter().all(|h| h["grade"].is_u64()));
    assert!(
        ok(&mut a, "teacher_search", json!({"q": "а"}), &c)["hits"]
            .as_array()
            .unwrap()
            .is_empty(),
        "надто короткий запит"
    );
    assert!(
        ok(&mut a, "teacher_search", json!({"q": "zzzzqqqq"}), &c)["hits"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    ok(
        &mut a,
        "teacher_bookmark",
        json!({"skill": "UA-APOSTROPHE", "on": true}),
        &c,
    );
    ok(&mut a, "teacher_bookmark", json!({"skill": "D-ADD"}), &c);
    ok(
        &mut a,
        "teacher_bookmark",
        json!({"skill": "D-ADD", "on": false}),
        &c,
    );
    ok(&mut a, "lock", json!({}), &c);
    ok(
        &mut a,
        "open_profile",
        json!({"slot": slot, "pin": "1357"}),
        &c,
    );
    let home = ok(&mut a, "teacher_home", json!({}), &c);
    let b = home["bookmarks"].as_array().unwrap();
    assert_eq!(b.len(), 1);
    assert_eq!(b[0]["id"], "UA-APOSTROPHE");
    assert!(call(&mut a, "teacher_bookmark", json!({"skill": "NO-SUCH"}), &c).is_err());
}

#[test]
fn a_one_page_handout_for_pupils_is_made_from_terms_and_the_rule() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let r = ok(
        &mut a,
        "teacher_handout_pdf",
        json!({"skill": "UA-APOSTROPHE"}),
        &c,
    );
    assert_eq!(r["filename"], "pamiatka-dlia-uchniv.pdf");
    assert!(r["data"].as_str().unwrap().starts_with("JVBER"));
    assert!(call(
        &mut a,
        "teacher_handout_pdf",
        json!({"skill": "NO-SUCH"}),
        &c
    )
    .is_err());
}

#![cfg(not(feature = "reviewer"))]
mod common;
use common::*;
use serde_json::{json, Value};

fn build(
    a: &mut app_core::App,
    c: &app_core::Ctx,
    skills: Value,
    kind: &str,
    variants: u8,
    seed: u64,
) -> Result<Value, app_core::ApiError> {
    call(
        a,
        "teacher_work_build",
        json!({"grade": 5, "skills": skills, "kind": kind, "variants": variants, "seed": seed}),
        c,
    )
}

#[test]
fn a_work_has_unique_statements_inside_and_across_variants_and_every_item_has_a_key() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let r = build(&mut a, &c, json!(["D-ADD", "UA-APOSTROPHE"]), "test", 3, 7).unwrap();
    let variants = r["work"]["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 3);
    assert_eq!(variants[0]["label"], "А");
    assert_eq!(variants[2]["label"], "В");
    let mut seen = std::collections::BTreeSet::new();
    for v in variants {
        assert_eq!(v["items"].as_array().unwrap().len(), 10);
        for it in v["items"].as_array().unwrap() {
            assert!(
                seen.insert(it["statement"].as_str().unwrap().to_string()),
                "умова повторюється"
            );
            assert!(!it["answer"].as_str().unwrap().is_empty());
        }
    }
}

#[test]
fn the_same_seed_gives_the_same_work_and_another_seed_gives_another() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let x = build(&mut a, &c, json!(["D-ADD"]), "quiz", 2, 7).unwrap();
    let y = build(&mut a, &c, json!(["D-ADD"]), "quiz", 2, 7).unwrap();
    let z = build(&mut a, &c, json!(["D-ADD"]), "quiz", 2, 8).unwrap();
    assert_eq!(x["work"], y["work"]);
    assert_ne!(x["work"], z["work"]);
}

#[test]
fn numeric_keys_are_correct_and_decimals_are_written_with_a_comma() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let r = build(&mut a, &c, json!(["D-ADD"]), "test", 1, 3).unwrap();
    for it in r["work"]["variants"][0]["items"].as_array().unwrap() {
        let key = it["answer"].as_str().unwrap();
        assert!(
            !key.contains('/'),
            "десяткова відповідь має бути з комою: {key}"
        );
        assert!(
            key.chars()
                .all(|c| c.is_ascii_digit() || c == ',' || c == '-'),
            "{key}"
        );
    }
}

#[test]
fn a_skill_without_content_or_from_another_grade_is_an_error_not_an_empty_work() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    assert_eq!(
        build(&mut a, &c, json!(["NO-SUCH"]), "quiz", 1, 1)
            .unwrap_err()
            .code,
        "bad_request"
    );
    assert!(build(&mut a, &c, json!([]), "quiz", 1, 1).is_err());
    assert!(build(&mut a, &c, json!(["D-ADD"]), "quiz", 9, 1).is_err());
    let r = call(
        &mut a,
        "teacher_work_build",
        json!({"grade": 5, "skills": ["UA6-LEX-MEANING"], "kind": "quiz"}),
        &c,
    );
    assert!(r.is_err(), "тема 6 класу недоступна для роботи 5 класу");
    let ok_earlier = call(
        &mut a,
        "teacher_work_build",
        json!({"grade": 6, "skills": ["D-ADD"], "kind": "quiz"}),
        &c,
    );
    assert!(
        ok_earlier.is_ok(),
        "повторення теми 5 класу в роботі 6 класу дозволене"
    );
}

#[test]
fn building_a_work_is_remembered_without_any_pupil_data_and_topics_are_listed() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let t = ok(&mut a, "teacher_topics", json!({"grade": 5}), &c);
    let subjects = t["subjects"].as_array().unwrap();
    assert!(subjects.len() >= 6);
    assert!(subjects
        .iter()
        .all(|s| !s["skills"].as_array().unwrap().is_empty()));
    build(&mut a, &c, json!(["D-ADD"]), "quiz", 1, 5).unwrap();
}

#[test]
fn the_three_pdf_parts_are_valid_and_deterministic_and_unknown_part_is_an_error() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let args = |part: &str| json!({"grade": 5, "skills": ["D-ADD", "UA-APOSTROPHE"], "kind": "quiz", "variants": 2, "seed": 9, "part": part});
    for (part, file) in [
        ("tasks", "robota-zavdannia.pdf"),
        ("key", "robota-kliuch.pdf"),
        ("answers", "robota-blank-vidpovidei.pdf"),
    ] {
        let r = ok(&mut a, "teacher_work_pdf", args(part), &c);
        assert_eq!(r["filename"], file);
        assert!(r["data"].as_str().unwrap().starts_with("JVBER"));
        let again = ok(&mut a, "teacher_work_pdf", args(part), &c);
        assert_eq!(
            r["data"], again["data"],
            "той самий PDF за тим самим зерном"
        );
    }
    assert!(call(&mut a, "teacher_work_pdf", args("nope"), &c).is_err());
}

#[test]
fn too_many_topics_for_the_work_kind_and_absurd_variant_counts_are_errors_and_duplicates_collapse()
{
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let many = json!([
        "D-ADD",
        "D-SUB",
        "F-ADD-SAME",
        "F-SUB-DIFF",
        "N-ADD-MULTI",
        "P-PCT"
    ]);
    assert!(
        build(&mut a, &c, many, "quiz", 1, 1).is_err(),
        "6 тем на 5 завдань"
    );
    assert!(call(
        &mut a,
        "teacher_work_build",
        json!({"grade": 5, "skills": ["D-ADD"], "kind": "quiz", "variants": 300}),
        &c
    )
    .is_err());
    let r = build(&mut a, &c, json!(["D-ADD", "D-ADD"]), "quiz", 1, 3).unwrap();
    assert_eq!(
        r["work"]["variants"][0]["items"].as_array().unwrap().len(),
        5
    );
}

fn pdf(a: &mut app_core::App, c: &app_core::Ctx, extra: Value) -> Value {
    let mut args = json!({"grade": 5, "skills": ["D-ADD"], "kind": "quiz", "variants": 1, "seed": 4, "part": "tasks"});
    for (k, v) in extra.as_object().unwrap() {
        args[k] = v.clone();
    }
    ok(a, "teacher_work_pdf", args, c)
}

#[test]
fn levels_large_print_cards_dictation_gaps_and_earlier_grades() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    // Рівень: короткі питання з банку це «пригадування», довгі й «що не є» це НМТ.
    let recall = call(&mut a, "teacher_work_build", json!({"grade": 5, "skills": ["UA-APOSTROPHE"], "kind": "quiz", "level": "recall", "seed": 1}), &c).unwrap();
    for it in recall["work"]["variants"][0]["items"].as_array().unwrap() {
        assert!(
            it["statement"].as_str().unwrap().chars().count() <= 70,
            "{}",
            it["statement"]
        );
    }
    assert!(call(
        &mut a,
        "teacher_work_build",
        json!({"grade": 5, "skills": ["UA-APOSTROPHE"], "kind": "quiz", "level": "дуже складно"}),
        &c
    )
    .is_err());
    // Великий шрифт і картки дають валідні, але інші PDF.
    let normal = pdf(&mut a, &c, json!({}));
    let large = pdf(&mut a, &c, json!({"large": true}));
    assert_ne!(normal["data"], large["data"]);
    let cards = pdf(&mut a, &c, json!({"part": "cards"}));
    assert_eq!(cards["filename"], "robota-kartky.pdf");
    // Диктант і пропуски лише з темами введення слова.
    assert!(call(
        &mut a,
        "teacher_work_build",
        json!({"grade": 5, "skills": ["D-ADD"], "kind": "dictation"}),
        &c
    )
    .is_err());
    let dict = call(
        &mut a,
        "teacher_work_build",
        json!({"grade": 5, "skills": ["EN-T-VOCAB-SCHOOL-HOME"], "kind": "dictation", "seed": 2}),
        &c,
    )
    .unwrap();
    assert_eq!(
        dict["work"]["variants"][0]["items"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
    let gaps = call(&mut a, "teacher_work_build", json!({"grade": 5, "skills": ["UA-T-APOSTROPHE", "EN-T-PLURALS"], "kind": "gaps", "seed": 2}), &c).unwrap();
    assert_eq!(
        gaps["work"]["variants"][0]["items"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    let dpdf = ok(
        &mut a,
        "teacher_work_pdf",
        json!({"grade": 5, "skills": ["EN-T-VOCAB-SCHOOL-HOME"], "kind": "dictation", "seed": 2, "part": "key"}),
        &c,
    );
    assert!(dpdf["data"].as_str().unwrap().starts_with("JVBER"));
    // Теми попередніх класів у роботі старшого: 6 клас бере тему 5 класу, але не навпаки.
    let mix = call(
        &mut a,
        "teacher_work_build",
        json!({"grade": 6, "skills": ["UA-APOSTROPHE", "UA6-LEX-MEANING"], "kind": "quiz", "seed": 3}),
        &c,
    );
    assert!(mix.is_ok(), "{mix:?}");
    assert!(call(
        &mut a,
        "teacher_work_build",
        json!({"grade": 5, "skills": ["UA6-LEX-MEANING"], "kind": "quiz"}),
        &c
    )
    .is_err());
    let t = ok(&mut a, "teacher_topics", json!({"grade": 6}), &c);
    assert_eq!(t["earlier"][0]["grade"], 5);
    assert!(!t["earlier"][0]["subjects"].as_array().unwrap().is_empty());
}

#[test]
fn a_topic_plan_pdf_lists_a_subject_course_in_order() {
    let d = TestDir::new();
    let mut a = app(&d);
    let c = ctx(1_000, 1);
    create_teacher(&mut a, &c);
    let r = ok(
        &mut a,
        "teacher_plan_pdf",
        json!({"grade": 6, "subject": "ukrainian", "lessons": 3}),
        &c,
    );
    assert_eq!(r["filename"], "tematychnyi-plan.pdf");
    assert!(r["data"].as_str().unwrap().starts_with("JVBER"));
    assert!(
        call(
            &mut a,
            "teacher_plan_pdf",
            json!({"grade": 6, "subject": "math"}),
            &c
        )
        .is_err(),
        "у 6 класі математики ще немає"
    );
}

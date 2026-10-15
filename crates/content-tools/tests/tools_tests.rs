use content_tools::{csv, review, sign};
use serde_json::Value;

fn packs() -> Vec<Value> {
    subjects::current_packs()
        .into_iter()
        .map(|p| serde_json::to_value(p).unwrap())
        .collect()
}

#[test]
fn csv_round_trips_commas_quotes_and_newlines() {
    let rows = vec![
        vec![
            "a".to_string(),
            "б, в".to_string(),
            "\"лапки\"".to_string(),
            "два\nрядки".to_string(),
        ],
        vec![String::new(), "x".to_string(), String::new(), String::new()],
    ];
    let text = csv::write(&rows);
    assert!(text.starts_with('\u{feff}'));
    assert_eq!(csv::read(&text), rows);
}

#[test]
fn the_review_covers_every_task_and_lesson_text() {
    let ps = packs();
    let rows: usize = ps.iter().map(|p| review::rows_for(p).len()).sum();
    let prompts: usize = ps
        .iter()
        .flat_map(review::rows_for)
        .filter(|r| (r[2] == "bank" || r[2] == "typed") && r[4] == "prompt")
        .count();
    let expected: usize = subjects::all_banks().iter().map(|b| b.len()).sum::<usize>()
        + subjects::all_typed_banks()
            .iter()
            .map(|b| b.len())
            .sum::<usize>();
    assert_eq!(prompts, expected, "кожне завдання є у звіті");
    assert!(rows > prompts * 3);
    let html = review::to_html(&ps[0]);
    assert!(html.contains("<h2>") && !html.contains("<script"));
}

#[test]
fn corrections_are_applied_guarded_and_bump_the_version() {
    let mut ps = packs();
    let rows = review::rows_for(&ps[0]);
    let mut table: Vec<Vec<String>> = vec![review::HEADER.iter().map(|s| s.to_string()).collect()];
    // Виправлення пояснення першого завдання першого банку + «застарілий» рядок + видалення завдання.
    let expl = rows
        .iter()
        .find(|r| r[2] == "bank" && r[4] == "explanation")
        .unwrap()
        .clone();
    let mut fix = expl.clone();
    fix[6] = "fix".into();
    fix[7] = "Нове, точніше пояснення.".into();
    let mut stale = rows
        .iter()
        .find(|r| r[2] == "bank" && r[4] == "correct")
        .unwrap()
        .clone();
    stale[5] = "інший текст, ніж у пакеті".into();
    stale[6] = "fix".into();
    stale[7] = "x".into();
    let skill = expl[1].clone();
    let before_len = ps[0]["banks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["skill"] == skill.as_str())
        .unwrap()["items"]
        .as_array()
        .unwrap()
        .len();
    let mut rm = rows
        .iter()
        .find(|r| {
            r[1] == skill
                && r[2] == "bank"
                && r[4] == "prompt"
                && r[3] == (before_len - 1).to_string()
        })
        .unwrap()
        .clone();
    rm[6] = "remove".into();
    table.extend([fix, stale, rm]);
    let version = ps[0]["version"].as_u64().unwrap();
    let r = review::apply(&mut ps, &csv::write(&table));
    assert_eq!((r.fixed, r.removed), (1, 1), "{r:?}");
    assert_eq!(r.skipped.len(), 1, "застарілий рядок пропущено");
    assert_eq!(ps[0]["version"].as_u64().unwrap(), version + 1);
    let b = ps[0]["banks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["skill"] == skill.as_str())
        .unwrap();
    assert_eq!(b["items"].as_array().unwrap().len(), before_len - 1);
    assert_eq!(
        b["items"][expl[3].parse::<usize>().unwrap()]["explanation"],
        "Нове, точніше пояснення."
    );
    // Змінений пакет усе ще проходить перевірку застосунку.
    assert!(subjects::pack::parse(serde_json::to_string(&ps[0]).unwrap().as_bytes()).is_ok());
}

#[test]
fn signatures_verify_only_the_exact_bytes_with_the_right_key() {
    let (private, public) = sign::keygen();
    let key: [u8; 32] = sign::unhex(&public).unwrap().try_into().unwrap();
    let data = b"{\"hello\":1}";
    let sig = sign::sign(&private, data).unwrap();
    assert!(subjects::pack::verify_signature(data, &sig, &key).is_ok());
    assert!(subjects::pack::verify_signature(b"{\"hello\":2}", &sig, &key).is_err());
    let (_, other_public) = sign::keygen();
    let other: [u8; 32] = sign::unhex(&other_public).unwrap().try_into().unwrap();
    assert!(subjects::pack::verify_signature(data, &sig, &other).is_err());
    assert!(subjects::pack::verify_signature(data, &sig[..10], &key).is_err());
    assert_eq!(sign::public_from_private(&private).unwrap(), key);
}

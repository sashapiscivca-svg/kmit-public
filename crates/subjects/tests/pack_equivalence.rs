//! Пакети вмісту: розбір, збирання й експорт мають бути оборотними; пошкоджені пакети відхиляються.

use subjects::current_packs;
use subjects::pack::{build, export, parse, PackError};

#[test]
fn every_subject_survives_a_round_trip_through_json_unchanged() {
    let packs = current_packs();
    assert!(packs.len() >= 6);
    for p in packs {
        let bytes = serde_json::to_vec(&p).unwrap();
        let back = parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", p.id));
        assert_eq!(back, p, "{}: пакет змінився після читання", p.id);
        let built = build(&back);
        let again = export(
            &p.id,
            p.version,
            &built.subject,
            &built.banks,
            &built.typed,
            &built.lessons,
        );
        assert_eq!(again, p, "{}: експорт збудованого вмісту інший", p.id);
    }
}

#[test]
fn damaged_or_hostile_packs_are_rejected_with_a_reason() {
    assert!(matches!(parse(b"not json"), Err(PackError::Json(_))));
    let base = || current_packs().remove(0);
    let mut p = base();
    p.schema = "other".into();
    assert_eq!(
        parse(&serde_json::to_vec(&p).unwrap()),
        Err(PackError::Schema)
    );
    let mut p = base();
    p.skills[0].prerequisites = vec!["NO-SUCH".into()];
    assert!(matches!(
        parse(&serde_json::to_vec(&p).unwrap()),
        Err(PackError::Invalid(_))
    ));
    let mut p = base();
    let first = p.skills[0].id.clone();
    p.skills[0].prerequisites = vec![p.skills[1].id.clone()];
    p.skills[1].prerequisites = vec![first];
    assert!(
        matches!(
            parse(&serde_json::to_vec(&p).unwrap()),
            Err(PackError::Invalid(_))
        ),
        "цикл передумов"
    );
    let mut p = base();
    p.id = "Bad Id!".into();
    assert!(matches!(
        parse(&serde_json::to_vec(&p).unwrap()),
        Err(PackError::Invalid(_))
    ));
    let mut p = base();
    p.banks[0].items[0].wrong.truncate(1);
    assert!(matches!(
        parse(&serde_json::to_vec(&p).unwrap()),
        Err(PackError::Invalid(_))
    ));
}

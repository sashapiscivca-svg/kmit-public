use tutor_core::album::{Album, StickerKind, TaskFacts};

fn facts(solved: bool, wrong: u32, hints: u8, adult: bool) -> TaskFacts {
    TaskFacts {
        solved,
        wrong_attempts: wrong,
        hints_used: hints,
        asked_adult: adult,
    }
}

#[test]
fn a_clean_first_try_gives_no_strategy_sticker() {
    let mut a = Album::new();
    assert!(a
        .award_for_task("F-ADD-SAME", facts(true, 0, 0, false))
        .is_empty());
    assert_eq!(a.count(), 0);
}

#[test]
fn each_strategy_has_its_own_condition() {
    let mut a = Album::new();
    let got = a.award_for_task("S", facts(true, 1, 0, false));
    assert_eq!(
        got.iter().map(|s| s.kind).collect::<Vec<_>>(),
        vec![StickerKind::SelfCorrected]
    );

    let got = a.award_for_task("S", facts(true, 2, 0, false));
    // Ще один стікер (не здалася); «сама виправила» вже є й повторно не дається.
    assert_eq!(
        got.iter().map(|s| s.kind).collect::<Vec<_>>(),
        vec![StickerKind::Persisted]
    );

    let got = a.award_for_task("S", facts(true, 1, 2, false));
    assert_eq!(
        got.iter().map(|s| s.kind).collect::<Vec<_>>(),
        vec![StickerKind::AskedAndSolved]
    );
    assert_eq!(a.count(), 3);
}

#[test]
fn an_unsolved_task_or_an_adult_handoff_earns_nothing() {
    let mut a = Album::new();
    assert!(a.award_for_task("S", facts(false, 5, 4, true)).is_empty());
    assert!(
        a.award_for_task("S", facts(true, 0, 4, true)).is_empty(),
        "передано дорослому: це не «розв'язала сама»"
    );
    assert!(
        a.award_for_task("S", facts(true, 2, 0, true)).is_empty(),
        "передана дорослому задача не дає жодних стратегічних стікерів"
    );
    assert_eq!(a.count(), 0);
}

#[test]
fn stickers_are_given_once_per_skill_and_never_taken_away() {
    let mut a = Album::new();
    assert!(a.award_mastery("A").is_some());
    assert!(a.award_mastery("A").is_none(), "повторно не видається");
    assert!(a.award_mastery("B").is_some());
    assert!(a.has(StickerKind::Mastery, "A"));
    // Невдачі нічого не відбирають: альбом лише зростає.
    let before = a.count();
    a.award_for_task("A", facts(false, 9, 4, true));
    assert_eq!(a.count(), before);
}

#[test]
fn the_album_is_finite() {
    assert_eq!(Album::capacity(4), 16);
    let mut a = Album::new();
    for skill in ["A", "B"] {
        a.award_mastery(skill);
        a.award_for_task(skill, facts(true, 2, 1, false));
        a.award_for_task(skill, facts(true, 1, 0, false));
        a.award_for_task(skill, facts(true, 1, 3, false));
    }
    assert!(a.count() <= Album::capacity(2));
    assert_eq!(a.count(), Album::capacity(2));
}

#[test]
fn keys_round_trip_and_unknown_keys_are_ignored() {
    let mut a = Album::new();
    a.award_mastery("F-ADD-SAME");
    a.award_for_task("D-ADD", facts(true, 2, 1, false));
    let keys = a.keys();
    let b = Album::from_keys(keys.iter().map(String::as_str));
    assert_eq!(a.keys(), b.keys());
    let c = Album::from_keys(["mystery:X", "garbage", "mastery:OK"]);
    assert_eq!(c.keys(), vec!["mastery:OK".to_string()]);
}

#[test]
fn nothing_depends_on_time_or_volume() {
    // Одне й те саме поведінкове «досьє» дає той самий результат, скільки б разів і коли його не подавали.
    let mut a = Album::new();
    let mut b = Album::new();
    for _ in 0..1_000 {
        a.award_for_task("S", facts(true, 2, 1, false));
    }
    b.award_for_task("S", facts(true, 2, 1, false));
    assert_eq!(a.keys(), b.keys());
}

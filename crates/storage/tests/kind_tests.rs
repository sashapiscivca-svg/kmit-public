//! Тип профілю (учень чи вчитель): видно в метаданих слота до введення PIN, зберігається в стані, старі профілі дитячі.
use storage::crypto::KdfParams;
use storage::profiles::ProfileStore;
use storage::state::{AppState, ProfileKind, ReviewNote, WorkRecord};

mod common;

use common::TestDir;

const FAST: KdfParams = KdfParams::fast_for_tests();

#[test]
fn a_teacher_slot_is_listed_as_teacher_and_a_child_slot_as_child() {
    let d = TestDir::new();
    let store = ProfileStore::new(d.path());
    let (c, _, _) = store.create("4821", 3, FAST, None).unwrap();
    let slot = store.free_slot().unwrap();
    let _ = store
        .create_in_slot_kind(slot, "1357", 5, ProfileKind::Teacher, FAST, None)
        .unwrap();
    let list = store.list().unwrap();
    assert_eq!(
        list.iter().find(|s| s.slot == c).unwrap().kind,
        ProfileKind::Child
    );
    assert_eq!(
        list.iter().find(|s| s.slot == slot).unwrap().kind,
        ProfileKind::Teacher
    );
    // Зміна вигляду не стирає тип.
    store.set_avatar(slot, 9).unwrap();
    assert_eq!(
        store
            .list()
            .unwrap()
            .iter()
            .find(|s| s.slot == slot)
            .unwrap()
            .kind,
        ProfileKind::Teacher
    );
}

#[test]
fn old_state_without_a_kind_row_is_a_child_and_teacher_data_round_trips() {
    let mut s = AppState::new("Оля", 3);
    assert_eq!(s.kind, ProfileKind::Child);
    let mut rows = s.to_rows();
    rows.retain(|(k, _)| k != "kind" && k != "teacher_works" && k != "teacher_notes");
    assert_eq!(AppState::from_rows(&rows).unwrap().kind, ProfileKind::Child);

    s.kind = ProfileKind::Teacher;
    s.teacher.works.push(WorkRecord {
        day: 12,
        grade: 5,
        kind: "test".into(),
        skills: vec!["D-ADD".into(), "UA-NOUN".into()],
        variants: 3,
        seed: 77,
    });
    s.teacher.notes.push(ReviewNote {
        skill: "UA-NOUN".into(),
        field: "prompt".into(),
        index: 4,
        text: "Рядок | з вертикальною рискою\nі переносом".into(),
        status: "fix".into(),
        correction: "Виправлено 100%".into(),
        comment: "див. підручник".into(),
    });
    s.teacher.bookmarks = vec!["UA-NOUN".into(), "D-ADD".into()];
    s.teacher.works_made = 1;
    s.teacher.celebrated.insert("works:1".into());
    let back = AppState::from_rows(&s.to_rows()).unwrap();
    assert_eq!(back.kind, ProfileKind::Teacher);
    assert_eq!(back.teacher, s.teacher);
}

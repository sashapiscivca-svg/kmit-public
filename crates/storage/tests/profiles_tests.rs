mod common;

use common::{contains, TestDir};
use storage::crypto::KdfParams;
use storage::profiles::{ProfileStore, MAX_PROFILES};
use storage::state::AppState;
use storage::StorageError;

const FAST: KdfParams = KdfParams::fast_for_tests();

#[test]
fn a_fresh_store_has_no_profiles() {
    let d = TestDir::new();
    let store = ProfileStore::new(d.path());
    assert!(store.list().unwrap().is_empty());
}

#[test]
fn profiles_get_distinct_slots_and_only_the_avatar_is_visible_without_a_pin() {
    let d = TestDir::new();
    let store = ProfileStore::new(d.path());
    let (s1, mut v1, c1) = store.create("4821", 3, FAST, None).unwrap();
    v1.save_state(&AppState::new("Мирослава", 3)).unwrap();
    drop(v1);
    let (s2, v2, c2) = store.create("1357", 9, FAST, None).unwrap();
    drop(v2);
    assert_ne!(s1, s2);
    assert_ne!(c1, c2);
    let list = store.list().unwrap();
    assert_eq!(
        list.iter().map(|p| (p.slot, p.avatar)).collect::<Vec<_>>(),
        vec![(s1, 3), (s2, 9)]
    );
    // Нікнейм зашифрований, у відкритих файлах слотів його немає.
    for e in walk(d.path()) {
        assert!(
            !contains(&std::fs::read(&e).unwrap(), "Мирослава".as_bytes()),
            "{e:?}"
        );
    }
    // Кожен профіль відкривається лише своїм PIN.
    assert!(store.open(s1, "4821", None).is_ok());
    assert_eq!(
        store.open(s1, "1357", None).err(),
        Some(StorageError::WrongPin)
    );
    assert!(store.open(s2, "1357", None).is_ok());
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}

#[test]
fn the_number_of_profiles_is_limited() {
    let d = TestDir::new();
    let store = ProfileStore::new(d.path());
    for i in 0..MAX_PROFILES {
        store
            .create("4821", u8::try_from(i).unwrap(), FAST, None)
            .unwrap();
    }
    assert_eq!(
        store.create("4821", 0, FAST, None).err(),
        Some(StorageError::TooManyProfiles)
    );
    assert_eq!(store.list().unwrap().len(), MAX_PROFILES);
}

#[test]
fn erasing_a_profile_frees_its_slot_and_leaves_the_others() {
    let d = TestDir::new();
    let store = ProfileStore::new(d.path());
    let (s1, v1, _) = store.create("4821", 1, FAST, None).unwrap();
    drop(v1);
    let (s2, v2, _) = store.create("1357", 2, FAST, None).unwrap();
    drop(v2);
    store.erase(s1, None).unwrap();
    assert_eq!(
        store
            .list()
            .unwrap()
            .iter()
            .map(|p| p.slot)
            .collect::<Vec<_>>(),
        vec![s2]
    );
    assert_eq!(
        store.open(s1, "4821", None).err(),
        Some(StorageError::NoSuchProfile)
    );
    assert!(store.open(s2, "1357", None).is_ok());
    let (s3, v3, _) = store.create("2468", 5, FAST, None).unwrap();
    drop(v3);
    assert_eq!(s3, s1, "звільнений слот використовується знову");
    store.erase(s1, None).unwrap();
    store.erase(s1, None).unwrap(); // повторно: без помилки
}

#[test]
fn recovery_works_per_profile() {
    let d = TestDir::new();
    let store = ProfileStore::new(d.path());
    let (slot, v, code) = store.create("4821", 1, FAST, None).unwrap();
    drop(v);
    let v = store.recover(slot, &code, "9999", None).unwrap();
    drop(v);
    assert!(store.open(slot, "9999", None).is_ok());
    assert_eq!(
        store.open(slot, "4821", None).err(),
        Some(StorageError::WrongPin)
    );
}

#[test]
fn invalid_slots_and_broken_meta_are_handled_gracefully() {
    let d = TestDir::new();
    let store = ProfileStore::new(d.path());
    assert_eq!(
        store.open(0, "4821", None).err(),
        Some(StorageError::NoSuchProfile)
    );
    assert_eq!(
        store.open(200, "4821", None).err(),
        Some(StorageError::NoSuchProfile)
    );
    assert!(
        store.erase(77, None).is_ok(),
        "неіснуючий слот стирається мовчки"
    );
    let (slot, v, _) = store.create("4821", 4, FAST, None).unwrap();
    drop(v);
    // Зіпсований відкритий файл-мета не ламає список і не дає аватар понад межі.
    let meta = d.path().join(format!("p{slot}")).join("slot.meta");
    std::fs::write(&meta, "avatar=250\n").unwrap();
    assert!(store.list().unwrap()[0].avatar <= storage::state::MAX_AVATAR);
    std::fs::write(&meta, "сміття").unwrap();
    assert_eq!(store.list().unwrap().len(), 1);
}

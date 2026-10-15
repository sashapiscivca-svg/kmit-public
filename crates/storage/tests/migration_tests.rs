use storage::state::{migrate_rows_with, AppState, CURRENT_SCHEMA};
use storage::StorageError;

type Rows = Vec<(String, String)>;

fn rows() -> Rows {
    AppState::new("Оля", 2).to_rows()
}

#[test]
fn saved_rows_carry_the_current_schema_version() {
    let r = rows();
    let v = r.iter().find(|(k, _)| k == "schema").expect("рядок schema");
    assert_eq!(v.1, CURRENT_SCHEMA.to_string());
}

#[test]
fn rows_saved_before_versioning_still_load() {
    let old: Rows = rows().into_iter().filter(|(k, _)| k != "schema").collect();
    assert!(
        AppState::from_rows(&old).is_ok(),
        "старий профіль без версії відкривається"
    );
}

#[test]
fn a_profile_from_a_newer_version_is_refused_not_corrupted() {
    let mut r = rows();
    for (k, v) in r.iter_mut() {
        if k == "schema" {
            *v = (CURRENT_SCHEMA + 1).to_string();
        }
    }
    assert!(matches!(
        AppState::from_rows(&r),
        Err(StorageError::NewerFormat)
    ));
}

#[test]
fn migrations_run_in_order_from_the_stored_version_and_stamp_the_result() {
    fn rename(rows: &mut Rows) {
        for (k, _) in rows.iter_mut() {
            if k == "old_name" {
                *k = "new_name".to_string();
            }
        }
    }
    fn append(rows: &mut Rows) {
        rows.push(("added".into(), "1".into()));
    }
    let steps: &[fn(&mut Rows)] = &[rename, append];
    let mut r: Rows = vec![
        ("schema".into(), "0".into()),
        ("old_name".into(), "x".into()),
    ];
    migrate_rows_with(&mut r, steps).unwrap();
    assert!(r.iter().any(|(k, v)| k == "new_name" && v == "x"));
    assert!(r.iter().any(|(k, _)| k == "added"));
    assert_eq!(r.iter().find(|(k, _)| k == "schema").unwrap().1, "2");
    // З версії 1 виконується лише останній крок.
    let mut r: Rows = vec![
        ("schema".into(), "1".into()),
        ("old_name".into(), "x".into()),
    ];
    migrate_rows_with(&mut r, steps).unwrap();
    assert!(
        r.iter().any(|(k, _)| k == "old_name"),
        "перший крок не повторюється"
    );
    assert!(r.iter().any(|(k, _)| k == "added"));
}

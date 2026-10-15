#![allow(dead_code)]
//! Допоміжні речі: тимчасова тека, виклики API і «сканер витоку» для кожної відповіді.

use app_core::{ApiError, App, Ctx};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use storage::crypto::KdfParams;
use tutor_core::numtok::numeric_tokens;

pub struct TestDir(pub PathBuf);

impl TestDir {
    pub fn new() -> Self {
        let mut b = [0u8; 8];
        getrandom_fill(&mut b);
        let name: String = b.iter().map(|x| format!("{x:02x}")).collect();
        let p = std::env::temp_dir().join(format!("kmit-app-test-{name}"));
        std::fs::create_dir_all(&p).unwrap();
        TestDir(p)
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn getrandom_fill(b: &mut [u8]) {
    getrandom::getrandom(b).unwrap();
}

pub fn app(dir: &TestDir) -> App {
    App::new(dir.path(), KdfParams::fast_for_tests(), None)
}

pub fn ctx(wall: u64, day: u32) -> Ctx {
    Ctx {
        wall,
        day,
        minute: 12 * 60,
    }
}

fn strings_of(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| strings_of(x, out)),
        Value::Object(o) => o.values().for_each(|x| strings_of(x, out)),
        _ => {}
    }
}

/// Виклик API. Кожна успішна відповідь перевіряється на витік: у текстах, які побачить дитина, не може бути
/// захищених значень задачі, що була активна до виклику чи після нього.
pub fn call(app: &mut App, name: &str, args: Value, c: &Ctx) -> Result<Value, ApiError> {
    let mut protected = app.peek_protected_values();
    let r = app.call(name, &args, c)?;
    protected.extend(app.peek_protected_values());
    let mut texts = Vec::new();
    strings_of(&r, &mut texts);
    for t in &texts {
        let tokens = numeric_tokens(t);
        for p in &protected {
            assert!(
                !tokens.contains(p),
                "відповідь на «{name}» містить захищене значення {p}: «{t}»"
            );
        }
    }
    Ok(r)
}

pub fn ok(app: &mut App, name: &str, args: Value, c: &Ctx) -> Value {
    call(app, name, args, c).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

/// Вмикає обов'язкові ліміти часу (за замовчуванням вони вимкнені).
pub fn enforce_limits(app: &mut App, c: &Ctx) {
    ok(app, "settings_set", json!({"enforce_limits": true}), c);
}

pub fn create_and_open(app: &mut App, c: &Ctx) -> (u8, String) {
    let r = create_and_open_raw(app, c);
    ok(app, "settings_set", json!({"lesson_first": false}), c);
    r
}

/// Як `create_and_open`, але з налаштуваннями за замовчуванням (нова тема починається з пропозиції пояснення).
pub fn create_and_open_raw(app: &mut App, c: &Ctx) -> (u8, String) {
    let r = ok(
        app,
        "create_profile",
        json!({"nickname": "Мирослава", "avatar": 2, "pin": "4821"}),
        c,
    );
    (
        u8::try_from(r["slot"].as_u64().unwrap()).unwrap(),
        r["recovery_code"].as_str().unwrap().to_string(),
    )
}

/// Створює вчительський профіль і лишає його відкритим.
pub fn create_teacher(a: &mut app_core::App, c: &app_core::Ctx) {
    ok(
        a,
        "create_profile",
        serde_json::json!({"nickname": "Оксана", "pin": "1357", "kind": "teacher"}),
        c,
    );
}

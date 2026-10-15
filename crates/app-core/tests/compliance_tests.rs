#![cfg(not(feature = "reviewer"))]
//! Відповідність вимогам Google Play для дитячого застосунку (Families Policy, Data safety): маніфест, залежності, CSP.
//! Див. ARCHITECTURE.md (розділ «Compliance tests»). Тести падають, якщо хтось додасть дозвіл на мережу, рекламний
//! ідентифікатор, геолокацію, камеру тощо, не оновивши декларації й політику.

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

const MANIFEST: &str = "app/src-tauri/gen/android/app/src/main/AndroidManifest.xml";

/// Склеює теги `<uses-permission ...>` у один рядок, щоб дозвіл, розбитий на кілька рядків, не обходив перевірку.
fn permission_tags(manifest: &str) -> Vec<String> {
    let flat = manifest.split_whitespace().collect::<Vec<_>>().join(" ");
    flat.split('<')
        .filter(|t| t.starts_with("uses-permission"))
        .map(|t| t.split('>').next().unwrap_or("").to_string())
        .collect()
}

fn forbidden_in(manifest: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in permission_tags(manifest) {
        let line = line.as_str();
        if line.contains("android.permission.INTERNET") {
            // Дозволено лише як вилучення дозволу, який могла б додати залежність.
            if !line.contains(r#"tools:node="remove""#) {
                found.push(format!("INTERNET не має бути дозволений: {line}"));
            }
            continue;
        }
        for bad in [
            "AD_ID",
            "ACCESS_FINE_LOCATION",
            "ACCESS_COARSE_LOCATION",
            "ACCESS_BACKGROUND_LOCATION",
            "READ_CONTACTS",
            "READ_PHONE_STATE",
            "READ_PHONE_NUMBERS",
            "READ_EXTERNAL_STORAGE",
            "WRITE_EXTERNAL_STORAGE",
            "READ_MEDIA",
            "RECORD_AUDIO",
            "CAMERA",
            "BLUETOOTH",
            "GET_ACCOUNTS",
            "READ_SMS",
        ] {
            if line.contains(bad) && !line.contains(r#"tools:node="remove""#) {
                found.push(format!("заборонений дозвіл {bad}: {line}"));
            }
        }
    }
    found
}

#[test]
fn manifest_has_no_forbidden_permissions() {
    let found = forbidden_in(&read(MANIFEST));
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_check_really_catches_a_forbidden_permission() {
    let bad = r#"<uses-permission android:name="android.permission.CAMERA" />
<uses-permission android:name="android.permission.INTERNET" />
<uses-permission android:name="com.google.android.gms.permission.AD_ID" />"#;
    assert_eq!(forbidden_in(bad).len(), 3);
    let ok =
        r#"<uses-permission android:name="android.permission.INTERNET" tools:node="remove" />"#;
    assert!(forbidden_in(ok).is_empty());
}

#[test]
fn no_ads_or_analytics_dependencies() {
    let lock = read("Cargo.lock").to_lowercase();
    for bad in [
        "firebase",
        "sentry",
        "admob",
        "google-analytics",
        "appsflyer",
        "adjust",
        "mixpanel",
        "segment",
        "amplitude",
        "crashlytics",
        "facebook",
    ] {
        assert!(
            !lock.contains(&format!("name = \"{bad}")),
            "підозріла залежність {bad}: оновіть Data safety й політику"
        );
    }
}

#[test]
fn gradle_has_no_play_services_ads_or_analytics() {
    let dir = root().join("app/src-tauri/gen/android");
    let mut text = String::new();
    for f in [
        "app/build.gradle.kts",
        "build.gradle.kts",
        "settings.gradle",
    ] {
        if let Ok(t) = std::fs::read_to_string(dir.join(f)) {
            text.push_str(&t.to_lowercase());
        }
    }
    for bad in [
        "play-services-ads",
        "firebase",
        "crashlytics",
        "play-services-analytics",
        "appsflyer",
        "facebook",
    ] {
        assert!(!text.contains(bad), "Android-залежність {bad} заборонена");
    }
}

#[test]
fn csp_allows_only_local_sources() {
    let c = read("app/src-tauri/tauri.conf.json");
    let csp = c
        .lines()
        .find(|l| l.contains("\"csp\""))
        .expect("CSP має бути заданий");
    let value = csp
        .split_once(':')
        .map(|(_, v)| v)
        .unwrap_or("")
        .trim()
        .trim_matches(|ch| ch == '"' || ch == ',');
    // Кожна директива може мати лише ці значення: жодних віддалених вузлів, `*` чи `unsafe-*`.
    let allowed = ["'self'", "'none'", "data:", "ipc:", "http://ipc.localhost"];
    let mut seen = Vec::new();
    for directive in value.split(';').map(str::trim).filter(|d| !d.is_empty()) {
        let mut parts = directive.split_whitespace();
        let name = parts.next().unwrap();
        seen.push(name.to_string());
        for v in parts {
            assert!(
                allowed.contains(&v),
                "CSP: недозволене значення «{v}» у {name}"
            );
        }
    }
    for need in ["default-src", "script-src", "object-src", "connect-src"] {
        assert!(seen.iter().any(|d| d == need), "у CSP немає {need}");
    }
}

#[test]
fn ui_has_no_external_urls_in_scripts_or_styles() {
    let dir = root().join("app/ui");
    let mut checked = 0;
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let path = e.path();
        let ext = path.extension().and_then(|x| x.to_str()).unwrap_or("");
        if !["js", "html", "css"].contains(&ext) {
            continue;
        }
        checked += 1;
        let t = std::fs::read_to_string(&path).unwrap();
        for line in t.lines() {
            let l = line.trim_start();
            if l.starts_with("//") || l.starts_with("/*") || l.starts_with('*') {
                continue;
            }
            for bad in [
                "src=\"http",
                "href=\"http",
                "url(http",
                "url(\"http",
                "url('http",
                "fetch('http",
                "fetch(\"http",
                "import('http",
                "from 'http",
                "XMLHttpRequest",
                "WebSocket(",
                "sendBeacon",
            ] {
                assert!(
                    !l.contains(bad),
                    "{}: зовнішній ресурс чи мережа ({bad}): {line}",
                    path.display()
                );
            }
        }
    }
    assert!(checked >= 15, "перевірено лише {checked} файлів UI");
}

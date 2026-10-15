//! `content-tools`: перевірка пакетів, звіт для методиста, застосування виправлень, підпис.
//!   content-tools validate <тека пакетів>
//!   content-tools review <тека пакетів> <тека виводу>          (CSV і HTML по кожному пакету)
//!   content-tools apply <тека пакетів> <review.csv>            (застосовує виправлення, піднімає версії)
//!   content-tools approve <тека> <id пакета> <reviewed|approved> <хто> <дата>
//!   content-tools keygen <файл приватного ключа>               (друкує відкритий ключ)
//!   content-tools sign <файл приватного ключа> <пакет.json>    (створює <пакет>.sig)
//!   content-tools verify <пакет.json> <пакет.sig> <відкритий ключ hex>

use content_tools::{review, sign};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn packs_in(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|d| {
            d.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "json"))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let code = match a.first().map(String::as_str) {
        Some("validate") if a.len() == 2 => validate(Path::new(&a[1])),
        Some("review") if a.len() == 3 => review_cmd(Path::new(&a[1]), Path::new(&a[2])),
        Some("apply") if a.len() == 3 => apply_cmd(Path::new(&a[1]), Path::new(&a[2])),
        Some("approve") if a.len() == 6 => {
            approve_cmd(Path::new(&a[1]), &a[2], &a[3], &a[4], &a[5])
        }
        Some("keygen") if a.len() == 2 => keygen(Path::new(&a[1])),
        Some("sign") if a.len() == 3 => sign_cmd(Path::new(&a[1]), Path::new(&a[2])),
        Some("verify") if a.len() == 4 => verify(&a[1], &a[2], &a[3]),
        _ => {
            eprintln!(
                "{}",
                include_str!("main.rs")
                    .lines()
                    .take(9)
                    .map(|l| l.trim_start_matches("//! ").trim_start_matches("//!"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            2
        }
    };
    std::process::exit(code);
}

fn validate(dir: &Path) -> i32 {
    let mut bad = 0;
    for p in packs_in(dir) {
        let bytes = fs::read(&p).unwrap_or_default();
        match subjects::pack::parse(&bytes) {
            Ok(pack) => {
                let built = subjects::pack::build(&pack);
                let mut issues: Vec<String> = built.banks.iter().flat_map(|b| b.lint()).collect();
                issues.extend(built.typed.iter().flat_map(|b| b.lint()));
                println!(
                    "{}: тем {}, проблем {}",
                    p.display(),
                    pack.skills.len(),
                    issues.len()
                );
                for i in issues.iter().take(20) {
                    println!("  {i}");
                }
                bad += issues.len();
            }
            Err(e) => {
                println!("{}: {e}", p.display());
                bad += 1;
            }
        }
    }
    i32::from(bad > 0)
}

fn review_cmd(dir: &Path, out: &Path) -> i32 {
    fs::create_dir_all(out).expect("тека виводу");
    for p in packs_in(dir) {
        let v: Value = serde_json::from_slice(&fs::read(&p).unwrap_or_default()).expect("JSON");
        let id = v["id"].as_str().unwrap_or("pack").to_string();
        let rows = review::rows_for(&v);
        fs::write(
            out.join(format!("{id}-review.csv")),
            review::to_csv(rows.clone()),
        )
        .expect("csv");
        fs::write(out.join(format!("{id}-review.html")), review::to_html(&v)).expect("html");
        println!("{id}: {} рядків", rows.len());
    }
    0
}

fn apply_cmd(dir: &Path, csv: &Path) -> i32 {
    let files = packs_in(dir);
    let mut packs: Vec<Value> = files
        .iter()
        .map(|p| serde_json::from_slice(&fs::read(p).unwrap_or_default()).expect("JSON"))
        .collect();
    let before = packs.clone();
    let r = review::apply(&mut packs, &fs::read_to_string(csv).expect("CSV"));
    for (f, (p, b)) in files.iter().zip(packs.iter().zip(before.iter())) {
        if p != b {
            fs::write(f, serde_json::to_string_pretty(p).expect("JSON") + "\n").expect("запис");
        }
    }
    println!(
        "виправлено {}, видалено {}, пропущено {}",
        r.fixed,
        r.removed,
        r.skipped.len()
    );
    for s in &r.skipped {
        println!("  {s}");
    }
    0
}

/// Методист схвалив пакет: записує статус, ім'я чи ініціали й дату, піднімає версію (щоб оновлення замінило попереднє).
fn approve_cmd(dir: &Path, id: &str, status: &str, by: &str, date: &str) -> i32 {
    if !["reviewed", "approved"].contains(&status) {
        eprintln!("статус має бути reviewed або approved");
        return 2;
    }
    for f in packs_in(dir) {
        let mut v: Value = serde_json::from_slice(&fs::read(&f).unwrap_or_default()).expect("JSON");
        if v.get("id").and_then(Value::as_str) != Some(id) {
            continue;
        }
        let version = v.get("version").and_then(Value::as_u64).unwrap_or(1) + 1;
        v["version"] = serde_json::json!(version);
        v["review"] = serde_json::json!({"status": status, "by": by, "date": date, "notes": ""});
        fs::write(&f, serde_json::to_string_pretty(&v).expect("JSON") + "\n").expect("запис");
        println!("пакет {id}: {status}, версія {version}");
        return 0;
    }
    eprintln!("пакет {id} не знайдено");
    1
}

fn keygen(path: &Path) -> i32 {
    if path.exists() {
        eprintln!("файл уже є: не перезаписую");
        return 1;
    }
    let (private, public) = sign::keygen();
    fs::write(path, private).expect("запис ключа");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    println!("відкритий ключ (hex): {public}");
    0
}

fn sign_cmd(key: &Path, pack: &Path) -> i32 {
    let seed = fs::read_to_string(key).expect("ключ");
    let bytes = fs::read(pack).expect("пакет");
    if subjects::pack::parse(&bytes).is_err() {
        eprintln!("пакет не проходить перевірку: підписувати не можна");
        return 1;
    }
    let sig = sign::sign(&seed, &bytes).expect("хибний приватний ключ");
    let dest = pack.with_extension("sig");
    fs::write(&dest, sig).expect("запис підпису");
    println!("підпис: {}", dest.display());
    0
}

fn verify(pack: &str, sig: &str, public_hex: &str) -> i32 {
    let (Some(key), Ok(b), Ok(s)) = (
        sign::unhex(public_hex).and_then(|k| <[u8; 32]>::try_from(k).ok()),
        fs::read(pack),
        fs::read(sig),
    ) else {
        eprintln!("не вдалося прочитати вхідні дані");
        return 1;
    };
    match subjects::pack::verify_signature(&b, &s, &key) {
        Ok(()) => {
            println!("підпис правильний");
            0
        }
        Err(e) => {
            println!("підпис НЕ підходить: {e:?}");
            1
        }
    }
}

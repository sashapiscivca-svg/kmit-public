//! Дитячі пояснення типових помилок: покриття, відсутність чисел, довжина, нейтральність роду, різноманітність.

use generators_math::{all_generators, bug_message};
use std::collections::{BTreeMap, BTreeSet};

const VARIANTS: u64 = 3_000;

fn all_pairs() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    let mut m: BTreeMap<&'static str, BTreeSet<&'static str>> = BTreeMap::new();
    for g in all_generators() {
        for v in 0..VARIANTS {
            let t = g.generate(v);
            for b in &t.bugs {
                m.entry(g.skill_id()).or_default().insert(b.id);
            }
        }
    }
    m
}

#[test]
fn every_bug_has_a_message() {
    let mut missing = Vec::new();
    for (skill, bugs) in all_pairs() {
        for b in bugs {
            if bug_message(skill, b).is_none() {
                missing.push(format!("{skill}/{b}"));
            }
        }
    }
    assert!(missing.is_empty(), "немає повідомлень: {missing:?}");
}

#[test]
fn messages_are_clean() {
    let words = [
        "один",
        "одна",
        "одне",
        "два",
        "дві",
        "три",
        "чотири",
        "п'ять",
        "шість",
        "сім",
        "вісім",
        "дев'ять",
        "десять",
        "сто",
        "одного",
        "двох",
        "трьох",
        "двічі",
    ];
    for (skill, bugs) in all_pairs() {
        for b in bugs {
            let Some(m) = bug_message(skill, b) else {
                continue;
            };
            let id = format!("{skill}/{b}");
            assert!(!m.chars().any(|c| c.is_numeric()), "{id}: цифри");
            let n = m.chars().count();
            assert!((20..=220).contains(&n), "{id}: довжина {n}");
            for g in ["(ла)", "(-ла)", "(а)", "(ла", "(ий)"] {
                assert!(!m.contains(g), "{id}: рід {g}");
            }
            let low = m.to_lowercase();
            for w in words {
                assert!(
                    !low.split(|c: char| !c.is_alphabetic() && c != '\'')
                        .any(|x| x == w),
                    "{id}: число-слово {w}"
                );
            }
            for bad in [
                "зробив",
                "зробила",
                "помилився",
                "помилилась",
                "забув",
                "забула",
            ] {
                assert!(!low.contains(bad), "{id}: рід {bad}");
            }
        }
    }
}

#[test]
fn messages_differ_within_skill() {
    for (skill, bugs) in all_pairs() {
        let texts: BTreeSet<&str> = bugs.iter().filter_map(|b| bug_message(skill, b)).collect();
        assert_eq!(texts.len(), bugs.len(), "{skill}: однакові повідомлення");
    }
}

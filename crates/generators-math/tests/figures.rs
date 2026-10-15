//! Малюнки до задач (P6). Захист І1: малюнок показує лише дані умови, а не відповідь чи проміжні кроки.

use generators_math::all_generators;
use std::collections::BTreeMap;
use tutor_core::numtok::numeric_tokens;

const VARIANTS: u64 = 5_000;

/// Навички, для яких малюнки підключено (у частини варіантів малюнок може бракувати: це безпечніше за витік).
const WITH_FIGURES: [&str; 10] = [
    "F-ADD-SAME",
    "F-ADD-DIFF",
    "F-SUB-SAME",
    "F-SUB-DIFF",
    "F-MIXED",
    "F-OF",
    "D-ADD",
    "G-PERIM",
    "G-AREA",
    "A-ANGLE",
];

#[test]
fn figures_never_show_answer_or_steps() {
    let mut counts: BTreeMap<&'static str, (u64, u64)> = BTreeMap::new();
    for g in all_generators() {
        for v in 0..VARIANTS {
            let task = g.generate(v);
            let figs = g.figures(&task);
            let e = counts.entry(g.skill_id()).or_default();
            e.1 += 1;
            if figs.is_empty() {
                continue;
            }
            e.0 += 1;
            let prot = task.protected_values();
            for f in &figs {
                let id = format!("{} варіант {v}: {f:?}", g.skill_id());
                assert!(f.is_sane(), "{id}: невалідний малюнок");
                for val in f.values() {
                    assert!(
                        !prot.contains(&val),
                        "{id}: значення збігається із захищеним"
                    );
                }
                for t in f.texts() {
                    for tok in numeric_tokens(&t) {
                        assert!(!prot.contains(&tok), "{id}: підпис «{t}» містить захищене");
                    }
                }
            }
        }
    }
    for (skill, (with, all)) in &counts {
        eprintln!("{skill}: малюнки у {with} з {all}");
    }
    for skill in WITH_FIGURES {
        let (with, _) = counts[skill];
        // Сітка вміщає лише до трьох цілих, тому десяткові доданки більшого розміру лишаються без малюнка.
        let min = if skill == "D-ADD" { 1 } else { VARIANTS / 4 };
        assert!(with >= min, "{skill}: малюнків замало ({with})");
    }
}

#[test]
fn figures_are_deterministic_and_default_empty() {
    for g in all_generators() {
        let t = g.generate(7);
        assert_eq!(g.figures(&t), g.figures(&t));
    }
    // Навички без малюнків повертають порожньо.
    for g in all_generators() {
        if matches!(g.skill_id(), "N-ADD-MULTI" | "N-ORDER" | "AVG") {
            assert!(g.figures(&g.generate(1)).is_empty());
        }
    }
}

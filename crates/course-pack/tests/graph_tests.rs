use course_pack::{grade5_m0_graph, GraphError, SkillGraph, SkillNode};
use std::collections::BTreeSet;

fn node(id: &'static str, prerequisites: Vec<&'static str>) -> SkillNode {
    SkillNode {
        id,
        title: id,
        prerequisites,
    }
}

#[test]
fn builtin_graph_is_valid_and_ordered() {
    let g = grade5_m0_graph();
    let order = g.topological_order().unwrap();
    assert_eq!(order.len(), 35);
    let pos = |id: &str| order.iter().position(|x| *x == id).unwrap();
    assert!(pos("N-ADD-MULTI") < pos("F-ADD-SAME"));
    assert!(pos("F-ADD-SAME") < pos("F-ADD-DIFF"));
    assert!(pos("N-ADD-MULTI") < pos("D-ADD"));
    // Нові навички: кожна йде після всіх своїх передумов.
    for (skill, prerequisites) in [
        ("N-SUB-MULTI", vec!["N-ADD-MULTI"]),
        ("N-MUL-MULTI", vec!["N-ADD-MULTI"]),
        ("N-DIV-MULTI", vec!["N-MUL-MULTI"]),
        ("N-ORDER", vec!["N-MUL-MULTI"]),
        ("D-SUB", vec!["D-ADD", "N-SUB-MULTI"]),
        ("F-SUB-SAME", vec!["F-ADD-SAME", "N-SUB-MULTI"]),
        ("F-SUB-DIFF", vec!["F-ADD-DIFF", "F-SUB-SAME"]),
    ] {
        let node = g.get(skill).unwrap();
        assert_eq!(node.prerequisites, prerequisites, "{skill}");
        assert!(prerequisites.iter().all(|p| pos(p) < pos(skill)), "{skill}");
    }
}

/// Розширення курсу (рішення A20): передумови нових навичок, відсутність осиротілих вузлів і неланцюговий граф.
#[test]
fn extended_course_graph_has_sensible_prerequisites() {
    let g = grade5_m0_graph();
    let expected: [(&str, &[&str]); 24] = [
        ("N-ROUND", &["N-ADD-MULTI"]),
        ("N-DIVREM", &["N-DIV-MULTI"]),
        ("N-DIVTEST", &["N-DIVREM"]),
        ("N-GCD", &["N-DIVTEST"]),
        ("N-LCM", &["N-DIVTEST", "N-MUL-MULTI"]),
        ("N-POWER", &["N-ORDER"]),
        ("EQ-ADD-SUB", &["N-SUB-MULTI"]),
        ("EQ-MUL", &["EQ-ADD-SUB", "N-DIV-MULTI"]),
        ("WORD-1", &["N-SUB-MULTI", "N-DIV-MULTI"]),
        ("WORD-2", &["WORD-1", "N-ORDER"]),
        ("MOTION", &["WORD-2", "EQ-MUL"]),
        ("U-LEN", &["N-MUL-MULTI", "N-DIV-MULTI"]),
        ("U-MASS", &["U-LEN"]),
        ("U-TIME", &["U-LEN"]),
        ("G-PERIM", &["N-MUL-MULTI", "EQ-ADD-SUB"]),
        ("G-AREA", &["G-PERIM", "EQ-MUL"]),
        ("G-VOL", &["G-AREA", "N-POWER"]),
        ("A-ANGLE", &["N-SUB-MULTI"]),
        ("F-MIXED", &["F-ADD-SAME", "N-DIVREM"]),
        ("F-REDUCE", &["N-GCD", "F-ADD-SAME"]),
        ("F-OF", &["F-ADD-SAME", "N-DIV-MULTI"]),
        ("P-PCT", &["F-OF"]),
        ("D-MUL10", &["D-ADD", "N-MUL-MULTI"]),
        ("AVG", &["N-ADD-MULTI", "N-DIV-MULTI"]),
    ];
    let order = g.topological_order().unwrap();
    let pos = |id: &str| order.iter().position(|x| *x == id).unwrap();
    for (skill, prerequisites) in expected {
        let node = g
            .get(skill)
            .unwrap_or_else(|| panic!("немає вузла {skill}"));
        assert_eq!(node.prerequisites, prerequisites, "{skill}");
        assert!(prerequisites.iter().all(|p| pos(p) < pos(skill)), "{skill}");
        assert!(!node.title.is_empty());
    }
    // Єдиний корінь курсу: лише N-ADD-MULTI не має передумов; усе решта досяжне з нього.
    let roots: Vec<_> = g
        .ids()
        .into_iter()
        .filter(|id| g.get(id).unwrap().prerequisites.is_empty())
        .collect();
    assert_eq!(roots, vec!["N-ADD-MULTI"]);
    // Граф не є лінійним ланцюгом: після опанування кореня одразу доступно кілька тем.
    let mastered: BTreeSet<&'static str> = ["N-ADD-MULTI"].into_iter().collect();
    assert!(g.available(&mastered).len() >= 4);
    // Усі теми можна пройти: жадібно опановуємо доступні, доки є що.
    let mut done: BTreeSet<&'static str> = BTreeSet::new();
    loop {
        let next = g.available(&done);
        if next.is_empty() {
            break;
        }
        done.extend(next);
    }
    assert_eq!(done.len(), g.ids().len());
}

#[test]
fn duplicate_skill_is_rejected() {
    let r = SkillGraph::new(vec![node("A", vec![]), node("A", vec![])]);
    assert_eq!(r.unwrap_err(), GraphError::DuplicateSkill("A"));
}

#[test]
fn unknown_prerequisite_is_rejected() {
    let r = SkillGraph::new(vec![node("A", vec!["B"])]);
    assert_eq!(
        r.unwrap_err(),
        GraphError::UnknownPrerequisite {
            skill: "A",
            missing: "B"
        }
    );
}

#[test]
fn cycle_is_rejected() {
    let r = SkillGraph::new(vec![node("A", vec!["B"]), node("B", vec!["A"])]);
    assert!(matches!(r.unwrap_err(), GraphError::Cycle(_)));
    let self_loop = SkillGraph::new(vec![node("A", vec!["A"])]);
    assert!(matches!(self_loop.unwrap_err(), GraphError::Cycle(_)));
}

#[test]
fn available_skills_follow_mastery() {
    let g = grade5_m0_graph();
    let mut mastered: BTreeSet<&'static str> = BTreeSet::new();
    assert_eq!(g.available(&mastered), vec!["N-ADD-MULTI"]);
    mastered.insert("N-ADD-MULTI");
    assert_eq!(
        g.available(&mastered),
        vec![
            "D-ADD",
            "F-ADD-SAME",
            "N-MUL-MULTI",
            "N-ROUND",
            "N-SUB-MULTI"
        ]
    );
    mastered.insert("F-ADD-SAME");
    assert_eq!(
        g.available(&mastered),
        vec![
            "D-ADD",
            "F-ADD-DIFF",
            "N-MUL-MULTI",
            "N-ROUND",
            "N-SUB-MULTI"
        ]
    );
    mastered.insert("N-SUB-MULTI");
    let available = g.available(&mastered);
    assert!(available.contains(&"F-SUB-SAME"));
    assert!(available.contains(&"D-ADD") && !available.contains(&"D-SUB"));
}

#[test]
fn lookup_and_ids() {
    let g = grade5_m0_graph();
    assert!(g.get("D-ADD").is_some());
    assert!(g.get("nope").is_none());
    assert_eq!(g.ids().len(), 35);
}

//! Граф навичок курс-пакета: вузли, передумови, перевірка цілісності.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct SkillNode {
    pub id: &'static str,
    pub title: &'static str,
    pub prerequisites: Vec<&'static str>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GraphError {
    DuplicateSkill(&'static str),
    UnknownPrerequisite {
        skill: &'static str,
        missing: &'static str,
    },
    Cycle(&'static str),
}

#[derive(Clone, Debug)]
pub struct SkillGraph {
    nodes: BTreeMap<&'static str, SkillNode>,
}

impl SkillGraph {
    /// Будує граф і одразу перевіряє: без дублікатів, усі передумови існують, циклів немає.
    pub fn new(nodes: Vec<SkillNode>) -> Result<Self, GraphError> {
        let mut map = BTreeMap::new();
        for n in nodes {
            let id = n.id;
            if map.insert(id, n).is_some() {
                return Err(GraphError::DuplicateSkill(id));
            }
        }
        for n in map.values() {
            for p in &n.prerequisites {
                if !map.contains_key(p) {
                    return Err(GraphError::UnknownPrerequisite {
                        skill: n.id,
                        missing: p,
                    });
                }
            }
        }
        let g = SkillGraph { nodes: map };
        g.topological_order()?;
        Ok(g)
    }

    pub fn get(&self, id: &str) -> Option<&SkillNode> {
        self.nodes.get(id)
    }

    pub fn ids(&self) -> Vec<&'static str> {
        self.nodes.keys().copied().collect()
    }

    /// Порядок, у якому кожна навичка йде після всіх своїх передумов (детермінований).
    pub fn topological_order(&self) -> Result<Vec<&'static str>, GraphError> {
        let mut done: BTreeSet<&'static str> = BTreeSet::new();
        let mut order = Vec::new();
        let mut visiting: BTreeSet<&'static str> = BTreeSet::new();
        for id in self.nodes.keys().copied() {
            self.visit(id, &mut done, &mut visiting, &mut order)?;
        }
        Ok(order)
    }

    fn visit(
        &self,
        id: &'static str,
        done: &mut BTreeSet<&'static str>,
        visiting: &mut BTreeSet<&'static str>,
        order: &mut Vec<&'static str>,
    ) -> Result<(), GraphError> {
        if done.contains(id) {
            return Ok(());
        }
        if !visiting.insert(id) {
            return Err(GraphError::Cycle(id));
        }
        for p in &self.nodes[id].prerequisites {
            self.visit(p, done, visiting, order)?;
        }
        visiting.remove(id);
        done.insert(id);
        order.push(id);
        Ok(())
    }

    /// Навички, доступні дитині: усі передумови опановано, сама навичка ще ні.
    pub fn available(&self, mastered: &BTreeSet<&'static str>) -> Vec<&'static str> {
        self.nodes
            .values()
            .filter(|n| !mastered.contains(n.id))
            .filter(|n| n.prerequisites.iter().all(|p| mastered.contains(p)))
            .map(|n| n.id)
            .collect()
    }
}

/// Чернетка ядра «Математика 5 клас»: 35 навичок (11 початкових і 24 розширення за рішенням A20). Ребра графа й самі вправи авторські,
/// їх має переглянути методист (відкрите питання Q7).
pub fn grade5_graph() -> SkillGraph {
    let node = |id, title, prerequisites| SkillNode {
        id,
        title,
        prerequisites,
    };
    SkillGraph::new(vec![
        node("N-ADD-MULTI", "Додавання багатоцифрових чисел", vec![]),
        node("D-ADD", "Додавання десяткових дробів", vec!["N-ADD-MULTI"]),
        node(
            "F-ADD-SAME",
            "Додавання дробів з однаковими знаменниками",
            vec!["N-ADD-MULTI"],
        ),
        node(
            "F-ADD-DIFF",
            "Додавання дробів з різними знаменниками",
            vec!["F-ADD-SAME"],
        ),
        node(
            "N-SUB-MULTI",
            "Віднімання багатоцифрових чисел",
            vec!["N-ADD-MULTI"],
        ),
        node(
            "N-MUL-MULTI",
            "Множення багатоцифрових чисел",
            vec!["N-ADD-MULTI"],
        ),
        node(
            "N-DIV-MULTI",
            "Ділення багатоцифрових чисел",
            vec!["N-MUL-MULTI"],
        ),
        node("N-ORDER", "Порядок дій", vec!["N-MUL-MULTI"]),
        node(
            "D-SUB",
            "Віднімання десяткових дробів",
            vec!["D-ADD", "N-SUB-MULTI"],
        ),
        node(
            "F-SUB-SAME",
            "Віднімання дробів з однаковими знаменниками",
            vec!["F-ADD-SAME", "N-SUB-MULTI"],
        ),
        node(
            "F-SUB-DIFF",
            "Віднімання дробів з різними знаменниками",
            vec!["F-ADD-DIFF", "F-SUB-SAME"],
        ),
        // Розширення курсу (рішення A20). Натуральні числа.
        node(
            "N-ROUND",
            "Округлення натуральних чисел",
            vec!["N-ADD-MULTI"],
        ),
        node("N-DIVREM", "Ділення з остачею", vec!["N-DIV-MULTI"]),
        node("N-DIVTEST", "Ознаки подільності", vec!["N-DIVREM"]),
        node("N-GCD", "Найбільший спільний дільник", vec!["N-DIVTEST"]),
        node(
            "N-LCM",
            "Найменше спільне кратне",
            vec!["N-DIVTEST", "N-MUL-MULTI"],
        ),
        node("N-POWER", "Степінь числа", vec!["N-ORDER"]),
        // Рівняння й текстові задачі.
        node(
            "EQ-ADD-SUB",
            "Рівняння: додавання й віднімання",
            vec!["N-SUB-MULTI"],
        ),
        node(
            "EQ-MUL",
            "Рівняння: множення й ділення",
            vec!["EQ-ADD-SUB", "N-DIV-MULTI"],
        ),
        node(
            "WORD-1",
            "Текстові задачі в одну дію",
            vec!["N-SUB-MULTI", "N-DIV-MULTI"],
        ),
        node(
            "WORD-2",
            "Текстові задачі у дві дії",
            vec!["WORD-1", "N-ORDER"],
        ),
        node("MOTION", "Задачі на рух", vec!["WORD-2", "EQ-MUL"]),
        // Величини й геометрія.
        node(
            "U-LEN",
            "Одиниці довжини",
            vec!["N-MUL-MULTI", "N-DIV-MULTI"],
        ),
        node("U-MASS", "Одиниці маси", vec!["U-LEN"]),
        node("U-TIME", "Одиниці часу", vec!["U-LEN"]),
        node(
            "G-PERIM",
            "Периметр прямокутника й квадрата",
            vec!["N-MUL-MULTI", "EQ-ADD-SUB"],
        ),
        node(
            "G-AREA",
            "Площа прямокутника й квадрата",
            vec!["G-PERIM", "EQ-MUL"],
        ),
        node(
            "G-VOL",
            "Об'єм прямокутного паралелепіпеда",
            vec!["G-AREA", "N-POWER"],
        ),
        node(
            "A-ANGLE",
            "Кути: суміжні й у трикутнику",
            vec!["N-SUB-MULTI"],
        ),
        // Дроби, відсотки, десяткові, дані.
        node(
            "F-MIXED",
            "Неправильний дріб і мішане число",
            vec!["F-ADD-SAME", "N-DIVREM"],
        ),
        node("F-REDUCE", "Скорочення дробів", vec!["N-GCD", "F-ADD-SAME"]),
        node(
            "F-OF",
            "Частина від числа",
            vec!["F-ADD-SAME", "N-DIV-MULTI"],
        ),
        node("P-PCT", "Відсотки", vec!["F-OF"]),
        node(
            "D-MUL10",
            "Множення й ділення десяткових на 10, 100, 1000",
            vec!["D-ADD", "N-MUL-MULTI"],
        ),
        node(
            "AVG",
            "Середнє арифметичне",
            vec!["N-ADD-MULTI", "N-DIV-MULTI"],
        ),
    ])
    .expect("вбудований граф курсу коректний")
}

/// Старе ім'я (M0, чотири навички); тепер повертає весь граф курсу.
pub fn grade5_m0_graph() -> SkillGraph {
    grade5_graph()
}

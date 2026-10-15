//! Малюнки до задач і уроків (наочність: смужки дробів, сітка сотих, прямокутник, кут, числова пряма).
//!
//! Правило І1: малюнок до ЗАДАЧІ показує лише дані умови (операнди), ніколи відповідь чи проміжні значення.
//! Це перевіряється тестами курс-пакета через `values` і `texts`.

use crate::frac::Frac;

/// Схеми для уроків усіх предметів (текстові діаграми, що малюються як SVG). Дані лежать у `rows`:
/// Timeline `[мітка, текст]`; Compare перший рядок заголовок; Cycle і Flow `[крок]`; Mindmap перший рядок `[центр]`,
/// далі `[гілка, пункт, ...]`; Bars `[підпис, число]` (`title` одиниця); Sets `[A, B]`, `[лише A..]`, `[обидва..]`, `[лише B..]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagramKind {
    Timeline,
    Compare,
    Cycle,
    Flow,
    Mindmap,
    Bars,
    Sets,
}

impl DiagramKind {
    pub fn code(self) -> &'static str {
        match self {
            DiagramKind::Timeline => "timeline",
            DiagramKind::Compare => "compare",
            DiagramKind::Cycle => "cycle",
            DiagramKind::Flow => "flow",
            DiagramKind::Mindmap => "mindmap",
            DiagramKind::Bars => "bars",
            DiagramKind::Sets => "sets",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Figure {
    /// Схема з текстом (див. `DiagramKind`).
    Diagram {
        kind: DiagramKind,
        title: String,
        rows: Vec<Vec<String>>,
    },
    /// Смужка, поділена на `den` рівних частин, `num` зафарбовано (при `num > den` кілька смужок).
    FractionBar { num: i64, den: i64 },
    /// Квадрат 10×10 із зафарбованими `hundredths` клітинками (при > 100 кілька квадратів).
    DecimalGrid { hundredths: i64 },
    /// Прямокутник із підписаними сторонами (підписи довільний текст: «5 см», «a»).
    Rect { w: String, h: String },
    /// Кут у градусах (0..=180) з підписом.
    Angle { deg: i64, label: String },
    /// Числова пряма від `min` до `max`, на кожну одиницю `parts` поділок; позначені точки: (позиція в поділках, підпис).
    NumberLine {
        min: i64,
        max: i64,
        parts: i64,
        marks: Vec<(i64, String)>,
    },
}

impl Figure {
    /// Числа, які зображені (для перевірки, що малюнок не показує відповідь).
    pub fn values(&self) -> Vec<Frac> {
        match self {
            Figure::FractionBar { num, den } => Frac::new(*num, *den).into_iter().collect(),
            Figure::DecimalGrid { hundredths } => Frac::new(*hundredths, 100).into_iter().collect(),
            Figure::Rect { .. } | Figure::Diagram { .. } => Vec::new(),
            Figure::Angle { deg, .. } => vec![Frac::int(*deg)],
            Figure::NumberLine { parts, marks, .. } => marks
                .iter()
                .filter_map(|(pos, _)| Frac::new(*pos, (*parts).max(1)))
                .collect(),
        }
    }

    /// Тексти на малюнку (підписи), що їх бачить дитина.
    pub fn texts(&self) -> Vec<String> {
        match self {
            Figure::FractionBar { .. } | Figure::DecimalGrid { .. } => Vec::new(),
            Figure::Rect { w, h } => vec![w.clone(), h.clone()],
            Figure::Diagram { title, rows, .. } => std::iter::once(title.clone())
                .chain(rows.iter().flatten().cloned())
                .collect(),
            Figure::Angle { label, .. } => vec![label.clone()],
            Figure::NumberLine { marks, .. } => marks.iter().map(|(_, t)| t.clone()).collect(),
        }
    }

    /// Чи придатні параметри для малювання (обмеження захищають інтерфейс від величезних фігур).
    pub fn is_sane(&self) -> bool {
        match self {
            Figure::FractionBar { num, den } => {
                (1..=24).contains(den) && (0..=*den * 6).contains(num)
            }
            Figure::DecimalGrid { hundredths } => (0..=300).contains(hundredths),
            Figure::Diagram { kind, title, rows } => diagram_is_sane(*kind, title, rows),
            Figure::Rect { w, h } => w.chars().count() <= 20 && h.chars().count() <= 20,
            Figure::Angle { deg, label } => (0..=180).contains(deg) && label.chars().count() <= 20,
            Figure::NumberLine {
                min,
                max,
                parts,
                marks,
            } => {
                min < max
                    && max - min <= 20
                    && (1..=10).contains(parts)
                    && marks.len() <= 6
                    && marks.iter().all(|(p, t)| {
                        *p >= min * parts && *p <= max * parts && t.chars().count() <= 12
                    })
            }
        }
    }
}

fn diagram_is_sane(kind: DiagramKind, title: &str, rows: &[Vec<String>]) -> bool {
    let short = |s: &String| !s.trim().is_empty() && s.chars().count() <= 60;
    if title.chars().count() > 40 || rows.len() > 10 || !rows.iter().flatten().all(short) {
        return false;
    }
    let wide = |r: &Vec<String>, lo: usize, hi: usize| (lo..=hi).contains(&r.len());
    match kind {
        DiagramKind::Timeline => (2..=8).contains(&rows.len()) && rows.iter().all(|r| r.len() == 2),
        DiagramKind::Compare => {
            rows.len() >= 2 && wide(&rows[0], 2, 4) && rows.iter().all(|r| r.len() == rows[0].len())
        }
        DiagramKind::Cycle => (3..=8).contains(&rows.len()) && rows.iter().all(|r| r.len() == 1),
        DiagramKind::Flow => (2..=7).contains(&rows.len()) && rows.iter().all(|r| r.len() == 1),
        DiagramKind::Mindmap => {
            (2..=7).contains(&rows.len())
                && rows[0].len() == 1
                && rows[1..].iter().all(|r| wide(r, 2, 4))
        }
        DiagramKind::Bars => {
            (2..=8).contains(&rows.len())
                && rows.iter().all(|r| {
                    r.len() == 2
                        && r[1]
                            .parse::<i64>()
                            .is_ok_and(|v| (0..=1_000_000).contains(&v))
                })
        }
        DiagramKind::Sets => {
            rows.len() == 4
                && rows[0].len() == 2
                && rows[0].iter().all(|s| !s.trim().is_empty())
                && rows[1..].iter().all(|r| r.len() <= 4)
        }
    }
}

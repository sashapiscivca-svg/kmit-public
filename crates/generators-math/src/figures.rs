//! Малюнки до умов задач (P6): лише дані умови, ніколи відповідь чи проміжні значення (І1).
//! Центральний вибір за навичкою; кожен набір малюнків додатково проходить перевірку `is_sane` і захист від витоку.

use tutor_core::figure::Figure;
use tutor_core::numtok::numeric_tokens;
use tutor_core::task::Task;

/// Малюнки до задачі (порожньо, якщо малюнок не допомагає або міг би видати відповідь).
pub(crate) fn for_task(task: &Task) -> Vec<Figure> {
    let p = &task.params;
    let figs = match task.skill {
        "F-ADD-SAME" | "F-SUB-SAME" => vec![bar(p[0], p[2]), bar(p[1], p[2])],
        "F-ADD-DIFF" | "F-SUB-DIFF" => vec![bar(p[0], p[1]), bar(p[2], p[3])],
        // Смужка з кількох цілих покаже кількість цілих, тому лише для переходу до неправильного дробу.
        "F-MIXED" if p[0] == 2 => vec![bar(task.operands[1].num(), task.operands[1].den())],
        "F-OF" => vec![bar(p[1], p[2])],
        "D-ADD" => vec![
            Figure::DecimalGrid {
                hundredths: p[0] * 10,
            },
            Figure::DecimalGrid {
                hundredths: p[1] * 10,
            },
        ],
        "D-SUB" => vec![
            Figure::DecimalGrid { hundredths: p[0] },
            Figure::DecimalGrid {
                hundredths: p[1] * 10,
            },
        ],
        "G-PERIM" | "G-AREA" => rect_figures(task),
        "A-ANGLE" => task
            .operands
            .iter()
            .filter(|o| o.is_integer())
            .map(|o| Figure::Angle {
                deg: o.num(),
                label: format!("{}°", o.num()),
            })
            .collect(),
        // N-ROUND: числова пряма з позначкою видавала б напрямок округлення (відповідь), тож у задачах її немає; є в уроці.
        _ => Vec::new(),
    };
    // Усе або нічого: частковий малюнок (одна смужка з двох) збиває з пантелику.
    if figs.iter().all(|f| is_safe(task, f)) {
        figs
    } else {
        Vec::new()
    }
}

fn bar(num: i64, den: i64) -> Figure {
    Figure::FractionBar { num, den }
}

fn is_safe(task: &Task, f: &Figure) -> bool {
    let prot = task.protected_values();
    f.is_sane()
        && f.values().iter().all(|v| !prot.contains(v))
        && f.texts()
            .iter()
            .all(|t| numeric_tokens(t).iter().all(|v| !prot.contains(v)))
}

/// Число `label` з умови разом з одиницею, що йде за ним: «12 см».
fn with_unit(statement: &str, label: &str) -> Option<String> {
    let toks: Vec<&str> = statement.split_whitespace().collect();
    let pos = toks.iter().position(|t| *t == label)?;
    let unit = toks
        .get(pos + 1)?
        .trim_matches(|c: char| c == '.' || c == ',' || c == '?');
    Some(format!("{label} {unit}"))
}

/// Прямокутник для задач на периметр, площу й об'єм: підписані лише сторони, дані в умові
/// (невідома сторона позначена «?»).
fn rect_figures(task: &Task) -> Vec<Figure> {
    let kind = task.params[0];
    // Індекси операндів для довжини й ширини; `None` означає невідому сторону.
    let sides: Option<(Option<usize>, Option<usize>)> = match (task.skill, kind) {
        ("G-PERIM" | "G-AREA" | "G-VOL", 0) => Some((Some(0), Some(1))),
        ("G-PERIM" | "G-AREA" | "G-VOL", 1) => Some((Some(0), Some(0))),
        ("G-PERIM" | "G-AREA", 2) => Some((Some(1), None)),
        _ => None,
    };
    let Some((w, h)) = sides else {
        return Vec::new();
    };
    let get = |s: Option<usize>| match s {
        Some(i) => with_unit(&task.statement, task.operand_labels.get(i)?),
        None => Some("?".to_string()),
    };
    match (get(w), get(h)) {
        (Some(w), Some(h)) => vec![Figure::Rect { w, h }],
        _ => Vec::new(),
    }
}

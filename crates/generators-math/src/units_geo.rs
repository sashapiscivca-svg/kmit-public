//! Величини й геометрія: перетворення одиниць довжини, маси, часу; периметр, площа, об'єм; кути.
//! Сталі співвідношення в підказках записано словами (число-слово сканер витоків не бачить, а сталі на кшталт 100
//! могли б збігтися з відповіддю чи проміжним числом задачі).

use crate::kit::{by_level, fr, ints, pick, Make, Spec};
use tutor_core::frac::Frac;
use tutor_core::generator::ScriptedLevel;
use tutor_core::rng::SeededRng;
use tutor_core::task::Task;

fn i(n: i64) -> Frac {
    Frac::int(n)
}

// ---------------------------------------------------------------------------------------------
// Перетворення одиниць

/// Пара одиниць: більша й менша; множник; мовні форми цільових одиниць.
struct UnitPair {
    big: &'static str,
    small: &'static str,
    /// Менша одиниця у знахідному множини («сантиметри»).
    small_acc: &'static str,
    /// Більша одиниця у знахідному множини.
    big_acc: &'static str,
    /// Менша одиниця у місцевому множини («сантиметрах»).
    small_loc: &'static str,
    factor: i64,
}

const fn up(
    big: &'static str,
    small: &'static str,
    big_acc: &'static str,
    small_acc: &'static str,
    small_loc: &'static str,
    factor: i64,
) -> UnitPair {
    UnitPair {
        big,
        small,
        small_acc,
        big_acc,
        small_loc,
        factor,
    }
}

const LEN: [UnitPair; 6] = [
    up("км", "м", "кілометри", "метри", "метрах", 1000),
    up("м", "дм", "метри", "дециметри", "дециметрах", 10),
    up("м", "см", "метри", "сантиметри", "сантиметрах", 100),
    up("дм", "см", "дециметри", "сантиметри", "сантиметрах", 10),
    up("см", "мм", "сантиметри", "міліметри", "міліметрах", 10),
    up("м", "мм", "метри", "міліметри", "міліметрах", 1000),
];

const MASS: [UnitPair; 4] = [
    up("т", "кг", "тонни", "кілограми", "кілограмах", 1000),
    up("кг", "г", "кілограми", "грами", "грамах", 1000),
    up("ц", "кг", "центнери", "кілограми", "кілограмах", 100),
    up("т", "ц", "тонни", "центнери", "центнерах", 10),
];

const TIME: [UnitPair; 2] = [
    up("год", "хв", "години", "хвилини", "хвилинах", 60),
    up("хв", "с", "хвилини", "секунди", "секундах", 60),
];

/// Множник словами у давальному відмінку («одна одиниця дорівнює ста одиницям»).
fn factor_dative(f: i64) -> &'static str {
    match f {
        10 => "десяти",
        60 => "шістдесяти",
        100 => "ста",
        _ => "тисячі",
    }
}

/// Типова помилка «не те число нулів» (або «десяткова система для часу»): інший множник.
fn slip(family: usize, f: i64) -> Option<(&'static str, i64)> {
    match (family, f) {
        (2, _) => Some(("decimal_system", 100)),
        (_, 100) | (_, 1000) => Some(("wrong_zeros", f / 10)),
        _ => None,
    }
}

fn units_make(
    skill: &'static str,
    family: usize,
    pairs: &[UnitPair],
    rng: &mut SeededRng,
    variant: u64,
) -> Option<(Task, Frac)> {
    let pi = rng.range(0, pairs.len() as i64 - 1) as usize;
    let p = &pairs[pi];
    let f = p.factor;
    let kind = rng.range(0, 2);
    let n = if kind == 2 {
        rng.range(1, 20)
    } else {
        rng.range(2, 60)
    };
    let sl = slip(family, f);
    let (statement, operands, answer, oracle, bugs, k) = match kind {
        0 => {
            let mut bugs = vec![("wrong_direction", fr(n, f))];
            if let Some((id, s)) = sl {
                bugs.push((id, i(n * s)));
            }
            (
                format!("Переведи {n} {} у {}.", p.big, p.small_acc),
                vec![n],
                n * f,
                (0..n).map(|_| f).sum::<i64>(),
                bugs,
                0,
            )
        }
        1 => {
            let m = n * f;
            let mut bugs = vec![("wrong_direction", i(m * f))];
            if let Some((id, s)) = sl {
                bugs.push((id, fr(m, s)));
            }
            let (mut left, mut count) = (m, 0);
            while left >= f {
                left -= f;
                count += 1;
            }
            (
                format!("Переведи {m} {} у {}.", p.small, p.big_acc),
                vec![m],
                n,
                count,
                bugs,
                0,
            )
        }
        _ => {
            let k = rng.range(1, f - 1);
            let mut bugs = vec![("add_only", i(n + k))];
            if let Some((id, s)) = sl {
                bugs.push((id, i(n * s + k)));
            }
            (
                format!("Запиши в {}: {n} {} {k} {}.", p.small_loc, p.big, p.small),
                vec![n, k],
                n * f + k,
                (0..n).fold(k, |s, _| s + f),
                bugs,
                k,
            )
        }
    };
    let task = Make {
        skill,
        variant,
        statement,
        operands: ints(&operands),
        params: vec![family as i64, pi as i64, kind, n, k],
        answer: i(answer),
        steps: vec![],
        bugs,
    }
    .build();
    Some((task, i(oracle)))
}

fn pair_of(task: &Task) -> &'static UnitPair {
    let idx = task.params[1] as usize;
    match task.params[0] {
        0 => &LEN[idx],
        1 => &MASS[idx],
        _ => &TIME[idx],
    }
}

fn units_hint(task: &Task, level: ScriptedLevel) -> String {
    let p = pair_of(task);
    let r2 = match task.params[2] {
        0 => "Ми переходимо до менших одиниць, тому їх потрібно більше: множимо.",
        1 => "Ми переходимо до більших одиниць, тому їх потрібно менше: ділимо.",
        _ => "Спочатку переведи більшу одиницю в меншу, а потім додай решту.",
    };
    by_level(
        level,
        format!(
            "Яка з одиниць, «{}» чи «{}», більша? Скільки менших одиниць міститься в одній більшій?",
            p.big, p.small
        ),
        r2.to_string(),
        format!(
            "Згадай співвідношення: одна одиниця «{}» дорівнює {} одиницям «{}». Подумай, чи має відповідь стати більшою, чи меншою за дане число.",
            p.big,
            factor_dative(p.factor),
            p.small
        ),
    )
}

fn units_steps(c: &Task) -> Vec<String> {
    let p = pair_of(c);
    let how = match c.params[2] {
        0 => "Переходимо до менших одиниць, тому множимо на кількість менших одиниць в одній більшій.",
        1 => "Переходимо до більших одиниць, тому ділимо на кількість менших одиниць в одній більшій.",
        _ => "Більшу частину переводимо в менші одиниці множенням, а потім додаємо решту.",
    };
    vec![
        format!(
            "Порівнюємо одиниці «{}» і «{}» та згадуємо, скільки менших одиниць у більшій.",
            p.big, p.small
        ),
        how.to_string(),
    ]
}

macro_rules! units_spec {
    ($name:ident, $id:expr, $family:expr, $pairs:expr, $make:ident) => {
        fn $make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
            units_make($id, $family, &$pairs, rng, variant)
        }
        pub const $name: Spec = Spec {
            id: $id,
            make: $make,
            hint: units_hint,
            steps: units_steps,
            prompt: "Тепер ти: виконай перетворення й запиши число.",
        };
    };
}

units_spec!(U_LEN, "U-LEN", 0, LEN, u_len_make);
units_spec!(U_MASS, "U-MASS", 1, MASS, u_mass_make);
units_spec!(U_TIME, "U-TIME", 2, TIME, u_time_make);

// ---------------------------------------------------------------------------------------------
// Геометрія: периметр, площа, об'єм

const LEN_UNITS: [&str; 5] = ["см", "м", "дм", "мм", "км"];
const RECT_GEN: [&str; 5] = [
    "прямокутника",
    "прямокутної ділянки",
    "прямокутного аркуша",
    "прямокутної кімнати",
    "прямокутного поля",
];
const SQUARE_GEN: [&str; 4] = [
    "квадрата",
    "квадратної ділянки",
    "квадратного аркуша",
    "квадратної кімнати",
];

pub const G_PERIM: Spec = Spec {
    id: "G-PERIM",
    make: perim_make,
    hint: perim_hint,
    steps: perim_steps,
    prompt: "Тепер ти: доведи обчислення до кінця й запиши число.",
};

/// Чи правдоподібні одиниці й розміри для названого предмета (загальні «прямокутник» і «квадрат» годяться з будь-якими).
fn plausible(noun: &str, unit: &str, side: i64) -> bool {
    if noun.contains("ділянки") || noun.contains("поля") {
        unit == "м"
    } else if noun.contains("аркуша") {
        (unit == "см" && side <= 60) || (unit == "дм" && side <= 6)
    } else if noun.contains("кімнати") {
        unit == "м" && side <= 12
    } else {
        true
    }
}

fn perim_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let kind = rng.range(0, 3);
    let u = *pick(rng, &LEN_UNITS);
    let (a, b) = (rng.range(3, 60), rng.range(3, 60));
    let rect = *pick(rng, &RECT_GEN);
    let square = *pick(rng, &SQUARE_GEN);
    if !plausible(if kind % 2 == 0 { rect } else { square }, u, a.max(b)) {
        return None;
    }
    let (statement, operands, answer, oracle, steps, bugs) = match kind {
        0 => {
            if a == b {
                return None;
            }
            (
                format!("Знайди периметр {rect} зі сторонами {a} {u} і {b} {u}."),
                vec![a, b],
                2 * (a + b),
                [a, b, a, b].iter().sum::<i64>(),
                vec![],
                vec![("half_perimeter", i(a + b)), ("area_instead", i(a * b))],
            )
        }
        1 => (
            format!("Знайди периметр {square} зі стороною {a} {u}."),
            vec![a],
            4 * a,
            [a, a, a, a].iter().sum::<i64>(),
            vec![],
            vec![("area_instead", i(a * a)), ("three_sides", i(3 * a))],
        ),
        2 => {
            if a == b {
                return None;
            }
            let p = 2 * (a + b);
            (
                format!("Периметр {rect} дорівнює {p} {u}, одна зі сторін має довжину {a} {u}. Знайди довжину сусідньої сторони."),
                vec![p, a],
                b,
                (1..).find(|s| a + s + a + s == p)?,
                vec![i(a + b)],
                vec![("forgot_halving", i(p - a)), ("halve_only", i(a + b))],
            )
        }
        _ => {
            let p = 4 * a;
            (
                format!("Периметр {square} дорівнює {p} {u}. Знайди довжину сторони."),
                vec![p],
                a,
                (1..).find(|s| s * 4 == p)?,
                vec![],
                vec![("divide_by_two", i(p / 2)), ("three_sides", fr(p, 3))],
            )
        }
    };
    let task = Make {
        skill: "G-PERIM",
        variant,
        statement,
        operands: ints(&operands),
        params: vec![kind],
        answer: i(answer),
        steps,
        bugs,
    }
    .build();
    Some((task, i(oracle)))
}

fn perim_hint(task: &Task, level: ScriptedLevel) -> String {
    let r2 = match task.params[0] {
        0 => "У прямокутника протилежні сторони рівні: є дві довжини й дві ширини.",
        1 => "У квадрата всі чотири сторони однакові.",
        2 => "Дві сусідні сторони прямокутника разом дають половину периметра.",
        _ => "Усі чотири сторони квадрата рівні, тому периметр складається з чотирьох однакових доданків.",
    };
    let r3 = match task.params[0] {
        0 | 1 => "Периметр вимірюють у тих самих одиницях довжини, що й сторони. Перевір, чи враховано всі сторони.",
        _ => "Підстав знайдену сторону назад і перевір, чи периметр збігається з даним.",
    };
    by_level(
        level,
        "Периметр — це сума довжин усіх сторін фігури. Скільки сторін у цієї фігури й які з них рівні?".to_string(),
        r2.to_string(),
        r3.to_string(),
    )
}

fn perim_steps(c: &Task) -> Vec<String> {
    let s = match c.params[0] {
        0 => "Складаємо довжини всіх чотирьох сторін: дві довжини й дві ширини.",
        1 => "Складаємо довжини всіх чотирьох однакових сторін.",
        2 => "Спершу знаходимо, скільки становлять дві сусідні сторони разом, а потім віднімаємо відому.",
        _ => "Периметр ділимо на кількість однакових сторін.",
    };
    vec![
        s.to_string(),
        "Записуємо результат у тих самих одиницях довжини.".to_string(),
    ]
}

pub const G_AREA: Spec = Spec {
    id: "G-AREA",
    make: area_make,
    hint: area_hint,
    steps: area_steps,
    prompt: "Тепер ти: доведи обчислення до кінця й запиши число.",
};

fn area_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let kind = rng.range(0, 3);
    let u = *pick(rng, &LEN_UNITS);
    let (a, b) = (rng.range(2, 30), rng.range(2, 30));
    let rect = *pick(rng, &RECT_GEN);
    let square = *pick(rng, &SQUARE_GEN);
    if !plausible(if kind % 2 == 0 { rect } else { square }, u, a.max(b)) {
        return None;
    }
    let rows = |len: i64, count: i64| (0..count).map(|_| len).sum::<i64>();
    let (statement, operands, answer, oracle, bugs) = match kind {
        0 => {
            if a == b {
                return None;
            }
            (
                format!("Знайди площу {rect} зі сторонами {a} {u} і {b} {u}."),
                vec![a, b],
                a * b,
                rows(a, b),
                vec![
                    ("perimeter_instead", i(2 * (a + b))),
                    ("sum_sides", i(a + b)),
                ],
            )
        }
        1 => (
            format!("Знайди площу {square} зі стороною {a} {u}."),
            vec![a],
            a * a,
            rows(a, a),
            vec![("double_side", i(2 * a)), ("perimeter_instead", i(4 * a))],
        ),
        2 => {
            if a == b {
                return None;
            }
            let s = a * b;
            (
                format!("Площа {rect} дорівнює {s} {u}², одна зі сторін має довжину {a} {u}. Знайди довжину сусідньої сторони."),
                vec![s, a],
                b,
                (1..).find(|k| rows(a, *k) == s)?,
                vec![("multiply_instead", i(s * a)), ("subtract_instead", i(s - a))],
            )
        }
        _ => {
            let s = a * a;
            (
                format!("Площа {square} дорівнює {s} {u}². Знайди довжину сторони."),
                vec![s],
                a,
                (1..).find(|k| rows(*k, *k) == s)?,
                vec![
                    ("divide_by_two", fr(s, 2)),
                    ("perimeter_confusion", fr(s, 4)),
                ],
            )
        }
    };
    let task = Make {
        skill: "G-AREA",
        variant,
        statement,
        operands: ints(&operands),
        params: vec![kind],
        answer: i(answer),
        steps: vec![],
        bugs,
    }
    .build();
    Some((task, i(oracle)))
}

fn area_hint(task: &Task, level: ScriptedLevel) -> String {
    let r2 = match task.params[0] {
        0 => "Площа прямокутника дорівнює добутку його довжини на ширину.",
        1 => "Квадрат — це прямокутник, у якого довжина й ширина однакові: площа дорівнює стороні, помноженій на себе.",
        2 => "Площа прямокутника це добуток двох сусідніх сторін. Невідомий множник знаходять діленням.",
        _ => "Шукаємо число, яке, помножене саме на себе, дає площу квадрата.",
    };
    by_level(
        level,
        "Площа показує, скільки квадратиків одиничного розміру вміщається у фігурі. Не плутай її з периметром, який є сумою довжин сторін.".to_string(),
        r2.to_string(),
        match task.params[0] {
            0 | 1 => "Площу вимірюють у квадратних одиницях. Перевір себе: чи правильно перемножено сторони.".to_string(),
            2 => "Площу вимірюють у квадратних одиницях. Перевір себе: підстав знайдену сторону назад і перевір добуток.".to_string(),
            _ => "Площу вимірюють у квадратних одиницях. Перевір себе: знайдена сторона, помножена сама на себе, має дати площу.".to_string(),
        },
    )
}

fn area_steps(c: &Task) -> Vec<String> {
    let s = match c.params[0] {
        0 => "Площа прямокутника: довжину множимо на ширину.",
        1 => "Площа квадрата: сторону множимо на себе.",
        2 => "Площу ділимо на відому сторону й отримуємо сусідню.",
        _ => "Шукаємо число, яке, помножене на себе, дорівнює площі.",
    };
    vec![
        s.to_string(),
        "Підставляємо числа з умови й обчислюємо.".to_string(),
    ]
}

pub const G_VOL: Spec = Spec {
    id: "G-VOL",
    make: vol_make,
    hint: vol_hint,
    steps: vol_steps,
    prompt: "Тепер ти: доведи обчислення до кінця й запиши число.",
};

const BOX_WHO: [&str; 5] = ["Акваріум", "Контейнер", "Басейн", "Бак", "Ящик"];
const VOL_UNITS: [&str; 3] = ["см", "дм", "м"];

fn vol_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let kind = rng.range(0, 2);
    let u = *pick(rng, &VOL_UNITS);
    let (a, b, c) = (rng.range(2, 12), rng.range(2, 12), rng.range(2, 12));
    let layer = |x: i64, y: i64| (0..y).map(|_| x).sum::<i64>();
    let (statement, operands, answer, oracle, bugs) = match kind {
        0 => {
            let who = *pick(rng, &BOX_WHO);
            // Предмет і одиниця мають бути правдоподібними разом (акваріум у дм, басейн у м).
            let fits = match who {
                "Контейнер" | "Басейн" => u == "м",
                _ => u == "дм",
            };
            if !fits {
                return None;
            }
            (
                format!("{who} має форму прямокутного паралелепіпеда з довжиною {a} {u}, шириною {b} {u} і висотою {c} {u}. Знайди його об'єм."),
                vec![a, b, c],
                a * b * c,
                (0..c).map(|_| layer(a, b)).sum::<i64>(),
                vec![("edges_total", i(4 * (a + b + c))), ("forgot_height", i(a * b))],
            )
        }
        1 => (
            format!("Знайди об'єм куба з ребром {a} {u}."),
            vec![a],
            a * a * a,
            (0..a).map(|_| layer(a, a)).sum::<i64>(),
            vec![("triple_edge", i(3 * a)), ("forgot_third_factor", i(a * a))],
        ),
        _ => {
            let s = a * b;
            let v = s * c;
            (
                format!("Об'єм прямокутного паралелепіпеда дорівнює {v} {u}³, площа його основи {s} {u}². Знайди висоту."),
                vec![v, s],
                c,
                (1..).find(|h| s * h == v)?,
                vec![("multiply_instead", i(v * s)), ("subtract_instead", i(v - s))],
            )
        }
    };
    let task = Make {
        skill: "G-VOL",
        variant,
        statement,
        operands: ints(&operands),
        params: vec![kind],
        answer: i(answer),
        steps: vec![],
        bugs,
    }
    .build();
    Some((task, i(oracle)))
}

fn vol_hint(task: &Task, level: ScriptedLevel) -> String {
    let r2 = match task.params[0] {
        0 => "Об'єм прямокутного паралелепіпеда дорівнює добутку трьох вимірів: довжини, ширини й висоти.",
        1 => "Куб має три однакові виміри, тому його об'єм дорівнює ребру, помноженому на себе й ще раз на себе.",
        _ => "Об'єм дорівнює площі основи, помноженій на висоту. Невідомий множник знаходять діленням.",
    };
    by_level(
        level,
        "Об'єм показує, скільки кубиків одиничного розміру вміщається у фігурі. Не плутай його із сумою довжин ребер.".to_string(),
        r2.to_string(),
        if task.params[0] == 2 {
            "Об'єм вимірюють у кубічних одиницях. Підстав знайдену висоту й перевір добуток.".to_string()
        } else {
            "Об'єм вимірюють у кубічних одиницях. Перевір, чи всі три виміри перемножено.".to_string()
        },
    )
}

fn vol_steps(c: &Task) -> Vec<String> {
    let s = match c.params[0] {
        0 => "Перемножуємо три виміри: довжину, ширину й висоту.",
        1 => "Три однакові виміри перемножуємо: ребро множимо на себе двічі.",
        _ => "Об'єм ділимо на площу основи й отримуємо висоту.",
    };
    vec![
        s.to_string(),
        "Підставляємо числа з умови й обчислюємо.".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// A-ANGLE: суміжні кути, доповнення до прямого, третій кут трикутника

pub const A_ANGLE: Spec = Spec {
    id: "A-ANGLE",
    make: angle_make,
    hint: angle_hint,
    steps: angle_steps,
    prompt: "Тепер ти: знайди невідомий кут і запиши його градусну міру.",
};

#[allow(clippy::type_complexity)]
fn angle_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let kind = rng.range(0, 3);
    let x = match kind {
        0 => {
            let acute = rng.range(0, 1) == 0;
            if acute {
                rng.range(10, 80)
            } else {
                rng.range(100, 170)
            }
        }
        2 => rng.range(15, 100),
        _ => rng.range(10, 80),
    };
    let (statement, operands, answer, total, bugs): (String, Vec<i64>, i64, i64, Vec<(&'static str, Frac)>) = match kind {
        0 => (
            format!("Один із двох суміжних кутів дорівнює {x}°. Знайди другий суміжний кут."),
            vec![x],
            180 - x,
            180,
            vec![("use_right_angle", i((90 - x).abs())), ("full_turn", i(360 - x))],
        ),
        1 => (
            format!("Два кути разом утворюють прямий кут. Один із них дорівнює {x}°. Знайди другий кут."),
            vec![x],
            90 - x,
            90,
            vec![("use_straight_angle", i(180 - x))],
        ),
        2 => {
            let y = rng.range(15, 100);
            if x + y > 160 {
                return None;
            }
            (
                format!("У трикутнику два кути дорівнюють {x}° і {y}°. Знайди третій кут."),
                vec![x, y],
                180 - x - y,
                180,
                vec![("use_full_turn", i(360 - x - y)), ("forgot_one_angle", i(180 - x))],
            )
        }
        _ => (
            format!("У прямокутному трикутнику один гострий кут дорівнює {x}°. Знайди другий гострий кут."),
            vec![x],
            90 - x,
            90,
            vec![("use_straight_angle", i(180 - x))],
        ),
    };
    // Оракул: підбір кута, що доповнює суму до потрібної.
    let known: i64 = operands.iter().sum();
    let oracle = (1..=179).find(|z| known + z == total).unwrap_or(-1);
    let steps = if kind == 2 {
        vec![i(x + operands[1])]
    } else {
        vec![]
    };
    let task = Make {
        skill: "A-ANGLE",
        variant,
        statement,
        operands: ints(&operands),
        params: vec![kind],
        answer: i(answer),
        steps,
        bugs,
    }
    .build();
    Some((task, i(oracle)))
}

fn angle_hint(task: &Task, level: ScriptedLevel) -> String {
    let (r2, r3) = match task.params[0] {
        0 => (
            "Суміжні кути мають спільну сторону, а інші їхні сторони утворюють розгорнутий кут. Тому суміжні кути разом утворюють розгорнутий кут.",
            "Градусна міра розгорнутого кута дорівнює ста вісімдесяти градусам. Від неї відніми відомий кут.",
        ),
        1 | 3 => (
            "Прямий кут складається з двох кутів. У прямокутному трикутнику два гострі кути разом утворюють прямий кут.",
            "Градусна міра прямого кута дорівнює дев'яноста градусам. Від неї відніми відомий кут.",
        ),
        _ => (
            "Сума кутів будь-якого трикутника однакова, і це число треба пам'ятати.",
            "Сума кутів трикутника дорівнює ста вісімдесяти градусам. Додай відомі кути й відніми суму від цього числа.",
        ),
    };
    by_level(
        level,
        "Які кути тут відомі, а який треба знайти? Що відомо про суму цих кутів?".to_string(),
        r2.to_string(),
        r3.to_string(),
    )
}

fn angle_steps(c: &Task) -> Vec<String> {
    let s = match c.params[0] {
        0 => "Суміжні кути разом утворюють розгорнутий кут, тому віднімаємо відомий кут від градусної міри розгорнутого кута.",
        1 | 3 => "Прямий кут складається з двох кутів, тому віднімаємо відомий кут від градусної міри прямого кута.",
        _ => "Додаємо два відомі кути трикутника, а потім віднімаємо суму від градусної міри суми кутів трикутника.",
    };
    vec![
        s.to_string(),
        "Перевіряємо: відомий кут і знайдений разом дають потрібну суму.".to_string(),
    ]
}

//! Незалежний розв'язувач умов з уроків нових навичок: розбирає текст умови й рахує відповідь
//! власним способом (без коду генераторів). Якщо умови не вдається розібрати, тест падає.

use tutor_core::frac::{parse_answer, Frac};

fn int(n: i64) -> Frac {
    Frac::int(n)
}

fn ratio(a: i64, b: i64) -> Frac {
    Frac::new(a, b).expect("ненульовий знаменник")
}

/// Усі цілі числа з тексту (серії ASCII-цифр).
fn ints_in(s: &str) -> Vec<i64> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(cur.parse().unwrap());
            cur.clear();
        }
    }
    if !cur.is_empty() {
        out.push(cur.parse().unwrap());
    }
    out
}

/// Дроби виду a/b у тексті у «сирому» вигляді (без скорочення).
fn fractions_in(s: &str) -> Vec<(i64, i64)> {
    let cs: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        if cs[i].is_ascii_digit() {
            let start = i;
            while i < cs.len() && cs[i].is_ascii_digit() {
                i += 1;
            }
            if i + 1 < cs.len() && cs[i] == '/' && cs[i + 1].is_ascii_digit() {
                let a: i64 = cs[start..i].iter().collect::<String>().parse().unwrap();
                let mut j = i + 1;
                while j < cs.len() && cs[j].is_ascii_digit() {
                    j += 1;
                }
                let b: i64 = cs[i + 1..j].iter().collect::<String>().parse().unwrap();
                out.push((a, b));
                i = j;
            }
        } else {
            i += 1;
        }
    }
    out
}

fn gcd(a: i64, b: i64) -> i64 {
    (1..=a.min(b))
        .rev()
        .find(|d| a % d == 0 && b % d == 0)
        .unwrap()
}

fn lcm(a: i64, b: i64) -> i64 {
    (1..)
        .map(|k| a.max(b) * k)
        .find(|m| m % a.min(b) == 0)
        .unwrap()
}

fn superscript(c: char) -> Option<u32> {
    "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().position(|s| s == c).map(|p| p as u32)
}

fn power(st: &str) -> Frac {
    let body = st.strip_prefix("Обчисли: ").unwrap();
    let cs: Vec<char> = body.chars().collect();
    let mut i = 0;
    let mut base = String::new();
    while cs[i].is_ascii_digit() {
        base.push(cs[i]);
        i += 1;
    }
    let mut exp = 0;
    while i < cs.len() {
        match superscript(cs[i]) {
            Some(d) => {
                exp = exp * 10 + d;
                i += 1;
            }
            None => break,
        }
    }
    let base: i64 = base.parse().unwrap();
    let mut value = (0..exp).fold(1_i64, |acc, _| acc * base);
    let rest: String = cs[i..].iter().collect();
    let rest = rest.trim();
    if let Some(c) = rest.strip_prefix("+ ") {
        value += c.parse::<i64>().unwrap();
    } else if let Some(c) = rest.strip_prefix("- ") {
        value -= c.parse::<i64>().unwrap();
    } else {
        assert!(rest.is_empty(), "невідомий залишок виразу: {st}");
    }
    int(value)
}

fn equation(st: &str) -> Frac {
    let body = st.strip_prefix("Розв'яжи рівняння: ").unwrap();
    let t: Vec<&str> = body.split(' ').collect();
    assert!(t.len() == 5 && t[3] == "=", "{st}");
    let rhs: i64 = t[4].parse().unwrap();
    let side = |x: i64, s: &str| -> Option<i64> {
        if s == "x" {
            Some(x)
        } else {
            s.parse().ok()
        }
    };
    let found = (0..100_000).find(|&x| {
        let (a, b) = (side(x, t[0]).unwrap(), side(x, t[2]).unwrap());
        match t[1] {
            "+" => a + b == rhs,
            "-" => a - b == rhs,
            "×" => a * b == rhs,
            ":" => b != 0 && a % b == 0 && a / b == rhs,
            op => panic!("невідома дія {op}"),
        }
    });
    int(found.unwrap_or_else(|| panic!("рівняння без цілого кореня: {st}")))
}

/// Розмір одиниці в найменшій одиниці своєї величини (мм, г, с).
fn unit_size(abbr: &str) -> Option<i64> {
    Some(match abbr {
        "мм" | "г" | "с" => 1,
        "см" => 10,
        "дм" => 100,
        "м" => 1000,
        "км" => 1_000_000,
        "кг" => 1000,
        "ц" => 100_000,
        "т" => 1_000_000,
        "хв" => 60,
        "год" => 3600,
        _ => return None,
    })
}

fn unit_from_word(word: &str) -> i64 {
    let w = word.trim_matches(|c| c == '.' || c == ':');
    // Довші основи перевіряємо раніше за коротші («кілометр» містить «метр»).
    for (stem, abbr) in [
        ("кілометр", "км"),
        ("сантиметр", "см"),
        ("міліметр", "мм"),
        ("дециметр", "дм"),
        ("метр", "м"),
        ("кілограм", "кг"),
        ("грам", "г"),
        ("центнер", "ц"),
        ("тонн", "т"),
        ("секунд", "с"),
        ("хвилин", "хв"),
        ("годин", "год"),
    ] {
        if w.starts_with(stem) {
            return unit_size(abbr).unwrap();
        }
    }
    panic!("невідома одиниця «{word}»");
}

fn units(st: &str) -> Frac {
    let t: Vec<&str> = st.split(' ').collect();
    let clean = |s: &str| s.trim_matches(|c| c == '.' || c == ':').to_string();
    if t[0] == "Переведи" {
        let n: i64 = t[1].parse().unwrap();
        let from = unit_size(&clean(t[2])).unwrap();
        let to = unit_from_word(t[4]);
        ratio(n * from, to)
    } else {
        assert_eq!(t[0], "Запиши", "{st}");
        let to = unit_from_word(t[2]);
        let mut total = 0;
        let mut i = 3;
        while i + 1 < t.len() {
            let n: i64 = t[i].parse().unwrap();
            total += n * unit_size(&clean(t[i + 1])).unwrap();
            i += 2;
        }
        ratio(total, to)
    }
}

fn geometry(st: &str) -> Option<Frac> {
    let n = ints_in(st);
    let root = |s: i64| (1..).find(|r| r * r == s).unwrap();
    Some(int(
        if st.starts_with("Знайди периметр") && st.contains("прямокутн") {
            2 * (n[0] + n[1])
        } else if st.starts_with("Знайди периметр") && st.contains("квадрат") {
            4 * n[0]
        } else if st.starts_with("Периметр") && st.contains("прямокутн") {
            n[0] / 2 - n[1]
        } else if st.starts_with("Периметр") && st.contains("квадрат") {
            n[0] / 4
        } else if st.starts_with("Скільки сторін ураховують") {
            4
        } else if st.starts_with("Знайди площу") && st.contains("прямокутн") {
            n[0] * n[1]
        } else if st.starts_with("Знайди площу") && st.contains("квадрат") {
            n[0] * n[0]
        } else if st.starts_with("Площа") && st.contains("прямокутн") {
            assert_eq!(n[0] % n[1], 0);
            n[0] / n[1]
        } else if st.starts_with("Площа") && st.contains("квадрат") {
            root(n[0])
        } else if st.starts_with("Знайди об'єм куба") {
            n[0] * n[0] * n[0]
        } else if st.contains("з довжиною") && st.contains("шириною") {
            n[0] * n[1] * n[2]
        } else if st.starts_with("Об'єм прямокутного паралелепіпеда дорівнює")
        {
            assert_eq!(n[0] % n[1], 0);
            n[0] / n[1]
        } else if st.starts_with("Скільки літрів води вміщається") {
            // 1 л = 1 дм³
            n[0]
        } else if st.contains("суміжних") {
            180 - n[0]
        } else if st.starts_with("Два кути разом утворюють прямий кут")
        {
            90 - n[0]
        } else if st.starts_with("У трикутнику два кути") {
            180 - n[0] - n[1]
        } else if st.starts_with("У прямокутному трикутнику") {
            90 - n[0]
        } else {
            return None;
        },
    ))
}

fn fractions(st: &str) -> Option<Frac> {
    let n = ints_in(st);
    let f = fractions_in(st);
    if st.starts_with("Виділи цілу частину") {
        Some(int(f[0].0 / f[0].1))
    } else if st.contains("дробова частина") {
        Some(ratio(f[0].0 % f[0].1, f[0].1))
    } else if st.starts_with("Запиши мішане число") {
        // «W a/b»: числа W, a, b
        Some(int(n[0] * n[2] + n[1]))
    } else if st.starts_with("Скороти дріб") {
        let (a, b) = f[0];
        let g = gcd(a, b);
        Some(int(if st.contains("знаменник") {
            b / g
        } else {
            a / g
        }))
    } else if st.starts_with("На яке найбільше число можна скоротити")
    {
        Some(int(gcd(f[0].0, f[0].1)))
    } else if st.starts_with("Знайди число, якщо його") && !f.is_empty() {
        let (a, b) = f[0];
        Some(ratio(n[2] * b, a))
    } else if st.starts_with("Знайди число, якщо його") {
        // відсотки: n = [p, M]
        Some(ratio(n[1] * 100, n[0]))
    } else if st.starts_with("Знайди ") && !f.is_empty() {
        let (a, b) = f[0];
        Some(ratio(n[2] * a, b))
    } else if st.starts_with("Знайди ") && st.contains('%') {
        Some(ratio(n[1] * n[0], 100))
    } else if st.starts_with("Ціна товару") || st.starts_with("Довжина маршруту")
    {
        // відсотки від числа в історії: n = [число, відсотки]
        Some(ratio(n[0] * n[1], 100))
    } else if st.starts_with("Знижка становить") || st.starts_with("Туристи пройшли")
    {
        // число за відсотками в історії: n = [відсотки, частина]
        Some(ratio(n[1] * 100, n[0]))
    } else if !f.is_empty() && (st.starts_with("Пройшли") || st.starts_with("Відрізали"))
    {
        // ціле за частиною в історії: n = [a, b, частина]
        Some(ratio(n[2] * f[0].1, f[0].0))
    } else if !f.is_empty() {
        // дріб від числа в історії: n = [число, a, b]
        Some(ratio(n[0] * f[0].0, f[0].1))
    } else {
        None
    }
}

/// (початок умови, обчислення за числами з умови)
type Formula = (&'static str, fn(&[i64]) -> i64);

const STORIES: [Formula; 15] = [
    ("У шкільній їдальні було", |n| {
        n[0] - n[1]
    }),
    ("На ярмарку", |n| n[0] * n[1]),
    ("Довжина першого мосту", |n| n[0] - n[1]),
    ("Для 9 однакових кошиків", |n| {
        n[1] / n[0]
    }),
    ("В обід з'їли", |n| n[0] + n[1]),
    ("Купили 5 пакетів", |n| n[0] * n[1] + n[2]),
    ("У мішку було 40 кг", |n| n[0] - n[1] - n[2]),
    ("Купили 4 зошити", |n| n[0] * n[1] + n[2] * n[3]),
    ("На стоянці було 120 машин", |n| {
        n[0] + n[1] - n[2]
    }),
    ("Двоє учнів", |n| (n[0] + n[1]) / n[2]),
    ("Велосипедист їде зі швидкістю", |n| n[0] * n[1]),
    (
        "Назустріч одне одному вийшли двоє",
        |n| (n[0] + n[1]) * n[2],
    ),
    ("Потяг долає", |n| n[0] / n[1]),
    ("Пішохід іде зі швидкістю", |n| {
        n[1] / n[0]
    }),
    ("Літак летить", |n| n[0] * n[1]),
];

/// Розв'язує умову з уроку навички `skill`; `None`, якщо умову не розпізнано.
pub fn solve(skill: &str, st: &str) -> Option<Frac> {
    let n = ints_in(st);
    match skill {
        "N-ROUND" => {
            let p = if st.contains("тисяч") {
                1000
            } else if st.contains("сотень") {
                100
            } else {
                10
            };
            // Найближче кругле: додаємо половину кроку й відкидаємо залишок.
            Some(int((n[0] + p / 2) / p * p))
        }
        "N-DIVREM" => {
            if st.starts_with("Яка найбільша остача") {
                Some(int(n[0] - 1))
            } else if st.contains("остач") {
                Some(int(n[0] % n[1]))
            } else {
                Some(int(n[0] / n[1]))
            }
        }
        "N-DIVTEST" => {
            let (d, list) = n.split_last().unwrap();
            Some(int(list.iter().filter(|&&x| x % d == 0).count() as i64))
        }
        "N-GCD" => Some(int(gcd(n[0], n[1]))),
        "N-LCM" => Some(int(lcm(n[0], n[1]))),
        "N-POWER" => Some(power(st)),
        "EQ-ADD-SUB" | "EQ-MUL" => Some(equation(st)),
        "WORD-1" | "WORD-2" | "MOTION" => STORIES
            .iter()
            .find(|(prefix, _)| st.starts_with(prefix))
            .map(|(_, f)| int(f(&n))),
        "U-LEN" | "U-MASS" | "U-TIME" => Some(units(st)),
        "G-PERIM" | "G-AREA" | "G-VOL" | "A-ANGLE" => geometry(st),
        "F-MIXED" | "F-REDUCE" | "F-OF" | "P-PCT" => fractions(st),
        "D-MUL10" => {
            let body = st.strip_prefix("Обчисли: ").unwrap();
            let t: Vec<&str> = body.split(' ').collect();
            let a = parse_answer(t[0]).unwrap();
            let m = parse_answer(t[2]).unwrap();
            Some(match t[1] {
                "×" => a.checked_mul(m).unwrap(),
                ":" => a.checked_div(m).unwrap(),
                op => panic!("невідома дія {op}"),
            })
        }
        "AVG" => Some(ratio(n.iter().sum(), n.len() as i64)),
        _ => None,
    }
}

/// Навички розширення курсу (рішення A20), для яких працює `solve`.
pub const NEW_SKILLS: [&str; 24] = [
    "N-ROUND",
    "N-DIVREM",
    "N-DIVTEST",
    "N-GCD",
    "N-LCM",
    "N-POWER",
    "EQ-ADD-SUB",
    "EQ-MUL",
    "WORD-1",
    "WORD-2",
    "MOTION",
    "U-LEN",
    "U-MASS",
    "U-TIME",
    "G-PERIM",
    "G-AREA",
    "G-VOL",
    "A-ANGLE",
    "F-MIXED",
    "F-REDUCE",
    "F-OF",
    "P-PCT",
    "D-MUL10",
    "AVG",
];

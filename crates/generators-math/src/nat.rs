//! Натуральні числа: округлення, ділення з остачею, ознаки подільності, НСД, НСК, степінь.
//! Усі підказки й кроки аналогів без цифр, окрім записів чисел з умови (числа-константи словами).

use crate::kit::{by_level, gcd, ints, pick, smallest_prime_factor, Make, Spec};
use tutor_core::frac::Frac;
use tutor_core::generator::ScriptedLevel;
use tutor_core::rng::SeededRng;
use tutor_core::task::Task;

fn i(n: i64) -> Frac {
    Frac::int(n)
}

// ---------------------------------------------------------------------------------------------
// N-ROUND: округлення до десятків, сотень, тисяч

pub const N_ROUND: Spec = Spec {
    id: "N-ROUND",
    make: round_make,
    hint: round_hint,
    steps: round_steps,
    prompt: "Тепер ти: заміни зайві цифри нулями й запиши кругле число.",
};

fn place_word(p: i64) -> &'static str {
    match p {
        10 => "десятків",
        100 => "сотень",
        _ => "тисяч",
    }
}

fn round_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let p = *pick(rng, &[10_i64, 100, 1000]);
    let min_digits = match p {
        10 => 2,
        100 => 3,
        _ => 4,
    };
    let digits = rng.range(min_digits, 6);
    let n = rng.range(10_i64.pow(digits as u32 - 1), 10_i64.pow(digits as u32) - 1);
    let r = n % p;
    if r == 0 {
        return None;
    }
    let next_digit = r / (p / 10);
    let (low, high) = (n - r, n - r + p);
    let (answer, bug) = if next_digit >= 5 {
        (high, ("cut_off", i(low)))
    } else {
        (low, ("always_up", i(high)))
    };
    // Оракул: найближче кругле число, при рівній відстані більше.
    let oracle = if n - low < high - n { low } else { high };
    let task = Make {
        skill: "N-ROUND",
        variant,
        statement: format!("Округли число {n} до {}.", place_word(p)),
        operands: ints(&[n]),
        params: vec![p, n],
        answer: i(answer),
        steps: vec![],
        bugs: vec![bug],
    }
    .build();
    Some((task, i(oracle)))
}

fn round_hint(task: &Task, level: ScriptedLevel) -> String {
    let word = place_word(task.params[0]);
    by_level(
        level,
        format!("Округлити число {} до {word} означає замінити його найближчим круглим числом. Який розряд ти залишаєш і яка цифра стоїть одразу праворуч від нього?", task.operand_labels[0]),
        "Подивись на цифру праворуч від розряду округлення. Якщо вона менша від п'яти, розряд лишається таким самим. Якщо п'ять або більше, розряд збільшується на одиницю.".to_string(),
        "Після цього всі цифри праворуч від розряду округлення заміни нулями. Спитай себе: до якого з двох круглих чисел початкове число ближче? Якщо однаково близько до обох, беруть більше.".to_string(),
    )
}

fn round_steps(c: &Task) -> Vec<String> {
    vec![
        format!(
            "Округлюємо до {}: знаходимо цей розряд і дивимось на цифру праворуч від нього.",
            place_word(c.params[0])
        ),
        "Менша від п'яти: розряд лишається. П'ять або більше: розряд збільшується на одиницю. Цифри праворуч стають нулями.".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// N-DIVREM: ділення з остачею (остача або неповна частка)

pub const N_DIVREM: Spec = Spec {
    id: "N-DIVREM",
    make: divrem_make,
    hint: divrem_hint,
    steps: divrem_steps,
    prompt: "Тепер ти: доведи розв'язання до кінця й запиши те, про що питають.",
};

fn divrem_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let kind = rng.range(0, 1);
    let b = rng.range(3, 12);
    let q = rng.range(3, 30);
    let r = rng.range(1, b - 1);
    let a = b * q + r;
    let (statement, answer, step, bugs) = if kind == 0 {
        (
            format!("Знайди остачу від ділення {a} на {b}."),
            r,
            q,
            vec![("quotient_instead", i(q)), ("remainder_too_big", i(r + b))],
        )
    } else {
        (
            format!("Знайди неповну частку від ділення {a} на {b}."),
            q,
            r,
            vec![("remainder_instead", i(r)), ("round_up_quotient", i(q + 1))],
        )
    };
    // Оракул: віднімаємо дільник, поки можна.
    let (mut left, mut count) = (a, 0);
    while left >= b {
        left -= b;
        count += 1;
    }
    let oracle = if kind == 0 { left } else { count };
    let task = Make {
        skill: "N-DIVREM",
        variant,
        statement,
        operands: ints(&[a, b]),
        params: vec![kind, a, b],
        answer: i(answer),
        steps: vec![i(step)],
        bugs,
    }
    .build();
    Some((task, i(oracle)))
}

fn divrem_hint(task: &Task, level: ScriptedLevel) -> String {
    let (a, b) = (&task.operand_labels[0], &task.operand_labels[1]);
    by_level(
        level,
        format!("Скільки разів число {b} повністю вміщається в {a}? Згадай таблицю множення на {b}: яке кратне числа {b} найближче до {a}, але не більше за нього?"),
        format!("Знайди найбільше кратне числа {b} (добуток {b} на ціле число), яке не перевищує {a}. Число, на яке множили, це неповна частка, а те, що не вистачає до {a}, це остача."),
        "Остача завжди менша від дільника. Перевір себе: дільник, помножений на неповну частку, плюс остача має дати ділене.".to_string(),
    )
}

fn divrem_steps(c: &Task) -> Vec<String> {
    vec![
        format!(
            "Підбираємо найбільше кратне числа {} (добуток на ціле число), що не більше за {}.",
            c.operand_labels[1], c.operand_labels[0]
        ),
        "Число, на яке множили, це неповна частка, а різниця між діленим і цим кратним це остача."
            .to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// N-DIVTEST: скільки з наведених чисел діляться на 2, 3, 5, 9, 10

pub const N_DIVTEST: Spec = Spec {
    id: "N-DIVTEST",
    make: divtest_make,
    hint: divtest_hint,
    steps: divtest_steps,
    prompt: "Тепер ти: перевір так само кожне число й запиши, скільки їх підійшло.",
};

fn digit_sum(mut n: i64) -> i64 {
    let mut s = 0;
    while n > 0 {
        s += n % 10;
        n /= 10;
    }
    s
}

/// Ознака подільності за цифрами (те, чого вчить урок).
fn rule_holds(n: i64, d: i64) -> bool {
    let last = n % 10;
    match d {
        2 => last % 2 == 0,
        3 => digit_sum(n) % 3 == 0,
        5 => last == 0 || last == 5,
        9 => digit_sum(n) % 9 == 0,
        _ => last == 0,
    }
}

/// Хибне правило, яке учень може застосувати замість правильного.
fn wrong_rule_holds(n: i64, d: i64) -> bool {
    let last = n % 10;
    match d {
        2 => digit_sum(n) % 2 == 0,
        3 => last % 3 == 0,
        5 => last == 0,
        9 => digit_sum(n) % 3 == 0,
        _ => last == 0 || last == 5,
    }
}

fn wrong_rule_id(d: i64) -> &'static str {
    match d {
        2 => "digit_sum_rule",
        3 => "last_digit_rule",
        5 => "only_zero_end",
        9 => "rule_for_three",
        _ => "rule_for_five",
    }
}

fn divtest_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let d = *pick(rng, &[2_i64, 3, 5, 9, 10]);
    let multiples = rng.range(1, 5) as usize;
    let mut nums: Vec<i64> = Vec::new();
    while nums.len() < multiples {
        let m = rng.range(100 / d + 1, 999 / d) * d;
        if !nums.contains(&m) {
            nums.push(m);
        }
    }
    while nums.len() < 8 {
        let x = rng.range(100, 999);
        if x % d != 0 && !nums.contains(&x) {
            nums.push(x);
        }
    }
    for k in (1..nums.len()).rev() {
        let j = rng.range(0, k as i64) as usize;
        nums.swap(k, j);
    }
    let by_rule = nums.iter().filter(|&&n| rule_holds(n, d)).count() as i64;
    let wrong = nums.iter().filter(|&&n| wrong_rule_holds(n, d)).count() as i64;
    if wrong == by_rule {
        return None;
    }
    let oracle = nums.iter().filter(|&&n| n % d == 0).count() as i64;
    let mut operands = nums.clone();
    operands.push(d);
    let task = Make {
        skill: "N-DIVTEST",
        variant,
        statement: format!(
            "Скільки з чисел {} діляться на {d}?",
            nums.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        operands: ints(&operands),
        params: vec![d],
        answer: i(by_rule),
        steps: vec![],
        bugs: vec![(wrong_rule_id(d), i(wrong))],
    }
    .build();
    Some((task, i(oracle)))
}

fn divtest_hint(task: &Task, level: ScriptedLevel) -> String {
    let d = task.params[0];
    let label = &task.operand_labels[8];
    let rule = match d {
        2 => "Число ділиться на це, якщо закінчується парною цифрою: нулем, двійкою, четвіркою, шісткою чи вісімкою.".to_string(),
        5 => "Число ділиться на це, якщо закінчується нулем або п'ятіркою.".to_string(),
        10 => "Число ділиться на це, якщо закінчується нулем.".to_string(),
        _ => format!("Знайди суму цифр числа. Якщо сума ділиться на {label}, то й саме число ділиться на {label}."),
    };
    by_level(
        level,
        format!("Для кожного числа окремо перевір ознаку подільності на {label}. Згадай, на яку саму цифру чи суму цифр дивляться."),
        rule,
        "Переглянь усі числа по черзі й полічи, для скількох ознака виконується. Ділити в стовпчик не потрібно.".to_string(),
    )
}

fn divtest_steps(c: &Task) -> Vec<String> {
    vec![
        format!(
            "Перевіряємо кожне число за ознакою подільності на {}.",
            c.operand_labels[8]
        ),
        "Підраховуємо числа, для яких ознака виконується.".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// N-GCD і N-LCM: найбільший спільний дільник, найменше спільне кратне

pub const N_GCD: Spec = Spec {
    id: "N-GCD",
    make: gcd_make,
    hint: gcd_hint,
    steps: gcd_steps,
    prompt: "Тепер ти: знайди найбільший з усіх спільних дільників.",
};

pub const N_LCM: Spec = Spec {
    id: "N-LCM",
    make: lcm_make,
    hint: lcm_hint,
    steps: lcm_steps,
    prompt: "Тепер ти: знайди найменше з усіх спільних кратних.",
};

/// Два взаємно прості множники 2..=9, різні.
fn coprime_pair(rng: &mut SeededRng) -> Option<(i64, i64)> {
    let (m1, m2) = (rng.range(2, 9), rng.range(2, 9));
    (m1 != m2 && gcd(m1, m2) == 1).then_some((m1, m2))
}

fn gcd_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let g = *pick(rng, &[4_i64, 6, 8, 9, 10, 12, 14, 15, 16, 18, 20]);
    let (m1, m2) = coprime_pair(rng)?;
    let (a, b) = (g * m1, g * m2);
    let oracle = (1..=a.min(b))
        .rev()
        .find(|d| a % d == 0 && b % d == 0)
        .unwrap_or(1);
    let task = Make {
        skill: "N-GCD",
        variant,
        statement: format!("Знайди найбільший спільний дільник чисел {a} і {b}."),
        operands: ints(&[a, b]),
        params: vec![a, b],
        answer: i(g),
        steps: vec![],
        bugs: vec![
            ("not_greatest", i(smallest_prime_factor(g))),
            ("lcm_instead", i(g * m1 * m2)),
        ],
    }
    .build();
    Some((task, i(oracle)))
}

fn gcd_hint(task: &Task, level: ScriptedLevel) -> String {
    let (a, b) = (&task.operand_labels[0], &task.operand_labels[1]);
    by_level(
        level,
        format!("Подумай, на які числа ділиться кожне з чисел {a} і {b}. Які дільники в них спільні?"),
        "Випиши всі дільники меншого числа й залиш ті, на які ділиться й більше. Найбільший із них і є шуканий.".to_string(),
        "Можна розкласти обидва числа на прості множники й перемножити всі спільні множники. Перевір: чи діляться обидва числа на твою відповідь без остачі?".to_string(),
    )
}

fn gcd_steps(c: &Task) -> Vec<String> {
    vec![
        format!(
            "Випишемо дільники чисел {} і {}.",
            c.operand_labels[0], c.operand_labels[1]
        ),
        "Спільні дільники — ті, що є в обох списках. Нам потрібен найбільший з них.".to_string(),
    ]
}

fn lcm_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let g = rng.range(2, 9);
    let (m1, m2) = coprime_pair(rng)?;
    let (a, b) = (g * m1, g * m2);
    let lcm = g * m1 * m2;
    let (small, big) = (a.min(b), a.max(b));
    let oracle = (1..).map(|k| big * k).find(|x| x % small == 0)?;
    let task = Make {
        skill: "N-LCM",
        variant,
        statement: format!("Знайди найменше спільне кратне чисел {a} і {b}."),
        operands: ints(&[a, b]),
        params: vec![a, b],
        answer: i(lcm),
        steps: vec![],
        bugs: vec![("product", i(a * b)), ("gcd_instead", i(g))],
    }
    .build();
    Some((task, i(oracle)))
}

fn lcm_hint(task: &Task, level: ScriptedLevel) -> String {
    let (a, b) = (&task.operand_labels[0], &task.operand_labels[1]);
    by_level(
        level,
        format!("Кратні числа утворюються множенням на 1, 2, 3 і так далі. Випиши по кілька перших кратних чисел {a} і {b}."),
        "Шукай найменше число, яке є в обох списках кратних. Починай зі списку більшого числа: так швидше.".to_string(),
        "Добуток двох чисел завжди є їхнім спільним кратним, але не завжди найменшим. Перевір, чи є менше число, яке ділиться на обидва.".to_string(),
    )
}

fn lcm_steps(c: &Task) -> Vec<String> {
    vec![
        format!(
            "Випишемо перші кратні чисел {} і {}.",
            c.operand_labels[0], c.operand_labels[1]
        ),
        "Спільне кратне — число, яке є в обох списках. Нам потрібне найменше з них.".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// N-POWER: степінь (квадрат, куб, вищі степені малих чисел), іноді з додаванням чи відніманням

pub const N_POWER: Spec = Spec {
    id: "N-POWER",
    make: power_make,
    hint: power_hint,
    steps: power_steps,
    prompt: "Тепер ти: доведи обчислення до кінця й запиши результат.",
};

const SUPERSCRIPTS: [char; 4] = ['²', '³', '⁴', '⁵'];

fn power_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let e = *pick(rng, &[2_i64, 2, 2, 3, 3, 4, 5]);
    let base = match e {
        2 => rng.range(2, 20),
        3 => rng.range(2, 10),
        4 => rng.range(2, 6),
        _ => rng.range(2, 4),
    };
    let power = base.pow(e as u32);
    let sup = SUPERSCRIPTS[(e - 2) as usize];
    let extra = rng.range(0, 2);
    let c = rng.range(2, 40);
    let (statement, operands, answer, wrong, steps) = match extra {
        0 => (
            format!("Обчисли: {base}{sup}"),
            vec![base],
            power,
            base * e,
            vec![],
        ),
        1 => (
            format!("Обчисли: {base}{sup} + {c}"),
            vec![base, c],
            power + c,
            base * e + c,
            vec![i(power)],
        ),
        _ => {
            if c >= power || base * e <= c {
                return None;
            }
            (
                format!("Обчисли: {base}{sup} - {c}"),
                vec![base, c],
                power - c,
                base * e - c,
                vec![i(power)],
            )
        }
    };
    // Оракул: послідовне множення.
    let product = (0..e).fold(1, |acc, _| acc * base);
    let oracle = match extra {
        0 => product,
        1 => product + c,
        _ => product - c,
    };
    let task = Make {
        skill: "N-POWER",
        variant,
        statement,
        operands: ints(&operands),
        params: vec![base, e, extra, c],
        answer: i(answer),
        steps,
        bugs: vec![("multiply_by_exponent", i(wrong))],
    }
    .build();
    Some((task, i(oracle)))
}

fn power_hint(task: &Task, level: ScriptedLevel) -> String {
    let (e, extra) = (task.params[1], task.params[2]);
    let name = match e {
        2 => "Квадрат числа це число, помножене на себе.",
        3 => "Куб числа це число, помножене на себе, а результат ще раз на це саме число.",
        _ => "Степінь показує, скільки однакових множників треба перемножити.",
    };
    let tail = if extra == 0 {
        "Не плутай степінь з множенням на показник.".to_string()
    } else {
        "Спершу знайди степінь, потім додай чи відніми. Не плутай степінь з множенням на показник."
            .to_string()
    };
    by_level(
        level,
        "Степінь показує, скільки разів число-основу беремо множником. Яке число тут основа, а що означає маленьке число над ним?".to_string(),
        name.to_string(),
        tail,
    )
}

fn power_steps(c: &Task) -> Vec<String> {
    let mut v = vec![
        "Запишемо основу як добуток однакових множників: їх стільки, скільки показує показник степеня.".to_string(),
        "Перемножимо ці множники послідовно.".to_string(),
    ];
    if c.params[2] != 0 {
        v.push("Додавання чи віднімання виконуємо вже після того, як знайшли степінь.".to_string());
    }
    v
}

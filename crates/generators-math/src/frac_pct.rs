//! Дроби, відсотки, десяткові дроби і дані: неправильний дріб і мішане число, скорочення дробу,
//! частина від числа, відсотки від числа, множення й ділення десяткових на 10, 100, 1000, середнє арифметичне.
//!
//! Усе, що не вміщається у формат відповіді «ціле, дріб або десяткове», обходимо запитанням з такою відповіддю:
//! наприклад, про скорочений дріб питаємо його знаменник чи чисельник (бо будь-який еквівалентний запис
//! дробу `parse_answer` вважає тим самим числом).

use crate::kit::{by_level, fr, gcd, ints, pick, smallest_prime_factor, Make, Spec};
use tutor_core::frac::{parse_answer, Frac};
use tutor_core::generator::ScriptedLevel;
use tutor_core::rng::SeededRng;
use tutor_core::task::Task;

fn i(n: i64) -> Frac {
    Frac::int(n)
}

// ---------------------------------------------------------------------------------------------
// F-MIXED: неправильний дріб і мішане число

pub const F_MIXED: Spec = Spec {
    id: "F-MIXED",
    make: mixed_make,
    hint: mixed_hint,
    steps: mixed_steps,
    prompt: "Тепер ти: доведи перетворення до кінця й запиши те, про що питають.",
};

fn mixed_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let kind = rng.range(0, 2);
    let d = rng.range(2, 12);
    let q = rng.range(1, 12);
    let r = rng.range(1, d - 1);
    if gcd(r, d) != 1 {
        return None;
    }
    let n = q * d + r;
    let improper = (fr(n, d), format!("{n}/{d}"));
    // Оракул: віднімаємо знаменник, поки можна.
    let (mut left, mut whole) = (n, 0);
    while left >= d {
        left -= d;
        whole += 1;
    }
    let (statement, operands, answer, oracle, steps, bugs) = match kind {
        0 => (
            format!("Виділи цілу частину неправильного дробу {n}/{d}."),
            vec![improper],
            i(q),
            i(whole),
            vec![i(r)],
            vec![("remainder_as_whole", i(r)), ("round_up_whole", i(q + 1))],
        ),
        1 => (
            format!(
                "Запиши неправильний дріб {n}/{d} мішаним числом. Яка в нього дробова частина?"
            ),
            vec![improper],
            fr(r, d),
            fr(left, d),
            vec![i(q)],
            vec![
                ("whole_over_denominator", fr(q, d)),
                ("remainder_only", i(r)),
            ],
        ),
        _ => {
            let side_by_side: i64 = format!("{q}{r}").parse().ok()?;
            (
                format!(
                    "Запиши мішане число {q} {r}/{d} неправильним дробом. Чому дорівнює чисельник?"
                ),
                vec![(i(q), q.to_string()), (fr(r, d), format!("{r}/{d}"))],
                i(n),
                i((0..q).fold(r, |s, _| s + d)),
                vec![i(q * d)],
                vec![
                    ("forgot_to_add", i(q * d)),
                    ("digits_side_by_side", i(side_by_side)),
                ],
            )
        }
    };
    let task = Make {
        skill: "F-MIXED",
        variant,
        statement,
        operands,
        params: vec![kind, n, d],
        answer,
        steps,
        bugs,
    }
    .build();
    Some((task, oracle))
}

fn mixed_hint(task: &Task, level: ScriptedLevel) -> String {
    let l = &task.operand_labels;
    match task.params[0] {
        0 => by_level(
            level,
            format!("Дріб {} неправильний: чисельник не менший за знаменник. Скільки цілих одиниць у ньому міститься?", l[0]),
            "Поділи чисельник на знаменник з остачею: неповна частка це ціла частина, а остача це чисельник дробової частини.".to_string(),
            "Перевір: ціла частина, помножена на знаменник, плюс остача мають дати початковий чисельник.".to_string(),
        ),
        1 => by_level(
            level,
            format!("Дріб {} неправильний: чисельник не менший за знаменник. З нього можна виділити цілі одиниці й дробову частину.", l[0]),
            "Поділи чисельник на знаменник з остачею. У дробової частини чисельник це остача, а знаменник лишається тим самим.".to_string(),
            "Дробова частина мішаного числа завжди менша за одиницю: її чисельник менший за знаменник.".to_string(),
        ),
        _ => by_level(
            level,
            format!("Мішане число {} {} складається з цілої частини й дробу. Скільки часток міститься в кожній цілій одиниці, якщо знаменник показує розмір часток?", l[0], l[1]),
            "У кожній цілій одиниці стільки часток, яким є знаменник. Помнож цілу частину на знаменник і додай чисельник дробу.".to_string(),
            "Знаменник неправильного дробу залишається тим самим, змінюється лише чисельник.".to_string(),
        ),
    }
}

fn mixed_steps(c: &Task) -> Vec<String> {
    let s = match c.params[0] {
        0 | 1 => "Ділимо чисельник на знаменник з остачею: неповна частка це ціла частина, остача це чисельник дробової частини.",
        _ => "Цілу частину множимо на знаменник і додаємо чисельник дробу: це новий чисельник, а знаменник той самий.",
    };
    vec![
        s.to_string(),
        "Записуємо результат і перевіряємо зворотною дією.".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// F-REDUCE: скорочення дробу

pub const F_REDUCE: Spec = Spec {
    id: "F-REDUCE",
    make: reduce_make,
    hint: reduce_hint,
    steps: reduce_steps,
    prompt: "Тепер ти: скороти дріб до нескоротного й запиши те, про що питають.",
};

fn reduce_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let kind = rng.range(0, 2);
    let g = *pick(rng, &[4_i64, 6, 8, 9, 10, 12, 14, 15, 16, 18, 20]);
    let nd = rng.range(3, 12);
    let nn = rng.range(1, nd - 1);
    if gcd(nn, nd) != 1 {
        return None;
    }
    let (a, b) = (g * nn, g * nd);
    let p = smallest_prime_factor(g);
    // Оракул: нормалізація дробу `Frac` скорочує його сама.
    let reduced = Frac::new(a, b)?;
    let operand = vec![(reduced, format!("{a}/{b}"))];
    let (statement, answer, oracle, bugs) = match kind {
        0 => (
            format!("Скороти дріб {a}/{b} повністю. Який знаменник має скорочений дріб?"),
            nd,
            reduced.den(),
            vec![("partial_reduce", i(b / p)), ("asked_other_part", i(nn))],
        ),
        1 => (
            format!("Скороти дріб {a}/{b} повністю. Який чисельник має скорочений дріб?"),
            nn,
            reduced.num(),
            vec![("partial_reduce", i(a / p)), ("asked_other_part", i(nd))],
        ),
        _ => (
            format!("На яке найбільше число можна скоротити дріб {a}/{b}?"),
            g,
            (2..=a).rev().find(|d| a % d == 0 && b % d == 0)?,
            vec![("partial_reduce", i(p))],
        ),
    };
    let task = Make {
        skill: "F-REDUCE",
        variant,
        statement,
        operands: operand,
        params: vec![kind],
        answer: i(answer),
        steps: vec![],
        bugs,
    }
    .build();
    Some((task, i(oracle)))
}

fn reduce_hint(task: &Task, level: ScriptedLevel) -> String {
    let l = &task.operand_labels[0];
    let last = if task.params[0] == 2 {
        "Шукане число це найбільше число, на яке діляться і чисельник, і знаменник без остачі."
    } else {
        "Зверни увагу, про яку частину дробу питають: про чисельник чи про знаменник. Після повного скорочення спільних дільників, крім одиниці, вже немає."
    };
    by_level(
        level,
        format!("Скоротити дріб {l} означає поділити і чисельник, і знаменник на одне й те саме число, більше за одиницю. Які числа ділять і чисельник, і знаменник?"),
        "Щоб скоротити повністю, ділять на найбільший спільний дільник. Можна скорочувати по кроках, поки у чисельника й знаменника є спільні дільники.".to_string(),
        last.to_string(),
    )
}

fn reduce_steps(c: &Task) -> Vec<String> {
    vec![
        format!(
            "Шукаємо спільні дільники чисельника й знаменника дробу {}.",
            c.operand_labels[0]
        ),
        "Ділимо чисельник і знаменник на найбільший спільний дільник; після цього дріб нескоротний.".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// F-OF: частина від числа і число за його частиною

pub const F_OF: Spec = Spec {
    id: "F-OF",
    make: of_make,
    hint: of_hint,
    steps: of_steps,
    prompt: "Тепер ти: доведи обчислення до кінця й запиши число.",
};

const PARTS: [(i64, i64); 19] = [
    (1, 2),
    (1, 3),
    (2, 3),
    (1, 4),
    (3, 4),
    (1, 5),
    (2, 5),
    (3, 5),
    (4, 5),
    (1, 6),
    (5, 6),
    (1, 8),
    (3, 8),
    (5, 8),
    (7, 8),
    (1, 10),
    (3, 10),
    (7, 10),
    (9, 10),
];

fn of_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let (p, q) = *pick(rng, &PARTS);
    let m = rng.range(2, 20);
    let kind = i64::from(rng.range(0, 2) == 2);
    let v = rng.range(0, 2);
    let part = (fr(p, q), format!("{p}/{q}"));
    if kind == 0 {
        let whole = q * m;
        let statement = match v {
            0 => format!("Знайди {p}/{q} від числа {whole}."),
            1 => format!("У пляшці було {whole} мл соку. Випили {p}/{q} соку. Скільки мілілітрів соку випили?"),
            _ => format!("У мішку було {whole} кг борошна. Витратили {p}/{q} борошна. Скільки кілограмів борошна витратили?"),
        };
        // Оракул: ділимо на q рівних груп (підбір розміру групи) і беремо p груп.
        let group = (1..=whole).find(|g| g * q == whole)?;
        let oracle = (0..p).map(|_| group).sum::<i64>();
        let mut bugs = vec![("multiply_by_denominator", i(whole * q))];
        if p > 1 {
            bugs.push(("forgot_numerator", i(m)));
            bugs.push(("divide_by_numerator", fr(whole, p)));
        }
        let task = Make {
            skill: "F-OF",
            variant,
            statement,
            operands: vec![part, (i(whole), whole.to_string())],
            params: vec![kind, p, q],
            answer: i(m * p),
            steps: vec![i(m)],
            bugs,
        }
        .build();
        Some((task, i(oracle)))
    } else {
        let given = p * m;
        let statement = match v {
            0 => format!("Знайди число, якщо його {p}/{q} становить {given}."),
            1 => format!("Пройшли {p}/{q} шляху, і це {given} км. Скільки кілометрів становить увесь шлях?"),
            _ => format!("Відрізали {p}/{q} стрічки, і це {given} см. Яка довжина всієї стрічки в сантиметрах?"),
        };
        let oracle = (1..).find(|w| w % q == 0 && (w / q) * p == given)?;
        let mut bugs = vec![("fraction_of_given", fr(given * p, q))];
        if p > 1 {
            bugs.push(("multiply_by_denominator", i(given * q)));
        }
        let task = Make {
            skill: "F-OF",
            variant,
            statement,
            operands: vec![part, (i(given), given.to_string())],
            params: vec![kind, p, q],
            answer: i(m * q),
            steps: vec![i(m)],
            bugs,
        }
        .build();
        Some((task, i(oracle)))
    }
}

fn of_hint(task: &Task, level: ScriptedLevel) -> String {
    let f = &task.operand_labels[0];
    if task.params[0] == 0 {
        by_level(
            level,
            format!("Дріб {f} показує: ціле ділять на рівні частини й беруть кілька таких частин. На скільки рівних частин ділимо і скільки частин беремо?"),
            "Спочатку знайди одну частину: поділи ціле на знаменник дробу. Потім помнож цю частину на чисельник.".to_string(),
            "Перевір: частина від числа, якщо дріб правильний, менша за саме число.".to_string(),
        )
    } else {
        by_level(
            level,
            format!("Відомо, скільки становить частина {f} від невідомого цілого. Скільки таких часток складають ціле?"),
            "Спочатку знайди одну частку: поділи відоме число на чисельник дробу. Потім помнож на знаменник, щоб дізнатися, скільки становить ціле.".to_string(),
            "Перевір: ціле має бути більшим за свою частину, якщо дріб правильний.".to_string(),
        )
    }
}

fn of_steps(c: &Task) -> Vec<String> {
    let s = if c.params[0] == 0 {
        "Ділимо ціле на знаменник дробу, щоб знайти одну частину, і множимо її на чисельник."
    } else {
        "Ділимо відому частину на чисельник дробу, щоб знайти одну частку, і множимо її на знаменник."
    };
    vec![s.to_string(), format!("Тут дріб {}.", c.operand_labels[0])]
}

// ---------------------------------------------------------------------------------------------
// P-PCT: відсотки від числа і число за його відсотками

pub const P_PCT: Spec = Spec {
    id: "P-PCT",
    make: pct_make,
    hint: pct_hint,
    steps: pct_steps,
    prompt: "Тепер ти: доведи обчислення до кінця й запиши число.",
};

const PERCENTS: [i64; 8] = [5, 10, 20, 25, 40, 50, 60, 75];

fn pct_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let p = *pick(rng, &PERCENTS);
    // Найменше число, від якого p% є цілим.
    let unit = 100 / gcd(p, 100);
    let kind = i64::from(rng.range(0, 2) == 2);
    let v = rng.range(0, 2);
    let base = unit * rng.range(1, (800 / unit).max(2));
    if kind == 0 {
        let statement = match v {
            0 => format!("Знайди {p}% від числа {base}."),
            1 => format!("Ціна товару {base} грн. Знижка становить {p}% від ціни. Скільки гривень становить знижка?"),
            _ => format!("Довжина маршруту {base} км. Туристи вже пройшли {p}% маршруту. Скільки кілометрів вони пройшли?"),
        };
        let answer = base * p / 100;
        // Оракул: підбір числа k, для якого k сотих частин дорівнюють p частинам по base.
        let oracle = (1..=base).find(|k| k * 100 == base * p)?;
        let task = Make {
            skill: "P-PCT",
            variant,
            statement,
            operands: ints(&[p, base]),
            params: vec![kind, p],
            answer: i(answer),
            steps: vec![fr(base, 100)],
            bugs: vec![
                ("forgot_hundred", i(base * p)),
                ("percent_as_denominator", fr(base, p)),
            ],
        }
        .build();
        Some((task, i(oracle)))
    } else {
        let given = base * p / 100;
        if given < 2 {
            return None;
        }
        let statement = match v {
            0 => format!("Знайди число, якщо його {p}% становлять {given}."),
            1 => format!("Знижка становить {p}% від ціни, і це {given} грн. Скільки гривень коштував товар без знижки?"),
            _ => format!("Туристи пройшли {p}% маршруту, і це {given} км. Яка довжина всього маршруту в кілометрах?"),
        };
        let oracle = (1..).find(|w| w * p == given * 100)?;
        let task = Make {
            skill: "P-PCT",
            variant,
            statement,
            operands: ints(&[p, given]),
            params: vec![kind, p],
            answer: i(base),
            steps: vec![fr(given, p)],
            bugs: vec![
                ("percent_of_given", fr(given * p, 100)),
                ("multiply_by_percent", i(given * p)),
            ],
        }
        .build();
        Some((task, i(oracle)))
    }
}

fn pct_hint(task: &Task, level: ScriptedLevel) -> String {
    let p = &task.operand_labels[0];
    if task.params[0] == 0 {
        by_level(
            level,
            format!("Один відсоток це одна сота частина числа. Відсотків тут {p}: це {p} таких сотих."),
            "Відсотки можна записати дробом зі знаменником сто. Цей дріб часто скорочується: половина, чверть, десята частина.".to_string(),
            "Знайди частину числа цим дробом: поділи на знаменник, помнож на чисельник. Перевір: відсотків менше ніж сто, тож результат менший за саме число.".to_string(),
        )
    } else {
        by_level(
            level,
            format!("Відомо, скільки становлять {p} відсотків невідомого числа. Це число становить сто відсотків."),
            "Спочатку знайди один відсоток: поділи відоме число на кількість відсотків. Потім помнож на сто, щоб отримати все число.".to_string(),
            "Перевір: якщо відсотків менше ніж сто, то ціле більше за свою частину.".to_string(),
        )
    }
}

fn pct_steps(c: &Task) -> Vec<String> {
    let s = if c.params[0] == 0 {
        "Записуємо відсотки дробом зі знаменником сто, скорочуємо його й знаходимо частину числа."
    } else {
        "Знаходимо один відсоток, ділячи відоме число на кількість відсотків, а потім множимо на сто."
    };
    vec![
        s.to_string(),
        "Підставляємо числа з умови й обчислюємо.".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// D-MUL10: множення й ділення десяткових на 10, 100, 1000

pub const D_MUL10: Spec = Spec {
    id: "D-MUL10",
    make: dmul_make,
    hint: dmul_hint,
    steps: dmul_steps,
    prompt: "Тепер ти: перенеси кому й запиши результат.",
};

/// Результат переносу коми в записі числа `u / 10^d` на `k` розрядів ліворуч (від'ємне `k`: праворуч у діленні),
/// обчислений над рядком цифр, а не арифметикою.
fn shift_point(u: i64, d: i64, k: i64) -> Option<Frac> {
    let s = u.to_string();
    let len = s.len() as i64;
    let pos = len - d + k;
    let text = if pos <= 0 {
        format!("0.{}{}", "0".repeat((-pos) as usize), s)
    } else if pos >= len {
        format!("{}{}", s, "0".repeat((pos - len) as usize))
    } else {
        format!("{}.{}", &s[..pos as usize], &s[pos as usize..])
    };
    parse_answer(&text)
}

fn dmul_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let multiply = rng.range(0, 1) == 0;
    let k = rng.range(1, 3);
    let m = 10_i64.pow(k as u32);
    let d = if multiply {
        rng.range(1, 3)
    } else {
        rng.range(0, 3)
    };
    let u = if d == 0 {
        rng.range(2, 999)
    } else {
        rng.range(11, 9999)
    };
    if d > 0 && u % 10 == 0 {
        return None;
    }
    let x = fr(u, 10_i64.pow(d as u32));
    let xs = crate::kit::dec_str(u, d as u32);
    let mf = i(m);
    let (statement, answer, oracle, bugs) = if multiply {
        let mut bugs = vec![("wrong_direction", x.checked_div(mf)?)];
        if k >= 2 {
            bugs.push(("one_place_short", x.checked_mul(i(m / 10))?));
        }
        (
            format!("Обчисли: {xs} × {m}"),
            x.checked_mul(mf)?,
            shift_point(u, d, k)?,
            bugs,
        )
    } else {
        let mut bugs = vec![("wrong_direction", x.checked_mul(mf)?)];
        if k >= 2 {
            bugs.push(("one_place_short", x.checked_div(i(m / 10))?));
        }
        (
            format!("Обчисли: {xs} : {m}"),
            x.checked_div(mf)?,
            shift_point(u, d, -k)?,
            bugs,
        )
    };
    let task = Make {
        skill: "D-MUL10",
        variant,
        statement,
        operands: vec![(x, xs), (mf, m.to_string())],
        params: vec![i64::from(multiply), k],
        answer,
        steps: vec![],
        bugs,
    }
    .build();
    Some((task, oracle))
}

fn dmul_hint(task: &Task, level: ScriptedLevel) -> String {
    let (r2, r3) = if task.params[0] == 1 {
        (
            "Кому переносимо праворуч на стільки цифр, скільки нулів у другому числі. Якщо цифр не вистачає, дописуємо нулі.",
            "Перевір на око: після множення на 10, 100 чи 1000 число стає більшим.",
        )
    } else {
        (
            "Кому переносимо ліворуч на стільки цифр, скільки нулів у дільнику. Якщо цифр не вистачає, дописуємо нулі ліворуч.",
            "Перевір на око: після ділення на 10, 100 чи 1000 число стає меншим.",
        )
    };
    by_level(
        level,
        "Множення на 10, 100, 1000 і ділення на них зводяться до перенесення коми. Скільки нулів у другому числі? У який бік переносимо кому?".to_string(),
        r2.to_string(),
        r3.to_string(),
    )
}

fn dmul_steps(c: &Task) -> Vec<String> {
    let s = if c.params[0] == 1 {
        "Рахуємо нулі у другому числі й переносимо кому праворуч на стільки цифр."
    } else {
        "Рахуємо нулі у дільнику й переносимо кому ліворуч на стільки цифр."
    };
    vec![
        s.to_string(),
        "Якщо цифр не вистачає, дописуємо нулі.".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// AVG: середнє арифметичне

pub const AVG: Spec = Spec {
    id: "AVG",
    make: avg_make,
    hint: avg_hint,
    steps: avg_steps,
    prompt: "Тепер ти: знайди суму, поділи її на кількість і запиши число.",
};

fn avg_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let n = rng.range(3, 5) as usize;
    let mean = rng.range(8, 60);
    let mut devs: Vec<i64> = (0..n - 1).map(|_| rng.range(-6, 6)).collect();
    let last = -devs.iter().sum::<i64>();
    if last.abs() > 12 {
        return None;
    }
    devs.push(last);
    let nums: Vec<i64> = devs.iter().map(|d| mean + d).collect();
    let mut sorted = nums.clone();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != n {
        return None;
    }
    let v = rng.range(0, 4);
    // Правдоподібні величини: додатні; відро води, кавун і температура повітря не бувають завеликими.
    let max = nums.iter().copied().max().unwrap_or(0);
    let limit = match v {
        2 => 20,
        3 => 15,
        4 => 40,
        _ => i64::MAX,
    };
    if nums.iter().any(|x| *x <= 0) || max > limit {
        return None;
    }
    let with_unit = |unit: &str| {
        nums.iter()
            .map(|x| format!("{x} {unit}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let plain = format!(
        "{} і {}",
        nums[..n - 1]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", "),
        nums[n - 1]
    );
    let statement = match v {
        0 => format!("Знайди середнє арифметичне чисел {plain}."),
        1 => format!("Протягом кількох днів вимірювали довжину тіні дерева: {}. Знайди середню довжину тіні в сантиметрах.", with_unit("см")),
        2 => format!("У кількох відрах було води: {}. Скільки літрів води в середньому було в одному відрі?", with_unit("л")),
        3 => format!("Маса кількох кавунів: {}. Знайди середню масу одного кавуна в кілограмах.", with_unit("кг")),
        _ => format!("Температура повітря протягом кількох днів: {}. Знайди середню температуру в градусах Цельсія.", with_unit("°C")),
    };
    let sum: i64 = nums.iter().sum();
    // Оракул: число, відносно якого відхилення чисел від нього в сумі дають нуль.
    let oracle = (1..).find(|m| nums.iter().map(|x| x - m).sum::<i64>() == 0)?;
    let task = Make {
        skill: "AVG",
        variant,
        statement,
        operands: ints(&nums),
        params: vec![n as i64, v],
        answer: i(sum / n as i64),
        steps: vec![i(sum)],
        bugs: vec![
            ("forgot_divide", i(sum)),
            ("divide_by_count_minus_one", fr(sum, n as i64 - 1)),
        ],
    }
    .build();
    Some((task, i(oracle)))
}

fn avg_hint(_task: &Task, level: ScriptedLevel) -> String {
    by_level(
        level,
        "Середнє арифметичне показує, яким було б кожне число, якби всі вони були однакові. Як знайти загальну суму всіх чисел?".to_string(),
        "Спочатку знайди суму всіх чисел. Потім поділи її на кількість доданків.".to_string(),
        "Полічи, скільки всього чисел, щоб знати, на що ділити. Перевір: середнє лежить між найменшим і найбільшим із цих чисел.".to_string(),
    )
}

fn avg_steps(_: &Task) -> Vec<String> {
    vec![
        "Спершу додаємо всі числа, щоб знайти загальну суму.".to_string(),
        "Потім ділимо суму на те, скільки було чисел.".to_string(),
    ]
}

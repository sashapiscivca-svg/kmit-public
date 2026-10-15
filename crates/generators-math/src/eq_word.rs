//! Рівняння, текстові задачі в одну й у дві дії, задачі на рух.
//!
//! У текстових задачах числівник завжди стоїть у називному чи знахідному відмінку при іменнику
//! чоловічого чи середнього роду (або після «є», «було»), тож форма іменника однозначна (`kit::pl`).
//! Сюжети нейтральні: предмети, місця, одиниці виміру; без гендерних ролей.

use crate::kit::{by_level, fr, ints, pick, pl, Forms, Make, Spec};
use tutor_core::frac::Frac;
use tutor_core::generator::ScriptedLevel;
use tutor_core::rng::SeededRng;
use tutor_core::task::Task;

fn i(n: i64) -> Frac {
    Frac::int(n)
}

// ---------------------------------------------------------------------------------------------
// EQ-ADD-SUB і EQ-MUL: прості лінійні рівняння

pub const EQ_ADD_SUB: Spec = Spec {
    id: "EQ-ADD-SUB",
    make: eq_add_make,
    hint: eq_add_hint,
    steps: eq_steps,
    prompt: "Тепер ти: знайди невідоме число й запиши його.",
};

pub const EQ_MUL: Spec = Spec {
    id: "EQ-MUL",
    make: eq_mul_make,
    hint: eq_mul_hint,
    steps: eq_steps,
    prompt: "Тепер ти: знайди невідоме число й запиши його.",
};

fn eq_add_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let kind = rng.range(0, 3);
    let (a, b, x, statement, bug) = match kind {
        0 | 1 => {
            let (x, a) = (rng.range(5, 500), rng.range(5, 400));
            let b = x + a;
            let st = if kind == 0 {
                format!("Розв'яжи рівняння: x + {a} = {b}")
            } else {
                format!("Розв'яжи рівняння: {a} + x = {b}")
            };
            (a, b, x, st, ("add_instead", i(a + b)))
        }
        2 => {
            let (a, b) = (rng.range(5, 400), rng.range(5, 400));
            if a == b {
                return None;
            }
            let x = a + b;
            (
                a,
                b,
                x,
                format!("Розв'яжи рівняння: x - {a} = {b}"),
                ("subtract_instead", i((b - a).abs())),
            )
        }
        _ => {
            let (x, b) = (rng.range(5, 400), rng.range(5, 400));
            let a = x + b;
            (
                a,
                b,
                x,
                format!("Розв'яжи рівняння: {a} - x = {b}"),
                ("add_instead", i(a + b)),
            )
        }
    };
    // Оракул: підбір значення, яке перетворює рівність на правильну.
    let oracle = (0..=2000)
        .find(|&t| match kind {
            0 | 1 => t + a == b,
            2 => t - a == b,
            _ => a - t == b,
        })
        .unwrap_or(-1);
    let task = Make {
        skill: "EQ-ADD-SUB",
        variant,
        statement,
        operands: ints(&[a, b]),
        params: vec![kind, a, b],
        answer: i(x),
        steps: vec![],
        bugs: vec![bug],
    }
    .build();
    Some((task, i(oracle)))
}

fn eq_add_hint(task: &Task, level: ScriptedLevel) -> String {
    let (a, b) = (&task.operand_labels[0], &task.operand_labels[1]);
    let rule = match task.params[0] {
        0 | 1 => "Невідомий доданок знаходять так: від суми віднімають відомий доданок.",
        2 => "Невідоме зменшуване знаходять так: до різниці додають від'ємник.",
        _ => "Невідомий від'ємник знаходять так: від зменшуваного віднімають різницю.",
    };
    by_level(
        level,
        format!("Невідоме число тут позначено буквою x. Яка роль x в рівності: доданок, зменшуване чи від'ємник? Які числа, {a} і {b}, відомі?"),
        rule.to_string(),
        "Підстав знайдене число замість x і перевір, чи ліва частина дорівнює правій.".to_string(),
    )
}

fn eq_steps(_: &Task) -> Vec<String> {
    vec![
        "З'ясовуємо, яку роль у рівності відіграє невідоме число: доданок, зменшуване, від'ємник, множник, ділене чи дільник.".to_string(),
        "Застосовуємо відповідне правило знаходження невідомого, а потім перевіряємо підстановкою.".to_string(),
    ]
}

fn eq_mul_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let kind = rng.range(0, 3);
    let (a, b, x, statement, bugs) = match kind {
        0 | 1 => {
            let (a, x) = (rng.range(2, 15), rng.range(2, 40));
            let b = a * x;
            let st = if kind == 0 {
                format!("Розв'яжи рівняння: {a} × x = {b}")
            } else {
                format!("Розв'яжи рівняння: x × {a} = {b}")
            };
            (a, b, x, st, vec![("multiply_instead", i(a * b))])
        }
        2 => {
            let (a, b) = (rng.range(2, 12), rng.range(2, 30));
            let x = a * b;
            (
                a,
                b,
                x,
                format!("Розв'яжи рівняння: x : {a} = {b}"),
                vec![("divide_instead", fr(b, a))],
            )
        }
        _ => {
            let (b, x) = (rng.range(2, 12), rng.range(2, 30));
            let a = b * x;
            (
                a,
                b,
                x,
                format!("Розв'яжи рівняння: {a} : x = {b}"),
                vec![
                    ("multiply_instead", i(a * b)),
                    ("divisor_swapped", fr(b, a)),
                ],
            )
        }
    };
    let oracle = (1..=5000)
        .find(|&t| match kind {
            0 | 1 => a * t == b,
            2 => t % a == 0 && t / a == b,
            _ => a % t == 0 && a / t == b,
        })
        .unwrap_or(-1);
    let task = Make {
        skill: "EQ-MUL",
        variant,
        statement,
        operands: ints(&[a, b]),
        params: vec![kind, a, b],
        answer: i(x),
        steps: vec![],
        bugs,
    }
    .build();
    Some((task, i(oracle)))
}

fn eq_mul_hint(task: &Task, level: ScriptedLevel) -> String {
    let (a, b) = (&task.operand_labels[0], &task.operand_labels[1]);
    let rule = match task.params[0] {
        0 | 1 => "Невідомий множник знаходять так: добуток ділять на відомий множник.",
        2 => "Невідоме ділене знаходять так: частку множать на дільник.",
        _ => "Невідомий дільник знаходять так: ділене ділять на частку.",
    };
    by_level(
        level,
        format!("Невідоме число тут позначено буквою x. Яка роль x в рівності: множник, ділене чи дільник? Які числа, {a} і {b}, відомі?"),
        rule.to_string(),
        "Підстав знайдене число замість x і перевір, чи ліва частина дорівнює правій.".to_string(),
    )
}

// ---------------------------------------------------------------------------------------------
// Іменники для текстових задач

const BOOK: Forms = ["книжка", "книжки", "книжок"]; // жін.: лише після «є», «було»
const NOTE: Forms = ["зошит", "зошити", "зошитів"];
const APPLE: Forms = ["яблуко", "яблука", "яблук"];
const POSTCARD: Forms = ["листівка", "листівки", "листівок"]; // жін.
const CHAIR: Forms = ["стілець", "стільці", "стільців"];
const TREE: Forms = ["дерево", "дерева", "дерев"];
const FLAG: Forms = ["прапорець", "прапорці", "прапорців"];
const PENCIL: Forms = ["олівець", "олівці", "олівців"];
const BALL: Forms = ["м'яч", "м'ячі", "м'ячів"];
const BIKE: Forms = ["велосипед", "велосипеди", "велосипедів"];
const SHEET: Forms = ["аркуш", "аркуші", "аркушів"];
const BOX: Forms = ["коробка", "коробки", "коробок"]; // жін.
const ROW: Forms = ["ряд", "ряди", "рядів"];
const BAG: Forms = ["пакет", "пакети", "пакетів"];
const TICKET: Forms = ["квиток", "квитки", "квитків"];
const DAY: Forms = ["день", "дні", "днів"];
const PEAR: Forms = ["груша", "груші", "груш"]; // жін.
const BASKET: Forms = ["кошик", "кошики", "кошиків"];
const TANK: Forms = ["бак", "баки", "баків"];
const CRATE: Forms = ["ящик", "ящики", "ящиків"];
const SHELF: Forms = ["полиця", "полиці", "полиць"]; // жін.
const TABLE: Forms = ["стіл", "столи", "столів"];
const FLASK: Forms = ["бідон", "бідони", "бідонів"];

type Tpl = fn(i64, i64) -> String;

// ---------------------------------------------------------------------------------------------
// WORD-1: текстові задачі в одну дію

pub const WORD_1: Spec = Spec {
    id: "WORD-1",
    make: word1_make,
    hint: word1_hint,
    steps: word1_steps,
    prompt: "Тепер ти: вибери дію, виконай її й запиши відповідь.",
};

const ADD_T: [Tpl; 8] = [
    |a, b| {
        format!("На першій полиці шкільної бібліотеки є {}, а на другій є {}. Скільки книжок на двох полицях разом?", pl(a, &BOOK), pl(b, &BOOK))
    },
    |a, b| {
        format!("У понеділок у магазині продали {}, а у вівторок продали {}. Скільки зошитів продали за ці два дні?", pl(a, &NOTE), pl(b, &NOTE))
    },
    |a, b| {
        format!("З однієї яблуні в саду зібрали {}, а з іншої зібрали {}. Скільки яблук зібрали з двох яблунь?", pl(a, &APPLE), pl(b, &APPLE))
    },
    |a, b| {
        format!("У першому залі музею є {}, а в другому залі є {}. Скільки листівок у двох залах разом?", pl(a, &POSTCARD), pl(b, &POSTCARD))
    },
    |a, b| {
        format!(
            "На складі було {}, а потім привезли ще {}. Скільки стільців стало на складі?",
            pl(a, &CHAIR),
            pl(b, &CHAIR)
        )
    },
    |a, b| {
        format!("Навесні в парку висадили {}, а восени висадили ще {}. Скільки дерев висадили за цей рік?", pl(a, &TREE), pl(b, &TREE))
    },
    |a, b| {
        format!("Для свята біля входу повісили {}, а біля сцени повісили {}. Скільки прапорців повісили всього?", pl(a, &FLAG), pl(b, &FLAG))
    },
    |a, b| {
        format!(
            "У коробці є {}, а в іншій коробці є {}. Скільки олівців у двох коробках разом?",
            pl(a, &PENCIL),
            pl(b, &PENCIL)
        )
    },
];

const SUB_T: [Tpl; 8] = [
    |a, b| {
        format!("На полиці було {}. Після того як кілька книжок узяли, там залишилося {}. Скільки книжок узяли?", pl(a, &BOOK), pl(b, &BOOK))
    },
    |a, b| {
        format!(
            "У магазині було {}. За день продали {}. Скільки зошитів залишилося?",
            pl(a, &NOTE),
            pl(b, &NOTE)
        )
    },
    |a, b| {
        format!(
            "У саду росло {}. Під час бурі впало {}. Скільки дерев залишилося?",
            pl(a, &TREE),
            pl(b, &TREE)
        )
    },
    |a, b| {
        format!(
            "У кошику було {}. З нього взяли {}. Скільки яблук залишилося в кошику?",
            pl(a, &APPLE),
            pl(b, &APPLE)
        )
    },
    |a, b| {
        format!(
            "Для змагань підготували {}. Під час гри використали {}. Скільки м'ячів лишилося?",
            pl(a, &BALL),
            pl(b, &BALL)
        )
    },
    |a, b| {
        format!(
            "У залі стояло {}. Після концерту прибрали {}. Скільки стільців залишилося в залі?",
            pl(a, &CHAIR),
            pl(b, &CHAIR)
        )
    },
    |a, b| {
        format!(
            "На стоянці було {}. Увечері забрали {}. Скільки велосипедів залишилося на стоянці?",
            pl(a, &BIKE),
            pl(b, &BIKE)
        )
    },
    |a, b| {
        format!(
            "У пачці було {}. Для малювання використали {}. Скільки аркушів залишилося?",
            pl(a, &SHEET),
            pl(b, &SHEET)
        )
    },
];

const CMP_T: [Tpl; 7] = [
    |a, b| {
        format!("З першої грядки зібрали {a} кг моркви, а з другої грядки зібрали {b} кг. На скільки кілограмів більше зібрали з першої грядки?")
    },
    |a, b| {
        format!("Маса першого вантажу {a} кг, а маса другого вантажу {b} кг. На скільки кілограмів перший вантаж важчий?")
    },
    |a, b| {
        format!("Довжина першої стежки {a} м, а довжина другої стежки {b} м. На скільки метрів перша стежка довша?")
    },
    |a, b| {
        format!("Квиток у кіно коштує {a} грн, а квиток на виставку коштує {b} грн. На скільки гривень квиток у кіно дорожчий?")
    },
    |a, b| {
        format!("Висота першої вежі {a} м, а висота другої вежі {b} м. На скільки метрів друга вежа нижча?")
    },
    |a, b| {
        format!("Першою дорогою до школи йти {a} хв, а другою дорогою йти {b} хв. На скільки хвилин перша дорога довша?")
    },
    |a, b| {
        format!("У першій цистерні {a} л води, а в другій цистерні {b} л. На скільки літрів у другій цистерні менше води?")
    },
];

const MUL_T: [Tpl; 8] = [
    |a, b| {
        format!(
            "У кожній коробці є {}. Усього є {}. Скільки олівців у всіх коробках разом?",
            pl(b, &PENCIL),
            pl(a, &BOX)
        )
    },
    |a, b| {
        format!(
            "У кожному ряду актової зали є {}. Усього є {}. Скільки стільців у залі?",
            pl(b, &CHAIR),
            pl(a, &ROW)
        )
    },
    |a, b| {
        format!(
            "Один пакет містить {}. Придбали {}. Скільки яблук придбали?",
            pl(b, &APPLE),
            pl(a, &BAG)
        )
    },
    |a, b| {
        format!(
            "Один зошит коштує {b} грн. Скільки гривень треба заплатити за {}?",
            pl(a, &NOTE)
        )
    },
    |a, b| {
        format!(
            "Один квиток коштує {b} грн. Скільки гривень треба заплатити за {}?",
            pl(a, &TICKET)
        )
    },
    |a, b| {
        format!(
            "Туристична група щодня проходить {b} км. Скільки кілометрів вона пройде за {}?",
            pl(a, &DAY)
        )
    },
    |a, b| {
        format!(
            "У кожному кошику є {}. Усього є {}. Скільки груш у всіх кошиках разом?",
            pl(b, &PEAR),
            pl(a, &BASKET)
        )
    },
    |a, b| {
        format!(
            "Один бак вміщує {b} л води. Скільки літрів води вміститься в {}?",
            pl(a, &TANK)
        )
    },
];

/// Для ділення `a` — ділене, `b` — дільник.
const DIV_T: [Tpl; 5] = [
    |a, b| {
        format!(
            "Порівну розклали {} у {}. Скільки яблук у кожному кошику?",
            pl(a, &APPLE),
            pl(b, &BASKET)
        )
    },
    |a, b| {
        format!("Стрічку завдовжки {a} м розрізали на однакові відрізки по {b} м. Скільки відрізків вийшло?")
    },
    |a, b| {
        format!(
            "Заплатили {a} грн за {}, усі зошити однакові. Скільки гривень коштує один зошит?",
            pl(b, &NOTE)
        )
    },
    |a, b| {
        format!(
            "У залі порівну розставили {} у {}. Скільки стільців у кожному ряду?",
            pl(a, &CHAIR),
            pl(b, &ROW)
        )
    },
    |a, b| {
        format!(
            "Щоб наповнити бак на {a} л, носять воду відром на {b} л. Скільки відер води потрібно?"
        )
    },
];

fn word1_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let kind = rng.range(0, 4);
    let (idx, a, b, statement, answer, oracle, bugs) = match kind {
        0 => {
            let idx = rng.range(0, ADD_T.len() as i64 - 1);
            let (a, b) = (rng.range(20, 800), rng.range(20, 800));
            if a == b {
                return None;
            }
            let oracle = (1..=b).fold(a, |s, _| s + 1);
            (
                idx,
                a,
                b,
                ADD_T[idx as usize](a, b),
                a + b,
                oracle,
                vec![("subtract_instead", i((a - b).abs()))],
            )
        }
        1 => {
            let idx = rng.range(0, SUB_T.len() as i64 - 1);
            let (a, b) = (rng.range(40, 900), rng.range(20, 800));
            if a <= b {
                return None;
            }
            let oracle = (1..=b).fold(a, |s, _| s - 1);
            (
                idx,
                a,
                b,
                SUB_T[idx as usize](a, b),
                a - b,
                oracle,
                vec![("add_instead", i(a + b))],
            )
        }
        2 => {
            let idx = rng.range(0, CMP_T.len() as i64 - 1);
            let (a, b) = (rng.range(40, 900), rng.range(20, 800));
            if a <= b {
                return None;
            }
            let oracle = (1..=b).fold(a, |s, _| s - 1);
            (
                idx,
                a,
                b,
                CMP_T[idx as usize](a, b),
                a - b,
                oracle,
                vec![("add_instead", i(a + b))],
            )
        }
        3 => {
            let idx = rng.range(0, MUL_T.len() as i64 - 1);
            let (a, b) = (rng.range(3, 15), rng.range(3, 60));
            let oracle = (0..a).map(|_| b).sum::<i64>();
            (
                idx,
                a,
                b,
                MUL_T[idx as usize](a, b),
                a * b,
                oracle,
                vec![("add_instead", i(a + b))],
            )
        }
        _ => {
            let idx = rng.range(0, DIV_T.len() as i64 - 1);
            let (b, q) = (rng.range(3, 12), rng.range(3, 40));
            let a = b * q;
            // Оракул: скільки разів можна забрати по `b`.
            let (mut left, mut count) = (a, 0);
            while left >= b {
                left -= b;
                count += 1;
            }
            (
                idx,
                a,
                b,
                DIV_T[idx as usize](a, b),
                q,
                count,
                vec![
                    ("multiply_instead", i(a * b)),
                    ("subtract_instead", i(a - b)),
                ],
            )
        }
    };
    let task = Make {
        skill: "WORD-1",
        variant,
        statement,
        operands: ints(&[a, b]),
        params: vec![kind, idx, a, b],
        answer: i(answer),
        steps: vec![],
        bugs,
    }
    .build();
    Some((task, i(oracle)))
}

fn word1_hint(task: &Task, level: ScriptedLevel) -> String {
    let (a, b) = (&task.operand_labels[0], &task.operand_labels[1]);
    let r2 = match task.params[0] {
        0 => "Коли із частин треба знайти ціле, частини додають.",
        1 => "Коли від цілого щось забрали й треба знайти те, що лишилося (або те, що забрали), виконують віднімання.",
        2 => "Щоб дізнатися, на скільки одне число більше за інше, від більшого віднімають менше.",
        3 => "Коли однакове число повторюється кілька разів, виконують множення.",
        _ => "Коли ціле розділили на однакові частини або треба дізнатися, скільки разів одне число міститься в іншому, виконують ділення.",
    };
    by_level(
        level,
        "Прочитай умову ще раз. Що в ній відбувається: щось збирають разом, забирають, порівнюють, повторюють однакове чи ділять порівну?".to_string(),
        r2.to_string(),
        format!("Запиши розв'язання виразом з числами {a} і {b}. Потім перевір, чи відповідь має сенс: чи вона більша або менша за дані числа так, як це буває в такій історії."),
    )
}

fn word1_steps(c: &Task) -> Vec<String> {
    let step = match c.params[0] {
        0 => "Тут частини об'єднують, тому дію вибираємо таку, що знаходить ціле з частин.",
        1 => "Тут від цілого щось забирають, тому дія протилежна додаванню.",
        2 => "Тут порівнюють два числа й шукають, на скільки одне більше за інше.",
        3 => "Тут однакові групи повторюються, тому швидше рахувати множенням.",
        _ => "Тут ціле розподіляють на однакові частини, тому потрібне ділення.",
    };
    vec![
        step.to_string(),
        "Записуємо вираз з числами умови й обчислюємо його.".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// WORD-2: текстові задачі у дві дії

pub const WORD_2: Spec = Spec {
    id: "WORD-2",
    make: word2_make,
    hint: word2_hint,
    steps: word2_steps,
    prompt: "Тепер ти: виконай обидві дії й запиши відповідь на питання задачі.",
};

fn w2_statement(kind: i64, t: i64, n: [i64; 4]) -> String {
    let [a, b, c, d] = n;
    match (kind, t) {
        (0, 0) => format!("Купили {}. У кожному пакеті є {}, а ще окремо є {}. Скільки всього яблук?", pl(a, &BAG), pl(b, &APPLE), pl(c, &APPLE)),
        (0, 1) => format!("У їдальні є {}. Біля кожного столу є {}, а ще є {} окремо біля стіни. Скільки всього стільців у їдальні?", pl(a, &TABLE), pl(b, &CHAIR), pl(c, &CHAIR)),
        (0, _) => format!("На складі є {}. У кожному ящику є {}, а ще є {} просто на полиці. Скільки всього зошитів на складі?", pl(a, &CRATE), pl(b, &NOTE), pl(c, &NOTE)),
        (1, 0) => format!("У магазин привезли {}, у кожному ящику є {}. За день продали {}. Скільки яблук залишилося?", pl(a, &CRATE), pl(b, &APPLE), pl(c, &APPLE)),
        (1, 1) => format!("У бібліотеці є {}, на кожній полиці є {}. Із них {} зараз у читачів. Скільки книжок лишилося на полицях?", pl(a, &SHELF), pl(b, &BOOK), pl(c, &BOOK)),
        (1, _) => format!("У залі є {}, у кожному ряду є {}. Вільними залишилося {}. Скільки стільців зайнято?", pl(a, &ROW), pl(b, &CHAIR), pl(c, &CHAIR)),
        (2, 0) => format!("У шкільному магазині було {}. Привезли ще {}, а потім продали {}. Скільки зошитів стало в магазині?", pl(a, &NOTE), pl(b, &NOTE), pl(c, &NOTE)),
        (2, 1) => format!("На стоянці було {}. Приїхало ще {}, а {} поїхало. Скільки велосипедів на стоянці тепер?", pl(a, &BIKE), pl(b, &BIKE), pl(c, &BIKE)),
        (2, _) => format!("У мішку було {a} кг борошна. Додали ще {b} кг, а потім використали {c} кг. Скільки кілограмів борошна лишилося?"),
        (3, 0) => format!("У баку було {a} л води. Уранці використали {b} л, а ввечері ще {c} л. Скільки літрів води лишилося?"),
        (3, 1) => format!("На шкільному рахунку було {a} грн. Витратили {b} грн на папір і {c} грн на фарби. Скільки гривень лишилося?"),
        (3, _) => format!("Довжина стрічки {a} см. Спершу відрізали {b} см, а потім ще {c} см. Скільки сантиметрів стрічки лишилося?"),
        (4, 0) => format!("Першого дня зібрали {a} кг яблук, другого дня зібрали {b} кг. Усі яблука порівну розклали у {}. Скільки кілограмів яблук у кожному ящику?", pl(c, &CRATE)),
        (4, _) => format!("У двох бочках було {a} л і {b} л соку. Увесь сік порівну розлили у {}. Скільки літрів соку у кожному бідоні?", pl(c, &FLASK)),
        (_, 0) => format!("Купили {} по {b} грн кожен і {} по {d} грн кожен. Скільки гривень заплатили за всю покупку?", pl(a, &NOTE), pl(c, &PENCIL)),
        (_, 1) => format!("Продали {} для дітей по {b} грн і {} для дорослих по {d} грн. Скільки гривень виручили?", pl(a, &TICKET), pl(c, &TICKET)),
        (_, _) => format!("Туристи в перший день йшли {a} год зі швидкістю {b} км/год, а в другий день {c} год зі швидкістю {d} км/год. Скільки кілометрів вони пройшли за два дні?"),
    }
}

#[allow(clippy::type_complexity)]
fn word2_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let kind = rng.range(0, 5);
    let t = rng.range(0, if kind == 4 { 1 } else { 2 });
    let (n, operands, answer, steps, bugs, oracle): (
        [i64; 4],
        Vec<i64>,
        i64,
        Vec<i64>,
        Vec<(&'static str, i64)>,
        i64,
    ) = match kind {
        0 => {
            let (a, b, c) = (rng.range(3, 12), rng.range(3, 25), rng.range(2, 40));
            let mut total = c;
            (0..a).for_each(|_| total += b);
            (
                [a, b, c, 0],
                vec![a, b, c],
                a * b + c,
                vec![a * b],
                vec![("first_step_only", a * b), ("wrong_operation", a * b - c)],
                total,
            )
        }
        1 => {
            let (a, b) = (rng.range(3, 12), rng.range(3, 25));
            let c = rng.range(2, a * b / 2);
            let mut total = 0;
            (0..a).for_each(|_| total += b);
            (
                [a, b, c, 0],
                vec![a, b, c],
                a * b - c,
                vec![a * b],
                vec![("first_step_only", a * b), ("wrong_operation", a * b + c)],
                total - c,
            )
        }
        2 => {
            let (a, b) = (rng.range(50, 500), rng.range(10, 300));
            let c = rng.range(5, a + b - 10);
            let up = (1..=b).fold(a, |s, _| s + 1);
            let down = (1..=c).fold(up, |s, _| s - 1);
            (
                [a, b, c, 0],
                vec![a, b, c],
                a + b - c,
                vec![a + b],
                vec![("first_step_only", a + b), ("wrong_operation", a + b + c)],
                down,
            )
        }
        3 => {
            let a = rng.range(200, 900);
            let (b, c) = (rng.range(10, 300), rng.range(10, 300));
            if b + c + 5 > a {
                return None;
            }
            let first = (1..=b).fold(a, |s, _| s - 1);
            let second = (1..=c).fold(first, |s, _| s - 1);
            (
                [a, b, c, 0],
                vec![a, b, c],
                a - b - c,
                vec![a - b],
                vec![("first_step_only", a - b), ("wrong_operation", a - b + c)],
                second,
            )
        }
        4 => {
            let c = rng.range(3, 12);
            let q = rng.range(8, 40);
            let a = rng.range(10, c * q - 10);
            let b = c * q - a;
            // Оракул: підбір частки, яка при множенні на кількість частин дає суму.
            let found = (1..).find(|k| k * c == a + b)?;
            (
                [a, b, c, 0],
                vec![a, b, c],
                q,
                vec![a + b],
                vec![("first_step_only", a + b), ("wrong_operation", (a + b) * c)],
                found,
            )
        }
        _ => {
            let (a, b, c, d) = if t == 2 {
                (
                    rng.range(2, 8),
                    rng.range(3, 6),
                    rng.range(2, 8),
                    rng.range(3, 6),
                )
            } else {
                (
                    rng.range(2, 15),
                    rng.range(5, 60),
                    rng.range(2, 15),
                    rng.range(5, 60),
                )
            };
            let mut total = 0;
            (0..a).for_each(|_| total += b);
            (0..c).for_each(|_| total += d);
            (
                [a, b, c, d],
                vec![a, b, c, d],
                a * b + c * d,
                vec![a * b, c * d],
                vec![
                    ("first_step_only", a * b),
                    ("add_all_numbers", a + b + c + d),
                ],
                total,
            )
        }
    };
    let task = Make {
        skill: "WORD-2",
        variant,
        statement: w2_statement(kind, t, n),
        operands: ints(&operands),
        params: vec![kind, t, n[0], n[1], n[2], n[3]],
        answer: i(answer),
        steps: steps.into_iter().map(i).collect(),
        bugs: bugs.into_iter().map(|(id, v)| (id, i(v))).collect(),
    }
    .build();
    Some((task, i(oracle)))
}

fn word2_hint(task: &Task, level: ScriptedLevel) -> String {
    let r2 = match task.params[0] {
        0 => "Спершу дізнайся, скільки всього в повних однакових групах, а потім додай те, що є окремо.",
        1 => "Спершу знайди, скільки всього було в повних однакових групах, а потім відніми те, що вибуло.",
        2 => "Спершу дізнайся, скільки стало після того, як додали, а потім відніми те, що забрали.",
        3 => "Забирали двічі. Можна відняти обидві частини по черзі або спершу додати їх і відняти один раз.",
        4 => "Спершу дізнайся, скільки всього разом, а потім поділи порівну.",
        _ => "Спершу знайди окремо результат для кожного з двох видів, а потім додай їх.",
    };
    by_level(
        level,
        "Це задача у дві дії. Прочитай умову ще раз і скажи, що спершу треба дізнатися.".to_string(),
        r2.to_string(),
        "Запиши розв'язання двома діями або одним виразом, перевір порядок дій і те, чи відповідь має сенс у цій історії.".to_string(),
    )
}

fn word2_steps(c: &Task) -> Vec<String> {
    let first = match c.params[0] {
        0 => "Перша дія: скільки всього в однакових групах разом. Друга дія: додаємо те, що окремо.",
        1 => "Перша дія: скільки всього було в однакових групах. Друга дія: віднімаємо те, що вибуло.",
        2 => "Перша дія: скільки стало після додавання. Друга дія: віднімаємо те, що забрали.",
        3 => "Перша дія: віднімаємо першу частину. Друга дія: від того, що лишилося, віднімаємо другу.",
        4 => "Перша дія: знаходимо загальну кількість. Друга дія: ділимо її порівну.",
        _ => "Перша дія: знаходимо результат для кожного з двох видів. Друга дія: додаємо ці результати.",
    };
    vec![
        first.to_string(),
        "Записуємо кожну дію з числами умови й обчислюємо по черзі.".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// MOTION: задачі на рух (шлях = швидкість × час, швидкість зближення)

pub const MOTION: Spec = Spec {
    id: "MOTION",
    make: motion_make,
    hint: motion_hint,
    steps: motion_steps,
    prompt: "Тепер ти: знайди шукану величину й запиши число.",
};

/// (швидкість, відстань, час, час у родовому множини, найменший і найбільший час)
const SETS: [(&str, &str, &str, &str, i64, i64); 3] = [
    ("км/год", "км", "год", "годин", 2, 9),
    ("м/хв", "м", "хв", "хвилин", 5, 30),
    ("м/с", "м", "с", "секунд", 5, 30),
];

/// (хто, дієслово, набір одиниць, найменша й найбільша швидкість)
const MOVERS: [(&str, &str, usize, i64, i64); 11] = [
    ("Велосипедист", "їде", 0, 12, 24),
    ("Автобус", "їде", 0, 40, 80),
    ("Потяг", "рухається", 0, 60, 120),
    ("Електричка", "рухається", 0, 50, 90),
    ("Катер", "пливе", 0, 20, 40),
    ("Яхта", "пливе", 0, 8, 20),
    ("Літак", "летить", 0, 500, 900),
    ("Туристична група", "йде", 0, 3, 6),
    ("Пішохід", "йде", 1, 50, 90),
    ("Плавець", "пливе", 1, 30, 60),
    ("Бігун", "біжить", 2, 4, 9),
];

/// (хто в множині, дієслово, набір одиниць, швидкості)
const PAIRS: [(&str, &str, usize, i64, i64); 6] = [
    ("велосипедисти", "виїхали", 0, 12, 24),
    ("автобуси", "виїхали", 0, 40, 80),
    ("потяги", "вирушили", 0, 60, 120),
    ("катери", "вирушили", 0, 20, 40),
    ("пішоходи", "вийшли", 1, 50, 90),
    ("бігуни", "побігли", 2, 4, 9),
];

fn motion_make(rng: &mut SeededRng, variant: u64) -> Option<(Task, Frac)> {
    let kind = rng.range(0, 3);
    if kind == 3 {
        let (who, verb, set, vmin, vmax) = *pick(rng, &PAIRS);
        let (su, du, tu, _, tmin, tmax) = SETS[set];
        let (v1, v2) = (rng.range(vmin, vmax), rng.range(vmin, vmax));
        if v1 == v2 {
            return None;
        }
        let t = rng.range(tmin, tmax);
        let oracle = (0..t).map(|_| v1 + v2).sum::<i64>();
        let task = Make {
            skill: "MOTION",
            variant,
            statement: format!("Назустріч одне одному одночасно {verb} два {who}. Швидкість першого {v1} {su}, швидкість другого {v2} {su}. Вони зустрілися через {t} {tu}. Яка була початкова відстань між ними, у {du}?"),
            operands: ints(&[v1, v2, t]),
            params: vec![kind, set as i64, v1, v2, t],
            answer: i((v1 + v2) * t),
            steps: vec![i(v1 + v2)],
            bugs: vec![("forgot_time", i(v1 + v2)), ("one_object", i(v1 * t))],
        }
        .build();
        return Some((task, i(oracle)));
    }
    let (who, verb, set, vmin, vmax) = *pick(rng, &MOVERS);
    let (su, du, tu, tw, tmin, tmax) = SETS[set];
    let (v, t) = (rng.range(vmin, vmax), rng.range(tmin, tmax));
    let s = v * t;
    let (statement, operands, answer, oracle, bugs) = match kind {
        0 => (
            format!("{who} {verb} зі сталою швидкістю {v} {su}. Яку відстань подолано за {t} {tu}? Відповідь дай у {du}."),
            vec![v, t],
            s,
            (0..t).map(|_| v).sum::<i64>(),
            vec![("add_instead", i(v + t)), ("divide_instead", fr(v, t))],
        ),
        1 => (
            format!("{who} {verb} зі сталою швидкістю {v} {su}. Весь шлях становить {s} {du}. Скільки {tw} триває рух?"),
            vec![v, s],
            t,
            (1..).find(|k| v * k == s)?,
            vec![("multiply_instead", i(s * v))],
        ),
        _ => (
            format!("{who} {verb} рівномірно й долає {s} {du} за {t} {tu}. Знайди швидкість руху в {su}."),
            vec![s, t],
            v,
            (1..).find(|k| k * t == s)?,
            vec![("multiply_instead", i(s * t))],
        ),
    };
    let task = Make {
        skill: "MOTION",
        variant,
        statement,
        operands: ints(&operands),
        params: vec![kind, set as i64, v, t],
        answer: i(answer),
        steps: vec![],
        bugs,
    }
    .build();
    Some((task, i(oracle)))
}

fn motion_hint(task: &Task, level: ScriptedLevel) -> String {
    let r2 = match task.params[0] {
        0 => "Відстань дорівнює швидкості, помноженій на час.",
        1 => "Час дорівнює відстані, поділеній на швидкість.",
        2 => "Швидкість дорівнює відстані, поділеній на час.",
        _ => "Об'єкти наближаються один до одного, тому за кожну одиницю часу відстань між ними зменшується на суму їхніх швидкостей.",
    };
    by_level(
        level,
        "Які величини дано в задачі, а яку треба знайти? Пригадай зв'язок між швидкістю, часом і відстанню.".to_string(),
        r2.to_string(),
        "Перевір одиниці виміру: у відповіді мають бути ті одиниці, про які питають у запитанні.".to_string(),
    )
}

fn motion_steps(c: &Task) -> Vec<String> {
    let rule = match c.params[0] {
        0 => "Шукаємо відстань: швидкість множимо на час.",
        1 => "Шукаємо час: відстань ділимо на швидкість.",
        2 => "Шукаємо швидкість: відстань ділимо на час.",
        _ => "Спершу знаходимо швидкість зближення (сума швидкостей), а потім множимо її на час.",
    };
    vec![
        rule.to_string(),
        "Підставляємо числа з умови й обчислюємо.".to_string(),
    ]
}

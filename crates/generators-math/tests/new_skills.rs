//! Навички N-SUB-MULTI, N-MUL-MULTI, N-DIV-MULTI, N-ORDER, D-SUB, F-SUB-SAME, F-SUB-DIFF:
//! ручно обчислені приклади та перевірка за незалежними формулами з параметрів задачі.

use generators_math::generator_for;
use tutor_core::frac::Frac;
use tutor_core::task::{Task, Verdict};

fn find(skill: &str, statement: &str) -> Task {
    let g = generator_for(skill).expect("навичка існує");
    (1..=20_000)
        .map(|v| g.generate(v))
        .find(|t| t.statement == statement)
        .unwrap_or_else(|| panic!("{skill}: умови «{statement}» немає серед перших варіантів"))
}

fn frac(n: i64, d: i64) -> Frac {
    Frac::new(n, d).unwrap()
}

fn bug_of(t: &Task) -> Frac {
    assert_eq!(t.bugs.len(), 1);
    t.bugs[0].value
}

/// Типова помилка розпізнається під своїм id, правильна відповідь як правильна, помилка не збігається з відповіддю.
fn assert_answer_and_bug(t: &Task, answer: Frac, bug_id: &'static str, bug: Frac) {
    assert_eq!(t.answer, answer, "{}", t.statement);
    assert_eq!(t.bugs[0].id, bug_id);
    assert_eq!(bug_of(t), bug, "{}", t.statement);
    assert_ne!(answer, bug);
    assert_eq!(t.check(&answer.to_string()), Verdict::Correct);
    assert_eq!(t.check(&bug.to_string()), Verdict::Bug(bug_id));
}

#[test]
fn n_sub_multi_hand_example() {
    // 654 - 197 = 457; помилка: |6-1| |5-9| |4-7| = 543.
    let t = find("N-SUB-MULTI", "Обчисли: 654 - 197");
    assert_answer_and_bug(&t, Frac::int(457), "smaller_from_larger", Frac::int(543));
}

#[test]
fn n_mul_multi_hand_example() {
    // 27 × 68 = 1836; помилка: 27*8 + 27*6 = 216 + 162 = 378.
    let t = find("N-MUL-MULTI", "Обчисли: 27 × 68");
    assert_answer_and_bug(&t, Frac::int(1836), "no_shift", Frac::int(378));
}

#[test]
fn n_div_multi_hand_example() {
    // 604 : 2 = 302; помилка: нуль пропущено, 32.
    let t = find("N-DIV-MULTI", "Обчисли: 604 : 2");
    assert_answer_and_bug(&t, Frac::int(302), "skip_zero", Frac::int(32));
}

#[test]
fn n_order_hand_example() {
    // 21 - 4 × 5 = 1; помилка зліва направо: (21 - 4) × 5 = 85.
    let t = find("N-ORDER", "Обчисли: 21 - 4 × 5");
    assert_answer_and_bug(&t, Frac::int(1), "left_to_right", Frac::int(85));
}

#[test]
fn d_sub_hand_example() {
    // 8,18 - 1,6 = 6,58; помилка: 8-1 = 7 і 18-6 = 12 соті, тобто 7,12.
    let t = find("D-SUB", "Обчисли: 8,18 - 1,6");
    assert_answer_and_bug(
        &t,
        frac(658, 100),
        "fraction_digits_as_integers",
        frac(712, 100),
    );
    assert_eq!(t.check("6,58"), Verdict::Correct);
    assert_eq!(t.check("7,12"), Verdict::Bug("fraction_digits_as_integers"));
}

#[test]
fn f_sub_same_hand_example() {
    // 14/18 - 9/18 = 5/18; помилка: 5/36.
    let t = find("F-SUB-SAME", "Обчисли: 14/18 - 9/18");
    assert_answer_and_bug(&t, frac(5, 18), "add_denominators", frac(5, 36));
}

#[test]
fn f_sub_diff_hand_example() {
    // 3/4 - 1/2 = 1/4; помилка: (3-1)/(4-2) = 1.
    let t = find("F-SUB-DIFF", "Обчисли: 3/4 - 1/2");
    assert_answer_and_bug(&t, frac(1, 4), "subtract_all_parts", Frac::int(1));
    assert_eq!(t.check("0,25"), Verdict::Correct);
}

/// Незалежна перевірка за параметрами: відповіді додатні, у тому числі проміжні.
#[test]
fn answers_match_independent_formulas() {
    let g = |s| generator_for(s).unwrap();
    for v in 1..=2_000 {
        let t = g("N-SUB-MULTI").generate(v);
        let (a, b) = (t.params[0], t.params[1]);
        assert!(
            a > b && a % 10 < b % 10,
            "потрібне позичання: {}",
            t.statement
        );
        assert_eq!(t.answer, Frac::int(a - b));

        let t = g("N-MUL-MULTI").generate(v);
        let (a, b) = (t.params[0], t.params[1]);
        assert_eq!(t.answer, Frac::int(a * b));
        assert_eq!(bug_of(&t), Frac::int(a * (b % 10) + a * (b / 10)));

        let t = g("N-DIV-MULTI").generate(v);
        let (n, d) = (t.params[0], t.params[1]);
        assert_eq!(n % d, 0, "ділення націло: {}", t.statement);
        assert_eq!(t.answer, Frac::int(n / d));
        assert!(
            t.answer.to_string().contains('0'),
            "у частці є нуль: {}",
            t.statement
        );
        assert!(!bug_of(&t).to_string().contains('0'));

        let t = g("N-ORDER").generate(v);
        let (a, b, c, plus) = (t.params[0], t.params[1], t.params[2], t.params[3] == 1);
        let (ans, ltr) = if plus {
            (a + b * c, (a + b) * c)
        } else {
            (a - b * c, (a - b) * c)
        };
        assert!(ans > 0 && ltr > 0, "{}", t.statement);
        assert_eq!(t.answer, Frac::int(ans));
        assert_eq!(bug_of(&t), Frac::int(ltr));

        let t = g("D-SUB").generate(v);
        assert!(t.answer > Frac::int(0));
        assert!(t.operands[0] > t.operands[1]);

        let t = g("F-SUB-SAME").generate(v);
        let (n1, n2, d) = (t.params[0], t.params[1], t.params[2]);
        assert!(n1 > n2 && n1 < d);
        assert_eq!(t.answer, frac(n1 - n2, d));

        let t = g("F-SUB-DIFF").generate(v);
        let (d1, d2) = (t.params[1], t.params[3]);
        assert_ne!(d1, d2, "знаменники різні: {}", t.statement);
        assert!(t.answer > Frac::int(0) && t.answer < Frac::int(1));
        assert_eq!(t.answer, t.operands[0].checked_sub(t.operands[1]).unwrap());
    }
}

/// Відповідь завжди ціле, правильний дріб або скінченний десятковий: мішаних чисел у відповідях немає.
#[test]
fn no_mixed_numbers_in_answers() {
    for id in [
        "N-SUB-MULTI",
        "N-MUL-MULTI",
        "N-DIV-MULTI",
        "N-ORDER",
        "D-SUB",
        "F-SUB-SAME",
        "F-SUB-DIFF",
    ] {
        let g = generator_for(id).unwrap();
        for v in 1..=1_000 {
            let t = g.generate(v);
            let ok = t.answer.is_integer()
                || t.answer < Frac::int(1)
                || (id == "D-SUB" && 100 % t.answer.den() == 0);
            assert!(ok, "{id} #{v}: {}", t.statement);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Навички розширення курсу (рішення A20): ручні приклади, перелік помилок, аркуш, незалежний розв'язувач.

#[path = "lessons_solver/mod.rs"]
mod solver;

use tutor_core::sheet::SheetContent;

/// Перевіряє відповідь і повний перелік помилок задачі (id і значення в порядку генератора).
fn assert_task(skill: &str, statement: &str, answer: Frac, bugs: &[(&str, Frac)]) {
    let t = find(skill, statement);
    assert_eq!(t.answer, answer, "{statement}");
    let got: Vec<(&str, Frac)> = t.bugs.iter().map(|b| (b.id, b.value)).collect();
    assert_eq!(got, bugs, "{statement}");
    assert_eq!(t.check(&answer.to_string()), Verdict::Correct);
    for (id, v) in bugs {
        assert_ne!(*v, answer);
        assert!(matches!(t.check(&v.to_string()), Verdict::Bug(b) if b == *id));
    }
}

fn i(n: i64) -> Frac {
    Frac::int(n)
}

#[test]
fn n_round_hand_examples() {
    // 4208 до сотень: наступна цифра 0, округлюємо вниз; помилка: завжди вгору.
    assert_task(
        "N-ROUND",
        "Округли число 4208 до сотень.",
        i(4200),
        &[("always_up", i(4300))],
    );
    // 61278 до десятків: наступна цифра 8, вгору; помилка: просто відкинути.
    assert_task(
        "N-ROUND",
        "Округли число 61278 до десятків.",
        i(61280),
        &[("cut_off", i(61270))],
    );
}

#[test]
fn n_divrem_hand_examples() {
    // 173 = 8 · 21 + 5.
    assert_task(
        "N-DIVREM",
        "Знайди остачу від ділення 173 на 8.",
        i(5),
        &[("quotient_instead", i(21)), ("remainder_too_big", i(13))],
    );
    // 113 = 4 · 28 + 1.
    assert_task(
        "N-DIVREM",
        "Знайди неповну частку від ділення 113 на 4.",
        i(28),
        &[("remainder_instead", i(1)), ("round_up_quotient", i(29))],
    );
}

#[test]
fn n_divtest_hand_example() {
    // На 9 діляться 963, 198, 801, 477, 594 (сума цифр 18 чи 9): п'ять. На 3 ще й 447: шість.
    assert_task(
        "N-DIVTEST",
        "Скільки з чисел 401, 787, 963, 198, 801, 477, 594, 447 діляться на 9?",
        i(5),
        &[("rule_for_three", i(6))],
    );
    // 249 (сума 15) ділиться на 3, але в іншому списку: перевірка загального правила діленням.
    for v in 1..=500 {
        let t = generator_for("N-DIVTEST").unwrap().generate(v);
        let d = t.params[0];
        let nums = &t.operands[..t.operands.len() - 1];
        let by_division = nums.iter().filter(|n| n.num() % d == 0).count() as i64;
        assert_eq!(t.answer, i(by_division), "{}", t.statement);
    }
}

#[test]
fn n_gcd_lcm_hand_examples() {
    assert_task(
        "N-GCD",
        "Знайди найбільший спільний дільник чисел 36 і 48.",
        i(12),
        &[("not_greatest", i(2)), ("lcm_instead", i(144))],
    );
    assert_task(
        "N-LCM",
        "Знайди найменше спільне кратне чисел 25 і 45.",
        i(225),
        &[("product", i(1125)), ("gcd_instead", i(5))],
    );
}

#[test]
fn n_power_hand_examples() {
    // 7³ - 2 = 343 - 2 = 341; помилка: 7 · 3 - 2 = 19.
    assert_task(
        "N-POWER",
        "Обчисли: 7³ - 2",
        i(341),
        &[("multiply_by_exponent", i(19))],
    );
    // 12² + 15 = 144 + 15 = 159; помилка: 12 · 2 + 15 = 39.
    assert_task(
        "N-POWER",
        "Обчисли: 12² + 15",
        i(159),
        &[("multiply_by_exponent", i(39))],
    );
    assert_task(
        "N-POWER",
        "Обчисли: 4²",
        i(16),
        &[("multiply_by_exponent", i(8))],
    );
}

#[test]
fn equation_hand_examples() {
    assert_task(
        "EQ-ADD-SUB",
        "Розв'яжи рівняння: 147 + x = 447",
        i(300),
        &[("add_instead", i(594))],
    );
    // x - 143 = 202: x = 345; помилка: різниця чисел.
    assert_task(
        "EQ-ADD-SUB",
        "Розв'яжи рівняння: x - 143 = 202",
        i(345),
        &[("subtract_instead", i(59))],
    );
    // 482 - x = 228: x = 254.
    assert_task(
        "EQ-ADD-SUB",
        "Розв'яжи рівняння: 482 - x = 228",
        i(254),
        &[("add_instead", i(710))],
    );
    assert_task(
        "EQ-MUL",
        "Розв'яжи рівняння: x : 9 = 19",
        i(171),
        &[("divide_instead", frac(19, 9))],
    );
    // 300 : x = 12: x = 25.
    assert_task(
        "EQ-MUL",
        "Розв'яжи рівняння: 300 : x = 12",
        i(25),
        &[
            ("multiply_instead", i(3600)),
            ("divisor_swapped", frac(1, 25)),
        ],
    );
}

#[test]
fn word_problem_hand_examples() {
    assert_task(
        "WORD-1",
        "Туристична група щодня проходить 9 км. Скільки кілометрів вона пройде за 7 днів?",
        i(63),
        &[("add_instead", i(16))],
    );
    assert_task(
        "WORD-1",
        "У залі стояло 702 стільці. Після концерту прибрали 477 стільців. Скільки стільців залишилося в залі?",
        i(225),
        &[("add_instead", i(1179))],
    );
    // 4 · 14 - 25 = 31.
    assert_task(
        "WORD-2",
        "У залі є 4 ряди, у кожному ряду є 14 стільців. Вільними залишилося 25 стільців. Скільки стільців зайнято?",
        i(31),
        &[("first_step_only", i(56)), ("wrong_operation", i(81))],
    );
    // 491 + 287 - 65 = 713.
    assert_task(
        "WORD-2",
        "На стоянці було 491 велосипед. Приїхало ще 287 велосипедів, а 65 велосипедів поїхало. Скільки велосипедів на стоянці тепер?",
        i(713),
        &[("first_step_only", i(778)), ("wrong_operation", i(843))],
    );
}

#[test]
fn motion_hand_examples() {
    assert_task(
        "MOTION",
        "Автобус їде зі сталою швидкістю 68 км/год. Весь шлях становить 408 км. Скільки годин триває рух?",
        i(6),
        &[("multiply_instead", i(27744))],
    );
    // (87 + 68) · 7 = 1085.
    assert_task(
        "MOTION",
        "Назустріч одне одному одночасно вийшли два пішоходи. Швидкість першого 87 м/хв, швидкість другого 68 м/хв. Вони зустрілися через 7 хв. Яка була початкова відстань між ними, у м?",
        i(1085),
        &[("forgot_time", i(155)), ("one_object", i(609))],
    );
}

#[test]
fn unit_conversion_hand_examples() {
    // 18 км 401 м = 18 401 м; помилки: 18 + 401 і 18 · 100 + 401.
    assert_task(
        "U-LEN",
        "Запиши в метрах: 18 км 401 м.",
        i(18401),
        &[("add_only", i(419)), ("wrong_zeros", i(2201))],
    );
    assert_task(
        "U-LEN",
        "Переведи 29 км у метри.",
        i(29000),
        &[
            ("wrong_direction", frac(29, 1000)),
            ("wrong_zeros", i(2900)),
        ],
    );
    // 3 кг 31 г = 3031 г (нуль у «031» не губимо).
    assert_task(
        "U-MASS",
        "Запиши в грамах: 3 кг 31 г.",
        i(3031),
        &[("add_only", i(34)), ("wrong_zeros", i(331))],
    );
    // т -> ц: співвідношення 10, помилки «не те число нулів» немає.
    assert_task(
        "U-MASS",
        "Переведи 31 т у центнери.",
        i(310),
        &[("wrong_direction", frac(31, 10))],
    );
    // 7 год = 420 хв; помилка десяткової системи: 700.
    assert_task(
        "U-TIME",
        "Переведи 7 год у хвилини.",
        i(420),
        &[("wrong_direction", frac(7, 60)), ("decimal_system", i(700))],
    );
    assert_task(
        "U-TIME",
        "Переведи 2460 с у хвилини.",
        i(41),
        &[
            ("wrong_direction", i(147_600)),
            ("decimal_system", frac(123, 5)),
        ],
    );
}

#[test]
fn geometry_hand_examples() {
    // Периметр квадрата 34 дм: 136; площа 1156; три сторони 102.
    assert_task(
        "G-PERIM",
        "Знайди периметр квадратного аркуша зі стороною 34 см.",
        i(136),
        &[("area_instead", i(1156)), ("three_sides", i(102))],
    );
    assert_task(
        "G-PERIM",
        "Знайди периметр прямокутника зі сторонами 31 км і 38 км.",
        i(138),
        &[("half_perimeter", i(69)), ("area_instead", i(1178))],
    );
    // 25 · 25 = 625.
    assert_task(
        "G-AREA",
        "Площа квадрата дорівнює 625 см². Знайди довжину сторони.",
        i(25),
        &[
            ("divide_by_two", frac(625, 2)),
            ("perimeter_confusion", frac(625, 4)),
        ],
    );
    // 322 : 23 = 14.
    assert_task(
        "G-AREA",
        "Площа прямокутної ділянки дорівнює 322 м², одна зі сторін має довжину 23 м. Знайди довжину сусідньої сторони.",
        i(14),
        &[("multiply_instead", i(7406)), ("subtract_instead", i(299))],
    );
    assert_task(
        "G-VOL",
        "Знайди об'єм куба з ребром 11 м.",
        i(1331),
        &[("triple_edge", i(33)), ("forgot_third_factor", i(121))],
    );
    // 576 : 48 = 12.
    assert_task(
        "G-VOL",
        "Об'єм прямокутного паралелепіпеда дорівнює 576 м³, площа його основи 48 м². Знайди висоту.",
        i(12),
        &[("multiply_instead", i(27648)), ("subtract_instead", i(528))],
    );
    assert_task(
        "A-ANGLE",
        "Один із двох суміжних кутів дорівнює 144°. Знайди другий суміжний кут.",
        i(36),
        &[("use_right_angle", i(54)), ("full_turn", i(216))],
    );
    assert_task(
        "A-ANGLE",
        "У трикутнику два кути дорівнюють 73° і 27°. Знайди третій кут.",
        i(80),
        &[("use_full_turn", i(260)), ("forgot_one_angle", i(107))],
    );
}

#[test]
fn fraction_percent_and_data_hand_examples() {
    // 3 3/7 = (3 · 7 + 3)/7 = 24/7; помилки: 21 і «33».
    assert_task(
        "F-MIXED",
        "Запиши мішане число 3 3/7 неправильним дробом. Чому дорівнює чисельник?",
        i(24),
        &[("forgot_to_add", i(21)), ("digits_side_by_side", i(33))],
    );
    // 57 = 5 · 11 + 2.
    assert_task(
        "F-MIXED",
        "Запиши неправильний дріб 57/5 мішаним числом. Яка в нього дробова частина?",
        frac(2, 5),
        &[
            ("whole_over_denominator", frac(11, 5)),
            ("remainder_only", i(2)),
        ],
    );
    // 144/162 = 8/9 (НСД 18); частково скоротили на 2: знаменник 81.
    assert_task(
        "F-REDUCE",
        "Скороти дріб 144/162 повністю. Який знаменник має скорочений дріб?",
        i(9),
        &[("partial_reduce", i(81)), ("asked_other_part", i(8))],
    );
    assert_task(
        "F-REDUCE",
        "На яке найбільше число можна скоротити дріб 8/32?",
        i(8),
        &[("partial_reduce", i(2))],
    );
    // 2/5 від 35 = 14.
    assert_task(
        "F-OF",
        "У мішку було 35 кг борошна. Витратили 2/5 борошна. Скільки кілограмів борошна витратили?",
        i(14),
        &[
            ("multiply_by_denominator", i(175)),
            ("forgot_numerator", i(7)),
            ("divide_by_numerator", frac(35, 2)),
        ],
    );
    // 1/2 від 14 = 7; чисельник 1: помилки «забув помножити» немає.
    assert_task(
        "F-OF",
        "У пляшці було 14 мл соку. Випили 1/2 соку. Скільки мілілітрів соку випили?",
        i(7),
        &[("multiply_by_denominator", i(28))],
    );
    // 50% числа дорівнюють 354, число 708.
    assert_task(
        "P-PCT",
        "Знижка становить 50% від ціни, і це 354 грн. Скільки гривень коштував товар без знижки?",
        i(708),
        &[
            ("percent_of_given", i(177)),
            ("multiply_by_percent", i(17700)),
        ],
    );
    assert_task(
        "P-PCT",
        "Знайди 20% від числа 770.",
        i(154),
        &[
            ("forgot_hundred", i(15400)),
            ("percent_as_denominator", frac(77, 2)),
        ],
    );
    // 179,6 : 100 = 1,796; кома не в той бік: 17960; на цифру менше: 17,96.
    assert_task(
        "D-MUL10",
        "Обчисли: 179,6 : 100",
        frac(1796, 1000),
        &[
            ("wrong_direction", i(17960)),
            ("one_place_short", frac(1796, 100)),
        ],
    );
    assert_task(
        "D-MUL10",
        "Обчисли: 936,2 × 1000",
        i(936_200),
        &[
            ("wrong_direction", frac(4681, 5000)),
            ("one_place_short", i(93_620)),
        ],
    );
    // (38 + 42 + 49) : 3 = 43.
    assert_task(
        "AVG",
        "Знайди середнє арифметичне чисел 38, 42 і 49.",
        i(43),
        &[
            ("forgot_divide", i(129)),
            ("divide_by_count_minus_one", frac(129, 2)),
        ],
    );
}

/// Для кожної нової навички перелік типових помилок збігається з задокументованим: нічого не зникло мовчки.
#[test]
fn new_skills_keep_their_documented_bug_ids() {
    let expected: [(&str, &[&str]); 24] = [
        ("N-ROUND", &["always_up", "cut_off"]),
        (
            "N-DIVREM",
            &[
                "quotient_instead",
                "remainder_instead",
                "remainder_too_big",
                "round_up_quotient",
            ],
        ),
        (
            "N-DIVTEST",
            &[
                "digit_sum_rule",
                "last_digit_rule",
                "only_zero_end",
                "rule_for_five",
                "rule_for_three",
            ],
        ),
        ("N-GCD", &["lcm_instead", "not_greatest"]),
        ("N-LCM", &["gcd_instead", "product"]),
        ("N-POWER", &["multiply_by_exponent"]),
        ("EQ-ADD-SUB", &["add_instead", "subtract_instead"]),
        (
            "EQ-MUL",
            &["divide_instead", "divisor_swapped", "multiply_instead"],
        ),
        (
            "WORD-1",
            &["add_instead", "multiply_instead", "subtract_instead"],
        ),
        (
            "WORD-2",
            &["add_all_numbers", "first_step_only", "wrong_operation"],
        ),
        (
            "MOTION",
            &[
                "add_instead",
                "divide_instead",
                "forgot_time",
                "multiply_instead",
                "one_object",
            ],
        ),
        ("U-LEN", &["add_only", "wrong_direction", "wrong_zeros"]),
        ("U-MASS", &["add_only", "wrong_direction", "wrong_zeros"]),
        ("U-TIME", &["add_only", "decimal_system", "wrong_direction"]),
        (
            "G-PERIM",
            &[
                "area_instead",
                "divide_by_two",
                "forgot_halving",
                "half_perimeter",
                "halve_only",
                "three_sides",
            ],
        ),
        (
            "G-AREA",
            &[
                "divide_by_two",
                "double_side",
                "multiply_instead",
                "perimeter_confusion",
                "perimeter_instead",
                "subtract_instead",
                "sum_sides",
            ],
        ),
        (
            "G-VOL",
            &[
                "edges_total",
                "forgot_height",
                "forgot_third_factor",
                "multiply_instead",
                "subtract_instead",
                "triple_edge",
            ],
        ),
        (
            "A-ANGLE",
            &[
                "forgot_one_angle",
                "full_turn",
                "use_full_turn",
                "use_right_angle",
                "use_straight_angle",
            ],
        ),
        (
            "F-MIXED",
            &[
                "digits_side_by_side",
                "forgot_to_add",
                "remainder_as_whole",
                "remainder_only",
                "round_up_whole",
                "whole_over_denominator",
            ],
        ),
        ("F-REDUCE", &["asked_other_part", "partial_reduce"]),
        (
            "F-OF",
            &[
                "divide_by_numerator",
                "forgot_numerator",
                "fraction_of_given",
                "multiply_by_denominator",
            ],
        ),
        (
            "P-PCT",
            &[
                "forgot_hundred",
                "multiply_by_percent",
                "percent_as_denominator",
                "percent_of_given",
            ],
        ),
        ("D-MUL10", &["one_place_short", "wrong_direction"]),
        ("AVG", &["divide_by_count_minus_one", "forgot_divide"]),
    ];
    for (skill, ids) in expected {
        let g = generator_for(skill).unwrap();
        let mut seen: Vec<&str> = (1..=3_000)
            .flat_map(|v| g.generate(v).bugs.iter().map(|b| b.id).collect::<Vec<_>>())
            .collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen, ids, "{skill}");
    }
}

/// Кожна помилка кожної нової навички має опис, ідею для учителя, а навичка має загальну ідею.
#[test]
fn every_new_bug_and_skill_has_sheet_text() {
    let sheet = generators_math::MathSheetContent;
    for skill in solver::NEW_SKILLS {
        let g = generator_for(skill).unwrap();
        assert!(
            sheet.idea_for_skill(skill).is_some(),
            "{skill}: немає ідеї для навички"
        );
        let mut bugs: Vec<&str> = (1..=3_000)
            .flat_map(|v| g.generate(v).bugs.iter().map(|b| b.id).collect::<Vec<_>>())
            .collect();
        bugs.sort_unstable();
        bugs.dedup();
        for bug in bugs {
            let d = sheet
                .describe_bug(skill, bug)
                .unwrap_or_else(|| panic!("{skill}/{bug}: немає опису"));
            assert!(
                d.starts_with("Може "),
                "{skill}/{bug}: нейтральний тон «Може …»: {d}"
            );
            assert!(
                sheet.idea_for_bug(skill, bug).is_some(),
                "{skill}/{bug}: немає ідеї"
            );
        }
    }
}

/// Незалежний розв'язувач (він же перераховує уроки) збігається з відповіддю генератора на розбірних умовах.
#[test]
fn independent_solver_agrees_with_generators() {
    for skill in solver::NEW_SKILLS {
        let g = generator_for(skill).unwrap();
        let (mut solved, mut total) = (0, 0);
        for v in 1..=1_500 {
            let t = g.generate(v);
            total += 1;
            if matches!(skill, "WORD-1" | "WORD-2" | "MOTION") {
                continue;
            }
            if let Some(got) = solver::solve(skill, &t.statement) {
                solved += 1;
                assert_eq!(got, t.answer, "{skill} #{v}: {}", t.statement);
            }
        }
        if !matches!(skill, "WORD-1" | "WORD-2" | "MOTION") {
            // Історії (відсотки, дроби, величини) розв'язувач не розбирає, але чистих умов має бути багато.
            assert!(
                solved * 3 >= total,
                "{skill}: розв'язано лише {solved} з {total}"
            );
        }
    }
}

/// Змістові властивості: діапазони й форми відповідей.
#[test]
fn new_skills_semantic_properties() {
    let g = |s| generator_for(s).unwrap();
    for v in 1..=2_000 {
        let t = g("N-DIVREM").generate(v);
        let (a, b) = (t.params[1], t.params[2]);
        let (q, r) = (a / b, a % b);
        assert!(r > 0 && r < b);
        assert_eq!(
            t.answer,
            i(if t.params[0] == 0 { r } else { q }),
            "{}",
            t.statement
        );

        let t = g("N-GCD").generate(v);
        let (a, b) = (t.params[0], t.params[1]);
        assert!(t.answer > i(1) && a % t.answer.num() == 0 && b % t.answer.num() == 0);

        let t = g("N-LCM").generate(v);
        let (a, b) = (t.params[0], t.params[1]);
        assert!(t.answer.num() % a == 0 && t.answer.num() % b == 0 && t.answer.num() < a * b);

        let t = g("N-ROUND").generate(v);
        let (p, n) = (t.params[0], t.params[1]);
        assert_eq!(t.answer.num() % p, 0);
        assert!((t.answer.num() - n).abs() * 2 <= p);

        let t = g("F-MIXED").generate(v);
        assert!(t.answer.den() <= 12 || t.answer.is_integer());

        let t = g("AVG").generate(v);
        assert!(t.answer.is_integer());
        assert!(
            t.operands.iter().min().unwrap() < &t.answer
                && &t.answer < t.operands.iter().max().unwrap()
        );

        let t = g("A-ANGLE").generate(v);
        assert!(t.answer > i(0) && t.answer < i(180));
    }
}

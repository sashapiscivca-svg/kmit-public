//! Роботи для вчителя: бланк завдань, ключ і бланк відповідей із клітинками (PDF для друку). Дані учнів тут не
//! використовуються: лише згенеровані завдання. Лише латинські назви шрифтів із `canvas`.

use crate::canvas::{Align, Canvas, FontId, Rgb, A4};
use crate::pdf::PdfError;

const MARGIN: f32 = 44.0;
const BOTTOM: f32 = 56.0;
const INK: Rgb = Rgb(27, 42, 107);
const SOFT: Rgb = Rgb(110, 120, 160);
const LINE: Rgb = Rgb(150, 160, 190);
const LETTERS: [char; 6] = ['А', 'Б', 'В', 'Г', 'Д', 'Е'];

#[derive(Default)]
pub struct ItemDoc {
    pub text: String,
    /// Варіанти відповіді (порожньо: учень пише відповідь сам).
    pub choices: Vec<String>,
    pub answer: String,
}

#[derive(Default)]
pub struct VariantDoc {
    pub label: char,
    pub items: Vec<ItemDoc>,
}

#[derive(Default)]
pub struct WorkDoc {
    /// Масштаб шрифту й інтервалів для друку великим шрифтом (1.0 звичайний).
    pub scale: f32,
    /// Словниковий диктант: учневі друкуються лише номери з рядками, умови читає вчитель (вони в ключі).
    pub dictation: bool,
    pub title: String,
    /// Теми через кому (для колонтитула).
    pub subtitle: String,
    pub grade: u8,
    pub seed: u64,
    pub variants: Vec<VariantDoc>,
}

fn check(d: &WorkDoc) -> Result<(), PdfError> {
    if d.variants.is_empty() || d.variants.iter().any(|v| v.items.is_empty()) {
        return Err(PdfError::Empty);
    }
    Ok(())
}

/// Літера правильного варіанта в завданні з вибором.
fn key_letter(it: &ItemDoc) -> Option<char> {
    it.choices
        .iter()
        .position(|c| c.trim() == it.answer.trim())
        .and_then(|i| LETTERS.get(i).copied())
}

struct Page<'c, 'a> {
    c: &'c mut Canvas<'a>,
    y: f32,
}

impl Page<'_, '_> {
    fn ensure(&mut self, need: f32, head: &dyn Fn(&mut Canvas<'_>) -> f32) {
        if self.y - need < BOTTOM {
            self.c.new_page();
            self.y = head(self.c);
        }
    }
}

fn header(c: &mut Canvas<'_>, d: &WorkDoc, v: &VariantDoc, what: &str, with_blanks: bool) -> f32 {
    let w = c.width;
    let top = c.height - MARGIN;
    c.text(
        FontId::Bold,
        17.0,
        MARGIN,
        top - 14.0,
        Align::Left,
        INK,
        &format!("{} · {what}", d.title),
    );
    c.text(
        FontId::Bold,
        15.0,
        w - MARGIN,
        top - 14.0,
        Align::Right,
        INK,
        &format!("Варіант {}", v.label),
    );
    let sub = format!("{} клас · {}", d.grade, d.subtitle);
    let mut sub_fit = sub.clone();
    while sub_fit.chars().count() > 3
        && c.text_width(FontId::Body, 10.0, &sub_fit) > w - 2.0 * MARGIN
    {
        sub_fit.pop();
    }
    c.text(
        FontId::Body,
        10.0,
        MARGIN,
        top - 30.0,
        Align::Left,
        SOFT,
        &sub_fit,
    );
    let mut y = top - 46.0;
    if with_blanks {
        c.text(
            FontId::Body,
            11.0,
            MARGIN,
            y - 10.0,
            Align::Left,
            INK,
            "Ім'я та прізвище:",
        );
        c.line(
            MARGIN + 100.0,
            y - 12.0,
            w - MARGIN - 150.0,
            y - 12.0,
            LINE,
            0.8,
            false,
        );
        c.text(
            FontId::Body,
            11.0,
            w - MARGIN - 140.0,
            y - 10.0,
            Align::Left,
            INK,
            "Дата:",
        );
        c.line(
            w - MARGIN - 105.0,
            y - 12.0,
            w - MARGIN,
            y - 12.0,
            LINE,
            0.8,
            false,
        );
        y -= 26.0;
    }
    c.line(MARGIN, y, w - MARGIN, y, INK, 1.2, false);
    y - 16.0
}

/// Бланк завдань: кожен варіант з нової сторінки. Великий шрифт збільшує текст і проміжки; диктант друкує лише рядки.
pub fn work_tasks_pdf(d: &WorkDoc) -> Result<Vec<u8>, PdfError> {
    check(d)?;
    let k = if d.scale >= 1.0 {
        d.scale.min(1.8)
    } else {
        1.0
    };
    let mut c = Canvas::new(A4)?;
    for (vi, v) in d.variants.iter().enumerate() {
        if vi > 0 {
            c.new_page();
        }
        let head = |c: &mut Canvas<'_>| header(c, d, v, "завдання", false);
        let first = header(&mut c, d, v, "завдання", true);
        let mut p = Page {
            c: &mut c,
            y: first,
        };
        let text_w = A4.0 - 2.0 * MARGIN - 26.0 * k;
        for (i, it) in v.items.iter().enumerate() {
            if d.dictation {
                p.ensure(34.0 * k, &head);
                p.c.text(
                    FontId::Bold,
                    12.0 * k,
                    MARGIN,
                    p.y - 14.0 * k,
                    Align::Left,
                    INK,
                    &format!("{}.", i + 1),
                );
                p.c.line(
                    MARGIN + 30.0 * k,
                    p.y - 16.0 * k,
                    A4.0 - MARGIN,
                    p.y - 16.0 * k,
                    LINE,
                    0.8,
                    false,
                );
                p.y -= 34.0 * k;
                continue;
            }
            let lines = p.c.lines_needed(FontId::Body, 12.0 * k, text_w, &it.text);
            let mut need = lines as f32 * 15.5 * k + 22.0 * k;
            for ch in &it.choices {
                need +=
                    p.c.lines_needed(FontId::Body, 11.5 * k, text_w - 20.0 * k, ch) as f32
                        * 14.5
                        * k
                        + 2.0;
            }
            p.ensure(need, &head);
            p.c.text(
                FontId::Bold,
                12.0 * k,
                MARGIN,
                p.y - 12.0 * k,
                Align::Left,
                INK,
                &format!("{}.", i + 1),
            );
            let n = p.c.paragraph(
                FontId::Body,
                12.0 * k,
                MARGIN + 26.0 * k,
                p.y,
                text_w,
                1.3,
                Align::Left,
                INK,
                &it.text,
            );
            p.y -= n as f32 * 15.5 * k + 4.0;
            if it.choices.is_empty() {
                p.c.text(
                    FontId::Body,
                    11.0 * k,
                    MARGIN + 26.0 * k,
                    p.y - 9.0 * k,
                    Align::Left,
                    SOFT,
                    "Відповідь:",
                );
                p.c.line(
                    MARGIN + 90.0 * k,
                    p.y - 11.0 * k,
                    A4.0 - MARGIN - 20.0,
                    p.y - 11.0 * k,
                    LINE,
                    0.8,
                    true,
                );
                p.y -= 22.0 * k;
            } else {
                for (kk, ch) in it.choices.iter().enumerate().take(6) {
                    let n = p.c.paragraph(
                        FontId::Body,
                        11.5 * k,
                        MARGIN + 46.0 * k,
                        p.y,
                        text_w - 20.0 * k,
                        1.25,
                        Align::Left,
                        INK,
                        ch,
                    );
                    p.c.text(
                        FontId::Bold,
                        11.5 * k,
                        MARGIN + 28.0 * k,
                        p.y - 11.5 * k,
                        Align::Left,
                        INK,
                        &format!("{})", LETTERS[kk]),
                    );
                    p.y -= n as f32 * 14.5 * k + 2.0;
                }
                p.y -= 8.0 * k;
            }
        }
        footer(&mut c, d);
    }
    c.finish(&d.title)
}

/// Картки для розрізання: по 8 карток на аркуші (2×4), кожне завдання в рамці, пунктир для різання.
pub fn work_cards_pdf(d: &WorkDoc) -> Result<Vec<u8>, PdfError> {
    check(d)?;
    let mut c = Canvas::new(A4)?;
    let (cols, rows) = (2usize, 4usize);
    let cw = (A4.0 - 2.0 * MARGIN) / cols as f32;
    let ch = (A4.1 - 2.0 * MARGIN - 20.0) / rows as f32;
    let mut first_page = true;
    for v in &d.variants {
        for (page_start, chunk) in v.items.chunks(cols * rows).enumerate() {
            if !first_page {
                c.new_page();
            }
            first_page = false;
            let _ = page_start;
            for (idx, it) in chunk.iter().enumerate() {
                let (col, row) = (idx % cols, idx / cols);
                let x = MARGIN + col as f32 * cw;
                let y_top = A4.1 - MARGIN - row as f32 * ch;
                c.rect(
                    x + 3.0,
                    y_top - ch + 3.0,
                    cw - 6.0,
                    ch - 6.0,
                    6.0,
                    None,
                    Some((LINE, 0.8)),
                );
                let n = v
                    .items
                    .iter()
                    .position(|x| std::ptr::eq(x, it))
                    .map_or(0, |p| p + 1);
                c.text(
                    FontId::Bold,
                    10.0,
                    x + 10.0,
                    y_top - 18.0,
                    Align::Left,
                    INK,
                    &format!("{} · {n}", v.label),
                );
                let mut size = 11.0;
                while size > 8.0
                    && c.lines_needed(FontId::Body, size, cw - 28.0, &it.text) as f32 * size * 1.3
                        > ch * 0.5
                {
                    size -= 0.5;
                }
                let lines = c.paragraph(
                    FontId::Body,
                    size,
                    x + 10.0,
                    y_top - 24.0,
                    cw - 28.0,
                    1.3,
                    Align::Left,
                    INK,
                    &it.text,
                );
                let mut y = y_top - 24.0 - lines as f32 * size * 1.3 - 4.0;
                for (kk, choice) in it.choices.iter().enumerate().take(6) {
                    let mut cs = (size - 1.0).max(7.5);
                    while cs > 7.5 && c.lines_needed(FontId::Body, cs, cw - 44.0, choice) > 2 {
                        cs -= 0.5;
                    }
                    c.text(
                        FontId::Bold,
                        cs,
                        x + 12.0,
                        y - cs,
                        Align::Left,
                        INK,
                        &format!("{})", LETTERS[kk]),
                    );
                    let l = c.paragraph(
                        FontId::Body,
                        cs,
                        x + 28.0,
                        y,
                        cw - 44.0,
                        1.2,
                        Align::Left,
                        INK,
                        choice,
                    );
                    y -= l as f32 * cs * 1.2 + 2.0;
                }
                if it.choices.is_empty() {
                    c.line(
                        x + 12.0,
                        y_top - ch + 16.0,
                        x + cw - 14.0,
                        y_top - ch + 16.0,
                        LINE,
                        0.8,
                        true,
                    );
                }
            }
            footer(&mut c, d);
        }
    }
    c.finish(&format!("{}: картки", d.title))
}

fn footer(c: &mut Canvas<'_>, d: &WorkDoc) {
    let w = c.width;
    c.text(
        FontId::Body,
        8.0,
        w - MARGIN,
        28.0,
        Align::Right,
        SOFT,
        &format!("Kmit · зерно {}", d.seed),
    );
}

/// Ключ відповідей (лише для вчителя): по варіантах.
pub fn work_key_pdf(d: &WorkDoc) -> Result<Vec<u8>, PdfError> {
    check(d)?;
    let mut c = Canvas::new(A4)?;
    for (vi, v) in d.variants.iter().enumerate() {
        if vi > 0 {
            c.new_page();
        }
        let head = |c: &mut Canvas<'_>| header(c, d, v, "ключ (лише для вчителя)", false);
        let first = header(&mut c, d, v, "ключ (лише для вчителя)", false);
        let mut p = Page {
            c: &mut c,
            y: first,
        };
        for (i, it) in v.items.iter().enumerate() {
            let shown = match key_letter(it) {
                Some(l) => format!("{l}) {}", it.answer),
                None if d.dictation => format!("{} → {}", it.text, it.answer),
                None => it.answer.clone(),
            };
            let n =
                p.c.lines_needed(FontId::Body, 12.0, A4.0 - 2.0 * MARGIN - 30.0, &shown);
            p.ensure(n as f32 * 15.0 + 8.0, &head);
            p.c.text(
                FontId::Bold,
                12.0,
                MARGIN,
                p.y - 12.0,
                Align::Left,
                INK,
                &format!("{}.", i + 1),
            );
            p.c.paragraph(
                FontId::Body,
                12.0,
                MARGIN + 30.0,
                p.y,
                A4.0 - 2.0 * MARGIN - 30.0,
                1.25,
                Align::Left,
                INK,
                &shown,
            );
            p.y -= n as f32 * 15.0 + 6.0;
        }
        footer(&mut c, d);
    }
    c.finish(&format!("{}: ключ", d.title))
}

/// Бланк відповідей із клітинками: літери для тестів, клітинки для чисел і слів.
pub fn work_answers_pdf(d: &WorkDoc) -> Result<Vec<u8>, PdfError> {
    check(d)?;
    let mut c = Canvas::new(A4)?;
    for (vi, v) in d.variants.iter().enumerate() {
        if vi > 0 {
            c.new_page();
        }
        let head = |c: &mut Canvas<'_>| header(c, d, v, "бланк відповідей", true);
        let first = header(&mut c, d, v, "бланк відповідей", true);
        let mut p = Page {
            c: &mut c,
            y: first,
        };
        for (i, it) in v.items.iter().enumerate() {
            p.ensure(40.0, &head);
            let y = p.y;
            p.c.text(
                FontId::Bold,
                12.0,
                MARGIN,
                y - 22.0,
                Align::Left,
                INK,
                &format!("{}.", i + 1),
            );
            if it.choices.is_empty() {
                // 12 клітинок: число чи слово (кома й риска дробу займають власні клітинки).
                for k in 0..12 {
                    let x = MARGIN + 30.0 + k as f32 * 26.0;
                    p.c.rect(x, y - 30.0, 24.0, 28.0, 2.0, None, Some((INK, 0.9)));
                }
            } else {
                for (k, l) in LETTERS.iter().enumerate().take(it.choices.len().min(6)) {
                    let x = MARGIN + 30.0 + k as f32 * 40.0;
                    p.c.rect(x, y - 30.0, 34.0, 28.0, 4.0, None, Some((INK, 0.9)));
                    p.c.text(
                        FontId::Body,
                        9.0,
                        x + 17.0,
                        y - 40.0,
                        Align::Center,
                        SOFT,
                        &l.to_string(),
                    );
                }
            }
            p.y -= 54.0;
        }
        footer(&mut c, d);
    }
    c.finish(&format!("{}: бланк відповідей", d.title))
}

/// Розділ плану уроку: заголовок і абзаци (маркер «• » на початку рядка малюється як пункт).
pub struct PlanSection {
    pub heading: String,
    pub lines: Vec<String>,
}

pub struct PlanDoc {
    pub title: String,
    pub subtitle: String,
    pub sections: Vec<PlanSection>,
    /// Завдання на закріплення: (умова, відповідь для вчителя).
    pub practice: Vec<(String, String)>,
}

/// План уроку на 45 хвилин (A4, для друку вчителем).
pub fn lesson_plan_pdf(d: &PlanDoc) -> Result<Vec<u8>, PdfError> {
    if d.sections.is_empty() {
        return Err(PdfError::Empty);
    }
    let mut c = Canvas::new(A4)?;
    let w = A4.0 - 2.0 * MARGIN;
    let head = |c: &mut Canvas<'_>| {
        let top = c.height - MARGIN;
        c.text(
            FontId::Body,
            9.0,
            MARGIN,
            top - 4.0,
            Align::Left,
            SOFT,
            "Kmit · план уроку (продовження)",
        );
        c.line(
            MARGIN,
            top - 10.0,
            c.width - MARGIN,
            top - 10.0,
            LINE,
            0.6,
            false,
        );
        top - 24.0
    };
    let top = c.height - MARGIN;
    c.text(
        FontId::Bold,
        20.0,
        MARGIN,
        top - 18.0,
        Align::Left,
        INK,
        &d.title,
    );
    c.text(
        FontId::Body,
        11.0,
        MARGIN,
        top - 36.0,
        Align::Left,
        SOFT,
        &d.subtitle,
    );
    c.line(
        MARGIN,
        top - 44.0,
        c.width - MARGIN,
        top - 44.0,
        INK,
        1.2,
        false,
    );
    let mut p = Page {
        c: &mut c,
        y: top - 62.0,
    };
    for s in &d.sections {
        p.ensure(60.0, &head);
        p.c.text(
            FontId::Bold,
            13.5,
            MARGIN,
            p.y - 13.0,
            Align::Left,
            INK,
            &s.heading,
        );
        p.y -= 24.0;
        for l in &s.lines {
            let (bullet, text) = match l.strip_prefix("• ") {
                Some(t) => (true, t),
                None => (false, l.as_str()),
            };
            let x = if bullet { MARGIN + 14.0 } else { MARGIN };
            let n = p.c.lines_needed(FontId::Body, 11.0, w - (x - MARGIN), text);
            p.ensure(n as f32 * 14.0 + 6.0, &head);
            if bullet {
                p.c.text(
                    FontId::Bold,
                    11.0,
                    MARGIN + 2.0,
                    p.y - 11.0,
                    Align::Left,
                    INK,
                    "•",
                );
            }
            p.c.paragraph(
                FontId::Body,
                11.0,
                x,
                p.y,
                w - (x - MARGIN),
                1.25,
                Align::Left,
                INK,
                text,
            );
            p.y -= n as f32 * 14.0 + 5.0;
        }
        p.y -= 8.0;
    }
    if !d.practice.is_empty() {
        p.ensure(60.0, &head);
        p.c.text(
            FontId::Bold,
            13.5,
            MARGIN,
            p.y - 13.0,
            Align::Left,
            INK,
            "Завдання на закріплення",
        );
        p.y -= 24.0;
        for (i, (q, a)) in d.practice.iter().enumerate() {
            let n = p.c.lines_needed(FontId::Body, 11.0, w - 24.0, q);
            p.ensure(n as f32 * 14.0 + 22.0, &head);
            p.c.text(
                FontId::Bold,
                11.0,
                MARGIN,
                p.y - 11.0,
                Align::Left,
                INK,
                &format!("{}.", i + 1),
            );
            p.c.paragraph(
                FontId::Body,
                11.0,
                MARGIN + 22.0,
                p.y,
                w - 24.0,
                1.25,
                Align::Left,
                INK,
                q,
            );
            p.y -= n as f32 * 14.0 + 3.0;
            p.c.text(
                FontId::Body,
                10.0,
                MARGIN + 22.0,
                p.y - 10.0,
                Align::Left,
                SOFT,
                &format!("Відповідь: {a}"),
            );
            p.y -= 16.0;
        }
    }
    footer_plain(&mut c);
    c.finish(&d.title)
}

fn footer_plain(c: &mut Canvas<'_>) {
    let w = c.width;
    c.text(
        FontId::Body,
        8.0,
        w - MARGIN,
        28.0,
        Align::Right,
        SOFT,
        "Kmit · кабінет вчителя",
    );
}

/// Рядок тематичного плану.
pub struct TopicRow {
    pub title: String,
    pub lessons: u8,
}

pub struct TopicPlanDoc {
    pub title: String,
    pub subtitle: String,
    pub rows: Vec<TopicRow>,
}

/// Тематичний план на семестр: порядок тем курсу, кількість уроків, місця для дат і приміток.
pub fn topic_plan_pdf(d: &TopicPlanDoc) -> Result<Vec<u8>, PdfError> {
    if d.rows.is_empty() {
        return Err(PdfError::Empty);
    }
    let mut c = Canvas::new(A4)?;
    let w = A4.0 - 2.0 * MARGIN;
    let (x_n, x_t, x_l, x_d) = (
        MARGIN,
        MARGIN + 28.0,
        MARGIN + w - 190.0,
        MARGIN + w - 150.0,
    );
    let head = |c: &mut Canvas<'_>| {
        let top = c.height - MARGIN;
        c.text(
            FontId::Body,
            9.0,
            MARGIN,
            top - 4.0,
            Align::Left,
            SOFT,
            "Kmit · тематичний план (продовження)",
        );
        c.line(
            MARGIN,
            top - 10.0,
            c.width - MARGIN,
            top - 10.0,
            LINE,
            0.6,
            false,
        );
        top - 24.0
    };
    let top = c.height - MARGIN;
    c.text(
        FontId::Bold,
        20.0,
        MARGIN,
        top - 18.0,
        Align::Left,
        INK,
        &d.title,
    );
    c.text(
        FontId::Body,
        11.0,
        MARGIN,
        top - 36.0,
        Align::Left,
        SOFT,
        &d.subtitle,
    );
    c.line(
        MARGIN,
        top - 44.0,
        c.width - MARGIN,
        top - 44.0,
        INK,
        1.2,
        false,
    );
    let mut p = Page {
        c: &mut c,
        y: top - 60.0,
    };
    let table_head = |p: &mut Page<'_, '_>| {
        p.c.text(FontId::Bold, 10.0, x_n, p.y - 10.0, Align::Left, SOFT, "№");
        p.c.text(
            FontId::Bold,
            10.0,
            x_t,
            p.y - 10.0,
            Align::Left,
            SOFT,
            "Тема",
        );
        p.c.text(
            FontId::Bold,
            10.0,
            x_l,
            p.y - 10.0,
            Align::Left,
            SOFT,
            "Уроків",
        );
        p.c.text(
            FontId::Bold,
            10.0,
            x_d,
            p.y - 10.0,
            Align::Left,
            SOFT,
            "Дата й примітки",
        );
        p.y -= 18.0;
    };
    table_head(&mut p);
    let mut total = 0u32;
    for (i, r) in d.rows.iter().enumerate() {
        let lines =
            p.c.lines_needed(FontId::Body, 11.0, x_l - x_t - 8.0, &r.title);
        let h = (lines as f32 * 14.0 + 14.0).max(30.0);
        if p.y - h < BOTTOM {
            p.c.new_page();
            p.y = head(p.c);
            table_head(&mut p);
        }
        p.c.text(
            FontId::Bold,
            11.0,
            x_n,
            p.y - 12.0,
            Align::Left,
            INK,
            &format!("{}", i + 1),
        );
        p.c.paragraph(
            FontId::Body,
            11.0,
            x_t,
            p.y - 1.0,
            x_l - x_t - 8.0,
            1.25,
            Align::Left,
            INK,
            &r.title,
        );
        p.c.text(
            FontId::Body,
            11.0,
            x_l + 10.0,
            p.y - 12.0,
            Align::Left,
            INK,
            &r.lessons.to_string(),
        );
        p.c.line(
            x_d,
            p.y - h + 6.0,
            MARGIN + w,
            p.y - h + 6.0,
            LINE,
            0.7,
            true,
        );
        total += u32::from(r.lessons);
        p.y -= h;
    }
    p.ensure(30.0, &head);
    p.c.text(
        FontId::Bold,
        11.0,
        MARGIN,
        p.y - 16.0,
        Align::Left,
        INK,
        &format!("Усього: тем {}, уроків {total}", d.rows.len()),
    );
    footer_plain(&mut c);
    c.finish(&d.title)
}

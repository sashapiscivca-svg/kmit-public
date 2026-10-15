//! Оформлені PDF: аркуш для вчителя й листівка подяки. Кольорова шапка за предметом, картки з кольоровими смужками,
//! плитки підсумків, піксельний герой дитини. Тон лишається нейтральним: це підказка, а не оцінка.

use crate::canvas::{Align, Canvas, FontId, Rgb, A4, A4_LANDSCAPE};
use crate::pdf::PdfError;
use tutor_core::hero::Px;
use tutor_core::sheet::TeacherSheet;

const INK: Rgb = Rgb(27, 42, 107);
const SOFT: Rgb = Rgb(74, 90, 150);
const PAPER: Rgb = Rgb(243, 247, 251);
const MARGIN: f32 = 40.0;
const BOTTOM: f32 = 54.0;

fn on(accent: Rgb) -> Rgb {
    // Текст на кольоровій шапці: темний на світлих кольорах, світлий на темних.
    let l = 0.299 * f32::from(accent.0) + 0.587 * f32::from(accent.1) + 0.114 * f32::from(accent.2);
    if l > 150.0 {
        INK
    } else {
        Rgb(255, 255, 255)
    }
}

/// Піксельний герой у прямокутнику з лівим нижнім кутом `(x, y)` і розміром пікселя `px`.
fn draw_hero(c: &mut Canvas, hero: &[Px], x: f32, y: f32, px: f32) {
    for p in hero {
        c.rect(
            x + f32::from(p.x) * px,
            y + (20.0 - f32::from(p.y) - f32::from(p.h)) * px,
            f32::from(p.w) * px,
            f32::from(p.h) * px,
            0.0,
            Some(Rgb::hex(&p.color)),
            None,
        );
    }
}

enum Item {
    Main(String),
    Sub(String),
}

struct Cursor<'a, 'b> {
    c: &'b mut Canvas<'a>,
    y: f32,
    accent: Rgb,
    title: String,
    subject: String,
}

impl<'a, 'b> Cursor<'a, 'b> {
    fn header_small(&mut self) {
        let h = 38.0;
        let w = self.c.width;
        let top = self.c.height;
        self.c
            .rect(0.0, top - h, w, h, 0.0, Some(self.accent), None);
        let fg = on(self.accent);
        let t = format!("{}: {}", self.title, self.subject);
        self.c
            .text(FontId::Bold, 13.0, MARGIN, top - 24.0, Align::Left, fg, &t);
        self.y = top - h - 18.0;
    }

    fn page_break(&mut self) {
        self.c.new_page();
        let w = self.c.width;
        let h = self.c.height;
        self.c.rect(0.0, 0.0, w, h, 0.0, Some(PAPER), None);
        self.header_small();
    }

    /// Картка з кольоровою смужкою: заголовок і пункти; переносить на нову сторінку, коли не вміщається.
    fn card(&mut self, title: &str, color: Rgb, items: &[Item]) {
        let w = self.c.width - 2.0 * MARGIN;
        let size = 10.5;
        let lead = 1.38;
        let inner = w - 34.0;
        let mut idx = 0;
        let mut first = true;
        while idx < items.len() || first {
            let head_h = 30.0;
            // Скільки пунктів вміщається на цій сторінці.
            let mut used = head_h + 10.0;
            let mut end = idx;
            while end < items.len() {
                let (txt, indent) = match &items[end] {
                    Item::Main(t) => (t, 14.0),
                    Item::Sub(t) => (t, 28.0),
                };
                let n = self.c.lines_needed(FontId::Body, size, inner - indent, txt);
                let h = n as f32 * size * lead + 4.0;
                if self.y - (used + h) < BOTTOM && end > idx {
                    break;
                }
                used += h;
                end += 1;
                if self.y - used < BOTTOM {
                    break;
                }
            }
            if self.y - used < BOTTOM && !first {
                self.page_break();
                continue;
            }
            if self.y - used < BOTTOM {
                self.page_break();
            }
            let top = self.y;
            let card_h = used + 6.0;
            self.c.rect(
                MARGIN,
                top - card_h,
                w,
                card_h,
                10.0,
                Some(Rgb(255, 255, 255)),
                Some((INK, 1.4)),
            );
            self.c.rect(
                MARGIN,
                top - card_h,
                9.0,
                card_h,
                4.0,
                Some(color),
                Some((INK, 1.4)),
            );
            let label = if first {
                title.to_string()
            } else {
                format!("{title} (продовження)")
            };
            self.c.text(
                FontId::Bold,
                12.5,
                MARGIN + 22.0,
                top - 20.0,
                Align::Left,
                INK,
                &label,
            );
            let mut y = top - head_h - 6.0;
            if items.is_empty() {
                self.c.text(
                    FontId::Body,
                    size,
                    MARGIN + 22.0,
                    y - size,
                    Align::Left,
                    SOFT,
                    "поки немає",
                );
                y -= size * lead;
            }
            for it in &items[idx..end] {
                let (txt, indent, dot) = match it {
                    Item::Main(t) => (t, 14.0, true),
                    Item::Sub(t) => (t, 28.0, false),
                };
                let n = self.c.paragraph(
                    FontId::Body,
                    size,
                    MARGIN + 22.0 + indent,
                    y,
                    inner - indent,
                    lead,
                    Align::Left,
                    INK,
                    txt,
                );
                if dot {
                    self.c.rect(
                        MARGIN + 22.0 + indent - 11.0,
                        y - size + 1.0,
                        4.5,
                        4.5,
                        2.2,
                        Some(color),
                        None,
                    );
                } else {
                    self.c.rect(
                        MARGIN + 22.0 + indent - 10.0,
                        y - size + 2.0,
                        3.0,
                        3.0,
                        0.0,
                        Some(SOFT),
                        None,
                    );
                }
                y -= n as f32 * size * lead + 4.0;
            }
            self.y = top - card_h - 12.0;
            idx = end;
            first = false;
            if idx >= items.len() {
                break;
            }
            self.page_break();
        }
    }
}

/// Аркуш для вчителя одного предмета. `accent` це колір предмета (`#rrggbb`), `hero` піксельний герой дитини.
pub fn styled_sheet_pdf(s: &TeacherSheet, accent: &str, hero: &[Px]) -> Result<Vec<u8>, PdfError> {
    let acc = Rgb::hex(accent);
    let mut c = Canvas::new(A4)?;
    let (w, h) = (c.width, c.height);
    c.rect(0.0, 0.0, w, h, 0.0, Some(PAPER), None);
    // Шапка.
    let band = 150.0;
    c.rect(0.0, h - band, w, band, 0.0, Some(acc), None);
    c.rect(0.0, h - band - 4.0, w, 4.0, 0.0, Some(INK), None);
    let fg = on(acc);
    c.text(
        FontId::Body,
        11.0,
        MARGIN,
        h - 40.0,
        Align::Left,
        fg,
        "АРКУШ ДЛЯ ВЧИТЕЛЯ",
    );
    let subject = if s.subject.is_empty() {
        "Усі предмети"
    } else {
        s.subject.as_str()
    };
    c.text(
        FontId::Hand,
        46.0,
        MARGIN,
        h - 92.0,
        Align::Left,
        fg,
        subject,
    );
    let who = if s.nickname.is_empty() {
        "Учень або учениця"
    } else {
        s.nickname.as_str()
    };
    c.text(FontId::Bold, 15.0, MARGIN, h - 120.0, Align::Left, fg, who);
    c.text(
        FontId::Body,
        10.0,
        MARGIN,
        h - 138.0,
        Align::Left,
        fg,
        "Це підказка, а не оцінка. Дитина показує аркуш сама, коли захоче.",
    );
    if !hero.is_empty() {
        let px = 5.0;
        c.rect(
            w - MARGIN - 16.0 * px - 8.0,
            h - band + 14.0,
            16.0 * px + 16.0,
            20.0 * px + 16.0,
            8.0,
            Some(Rgb(255, 255, 255)),
            Some((INK, 2.0)),
        );
        draw_hero(&mut c, hero, w - MARGIN - 16.0 * px, h - band + 22.0, px);
    }
    // Плитки підсумків.
    let tiles = [
        ("Опановано", s.mastered.len(), Rgb::hex("#d9f3e8")),
        ("Працюємо", s.in_progress.len(), Rgb::hex("#dfe7ff")),
        ("Пора повторити", s.to_repeat.len(), Rgb::hex("#fff3c2")),
        ("Складнощі", s.gaps.len(), Rgb::hex("#fde6e3")),
    ];
    let tw = (w - 2.0 * MARGIN - 3.0 * 10.0) / 4.0;
    let ty = h - band - 4.0 - 24.0 - 52.0;
    for (i, (label, n, col)) in tiles.iter().enumerate() {
        let x = MARGIN + i as f32 * (tw + 10.0);
        c.rect(x, ty, tw, 52.0, 10.0, Some(*col), Some((INK, 1.4)));
        c.text(
            FontId::Bold,
            22.0,
            x + tw / 2.0,
            ty + 26.0,
            Align::Center,
            INK,
            &n.to_string(),
        );
        c.text(
            FontId::Body,
            9.5,
            x + tw / 2.0,
            ty + 10.0,
            Align::Center,
            SOFT,
            label,
        );
    }
    let mut cur = Cursor {
        c: &mut c,
        y: ty - 20.0,
        accent: acc,
        title: "Аркуш для вчителя".into(),
        subject: subject.to_string(),
    };
    let main = |v: Vec<String>| v.into_iter().map(Item::Main).collect::<Vec<_>>();
    cur.card(
        "Що вже опановано",
        Rgb::hex("#13875f"),
        &main(s.mastered.iter().map(|x| x.title.clone()).collect()),
    );
    if !s.to_repeat.is_empty() {
        cur.card(
            "Що пора повторити",
            Rgb::hex("#e0a800"),
            &main(s.to_repeat.iter().map(|x| x.title.clone()).collect()),
        );
    }
    cur.card(
        "Над чим працюємо зараз",
        Rgb::hex("#2b4bd6"),
        &main(s.in_progress.iter().map(|x| x.title.clone()).collect()),
    );
    let mut gaps = Vec::new();
    for g in &s.gaps {
        gaps.push(Item::Main(g.title.clone()));
        for hy in &g.hypotheses {
            gaps.push(Item::Sub(format!(
                "Можлива причина: {}.",
                hy.trim_end_matches('.')
            )));
        }
        for i in &g.ideas {
            gaps.push(Item::Sub(format!("Ідея на 10 хвилин: {i}")));
        }
    }
    cur.card(
        "Над чим варто попрацювати разом",
        Rgb::hex("#d93a2f"),
        &gaps,
    );
    let ask: Vec<Item> = s
        .ask_adult
        .iter()
        .map(|a| Item::Main(format!("{}: {}", a.title, a.statement)))
        .collect();
    cur.card("Для обговорення з дорослим", Rgb::hex("#7b4fc9"), &ask);
    // Підвал (на останній сторінці).
    c.text(
        FontId::Hand,
        20.0,
        w / 2.0,
        26.0,
        Align::Center,
        INK,
        "Дякуємо вам за роботу з дітьми",
    );
    c.text(
        FontId::Body,
        8.5,
        w / 2.0,
        12.0,
        Align::Center,
        SOFT,
        "Kmit · лише теми й складнощі, без даних про особу",
    );
    c.finish("Аркуш для вчителя")
}

/// Листівка подяки вчителеві: A4 альбомна, дитина пише текст сама.
pub fn thanks_card_pdf(to: &str, text: &str, from: &str, hero: &[Px]) -> Result<Vec<u8>, PdfError> {
    let mut c = Canvas::new(A4_LANDSCAPE)?;
    let (w, h) = (c.width, c.height);
    c.rect(0.0, 0.0, w, h, 0.0, Some(Rgb::hex("#fff8e1")), None);
    c.rect(
        22.0,
        22.0,
        w - 44.0,
        h - 44.0,
        22.0,
        None,
        Some((Rgb::hex("#d93a2f"), 4.0)),
    );
    c.rect(
        34.0,
        34.0,
        w - 68.0,
        h - 68.0,
        16.0,
        None,
        Some((Rgb::hex("#ffd23f"), 2.5)),
    );
    // Піксельні квіточки по кутах.
    for (fx, fy) in [
        (52.0, h - 82.0),
        (w - 82.0, h - 82.0),
        (52.0, 52.0),
        (w - 82.0, 52.0),
    ] {
        flower(&mut c, fx, fy);
    }
    c.text(
        FontId::Hand,
        78.0,
        w / 2.0,
        h - 130.0,
        Align::Center,
        Rgb::hex("#d93a2f"),
        "Дякую!",
    );
    let to = to.trim();
    if !to.is_empty() {
        c.text(
            FontId::Bold,
            18.0,
            w / 2.0,
            h - 168.0,
            Align::Center,
            INK,
            to,
        );
    }
    let max = w - 2.0 * 120.0;
    let mut size = 32.0;
    while size > 13.0
        && c.lines_needed(FontId::Hand, size, max, text) as f32 * size * 1.25 > h - 330.0
    {
        size -= 1.0;
    }
    c.paragraph(
        FontId::Hand,
        size,
        120.0,
        h - 190.0,
        max,
        1.25,
        Align::Center,
        INK,
        text,
    );
    let from = from.trim();
    if !from.is_empty() {
        c.text(
            FontId::Hand,
            30.0,
            w - 130.0,
            80.0,
            Align::Right,
            Rgb::hex("#2b4bd6"),
            &format!("З повагою, {from}"),
        );
    }
    if !hero.is_empty() {
        draw_hero(&mut c, hero, 100.0, 60.0, 5.0);
    }
    c.finish("Лист подяки")
}

/// Дані свідоцтва про перехід (прізвисько, класи, назви світів, підсумок, дата).
pub struct Certificate<'a> {
    pub nickname: &'a str,
    pub from: u8,
    pub to: u8,
    pub world_from: &'a str,
    pub world_to: &'a str,
    pub mastered: usize,
    pub total: usize,
    pub stickers: usize,
    pub date: &'a str,
    /// Випускне свідоцтво (кінець шляху): без нового світу.
    pub finale: bool,
}

/// Свідоцтво про перехід до наступного класу: для друку й показу вдома чи вчителю. Лише прізвисько й підсумок, без порівнянь.
pub fn certificate_pdf(d: &Certificate, hero: &[Px]) -> Result<Vec<u8>, PdfError> {
    let Certificate {
        nickname,
        from,
        to,
        world_from,
        world_to,
        mastered,
        total,
        stickers,
        date,
        finale,
    } = d;
    let mut c = Canvas::new(A4_LANDSCAPE)?;
    let (w, h) = (c.width, c.height);
    c.rect(0.0, 0.0, w, h, 0.0, Some(Rgb::hex("#fffaf0")), None);
    c.rect(
        22.0,
        22.0,
        w - 44.0,
        h - 44.0,
        22.0,
        None,
        Some((Rgb::hex("#2b4bd6"), 4.0)),
    );
    c.rect(
        34.0,
        34.0,
        w - 68.0,
        h - 68.0,
        16.0,
        None,
        Some((Rgb::hex("#ffd23f"), 2.5)),
    );
    for (fx, fy) in [
        (52.0, h - 82.0),
        (w - 82.0, h - 82.0),
        (52.0, 52.0),
        (w - 82.0, 52.0),
    ] {
        flower(&mut c, fx, fy);
    }
    c.text(
        FontId::Bold,
        20.0,
        w / 2.0,
        h - 100.0,
        Align::Center,
        SOFT,
        "Kmit",
    );
    c.text(
        FontId::Hand,
        64.0,
        w / 2.0,
        h - 170.0,
        Align::Center,
        Rgb::hex("#2b4bd6"),
        &if *finale {
            "Шлях пройдено".to_string()
        } else {
            format!("Перехід до {to} класу")
        },
    );
    let name = nickname.trim();
    if !name.is_empty() {
        c.text(
            FontId::Hand,
            48.0,
            w / 2.0,
            h - 240.0,
            Align::Center,
            Rgb::hex("#d93a2f"),
            name,
        );
    }
    let max = w - 2.0 * 140.0;
    let lines = if *finale {
        [
            format!("Шлях у Kmit пройдено до {from} класу. Це справжнє досягнення."),
            format!("Опановано тем: {mastered} із {total}. Нагород у колекціях: {stickers}."),
            "Що б не було далі, знання лишаються з тобою. Дякуємо, що вчилися разом з Kmit."
                .to_string(),
        ]
    } else {
        [
            format!("{from} клас завершено. Попереду новий світ: «{world_to}»."),
            format!("У світі «{world_from}» опановано тем: {mastered} із {total}. Нагород у колекції: {stickers}."),
            "Крок за кроком, навіть коли було важко: так і вчаться.".to_string(),
        ]
    };
    let mut y = h - 300.0;
    for l in &lines {
        c.paragraph(
            FontId::Body,
            17.0,
            140.0,
            y,
            max,
            1.3,
            Align::Center,
            INK,
            l,
        );
        y -= 40.0;
    }
    if !date.is_empty() {
        c.text(
            FontId::Body,
            14.0,
            w - 130.0,
            80.0,
            Align::Right,
            SOFT,
            date,
        );
    }
    if !hero.is_empty() {
        draw_hero(&mut c, hero, 100.0, 60.0, 5.0);
    }
    c.finish("Свідоцтво про перехід")
}

/// Аркуш із кодом відновлення: його друкують або зберігають у надійному місці, замість переписування вручну.
pub fn recovery_sheet_pdf(nickname: &str, code: &str) -> Result<Vec<u8>, PdfError> {
    let mut c = Canvas::new(A4)?;
    let (w, h) = (c.width, c.height);
    c.rect(0.0, 0.0, w, h, 0.0, Some(Rgb::hex("#ffffff")), None);
    c.rect(
        MARGIN,
        MARGIN,
        w - 2.0 * MARGIN,
        h - 2.0 * MARGIN,
        14.0,
        None,
        Some((INK, 2.5)),
    );
    c.text(
        FontId::Bold,
        26.0,
        w / 2.0,
        h - 110.0,
        Align::Center,
        INK,
        "Код відновлення профілю",
    );
    let nickname = nickname.trim();
    if !nickname.is_empty() {
        c.text(
            FontId::Body,
            16.0,
            w / 2.0,
            h - 140.0,
            Align::Center,
            SOFT,
            &format!("Профіль: {nickname}"),
        );
    }
    c.rect(
        70.0,
        h - 280.0,
        w - 140.0,
        90.0,
        12.0,
        Some(PAPER),
        Some((SOFT, 1.5)),
    );
    let mut size = 34.0;
    while size > 16.0 && c.text_width(FontId::Bold, size, code) > w - 180.0 {
        size -= 1.0;
    }
    c.text(
        FontId::Bold,
        size,
        w / 2.0,
        h - 245.0,
        Align::Center,
        INK,
        code,
    );
    let notes = [
        "Цей код потрібен, якщо забути PIN профілю: з ним дані можна відновити, без нього ні.",
        "Збережи аркуш окремо від комп’ютера й телефона: у документах або в дорослого.",
        "Код показується лише один раз. Новий код робить цей недійсним.",
        "Нікому не показуй код: хто його має, може відкрити профіль.",
    ];
    let mut y = h - 340.0;
    for n in notes {
        let lines = c.lines_needed(FontId::Body, 14.0, w - 190.0, n);
        c.paragraph(
            FontId::Body,
            14.0,
            95.0,
            y,
            w - 190.0,
            1.35,
            Align::Left,
            INK,
            n,
        );
        y -= 14.0 * 1.35 * lines as f32 + 14.0;
    }
    c.finish("Код відновлення")
}

/// Маленька піксельна квітка 5×5 (піксель 5 пт).
fn flower(c: &mut Canvas, x: f32, y: f32) {
    let g = [".pyp.", "pyyyp", ".pyp.", "..g..", ".gg.."];
    for (r, row) in g.iter().enumerate() {
        for (i, ch) in row.chars().enumerate() {
            let col = match ch {
                'p' => "#ff6b9a",
                'y' => "#ffd23f",
                'g' => "#3aa66a",
                _ => continue,
            };
            c.rect(
                x + i as f32 * 6.0,
                y + (4 - r) as f32 * 6.0,
                6.0,
                6.0,
                0.0,
                Some(Rgb::hex(col)),
                None,
            );
        }
    }
}

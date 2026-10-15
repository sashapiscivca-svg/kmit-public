//! Мінімальний генератор PDF 1.4: текст на сторінках A4 із вбудованим шрифтом (Type0, Identity-H) і ToUnicode.
//! Лише потрібне для аркуша: заголовки, абзаци, списки, лінії, нумерація сторінок.

use crate::ttf::Font;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

const PAGE_W: f32 = 595.28;
const PAGE_H: f32 = 841.89;
const MARGIN: f32 = 56.7; // 20 мм
const BOTTOM: f32 = 64.0;
const BODY: f32 = 11.0;
const HEADING: f32 = 13.5;
const TITLE: f32 = 20.0;
const LEAD: f32 = 1.35;
const BULLET_INDENT: f32 = 14.0;

#[derive(Clone, Debug)]
pub enum Block {
    Title(String),
    Heading(String),
    Para(String),
    Bullet(String),
    /// Вкладений пункт (з більшим відступом).
    SubBullet(String),
    Rule,
    Space(f32),
}

#[derive(Clone, Debug)]
pub struct Document {
    pub title: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PdfError {
    /// Вбудований шрифт не розібрано.
    Font,
    /// Немає чого друкувати (порожня робота).
    Empty,
}

enum Op {
    Text {
        x: f32,
        y: f32,
        size: f32,
        gids: Vec<u16>,
    },
    Line {
        x1: f32,
        x2: f32,
        y: f32,
    },
}

fn is_invisible(c: char) -> bool {
    matches!(c as u32, 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064 | 0x2066..=0x2069 | 0xFEFF | 0x00AD)
}

/// Пробіли стають пробілами, керівні й невидимі символи зникають, відсутні у шрифті замінюються на `?`.
pub(crate) fn clean(font: &Font, s: &str) -> String {
    s.chars()
        .filter_map(|c| {
            if c.is_whitespace() {
                Some(' ')
            } else if c.is_control() || is_invisible(c) {
                None
            } else if font.glyph(c) == 0 {
                Some('?')
            } else {
                Some(c)
            }
        })
        .collect()
}

/// Розбиває текст на рядки заданої ширини; надто довге слово ламається за символами.
pub(crate) fn wrap(font: &Font, text: &str, size: f32, max: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if font.text_width(word, size) > max {
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            let mut piece = String::new();
            for ch in word.chars() {
                let mut trial = piece.clone();
                trial.push(ch);
                if font.text_width(&trial, size) > max && !piece.is_empty() {
                    lines.push(std::mem::take(&mut piece));
                }
                piece.push(ch);
            }
            cur = piece;
            continue;
        }
        let candidate = if cur.is_empty() {
            word.to_string()
        } else {
            format!("{cur} {word}")
        };
        if font.text_width(&candidate, size) <= max {
            cur = candidate;
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

struct Layout<'a> {
    font: &'a Font<'a>,
    pages: Vec<Vec<Op>>,
    y: f32, // верхня межа вільного місця поточної сторінки
}

impl<'a> Layout<'a> {
    fn new(font: &'a Font<'a>) -> Self {
        Layout {
            font,
            pages: vec![Vec::new()],
            y: PAGE_H - MARGIN,
        }
    }

    fn need(&mut self, h: f32) {
        if self.y - h < BOTTOM {
            self.pages.push(Vec::new());
            self.y = PAGE_H - MARGIN;
        }
    }

    fn lines(&mut self, text: &str, size: f32, x: f32, bullet: Option<f32>) {
        let width = PAGE_W - MARGIN - x;
        let text = clean(self.font, text);
        let wrapped = wrap(self.font, &text, size, width);
        let wrapped = if wrapped.is_empty() {
            vec![String::new()]
        } else {
            wrapped
        };
        for (i, line) in wrapped.iter().enumerate() {
            self.need(size * LEAD);
            self.y -= size * LEAD;
            let y = self.y + size * 0.3;
            if i == 0 {
                if let Some(bx) = bullet {
                    let g = vec![self.font.glyph('•').max(self.font.glyph('-'))];
                    self.pages.last_mut().expect("є сторінка").push(Op::Text {
                        x: bx,
                        y,
                        size,
                        gids: g,
                    });
                }
            }
            if !line.is_empty() {
                let gids = line.chars().map(|c| self.font.glyph(c)).collect();
                self.pages
                    .last_mut()
                    .expect("є сторінка")
                    .push(Op::Text { x, y, size, gids });
            }
        }
    }

    fn gap(&mut self, h: f32) {
        self.y -= h;
    }

    fn add(&mut self, b: &Block) {
        match b {
            Block::Title(t) => {
                self.lines(t, TITLE, MARGIN, None);
                self.gap(8.0);
            }
            Block::Heading(t) => {
                self.gap(10.0);
                self.need(HEADING * LEAD * 2.0);
                self.lines(t, HEADING, MARGIN, None);
                self.gap(3.0);
            }
            Block::Para(t) => {
                self.lines(t, BODY, MARGIN, None);
                self.gap(4.0);
            }
            Block::Bullet(t) => {
                self.lines(t, BODY, MARGIN + BULLET_INDENT, Some(MARGIN));
                self.gap(2.0);
            }
            Block::SubBullet(t) => {
                self.lines(
                    t,
                    BODY,
                    MARGIN + BULLET_INDENT * 2.0,
                    Some(MARGIN + BULLET_INDENT),
                );
                self.gap(2.0);
            }
            Block::Rule => {
                self.gap(6.0);
                self.need(4.0);
                let y = self.y;
                self.pages.last_mut().expect("є сторінка").push(Op::Line {
                    x1: MARGIN,
                    x2: PAGE_W - MARGIN,
                    y,
                });
                self.gap(6.0);
            }
            Block::Space(h) => self.gap(h.clamp(0.0, 200.0)),
        }
    }
}

pub(crate) fn f(v: f32) -> String {
    format!("{v:.2}")
}

pub(crate) fn hex_gids(g: &[u16]) -> String {
    let mut s = String::with_capacity(g.len() * 4 + 2);
    s.push('<');
    for x in g {
        let _ = write!(s, "{x:04X}");
    }
    s.push('>');
    s
}

pub(crate) fn utf16_hex(s: &str) -> String {
    let mut out = String::from("<FEFF");
    for u in s.encode_utf16() {
        let _ = write!(out, "{u:04X}");
    }
    out.push('>');
    out
}

pub(crate) struct Writer {
    pub(crate) buf: Vec<u8>,
    pub(crate) offsets: BTreeMap<usize, usize>,
}

impl Writer {
    pub(crate) fn obj(&mut self, n: usize, body: &[u8]) {
        self.offsets.insert(n, self.buf.len());
        self.buf
            .extend_from_slice(format!("{n} 0 obj\n").as_bytes());
        self.buf.extend_from_slice(body);
        self.buf.extend_from_slice(b"\nendobj\n");
    }

    pub(crate) fn stream(&mut self, n: usize, dict_extra: &str, data: &[u8]) {
        let mut body = format!("<< /Length {}{dict_extra} >>\nstream\n", data.len()).into_bytes();
        body.extend_from_slice(data);
        body.extend_from_slice(b"\nendstream");
        self.obj(n, &body);
    }
}

/// Складає PDF із документа.
pub fn render(doc: &Document) -> Result<Vec<u8>, PdfError> {
    let font = Font::parse(crate::FONT_BYTES).ok_or(PdfError::Font)?;
    let mut lay = Layout::new(&font);
    for b in &doc.blocks {
        lay.add(b);
    }
    let mut pages = lay.pages;
    let total = pages.len();
    // Нумерація сторінок унизу.
    for (i, p) in pages.iter_mut().enumerate() {
        let label = clean(&font, &format!("Сторінка {} з {}", i + 1, total));
        let gids = label.chars().map(|c| font.glyph(c)).collect();
        p.push(Op::Text {
            x: MARGIN,
            y: 36.0,
            size: 9.0,
            gids,
        });
    }

    // Використані гліфи для ширин і ToUnicode.
    let mut used: BTreeSet<u16> = BTreeSet::new();
    for p in &pages {
        for op in p {
            if let Op::Text { gids, .. } = op {
                used.extend(gids.iter().copied());
            }
        }
    }

    let scale = 1000.0 / f32::from(font.units_per_em());
    let mut w = Writer {
        buf: b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec(),
        offsets: BTreeMap::new(),
    };

    // 1: каталог, 2: сторінки, 3..8: шрифт та службові об'єкти; сторінки з 9.
    let first_page_obj = 9;
    let kids: String = (0..total)
        .map(|i| format!("{} 0 R", first_page_obj + i * 2))
        .collect::<Vec<_>>()
        .join(" ");
    w.obj(1, b"<< /Type /Catalog /Pages 2 0 R >>");
    w.obj(
        2,
        format!("<< /Type /Pages /Kids [{kids}] /Count {total} >>").as_bytes(),
    );
    w.obj(
        3,
        b"<< /Type /Font /Subtype /Type0 /BaseFont /LiberationSans /Encoding /Identity-H /DescendantFonts [4 0 R] /ToUnicode 7 0 R >>",
    );
    let mut widths = String::from("[");
    for g in &used {
        let _ = write!(
            widths,
            " {} [ {} ]",
            g,
            (f32::from(font.advance(*g)) * scale).round() as i32
        );
    }
    widths.push_str(" ]");
    w.obj(
        4,
        format!(
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /LiberationSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 5 0 R /CIDToGIDMap /Identity /DW 600\n/W {widths}\n>>"
        )
        .as_bytes(),
    );
    let bb = font.bbox();
    w.obj(
        5,
        format!(
            "<< /Type /FontDescriptor /FontName /LiberationSans /Flags 32 /FontBBox [{} {} {} {}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 6 0 R >>",
            (f32::from(bb[0]) * scale).round() as i32,
            (f32::from(bb[1]) * scale).round() as i32,
            (f32::from(bb[2]) * scale).round() as i32,
            (f32::from(bb[3]) * scale).round() as i32,
            (f32::from(font.ascent()) * scale).round() as i32,
            (f32::from(font.descent()) * scale).round() as i32,
            (f32::from(font.ascent()) * scale * 0.72).round() as i32,
        )
        .as_bytes(),
    );
    w.stream(6, &format!(" /Length1 {}", font.data().len()), font.data());

    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let entries: Vec<(u16, u32)> = used
        .iter()
        .filter_map(|g| font.char_for_glyph(*g).map(|c| (*g, c as u32)))
        .filter(|(_, c)| *c <= 0xFFFF)
        .collect();
    for chunk in entries.chunks(100) {
        let _ = writeln!(cmap, "{} beginbfchar", chunk.len());
        for (g, c) in chunk {
            let _ = writeln!(cmap, "<{g:04X}> <{c:04X}>");
        }
        cmap.push_str("endbfchar\n");
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend");
    w.stream(7, "", cmap.as_bytes());
    w.obj(
        8,
        format!(
            "<< /Title {} /Producer (Teacher) /Creator (Teacher) >>",
            utf16_hex(&doc.title)
        )
        .as_bytes(),
    );

    for (i, ops) in pages.iter().enumerate() {
        let page_n = first_page_obj + i * 2;
        let mut content = String::new();
        for op in ops {
            match op {
                Op::Text { x, y, size, gids } => {
                    let _ = writeln!(
                        content,
                        "BT /F1 {} Tf {} {} Td {} Tj ET",
                        f(*size),
                        f(*x),
                        f(*y),
                        hex_gids(gids)
                    );
                }
                Op::Line { x1, x2, y } => {
                    let _ = writeln!(
                        content,
                        "0.6 w {} {} m {} {} l S",
                        f(*x1),
                        f(*y),
                        f(*x2),
                        f(*y)
                    );
                }
            }
        }
        w.obj(
            page_n,
            format!(
                "<< /Type /Page\n/Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
                f(PAGE_W),
                f(PAGE_H),
                page_n + 1
            )
            .as_bytes(),
        );
        w.stream(page_n + 1, "", content.as_bytes());
    }

    let n_objs = first_page_obj + total * 2; // номери 1..n_objs-1
    let xref_at = w.buf.len();
    w.buf
        .extend_from_slice(format!("xref\n0 {n_objs}\n").as_bytes());
    w.buf.extend_from_slice(b"0000000000 65535 f \n");
    for n in 1..n_objs {
        let off = w.offsets.get(&n).copied().unwrap_or(0);
        w.buf
            .extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    w.buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {n_objs} /Root 1 0 R /Info 8 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        )
        .as_bytes(),
    );
    Ok(w.buf)
}

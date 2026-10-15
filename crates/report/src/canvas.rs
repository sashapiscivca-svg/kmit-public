//! Полотно для оформлених PDF (аркуш для вчителя, постер героя, листівка подяки): кольорові прямокутники, кілька
//! шрифтів (Manrope для тексту, Caveat для рукописних акцентів), вирівнювання, перенесення рядків. Тільки векторна
//! графіка й текст із вбудованими шрифтами (кирилиця). Координати в пунктах, початок у лівому нижньому куті.

#![allow(clippy::too_many_arguments)]

use crate::pdf::{clean, f, hex_gids, utf16_hex, wrap, PdfError, Writer};
use crate::ttf::Font;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

pub const A4: (f32, f32) = (595.28, 841.89);
pub const A4_LANDSCAPE: (f32, f32) = (841.89, 595.28);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// З рядка `#rrggbb`; хибний колір стає чорним.
    pub fn hex(s: &str) -> Rgb {
        let t = s.trim_start_matches('#');
        if t.len() != 6 {
            return Rgb(0, 0, 0);
        }
        let p = |i: usize| u8::from_str_radix(&t[i..i + 2], 16).unwrap_or(0);
        Rgb(p(0), p(2), p(4))
    }

    fn pdf(self) -> String {
        format!(
            "{} {} {}",
            f(f32::from(self.0) / 255.0),
            f(f32::from(self.1) / 255.0),
            f(f32::from(self.2) / 255.0)
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontId {
    /// Manrope ExtraBold: заголовки.
    Bold,
    /// Manrope SemiBold: основний текст.
    Body,
    /// Caveat Bold: рукописні акценти.
    Hand,
}

const FONTS: [(FontId, &str, &[u8]); 3] = [
    (
        FontId::Bold,
        "Manrope-ExtraBold",
        include_bytes!("../assets/Manrope-800.ttf"),
    ),
    (
        FontId::Body,
        "Manrope-SemiBold",
        include_bytes!("../assets/Manrope-600.ttf"),
    ),
    (
        FontId::Hand,
        "Caveat-Bold",
        include_bytes!("../assets/Caveat-700.ttf"),
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

enum Op {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        fill: Option<Rgb>,
        stroke: Option<(Rgb, f32)>,
    },
    Text {
        font: usize,
        size: f32,
        x: f32,
        y: f32,
        color: Rgb,
        gids: Vec<u16>,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: Rgb,
        width: f32,
        dash: bool,
    },
}

pub struct Canvas<'a> {
    fonts: Vec<Font<'a>>,
    pub width: f32,
    pub height: f32,
    pages: Vec<Vec<Op>>,
}

impl<'a> Canvas<'a> {
    pub fn new(size: (f32, f32)) -> Result<Canvas<'a>, PdfError> {
        let mut fonts = Vec::new();
        for (_, _, bytes) in FONTS {
            fonts.push(Font::parse(bytes).ok_or(PdfError::Font)?);
        }
        Ok(Canvas {
            fonts,
            width: size.0,
            height: size.1,
            pages: vec![Vec::new()],
        })
    }

    fn idx(font: FontId) -> usize {
        FONTS.iter().position(|(id, _, _)| *id == font).unwrap_or(0)
    }

    fn cur(&mut self) -> &mut Vec<Op> {
        self.pages.last_mut().expect("є сторінка")
    }

    /// Нова сторінка.
    pub fn new_page(&mut self) {
        self.pages.push(Vec::new());
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Прямокутник із заливкою й/або рамкою; `r` радіус заокруглення.
    pub fn rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        fill: Option<Rgb>,
        stroke: Option<(Rgb, f32)>,
    ) {
        self.cur().push(Op::Rect {
            x,
            y,
            w,
            h,
            r,
            fill,
            stroke,
        });
    }

    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Rgb, width: f32, dash: bool) {
        self.cur().push(Op::Line {
            x1,
            y1,
            x2,
            y2,
            color,
            width,
            dash,
        });
    }

    pub fn text_width(&self, font: FontId, size: f32, text: &str) -> f32 {
        let f = &self.fonts[Self::idx(font)];
        f.text_width(&clean(f, text), size)
    }

    /// Один рядок тексту (без перенесення). `y` це базова лінія.
    pub fn text(
        &mut self,
        font: FontId,
        size: f32,
        x: f32,
        y: f32,
        align: Align,
        color: Rgb,
        text: &str,
    ) {
        let i = Self::idx(font);
        let t = clean(&self.fonts[i], text);
        let w = self.fonts[i].text_width(&t, size);
        let x = match align {
            Align::Left => x,
            Align::Center => x - w / 2.0,
            Align::Right => x - w,
        };
        let gids: Vec<u16> = t.chars().map(|c| self.fonts[i].glyph(c)).collect();
        if !gids.is_empty() {
            self.cur().push(Op::Text {
                font: i,
                size,
                x,
                y,
                color,
                gids,
            });
        }
    }

    /// Абзац у межах `max` по ширині; повертає кількість рядків. `lead` міжрядковий інтервал у множниках розміру.
    pub fn paragraph(
        &mut self,
        font: FontId,
        size: f32,
        x: f32,
        y_top: f32,
        max: f32,
        lead: f32,
        align: Align,
        color: Rgb,
        text: &str,
    ) -> usize {
        let i = Self::idx(font);
        let lines = wrap(&self.fonts[i], &clean(&self.fonts[i], text), size, max);
        for (k, l) in lines.iter().enumerate() {
            let y = y_top - size - k as f32 * size * lead;
            let ax = match align {
                Align::Left => x,
                Align::Center => x + max / 2.0,
                Align::Right => x + max,
            };
            self.text(font, size, ax, y, align, color, l);
        }
        lines.len()
    }

    /// Скільки рядків займе абзац (для розмітки без малювання).
    pub fn lines_needed(&self, font: FontId, size: f32, max: f32, text: &str) -> usize {
        let f = &self.fonts[Self::idx(font)];
        wrap(f, &clean(f, text), size, max).len()
    }

    /// Складає PDF.
    pub fn finish(self, title: &str) -> Result<Vec<u8>, PdfError> {
        let fonts = &self.fonts;
        let mut used: Vec<BTreeSet<u16>> = vec![BTreeSet::new(); fonts.len()];
        for p in &self.pages {
            for op in p {
                if let Op::Text { font, gids, .. } = op {
                    used[*font].extend(gids.iter().copied());
                }
            }
        }
        let mut w = Writer {
            buf: b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec(),
            offsets: BTreeMap::new(),
        };
        let nf = fonts.len();
        // Об'єкти: 1 каталог, 2 сторінки, 3 інформація, далі по 5 на шрифт, потім по 2 на сторінку.
        let first_page = 4 + nf * 5;
        let total = self.pages.len();
        let kids: String = (0..total)
            .map(|i| format!("{} 0 R", first_page + i * 2))
            .collect::<Vec<_>>()
            .join(" ");
        w.obj(1, b"<< /Type /Catalog /Pages 2 0 R >>");
        w.obj(
            2,
            format!("<< /Type /Pages /Kids [{kids}] /Count {total} >>").as_bytes(),
        );
        w.obj(
            3,
            format!(
                "<< /Title {} /Producer (Kmit) /Creator (Kmit) >>",
                utf16_hex(title)
            )
            .as_bytes(),
        );
        for (i, font) in fonts.iter().enumerate() {
            let base = 4 + i * 5;
            let name = FONTS[i].1;
            let scale = 1000.0 / f32::from(font.units_per_em());
            w.obj(base, format!("<< /Type /Font /Subtype /Type0 /BaseFont /{name} /Encoding /Identity-H /DescendantFonts [{} 0 R] /ToUnicode {} 0 R >>", base + 1, base + 4).as_bytes());
            let mut widths = String::from("[");
            for g in &used[i] {
                let _ = write!(
                    widths,
                    " {} [ {} ]",
                    g,
                    (f32::from(font.advance(*g)) * scale).round() as i32
                );
            }
            widths.push_str(" ]");
            w.obj(base + 1, format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{name} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {} 0 R /CIDToGIDMap /Identity /DW 600\n/W {widths}\n>>", base + 2).as_bytes());
            let bb = font.bbox();
            w.obj(base + 2, format!(
                "<< /Type /FontDescriptor /FontName /{name} /Flags 32 /FontBBox [{} {} {} {}] /ItalicAngle 0 /Ascent {} /Descent {} /CapHeight {} /StemV 80 /FontFile2 {} 0 R >>",
                (f32::from(bb[0]) * scale).round() as i32, (f32::from(bb[1]) * scale).round() as i32,
                (f32::from(bb[2]) * scale).round() as i32, (f32::from(bb[3]) * scale).round() as i32,
                (f32::from(font.ascent()) * scale).round() as i32, (f32::from(font.descent()) * scale).round() as i32,
                (f32::from(font.ascent()) * scale * 0.72).round() as i32, base + 3).as_bytes());
            w.stream(
                base + 3,
                &format!(" /Length1 {}", font.data().len()),
                font.data(),
            );
            let mut cmap = String::from("/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n");
            let entries: Vec<(u16, u32)> = used[i]
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
            w.stream(base + 4, "", cmap.as_bytes());
        }
        let font_res: String = (0..nf)
            .map(|i| format!("/F{} {} 0 R", i + 1, 4 + i * 5))
            .collect::<Vec<_>>()
            .join(" ");
        for (pi, ops) in self.pages.iter().enumerate() {
            let page_n = first_page + pi * 2;
            let mut c = String::new();
            for op in ops {
                match op {
                    Op::Rect {
                        x,
                        y,
                        w: rw,
                        h,
                        r,
                        fill,
                        stroke,
                    } => {
                        if let Some(col) = fill {
                            let _ = writeln!(c, "{} rg", col.pdf());
                        }
                        if let Some((col, lw)) = stroke {
                            let _ = writeln!(c, "{} RG {} w", col.pdf(), f(*lw));
                        }
                        let path = rounded(*x, *y, *rw, *h, *r);
                        let paint = match (fill.is_some(), stroke.is_some()) {
                            (true, true) => "B",
                            (true, false) => "f",
                            (false, true) => "S",
                            (false, false) => "n",
                        };
                        let _ = writeln!(c, "{path}{paint}");
                    }
                    Op::Text {
                        font,
                        size,
                        x,
                        y,
                        color,
                        gids,
                    } => {
                        let _ = writeln!(
                            c,
                            "{} rg BT /F{} {} Tf {} {} Td {} Tj ET",
                            color.pdf(),
                            font + 1,
                            f(*size),
                            f(*x),
                            f(*y),
                            hex_gids(gids)
                        );
                    }
                    Op::Line {
                        x1,
                        y1,
                        x2,
                        y2,
                        color,
                        width,
                        dash,
                    } => {
                        let d = if *dash { "[3 3] 0 d" } else { "[] 0 d" };
                        let _ = writeln!(
                            c,
                            "{} RG {} w {} {} {} m {} {} l S",
                            color.pdf(),
                            f(*width),
                            d,
                            f(*x1),
                            f(*y1),
                            f(*x2),
                            f(*y2)
                        );
                    }
                }
            }
            w.obj(page_n, format!("<< /Type /Page\n/Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /Font << {font_res} >> >> /Contents {} 0 R >>", f(self.width), f(self.height), page_n + 1).as_bytes());
            w.stream(page_n + 1, "", c.as_bytes());
        }
        let n_objs = first_page + total * 2;
        let xref_at = w.buf.len();
        w.buf
            .extend_from_slice(format!("xref\n0 {n_objs}\n").as_bytes());
        w.buf.extend_from_slice(b"0000000000 65535 f \n");
        for n in 1..n_objs {
            let off = w.offsets.get(&n).copied().unwrap_or(0);
            w.buf
                .extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        w.buf.extend_from_slice(format!("trailer\n<< /Size {n_objs} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{xref_at}\n%%EOF\n").as_bytes());
        Ok(w.buf)
    }
}

/// Контур прямокутника (із заокругленням) у PDF-операторах.
fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> String {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    if r < 0.1 {
        return format!("{} {} {} {} re\n", f(x), f(y), f(w), f(h));
    }
    let k = r * 0.5523;
    let (x2, y2) = (x + w, y + h);
    let mut s = String::new();
    let _ = writeln!(s, "{} {} m", f(x + r), f(y));
    let _ = writeln!(s, "{} {} l", f(x2 - r), f(y));
    let _ = writeln!(
        s,
        "{} {} {} {} {} {} c",
        f(x2 - r + k),
        f(y),
        f(x2),
        f(y + r - k),
        f(x2),
        f(y + r)
    );
    let _ = writeln!(s, "{} {} l", f(x2), f(y2 - r));
    let _ = writeln!(
        s,
        "{} {} {} {} {} {} c",
        f(x2),
        f(y2 - r + k),
        f(x2 - r + k),
        f(y2),
        f(x2 - r),
        f(y2)
    );
    let _ = writeln!(s, "{} {} l", f(x + r), f(y2));
    let _ = writeln!(
        s,
        "{} {} {} {} {} {} c",
        f(x + r - k),
        f(y2),
        f(x),
        f(y2 - r + k),
        f(x),
        f(y2 - r)
    );
    let _ = writeln!(s, "{} {} l", f(x), f(y + r));
    let _ = writeln!(
        s,
        "{} {} {} {} {} {} c h",
        f(x),
        f(y + r - k),
        f(x + r - k),
        f(y),
        f(x + r),
        f(y)
    );
    s
}

//! Мінімальний читач TrueType: відображення символів у гліфи (cmap формати 4 і 12), ширини (hmtx), метрики.
//! Усі читання з перевіркою меж: зіпсований шрифт дає `None`, а не паніку.

use std::collections::BTreeMap;

/// Максимум записів cmap, які ми готові прочитати (захист від зіпсованих даних).
const MAX_MAPPINGS: usize = 300_000;

pub struct Font<'a> {
    data: &'a [u8],
    upem: u16,
    ascent: i16,
    descent: i16,
    bbox: [i16; 4],
    num_hmetrics: u16,
    hmtx: usize,
    forward: BTreeMap<u32, u16>,
    reverse: BTreeMap<u16, u32>,
}

fn u16_at(d: &[u8], off: usize) -> Option<u16> {
    let b = d.get(off..off.checked_add(2)?)?;
    Some(u16::from_be_bytes([b[0], b[1]]))
}

fn i16_at(d: &[u8], off: usize) -> Option<i16> {
    u16_at(d, off).map(|v| v as i16)
}

fn u32_at(d: &[u8], off: usize) -> Option<u32> {
    let b = d.get(off..off.checked_add(4)?)?;
    Some(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

fn table(d: &[u8], tag: &[u8; 4]) -> Option<(usize, usize)> {
    let n = usize::from(u16_at(d, 4)?);
    for i in 0..n {
        let rec = 12 + i * 16;
        if d.get(rec..rec + 4)? == tag {
            let off = usize::try_from(u32_at(d, rec + 8)?).ok()?;
            let len = usize::try_from(u32_at(d, rec + 12)?).ok()?;
            d.get(off..off.checked_add(len)?)?;
            return Some((off, len));
        }
    }
    None
}

fn read_format4(d: &[u8], sub: usize, map: &mut BTreeMap<u32, u16>) -> Option<()> {
    let seg = usize::from(u16_at(d, sub + 6)? / 2);
    let end_at = sub + 14;
    let start_at = end_at + seg * 2 + 2;
    let delta_at = start_at + seg * 2;
    let range_at = delta_at + seg * 2;
    for i in 0..seg {
        let end = u16_at(d, end_at + i * 2)?;
        let start = u16_at(d, start_at + i * 2)?;
        let delta = i16_at(d, delta_at + i * 2)?;
        let range = u16_at(d, range_at + i * 2)?;
        if start > end {
            continue;
        }
        for c in start..=end {
            if c == 0xFFFF {
                continue;
            }
            let gid = if range == 0 {
                (i32::from(c) + i32::from(delta)).rem_euclid(65_536) as u16
            } else {
                let addr = range_at + i * 2 + usize::from(range) + usize::from(c - start) * 2;
                match u16_at(d, addr)? {
                    0 => 0,
                    g => (i32::from(g) + i32::from(delta)).rem_euclid(65_536) as u16,
                }
            };
            if gid != 0 {
                map.insert(u32::from(c), gid);
                if map.len() > MAX_MAPPINGS {
                    return None;
                }
            }
        }
    }
    Some(())
}

fn read_format12(d: &[u8], sub: usize, map: &mut BTreeMap<u32, u16>) -> Option<()> {
    let groups = usize::try_from(u32_at(d, sub + 12)?).ok()?;
    for g in 0..groups.min(MAX_MAPPINGS) {
        let at = sub + 16 + g * 12;
        let (start, end, gid0) = (u32_at(d, at)?, u32_at(d, at + 4)?, u32_at(d, at + 8)?);
        if start > end || end > 0x10_FFFF {
            return None;
        }
        for (k, c) in (start..=end).enumerate() {
            let gid = u16::try_from(gid0.checked_add(u32::try_from(k).ok()?)?).ok()?;
            if gid != 0 {
                map.insert(c, gid);
                if map.len() > MAX_MAPPINGS {
                    return None;
                }
            }
        }
    }
    Some(())
}

impl<'a> Font<'a> {
    pub fn parse(data: &'a [u8]) -> Option<Font<'a>> {
        let (head, _) = table(data, b"head")?;
        let (hhea, _) = table(data, b"hhea")?;
        let (hmtx, hmtx_len) = table(data, b"hmtx")?;
        let (cmap, _) = table(data, b"cmap")?;
        let upem = u16_at(data, head + 18)?;
        if upem == 0 {
            return None;
        }
        let bbox = [
            i16_at(data, head + 36)?,
            i16_at(data, head + 38)?,
            i16_at(data, head + 40)?,
            i16_at(data, head + 42)?,
        ];
        let ascent = i16_at(data, hhea + 4)?;
        let descent = i16_at(data, hhea + 6)?;
        let num_hmetrics = u16_at(data, hhea + 34)?;
        if num_hmetrics == 0 || usize::from(num_hmetrics) * 4 > hmtx_len {
            return None;
        }

        let mut forward = BTreeMap::new();
        let n_sub = usize::from(u16_at(data, cmap + 2)?);
        // Перевага: Unicode повний (3,10), потім Unicode BMP (3,1), потім платформа 0.
        let mut best: Option<(u8, usize, u16)> = None;
        for i in 0..n_sub {
            let rec = cmap + 4 + i * 8;
            let (platform, encoding) = (u16_at(data, rec)?, u16_at(data, rec + 2)?);
            let off = cmap + usize::try_from(u32_at(data, rec + 4)?).ok()?;
            let format = u16_at(data, off)?;
            let rank = match (platform, encoding, format) {
                (3, 10, 12) => 3,
                (3, 1, 4) | (0, _, 4) => 2,
                (0, _, 12) => 3,
                _ => continue,
            };
            if best.map_or(true, |(r, _, _)| rank > r) {
                best = Some((rank, off, format));
            }
        }
        let (_, sub, format) = best?;
        match format {
            4 => read_format4(data, sub, &mut forward)?,
            12 => read_format12(data, sub, &mut forward)?,
            _ => return None,
        }
        let mut reverse = BTreeMap::new();
        for (c, g) in &forward {
            reverse.entry(*g).or_insert(*c);
        }
        Some(Font {
            data,
            upem,
            ascent,
            descent,
            bbox,
            num_hmetrics,
            hmtx,
            forward,
            reverse,
        })
    }

    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    pub fn units_per_em(&self) -> u16 {
        self.upem
    }

    pub fn ascent(&self) -> i16 {
        self.ascent
    }

    pub fn descent(&self) -> i16 {
        self.descent
    }

    pub fn bbox(&self) -> [i16; 4] {
        self.bbox
    }

    /// Гліф для символа; 0 означає, що символа в шрифті немає.
    pub fn glyph(&self, c: char) -> u16 {
        self.forward.get(&(c as u32)).copied().unwrap_or(0)
    }

    pub fn char_for_glyph(&self, gid: u16) -> Option<char> {
        self.reverse.get(&gid).and_then(|c| char::from_u32(*c))
    }

    /// Ширина гліфа в одиницях шрифту.
    pub fn advance(&self, gid: u16) -> u16 {
        let idx = gid.min(self.num_hmetrics - 1);
        u16_at(self.data, self.hmtx + usize::from(idx) * 4).unwrap_or(0)
    }

    pub fn text_width(&self, text: &str, size: f32) -> f32 {
        let units: u32 = text
            .chars()
            .map(|c| u32::from(self.advance(self.glyph(c))))
            .sum();
        units as f32 * size / f32::from(self.upem)
    }
}

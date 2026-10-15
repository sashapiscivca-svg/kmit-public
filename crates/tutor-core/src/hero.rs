//! Піксельний герой дитини: сітка 16×20, шари (тло, спина, одяг, тіло, волосся, головний убір, обличчя, предмет у руці,
//! улюбленець, рамка). Єдине джерело малюнків для екрана й для PDF-постера. Предмети дитина отримує за старанність
//! раз на тиждень (див. `app-core/src/weekly.rs`); базовий вигляд (колір шкіри, зачіска, колір волосся) вільний.
//!
//! Предмет — це ASCII-малюнок: `.` прозорий, інші символи беруть колір із палітри предмета, плюс необов'язкові
//! прямокутники (для тла й рамок). Особливі кольори `$S` (шкіра), `$D` (темніша шкіра), `$H` (колір волосся)
//! підставляються з вигляду дитини. Каталог предметів: `hero_items.rs`.

use crate::hero_items::ITEMS;
use std::collections::BTreeMap;

pub const GRID_W: u8 = 16;
pub const GRID_H: u8 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Slot {
    Bg,
    Back,
    Outfit,
    Hair,
    Hat,
    Face,
    Hand,
    Pet,
    Frame,
}

impl Slot {
    /// Порядок шарів знизу вгору (тіло з головою малюється між одягом і волоссям).
    pub const ALL: [Slot; 9] = [
        Slot::Bg,
        Slot::Back,
        Slot::Outfit,
        Slot::Hair,
        Slot::Hat,
        Slot::Face,
        Slot::Hand,
        Slot::Pet,
        Slot::Frame,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Slot::Bg => "bg",
            Slot::Back => "back",
            Slot::Outfit => "outfit",
            Slot::Hair => "hair",
            Slot::Hat => "hat",
            Slot::Face => "face",
            Slot::Hand => "hand",
            Slot::Pet => "pet",
            Slot::Frame => "frame",
        }
    }

    pub fn from_code(c: &str) -> Option<Slot> {
        Slot::ALL.into_iter().find(|s| s.code() == c)
    }

    pub fn title(self) -> &'static str {
        match self {
            Slot::Bg => "Тло",
            Slot::Back => "За спиною",
            Slot::Outfit => "Одяг",
            Slot::Hair => "Зачіска",
            Slot::Hat => "Головний убір",
            Slot::Face => "Обличчя",
            Slot::Hand => "У руці",
            Slot::Pet => "Друг",
            Slot::Frame => "Рамка",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rarity {
    Common,
    Rare,
    Epic,
}

impl Rarity {
    pub fn code(self) -> &'static str {
        match self {
            Rarity::Common => "common",
            Rarity::Rare => "rare",
            Rarity::Epic => "epic",
        }
    }
}

pub struct Item {
    pub id: &'static str,
    pub name: &'static str,
    pub slot: Slot,
    pub rarity: Rarity,
    /// Вільний предмет (є з початку, не видається за тиждень).
    pub free: bool,
    /// Лівий верхній кут малюнка на сітці.
    pub x: u8,
    pub y: u8,
    pub rows: &'static [&'static str],
    pub pal: &'static [(char, &'static str)],
    /// Додаткові суцільні прямокутники (x, y, w, h, колір): для тла й рамок.
    pub rects: &'static [(u8, u8, u8, u8, &'static str)],
}

/// Один зафарбований прямокутник сітки (суміжні клітинки одного кольору в рядку об'єднано).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Px {
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
    pub color: String,
}

/// Вигляд героя: базові вибори й одягнені предмети.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Look {
    pub skin: u8,
    pub hair_color: u8,
    pub equipped: BTreeMap<Slot, String>,
}

pub const SKINS: [(&str, &str, &str); 6] = [
    ("#ffdfc4", "#e8b896", "Дуже світла"),
    ("#f3cd9c", "#d9a877", "Світла"),
    ("#e0a370", "#c0825a", "Золотава"),
    ("#c68642", "#a1672f", "Смаглява"),
    ("#8d5524", "#6e3f17", "Темна"),
    ("#5c3a21", "#43271a", "Дуже темна"),
];

pub const HAIR_COLORS: [(&str, &str); 8] = [
    ("#2b2b2b", "Чорне"),
    ("#6b4226", "Каштанове"),
    ("#d9a441", "Русяве"),
    ("#b5472b", "Руде"),
    ("#3b6fd6", "Синє"),
    ("#e87aa8", "Рожеве"),
    ("#e8e8e8", "Біле"),
    ("#3aa66a", "Зелене"),
];

impl Default for Look {
    fn default() -> Self {
        let mut equipped = BTreeMap::new();
        equipped.insert(Slot::Outfit, "outfit_school".to_string());
        equipped.insert(Slot::Hair, "hair_short".to_string());
        equipped.insert(Slot::Bg, "bg_sky".to_string());
        Look {
            skin: 1,
            hair_color: 1,
            equipped,
        }
    }
}

pub fn item(id: &str) -> Option<&'static Item> {
    ITEMS.iter().find(|i| i.id == id)
}

pub fn items() -> &'static [Item] {
    ITEMS
}

/// Голова, шия й руки (колір шкіри з вигляду); малюються між одягом і волоссям.
const HEAD: [&str; 7] = [
    ".SSSSSS.", ".SSSSSS.", "SSkSSkSS", "SSSSSSSS", ".SSddSS.", ".SSSSSS.", "...SS...",
];

fn resolve(color: &str, look: &Look) -> String {
    let skin = SKINS[usize::from(look.skin).min(SKINS.len() - 1)];
    match color {
        "$S" => skin.0.to_string(),
        "$D" => skin.1.to_string(),
        "$H" => HAIR_COLORS[usize::from(look.hair_color).min(HAIR_COLORS.len() - 1)]
            .0
            .to_string(),
        c => c.to_string(),
    }
}

/// Піксельний малюнок рядків ASCII у прямокутники (горизонтальні серії одного кольору).
fn rows_to_px(
    x0: u8,
    y0: u8,
    rows: &[&str],
    pal: &dyn Fn(char) -> Option<String>,
    out: &mut Vec<Px>,
) {
    for (dy, row) in rows.iter().enumerate() {
        let chars: Vec<char> = row.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let Some(color) = (c != '.').then(|| pal(c)).flatten() else {
                i += 1;
                continue;
            };
            let mut j = i;
            while j < chars.len() && chars[j] == c {
                j += 1;
            }
            out.push(Px {
                x: x0 + u8::try_from(i).unwrap_or(0),
                y: y0 + u8::try_from(dy).unwrap_or(0),
                w: u8::try_from(j - i).unwrap_or(1),
                h: 1,
                color: color.clone(),
            });
            i = j;
        }
    }
}

/// Прямокутники одного предмета для вигляду `look`.
pub fn item_px(it: &Item, look: &Look) -> Vec<Px> {
    let mut out = Vec::new();
    let pal = |c: char| {
        it.pal
            .iter()
            .find(|(k, _)| *k == c)
            .map(|(_, v)| resolve(v, look))
    };
    for (x, y, w, h, c) in it.rects {
        out.push(Px {
            x: *x,
            y: *y,
            w: *w,
            h: *h,
            color: resolve(c, look),
        });
    }
    rows_to_px(it.x, it.y, it.rows, &pal, &mut out);
    out
}

/// Тіло: голова, шия, руки.
pub fn body_px(look: &Look) -> Vec<Px> {
    let mut out = Vec::new();
    let pal = |c: char| match c {
        'S' => Some(resolve("$S", look)),
        'd' => Some(resolve("$D", look)),
        'k' => Some("#2b2b2b".to_string()),
        _ => None,
    };
    rows_to_px(4, 3, &HEAD, &pal, &mut out);
    for x in [3u8, 12] {
        out.push(Px {
            x,
            y: 15,
            w: 1,
            h: 1,
            color: resolve("$S", look),
        });
    }
    out
}

/// Шари героя знизу вгору: (позначка шару, прямокутники). Шар `body` це голова й руки.
pub fn layers(look: &Look) -> Vec<(&'static str, Vec<Px>)> {
    let mut out = Vec::new();
    for slot in Slot::ALL {
        if slot == Slot::Hair {
            out.push(("body", body_px(look)));
        }
        if let Some(it) = look.equipped.get(&slot).and_then(|id| item(id)) {
            out.push((slot.code(), item_px(it, look)));
        }
    }
    out
}

/// Усі прямокутники героя в порядку малювання.
pub fn compose(look: &Look) -> Vec<Px> {
    layers(look).into_iter().flat_map(|(_, p)| p).collect()
}

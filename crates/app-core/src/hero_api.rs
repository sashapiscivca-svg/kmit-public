//! Герой дитини для екрана: каталог із прев'ю на її вигляді, вибір базового вигляду, одягання предметів, PDF-постер.

use crate::error::ApiError;
use serde_json::{json, Value};
use storage::state::AppState;
use tutor_core::hero::{compose, item, item_px, items, Look, Px, Slot, HAIR_COLORS, SKINS};

pub(crate) fn px_json(px: &[Px]) -> Value {
    Value::Array(
        px.iter()
            .map(|p| json!([p.x, p.y, p.w, p.h, p.color]))
            .collect(),
    )
}

fn owned(st: &AppState, id: &str) -> bool {
    item(id).is_some_and(|i| i.free || st.wardrobe.contains(id))
}

/// Публічний код вигляду для списку профілів: колір шкіри × 8 + колір волосся.
pub(crate) fn avatar_code(skin: u8, hair_color: u8) -> u8 {
    skin.min(5) * 8 + hair_color.min(7)
}

/// Герой зі списку профілів (до входу): лише колір шкіри й волосся, решта за замовчуванням.
pub(crate) fn avatar_px(code: u8) -> Value {
    let look = Look {
        skin: (code / 8).min(5),
        hair_color: code % 8,
        ..Look::default()
    };
    px_json(&compose(&look))
}

/// Вибір для створення героя до входу в профіль: кольори й вільні зачіски.
pub(crate) fn options_json() -> Value {
    json!({
        "skins": SKINS.iter().enumerate().map(|(i, (c, _, n))| json!({"i": i, "color": c, "name": n})).collect::<Vec<_>>(),
        "hair_colors": HAIR_COLORS.iter().enumerate().map(|(i, (c, n))| json!({"i": i, "color": c, "name": n})).collect::<Vec<_>>(),
        "hair_styles": items().iter().filter(|i| i.slot == Slot::Hair && i.free).map(|i| json!({"id": i.id, "name": i.name})).collect::<Vec<_>>(),
    })
}

/// Прев'ю героя до створення профілю.
pub(crate) fn preview_json(args: &Value) -> Value {
    let skin = args
        .get("skin")
        .and_then(Value::as_u64)
        .map_or(1, |n| n.min(5) as u8);
    let hair_color = args
        .get("hair_color")
        .and_then(Value::as_u64)
        .map_or(1, |n| n.min(7) as u8);
    let mut look = Look {
        skin,
        hair_color,
        ..Look::default()
    };
    if let Some(it) = args
        .get("hair_style")
        .and_then(Value::as_str)
        .and_then(item)
    {
        if it.free && it.slot == Slot::Hair {
            look.equipped.insert(Slot::Hair, it.id.to_string());
        }
    }
    json!({"px": px_json(&compose(&look))})
}

/// Лише вигляд героя (для головної й постера).
pub(crate) fn look_json(st: &AppState) -> Value {
    json!({
        "skin": st.hero.skin, "hair_color": st.hero.hair_color,
        "equipped": st.hero.equipped.iter().map(|(s, id)| (s.code().to_string(), json!(id))).collect::<serde_json::Map<_, _>>(),
        "px": px_json(&compose(&st.hero)),
    })
}

/// Увесь каталог: кожен предмет у прев'ю на вигляді дитини (колір шкіри й волосся збережено).
pub(crate) fn catalog_json(st: &AppState) -> Value {
    let slots: Vec<Value> = Slot::ALL
        .into_iter()
        .map(|slot| {
            let list: Vec<Value> = items()
                .iter()
                .filter(|i| i.slot == slot)
                .map(|i| {
                    // Прев'ю: герой у цьому предметі.
                    let mut preview = st.hero.clone();
                    preview.equipped.insert(slot, i.id.to_string());
                    json!({
                        "id": i.id, "name": i.name, "rarity": i.rarity.code(), "free": i.free,
                        "owned": owned(st, i.id),
                        "equipped": st.hero.equipped.get(&slot).is_some_and(|e| e == i.id),
                        "px": px_json(&compose(&preview)),
                        "own_px": px_json(&item_px(i, &st.hero)),
                    })
                })
                .collect();
            json!({"slot": slot.code(), "title": slot.title(), "items": list,
                   "removable": !matches!(slot, Slot::Outfit | Slot::Hair | Slot::Bg)})
        })
        .collect();
    json!({
        "look": look_json(st),
        "skins": SKINS.iter().enumerate().map(|(i, (c, _, n))| json!({"i": i, "color": c, "name": n})).collect::<Vec<_>>(),
        "hair_colors": HAIR_COLORS.iter().enumerate().map(|(i, (c, n))| json!({"i": i, "color": c, "name": n})).collect::<Vec<_>>(),
        "slots": slots,
        "collected": st.wardrobe.len(),
        "collection_total": items().iter().filter(|i| !i.free).count(),
    })
}

/// Змінює вигляд: колір шкіри, колір волосся, одягнути чи зняти предмет. Одягнути можна лише власне.
pub(crate) fn set(st: &mut AppState, args: &Value) -> Result<Value, ApiError> {
    if let Some(n) = args.get("skin").and_then(Value::as_u64) {
        st.hero.skin = u8::try_from(n)
            .ok()
            .filter(|n| usize::from(*n) < SKINS.len())
            .ok_or_else(|| ApiError::bad_request("невідомий колір шкіри"))?;
    }
    if let Some(n) = args.get("hair_color").and_then(Value::as_u64) {
        st.hero.hair_color = u8::try_from(n)
            .ok()
            .filter(|n| usize::from(*n) < HAIR_COLORS.len())
            .ok_or_else(|| ApiError::bad_request("невідомий колір волосся"))?;
    }
    if let Some(slot) = args.get("slot").and_then(Value::as_str) {
        let slot = Slot::from_code(slot).ok_or_else(|| ApiError::bad_request("невідомий шар"))?;
        match args.get("item").and_then(Value::as_str) {
            None | Some("") => {
                if matches!(slot, Slot::Outfit | Slot::Hair | Slot::Bg) {
                    return Err(ApiError::bad_request("цей шар не можна зняти"));
                }
                st.hero.equipped.remove(&slot);
            }
            Some(id) => {
                let it = item(id)
                    .filter(|i| i.slot == slot)
                    .ok_or_else(|| ApiError::bad_request("невідомий предмет"))?;
                if !owned(st, it.id) {
                    return Err(ApiError::bad_request("цього предмета ще немає"));
                }
                st.hero.equipped.insert(slot, it.id.to_string());
            }
        }
    }
    st.profile.avatar = avatar_code(st.hero.skin, st.hero.hair_color);
    Ok(look_json(st))
}

/// Постер героя для друку: повісити на стіну чи показати батькам.
pub(crate) fn poster_pdf(st: &AppState, weeks_done: usize) -> Result<Vec<u8>, ApiError> {
    use report::canvas::{Align, Canvas, FontId, Rgb, A4};
    let err = || ApiError::new("pdf", "Не вдалося створити постер");
    let mut c = Canvas::new(A4).map_err(|_| err())?;
    let (w, h) = (A4.0, A4.1);
    let ink = Rgb::hex("#1b2a6b");
    // Тло: аркуш у клітинку.
    c.rect(0.0, 0.0, w, h, 0.0, Some(Rgb::hex("#f3f7fb")), None);
    let mut gx = 0.0;
    while gx < w {
        c.line(gx, 0.0, gx, h, Rgb::hex("#d3e0ee"), 0.4, false);
        gx += 24.0;
    }
    let mut gy = 0.0;
    while gy < h {
        c.line(0.0, gy, w, gy, Rgb::hex("#d3e0ee"), 0.4, false);
        gy += 24.0;
    }
    c.rect(18.0, 18.0, w - 36.0, h - 36.0, 18.0, None, Some((ink, 3.0)));
    c.text(
        FontId::Hand,
        44.0,
        w / 2.0,
        h - 90.0,
        Align::Center,
        ink,
        "Мій герой",
    );
    let name = if st.profile.nickname.is_empty() {
        "Герой"
    } else {
        st.profile.nickname.as_str()
    };
    c.text(
        FontId::Bold,
        22.0,
        w / 2.0,
        h - 122.0,
        Align::Center,
        Rgb::hex("#2b4bd6"),
        name,
    );
    // Герой: піксель = 17 пт.
    let px = 17.0;
    let (hw, hh) = (16.0 * px, 20.0 * px);
    let (ox, oy) = ((w - hw) / 2.0, h - 150.0 - hh);
    c.rect(
        ox - 8.0,
        oy - 8.0,
        hw + 16.0,
        hh + 16.0,
        10.0,
        Some(Rgb::hex("#ffffff")),
        Some((ink, 2.5)),
    );
    for p in compose(&st.hero) {
        c.rect(
            ox + f32::from(p.x) * px,
            oy + (20.0 - f32::from(p.y) - f32::from(p.h)) * px,
            f32::from(p.w) * px,
            f32::from(p.h) * px,
            0.0,
            Some(Rgb::hex(&p.color)),
            None,
        );
    }
    // Підсумок збору.
    let total = items().iter().filter(|i| !i.free).count();
    let y0 = oy - 36.0;
    c.text(
        FontId::Bold,
        15.0,
        w / 2.0,
        y0,
        Align::Center,
        ink,
        &format!("Зібрано предметів: {} з {}", st.wardrobe.len(), total),
    );
    c.text(
        FontId::Body,
        12.0,
        w / 2.0,
        y0 - 20.0,
        Align::Center,
        Rgb::hex("#4a5a96"),
        &format!("Тижнів зі старанністю: {weeks_done}"),
    );
    // Список здобутого чипами.
    let mut x = 48.0;
    let mut y = y0 - 56.0;
    for id in &st.wardrobe {
        let Some(it) = item(id) else { continue };
        let tw = c.text_width(FontId::Body, 10.5, it.name) + 18.0;
        if x + tw > w - 48.0 {
            x = 48.0;
            y -= 24.0;
        }
        if y < 70.0 {
            break;
        }
        let col = match it.rarity {
            tutor_core::hero::Rarity::Common => "#dfe7ff",
            tutor_core::hero::Rarity::Rare => "#fff3c2",
            tutor_core::hero::Rarity::Epic => "#ffd9e2",
        };
        c.rect(
            x,
            y - 6.0,
            tw,
            19.0,
            9.0,
            Some(Rgb::hex(col)),
            Some((ink, 1.0)),
        );
        c.text(FontId::Body, 10.5, x + 9.0, y, Align::Left, ink, it.name);
        x += tw + 8.0;
    }
    c.text(
        FontId::Hand,
        20.0,
        w / 2.0,
        44.0,
        Align::Center,
        ink,
        "Старанність робить героя сильнішим. Kmit",
    );
    c.finish("Мій герой").map_err(|_| err())
}

use std::collections::BTreeSet;
use tutor_core::hero::{
    compose, item, items, layers, Look, Rarity, Slot, GRID_H, GRID_W, HAIR_COLORS, SKINS,
};

fn valid_color(c: &str) -> bool {
    matches!(c, "$S" | "$D" | "$H")
        || (c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit()))
}

#[test]
fn every_item_is_well_formed_and_fits_the_grid() {
    let mut ids = BTreeSet::new();
    for it in items() {
        assert!(ids.insert(it.id), "повторний id {}", it.id);
        let width = it.rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
        for r in it.rows {
            assert_eq!(r.chars().count(), width, "{}: рядки різної довжини", it.id);
            for ch in r.chars().filter(|c| *c != '.') {
                let color = it.pal.iter().find(|(k, _)| *k == ch).map(|(_, v)| *v);
                assert!(
                    color.is_some_and(valid_color),
                    "{}: немає кольору для «{ch}»",
                    it.id
                );
            }
        }
        if !it.rows.is_empty() {
            assert!(
                usize::from(it.x) + width <= usize::from(GRID_W),
                "{}: виходить за ширину",
                it.id
            );
            assert!(
                usize::from(it.y) + it.rows.len() <= usize::from(GRID_H),
                "{}: виходить за висоту",
                it.id
            );
        }
        for (x, y, w, h, c) in it.rects {
            assert!(valid_color(c), "{}: колір {c}", it.id);
            assert!(
                *w > 0
                    && *h > 0
                    && u16::from(*x + *w) <= u16::from(GRID_W)
                    && u16::from(*y + *h) <= u16::from(GRID_H),
                "{}: прямокутник",
                it.id
            );
        }
        assert!(
            !it.rows.is_empty() || !it.rects.is_empty(),
            "{}: порожній",
            it.id
        );
        assert!(!it.name.trim().is_empty());
    }
}

#[test]
fn the_default_look_is_complete_and_the_wardrobe_has_enough_to_collect() {
    let look = Look::default();
    for (slot, id) in &look.equipped {
        let it = item(id).unwrap();
        assert_eq!(it.slot, *slot);
        assert!(it.free, "{id} має бути вільним");
    }
    // Вільні: щонайменше шість зачісок, один одяг, одне тло.
    let free = |s: Slot| items().iter().filter(|i| i.slot == s && i.free).count();
    assert!(free(Slot::Hair) >= 6 && free(Slot::Outfit) >= 1 && free(Slot::Bg) >= 1);
    // Збирати: на навчальний рік (36 тижнів) вистачає, і є всі три рівні рідкості.
    let collectible: Vec<_> = items().iter().filter(|i| !i.free).collect();
    assert!(collectible.len() >= 36, "предметів: {}", collectible.len());
    for r in [Rarity::Common, Rarity::Rare, Rarity::Epic] {
        assert!(
            collectible.iter().filter(|i| i.rarity == r).count() >= 3,
            "{r:?}"
        );
    }
    for s in [
        Slot::Hat,
        Slot::Back,
        Slot::Hand,
        Slot::Pet,
        Slot::Outfit,
        Slot::Bg,
        Slot::Frame,
    ] {
        assert!(collectible.iter().any(|i| i.slot == s), "{s:?}");
    }
}

#[test]
fn composition_draws_layers_in_order_with_the_childs_colors_and_stays_on_the_grid() {
    let mut look = Look {
        skin: 4,
        hair_color: 5,
        ..Look::default()
    };
    let px = compose(&look);
    assert!(!px.is_empty());
    assert!(px
        .iter()
        .all(|p| p.x + p.w <= GRID_W && p.y + p.h <= GRID_H && p.w > 0));
    assert!(
        px.iter().any(|p| p.color == SKINS[4].0),
        "колір шкіри застосовано"
    );
    assert!(
        px.iter().any(|p| p.color == HAIR_COLORS[5].0),
        "колір волосся застосовано"
    );
    assert!(
        px.iter().all(|p| p.color.starts_with('#')),
        "жодного невизначеного кольору"
    );
    // Порядок шарів: тіло між одягом і волоссям; одягнений убір малюється після волосся.
    look.equipped.insert(Slot::Hat, "hat_crown".into());
    look.equipped.insert(Slot::Frame, "frame_gold".into());
    let names: Vec<&str> = layers(&look).iter().map(|(n, _)| *n).collect();
    assert_eq!(names, ["bg", "outfit", "body", "hair", "hat", "frame"]);
    // Один і той самий вигляд завжди дає однакові прямокутники.
    assert_eq!(compose(&look), compose(&look));
}

//! Свято переходу до наступного класу. Перехід нічим не блокується: свято пропонується, коли в класі опановано
//! щонайменше `PROMOTE_PERCENT` % тем, але дитина з дорослим може перейти й раніше, а повернутися назад можна завжди.
//! Нагороди за теми свято не видає (інваріант «нагорода не за дарма»): стартова істота нового світу це подарунок
//! від самого переходу, а не винагорода за результат.

use crate::app::Env;
use crate::catalog;
use crate::error::ApiError;
use serde_json::{json, Value};
use storage::state::AppState;
use tutor_core::album::StickerKind;

/// Частка опанованих тем, з якої застосунок пропонує свято на головній.
pub(crate) const PROMOTE_PERCENT: u32 = 80;

/// Опановано тем і тем усього в класі.
pub(crate) fn progress(env: &Env, state: &AppState, grade: u8) -> (usize, usize) {
    let ids: Vec<&str> = env
        .graph
        .ids()
        .into_iter()
        .filter(|id| catalog::grade_of(id) == grade)
        .collect();
    let mastered = ids
        .iter()
        .filter(|id| state.learner.is_mastered(id))
        .count();
    (mastered, ids.len())
}

fn percent(mastered: usize, total: usize) -> u32 {
    (mastered * 100)
        .checked_div(total)
        .and_then(|p| u32::try_from(p).ok())
        .unwrap_or(0)
}

/// Наступний клас, якщо для нього вже є вміст.
fn next_ready(grade: u8) -> Option<u8> {
    let to = grade.checked_add(1)?;
    crate::grades::valid(to).then_some(())?;
    crate::grades::is_ready(to).then_some(to)
}

/// Останній клас (підготовка до НМТ): після нього наступного класу немає, тож замість переходу «кінець освіти» у Kmit
/// (випускне свято): без нового світу, зі свідоцтвом.
pub(crate) fn is_final(grade: u8) -> bool {
    grade == crate::grades::NMT
}

/// Чи пропонувати свято на головній: опановано достатньо, є куди переходити (або це випускний), свято цього класу ще не було.
pub(crate) fn offer(env: &Env, state: &AppState) -> Option<Value> {
    let from = state.settings.grade;
    let to = next_ready(from);
    if (to.is_none() && !is_final(from)) || state.ceremonies.contains(&from) {
        return None;
    }
    let (m, t) = progress(env, state, from);
    (percent(m, t) >= PROMOTE_PERCENT)
        .then(|| json!({"from": from, "to": to, "final": is_final(from), "percent": percent(m, t)}))
}

/// Підсумок за всі класи разом (для випускного): опановано, тем, нагород.
fn overall(env: &Env, state: &AppState) -> Value {
    let (mut m, mut t, mut s) = (0usize, 0usize, 0usize);
    for g in crate::grades::READY_GRADE..=crate::grades::NMT {
        let (gm, gt) = progress(env, state, g);
        m += gm;
        t += gt;
        s += stickers_in(env, state, g)["total"].as_u64().unwrap_or(0) as usize;
    }
    json!({"mastered": m, "total": t, "stickers": s})
}

fn stickers_in(env: &Env, state: &AppState, grade: u8) -> Value {
    let mut by = serde_json::Map::new();
    let mut total = 0usize;
    for kind in StickerKind::ALL {
        let n = env
            .graph
            .ids()
            .into_iter()
            .filter(|id| catalog::grade_of(id) == grade && state.album.has(kind, id))
            .count();
        total += n;
        by.insert(format!("{kind:?}").to_lowercase(), json!(n));
    }
    json!({"total": total, "by_kind": by})
}

/// Дані для екрана свята: підсумок пройденого класу без порівнянь і опис нового світу.
pub(crate) fn status_json(env: &Env, state: &AppState) -> Value {
    let from = state.settings.grade;
    let to = next_ready(from);
    let (m, t) = progress(env, state, from);
    let (world_id, world_name) = catalog::world_of(from);
    let next_world = to.map(catalog::world_of);
    json!({
        "from": from,
        "to": to,
        "final": is_final(from),
        "can": to.is_some() || is_final(from),
        "overall": overall(env, state),
        "done_before": state.ceremonies.contains(&from),
        "mastered": m,
        "total": t,
        "percent": percent(m, t),
        "offer_percent": PROMOTE_PERCENT,
        "stickers": stickers_in(env, state, from),
        "world": {"id": world_id, "name": world_name},
        "next_world": next_world.map(|(id, name)| json!({"id": id, "name": name})),
        "left": t.saturating_sub(m),
    })
}

/// Виконує перехід: фіксує свято, перемикає клас. Повертає підсумок класу, який щойно завершено.
pub(crate) fn run(env: &Env, state: &mut AppState) -> Result<Value, ApiError> {
    let from = state.settings.grade;
    if is_final(from) {
        // Кінець шляху: свято фіксується, клас лишається тим самим (колекції лишаються доступними).
        let summary = status_json(env, state);
        state.ceremonies.insert(from);
        return Ok(summary);
    }
    let to =
        next_ready(from).ok_or_else(|| ApiError::bad_request("Наступний клас ще готується"))?;
    let summary = status_json(env, state);
    state.ceremonies.insert(from);
    state.settings.grade = to;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_is_safe_for_empty_classes() {
        assert_eq!(percent(0, 0), 0);
        assert_eq!(percent(4, 5), 80);
        assert_eq!(percent(79, 100), 79);
    }

    #[test]
    fn the_next_grade_must_exist_and_have_content() {
        assert_eq!(next_ready(5), Some(6));
        assert_eq!(next_ready(6), Some(7));
        assert_eq!(next_ready(7), None, "8 клас ще без вмісту");
        assert_eq!(next_ready(12), None);
        assert!(is_final(12) && !is_final(11));
    }

    #[test]
    fn every_grade_has_a_world_and_new_grades_repeat_in_a_circle() {
        assert_eq!(catalog::world_of(5).0, "forest");
        assert_eq!(catalog::world_of(6).0, "sea");
        assert_eq!(catalog::world_of(7).0, "sky");
        let ids: std::collections::BTreeSet<&str> =
            (5..=12).map(|g| catalog::world_of(g).0).collect();
        assert_eq!(ids.len(), 8, "кожен клас 5–12 має власний світ");
        assert_eq!(catalog::world_of(8).0, "cave");
        assert_eq!(catalog::world_of(12).0, "summit");
        assert_eq!(catalog::world_of(13).0, "forest", "поза діапазоном по колу");
    }
}

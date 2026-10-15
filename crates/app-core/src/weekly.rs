//! «Лінійка тижня»: що дитина зробила за тиждень (старанність, а не оцінки) і нагорода раз на тиждень, один із трьох
//! предметів для піксельного героя. Нічого не віднімається: слабкий тиждень просто не дає вибору, наступний починається
//! з чистого аркуша. Одиниці старанності (за день не більше трьох):
//!   1. поспрацював: щонайменше 5 задач, над якими справді думав (не відповідав навмання за кілька секунд);
//!   2. пройшов урок до кінця (приклади розібрано, питання розв'язано самостійно, пояснення здано);
//!   3. повторив: завершив повторення чи пригадування.
//!
//! Плюс до двох подоланих складних прикладів за тиждень. Поділки лінійки: 3, 6, 9, 12, 15 одиниць.

use serde_json::{json, Value};
use storage::state::{AppState, WeekRec, MAX_WEEKS};
use tutor_core::hero::{item, items, Rarity, Slot};

/// Скільки одиниць відкриває кожну поділку.
pub(crate) const MARKS: [u32; 5] = [3, 6, 9, 12, 15];
/// Скільки задач за день вважається «поспрацював».
const TASKS_PER_DAY: u8 = 5;
/// Скільки секунд на задачу вважається справжньою спробою.
pub(crate) const ENGAGED_SECS: u32 = 8;
/// Нагороду за минулі тижні можна забрати протягом стількох тижнів.
const CLAIM_WEEKS: u32 = 8;

/// Номер тижня (від понеділка) і день тижня 0..=6 для номера місцевого дня (день 0 — це четвер, 1 січня 1970).
pub(crate) fn week_of(day: u32) -> (u32, usize) {
    ((day + 3) / 7, ((day + 3) % 7) as usize)
}

fn rec_mut(st: &mut AppState, day: u32) -> (&mut WeekRec, usize) {
    let (w, wd) = week_of(day);
    st.weeks.entry(w).or_default();
    // Зберігаємо лише останні тижні.
    while st.weeks.len() > MAX_WEEKS {
        if let Some(first) = st.weeks.keys().next().copied() {
            st.weeks.remove(&first);
        }
    }
    (st.weeks.entry(w).or_default(), wd)
}

/// Задача, над якою дитина справді думала.
pub(crate) fn note_task(st: &mut AppState, day: u32) {
    let (r, wd) = rec_mut(st, day);
    r.tasks[wd] = r.tasks[wd].saturating_add(1);
}

pub(crate) fn note_lesson(st: &mut AppState, day: u32) {
    let (r, wd) = rec_mut(st, day);
    r.lessons[wd] = r.lessons[wd].saturating_add(1);
}

pub(crate) fn note_review(st: &mut AppState, day: u32) {
    let (r, wd) = rec_mut(st, day);
    r.reviews[wd] = r.reviews[wd].saturating_add(1);
}

pub(crate) fn note_gap(st: &mut AppState, day: u32) {
    let (r, _) = rec_mut(st, day);
    r.gaps = r.gaps.saturating_add(1);
}

pub(crate) struct Summary {
    pub units: u32,
    pub active_days: u32,
    pub lessons: u32,
    pub reviews: u32,
    pub gaps: u32,
    pub tasks: u32,
}

pub(crate) fn summarize(r: &WeekRec) -> Summary {
    let mut units = 0;
    let mut active_days = 0;
    for d in 0..7 {
        let a = u32::from(r.tasks[d] >= TASKS_PER_DAY);
        let b = u32::from(r.lessons[d] > 0);
        let c = u32::from(r.reviews[d] > 0);
        units += a + b + c;
        active_days += u32::from(a + b + c > 0);
    }
    let gaps = u32::from(r.gaps);
    units += gaps.min(2);
    Summary {
        units,
        active_days,
        lessons: r.lessons.iter().map(|x| u32::from(*x)).sum(),
        reviews: r.reviews.iter().map(|x| u32::from(*x)).sum(),
        gaps,
        tasks: r.tasks.iter().map(|x| u32::from(*x)).sum(),
    }
}

/// Рівень нагороди: 0 немає, 1 звичайна, 2 рідкісна, 3 дуже рідкісна.
pub(crate) fn tier_of(units: u32) -> u8 {
    match units {
        15.. => 3,
        12.. => 2,
        9.. => 1,
        _ => 0,
    }
}

/// Три предмети на вибір. Однаковий результат для однакових вхідних (тиждень, скільки вже зібрано, рівень), тож перегляд
/// не міняє пропозиції. Різні шари, якщо вдається; щонайменше один предмет найвищої доступної рідкості.
pub(crate) fn offer(st: &AppState, week: u32, tier: u8) -> Vec<&'static str> {
    if tier == 0 {
        return Vec::new();
    }
    let top = match tier {
        3 => Rarity::Epic,
        2 => Rarity::Rare,
        _ => Rarity::Common,
    };
    let pool: Vec<&'static tutor_core::hero::Item> = items()
        .iter()
        .filter(|i| !i.free && i.rarity <= top && !st.wardrobe.contains(i.id))
        .collect();
    let mut seed = u64::from(week).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (st.wardrobe.len() as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ u64::from(tier);
    let mut next = |n: usize| -> usize {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((seed >> 33) as usize) % n.max(1)
    };
    let mut chosen: Vec<&'static tutor_core::hero::Item> = Vec::new();
    // Спершу предмет найвищої рідкості (якщо рівень вищий за звичайний).
    let tops: Vec<_> = pool.iter().copied().filter(|i| i.rarity == top).collect();
    if tier > 1 && !tops.is_empty() {
        chosen.push(tops[next(tops.len())]);
    }
    let mut guard = 0;
    while chosen.len() < 3 && chosen.len() < pool.len() && guard < 200 {
        guard += 1;
        let cand = pool[next(pool.len())];
        if chosen.iter().any(|c| c.id == cand.id) {
            continue;
        }
        // Різні шари, доки є з чого обирати.
        let slots_left = pool
            .iter()
            .any(|p| !chosen.iter().any(|c| c.slot == p.slot || c.id == p.id));
        if slots_left && chosen.iter().any(|c| c.slot == cand.slot) && guard < 120 {
            continue;
        }
        chosen.push(cand);
    }
    chosen.iter().map(|i| i.id).collect()
}

fn item_json(id: &str) -> Value {
    item(id).map_or(Value::Null, |i| {
        json!({"id": i.id, "name": i.name, "slot": i.slot.code(), "slot_title": i.slot.title(), "rarity": i.rarity.code()})
    })
}

fn week_json(st: &AppState, week: u32, today_week: u32) -> Value {
    let rec = st.weeks.get(&week).cloned().unwrap_or_default();
    let s = summarize(&rec);
    let tier = tier_of(s.units);
    let claimed = rec.claimed.clone();
    let offer_ids = if claimed.is_none() {
        offer(st, week, tier)
    } else {
        Vec::new()
    };
    json!({
        "week": week, "current": week == today_week, "units": s.units, "tier": tier,
        "marks": MARKS, "breakdown": {"active_days": s.active_days, "lessons": s.lessons,
            "reviews": s.reviews, "gaps": s.gaps, "tasks": s.tasks},
        "claimed": claimed.as_deref().map(item_json),
        "offer": offer_ids.iter().map(|id| item_json(id)).collect::<Vec<_>>(),
        "can_claim": claimed.is_none() && tier > 0 && !offer_ids.is_empty(),
    })
}

/// Стан поточного тижня й незабраних нагород за минулі тижні.
pub(crate) fn status(st: &AppState, day: u32) -> Value {
    let (w, _) = week_of(day);
    let pending: Vec<Value> = (w.saturating_sub(CLAIM_WEEKS)..w)
        .rev()
        .filter(|k| st.weeks.contains_key(k))
        .map(|k| week_json(st, k, w))
        .filter(|v| v["can_claim"] == true)
        .collect();
    let total = items().iter().filter(|i| !i.free).count();
    json!({"current": week_json(st, w, w), "pending": pending,
           "collected": st.wardrobe.len(), "collection_total": total})
}

/// Дитина обирає предмет нагородою за тиждень `week`. Лише з пропозиції цього тижня; раз на тиждень.
pub(crate) fn claim(
    st: &mut AppState,
    day: u32,
    week: u32,
    item_id: &str,
) -> Result<Value, &'static str> {
    let (w, _) = week_of(day);
    if week > w || w - week > CLAIM_WEEKS {
        return Err("цей тиждень уже не можна забрати");
    }
    let rec = st.weeks.get(&week).cloned().unwrap_or_default();
    if rec.claimed.is_some() {
        return Err("нагороду за цей тиждень уже забрано");
    }
    let tier = tier_of(summarize(&rec).units);
    if !offer(st, week, tier).contains(&item_id) {
        return Err("цього предмета немає серед пропозицій");
    }
    st.wardrobe.insert(item_id.to_string());
    st.weeks.entry(week).or_default().claimed = Some(item_id.to_string());
    // Новий предмет одразу одягається (дитина може змінити).
    if let Some(it) = item(item_id) {
        if it.slot != Slot::Hair {
            st.hero.equipped.insert(it.slot, it.id.to_string());
        }
    }
    Ok(json!({"item": item_json(item_id)}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weeks_start_on_monday_and_marks_follow_effort_not_correctness() {
        // День 0 — це четвер 1.01.1970; день 4 — це понеділок 5.01.1970.
        assert_eq!(week_of(0), (0, 3));
        assert_eq!(week_of(4), (1, 0));
        assert_eq!(week_of(10), (1, 6));
        assert_eq!(tier_of(8), 0);
        assert_eq!(tier_of(9), 1);
        assert_eq!(tier_of(12), 2);
        assert_eq!(tier_of(20), 3);
    }

    #[test]
    fn the_offer_is_stable_has_three_distinct_unowned_items_and_scales_with_the_tier() {
        let mut st = AppState::new("Т", 1);
        for tier in 1..=3u8 {
            let a = offer(&st, 2900, tier);
            assert_eq!(a, offer(&st, 2900, tier), "стабільна пропозиція");
            assert_eq!(a.len(), 3);
            let slots: std::collections::BTreeSet<_> =
                a.iter().filter_map(|id| item(id)).map(|i| i.slot).collect();
            assert!(slots.len() >= 2, "різні шари");
            let top = a
                .iter()
                .filter_map(|id| item(id))
                .map(|i| i.rarity)
                .max()
                .unwrap();
            assert!(
                top <= match tier {
                    1 => Rarity::Common,
                    2 => Rarity::Rare,
                    _ => Rarity::Epic,
                }
            );
        }
        assert!(offer(&st, 2900, 0).is_empty());
        // Дуже рідкісний рівень гарантує щонайменше один найрідкісніший предмет.
        assert!(offer(&st, 2900, 3)
            .iter()
            .filter_map(|id| item(id))
            .any(|i| i.rarity == Rarity::Epic));
        // Уже здобуте не пропонується знову.
        let first = offer(&st, 2900, 1)[0];
        st.wardrobe.insert(first.to_string());
        assert!(!offer(&st, 2900, 1).contains(&first));
    }
}

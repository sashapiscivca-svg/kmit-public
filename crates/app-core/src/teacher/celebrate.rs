//! Свята кабінету вчителя: невеликі приємні події, щоб користуватися застосунком було приємно. Жодних даних учнів:
//! лише власна робота вчителя (створені роботи) і календар. Кожне свято показується один раз.

use crate::datefmt::{days_of, weekday, ymd_of};
use serde_json::{json, Value};
use storage::state::AppState;

/// Скільки створених робіт відзначаємо.
const MILESTONES: [u32; 5] = [1, 10, 25, 50, 100];

fn milestone_text(n: u32) -> (&'static str, &'static str) {
    match n {
        1 => (
            "Перша робота готова!",
            "Дякуємо, що ви з нами. Перша робота створена за хвилину, наступні будуть ще швидшими.",
        ),
        10 => (
            "Десять робіт!",
            "Вже десять робіт, які не довелося складати вручну. Kmit створений, щоб прибирати рутину, а не час із дітьми.",
        ),
        25 => (
            "Двадцять п'ять робіт!",
            "Двадцять п'ять варіантів з ключами й бланками. Ваш час тепер іде на уроки, а не на копіювання.",
        ),
        50 => (
            "П'ятдесят робіт!",
            "П'ятдесят робіт створено. Дякуємо за довіру й за те, що тримаєте вимоги до якості високо.",
        ),
        _ => (
            "Сто робіт!",
            "Сто робіт: це справжня бібліотека. Дякуємо, що ви працюєте з нами, і за кожну виправлену помилку в матеріалах.",
        ),
    }
}

/// Свято, яке варто показати зараз (календарне важливіше за віху), якщо його ще не бачили.
pub(crate) fn pending(state: &AppState, day: u32) -> Option<Value> {
    let (year, month, _) = ymd_of(day);
    let seen = &state.teacher.celebrated;
    let today = i64::from(day);
    // День працівників освіти: перша неділя жовтня й тиждень після неї.
    let first = days_of(year, 10, 1);
    let sunday = first + (6 - weekday(first)).rem_euclid(7);
    let id = format!("td:{year}");
    if (sunday..sunday + 7).contains(&today) && !seen.contains(&id) {
        return Some(
            json!({"id": id, "kind": "calendar", "title": "З Днем працівників освіти!",
            "text": "Дякуємо вам за працю, терпіння й віру в дітей. Нехай цього тижня знайдеться час для чашки чаю й теплих слів від учнів."}),
        );
    }
    // День знань: перший тиждень вересня.
    let id = format!("dk:{year}");
    if month == 9 && today < days_of(year, 9, 8) && !seen.contains(&id) {
        return Some(
            json!({"id": id, "kind": "calendar", "title": "З Днем знань!",
            "text": "Нехай новий навчальний рік буде спокійним і радісним. Роботи, плани уроків і бібліотека тем уже чекають на вас."}),
        );
    }
    MILESTONES
        .iter()
        .find(|n| state.teacher.works_made >= **n && !seen.contains(&format!("works:{n}")))
        .map(|n| {
            let (title, text) = milestone_text(*n);
            json!({"id": format!("works:{n}"), "kind": "milestone", "title": title, "text": text})
        })
}

/// Позначає свято баченим. Приймає лише ідентифікатори, схожі на справжні (`works:10`, `td:2026`).
pub(crate) fn seen(state: &mut AppState, id: &str) -> bool {
    let ok = !id.is_empty()
        && id.len() <= 24
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == ':');
    if ok {
        state.teacher.celebrated.insert(id.to_string());
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i64, m: u32, d: u32) -> u32 {
        u32::try_from(days_of(y, m, d)).unwrap()
    }

    #[test]
    fn teachers_day_is_the_first_sunday_of_october_and_lasts_a_week() {
        let st = AppState::new("Х", 0);
        // 2026: перша неділя жовтня 4-го.
        assert!(pending(&st, day(2026, 10, 3)).is_none());
        assert_eq!(pending(&st, day(2026, 10, 4)).unwrap()["id"], "td:2026");
        assert_eq!(pending(&st, day(2026, 10, 10)).unwrap()["id"], "td:2026");
        assert!(pending(&st, day(2026, 10, 11)).is_none());
        // 2027: перша неділя жовтня 3-го.
        assert_eq!(pending(&st, day(2027, 10, 3)).unwrap()["id"], "td:2027");
    }

    #[test]
    fn the_knowledge_day_week_and_milestones_show_once() {
        let mut st = AppState::new("Х", 0);
        assert_eq!(pending(&st, day(2026, 9, 1)).unwrap()["id"], "dk:2026");
        assert!(pending(&st, day(2026, 9, 8)).is_none());
        assert!(seen(&mut st, "dk:2026"));
        assert!(pending(&st, day(2026, 9, 2)).is_none(), "лише один раз");
        st.teacher.works_made = 12;
        let first = pending(&st, day(2026, 6, 1)).unwrap();
        assert_eq!(first["id"], "works:1");
        seen(&mut st, "works:1");
        assert_eq!(pending(&st, day(2026, 6, 1)).unwrap()["id"], "works:10");
        seen(&mut st, "works:10");
        assert!(pending(&st, day(2026, 6, 1)).is_none());
    }

    #[test]
    fn only_sane_ids_are_accepted() {
        let mut st = AppState::new("Х", 0);
        assert!(!seen(&mut st, ""));
        assert!(!seen(&mut st, "../../etc"));
        assert!(!seen(&mut st, &"a".repeat(40)));
        assert!(st.teacher.celebrated.is_empty());
    }
}

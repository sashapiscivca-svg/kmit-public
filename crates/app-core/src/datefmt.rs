//! Дати для документів і свят із числа днів від 1970-01-01 (годинника в ядрі немає, день приходить від інтерфейсу).

/// Рік, місяць, день за числом днів від 1970-01-01 (алгоритм цивільного календаря).
pub(crate) fn ymd_of(day: u32) -> (i64, u32, u32) {
    let z = i64::from(day) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let m = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// Число днів від 1970-01-01 для дати (обернена до `ymd_of`).
pub(crate) fn days_of(y: i64, m: u32, d: u32) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = i64::from(if m > 2 { m - 3 } else { m + 9 });
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// День тижня: 0 = понеділок … 6 = неділя (1970-01-01 був четвергом).
pub(crate) fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

/// `РРРР-ММ-ДД`.
pub(crate) fn date_of(day: u32) -> String {
    let (y, m, d) = ymd_of(day);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_right() {
        assert_eq!(date_of(0), "1970-01-01");
        assert_eq!(date_of(20_000), "2024-10-04");
        assert_eq!(date_of(20_646), "2026-07-12");
    }

    #[test]
    fn days_and_dates_are_inverse_and_weekdays_are_right() {
        for day in [0u32, 59, 60, 365, 11_017, 20_000, 20_646, 30_000] {
            let (y, m, d) = ymd_of(day);
            assert_eq!(days_of(y, m, d), i64::from(day), "{y}-{m}-{d}");
        }
        assert_eq!(weekday(0), 3, "1970-01-01 четвер");
        assert_eq!(weekday(days_of(2026, 10, 4)), 6, "4 жовтня 2026 неділя");
    }
}

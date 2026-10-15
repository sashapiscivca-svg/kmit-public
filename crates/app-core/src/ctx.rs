//! Контекст часу кожного виклику. Годинник читає оболонка; ядро застосунку нічого не читає саме.

use serde_json::Value;

#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    /// Секунди (стінний годинник пристрою).
    pub wall: u64,
    /// Номер місцевого дня.
    pub day: u32,
    /// Хвилина місцевої доби, 0..1439.
    pub minute: u16,
}

impl Ctx {
    /// З JSON-об'єкта `{wall, day, minute}`; відсутні чи хибні поля стають безпечними нулями.
    pub fn from_json(v: &Value) -> Ctx {
        let wall = v.get("wall").and_then(Value::as_u64).unwrap_or(0);
        let day = v
            .get("day")
            .and_then(Value::as_u64)
            .and_then(|d| u32::try_from(d).ok())
            .unwrap_or(0);
        let minute = v
            .get("minute")
            .and_then(Value::as_u64)
            .and_then(|m| u16::try_from(m).ok())
            .map_or(0, |m| m.min(1439));
        Ctx { wall, day, minute }
    }
}

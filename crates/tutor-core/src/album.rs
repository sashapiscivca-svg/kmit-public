//! Альбом стікерів (spec розділ 8): скінченний, нагороди за опанування й стратегії, нічого не відбирається.
//! Нагорода не залежить від часу в застосунку, кількості входів чи швидкості. Тексти нагород живуть у контенті.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StickerKind {
    /// Навичку опановано.
    Mastery,
    /// Не здалася: розв'язала після кількох невдалих спроб.
    Persisted,
    /// Помітила помилку сама: після невдалої спроби розв'язала без підказок.
    SelfCorrected,
    /// Попросила підказку й розв'язала.
    AskedAndSolved,
}

impl StickerKind {
    pub const ALL: [StickerKind; 4] = [
        StickerKind::Mastery,
        StickerKind::Persisted,
        StickerKind::SelfCorrected,
        StickerKind::AskedAndSolved,
    ];

    fn code(self) -> &'static str {
        match self {
            StickerKind::Mastery => "mastery",
            StickerKind::Persisted => "persisted",
            StickerKind::SelfCorrected => "self_corrected",
            StickerKind::AskedAndSolved => "asked_and_solved",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StickerId {
    pub kind: StickerKind,
    pub skill: String,
}

impl StickerId {
    /// Стабільний ключ для сховища й для текстів у контенті: `kind:SKILL`.
    pub fn key(&self) -> String {
        format!("{}:{}", self.kind.code(), self.skill)
    }
}

/// Факти про щойно завершену задачу, на які дивиться альбом. Лише поведінка, ніяких часів.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskFacts {
    pub solved: bool,
    pub wrong_attempts: u32,
    pub hints_used: u8,
    pub asked_adult: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Album {
    awarded: BTreeSet<StickerId>,
}

impl Album {
    pub fn new() -> Self {
        Self::default()
    }

    /// Альбом із раніше збережених ключів (наприклад, зі сховища); невідомі ключі ігноруються.
    pub fn from_keys<'a>(keys: impl IntoIterator<Item = &'a str>) -> Self {
        let mut a = Album::new();
        for k in keys {
            if let Some((code, skill)) = k.split_once(':') {
                if let Some(kind) = StickerKind::ALL.iter().copied().find(|x| x.code() == code) {
                    a.awarded.insert(StickerId {
                        kind,
                        skill: skill.to_string(),
                    });
                }
            }
        }
        a
    }

    pub fn keys(&self) -> Vec<String> {
        self.awarded.iter().map(StickerId::key).collect()
    }

    pub fn has(&self, kind: StickerKind, skill: &str) -> bool {
        self.awarded.contains(&StickerId {
            kind,
            skill: skill.to_string(),
        })
    }

    pub fn count(&self) -> usize {
        self.awarded.len()
    }

    /// Скільки стікерів у альбомі всього для заданої кількості навичок (скінченність).
    pub fn capacity(skills: usize) -> usize {
        skills * StickerKind::ALL.len()
    }

    fn award(&mut self, kind: StickerKind, skill: &str) -> Option<StickerId> {
        let id = StickerId {
            kind,
            skill: skill.to_string(),
        };
        self.awarded.insert(id.clone()).then_some(id)
    }

    /// Стікер за опанування навички.
    pub fn award_mastery(&mut self, skill: &str) -> Option<StickerId> {
        self.award(StickerKind::Mastery, skill)
    }

    /// Стікери за стратегію після задачі. Повертає лише нові; кожен стікер дається один раз за навичку.
    pub fn award_for_task(&mut self, skill: &str, f: TaskFacts) -> Vec<StickerId> {
        let mut out = Vec::new();
        // Передана дорослому задача не дає жодних стікерів: стратегії відзначаємо лише за самостійне розв'язання.
        if !f.solved || f.asked_adult {
            return out;
        }
        if f.wrong_attempts >= 2 {
            out.extend(self.award(StickerKind::Persisted, skill));
        }
        if f.wrong_attempts >= 1 && f.hints_used == 0 {
            out.extend(self.award(StickerKind::SelfCorrected, skill));
        }
        if f.hints_used >= 1 {
            out.extend(self.award(StickerKind::AskedAndSolved, skill));
        }
        out
    }
}

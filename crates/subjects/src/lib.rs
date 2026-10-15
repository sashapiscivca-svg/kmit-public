//! Предмети 5 класу, крім математики: завдання з вибором і з введенням слова, уроки й граф тем.
//! Вміст — це ДАНІ: JSON-пакети `packs/*.json` (формат `kmit-pack-1`, див. `pack.rs` і docs/content-packs.md), вбудовані
//! в застосунок; новіші підписані пакети можна додати під час запуску (`install_extra`).
//! Принцип (рішення C42): для фактичних знань після власної спроби показується правильна відповідь із поясненням,
//! повторення розкладається за інтервалами. Увесь вміст авторський і потребує перегляду методистом (Q7, A15).

pub mod pack;

use std::sync::OnceLock;
use tutor_core::bank::{BankGenerator, TypedBankGenerator};
use tutor_core::generator::SkillGenerator;
use tutor_core::lesson::{Lesson, LessonSource};

/// Тема (навичка) предмета з передумовами.
#[derive(Clone, Debug)]
pub struct SubjectSkill {
    pub id: &'static str,
    pub title: &'static str,
    pub prerequisites: &'static [&'static str],
}

#[derive(Clone, Debug)]
pub struct Subject {
    /// Предмет (спільний для класів): `history`, `ukrainian`…
    pub id: &'static str,
    /// Пакет, з якого походить цей предмет у цьому класі.
    pub pack: &'static str,
    pub grade: u8,
    pub title: &'static str,
    pub skills: Vec<SubjectSkill>,
}

/// Вбудовані пакети (порядок = порядок предметів у застосунку).
const BASE_PACKS: [&str; 23] = [
    include_str!("../packs/math.json"),
    include_str!("../packs/ukrainian.json"),
    include_str!("../packs/literature.json"),
    include_str!("../packs/english.json"),
    include_str!("../packs/history.json"),
    include_str!("../packs/nature.json"),
    include_str!("../packs/informatics.json"),
    include_str!("../packs/ukrainian_g6.json"),
    include_str!("../packs/literature_g6.json"),
    include_str!("../packs/english_g6.json"),
    include_str!("../packs/history_g6.json"),
    include_str!("../packs/ukrainian_g7.json"),
    include_str!("../packs/literature_g7.json"),
    include_str!("../packs/english_g7.json"),
    include_str!("../packs/history_g7.json"),
    include_str!("../packs/biology_g6.json"),
    include_str!("../packs/geography_g6.json"),
    include_str!("../packs/informatics_g6.json"),
    include_str!("../packs/biology_g7.json"),
    include_str!("../packs/chemistry_g7.json"),
    include_str!("../packs/geography_g7.json"),
    include_str!("../packs/informatics_g7.json"),
    include_str!("../packs/physics_g7.json"),
];

struct Registry {
    subjects: Vec<Subject>,
    banks: Vec<BankGenerator>,
    typed: Vec<TypedBankGenerator>,
    lessons: Vec<Lesson>,
    versions: Vec<(String, u32)>,
    /// Статус перевірки за темою: (тема, статус пакета).
    review: Vec<(&'static str, String)>,
}

static EXTRA: OnceLock<Vec<pack::Pack>> = OnceLock::new();
static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Додає пакети (наприклад, оновлені, перевірені за підписом) до вбудованих. Пакет з тим самим id і більшою версією
/// замінює вбудований, пакет нового предмета додається. Викликати до першого звернення до вмісту; `false`, якщо запізно.
pub fn install_extra(packs: Vec<pack::Pack>) -> bool {
    REGISTRY.get().is_none() && EXTRA.set(packs).is_ok()
}

fn registry() -> &'static Registry {
    REGISTRY.get_or_init(|| {
        let mut packs: Vec<pack::Pack> = BASE_PACKS
            .iter()
            .map(|t| pack::parse(t.as_bytes()).expect("вбудований пакет вмісту коректний"))
            .collect();
        for extra in EXTRA.get().cloned().unwrap_or_default() {
            match packs.iter().position(|p| p.id == extra.id) {
                Some(i) if packs[i].version < extra.version => packs[i] = extra,
                Some(_) => {}
                None => packs.push(extra),
            }
        }
        let mut r = Registry {
            subjects: Vec::new(),
            banks: Vec::new(),
            typed: Vec::new(),
            lessons: Vec::new(),
            versions: Vec::new(),
            review: Vec::new(),
        };
        for p in &packs {
            let b = pack::build(p);
            for l in &b.lessons {
                r.review.push((l.skill, p.review.status.clone()));
            }
            for k in &b.subject.skills {
                if !r.review.iter().any(|(s, _)| *s == k.id) {
                    r.review.push((k.id, p.review.status.clone()));
                }
            }
            r.versions.push((p.id.clone(), p.version));
            if !b.subject.skills.is_empty() {
                r.subjects.push(b.subject);
            }
            r.banks.extend(b.banks);
            r.typed.extend(b.typed);
            r.lessons.extend(b.lessons);
        }
        r
    })
}

/// Версії чинних пакетів: (id предмета, версія).
pub fn pack_versions() -> Vec<(String, u32)> {
    registry().versions.clone()
}

/// Версії вбудованих пакетів (для рішення, чи нове оновлення новіше).
pub fn base_versions() -> Vec<(String, u32)> {
    BASE_PACKS
        .iter()
        .filter_map(|t| pack::parse(t.as_bytes()).ok())
        .map(|p| (p.id, p.version))
        .collect()
}

/// Усі предмети цього крейта.
pub fn subjects() -> Vec<Subject> {
    registry().subjects.clone()
}

/// Банки з вибором відповіді для перевірок якості авторського вмісту (клони дешеві: вміст спільний).
pub fn all_banks() -> Vec<BankGenerator> {
    registry().banks.clone()
}

/// Банки із введенням слова для перевірок якості вмісту.
pub fn all_typed_banks() -> Vec<TypedBankGenerator> {
    registry().typed.clone()
}

pub fn generator_for(skill: &str) -> Option<Box<dyn SkillGenerator>> {
    let r = registry();
    r.banks
        .iter()
        .find(|g| g.skill_id() == skill)
        .map(|g| Box::new(g.clone()) as Box<dyn SkillGenerator>)
        .or_else(|| {
            r.typed
                .iter()
                .find(|g| g.skill_id() == skill)
                .map(|g| Box::new(g.clone()) as Box<dyn SkillGenerator>)
        })
}

pub fn all_generators() -> Vec<Box<dyn SkillGenerator>> {
    let r = registry();
    r.banks
        .iter()
        .map(|g| Box::new(g.clone()) as Box<dyn SkillGenerator>)
        .chain(
            r.typed
                .iter()
                .map(|g| Box::new(g.clone()) as Box<dyn SkillGenerator>),
        )
        .collect()
}

pub struct SubjectLessons;

impl LessonSource for SubjectLessons {
    fn lesson(&self, skill: &str) -> Option<Lesson> {
        registry()
            .lessons
            .iter()
            .find(|l| l.skill == skill)
            .cloned()
    }
}

/// Предмет, до якого належить тема.
pub fn subject_of(skill: &str) -> Option<&'static str> {
    registry()
        .subjects
        .iter()
        .find(|s| s.skills.iter().any(|k| k.id == skill))
        .map(|s| s.id)
}

/// Клас, до якого належить тема (`None`, якщо теми немає серед предметів цього крейта).
pub fn grade_of(skill: &str) -> Option<u8> {
    registry()
        .subjects
        .iter()
        .find(|s| s.skills.iter().any(|k| k.id == skill))
        .map(|s| s.grade)
}

/// Класи, для яких є вміст предметів (за зростанням).
pub fn grades() -> Vec<u8> {
    let mut v: Vec<u8> = registry().subjects.iter().map(|s| s.grade).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// Чинний вміст у вигляді пакетів (для експорту, огляду й підпису).
pub fn current_packs() -> Vec<pack::Pack> {
    let r = registry();
    r.subjects
        .iter()
        .map(|s| {
            let v = r
                .versions
                .iter()
                .find(|(id, _)| id == s.pack)
                .map_or(1, |(_, v)| *v);
            pack::export(s.pack, v, s, &r.banks, &r.typed, &r.lessons)
        })
        .collect()
}

/// Пакет, до якого належить тема (лише він, без копіювання решти вмісту).
pub fn pack_for_skill(skill: &str) -> Option<pack::Pack> {
    let r = registry();
    let s = r
        .subjects
        .iter()
        .find(|s| s.skills.iter().any(|k| k.id == skill))?;
    let v = r
        .versions
        .iter()
        .find(|(id, _)| id == s.pack)
        .map_or(1, |(_, v)| *v);
    Some(pack::export(s.pack, v, s, &r.banks, &r.typed, &r.lessons))
}

/// Ідентифікатори тем, для яких є урок (включно з темами, заданими деінде, наприклад математика).
pub fn lesson_skill_ids() -> Vec<&'static str> {
    registry().lessons.iter().map(|l| l.skill).collect()
}

/// Стан перевірки вмісту теми людиною: `draft`, `reviewed` чи `approved` (немає вмісту: `draft`).
pub fn review_status(skill: &str) -> String {
    registry()
        .review
        .iter()
        .find(|(s, _)| *s == skill)
        .map_or_else(|| "draft".to_string(), |(_, st)| st.clone())
}

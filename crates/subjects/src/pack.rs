//! Пакет вмісту предмета як дані (JSON): теми, банки завдань, завдання з введенням слова, уроки.
//! Методисти й автори працюють із цим форматом, а не з кодом; застосунок читає його при запуску.
//! Формат версії `kmit-pack-1`. Опис і приклад: docs/content-packs.md.

use serde::{Deserialize, Serialize};
use tutor_core::bank::{BankGenerator, BankItem, TypedBankGenerator, TypedItem};
use tutor_core::figure::{DiagramKind, Figure};
use tutor_core::lesson::{Lesson, LessonBlock};

pub const SCHEMA: &str = "kmit-pack-1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Pack {
    pub schema: String,
    pub id: String,
    pub version: u32,
    pub title: String,
    /// Предмет, до якого належить пакет (за замовчуванням — id пакета): кілька пакетів різних класів ділять предмет.
    #[serde(default)]
    pub subject: String,
    /// Клас, для якого пакет (за замовчуванням 5).
    #[serde(default = "default_grade")]
    pub grade: u8,
    pub skills: Vec<PSkill>,
    #[serde(default)]
    pub banks: Vec<PBank>,
    #[serde(default)]
    pub typed: Vec<PTyped>,
    #[serde(default)]
    pub lessons: Vec<PLesson>,
    /// Хто й коли перевірив вміст пакета (методист). Без запису пакет вважається чернеткою.
    #[serde(default)]
    pub review: Review,
}

fn default_grade() -> u8 {
    5
}

/// Стан перевірки пакета людиною: `draft` (чернетка), `reviewed` (вичитано), `approved` (схвалено до використання).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Review {
    #[serde(default = "draft")]
    pub status: String,
    #[serde(default)]
    pub by: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub notes: String,
}

fn draft() -> String {
    "draft".to_string()
}

impl Default for Review {
    fn default() -> Self {
        Review {
            status: draft(),
            by: String::new(),
            date: String::new(),
            notes: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PSkill {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub prerequisites: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PBank {
    pub skill: String,
    pub items: Vec<PItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PItem {
    pub prompt: String,
    pub correct: String,
    pub wrong: Vec<String>,
    pub explanation: String,
    pub hints: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PTyped {
    pub skill: String,
    pub items: Vec<PTypedItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PTypedItem {
    pub prompt: String,
    pub accepted: Vec<String>,
    pub explanation: String,
    pub hints: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PLesson {
    pub skill: String,
    pub title: String,
    pub note_prompts: Vec<String>,
    pub blocks: Vec<PBlock>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PBlock {
    Text {
        text: String,
    },
    Example {
        statement: String,
        steps: Vec<String>,
        result: String,
    },
    Selfcheck {
        prompt: String,
        answer: String,
        explanation: String,
    },
    Quiz {
        prompt: String,
        options: Vec<String>,
        correct: usize,
        explanation: String,
    },
    Remember {
        text: String,
    },
    Figure {
        figure: PFigure,
        caption: String,
    },
    Hook {
        prompt: String,
    },
    Term {
        term: String,
        definition: String,
    },
    Explain {
        prompt: String,
        key_ideas: Vec<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PEvent {
    pub label: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PBranch {
    pub name: String,
    pub items: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PBar {
    pub label: String,
    pub value: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PFigure {
    Timeline {
        events: Vec<PEvent>,
    },
    Compare {
        head: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Cycle {
        steps: Vec<String>,
    },
    Flow {
        steps: Vec<String>,
    },
    Mindmap {
        center: String,
        branches: Vec<PBranch>,
    },
    Bars {
        unit: String,
        items: Vec<PBar>,
    },
    Sets {
        a: String,
        b: String,
        only_a: Vec<String>,
        both: Vec<String>,
        only_b: Vec<String>,
    },
    FractionBar {
        num: i64,
        den: i64,
    },
    DecimalGrid {
        hundredths: i64,
    },
    Rect {
        w: String,
        h: String,
    },
    Angle {
        deg: i64,
        label: String,
    },
    NumberLine {
        min: i64,
        max: i64,
        parts: i64,
        marks: Vec<PMark>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PMark {
    pub pos: i64,
    pub label: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PackError {
    Json(String),
    Schema,
    Invalid(String),
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackError::Json(e) => write!(f, "пакет не читається: {e}"),
            PackError::Schema => write!(f, "невідомий формат пакета"),
            PackError::Invalid(e) => write!(f, "пакет хибний: {e}"),
        }
    }
}

/// Розбирає пакет і перевіряє його цілісність (без довіри до джерела: файл може бути пошкоджений).
pub fn parse(bytes: &[u8]) -> Result<Pack, PackError> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(PackError::Invalid("пакет завеликий".into()));
    }
    let pack: Pack = serde_json::from_slice(bytes).map_err(|e| PackError::Json(e.to_string()))?;
    if pack.schema != SCHEMA {
        return Err(PackError::Schema);
    }
    validate(&pack)?;
    Ok(pack)
}

fn bad<T>(m: impl Into<String>) -> Result<T, PackError> {
    Err(PackError::Invalid(m.into()))
}

fn validate(p: &Pack) -> Result<(), PackError> {
    if !["draft", "reviewed", "approved"].contains(&p.review.status.as_str()) {
        return bad("хибний статус перевірки");
    }
    if p.id.is_empty()
        || p.id.len() > 40
        || !p
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return bad("хибний id пакета");
    }
    if !(5..=11).contains(&p.grade)
        || p.subject.len() > 40
        || !p
            .subject
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '_')
    {
        return bad("хибний клас чи предмет пакета");
    }
    // Пакет без тем можливий лише як «пакет уроків» (уроки до тем, що задані деінде, наприклад математика).
    let lessons_only =
        p.skills.is_empty() && p.banks.is_empty() && p.typed.is_empty() && !p.lessons.is_empty();
    if (p.skills.is_empty() && !lessons_only) || p.skills.len() > 200 {
        return bad("кількість тем поза межами");
    }
    let ids: std::collections::BTreeSet<&str> = p.skills.iter().map(|s| s.id.as_str()).collect();
    if ids.len() != p.skills.len() {
        return bad("id тем повторюються");
    }
    for s in &p.skills {
        if s.id.is_empty()
            || s.id.len() > 40
            || s.id.contains(|c: char| c.is_whitespace() || c == '|')
        {
            return bad(format!("хибний id теми {}", s.id));
        }
        if s.title.trim().is_empty() || s.title.chars().count() > 120 {
            return bad(format!("хибна назва теми {}", s.id));
        }
        for q in &s.prerequisites {
            if !ids.contains(q.as_str()) {
                return bad(format!("тема {}: невідома передумова {q}", s.id));
            }
        }
    }
    // Ациклічність передумов.
    let mut done = std::collections::BTreeSet::new();
    while done.len() < p.skills.len() {
        let before = done.len();
        for s in &p.skills {
            if !done.contains(s.id.as_str())
                && s.prerequisites.iter().all(|q| done.contains(q.as_str()))
            {
                done.insert(s.id.as_str());
            }
        }
        if done.len() == before {
            return bad("передумови утворюють цикл");
        }
    }
    let known = |id: &str| lessons_only || ids.contains(id);
    for b in &p.banks {
        if !known(&b.skill) || b.items.is_empty() || b.items.len() > 2000 {
            return bad(format!("банк теми {}: хибна тема чи обсяг", b.skill));
        }
        for it in &b.items {
            if it.wrong.len() < 2 || it.wrong.len() > 5 || it.correct.is_empty() {
                return bad(format!(
                    "тема {}: завдання має мати 2-5 хибних варіантів і правильний",
                    b.skill
                ));
            }
        }
    }
    for t in &p.typed {
        if !known(&t.skill) || t.items.is_empty() || t.items.len() > 2000 {
            return bad(format!(
                "теми із введенням слова {}: хибна тема чи обсяг",
                t.skill
            ));
        }
        if t.items.iter().any(|i| i.accepted.is_empty()) {
            return bad(format!(
                "тема {}: завдання без прийнятної відповіді",
                t.skill
            ));
        }
    }
    for l in &p.lessons {
        if !known(&l.skill) {
            return bad(format!("урок невідомої теми {}", l.skill));
        }
        for b in &l.blocks {
            if let PBlock::Quiz {
                options, correct, ..
            } = b
            {
                if *correct >= options.len() || options.len() < 2 {
                    return bad(format!("урок {}: хибне питання з вибором", l.skill));
                }
            }
            if let PBlock::Explain { key_ideas, .. } = b {
                if !(2..=6).contains(&key_ideas.len())
                    || key_ideas.iter().any(|k| k.trim().is_empty())
                {
                    return bad(format!(
                        "урок {}: у поясненні має бути 2-6 ключових ідей",
                        l.skill
                    ));
                }
            }
            if let PBlock::Figure { figure, .. } = b {
                if !to_figure(figure).is_sane() {
                    return bad(format!("урок {}: хибний малюнок", l.skill));
                }
            }
        }
    }
    Ok(())
}

fn leak(s: &str) -> &'static str {
    // Пакети завантажуються один раз за запуск; їхні рядки живуть до кінця процесу (кілька сотень кілобайт).
    Box::leak(s.to_string().into_boxed_str())
}

fn diagram(kind: DiagramKind, title: &str, rows: Vec<Vec<String>>) -> Figure {
    Figure::Diagram {
        kind,
        title: title.to_string(),
        rows,
    }
}

fn to_figure(f: &PFigure) -> Figure {
    match f {
        PFigure::Timeline { events } => diagram(
            DiagramKind::Timeline,
            "",
            events
                .iter()
                .map(|e| vec![e.label.clone(), e.text.clone()])
                .collect(),
        ),
        PFigure::Compare { head, rows } => diagram(
            DiagramKind::Compare,
            "",
            std::iter::once(head.clone())
                .chain(rows.iter().cloned())
                .collect(),
        ),
        PFigure::Cycle { steps } => diagram(
            DiagramKind::Cycle,
            "",
            steps.iter().map(|s| vec![s.clone()]).collect(),
        ),
        PFigure::Flow { steps } => diagram(
            DiagramKind::Flow,
            "",
            steps.iter().map(|s| vec![s.clone()]).collect(),
        ),
        PFigure::Mindmap { center, branches } => diagram(
            DiagramKind::Mindmap,
            "",
            std::iter::once(vec![center.clone()])
                .chain(branches.iter().map(|b| {
                    std::iter::once(b.name.clone())
                        .chain(b.items.iter().cloned())
                        .collect()
                }))
                .collect(),
        ),
        PFigure::Bars { unit, items } => diagram(
            DiagramKind::Bars,
            unit,
            items
                .iter()
                .map(|i| vec![i.label.clone(), i.value.to_string()])
                .collect(),
        ),
        PFigure::Sets {
            a,
            b,
            only_a,
            both,
            only_b,
        } => diagram(
            DiagramKind::Sets,
            "",
            vec![
                vec![a.clone(), b.clone()],
                only_a.clone(),
                both.clone(),
                only_b.clone(),
            ],
        ),
        PFigure::FractionBar { num, den } => Figure::FractionBar {
            num: *num,
            den: *den,
        },
        PFigure::DecimalGrid { hundredths } => Figure::DecimalGrid {
            hundredths: *hundredths,
        },
        PFigure::Rect { w, h } => Figure::Rect {
            w: w.clone(),
            h: h.clone(),
        },
        PFigure::Angle { deg, label } => Figure::Angle {
            deg: *deg,
            label: label.clone(),
        },
        PFigure::NumberLine {
            min,
            max,
            parts,
            marks,
        } => Figure::NumberLine {
            min: *min,
            max: *max,
            parts: *parts,
            marks: marks.iter().map(|m| (m.pos, m.label.clone())).collect(),
        },
    }
}

fn from_figure(f: &Figure) -> PFigure {
    match f {
        Figure::Diagram { kind, title, rows } => from_diagram(*kind, title, rows),
        Figure::FractionBar { num, den } => PFigure::FractionBar {
            num: *num,
            den: *den,
        },
        Figure::DecimalGrid { hundredths } => PFigure::DecimalGrid {
            hundredths: *hundredths,
        },
        Figure::Rect { w, h } => PFigure::Rect {
            w: w.clone(),
            h: h.clone(),
        },
        Figure::Angle { deg, label } => PFigure::Angle {
            deg: *deg,
            label: label.clone(),
        },
        Figure::NumberLine {
            min,
            max,
            parts,
            marks,
        } => PFigure::NumberLine {
            min: *min,
            max: *max,
            parts: *parts,
            marks: marks
                .iter()
                .map(|(p, t)| PMark {
                    pos: *p,
                    label: t.clone(),
                })
                .collect(),
        },
    }
}

/// Готові до використання об'єкти пакета.
pub struct Built {
    pub subject: crate::Subject,
    pub banks: Vec<BankGenerator>,
    pub typed: Vec<TypedBankGenerator>,
    pub lessons: Vec<Lesson>,
}

pub fn build(p: &Pack) -> Built {
    let skills = p
        .skills
        .iter()
        .map(|s| {
            let prereqs: Vec<&'static str> = s.prerequisites.iter().map(|q| leak(q)).collect();
            crate::SubjectSkill {
                id: leak(&s.id),
                title: leak(&s.title),
                prerequisites: Box::leak(prereqs.into_boxed_slice()),
            }
        })
        .collect();
    let banks = p
        .banks
        .iter()
        .map(|b| {
            BankGenerator::new(
                leak(&b.skill),
                b.items
                    .iter()
                    .map(|i| BankItem {
                        prompt: i.prompt.clone(),
                        correct: i.correct.clone(),
                        wrong: i.wrong.clone(),
                        explanation: i.explanation.clone(),
                        hints: i.hints.clone(),
                    })
                    .collect(),
            )
        })
        .collect();
    let typed = p
        .typed
        .iter()
        .map(|t| {
            TypedBankGenerator::new(
                leak(&t.skill),
                t.items
                    .iter()
                    .map(|i| TypedItem {
                        prompt: i.prompt.clone(),
                        accepted: i.accepted.clone(),
                        explanation: i.explanation.clone(),
                        hints: i.hints.clone(),
                    })
                    .collect(),
            )
        })
        .collect();
    let lessons = p
        .lessons
        .iter()
        .map(|l| Lesson {
            skill: leak(&l.skill),
            title: l.title.clone(),
            note_prompts: l.note_prompts.clone(),
            blocks: l
                .blocks
                .iter()
                .map(|b| match b {
                    PBlock::Text { text } => LessonBlock::Text(text.clone()),
                    PBlock::Example {
                        statement,
                        steps,
                        result,
                    } => LessonBlock::Example {
                        statement: statement.clone(),
                        steps: steps.clone(),
                        result: result.clone(),
                    },
                    PBlock::Selfcheck {
                        prompt,
                        answer,
                        explanation,
                    } => LessonBlock::SelfCheck {
                        prompt: prompt.clone(),
                        answer: answer.clone(),
                        explanation: explanation.clone(),
                    },
                    PBlock::Quiz {
                        prompt,
                        options,
                        correct,
                        explanation,
                    } => {
                        // Автори ставлять правильний варіант першим; дитина не повинна це помічати,
                        // тому порядок детерміновано обертається за умовою (стабільно між запусками).
                        let n = options.len().max(1);
                        let r = quiz_shift(prompt, n);
                        let mut opts = options.clone();
                        opts.rotate_right(r);
                        LessonBlock::Quiz {
                            prompt: prompt.clone(),
                            options: opts,
                            correct: (*correct + r) % n,
                            explanation: explanation.clone(),
                        }
                    }
                    PBlock::Remember { text } => LessonBlock::Remember(text.clone()),
                    PBlock::Hook { prompt } => LessonBlock::Hook(prompt.clone()),
                    PBlock::Term { term, definition } => LessonBlock::Term {
                        term: term.clone(),
                        definition: definition.clone(),
                    },
                    PBlock::Explain { prompt, key_ideas } => LessonBlock::Explain {
                        prompt: prompt.clone(),
                        key_ideas: key_ideas.clone(),
                    },
                    PBlock::Figure { figure, caption } => LessonBlock::Figure {
                        figure: to_figure(figure),
                        caption: caption.clone(),
                    },
                })
                .collect(),
        })
        .collect();
    Built {
        subject: crate::Subject {
            id: leak(if p.subject.is_empty() {
                &p.id
            } else {
                &p.subject
            }),
            pack: leak(&p.id),
            grade: p.grade,
            title: leak(&p.title),
            skills,
        },
        banks,
        typed,
        lessons,
    }
}

/// Вміст у вигляді пакета (для експорту наявного вмісту й перевірок рівності).
pub fn export(
    id: &str,
    version: u32,
    subject: &crate::Subject,
    banks: &[BankGenerator],
    typed: &[TypedBankGenerator],
    lessons: &[Lesson],
) -> Pack {
    use tutor_core::generator::SkillGenerator;
    let have = |skill: &str| subject.skills.iter().any(|s| s.id == skill);
    Pack {
        review: Review::default(),
        schema: SCHEMA.to_string(),
        id: id.to_string(),
        version,
        title: subject.title.to_string(),
        subject: if subject.id == id {
            String::new()
        } else {
            subject.id.to_string()
        },
        grade: subject.grade,
        skills: subject
            .skills
            .iter()
            .map(|s| PSkill {
                id: s.id.to_string(),
                title: s.title.to_string(),
                prerequisites: s.prerequisites.iter().map(|q| (*q).to_string()).collect(),
            })
            .collect(),
        banks: banks
            .iter()
            .filter(|b| have(b.skill_id()))
            .map(|b| PBank {
                skill: b.skill_id().to_string(),
                items: b
                    .items()
                    .iter()
                    .map(|i| PItem {
                        prompt: i.prompt.clone(),
                        correct: i.correct.clone(),
                        wrong: i.wrong.clone(),
                        explanation: i.explanation.clone(),
                        hints: i.hints.clone(),
                    })
                    .collect(),
            })
            .collect(),
        typed: typed
            .iter()
            .filter(|b| have(b.skill_id()))
            .map(|b| PTyped {
                skill: b.skill_id().to_string(),
                items: b
                    .items()
                    .iter()
                    .map(|i| PTypedItem {
                        prompt: i.prompt.clone(),
                        accepted: i.accepted.clone(),
                        explanation: i.explanation.clone(),
                        hints: i.hints.clone(),
                    })
                    .collect(),
            })
            .collect(),
        lessons: lessons
            .iter()
            .filter(|l| have(l.skill))
            .map(|l| PLesson {
                skill: l.skill.to_string(),
                title: l.title.clone(),
                note_prompts: l.note_prompts.clone(),
                blocks: l
                    .blocks
                    .iter()
                    .map(|b| match b {
                        LessonBlock::Text(t) => PBlock::Text { text: t.clone() },
                        LessonBlock::Example {
                            statement,
                            steps,
                            result,
                        } => PBlock::Example {
                            statement: statement.clone(),
                            steps: steps.clone(),
                            result: result.clone(),
                        },
                        LessonBlock::SelfCheck {
                            prompt,
                            answer,
                            explanation,
                        } => PBlock::Selfcheck {
                            prompt: prompt.clone(),
                            answer: answer.clone(),
                            explanation: explanation.clone(),
                        },
                        LessonBlock::Quiz {
                            prompt,
                            options,
                            correct,
                            explanation,
                        } => {
                            // Обернути назад обертання з `build`: вихід збігається з авторським порядком.
                            let n = options.len().max(1);
                            let r = quiz_shift(prompt, n);
                            let mut opts = options.clone();
                            opts.rotate_left(r);
                            PBlock::Quiz {
                                prompt: prompt.clone(),
                                options: opts,
                                correct: (*correct + n - r) % n,
                                explanation: explanation.clone(),
                            }
                        }
                        LessonBlock::Remember(t) => PBlock::Remember { text: t.clone() },
                        LessonBlock::Hook(p) => PBlock::Hook { prompt: p.clone() },
                        LessonBlock::Term { term, definition } => PBlock::Term {
                            term: term.clone(),
                            definition: definition.clone(),
                        },
                        LessonBlock::Explain { prompt, key_ideas } => PBlock::Explain {
                            prompt: prompt.clone(),
                            key_ideas: key_ideas.clone(),
                        },
                        LessonBlock::Figure { figure, caption } => PBlock::Figure {
                            figure: from_figure(figure),
                            caption: caption.clone(),
                        },
                    })
                    .collect(),
            })
            .collect(),
    }
}

// ---- Підпис пакетів ----

/// Відкритий ключ видавця пакетів (Ed25519). Зараз це ТЕСТОВИЙ ключ розробки: перед публікацією його замінює справжній
/// ключ власника (питання Q30); приватна частина ніколи не потрапляє в репозиторій.
pub const PUBLISHER_PUBLIC_KEY: [u8; 32] = DEV_PUBLIC_KEY;
const DEV_PUBLIC_KEY: [u8; 32] = [
    0xb7, 0x2e, 0x5c, 0x40, 0xe5, 0x23, 0x07, 0xce, 0x1d, 0x8c, 0x34, 0xb2, 0xd9, 0x24, 0x99, 0xa8,
    0x80, 0x67, 0x17, 0x81, 0x0b, 0xce, 0x03, 0xc6, 0xc7, 0x85, 0x41, 0xc3, 0xf7, 0x1b, 0x8a, 0x59,
];

#[derive(Debug, PartialEq, Eq)]
pub enum VerifyError {
    BadKey,
    BadSignature,
}

/// Перевіряє підпис пакета відкритим ключем видавця.
pub fn verify_signature(
    bytes: &[u8],
    signature: &[u8],
    public_key: &[u8; 32],
) -> Result<(), VerifyError> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let key = VerifyingKey::from_bytes(public_key).map_err(|_| VerifyError::BadKey)?;
    let sig = Signature::from_slice(signature).map_err(|_| VerifyError::BadSignature)?;
    key.verify(bytes, &sig)
        .map_err(|_| VerifyError::BadSignature)
}

/// Зсув варіантів уроку: залежить лише від умови, тож стабільний між запусками.
fn quiz_shift(prompt: &str, n: usize) -> usize {
    prompt.bytes().map(usize::from).sum::<usize>() % n
}

/// Зворотне перетворення схеми (для експорту пакета). Сюди потрапляють лише схеми, створені з пакета, тож форма рядків відома.
fn from_diagram(kind: DiagramKind, title: &str, rows: &[Vec<String>]) -> PFigure {
    let first = |r: &Vec<String>| r.first().cloned().unwrap_or_default();
    match kind {
        DiagramKind::Timeline => PFigure::Timeline {
            events: rows
                .iter()
                .map(|r| PEvent {
                    label: r[0].clone(),
                    text: r[1].clone(),
                })
                .collect(),
        },
        DiagramKind::Compare => PFigure::Compare {
            head: rows[0].clone(),
            rows: rows[1..].to_vec(),
        },
        DiagramKind::Cycle => PFigure::Cycle {
            steps: rows.iter().map(first).collect(),
        },
        DiagramKind::Flow => PFigure::Flow {
            steps: rows.iter().map(first).collect(),
        },
        DiagramKind::Mindmap => PFigure::Mindmap {
            center: first(&rows[0]),
            branches: rows[1..]
                .iter()
                .map(|r| PBranch {
                    name: r[0].clone(),
                    items: r[1..].to_vec(),
                })
                .collect(),
        },
        DiagramKind::Bars => PFigure::Bars {
            unit: title.to_string(),
            items: rows
                .iter()
                .map(|r| PBar {
                    label: r[0].clone(),
                    value: r[1].parse().unwrap_or(0),
                })
                .collect(),
        },
        DiagramKind::Sets => PFigure::Sets {
            a: rows[0][0].clone(),
            b: rows[0][1].clone(),
            only_a: rows[1].clone(),
            both: rows[2].clone(),
            only_b: rows[3].clone(),
        },
    }
}

/// Пакет у вигляді читабельного JSON (для експорту й інструментів).
pub fn to_json(p: &Pack) -> String {
    serde_json::to_string_pretty(p).unwrap_or_default()
}

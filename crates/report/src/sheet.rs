//! Аркуш для вчителя: з моделі `TeacherSheet` у документ і PDF. Тон нейтральний: це підказка, а не оцінка.

use crate::pdf::{render, Block, Document, PdfError};
use tutor_core::sheet::TeacherSheet;

const NONE_YET: &str = "поки немає";

fn bullets_or_none(out: &mut Vec<Block>, items: impl Iterator<Item = String>) {
    let mut any = false;
    for it in items {
        any = true;
        out.push(Block::Bullet(it));
    }
    if !any {
        out.push(Block::Para(NONE_YET.to_string()));
    }
}

pub fn sheet_document(s: &TeacherSheet) -> Document {
    let mut b = vec![
        Block::Title(if s.subject.is_empty() {
            "Аркуш для вчителя".to_string()
        } else {
            format!("Аркуш для вчителя: {}", s.subject)
        }),
        Block::Para(
            "Це підказка для вчителя, а не оцінка. Дитина показує його сама, коли захоче."
                .to_string(),
        ),
        Block::Space(6.0),
        Block::Para("Ім’я: ______________________________   Клас: ________".to_string()),
    ];
    if !s.nickname.is_empty() {
        b.push(Block::Para(format!("Нікнейм у застосунку: {}", s.nickname)));
    }
    b.push(Block::Para(format!(
        "Стікерів в альбомі: {} із {}",
        s.stickers, s.sticker_capacity
    )));
    b.push(Block::Rule);

    b.push(Block::Heading("Що вже опановано".to_string()));
    bullets_or_none(&mut b, s.mastered.iter().map(|x| x.title.clone()));

    if !s.to_repeat.is_empty() {
        b.push(Block::Heading("Що пора повторити".to_string()));
        for x in &s.to_repeat {
            b.push(Block::Bullet(x.title.clone()));
        }
    }

    b.push(Block::Heading("Над чим працюємо зараз".to_string()));
    bullets_or_none(&mut b, s.in_progress.iter().map(|x| x.title.clone()));

    b.push(Block::Heading(
        "Над чим варто попрацювати разом".to_string(),
    ));
    if s.gaps.is_empty() {
        b.push(Block::Para(NONE_YET.to_string()));
    }
    for g in &s.gaps {
        b.push(Block::Bullet(g.title.clone()));
        for h in &g.hypotheses {
            b.push(Block::SubBullet(format!(
                "Можлива причина: {}.",
                h.trim_end_matches('.')
            )));
        }
        for i in &g.ideas {
            b.push(Block::SubBullet(format!("Ідея на 10 хвилин: {i}")));
        }
    }

    b.push(Block::Heading(
        "З чим потрібна допомога дорослого".to_string(),
    ));
    bullets_or_none(
        &mut b,
        s.ask_adult
            .iter()
            .map(|a| format!("{}: {}", a.title, a.statement)),
    );

    b.push(Block::Rule);
    b.push(Block::Para(
        "Аркуш створено на пристрої дитини за її бажанням, дані нікуди не передавалися. Він не містить відповідей до задач."
            .to_string(),
    ));
    Document {
        title: "Аркуш для вчителя".to_string(),
        blocks: b,
    }
}

pub fn sheet_pdf(s: &TeacherSheet) -> Result<Vec<u8>, PdfError> {
    render(&sheet_document(s))
}

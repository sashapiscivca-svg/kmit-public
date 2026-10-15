//! «Мій конспект»: усі власні конспекти учня одним PDF, який можна друкувати чи показати вчителю.

use crate::pdf::{render, Block, Document, PdfError};

/// `entries`: пари (назва теми, текст конспекту) у порядку курсу; порожні тексти не передаються.
pub fn notes_document(nickname: &str, entries: &[(String, String)]) -> Document {
    let mut b = vec![Block::Title("Мій конспект".to_string())];
    if !nickname.is_empty() {
        b.push(Block::Para(format!("Автор: {nickname}")));
    }
    b.push(Block::Rule);
    if entries.is_empty() {
        b.push(Block::Para("Конспект поки порожній.".to_string()));
    }
    for (title, text) in entries {
        b.push(Block::Heading(title.clone()));
        for line in text.lines() {
            let line = line.trim_end();
            if line.is_empty() {
                b.push(Block::Space(4.0));
            } else {
                b.push(Block::Para(line.to_string()));
            }
        }
    }
    Document {
        title: "Мій конспект".to_string(),
        blocks: b,
    }
}

pub fn notes_pdf(nickname: &str, entries: &[(String, String)]) -> Result<Vec<u8>, PdfError> {
    render(&notes_document(nickname, entries))
}

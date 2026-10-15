//! Читабельність пояснень для 10–11 років: довгі речення й надто довгі слова ускладнюють читання (особливо дітям
//! з дислексією). Межі помірні: це запобіжник від явних стін тексту, а не оцінка стилю.

use subjects::{subjects, SubjectLessons};
use tutor_core::lesson::{LessonBlock, LessonSource};

fn sentences(text: &str) -> Vec<String> {
    text.split(['.', '!', '?'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn explanations_use_short_sentences_and_short_words() {
    let mut worst = (0usize, String::new());
    let mut long: Vec<String> = Vec::new();
    for s in subjects() {
        for k in &s.skills {
            let l = SubjectLessons.lesson(k.id).unwrap();
            for b in &l.blocks {
                let LessonBlock::Text(t) = b else { continue };
                let sents = sentences(t);
                assert!(!sents.is_empty());
                for sent in &sents {
                    let words: Vec<&str> = sent.split_whitespace().collect();
                    if words.len() > worst.0 {
                        worst = (words.len(), format!("{}: {sent}", k.id));
                    }
                    assert!(
                        words.len() <= 40,
                        "{}: речення на {} слів: {sent}",
                        k.id,
                        words.len()
                    );
                    assert!(
                        words.iter().all(|w| w.chars().count() <= 24),
                        "{}: надто довге слово у «{sent}»",
                        k.id
                    );
                }
                let total: usize = sents.iter().map(|x| x.split_whitespace().count()).sum();
                let avg = total as f64 / sents.len() as f64;
                if avg > 22.0 {
                    long.push(format!("{}: середнє речення {avg:.1} слів", k.id));
                }
            }
        }
    }
    println!("найдовше речення: {} слів: {}", worst.0, worst.1);
    println!("абзаци з довгими реченнями: {long:?}");
    // Кілька абзаців із переліками літер чи слів (правила) можуть мати довгі «речення»; їх має бути небагато.
    assert!(
        long.len() <= 8,
        "забагато абзаців із довгими реченнями: {long:?}"
    );
}

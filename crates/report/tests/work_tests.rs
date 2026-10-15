use report::pdf::PdfError;
use report::work::{work_answers_pdf, work_key_pdf, work_tasks_pdf, ItemDoc, VariantDoc, WorkDoc};

fn doc(variants: usize, items: usize, long: bool) -> WorkDoc {
    let text = if long {
        "Дуже довге умовне речення про те, що саме треба зробити в цьому завданні, ".repeat(3)
    } else {
        "Обчисли: 1,5 + 2,25".to_string()
    };
    WorkDoc {
        scale: 1.0,
        title: "Контрольна робота".into(),
        subtitle: "Додавання десяткових дробів, Апостроф".into(),
        grade: 5,
        seed: 42,
        dictation: false,
        variants: (0..variants)
            .map(|v| VariantDoc {
                label: ['А', 'Б', 'В', 'Г'][v],
                items: (0..items)
                    .map(|i| {
                        if i % 2 == 0 {
                            ItemDoc {
                                text: text.clone(),
                                choices: vec![],
                                answer: "3,75".into(),
                            }
                        } else {
                            ItemDoc {
                                text: text.clone(),
                                choices: vec![
                                    "слово".into(),
                                    "слів'я".into(),
                                    "словце".into(),
                                    "слівце".into(),
                                ],
                                answer: "слів'я".into(),
                            }
                        }
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[test]
fn all_three_pdfs_are_valid() {
    let d = doc(3, 10, false);
    for pdf in [
        work_tasks_pdf(&d).unwrap(),
        work_key_pdf(&d).unwrap(),
        work_answers_pdf(&d).unwrap(),
    ] {
        assert!(pdf.starts_with(b"%PDF-1.4"));
        let pages = String::from_utf8_lossy(&pdf)
            .matches("/Type /Page\n")
            .count()
            + String::from_utf8_lossy(&pdf)
                .matches("/Type /Page ")
                .count();
        let _ = pages;
        assert!(pdf.len() > 3_000);
    }
}

#[test]
fn long_statements_and_many_items_paginate_without_panicking() {
    let d = doc(2, 40, true);
    assert!(work_tasks_pdf(&d).unwrap().len() > 5_000);
    assert!(work_answers_pdf(&d).unwrap().len() > 5_000);
}

#[test]
fn an_empty_work_is_an_error() {
    let mut d = doc(1, 3, false);
    d.variants[0].items.clear();
    assert!(matches!(work_tasks_pdf(&d), Err(PdfError::Empty)));
    d.variants.clear();
    assert!(matches!(work_key_pdf(&d), Err(PdfError::Empty)));
}

#[test]
fn large_print_dictation_cards_and_a_topic_plan_make_valid_pdfs() {
    use report::work::{topic_plan_pdf, work_cards_pdf, TopicPlanDoc, TopicRow};
    let mut d = doc(2, 14, true);
    d.scale = 1.4;
    let big = work_tasks_pdf(&d).unwrap();
    d.scale = 1.0;
    let normal = work_tasks_pdf(&d).unwrap();
    assert!(
        big.len() > normal.len(),
        "великий шрифт займає більше місця"
    );
    d.dictation = true;
    assert!(work_tasks_pdf(&d).unwrap().starts_with(b"%PDF"));
    assert!(work_key_pdf(&d).unwrap().starts_with(b"%PDF"));
    d.dictation = false;
    assert!(work_cards_pdf(&d).unwrap().starts_with(b"%PDF"));
    let plan = TopicPlanDoc {
        title: "Тематичний план".into(),
        subtitle: "Українська мова, 6 клас".into(),
        rows: (0..40)
            .map(|i| TopicRow {
                title: format!(
                    "Тема номер {i} з досить довгою назвою, яка переноситься на другий рядок"
                ),
                lessons: 2,
            })
            .collect(),
    };
    assert!(topic_plan_pdf(&plan).unwrap().starts_with(b"%PDF"));
    assert!(matches!(
        topic_plan_pdf(&TopicPlanDoc {
            title: "x".into(),
            subtitle: "y".into(),
            rows: vec![]
        }),
        Err(PdfError::Empty)
    ));
}

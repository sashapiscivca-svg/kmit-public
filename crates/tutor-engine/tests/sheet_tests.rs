use course_pack::grade5_m0_graph;
use generators_math::MathSheetContent;
use tutor_core::album::{Album, TaskFacts};
use tutor_core::bkt::BktParams;
use tutor_core::config::Config;
use tutor_core::learner::LearnerState;
use tutor_core::sheet::{sanitize_nickname, SheetContent, NICKNAME_MAX_CHARS};
use tutor_engine::sheet::{build_sheet, MAX_GAPS, MAX_IDEAS};

/// Місткість альбому: чотири стікери на кожну навичку графа курсу.
fn album_capacity() -> usize {
    Album::capacity(grade5_m0_graph().ids().len())
}

fn env() -> (BktParams, Config, LearnerState) {
    (
        BktParams::numeric_default(),
        Config::default(),
        LearnerState::new(),
    )
}

fn master(l: &mut LearnerState, p: &BktParams, c: &Config, skill: &str, day0: u32) {
    l.begin_day(day0);
    for _ in 0..6 {
        l.record_task(p, c, skill, true, 0, None);
    }
    l.begin_day(day0 + 1);
    for _ in 0..6 {
        l.record_task(p, c, skill, true, 0, None);
    }
}

fn sheet(l: &LearnerState, album: &Album, name: &str) -> tutor_core::sheet::TeacherSheet {
    build_sheet(
        &grade5_m0_graph(),
        l,
        album,
        &MathSheetContent,
        name,
        &BktParams::numeric_default(),
    )
}

#[test]
fn an_empty_learner_gives_an_empty_sheet() {
    let (_, _, l) = env();
    let s = sheet(&l, &Album::new(), "Мирослава");
    assert_eq!(s.nickname, "Мирослава");
    assert!(
        s.mastered.is_empty()
            && s.in_progress.is_empty()
            && s.gaps.is_empty()
            && s.ask_adult.is_empty()
    );
    assert_eq!((s.stickers, s.sticker_capacity), (0, album_capacity()));
}

#[test]
fn mastered_and_in_progress_skills_are_listed_with_titles_in_course_order() {
    let (p, c, mut l) = env();
    master(&mut l, &p, &c, "N-ADD-MULTI", 0);
    l.begin_day(3);
    l.record_task(&p, &c, "F-ADD-SAME", true, 0, None);
    let s = sheet(&l, &Album::new(), "Дід");
    assert_eq!(s.mastered.len(), 1);
    assert_eq!(s.mastered[0].skill, "N-ADD-MULTI");
    assert_eq!(s.mastered[0].title, "Додавання багатоцифрових чисел");
    assert_eq!(
        s.in_progress
            .iter()
            .map(|x| x.skill.as_str())
            .collect::<Vec<_>>(),
        vec!["F-ADD-SAME"]
    );
}

#[test]
fn a_repeated_bug_becomes_a_neutral_hypothesis_with_an_idea() {
    let (p, c, mut l) = env();
    l.begin_day(0);
    l.record_task(&p, &c, "F-ADD-SAME", false, 0, Some("add_denominators"));
    assert!(
        sheet(&l, &Album::new(), "Н").gaps.is_empty(),
        "одиничний збіг не прогалина"
    );
    l.record_task(&p, &c, "F-ADD-SAME", false, 1, Some("add_denominators"));
    let s = sheet(&l, &Album::new(), "Н");
    assert_eq!(s.gaps.len(), 1);
    let g = &s.gaps[0];
    assert_eq!(g.skill, "F-ADD-SAME");
    assert_eq!(
        g.hypotheses,
        vec!["Може додавати не лише чисельники, а й знаменники".to_string()]
    );
    assert_eq!(g.ideas.len(), 1);
    // Нейтральність: жодних оцінних слів про дитину.
    let all = format!("{g:?}").to_lowercase();
    for bad in [
        "не вміє",
        "слабк",
        "погано",
        "лінив",
        "тупи",
        "дислекс",
        "дискалькул",
    ] {
        assert!(
            !all.contains(bad),
            "оцінне формулювання «{bad}» в аркуші: {all}"
        );
    }
}

#[test]
fn a_low_estimate_after_several_attempts_is_a_gap_even_without_a_known_bug() {
    let (p, c, mut l) = env();
    l.begin_day(0);
    for _ in 0..4 {
        l.record_task(&p, &c, "D-ADD", false, 0, None);
    }
    let s = sheet(&l, &Album::new(), "Н");
    assert_eq!(s.gaps.len(), 1);
    assert!(s.gaps[0].hypotheses.is_empty());
    assert_eq!(s.gaps[0].ideas.len(), 1, "загальна ідея для навички");
}

#[test]
fn gaps_and_ideas_are_capped() {
    let (p, c, mut l) = env();
    l.begin_day(0);
    for (skill, bug) in [
        ("N-ADD-MULTI", "forget_carry"),
        ("D-ADD", "tenths_as_hundredths"),
        ("F-ADD-SAME", "add_denominators"),
        ("F-ADD-DIFF", "add_all_parts"),
    ] {
        for _ in 0..3 {
            l.record_task(&p, &c, skill, false, 0, Some(bug));
        }
    }
    let s = sheet(&l, &Album::new(), "Н");
    assert!(s.gaps.len() <= MAX_GAPS);
    let ideas: usize = s.gaps.iter().map(|g| g.ideas.len()).sum();
    assert!(ideas <= MAX_IDEAS, "ідей {ideas}");
    assert_eq!(ideas, MAX_IDEAS);
}

#[test]
fn the_ask_adult_list_shows_the_task_statement_and_never_a_solution() {
    let (_, _, mut l) = env();
    l.record_ask_adult("F-ADD-DIFF", "Обчисли: 1/3 + 1/6");
    let s = sheet(&l, &Album::new(), "Н");
    assert_eq!(s.ask_adult.len(), 1);
    assert_eq!(s.ask_adult[0].statement, "Обчисли: 1/3 + 1/6");
    assert_eq!(
        s.ask_adult[0].title,
        "Додавання дробів з різними знаменниками"
    );
    assert!(
        !format!("{:?}", s.ask_adult).contains("1/2"),
        "відповіді (1/2) серед задач для дорослого немає"
    );
}

#[test]
fn stickers_are_counted_against_a_finite_album() {
    let mut a = Album::new();
    a.award_mastery("N-ADD-MULTI");
    a.award_for_task(
        "D-ADD",
        TaskFacts {
            solved: true,
            wrong_attempts: 2,
            hints_used: 1,
            asked_adult: false,
        },
    );
    let (_, _, l) = env();
    let s = sheet(&l, &a, "Н");
    assert_eq!(
        (s.stickers, s.sticker_capacity),
        (a.count(), album_capacity())
    );
}

#[test]
fn nicknames_are_cleaned_and_limited() {
    assert_eq!(sanitize_nickname("  Оля   Іванівна \n"), "Оля Іванівна");
    assert_eq!(sanitize_nickname("a\u{0}b\u{7}c"), "abc");
    let long = "Я".repeat(100);
    assert_eq!(sanitize_nickname(&long).chars().count(), NICKNAME_MAX_CHARS);
    assert_eq!(sanitize_nickname("   "), "");
}

#[test]
fn content_has_neutral_texts_for_every_math_bug() {
    let c = MathSheetContent;
    for (s, b) in [
        ("N-ADD-MULTI", "forget_carry"),
        ("D-ADD", "tenths_as_hundredths"),
        ("F-ADD-SAME", "add_denominators"),
        ("F-ADD-DIFF", "add_all_parts"),
    ] {
        assert!(
            c.describe_bug(s, b).is_some()
                && c.idea_for_bug(s, b).is_some()
                && c.idea_for_skill(s).is_some()
        );
    }
    assert!(c.describe_bug("X", "y").is_none());
}

#[test]
fn building_the_sheet_is_deterministic() {
    let (p, c, mut l) = env();
    master(&mut l, &p, &c, "N-ADD-MULTI", 0);
    l.begin_day(3);
    l.record_task(&p, &c, "F-ADD-SAME", false, 0, Some("add_denominators"));
    l.record_task(&p, &c, "F-ADD-SAME", false, 0, Some("add_denominators"));
    let a = sheet(&l, &Album::new(), "Н");
    let b = sheet(&l, &Album::new(), "Н");
    assert_eq!(a, b);
}

#[test]
fn a_mastered_skill_is_never_listed_as_a_gap_even_with_old_bug_evidence() {
    let (p, c, mut l) = env();
    l.begin_day(0);
    l.record_task(&p, &c, "N-ADD-MULTI", false, 0, Some("forget_carry"));
    l.record_task(&p, &c, "N-ADD-MULTI", false, 0, Some("forget_carry"));
    master(&mut l, &p, &c, "N-ADD-MULTI", 1);
    let s = sheet(&l, &Album::new(), "Н");
    assert_eq!(s.mastered.len(), 1);
    assert!(s.gaps.is_empty(), "суперечливий аркуш: {:?}", s.gaps);
}

#[test]
fn the_ask_adult_list_on_the_sheet_is_capped_and_keeps_the_most_recent() {
    let (_, _, mut l) = env();
    for n in 0..25 {
        l.record_ask_adult("D-ADD", &format!("Обчисли: {n},1 + 1,1"));
    }
    let s = sheet(&l, &Album::new(), "Н");
    assert_eq!(s.ask_adult.len(), 10);
    assert_eq!(
        s.ask_adult.last().unwrap().statement,
        "Обчисли: 24,1 + 1,1",
        "останні додані лишаються"
    );
    assert_eq!(
        s.ask_adult.first().unwrap().statement,
        "Обчисли: 15,1 + 1,1"
    );
}

#[test]
fn nicknames_keep_only_letters_digits_spaces_and_simple_punctuation() {
    assert_eq!(sanitize_nickname("Оля\u{202E}ляО"), "ОляляО");
    assert_eq!(sanitize_nickname("a\u{200B}b\u{FEFF}c\u{2066}d"), "abcd");
    assert_eq!(sanitize_nickname("Ґрета-Марія"), "Ґрета-Марія");
    assert_eq!(sanitize_nickname("О'Ніл ✨ 7"), "О'Ніл 7");
    assert_eq!(sanitize_nickname("<script>x</script>"), "scriptxscript");
}

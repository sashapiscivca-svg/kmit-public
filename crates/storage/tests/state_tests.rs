use storage::state::{AppState, Profile, Settings, MAX_AVATAR};
use tutor_core::album::{StickerKind, TaskFacts};
use tutor_core::bkt::BktParams;
use tutor_core::config::Config;

fn rich_state() -> AppState {
    let (p, c) = (BktParams::numeric_default(), Config::default());
    let mut st = AppState::new("  Оля   Мала\n", 3);
    st.learner.begin_day(4);
    for _ in 0..4 {
        st.learner.record_task(&p, &c, "F-ADD-SAME", true, 0, None);
    }
    st.learner
        .record_ask_adult("F-ADD-DIFF", "Обчисли: 1/3 + 1/6");
    st.album.award_mastery("N-ADD-MULTI");
    st.album.award_for_task(
        "D-ADD",
        TaskFacts {
            solved: true,
            wrong_attempts: 2,
            hints_used: 1,
            asked_adult: false,
        },
    );
    st.usage.begin_day(4);
    st.usage.start(1_000, false, false, &c).unwrap();
    st.usage.finish(2_200);
    st.ceremonies.insert(5);
    st.ceremonies.insert(6);
    st.settings = Settings {
        font_scale_percent: 130,
        reduce_motion: true,
        sound: true,
        quiet_hours: Some((21 * 60, 7 * 60)),
        adult_extra_sessions: true,
        enforce_limits: true,
        lesson_first: false,
        ask_confidence: true,
        reading_mode: true,
        onboarded: true,
        allow_screenshots: true,
        weekly_goal_days: 4,
        grade: 6,
        adult_notice_ack: true,
    };
    st
}

#[test]
fn a_new_state_has_a_clean_nickname_and_default_settings() {
    let st = AppState::new("  Оля   Мала\n", 3);
    assert_eq!(
        st.profile,
        Profile {
            nickname: "Оля Мала".to_string(),
            avatar: 3
        }
    );
    assert_eq!(st.settings, Settings::default());
    assert_eq!(st.settings.font_scale_percent, 100);
    assert!(
        !st.settings.sound && !st.settings.reduce_motion,
        "звук і анімації: за замовчуванням вимкнено/спокійно"
    );
    assert!(st.settings.quiet_hours.is_none());
}

#[test]
fn avatars_are_bounded() {
    assert_eq!(AppState::new("Х", 200).profile.avatar, MAX_AVATAR);
}

#[test]
fn state_round_trips_through_text() {
    let st = rich_state();
    let rows = st.to_rows();
    let back = AppState::from_rows(&rows).unwrap();
    assert_eq!(back.profile, st.profile);
    assert_eq!(back.settings, st.settings);
    assert_eq!(back.learner.encode(), st.learner.encode());
    assert_eq!(back.album.keys(), st.album.keys());
    assert_eq!(back.usage.parts(), st.usage.parts());
    assert_eq!(
        back.ceremonies, st.ceremonies,
        "свята переходу зберігаються"
    );
    assert!(back.album.has(StickerKind::Mastery, "N-ADD-MULTI"));
    assert_eq!(rows, back.to_rows(), "кодування стабільне");
}

#[test]
fn broken_rows_are_errors_not_panics() {
    let mut rows = rich_state().to_rows();
    assert!(AppState::from_rows(&[]).is_err(), "порожній набір");
    for i in 0..rows.len() {
        let saved = rows[i].1.clone();
        rows[i].1 = "%%%\u{0}garbage".to_string();
        let _ = AppState::from_rows(&rows); // не панікує
        rows[i].1 = saved;
    }
    let mut missing = rich_state().to_rows();
    missing.retain(|(k, _)| k != "learner");
    assert!(AppState::from_rows(&missing).is_err(), "бракує стану учня");
}

#[test]
fn settings_are_clamped_into_safe_ranges() {
    let mut rows = rich_state().to_rows();
    for (k, v) in &mut rows {
        if k == "settings" {
            *v = "font_scale_percent=9999\nreduce_motion=1\nsound=0\nquiet=1500-5\nadult_extra=1\n"
                .to_string();
        }
    }
    let st = AppState::from_rows(&rows).unwrap();
    assert!(st.settings.font_scale_percent <= 200 && st.settings.font_scale_percent >= 80);
    assert_eq!(
        st.settings.quiet_hours, None,
        "хвилини доби понад 1439 відкидаються"
    );
}

#[test]
fn quiet_hours_may_wrap_around_midnight() {
    let s = Settings {
        quiet_hours: Some((21 * 60, 7 * 60)),
        ..Settings::default()
    };
    assert!(s.is_quiet(22 * 60) && s.is_quiet(0) && s.is_quiet(6 * 60 + 59));
    assert!(!s.is_quiet(7 * 60) && !s.is_quiet(12 * 60) && !s.is_quiet(20 * 60 + 59));
    let day = Settings {
        quiet_hours: Some((13 * 60, 15 * 60)),
        ..Settings::default()
    };
    assert!(day.is_quiet(14 * 60) && !day.is_quiet(15 * 60) && !day.is_quiet(12 * 60));
    assert!(
        !Settings::default().is_quiet(3 * 60),
        "без налаштування тихих годин немає"
    );
}

#[test]
fn an_unfinished_session_is_saved_and_an_absent_one_means_none() {
    let c = Config::default();
    let mut st = rich_state();
    assert!(st.session.is_none());
    let rows = st.to_rows();
    assert!(AppState::from_rows(&rows).unwrap().session.is_none());

    let mut d = tutor_core::sessionctl::DayUsage::new();
    d.begin_day(4);
    let mut s = d.start(5_000, false, false, &c).unwrap();
    s.activity(5_030, &c);
    s.record_task(20, true, 0);
    let text = s.encode();
    st.session = Some(s);
    let back = AppState::from_rows(&st.to_rows()).unwrap();
    assert_eq!(back.session.unwrap().encode(), text);

    // Старі збережені стани без рядка сесії відкриваються як «сесії немає».
    let mut old = st.to_rows();
    old.retain(|(k, _)| k != "session");
    assert!(AppState::from_rows(&old).unwrap().session.is_none());
    // Зіпсований рядок сесії це помилка, а не паніка.
    let mut bad = st.to_rows();
    for (k, v) in &mut bad {
        if k == "session" {
            *v = "зіпсовано".to_string();
        }
    }
    assert!(AppState::from_rows(&bad).is_err());
}

#[test]
fn a_broken_cosmetic_field_does_not_make_the_progress_unloadable() {
    let mut rows = rich_state().to_rows();
    for (k, v) in &mut rows {
        if k == "profile" {
            *v = "avatar=999\nnickname=Оля\n".to_string();
        }
    }
    let st = AppState::from_rows(&rows).expect("хибний аватар не ламає завантаження");
    assert_eq!(st.profile.nickname, "Оля");
    assert!(st.profile.avatar <= MAX_AVATAR);
    // Ім'я ж обов'язкове: без нього рядок профілю зіпсовано.
    for (k, v) in &mut rows {
        if k == "profile" {
            *v = "avatar=3\n".to_string();
        }
    }
    assert!(AppState::from_rows(&rows).is_err());
}

#[test]
fn notes_round_trip_with_newlines_and_percent_signs_and_old_profiles_have_none() {
    let mut st = rich_state();
    st.notes.insert(
        "F-ADD-SAME".to_string(),
        "Правило: додаю чисельники.\nЗнаменник 100% не змінюю = так".to_string(),
    );
    st.notes
        .insert("D-ADD".to_string(), "кома під комою".to_string());
    let rows = st.to_rows();
    let back = AppState::from_rows(&rows).unwrap();
    assert_eq!(back.notes, st.notes);
    // Профіль, збережений до появи конспектів: рядка немає, конспект порожній.
    let old: Vec<_> = rows.into_iter().filter(|(k, _)| k != "notes").collect();
    assert!(AppState::from_rows(&old).unwrap().notes.is_empty());
}

#[test]
fn a_garbled_notes_row_never_makes_the_progress_unloadable() {
    let st = rich_state();
    let mut rows: Vec<_> = st
        .to_rows()
        .into_iter()
        .filter(|(k, _)| k != "notes")
        .collect();
    rows.push((
        "notes".to_string(),
        "без-дорівнює\n=порожній ключ\nok=текст".to_string(),
    ));
    let back = AppState::from_rows(&rows).unwrap();
    assert_eq!(back.notes.get("ok").map(String::as_str), Some("текст"));
    assert_eq!(back.notes.len(), 1);
}

#[test]
fn lesson_progress_and_badges_round_trip_and_garbage_is_ignored() {
    let mut st = rich_state();
    st.lessons.insert("D-ADD".to_string(), 0b101);
    st.badges.insert("lesson:D-ADD".to_string());
    st.badges.insert("course:math5".to_string());
    let rows = st.to_rows();
    let back = AppState::from_rows(&rows).unwrap();
    assert_eq!(back.lessons, st.lessons);
    assert_eq!(back.badges, st.badges);
    let mut rows: Vec<_> = rows
        .into_iter()
        .filter(|(k, _)| k != "lessons" && k != "badges")
        .collect();
    rows.push(("lessons".to_string(), "D-ADD=x\n=5\nok=3".to_string()));
    rows.push(("badges".to_string(), "\n\nfine".to_string()));
    let back = AppState::from_rows(&rows).unwrap();
    assert_eq!(back.lessons.len(), 1);
    assert_eq!(back.lessons["ok"], 3);
    assert_eq!(back.badges.iter().collect::<Vec<_>>(), ["fine"]);
}

#[test]
fn item_repetition_and_item_statistics_round_trip_and_garbage_is_ignored() {
    use storage::state::ItemSrs;
    let mut st = rich_state();
    st.item_srs.push(ItemSrs {
        skill: "UA-APOSTROPHE".into(),
        item: 4,
        due_day: 12,
        stage: 1,
        kind: 2,
        opened: 9,
        misses: 3,
    });
    st.item_srs.push(ItemSrs {
        skill: "EN-TO-BE".into(),
        item: 0,
        due_day: 3,
        stage: 0,
        kind: 0,
        opened: 3,
        misses: 0,
    });
    st.closed_gaps.push(("D-ADD".into(), 2, 20));
    st.gaps_closed_total = 7;
    st.item_stats.insert(("UA-APOSTROPHE".into(), 4), (5, 2));
    let rows = st.to_rows();
    let back = AppState::from_rows(&rows).unwrap();
    assert_eq!(back.item_srs, st.item_srs);
    assert_eq!(back.item_stats, st.item_stats);
    assert_eq!(back.closed_gaps, st.closed_gaps);
    assert_eq!(back.gaps_closed_total, 7);
    let mut rows: Vec<_> = rows
        .into_iter()
        .filter(|(k, _)| k != "item_srs" && k != "item_stats")
        .collect();
    rows.push(("item_srs".into(), "A|x|1|1\nB|2|3\nOK|7|9|2\n|1|1|1".into()));
    rows.push((
        "item_stats".into(),
        "A|1|2\nB|1|2|9\nC|1|2|x\nD|3|4|2".into(),
    ));
    let back = AppState::from_rows(&rows).unwrap();
    assert_eq!(back.item_srs.len(), 1);
    assert_eq!(back.item_srs[0].skill, "OK");
    assert_eq!(
        (back.item_srs[0].kind, back.item_srs[0].misses),
        (0, 0),
        "старий запис без нових полів читається зі значеннями за замовчуванням"
    );
    assert_eq!(
        back.item_stats.len(),
        1,
        "відкидаємо хибний рядок і неможливі лічильники (правильних більше за спроби)"
    );
}

#[test]
fn item_flags_round_trip_and_invalid_reasons_are_dropped() {
    use storage::state::ItemFlag;
    let mut st = rich_state();
    st.flags.push(ItemFlag {
        skill: "UA-SOUNDS".into(),
        key: "4".into(),
        reason: "unclear".into(),
        statement: "Скільки звуків у слові «яма»?\nдвічі".into(),
    });
    let rows = st.to_rows();
    let back = AppState::from_rows(&rows).unwrap();
    assert_eq!(back.flags, st.flags);
    let mut rows: Vec<_> = rows.into_iter().filter(|(k, _)| k != "flags").collect();
    rows.push((
        "flags".into(),
        "A|1|bogus|x\nB|2|other|ok|extra|pipes".into(),
    ));
    let back = AppState::from_rows(&rows).unwrap();
    assert_eq!(back.flags.len(), 1);
    assert_eq!(back.flags[0].statement, "ok|extra|pipes");
}

#[test]
fn the_hero_wardrobe_and_weeks_round_trip_and_garbage_is_dropped() {
    use std::collections::BTreeSet;
    use storage::state::WeekRec;
    use tutor_core::hero::Slot;
    let mut st = rich_state();
    st.hero.skin = 4;
    st.hero.hair_color = 6;
    st.wardrobe = BTreeSet::from(["hat_crown".to_string(), "pet_cat".to_string()]);
    st.hero.equipped.insert(Slot::Hat, "hat_crown".into());
    st.hero.equipped.insert(Slot::Pet, "pet_cat".into());
    st.weeks.insert(
        2900,
        WeekRec {
            tasks: [5, 0, 6, 0, 0, 7, 0],
            lessons: [1, 0, 0, 0, 0, 0, 0],
            reviews: [0, 0, 1, 0, 0, 0, 0],
            gaps: 2,
            claimed: Some("hat_crown".into()),
        },
    );
    let back = AppState::from_rows(&st.to_rows()).unwrap();
    assert_eq!(back.hero, st.hero);
    assert_eq!(back.wardrobe, st.wardrobe);
    assert_eq!(back.weeks, st.weeks);
    // Зіпсований запис не губить профіль: невідомі й чужі предмети відкидаються, шари за замовчуванням повертаються.
    let mut rows: Vec<_> = st
        .to_rows()
        .into_iter()
        .filter(|(k, _)| k != "hero" && k != "weeks")
        .collect();
    rows.push(("hero".into(), "skin=99\nhair=x\neq=hat:hat_wizard,pet:hat_crown,nonsense\nwardrobe=hat_crown,nope,hair_short\n".into()));
    rows.push((
        "weeks".into(),
        "1|1,2|0|0|0|\nbad\n5|1,1,1,1,1,1,1|0,0,0,0,0,0,0|0,0,0,0,0,0,0|1|\n".into(),
    ));
    let back = AppState::from_rows(&rows).unwrap();
    assert_eq!(back.hero.skin, 1);
    assert!(
        !back.hero.equipped.contains_key(&Slot::Hat),
        "чужий убір не одягається"
    );
    assert!(back.hero.equipped.contains_key(&Slot::Outfit));
    assert_eq!(back.wardrobe, BTreeSet::from(["hat_crown".to_string()]));
    assert_eq!(back.weeks.len(), 1);
    // Старий профіль без героя читається зі значеннями за замовчуванням.
    let rows: Vec<_> = st
        .to_rows()
        .into_iter()
        .filter(|(k, _)| k != "hero" && k != "weeks")
        .collect();
    let back = AppState::from_rows(&rows).unwrap();
    assert!(back.wardrobe.is_empty() && back.weeks.is_empty());
}

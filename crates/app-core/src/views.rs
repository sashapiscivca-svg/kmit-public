//! Побудова JSON для інтерфейсу. Лише безпечні поля: ніяких відповідей, проміжних результатів чи типових помилок.

use crate::app::Env;
use serde_json::{json, Value};
use storage::state::Settings;
use tutor_core::album::{Album, StickerId, StickerKind};
use tutor_core::sessionctl::StartDenied;
use tutor_core::sheet::TeacherSheet;

/// Малюнок для інтерфейсу. Нездорові параметри відкидаються (малюнок просто не показується).
pub(crate) fn figure_json(f: &tutor_core::figure::Figure) -> Option<Value> {
    use tutor_core::figure::Figure;
    if !f.is_sane() {
        return None;
    }
    Some(match f {
        Figure::FractionBar { num, den } => json!({"kind": "fraction_bar", "num": num, "den": den}),
        Figure::DecimalGrid { hundredths } => {
            json!({"kind": "decimal_grid", "hundredths": hundredths})
        }
        Figure::Rect { w, h } => json!({"kind": "rect", "w": w, "h": h}),
        Figure::Diagram { kind, title, rows } => {
            json!({"kind": kind.code(), "title": title, "rows": rows})
        }
        Figure::Angle { deg, label } => json!({"kind": "angle", "deg": deg, "label": label}),
        Figure::NumberLine {
            min,
            max,
            parts,
            marks,
        } => json!({
            "kind": "number_line", "min": min, "max": max, "parts": parts,
            "marks": marks.iter().map(|(p, t)| json!({"pos": p, "label": t})).collect::<Vec<_>>(),
        }),
    })
}

pub(crate) fn figures_json(fs: &[tutor_core::figure::Figure]) -> Vec<Value> {
    fs.iter().filter_map(figure_json).collect()
}

pub(crate) fn skill_title(env: &Env, id: &str) -> String {
    env.graph
        .get(id)
        .map_or_else(|| id.to_string(), |n| n.title.to_string())
}

fn kind_code(k: StickerKind) -> &'static str {
    match k {
        StickerKind::Mastery => "mastery",
        StickerKind::Persisted => "persisted",
        StickerKind::SelfCorrected => "self_corrected",
        StickerKind::AskedAndSolved => "asked_and_solved",
    }
}

pub(crate) fn sticker_json(env: &Env, id: &StickerId) -> Value {
    json!({
        "key": id.key(),
        "kind": kind_code(id.kind),
        "skill": id.skill,
        "skill_title": skill_title(env, &id.skill),
        "subject": crate::catalog::subject_of(&id.skill),
    })
}

/// Увесь скінченний альбом: кожна клітинка з ознакою «отримано».
pub(crate) fn album_json(env: &Env, album: &Album, grade: u8) -> Value {
    let mut cells = Vec::new();
    for skill in env.graph.topological_order().unwrap_or_default() {
        if crate::catalog::grade_of(skill) != grade {
            continue;
        }
        for kind in StickerKind::ALL {
            let id = StickerId {
                kind,
                skill: skill.to_string(),
            };
            let mut v = sticker_json(env, &id);
            v["earned"] = json!(album.has(kind, skill));
            v["subject"] = json!(crate::catalog::subject_of(skill));
            cells.push(v);
        }
    }
    let subjects: Vec<Value> = crate::catalog::subject_list(grade)
        .into_iter()
        .map(|(id, title)| json!({"id": id, "title": title}))
        .collect();
    let in_grade = env
        .graph
        .ids()
        .into_iter()
        .filter(|id| crate::catalog::grade_of(id) == grade)
        .count();
    json!({ "count": album.count(), "capacity": Album::capacity(in_grade), "cells": cells, "subjects": subjects })
}

pub(crate) fn settings_json(s: &Settings) -> Value {
    json!({
        "font_scale_percent": s.font_scale_percent,
        "reduce_motion": s.reduce_motion,
        "sound": s.sound,
        "quiet_hours": s.quiet_hours.map(|(from, to)| json!({"from": from, "to": to})),
        "adult_extra_sessions": s.adult_extra_sessions,
        "enforce_limits": s.enforce_limits,
        "lesson_first": s.lesson_first,
        "ask_confidence": s.ask_confidence,
        "reading_mode": s.reading_mode,
        "onboarded": s.onboarded,
        "allow_screenshots": s.allow_screenshots,
        "adult_notice_ack": s.adult_notice_ack,
        "weekly_goal_days": s.weekly_goal_days,
        "grade": s.grade,
    })
}

pub(crate) fn denied_json(d: StartDenied) -> Value {
    match d {
        StartDenied::MaxSessionsToday => json!({"ok": false, "reason": "limit"}),
        StartDenied::QuietHours => json!({"ok": false, "reason": "quiet"}),
        StartDenied::BreakNotOver { secs } => json!({"ok": false, "reason": "break", "secs": secs}),
    }
}

pub(crate) fn sheet_json(s: &TeacherSheet) -> Value {
    let line = |l: &tutor_core::sheet::SkillLine| json!({"skill": l.skill, "title": l.title});
    json!({
        "subject": s.subject,
        "to_repeat": s.to_repeat.iter().map(line).collect::<Vec<_>>(),
        "nickname": s.nickname,
        "mastered": s.mastered.iter().map(line).collect::<Vec<_>>(),
        "in_progress": s.in_progress.iter().map(line).collect::<Vec<_>>(),
        "gaps": s.gaps.iter().map(|g| json!({
            "skill": g.skill, "title": g.title, "hypotheses": g.hypotheses, "ideas": g.ideas,
        })).collect::<Vec<_>>(),
        "ask_adult": s.ask_adult.iter().map(|a| json!({
            "skill": a.skill, "title": a.title, "statement": a.statement,
        })).collect::<Vec<_>>(),
        "stickers": s.stickers,
        "sticker_capacity": s.sticker_capacity,
    })
}

//! Повторення й адаптація до дитини: що пора повторити (за предметами), як міцно тримаються знання, і план на сьогодні,
//! що підлаштовується під її результати. Усе виводиться зі стану учня, нових даних про дитину не збирається.

use crate::app::{day_suggestion, skill_state, Env, Open};
use crate::catalog;
use crate::ctx::Ctx;
use crate::views::skill_title;
use serde_json::{json, Value};
use storage::state::AppState;
use tutor_core::lesson::LessonSource;

fn today_of(open: &mut Open, ctx: &Ctx) -> u32 {
    open.state.learner.begin_day(ctx.day);
    open.state.learner.today()
}

/// Пора повторити в межах предмета: теми за розкладом, теми з пропущеними завданнями, картки пригадування.
fn due_for_subject(
    env: &Env,
    st: &AppState,
    subject: &str,
    today: u32,
) -> (Vec<Value>, Vec<String>, Vec<String>) {
    let sg = catalog::sub_graph(&env.graph, subject, st.settings.grade);
    let scheduled: Vec<Value> = st
        .learner
        .all_due_reviews(&env.cfg)
        .into_iter()
        .filter(|d| sg.get(&d.skill).is_some())
        .map(|d| {
            json!({"skill": d.skill, "title": skill_title(env, &d.skill), "overdue_days": d.overdue_days,
                   "memory": st.learner.memory_pct(&d.skill, &env.cfg), "microtest": d.microtest})
        })
        .collect();
    let mut items: Vec<String> = Vec::new();
    for i in &st.item_srs {
        if i.due_day <= today
            && sg.get(&i.skill).is_some()
            && !items.contains(&i.skill)
            && !scheduled.iter().any(|v| v["skill"] == i.skill.as_str())
        {
            items.push(i.skill.clone());
        }
    }
    let recall: Vec<String> = crate::recall::due_skills(env, st, today)
        .into_iter()
        .filter(|s| sg.get(s).is_some())
        .collect();
    (scheduled, items, recall)
}

/// Огляд повторення: що пора повторити по предметах і скільки повторень чекає в найближчі дні.
pub(crate) fn overview(env: &Env, open: &mut Open, ctx: &Ctx) -> Value {
    let today = today_of(open, ctx);
    crate::recall::seed(env, &mut open.state, ctx.day);
    let mut subjects = Vec::new();
    let mut total = 0usize;
    for (sid, title) in catalog::subject_list(open.state.settings.grade) {
        let (scheduled, items, recall) = due_for_subject(env, &open.state, sid, today);
        let n = scheduled.len() + items.len();
        total += n + recall.len();
        subjects.push(json!({
            "id": sid, "title": title, "scheduled": scheduled,
            "item_skills": items.iter().map(|s| json!({"skill": s, "title": skill_title(env, s)})).collect::<Vec<_>>(),
            "recall": recall.len(), "tasks_due": n,
        }));
    }
    // Скільки тем потребуватимуть повторення в найближчі 7 днів.
    let mut week = vec![0u32; 7];
    for (_, _, due) in open.state.learner.schedule_list() {
        if due > today && due <= today + 7 {
            week[(due - today - 1) as usize] += 1;
        }
    }
    let tracked = open.state.learner.schedule_list().len();
    json!({"total_due": total, "subjects": subjects, "week": week, "tracked": tracked})
}

/// Профіль знань за предметом: що тримається міцно, що слабшає, що потребує уваги.
pub(crate) fn profile(env: &Env, open: &mut Open, subject: &str, ctx: &Ctx) -> Value {
    let today = today_of(open, ctx);
    let sg = catalog::sub_graph(&env.graph, subject, open.state.settings.grade);
    let st = &open.state;
    let mut topics = Vec::new();
    for id in sg.topological_order().unwrap_or_default() {
        let state = skill_state(env, &st.learner, id);
        let sk = st.learner.skill(id);
        let p = sk.map_or(0, |s| (s.p_known * 100.0).round() as u32);
        let attempts = sk.map_or(0, |s| s.attempts);
        let memory = st.learner.memory_pct(id, &env.cfg);
        let due = st.learner.schedule(id).is_some_and(|s| s.is_due(today));
        let missed = st.item_srs.iter().filter(|i| i.skill == id).count();
        // Один зрозумілий ярлик: чим зараз потрібно зайнятися в цій темі.
        let label = match state {
            "locked" => "locked",
            "available" => "new",
            "mastered" if due => "repeat",
            "mastered" if memory.unwrap_or(100) < 67 => "fading",
            "mastered" => "strong",
            _ if attempts >= 3 && p < 50 => "weak",
            _ => "learning",
        };
        topics.push(json!({
            "skill": id, "title": skill_title(env, id), "state": state, "label": label,
            "known": p, "attempts": attempts, "memory": memory, "missed_items": missed,
            "review_level": st.learner.schedule(id).map(|s| s.level()),
        }));
    }
    let count = |l: &str| topics.iter().filter(|t| t["label"] == l).count();
    json!({
        "subject": subject, "topics": topics,
        "summary": {"strong": count("strong"), "fading": count("fading"), "repeat": count("repeat"),
                    "weak": count("weak"), "learning": count("learning"), "new": count("new")},
    })
}

/// Режим плану за результатами останніх днів: `consolidate` (спершу закріпити), `advance` (можна швидше), `steady`.
pub(crate) fn mode_of(tasks: u32, correct: u32) -> &'static str {
    if tasks >= 8 && correct * 100 < tasks * 60 {
        "consolidate"
    } else if tasks >= 12 && correct * 100 >= tasks * 85 {
        "advance"
    } else {
        "steady"
    }
}

fn gap_kind(k: u8) -> &'static str {
    match k {
        2 => "asked_adult",
        1 => "struggled",
        _ => "missed",
    }
}

/// Граф пам'яті предмета (знеособлений): теми з міцністю пам'яті, складні приклади (прогалини), подолані прогалини й
/// типові непорозуміння. Тексти прикладів показуються лише поза активною задачею (перевіряє виклик).
pub(crate) fn memory_graph(env: &Env, open: &mut Open, subject: &str, ctx: &Ctx) -> Value {
    let today = today_of(open, ctx);
    let sg = catalog::sub_graph(&env.graph, subject, open.state.settings.grade);
    let st = &open.state;
    let ids: Vec<&'static str> = sg.topological_order().unwrap_or_default();
    let mut gaps = Vec::new();
    for g in st.item_srs.iter().filter(|g| sg.get(&g.skill).is_some()) {
        let statement = catalog::generator_for(&g.skill)
            .map(|gen| gen.generate(u64::from(g.item)).statement)
            .unwrap_or_default();
        gaps.push(json!({
            "skill": g.skill, "title": skill_title(env, &g.skill), "statement": statement,
            "kind": gap_kind(g.kind),
            "stage": g.stage, "stages": storage::state::MAX_GAP_STAGE + 1,
            "due_in": i64::from(g.due_day) - i64::from(today), "misses": g.misses,
            "age_days": today.saturating_sub(g.opened),
        }));
    }
    let closed: Vec<Value> = st
        .closed_gaps
        .iter()
        .filter(|(s, _, _)| sg.get(s).is_some())
        .map(|(s, a, b)| json!({"skill": s, "title": skill_title(env, s), "days": b.saturating_sub(*a)}))
        .collect();
    let mut topics = Vec::new();
    let mut misconceptions = Vec::new();
    for id in &ids {
        let has_gap = st.item_srs.iter().any(|g| g.skill == *id);
        let due = st.learner.schedule(id).is_some_and(|s| s.is_due(today));
        let memory = st.learner.memory_pct(id, &env.cfg);
        let sk = st.learner.skill(id);
        let weak =
            !st.learner.is_mastered(id) && sk.is_some_and(|s| s.attempts >= 3 && s.p_known < 0.5);
        let fading = memory.is_some_and(|m| m < 67);
        let bugs: Vec<String> = st.learner.bug_hypotheses(id, 2);
        for b in &bugs {
            if let Some(m) = catalog::bug_message(id, b) {
                misconceptions.push(json!({"skill": id, "title": skill_title(env, id), "text": m}));
            }
        }
        if has_gap || due || weak || fading || !bugs.is_empty() {
            topics.push(json!({
                "skill": id, "title": skill_title(env, id), "memory": memory,
                "known": sk.map(|s| (s.p_known * 100.0).round() as u32),
                "gaps": st.item_srs.iter().filter(|g| g.skill == *id).count(),
                "state": skill_state(env, &st.learner, id),
            }));
        }
    }
    // Зв'язки «передумова → тема» лише між показаними темами й їхніми найближчими передумовами.
    let shown: Vec<String> = topics
        .iter()
        .filter_map(|t| t["skill"].as_str().map(String::from))
        .collect();
    let mut nodes = topics;
    let mut edges = Vec::new();
    for id in &shown {
        if let Some(n) = sg.get(id) {
            for p in &n.prerequisites {
                if !nodes.iter().any(|t| t["skill"] == *p) {
                    nodes.push(json!({"skill": p, "title": skill_title(env, p),
                        "memory": st.learner.memory_pct(p, &env.cfg), "gaps": 0,
                        "state": skill_state(env, &st.learner, p), "context": true}));
                }
                edges.push(json!({"from": p, "to": id}));
            }
        }
    }
    json!({"subject": subject, "topics": nodes, "edges": edges, "gaps": gaps, "closed": closed,
           "closed_total": st.gaps_closed_total, "misconceptions": misconceptions})
}

/// Точність за останні три дні: (задач, правильних).
fn recent_accuracy(st: &AppState, today: u32) -> (u32, u32) {
    st.activity
        .iter()
        .filter(|(d, _)| **d + 3 > today)
        .fold((0, 0), |(t, c), (_, (a, b))| (t + a, c + b))
}

/// План на сьогодні: повторити, виправити слабке, рухатись далі, пригадати. Підлаштовується під результати:
/// після невдалих днів новий матеріал відкладається, і спершу закріплюється слабке й давнє.
pub(crate) fn plan(env: &Env, open: &mut Open, ctx: &Ctx) -> Value {
    let today = today_of(open, ctx);
    crate::recall::seed(env, &mut open.state, ctx.day);
    let (tasks, correct) = recent_accuracy(&open.state, today);
    let mode = mode_of(tasks, correct);
    let struggling = mode == "consolidate";
    let mut blocks: Vec<Value> = Vec::new();

    // 1. Повторення: предмет, у якому найбільше боргу.
    let mut best_review: Option<(usize, &'static str, String, usize)> = None;
    for (sid, title) in catalog::subject_list(open.state.settings.grade) {
        let (sch, items, _) = due_for_subject(env, &open.state, sid, today);
        let n = sch.len() + items.len();
        if n > 0 && best_review.as_ref().map_or(true, |b| n > b.0) {
            best_review = Some((n, sid, title, n));
        }
    }
    if let Some((n, sid, title, _)) = &best_review {
        let sg = catalog::sub_graph(&env.graph, sid, open.state.settings.grade);
        let hard = open
            .state
            .item_srs
            .iter()
            .filter(|g| g.due_day <= today && sg.get(&g.skill).is_some())
            .count();
        blocks.push(
            json!({"kind": "review", "subject": sid, "subject_title": title, "count": n,
                           "hard": hard, "minutes": (2 * n).clamp(3, 10)}),
        );
    }

    // 2. Слабка тема: перечитати пояснення й потренуватися (або після невдалих днів: першою).
    let mut weak: Option<(u32, String, String)> = None;
    for (sid, _) in catalog::subject_list(open.state.settings.grade) {
        let sg = catalog::sub_graph(&env.graph, sid, open.state.settings.grade);
        for id in sg.ids() {
            let Some(s) = open.state.learner.skill(id) else {
                continue;
            };
            if !open.state.learner.is_mastered(id) && s.attempts >= 3 && s.p_known < 0.5 {
                let p = (s.p_known * 100.0).round() as u32;
                if weak.as_ref().map_or(true, |w| p < w.0) {
                    weak = Some((p, id.to_string(), sid.to_string()));
                }
            }
        }
    }
    if let Some((_, skill, sid)) = weak {
        let b = json!({"kind": "fix", "subject": sid, "skill": skill, "skill_title": skill_title(env, &skill),
                       "has_lesson": env.lessons.lesson(&skill).is_some(), "minutes": 8});
        if struggling {
            blocks.insert(0, b);
        } else {
            blocks.push(b);
        }
    }

    // 3. Рух далі: продовження або нова тема; після невдалих днів не пропонується, поки є що закріплювати.
    let sug = day_suggestion(env, open);
    let hold_new = struggling && !blocks.is_empty();
    if !sug.is_null() && sug["reason"] != "review" && !hold_new {
        blocks.push(json!({"kind": if sug["reason"] == "new" { "new" } else { "continue" },
                           "subject": sug["subject"], "subject_title": sug["subject_title"],
                           "skill": sug["skill"], "skill_title": sug["skill_title"], "minutes": 10}));
    }

    // 4. Пригадування своїми словами.
    let recall = crate::recall::due_skills(env, &open.state, today).len();
    if recall > 0 {
        blocks.push(json!({"kind": "recall", "count": recall, "minutes": 5}));
    }
    let minutes: u64 = blocks
        .iter()
        .map(|b| b["minutes"].as_u64().unwrap_or(0))
        .sum();
    json!({"blocks": blocks, "minutes": minutes, "mode": mode,
           "recent": {"tasks": tasks, "correct": correct}})
}

/// Поради для аркуша вчителя з предметів без бібліотеки типових помилок: загальна ідея на 10 хвилин за предметом.
pub(crate) struct SubjectSheetContent(pub &'static str);

impl tutor_core::sheet::SheetContent for SubjectSheetContent {
    fn describe_bug(&self, _skill: &str, _bug: &str) -> Option<String> {
        None
    }
    fn idea_for_bug(&self, _skill: &str, _bug: &str) -> Option<String> {
        None
    }
    fn idea_for_skill(&self, _skill: &str) -> Option<String> {
        Some(
            match self.0 {
                "ukrainian" => "Разом прочитайте правило з пояснення, придумайте по три слова на кожен випадок і нехай дитина сама пояснить, чому слово пишеться саме так.",
                "english" => "Повторіть слова теми вголос, складіть із ними по два прості речення й розіграйте коротку розмову.",
                "literature" => "Прочитайте уривок разом і попросіть дитину переказати його своїми словами, назвати героя й головну думку.",
                "history" => "Знайдіть подію на стрічці часу, розкажіть її як історію з початком і кінцем і назвіть, що змінилося після неї.",
                "nature" => "Знайдіть приклад теми поруч (у дворі, на кухні, на вікні) і нехай дитина пояснить, що там відбувається і чому.",
                "informatics" => "Покажіть тему на справжньому пристрої чи схемі й попросіть дитину пояснити послідовність кроків.",
                _ => "Перечитайте пояснення теми й розберіть разом два-три приклади; нехай дитина пояснить правило своїми словами.",
            }
            .to_string(),
        )
    }
}

/// Аркуш для вчителя одного предмета (учитель зазвичай вчить один предмет), з темами, що чекають повторення.
pub(crate) fn subject_sheet(
    env: &Env,
    open: &mut Open,
    subject: &str,
    ctx: &Ctx,
) -> Option<tutor_core::sheet::TeacherSheet> {
    today_of(open, ctx);
    let (_, title) = catalog::subject_list(open.state.settings.grade)
        .into_iter()
        .find(|(id, _)| *id == subject)?;
    let sg = catalog::sub_graph(&env.graph, subject, open.state.settings.grade);
    let st = &open.state;
    let mut sheet = if subject == catalog::MATH {
        tutor_engine::sheet::build_sheet(
            &sg,
            &st.learner,
            &st.album,
            &generators_math::MathSheetContent,
            &st.profile.nickname,
            &env.bkt,
        )
    } else {
        let content = SubjectSheetContent(match subject {
            "ukrainian" => "ukrainian",
            "english" => "english",
            "literature" => "literature",
            "history" => "history",
            "nature" => "nature",
            "informatics" => "informatics",
            _ => "other",
        });
        tutor_engine::sheet::build_sheet(
            &sg,
            &st.learner,
            &st.album,
            &content,
            &st.profile.nickname,
            &env.bkt,
        )
    };
    sheet.subject = title;
    // Стікери лише цього предмета (усі види за темами предмета).
    sheet.stickers = sg
        .ids()
        .iter()
        .map(|id| {
            tutor_core::album::StickerKind::ALL
                .iter()
                .filter(|k| st.album.has(**k, id))
                .count()
        })
        .sum();
    sheet.sticker_capacity = tutor_core::album::Album::capacity(sg.ids().len());
    sheet.to_repeat = st
        .learner
        .all_due_reviews(&env.cfg)
        .into_iter()
        .filter(|d| sg.get(&d.skill).is_some())
        .map(|d| tutor_core::sheet::SkillLine {
            title: skill_title(env, &d.skill),
            skill: d.skill,
        })
        .collect();
    Some(sheet)
}

#[cfg(test)]
mod tests {
    use super::mode_of;

    #[test]
    fn the_plan_mode_follows_recent_results_and_needs_enough_tasks() {
        assert_eq!(mode_of(3, 0), "steady", "замало задач для висновків");
        assert_eq!(mode_of(10, 4), "consolidate");
        assert_eq!(mode_of(10, 6), "steady");
        assert_eq!(mode_of(12, 11), "advance");
        assert_eq!(mode_of(12, 9), "steady");
    }
}

//! Збирач аркуша для вчителя зі стану учня. Лише дані, які вже є в рушії; жодних вільних текстів дитини.

use course_pack::SkillGraph;
use tutor_core::album::Album;
use tutor_core::bkt::BktParams;
use tutor_core::learner::LearnerState;
use tutor_core::sheet::{
    sanitize_nickname, AskAdultItem, Gap, SheetContent, SkillLine, TeacherSheet,
};

/// Скільки прогалин показується на аркуші (3-5 за специфікацією).
pub const MAX_GAPS: usize = 5;
/// Скільки ідей на 10 хвилин на аркуші (2-3 за специфікацією).
pub const MAX_IDEAS: usize = 3;
/// Скільки задач із «запитай дорослого» показується на аркуші (останні додані).
pub const MAX_ASK_ADULT: usize = 10;
/// Скільки збігів помилки потрібно, щоб назвати її гіпотезою.
pub const MIN_BUG_HITS: u32 = 2;
/// Навичка вважається прогалиною, якщо оцінка знання нижча за цей поріг і спроб було достатньо.
const GAP_P_BELOW: f64 = 0.5;
const GAP_MIN_ATTEMPTS: u32 = 3;

fn title_of(graph: &SkillGraph, skill: &str) -> String {
    graph
        .get(skill)
        .map_or_else(|| skill.to_string(), |n| n.title.to_string())
}

pub fn build_sheet(
    graph: &SkillGraph,
    learner: &LearnerState,
    album: &Album,
    content: &dyn SheetContent,
    nickname: &str,
    _params: &BktParams,
) -> TeacherSheet {
    let line = |skill: &str| SkillLine {
        skill: skill.to_string(),
        title: title_of(graph, skill),
    };

    let mastered: Vec<SkillLine> = graph
        .topological_order()
        .unwrap_or_default()
        .into_iter()
        .filter(|id| learner.is_mastered(id))
        .map(line)
        .collect();

    let order: Vec<&'static str> = graph.topological_order().unwrap_or_default();
    let in_progress: Vec<SkillLine> = order
        .iter()
        .filter(|id| !learner.is_mastered(id) && learner.skill(id).is_some_and(|s| s.attempts > 0))
        .map(|id| line(id))
        .collect();

    // Прогалини: навички з гіпотезами про помилки або з низькою оцінкою після кількох спроб.
    let mut candidates: Vec<(usize, f64, &'static str, Vec<String>)> = Vec::new();
    for id in &order {
        if learner.is_mastered(id) {
            continue; // опанована навичка не може бути прогалиною на тому ж аркуші
        }
        let hyps = learner.bug_hypotheses(id, MIN_BUG_HITS);
        let weak = learner
            .skill(id)
            .is_some_and(|s| s.attempts >= GAP_MIN_ATTEMPTS && s.p_known < GAP_P_BELOW);
        if hyps.is_empty() && !weak {
            continue;
        }
        let p = learner.skill(id).map_or(1.0, |s| s.p_known);
        candidates.push((hyps.len(), p, id, hyps));
    }
    // Спершу ті, де більше гіпотез, далі ті, де нижча оцінка.
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(b.2)));
    candidates.truncate(MAX_GAPS);

    let mut ideas_left = MAX_IDEAS;
    let mut gaps = Vec::new();
    for (_, _, id, hyps) in candidates {
        let mut descriptions = Vec::new();
        let mut ideas = Vec::new();
        for bug in &hyps {
            if let Some(d) = content.describe_bug(id, bug) {
                descriptions.push(d);
            }
            if ideas_left > 0 {
                if let Some(i) = content.idea_for_bug(id, bug) {
                    ideas.push(i);
                    ideas_left -= 1;
                }
            }
        }
        if ideas.is_empty() && ideas_left > 0 {
            if let Some(i) = content.idea_for_skill(id) {
                ideas.push(i);
                ideas_left -= 1;
            }
        }
        gaps.push(Gap {
            skill: id.to_string(),
            title: title_of(graph, id),
            hypotheses: descriptions,
            ideas,
        });
    }

    let all_items = learner.ask_adult_items();
    let skip = all_items.len().saturating_sub(MAX_ASK_ADULT);
    let ask_adult = all_items
        .into_iter()
        .skip(skip)
        .map(|(skill, statement)| AskAdultItem {
            title: title_of(graph, &skill),
            skill,
            statement,
        })
        .collect();

    TeacherSheet {
        subject: String::new(),
        to_repeat: Vec::new(),
        nickname: sanitize_nickname(nickname),
        mastered,
        in_progress,
        gaps,
        ask_adult,
        stickers: album.count(),
        sticker_capacity: Album::capacity(graph.ids().len()),
    }
}

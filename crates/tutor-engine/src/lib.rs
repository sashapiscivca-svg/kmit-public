//! Вибір наступної навички й задачі: посильність, перемежування, повторення, ліміти, дедуплікація.
//! Чиста логіка без I/O. Детермінована: однаковий стан дає однаковий вибір.

pub mod sheet;

use course_pack::SkillGraph;
use std::collections::BTreeSet;
use tutor_core::bkt::BktParams;
use tutor_core::config::Config;
use tutor_core::generator::{SkillGenerator, WorkedExample};
use tutor_core::learner::{DueReview, LearnerState};
use tutor_core::task::Task;

/// Скільки варіантів перебирається, перш ніж вважати простір умов навички вичерпаним.
const MAX_VARIANT_TRIES: u32 = 2_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    /// Повторення опанованої навички; `microtest` для довгого пропуску.
    Review { microtest: bool },
    /// Навичка вже розпочата й лишається посильною.
    Continue,
    /// Нова навичка, усі передумови якої опановано.
    New,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub skill: String,
    pub reason: Reason,
}

fn review_selection(d: &DueReview) -> Selection {
    Selection {
        skill: d.skill.clone(),
        reason: Reason::Review {
            microtest: d.microtest,
        },
    }
}

/// Обирає навичку для наступної задачі дня. `task_index` — номер задачі за сьогодні (з нуля).
/// `None` означає «на сьогодні досить»: нічого посильного не лишилося.
pub fn select_skill(
    graph: &SkillGraph,
    learner: &LearnerState,
    params: &BktParams,
    cfg: &Config,
    task_index: u32,
) -> Option<Selection> {
    // Повторення лише навичок цього графа (предмета): інші предмети мають власні сесії.
    let due: Vec<_> = learner
        .due_reviews(cfg)
        .into_iter()
        .filter(|d| graph.get(&d.skill).is_some())
        .collect();

    // Перемежування: кожна n-та задача є повторенням, якщо є що повторювати.
    let slot =
        cfg.interleave_every > 0 && task_index % cfg.interleave_every == cfg.interleave_every - 1;
    if slot {
        if let Some(d) = due.first() {
            return Some(review_selection(d));
        }
    }

    // Наступні навички відкриваються, лише коли передумови опановані й надійні (оцінка не нижча за готовність).
    let mastered: BTreeSet<&'static str> = graph
        .ids()
        .into_iter()
        .filter(|id| {
            learner.is_mastered(id)
                && learner
                    .skill(id)
                    .is_some_and(|s| s.p_known >= cfg.readiness_p)
        })
        .collect();
    let order = graph.topological_order().ok()?;

    // Нова навичка вивчається блоком: розпочата, але ще не опанована навичка має пріоритет над новою,
    // поки не вичерпала ліміт дня. Усередині групи вибирається найближча до цілі за прогнозом успіху
    // (зона найближчого розвитку), за рівності йде порядок курсу.
    type Best = Option<(f64, usize, &'static str)>;
    let mut best_started: Best = None;
    let mut best_new: Best = None;
    for id in graph.available(&mastered) {
        let day_cap_hit =
            cfg.enforce_limits && learner.tasks_today(id) >= cfg.max_tasks_per_skill_per_day;
        if learner.is_exhausted(id) || day_cap_hit {
            continue;
        }
        let state = learner.skill(id);
        let started = state.is_some_and(|s| s.attempts > 0);
        let p = state.map_or(params.p_init(), |s| s.p_known);
        let dist = (params.predicted_success(p) - cfg.target_success).abs();
        let idx = order.iter().position(|x| *x == id).unwrap_or(usize::MAX);
        let slot = if started {
            &mut best_started
        } else {
            &mut best_new
        };
        let better = match slot {
            None => true,
            Some((best_dist, best_idx, _)) => {
                dist < *best_dist || (dist == *best_dist && idx < *best_idx)
            }
        };
        if better {
            *slot = Some((dist, idx, id));
        }
    }
    if let Some((_, _, id)) = best_started {
        return Some(Selection {
            skill: id.to_string(),
            reason: Reason::Continue,
        });
    }
    if let Some((_, _, id)) = best_new {
        return Some(Selection {
            skill: id.to_string(),
            reason: Reason::New,
        });
    }

    // Нового нічого немає: якщо є прострочені повторення, пропонуємо їх і поза слотом.
    due.first().map(review_selection)
}

#[derive(Debug)]
pub enum Pick {
    Task(Box<Task>),
    /// Нових умов для цієї навички більше немає.
    Exhausted,
}

/// Бере наступну ще не бачену умову й позначає її баченою. Коли весь скінченний пул показано, починається
/// нове коло: умови, не бачені серед останніх `repeat_guard_recent`, знову можна давати. Лише якщо навіть так
/// нових умов немає (пул не більший за вікно), навичку позначено вичерпаною.
pub fn pick_task(gen: &dyn SkillGenerator, learner: &mut LearnerState, cfg: &Config) -> Pick {
    let skill = gen.skill_id();
    for round in 0..2 {
        for _ in 0..MAX_VARIANT_TRIES {
            let variant = learner.next_variant(skill);
            let task = gen.generate(variant);
            if learner.mark_seen(skill, &task.statement) {
                return Pick::Task(Box::new(task));
            }
        }
        if round == 0 && !learner.recycle_seen(skill, cfg.repeat_guard_recent) {
            break;
        }
    }
    learner.mark_exhausted(skill);
    Pick::Exhausted
}

/// Умова аналога, який дитині вже показали, надалі не дається як окрема задача.
pub fn note_analog_shown(learner: &mut LearnerState, ex: &WorkedExample) {
    learner.mark_seen(ex.task.skill, &ex.task.statement);
}

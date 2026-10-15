//! Заняття: початок, відповіді, підказки, пакети, відгуки, статистика.

use super::*;

pub(super) fn handle(
    env: &Env,
    open: &mut Open,
    name: &str,
    args: &Value,
    ctx: &Ctx,
) -> Result<Option<Reply>, ApiError> {
    let out = match name {
        "session_state" => json!({}),
        "session_start" => flow::session_start(env, open, args, ctx)?,
        "practice_start" => flow::practice_start(env, open, args, ctx)?,
        "testout_start" => flow::testout_start(env, open, args, ctx)?,
        "review_start" => flow::review_start(env, open, args, ctx)?,
        "intro_continue" => flow::intro_continue(env, open, ctx)?,
        "packs_list" => {
            let base: std::collections::BTreeMap<String, u32> =
                subjects::base_versions().into_iter().collect();
            json!({"packs": subjects::pack_versions().into_iter().map(|(id, v)| json!({
                "id": id, "version": v, "updated": base.get(&id).is_some_and(|b| v > *b),
            })).collect::<Vec<_>>()})
        }
        "pack_install" => {
            // Підписаний пакет вмісту з файлу: перевірка підпису, формату й версії; діє після перезапуску застосунку.
            let data = base64::decode(str_arg(args, "data")?)
                .ok_or_else(|| ApiError::bad_request("файл пошкоджено"))?;
            let sig = base64::decode(str_arg(args, "sig")?)
                .ok_or_else(|| ApiError::bad_request("підпис пошкоджено"))?;
            let id = crate::packs::install(&env.packs_dir, &data, &sig)?;
            json!({"id": id, "restart": true})
        }
        "flag_item" => flow::flag_item(env, open, args)?,
        "backup_snooze" => {
            open.state.backup_snooze_day = open.state.learner.today().saturating_add(7);
            json!({"ok": true})
        }
        "week" => {
            sync_day(open, ctx);
            let today = open.state.learner.today();
            let days: Vec<Value> = (0..7u32)
                .rev()
                .map(|back| {
                    let day = today.saturating_sub(back);
                    let (t, s) = open.state.activity.get(&day).copied().unwrap_or((0, 0));
                    json!({"day": day, "tasks": t, "solved": s, "active": t > 0})
                })
                .collect();
            let tasks_total: u32 = days
                .iter()
                .map(|d| d["tasks"].as_u64().unwrap_or(0) as u32)
                .sum();
            let solved_total: u32 = days
                .iter()
                .map(|d| d["solved"].as_u64().unwrap_or(0) as u32)
                .sum();
            let active_days = days.iter().filter(|d| d["active"] == true).count();
            json!({"days": days, "active_days": active_days, "tasks_total": tasks_total,
                   "solved_total": solved_total, "goal": open.state.settings.weekly_goal_days,
                   "stickers": open.state.album.count()})
        }
        "flags_export" => {
            // Список проблем із завданнями для авторів: тема, ключ, причина, умова. Без імені й часу.
            let flags: Vec<Value> = open
                .state
                .flags
                .iter()
                .map(|f| json!({"skill": f.skill, "key": f.key, "reason": f.reason, "statement": f.statement}))
                .collect();
            let doc = json!({"schema": "kmit-flags-1", "flags": flags});
            json!({"data": base64::encode(doc.to_string().as_bytes()), "filename": "kmit-pomylky.json", "preview": doc})
        }
        "placement_next" => {
            let subject = args
                .get("subject")
                .and_then(Value::as_str)
                .unwrap_or("math");
            if !crate::catalog::subject_list(open.state.settings.grade)
                .iter()
                .any(|(id, _)| *id == subject)
            {
                return Err(ApiError::bad_request("невідомий предмет"));
            }
            let sg = crate::catalog::sub_graph(&env.graph, subject, open.state.settings.grade);
            let next = sg
                .topological_order()
                .unwrap_or_default()
                .into_iter()
                .find(|id| {
                    !open.state.learner.is_mastered(id)
                        && sg.get(id).is_some_and(|n| {
                            n.prerequisites
                                .iter()
                                .all(|p| open.state.learner.is_mastered(p))
                        })
                        && crate::catalog::generator_for(id).is_some()
                });
            match next {
                Some(id) => json!({"skill": id, "title": skill_title(env, id)}),
                None => json!({"skill": null}),
            }
        }
        "reflect" => {
            let skill = skill_arg(env, args)?.to_string();
            open.state
                .learner
                .record_ask_adult(&skill, "Дитина відзначила цю тему складною");
            json!({"ok": true})
        }
        "stats_export" => {
            // Анонімна статистика якості завдань: лише (тема, номер завдання, спроби, правильні); без імені, часу, профілю.
            let items: Vec<Value> = open
                .state
                .item_stats
                .iter()
                .map(|((skill, item), (a, c))| json!({"skill": skill, "item": item, "attempts": a, "correct": c}))
                .collect();
            let doc = json!({"schema": "kmit-item-stats-1", "items": items});
            let text = doc.to_string();
            json!({"data": base64::encode(text.as_bytes()), "filename": "kmit-statystyka.json", "preview": doc})
        }
        "answer" => flow::answer(env, open, args, ctx)?,
        "hint" => flow::hint(env, open, ctx)?,
        "hand_to_adult" => flow::hand_to_adult(env, open, ctx)?,
        "back_to_task" => flow::back_to_task(open)?,
        "continue" => flow::continue_(env, open, ctx)?,
        "end_session" => flow::end_session(env, open, ctx)?,
        _ => return Ok(None),
    };
    Ok(Some(Reply::Done(out)))
}

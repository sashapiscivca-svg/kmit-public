//! Уроки, повторення, «пригадування», профіль і план.

use super::*;

pub(super) fn handle(
    env: &Env,
    open: &mut Open,
    name: &str,
    args: &Value,
    ctx: &Ctx,
) -> Result<Option<Reply>, ApiError> {
    let out = match name {
        "lesson" => {
            ensure_no_open_task(open)?;
            let id = skill_arg(env, args)?;
            let l = env
                .lessons
                .lesson(id)
                .ok_or_else(|| ApiError::bad_request("для цієї теми ще немає уроку"))?;
            lesson_json(id, &l, &open.state)
        }
        "grades" => crate::grades::grades_json(open.state.settings.grade),
        "review_overview" => crate::repeat::overview(env, open, ctx),
        "profile" => {
            let subject = str_arg(args, "subject")?;
            crate::repeat::profile(env, open, subject, ctx)
        }
        "plan" => crate::repeat::plan(env, open, ctx),
        "memory_graph" => {
            ensure_no_open_task(open)?;
            let subject = str_arg(args, "subject")?;
            crate::repeat::memory_graph(env, open, subject, ctx)
        }
        "recall_queue" => crate::recall::queue_json(env, &mut open.state, ctx.day),
        "recall_show" => {
            let id = skill_arg(env, args)?;
            let text = args.get("text").and_then(Value::as_str).unwrap_or("");
            let r = crate::recall::show(env, id, text);
            if r["accepted"] == true {
                open.recall_shown.insert(id.to_string());
            }
            r
        }
        "recall_rate" => {
            let id = skill_arg(env, args)?.to_string();
            if !open.recall_shown.remove(&id) {
                return Err(ApiError::bad_request("спершу напиши своїми словами"));
            }
            let rating = args.get("rating").and_then(Value::as_str).unwrap_or("");
            let n = crate::recall::rate(&mut open.state, &id, rating, ctx.day)
                .ok_or_else(|| ApiError::bad_request("хибна самооцінка"))?;
            if rating != "no" {
                crate::weekly::note_review(&mut open.state, ctx.day);
            }
            json!({"ok": true, "recalls": n})
        }
        "lesson_example" => {
            ensure_no_open_task(open)?;
            let id = skill_arg(env, args)?.to_string();
            let index = small_index(args)?;
            let l = env
                .lessons
                .lesson(&id)
                .ok_or_else(|| ApiError::bad_request("для цієї теми ще немає уроку"))?;
            if index >= crate::lesson_prog::counts(&l).1 {
                return Err(ApiError::bad_request("такого прикладу немає"));
            }
            *open.state.lessons.entry(format!("{id}#ex")).or_insert(0) |= 1 << index;
            crate::lesson_prog::progress_json(&l, &id, &open.state)
        }
        "lesson_explain" => {
            ensure_no_open_task(open)?;
            let id = skill_arg(env, args)?.to_string();
            let index = small_index(args)?;
            let l = env
                .lessons
                .lesson(&id)
                .ok_or_else(|| ApiError::bad_request("для цієї теми ще немає уроку"))?;
            let Some(LessonBlock::Explain { key_ideas, .. }) = l
                .blocks
                .iter()
                .filter(|b| matches!(b, LessonBlock::Explain { .. }))
                .nth(index as usize)
            else {
                return Err(ApiError::bad_request("такого пояснення немає"));
            };
            // «Не знаю, покажи ідеї»: ідеї показуються без зарахування кроку (навчання нічим не блокується, A9).
            if args.get("skip").and_then(Value::as_bool) == Some(true) {
                return early(
                    json!({"accepted": true, "skipped": true, "unlocked": true, "key_ideas": key_ideas}),
                );
            }
            let text = args.get("text").and_then(Value::as_str).unwrap_or("");
            if !crate::lesson_prog::is_real_text(text, 8, 30) {
                return early(json!({"accepted": false,
                    "message": "Напиши кілька речень своїми словами (не менше восьми різних слів), тоді я покажу головні ідеї."}));
            }
            match args.get("recalled").and_then(Value::as_array) {
                None => json!({"accepted": true, "key_ideas": key_ideas}),
                Some(r) if r.len() == key_ideas.len() => {
                    *open.state.lessons.entry(format!("{id}#xp")).or_insert(0) |= 1 << index;
                    let got = r.iter().filter(|v| v.as_bool() == Some(true)).count();
                    json!({"accepted": true, "key_ideas": key_ideas, "recalled": got, "done": true,
                        "progress": crate::lesson_prog::progress_json(&l, &id, &open.state)})
                }
                Some(_) => return Err(ApiError::bad_request("хибний перелік ідей")),
            }
        }
        "lesson_check" => {
            ensure_no_open_task(open)?;
            let id = skill_arg(env, args)?.to_string();
            let index = small_index(args)?;
            let lesson = env
                .lessons
                .lesson(&id)
                .ok_or_else(|| ApiError::bad_request("для цієї теми ще немає уроку"))?;
            let input = args.get("input").and_then(Value::as_str).unwrap_or("");
            let give_up = args.get("give_up").and_then(Value::as_bool) == Some(true);
            let Some(block) = lesson
                .blocks
                .iter()
                .filter(|b| matches!(b, LessonBlock::SelfCheck { .. } | LessonBlock::Quiz { .. }))
                .nth(index as usize)
            else {
                return Err(ApiError::bad_request("такого питання немає"));
            };
            if give_up {
                // «Показати розбір»: після власної спроби чи підказки; відзнаки за це питання не буде.
                if open.lesson_wrong_day != ctx.day {
                    open.lesson_wrong.clear();
                    open.lesson_hints.clear();
                    open.lesson_wrong_day = ctx.day;
                }
                let key = (id.clone(), index);
                let tried = open.lesson_wrong.get(&key).copied().unwrap_or(0) >= 1
                    || open.lesson_hints.get(&key).copied().unwrap_or(0) >= 1;
                if !tried {
                    return Err(ApiError::bad_request(
                        "Спершу спробуй відповісти або візьми підказку",
                    ));
                }
                open.lesson_wrong.insert(key, 2);
                let (explanation, answer) = match block {
                    LessonBlock::SelfCheck {
                        answer,
                        explanation,
                        ..
                    } => (explanation.clone(), answer.clone()),
                    LessonBlock::Quiz {
                        options,
                        correct,
                        explanation,
                        ..
                    } => (explanation.clone(), options[*correct].clone()),
                    _ => return Err(ApiError::bad_request("такого питання немає")),
                };
                return early(
                    json!({"correct": false, "credited": false, "gave_up": true, "explanation": explanation,
                    "answer": answer, "unlocked": true, "attempts_left": 0,
                    "progress": crate::lesson_prog::progress_json(&lesson, &id, &open.state)}),
                );
            }
            let unreadable = |what: &str| json!({"correct": false, "unreadable": true, "explanation": what, "answer": ""});
            let (correct, explanation, answer) = match block {
                LessonBlock::SelfCheck {
                    answer,
                    explanation,
                    ..
                } => {
                    let given = (input.chars().count() <= 40)
                        .then(|| parse_answer(input.trim()))
                        .flatten();
                    let Some(given) = given else {
                        return early(unreadable(
                            "Не вдалося прочитати відповідь. Напиши число, дріб 3/4 або десятковий дріб 3,5.",
                        ));
                    };
                    (
                        parse_answer(answer) == Some(given),
                        explanation.clone(),
                        answer.clone(),
                    )
                }
                LessonBlock::Quiz {
                    options,
                    correct,
                    explanation,
                    ..
                } => {
                    let pos = input
                        .trim()
                        .parse::<usize>()
                        .ok()
                        .filter(|p| (1..=options.len()).contains(p));
                    let Some(pos) = pos else {
                        return early(unreadable("Обери один з варіантів."));
                    };
                    (
                        pos - 1 == *correct,
                        explanation.clone(),
                        options[*correct].clone(),
                    )
                }
                _ => return Err(ApiError::bad_request("такого питання немає")),
            };
            if open.lesson_wrong_day != ctx.day {
                open.lesson_wrong.clear();
                open.lesson_hints.clear();
                open.lesson_wrong_day = ctx.day;
            }
            let key = (id.clone(), index);
            let wrong = open.lesson_wrong.get(&key).copied().unwrap_or(0);
            let burnt = wrong >= 2;
            if correct && !burnt {
                *open.state.lessons.entry(id.clone()).or_insert(0) |= 1 << index;
            }
            if !correct {
                open.lesson_wrong.insert(key, wrong.saturating_add(1));
            }
            // Перша хибна спроба: підказка без відповіді; з другої: розбір із правильною відповіддю (C42), крок розблоковано,
            // але відзнака за нього не нараховується (отримати її можна, лише відповівши самому).
            let reveal = correct || wrong + 1 >= 2;
            json!({"correct": correct, "credited": correct && !burnt,
                   "explanation": if reveal { explanation } else { String::new() },
                   "answer": if reveal { answer } else { String::new() },
                   "unlocked": correct || wrong + 1 >= 2,
                   "attempts_left": 2u8.saturating_sub(wrong + 1),
                   "progress": crate::lesson_prog::progress_json(&lesson, &id, &open.state)})
        }
        "lesson_hint" => {
            ensure_no_open_task(open)?;
            let id = skill_arg(env, args)?.to_string();
            let index = small_index(args)?;
            let lesson = env
                .lessons
                .lesson(&id)
                .ok_or_else(|| ApiError::bad_request("для цієї теми ще немає уроку"))?;
            let Some(block) = lesson
                .blocks
                .iter()
                .filter(|b| matches!(b, LessonBlock::SelfCheck { .. } | LessonBlock::Quiz { .. }))
                .nth(index as usize)
            else {
                return Err(ApiError::bad_request("такого питання немає"));
            };
            if open.lesson_wrong_day != ctx.day {
                open.lesson_wrong.clear();
                open.lesson_hints.clear();
                open.lesson_wrong_day = ctx.day;
            }
            let key = (id, index);
            let level = (open.lesson_hints.get(&key).copied().unwrap_or(0) + 1).min(2);
            open.lesson_hints.insert(key, level);
            // Підказка не розкриває відповіді: спершу «куди дивитися», потім для питань з вибором прибирається один
            // хибний варіант. Відзнака за питання зберігається (підказка допомагає думати, а не відповідає замість дитини).
            match block {
                LessonBlock::Quiz {
                    correct, options, ..
                } if level == 2 => {
                    let remove = (0..options.len()).find(|i| i != correct);
                    json!({"level": 2, "kind": "eliminate", "remove": remove})
                }
                _ => json!({"level": level, "kind": if level == 1 { "reread" } else { "method" }}),
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(Reply::Done(out)))
}

//! Головна, тиждень, герой, альбом, сторінка теми.

use super::*;

pub(super) fn handle(
    env: &Env,
    open: &mut Open,
    name: &str,
    args: &Value,
    ctx: &Ctx,
) -> Result<Option<Reply>, ApiError> {
    let out = match name {
        "home" => home(env, open, ctx),
        "week_status" => crate::weekly::status(&open.state, ctx.day),
        "week_claim" => {
            let week = args
                .get("week")
                .and_then(Value::as_u64)
                .and_then(|w| u32::try_from(w).ok())
                .ok_or_else(|| ApiError::bad_request("немає тижня"))?;
            let item = str_arg(args, "item")?;
            let r = crate::weekly::claim(&mut open.state, ctx.day, week, item)
                .map_err(ApiError::bad_request)?;
            json!({"claimed": r["item"], "status": crate::weekly::status(&open.state, ctx.day),
                   "look": crate::hero_api::look_json(&open.state)})
        }
        "hero_catalog" => crate::hero_api::catalog_json(&open.state),
        "hero_set" => crate::hero_api::set(&mut open.state, args)?,
        "hero_pdf" => {
            let weeks_done = open
                .state
                .weeks
                .values()
                .filter(|w| w.claimed.is_some())
                .count();
            let pdf = crate::hero_api::poster_pdf(&open.state, weeks_done)?;
            json!({"data": base64::encode(&pdf), "filename": "mij-heroj.pdf"})
        }
        "ceremony_status" => crate::ceremony::status_json(env, &open.state),
        "ceremony_pdf" => {
            let from = u8_arg(args, "from")?;
            if !open.state.ceremonies.contains(&from) {
                return Err(ApiError::bad_request("Свято цього класу ще не було"));
            }
            let (m, t) = crate::ceremony::progress(env, &open.state, from);
            let stickers = env
                .graph
                .ids()
                .into_iter()
                .filter(|id| crate::catalog::grade_of(id) == from)
                .map(|id| {
                    tutor_core::album::StickerKind::ALL
                        .iter()
                        .filter(|k| open.state.album.has(**k, id))
                        .count()
                })
                .sum::<usize>();
            let hero = tutor_core::hero::compose(&open.state.hero);
            let (_, from_world) = crate::catalog::world_of(from);
            let (_, to_world) = crate::catalog::world_of(from.saturating_add(1));
            let date = crate::datefmt::date_of(ctx.day);
            let cert = report::styled::Certificate {
                nickname: &open.state.profile.nickname,
                from,
                to: from.saturating_add(1),
                world_from: from_world,
                world_to: to_world,
                mastered: m,
                total: t,
                stickers,
                date: &date,
                finale: crate::ceremony::is_final(from),
            };
            let pdf = report::styled::certificate_pdf(&cert, &hero)
                .map_err(|_| ApiError::new("pdf", "Не вдалося створити свідоцтво"))?;
            json!({"data": base64::encode(&pdf), "filename": "svidotstvo-pereihid.pdf"})
        }
        "ceremony_do" => crate::ceremony::run(env, &mut open.state)?,
        "album" => {
            let grade = args
                .get("grade")
                .and_then(Value::as_u64)
                .and_then(|g| u8::try_from(g).ok())
                .filter(|g| crate::grades::valid(*g))
                .unwrap_or(open.state.settings.grade);
            let mut v = album_json(env, &open.state.album, grade);
            v["grade"] = json!(grade);
            v["world"] = json!({"id": crate::catalog::world_of(grade).0, "name": crate::catalog::world_of(grade).1});
            v["worlds"] = json!(crate::catalog::content_grades()
                .into_iter()
                .filter(|g| *g <= open.state.settings.grade.max(open.state.ceremonies.iter().max().map_or(0, |m| m + 1)))
                .map(|g| json!({"grade": g, "world": crate::catalog::world_of(g).1, "id": crate::catalog::world_of(g).0, "ceremony_done": open.state.ceremonies.contains(&g)}))
                .collect::<Vec<_>>());
            let cat = badges::catalog(env);
            v["badges"] = json!(cat
                .iter()
                .map(|b| b.to_json(open.state.badges.contains(&b.id)))
                .collect::<Vec<_>>());
            v["badges_count"] = json!(open.state.badges.len());
            v["badges_total"] = json!(cat.len());
            v
        }
        "skill_page" => {
            let id = skill_arg(env, args)?;
            let state = skill_state(env, &open.state.learner, id);
            json!({
                "id": id,
                "title": skill_title(env, id),
                "state": state,
                "mastered": state == "mastered",
                "has_lesson": env.lessons.lesson(id).is_some(),
                "review": subjects::review_status(id),
                "note_chars": open.state.notes.get(id).map_or(0, |t| crate::lesson_prog::note_body(t).chars().count()),
                "note_prompts": env.lessons.lesson(id).map(|l| l.note_prompts).unwrap_or_default(),
            })
        }
        _ => return Ok(None),
    };
    Ok(Some(Reply::Done(out)))
}

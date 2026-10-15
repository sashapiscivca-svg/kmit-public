//! Команди вчителя й вичитки (лише профіль вчителя).

use super::*;

pub(super) fn handle(
    env: &Env,
    open: &mut Open,
    name: &str,
    args: &Value,
    ctx: &Ctx,
) -> Result<Option<Reply>, ApiError> {
    let out = match name {
        "teacher_lesson" => {
            let id = skill_arg(env, args)?;
            let l = env
                .lessons
                .lesson(id)
                .ok_or_else(|| ApiError::bad_request("для цієї теми ще немає уроку"))?;
            let mut v = lesson_json(id, &l, &open.state);
            v["reviewer"] = reviewer_answers(&l);
            v["review"] = json!(subjects::review_status(id));
            v["grade"] = json!(crate::catalog::grade_of(id));
            v
        }
        "teacher_lesson_plan_pdf" => {
            let id = skill_arg(env, args)?;
            let grade = crate::catalog::grade_of(id);
            let seed = u64::from(ctx.day) * 7919 + 13;
            let doc = crate::teacher::lesson_plan_doc(env, id, grade, seed)?;
            let pdf = report::work::lesson_plan_pdf(&doc)
                .map_err(|_| ApiError::new("pdf", "Не вдалося створити PDF"))?;
            json!({"data": base64::encode(&pdf), "filename": "plan-uroku.pdf"})
        }
        "teacher_handout_pdf" => {
            let id = skill_arg(env, args)?;
            let doc = crate::teacher::handout_doc(env, id, crate::catalog::grade_of(id))?;
            let pdf = report::work::lesson_plan_pdf(&doc)
                .map_err(|_| ApiError::new("pdf", "Не вдалося створити PDF"))?;
            json!({"data": base64::encode(&pdf), "filename": "pamiatka-dlia-uchniv.pdf"})
        }
        "teacher_plan_pdf" => {
            let grade = u8_arg(args, "grade")?;
            let subject = str_arg(args, "subject")?;
            let lessons = u8_arg(args, "lessons").unwrap_or(2).clamp(1, 6);
            let doc = crate::teacher::topic_plan_doc(env, grade, subject, lessons)?;
            let pdf = report::work::topic_plan_pdf(&doc)
                .map_err(|_| ApiError::new("pdf", "Не вдалося створити PDF"))?;
            json!({"data": base64::encode(&pdf), "filename": "tematychnyi-plan.pdf"})
        }
        "teacher_home" => crate::teacher::home_json(env, &open.state, ctx.day),
        "teacher_celebrate_seen" => {
            let id = args.get("id").and_then(Value::as_str).unwrap_or("");
            if !crate::teacher::celebration_seen(&mut open.state, id) {
                return Err(ApiError::bad_request("Невідоме свято"));
            }
            json!({"ok": true})
        }
        "teacher_search" => {
            let q = args.get("q").and_then(Value::as_str).unwrap_or("");
            crate::teacher::search_json(env, q)
        }
        "teacher_bookmark" => {
            let id = skill_arg(env, args)?;
            let on = args.get("on").and_then(Value::as_bool).unwrap_or(true);
            crate::teacher::bookmark_set(&mut open.state, id, on)?
        }
        "teacher_topics" => {
            let grade = u8_arg(args, "grade").unwrap_or(5);
            crate::teacher::topics_json(env, grade)
        }
        "teacher_work_build" => {
            let spec = crate::teacher::spec_from(args, ctx.day)?;
            let work = crate::teacher::build_work(env, &spec)?;
            let t = &mut open.state.teacher;
            t.works_made = t.works_made.saturating_add(1);
            t.works.push(storage::state::WorkRecord {
                day: ctx.day,
                grade: spec.grade,
                kind: spec.kind.code().to_string(),
                skills: spec.skills.clone(),
                variants: spec.variants,
                seed: spec.seed,
            });
            while t.works.len() > storage::state::MAX_TEACHER_WORKS {
                t.works.remove(0);
            }
            json!({"work": crate::teacher::work_json(&work), "seed": spec.seed})
        }
        "teacher_work_pdf" => {
            let spec = crate::teacher::spec_from(args, ctx.day)?;
            let work = crate::teacher::build_work(env, &spec)?;
            let doc = crate::teacher::work_doc(&spec, &work);
            let (pdf, name) = match args.get("part").and_then(Value::as_str) {
                Some("tasks") => (report::work::work_tasks_pdf(&doc), "robota-zavdannia.pdf"),
                Some("cards") => (report::work::work_cards_pdf(&doc), "robota-kartky.pdf"),
                Some("key") => (report::work::work_key_pdf(&doc), "robota-kliuch.pdf"),
                Some("answers") => (
                    report::work::work_answers_pdf(&doc),
                    "robota-blank-vidpovidei.pdf",
                ),
                _ => return Err(ApiError::bad_request("Невідома частина роботи")),
            };
            let pdf = pdf.map_err(|_| ApiError::new("pdf", "Не вдалося створити PDF"))?;
            json!({"data": base64::encode(&pdf), "filename": name})
        }
        _ => return Ok(None),
    };
    Ok(Some(Reply::Done(out)))
}

//! Ядро застосунку: єдиний JSON-інтерфейс команд для оболонок (Tauri на Android, браузерний сервер для
//! настільного тесту). Оболонка лише передає назву команди, аргументи й поточний час; уся логіка тут.
//!
//! Правило безпеки: усі відповіді будуються з типів-подань (`ChildTaskView`, `ChildHint`) і готових текстів;
//! відповідь, проміжні результати й типові помилки задачі в JSON потрапити не можуть.

mod app;
mod badges;
pub mod base64;
mod catalog;
mod ceremony;
mod ctx;
mod datefmt;
mod error;
mod flow;
mod grades;
mod hero_api;
mod lesson_prog;
mod packs;
mod recall;
mod repeat;
#[cfg(feature = "reviewer")]
mod review;
mod teacher;
mod views;
mod weekly;

pub use app::{App, HookFactory};
pub use ctx::Ctx;
pub use error::ApiError;

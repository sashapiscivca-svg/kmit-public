//! Аркуш для вчителя у форматі PDF: власний мінімальний генератор із вбудованим шрифтом (кирилиця).

pub mod canvas;
pub mod notes;
pub mod pdf;
pub mod sheet;
pub mod styled;
pub mod ttf;
pub mod work;

/// Вбудований шрифт: Liberation Sans Regular (SIL OFL 1.1, див. `assets/LICENSE-LiberationSans.txt`).
pub static FONT_BYTES: &[u8] = include_bytes!("../assets/LiberationSans-Regular.ttf");

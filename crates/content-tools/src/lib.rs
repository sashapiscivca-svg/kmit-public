//! Інструменти для вмісту: перевірка пакетів, звіт для методиста (CSV/HTML), застосування виправлень, підпис.

pub mod csv;
pub mod review;
#[cfg(feature = "sign")]
pub mod sign;

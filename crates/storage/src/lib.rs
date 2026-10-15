//! Зашифроване локальне сховище.

pub mod crypto;
pub mod error;
pub mod keyfile;
pub mod profiles;
pub mod state;
pub mod vault;

pub use error::StorageError;

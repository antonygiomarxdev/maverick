pub mod adapters;
pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod events;
pub mod ingester;
pub mod kernel;
pub mod ports;
pub mod storage_profile;
pub mod use_cases;

pub use error::{AppError, DomainError, Result};

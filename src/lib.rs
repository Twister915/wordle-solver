pub mod wordle;
pub mod util;
pub mod web;

#[cfg(debug_assertions)]
pub const LOG_LEVEL: log::Level = log::Level::Debug;

#[cfg(not(debug_assertions))]
pub const LOG_LEVEL: log::Level = log::Level::Info;
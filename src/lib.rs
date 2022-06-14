pub mod wordle;
pub mod util;
pub mod web;

#[cfg(debug_assertions)]
pub const LOG_LEVEL: log::Level = log::Level::Debug;

#[cfg(not(debug_assertions))]
pub const LOG_LEVEL: log::Level = log::Level::Info;

#[cfg(not(debug_assertions))]
pub const GIT_VERSION: &str = env!("GIT_HASH");
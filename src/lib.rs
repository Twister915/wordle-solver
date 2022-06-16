pub mod wordle;
pub mod util;
pub mod web;

#[cfg(not(debug_assertions))]
pub const GIT_VERSION: &str = env!("GIT_HASH");
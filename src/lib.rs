#![recursion_limit = "1024"]

mod app;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use wasm_bindgen::prelude::*;

#[cfg(debug_assertions)]
const LOG_LEVEL: log::Level = log::Level::Debug;

#[cfg(not(debug_assertions))]
const LOG_LEVEL: log::Level = log::Level::Info;

#[wasm_bindgen]
pub fn run_app() {
    wasm_logger::init(wasm_logger::Config::new(LOG_LEVEL));
    yew::start_app_with_props::<app::App>(());
}
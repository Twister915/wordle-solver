#![recursion_limit = "1024"]

use yew_agent::Threaded;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(debug_assertions)]
const LOG_LEVEL: log::Level = log::Level::Debug;

#[cfg(not(debug_assertions))]
const LOG_LEVEL: log::Level = log::Level::Debug;

pub fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::new(LOG_LEVEL));
    wordle_site::web::solver_agent::SolverAgent::register()
}
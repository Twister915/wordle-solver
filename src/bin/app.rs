#![recursion_limit = "1024"]

#[cfg(debug_assertions)]
pub const LOG_LEVEL: log::Level = log::Level::Debug;

#[cfg(not(debug_assertions))]
pub const LOG_LEVEL: log::Level = log::Level::Info;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::new(LOG_LEVEL));
    yew::start_app_with_props::<wordle_site::web::App>(());
}
#![recursion_limit = "1024"]

use wordle_site::LOG_LEVEL;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

pub fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    wasm_logger::init(wasm_logger::Config::new(LOG_LEVEL));
    yew::start_app_with_props::<wordle_site::web::App>(());
}
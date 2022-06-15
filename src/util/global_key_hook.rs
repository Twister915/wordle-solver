use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use std::fmt;

pub struct KeyEvent {
    event: Box<KeyboardEvent>,
    code: String,
    prevented: bool,
}

pub struct KeyListener {
    callback: Closure<dyn FnMut(KeyboardEvent)>,
}

impl KeyListener {
    pub fn create(target: Callback<KeyEvent>) -> Option<Self> {
        let window = web_sys::window()?;
        let callback = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            let code = event.code();
            target.emit(KeyEvent {
                event: Box::new(event),
                code,
                prevented: false,
            })
        }) as Box<dyn FnMut(_)>);

        let out = Self { callback };
        let cb_ref = out.callback.as_ref().unchecked_ref();
        if let Ok(()) = window.add_event_listener_with_callback("keydown", cb_ref) {
            log::debug!("registered global keydown listener");
            Some(out)
        } else {
            None
        }
    }
}

impl Drop for KeyListener {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            let cb_ref = self.callback.as_ref().unchecked_ref();
            if let Ok(()) = window.remove_event_listener_with_callback("keydown", cb_ref) {
                log::debug!("de-registered keydown callback");
                return;
            }
        }
    }
}

impl KeyEvent {
    pub fn prevent_default(&mut self) -> bool {
        if !self.prevented {
            log::debug!(
                "event for keycode {} was handled and will prevent_default",
                self.code()
            );
            self.event.prevent_default();
            self.prevented = true;
            true
        } else {
            false
        }
    }

    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    pub fn is_control_key(&self) -> bool {
        self.event.ctrl_key() || self.event.meta_key() || self.event.alt_key()
    }
}

impl fmt::Debug for KeyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KeyEvent[code={}, prevent_default={}]", self.code(), self.prevented)
    }
}
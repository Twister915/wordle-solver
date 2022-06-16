use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use yew::prelude::*;
use std::fmt;

pub struct KeyEvent {
    event: Box<KeyboardEvent>,
    code: String,
    prevented: bool,
}

/// Listens to keydown for the entire app. Allows you to have a message sent to your yew component
/// on keydown, and you can even prevent_default on the key event when you handle the message.
///
/// Automatically registers the listener on create, and de-registers on drop.
pub struct KeyListener {
    callback: Closure<dyn FnMut(KeyboardEvent)>,
}

impl KeyListener {
    /// Create a KeyListener with a given yew callback...
    ///
    /// This returns an Option because acquiring the JS window object itself returns an Option...
    /// Not really sure when the window would be None, but we propagate that problem to our caller.
    pub fn create(target: Callback<KeyEvent>) -> Option<Self> {
        let window = web_sys::window()?;
        // this is a sort of conversion from yew callback -> JS event handler
        let callback = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            let code = event.code();
            target.emit(KeyEvent {
                event: Box::new(event),
                code,
                prevented: false,
            })
        }) as Box<dyn FnMut(_)>);

        let cb_ref = callback.as_ref().unchecked_ref();
        if let Ok(()) = window.add_event_listener_with_callback("keydown", cb_ref) {
            log::debug!("registered global keydown listener");
            Some(Self { callback })
        } else {
            None
        }
    }
}

impl Drop for KeyListener {
    fn drop(&mut self) {
        // try to deregister the listener
        if let Some(window) = web_sys::window() {
            let cb_ref = self.callback.as_ref().unchecked_ref();
            if let Ok(()) = window.remove_event_listener_with_callback("keydown", cb_ref) {
                log::debug!("de-registered keydown callback");
                return;
            }
        }

        log::warn!("did not remove global keydown callback listener!")
    }
}

impl KeyEvent {
    /// When you get a KeyEvent and it causes some behavior in your app, you usually want to stop
    /// that event from causing other behavior on the webpage. This function calls the JS function
    /// prevent_default (checked for exactly once), and returns whether or not that did anything
    /// (which is useful because yew update functions should return true/false depending on
    /// whether or not they actually did anything).
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

    /// Returns the code as provided by JS (event.code)
    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    /// true when a control key is active at the same time as this key event (so we can handle the
    /// 'r' key being pressed without breaking Command+R/Control+R for refresh, for example)
    pub fn is_control_key(&self) -> bool {
        self.event.ctrl_key() || self.event.meta_key() || self.event.alt_key()
    }
}

impl fmt::Debug for KeyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KeyEvent[code={}, prevent_default={}]", self.code(), self.prevented)
    }
}
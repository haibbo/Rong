use crate::event_emitter::{Emitter, Events};
use rusty_js::*;

/// Represents an event target that can receive events and may have listeners for them.
#[js_class]
pub struct EventTarget {
    events: Events,
}

#[js_methods]
impl EventTarget {
    #[js_method(constructor)]
    pub fn new() -> Self {
        Self {
            events: Events::new(),
        }
    }
}

impl Default for EventTarget {
    fn default() -> Self {
        Self::new()
    }
}

impl Emitter for EventTarget {
    fn get_events(&self) -> &Events {
        &self.events
    }

    fn get_events_mut(&mut self) -> &mut Events {
        &mut self.events
    }
}

use crate::event_emitter::{Emitter, EventEmitter};
use rusty_js::*;

/// Represents an event target that can receive events and may have listeners for them.
#[js_class]
pub struct EventTarget {
    events: EventEmitter,
}

#[js_methods]
impl EventTarget {
    #[js_method(constructor)]
    pub fn new() -> Self {
        Self {
            events: EventEmitter::new(),
        }
    }
}

impl Default for EventTarget {
    fn default() -> Self {
        Self::new()
    }
}

impl Emitter for EventTarget {
    fn get_event_emitter(&self) -> &EventEmitter {
        &self.events
    }

    fn get_mut_event_emitter(&mut self) -> &mut EventEmitter {
        &mut self.events
    }
}

use rusty_js::{function::*, *};

use super::Event;

/// Represents a custom event object
#[js_class]
pub struct CustomEvent {
    detail: Option<JSValue>,
    event: Event,
}

#[js_methods]
impl CustomEvent {
    #[js_method(constructor)]
    pub fn new(type_: String, options: Optional<JSObject>) -> Self {
        // Clone the options object to avoid borrowing issues
        let detail = options
            .0
            .as_ref()
            .and_then(|opts| opts.get::<_, JSValue>("detail").ok());

        // Create new Event with cloned options
        Self {
            detail,
            event: Event::new(type_, options),
        }
    }

    // Returns the custom data
    #[js_method(getter)]
    pub fn detail(&self) -> Option<JSValue> {
        self.detail.clone()
    }

    // Delegate Event methods
    #[js_method(getter, rename = "type")]
    pub fn type_(&self) -> String {
        self.event.type_()
    }

    #[js_method(getter)]
    pub fn bubbles(&self) -> bool {
        self.event.bubbles()
    }

    #[js_method(getter)]
    pub fn cancelable(&self) -> bool {
        self.event.cancelable()
    }

    #[js_method(getter)]
    pub fn composed(&self) -> bool {
        self.event.composed()
    }
}

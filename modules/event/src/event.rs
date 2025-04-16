use rong::{function::*, *};

/// Represents an event object
#[js_export]
#[derive(Default)]
pub struct Event {
    pub(crate) type_: String,
    bubbles: bool,
    cancelable: bool,
    // whether the event will trigger listeners outside of a shadow root
    composed: bool,
}

#[js_class]
impl Event {
    #[js_method(constructor)]
    pub fn new(type_: String, options: Optional<JSObject>) -> Self {
        let mut event = Self::default();

        // Parse options if provided
        if let Some(opts) = options.0 {
            if let Ok(b) = opts.get::<_, bool>("bubbles") {
                event.bubbles = b;
            }
            if let Ok(c) = opts.get::<_, bool>("cancelable") {
                event.cancelable = c;
            }
            if let Ok(comp) = opts.get::<_, bool>("composed") {
                event.composed = comp;
            }
        }
        event.type_ = type_;
        event
    }

    // Returns the type of the event
    #[js_method(getter, rename = "type")]
    pub fn type_(&self) -> String {
        self.type_.clone()
    }

    // Returns whether the event bubbles
    #[js_method(getter)]
    pub fn bubbles(&self) -> bool {
        self.bubbles
    }

    // Returns whether the event can be canceled
    #[js_method(getter)]
    pub fn cancelable(&self) -> bool {
        self.cancelable
    }

    // Returns whether the event will trigger listeners outside of a shadow root
    #[js_method(getter)]
    pub fn composed(&self) -> bool {
        self.composed
    }
}

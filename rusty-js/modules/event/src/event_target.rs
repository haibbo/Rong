use rusty_js::{function::*, *};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

#[derive(Clone)]
struct EventListener {
    type_: String,
    callback: JSFunc,
    // A boolean indicating the listener should be invoked at most once
    once: bool,
    capture: bool,
}

/// Represents an event target that can receive events and may have listeners for them.
#[js_class]
pub struct EventTarget {
    listeners: Rc<Mutex<HashMap<String, Vec<EventListener>>>>,
}

#[js_methods]
impl EventTarget {
    #[js_method(constructor)]
    pub fn new() -> Self {
        Self {
            listeners: Rc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Appends an event listener for events whose type attribute value is type.
    /// The options parameter can be either a boolean (indicating useCapture) or an object with:
    #[js_method(rename = "addEventListener")]
    pub fn add_eventlistener(&self, type_: String, listener: JSFunc, options: Optional<JSValue>) {
        let mut listeners = self.listeners.lock().unwrap();

        let mut once = false;
        let mut capture = false;

        if let Some(opts) = options.0 {
            if let Some(opts_obj) = opts.into_object() {
                if let Ok(o) = opts_obj.get::<_, bool>("once") {
                    once = o;
                }
                if let Ok(c) = opts_obj.get::<_, bool>("capture") {
                    capture = c;
                }
            }
        }

        let event_listener = EventListener {
            callback: listener.clone(),
            capture,
            once,
            type_: type_.clone(),
        };

        // Check if same listener with same options already exists
        let type_listeners = listeners.entry(type_).or_default();
        if !type_listeners.iter().any(|l| 
            l.callback == listener && l.capture == capture && l.once == once
        ) {
            type_listeners.push(event_listener);
        }
    }

    /// Removes the event listener in target's event listener list with the same type and callback.
    #[js_method(rename = "removeEventListener")]
    pub fn remove_eventlistener(
        &self,
        type_: String,
        listener: JSFunc,
        options: Optional<JSValue>,
    ) {
        let mut listeners = self.listeners.lock().unwrap();

        let capture = options
            .0
            .and_then(|opts| opts.into_object())
            .and_then(|obj| obj.get::<_, bool>("capture").ok())
            .unwrap_or(false);

        if let Some(type_listeners) = listeners.get_mut(&type_) {
            type_listeners
                .retain(|l| !(type_ == l.type_ && l.callback == listener && l.capture == capture));
        }
    }

    /// Dispatches a synthetic event to target and returns true if either event's cancelable attribute value is false
    /// or its preventDefault() method was not invoked, and false otherwise.
    #[js_method(rename = "dispatchEvent")]
    pub fn dispatch_event(&self, this: This<JSObject>, event: JSObject) -> bool {
        let event_type = match event.get::<_, String>("type") {
            Ok(t) => t,
            Err(_) => return true,
        };

        let listeners = self.listeners.lock().unwrap();
        if let Some(type_listeners) = listeners.get(&event_type) {
            let mut to_remove = Vec::new();

            for (index, listener) in type_listeners.iter().enumerate() {
                let _ = listener
                    .callback
                    .call_with_this::<_, ()>(this.clone(), (event.clone(),));
                if listener.once {
                    to_remove.push(index);
                }
            }

            if !to_remove.is_empty() {
                drop(listeners); // Release lock for modification
                let mut listeners = self.listeners.lock().unwrap();
                if let Some(type_listeners) = listeners.get_mut(&event_type) {
                    // Remove from end to avoid index shifting
                    for index in to_remove.into_iter().rev() {
                        type_listeners.remove(index);
                    }
                }
            }
        }

        true
    }
}

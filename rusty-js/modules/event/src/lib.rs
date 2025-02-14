//! Event handling implementation following the Web standard.
//!
//! This module provides event handling functionality that matches the behavior of Web APIs:
//! - [`Event`]: The base event object
//! - [`CustomEvent`]: For custom events with additional data
//! - [`EventTarget`]: Interface for objects that can receive events
//!
//! # Event Listener Behavior
//!
//! - Event listeners are called in the order they were registered
//! - The same listener function is only registered once per event type and options
//! - Listeners with different options (e.g. capture) are treated as distinct
//! - Once listeners are automatically removed after being called
//!
//! # Event Object Properties
//!
//! Event objects have the following read-only properties:
//!
//! - `type`: The event type string
//! - `bubbles`: Whether the event bubbles up through the DOM
//! - `cancelable`: Whether the event can be canceled
//! - `composed`: Whether the event will trigger listeners outside of a shadow root
//!
//! CustomEvent objects additionally have:
//!
//! - `detail`: Custom data passed when creating the event

mod custom_event;
mod event;
mod event_target;

pub use custom_event::CustomEvent;
pub use event::Event;
pub use event_target::EventTarget;

use rusty_js::*;

/// Register event-related classes with the JavaScript engine
pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<Event>()?;
    ctx.register_class::<CustomEvent>()?;
    ctx.register_class::<EventTarget>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;

    #[test]
    fn test_event() {
        async_run!(|ctx: JSContext| async move {
            ctx.global().set(
                "print",
                JSFunc::new(&ctx, |msg: String| println!("JS: {}", msg)),
            )?;

            ctx.eval::<()>(Source::from_bytes(
                r#"
                    const console={
                        log: function(...args){
                            print(args.join(' '))
                        }
                    }
                "#,
            ))?;

            init(&ctx)?;

            let source = Source::from_path("tests/event.js").await.unwrap();
            let obj: JSObject = ctx.eval_async(source).await?;

            let total: i32 = obj.get("total")?;
            let passed: i32 = obj.get("passed")?;
            let success: bool = obj.get("success")?;

            if !success {
                let failed: JSArray = obj.get("failed")?;
                let error_messages: Vec<String> = failed.iter().collect::<JSResult<_>>()?;
                panic!(
                    "Path tests failed:\nPassed {}/{}\nFailures:\n{}",
                    passed,
                    total,
                    error_messages.join("\n")
                );
            }
            Ok(())
        });
    }
}

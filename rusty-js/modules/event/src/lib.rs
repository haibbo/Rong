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
mod event_emitter;
mod event_target;

pub use custom_event::CustomEvent;
pub use event::Event;
pub use event_emitter::{Emitter, EmitterExt, EventEmitter, Events};
pub use event_target::EventTarget;

use rusty_js::*;

/// Register event-related classes with the JavaScript engine
pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<EventEmitter>()?;
    EventEmitter::add_node_event_target_prototype(ctx)?;
    EventEmitter::add_web_event_target_prototype(ctx)?;

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
                JSFunc::new(&ctx, |msg: String| println!("{}", msg)),
            )?;

            ctx.eval::<()>(Source::from_bytes(
                r#"
                    const console={
                        log: function(...args){
                            print(args.join(' '))
                        },
                        error: function(...args){
                            print(args.join(' '))
                        }
                    }
                "#,
            ))?;

            init(&ctx)?;
            assert::init(&ctx)?;

            let current_dir = std::env::current_dir().unwrap();

            let runner = current_dir.join("../../tests/unit/test-runner.js");
            let source = Source::from_path(runner).await.unwrap();
            ctx.eval_async::<()>(source).await?;

            let test = current_dir.join("../../tests/unit/event.js");
            let source = Source::from_path(test).await.unwrap();
            ctx.eval_async::<()>(source).await?;

            let result: JSObject = ctx
                .eval_async(Source::from_bytes("runner.report()"))
                .await?;

            let failed: u32 = result.get("failed")?;
            let passed: u32 = result.get("passed")?;

            assert!(
                failed == 0,
                "Path tests passed: {}, failed: {}",
                failed,
                passed
            );
            Ok(())
        });
    }
}

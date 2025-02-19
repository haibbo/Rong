//! Event handling implementation combining Web and Node.js patterns
//!
//! This module provides event handling functionality inspired by both Web APIs and Node.js,
//! with a unified underlying implementation through the `Events` structure.
//!
//! ## Web Standard APIs:
//! - [`Event`]: The base event object
//! - [`CustomEvent`]: For custom events with additional data
//! - [`EventTarget`]: Interface for objects that can receive events
//!
//! ## Node.js APIs:
//! - [`EventEmitter`]: Node.js style event emitter with extended functionality
//! - [`Emitter`] and [`EmitterExt`]: Traits for implementing custom event emitters
//!
//! # Event Handling Patterns
//!
//! ## Web Style (EventTarget):
//! - Event listeners are called in the order they were registered
//! - Events are dispatched through the `dispatchEvent` method
//! - Basic event listener management with `addEventListener` and `removeEventListener`
//! - Note: The capture phase is currently not implemented
//!
//! ## Node.js Style (EventEmitter):
//! - Supports both string and symbol event types
//! - Configurable maximum number of listeners per event (default: 10)
//! - Rich API for event handling:
//!   - `on`/`addListener`: Add a listener
//!   - `once`: Add a one-time listener
//!   - `off`/`removeListener`: Remove a listener
//!   - `removeAllListeners`: Remove all listeners
//!   - `prependListener`: Add listener to the beginning
//!   - `prependOnceListener`: Add one-time listener to the beginning
//!   - `emit`: Trigger an event
//!   - `eventNames`: Get all registered event types
//!   - `setMaxListeners`/`getMaxListeners`: Configure listener limits
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
//!
//! # Implementation Details
//!
//! The module uses a unified `Events` structure internally, which provides:
//! - Thread-safe event listener management through `Mutex`
//! - Support for both string and symbol event types via `EventKey`
//! - Automatic cleanup of one-time listeners
//! - Configurable listener limits per event type
//! - Order-preserving listener execution

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
    EventTarget::add_web_event_target_prototype(ctx)?;
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

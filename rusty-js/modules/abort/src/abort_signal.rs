use dom_exception::{DOMException, DOMExceptionName};
use event::{Emitter, EmitterExt, EventEmitter, EventKey};
use rusty_js::{function::*, *};
use tokio::sync::watch;

// The AbortSignal interface represents a signal object that allows you to communicate
// with an asynchronous operation (such as a fetch request) and abort it if required
// via an AbortController object
#[js_class]
pub struct AbortSignal {
    aborted: bool,

    // The reason why the operation was aborted, which can be any JavaScript value
    reason: Option<JSValue>,

    emitter: EventEmitter,
    sender: watch::Sender<Option<JSValue>>,
}

/// A structure with one sender and multiple receivers
/// Uses watch channel to implement, where the sender can send abort signals and
/// multiple receivers can subscribe and receive the signal
#[derive(Clone, Debug)]
pub struct AbortReceiver {
    inner: watch::Receiver<Option<JSValue>>,
}

impl AbortSignal {
    /// Creates a new receiver
    /// Each receiver subscribes to the same sender, implementing a one-to-many
    /// communication pattern
    #[must_use]
    pub fn subscribe(&self) -> AbortReceiver {
        AbortReceiver {
            inner: self.sender.subscribe(),
        }
    }

    /// Sends an abort signal
    /// If the signal hasn't been sent yet, it will be sent to all subscribed receivers
    /// Uses watch channel to ensure all receivers receive the same signal
    pub fn send(&self, abort: JSValue) -> JSResult<()> {
        if self.sender.borrow().is_none() {
            // Check if there are any active receivers before sending
            if self.sender.receiver_count() > 0 {
                self.sender.send(Some(abort)).into_result()?;
            }
        }
        Ok(())
    }
}

impl AbortReceiver {
    /// Receives an abort signal
    /// Blocks until a signal is received
    pub async fn recv(&mut self) -> JSValue {
        loop {
            if let Some(value) = &*self.inner.borrow() {
                return value.clone();
            }
            self.inner.changed().await.unwrap();
        }
    }
}

#[js_methods]
impl AbortSignal {
    pub fn new() -> Self {
        let (sender, _) = watch::channel(None);
        Self {
            emitter: EventEmitter::new(),
            aborted: false,
            reason: None,
            sender,
        }
    }

    #[js_method(constructor)]
    pub fn constructor() -> JSResult<()> {
        Err(RustyJSError::TypeError(
            "Failed to construct 'AbortSignal': Illegal constructor".to_string(),
        ))
    }

    #[js_method(getter, enumerable, rename = "onabort")]
    pub fn get_on_abort(&self) -> Option<JSFunc> {
        self.emitter.get_listener(&EventKey::from("abort"))
    }

    #[js_method(setter, rename = "onabort")]
    pub fn set_on_abort(&self, this: This<JSObject>, listener: JSFunc) -> JSResult<()> {
        let key = EventKey::from("abort");
        Self::add_event_listener(this, key, listener, false, false)?;
        Ok(())
    }

    #[js_method(getter, enumerable)]
    pub fn aborted(&self) -> bool {
        self.aborted
    }

    #[js_method(getter, enumerable, rename = "reason")]
    pub fn get_reason(&self, ctx: JSContext) -> JSValue {
        self.reason.clone().unwrap_or(JSValue::undefined(&ctx))
    }

    #[js_method(setter, rename = "reason")]
    pub fn set_reason(&mut self, reason: Optional<JSValue>) {
        match reason.0 {
            Some(new_reason) if !new_reason.is_undefined() => self.reason.replace(new_reason),
            _ => self.reason.take(),
        };
    }

    #[js_method(rename = "throwIfAborted")]
    pub fn throw_if_aborted(&self, ctx: JSContext) -> JSValue {
        if self.aborted && self.reason.is_some() {
            // BUGFix: convert to exception
            return self.reason.clone().unwrap();
        }
        JSValue::undefined(&ctx)
    }

    /// static method takes an iterable of abort signals and returns an AbortSignal.
    /// The returned abort signal is aborted when any of the input iterable abort
    /// signals are aborted. The abort reason will be set to the reason of the first
    /// signal that is aborted. If any of the given abort signals are already aborted
    /// then so will be the returned AbortSignal.
    #[js_method]
    pub fn any(ctx: JSContext, signals: JSArray) -> JSResult<JSObject> {
        let mut new_abort = AbortSignal::new();
        let class = Class::get::<AbortSignal>(&ctx).unwrap();
        let mut unaborted_signals = Vec::with_capacity(signals.len() as _);

        for item in signals.iter::<JSObject>() {
            let signal = item?;
            let borrow = signal.borrow_mut::<AbortSignal>()?;

            // If any signal is already aborted, abort the new signal immediately
            if borrow.aborted {
                new_abort.aborted = true;
                new_abort.reason = borrow.reason.clone();
                let new_signal = class.instance::<AbortSignal>(new_abort);
                return Ok(new_signal);
            } else {
                drop(borrow);
                unaborted_signals.push(signal);
            }
        }

        let new_signal = class.instance::<AbortSignal>(new_abort);

        // Set up listeners for all signals
        for signal in unaborted_signals {
            let to_abort = new_signal.clone();
            let ctx_for_closure = ctx.clone();

            let notifier = JSFunc::new_once(&ctx, move |signal: This<JSObject>| {
                let mut borrow = to_abort.borrow_mut::<AbortSignal>()?;
                borrow.aborted = true;
                borrow.reason = signal.borrow::<AbortSignal>()?.reason.clone();
                drop(borrow); // for borrow again
                Self::send_aborted(&ctx_for_closure, This(to_abort))
            })?;
            Self::add_event_listener(This(signal), EventKey::from("abort"), notifier, false, true)?;
        }
        Ok(new_signal)
    }

    /// static method returns an AbortSignal that is already set as aborted, and
    /// which does not trigger an abort event
    #[js_method]
    pub fn abort(ctx: JSContext, reason: Optional<JSValue>) -> JSResult<JSObject> {
        let mut signal = Self::new();
        signal.set_reason(reason);
        let instance = Class::get::<AbortSignal>(&ctx)?.instance(signal);
        Self::send_aborted(&ctx, This(instance.clone()))?;
        Ok(instance)
    }

    /// static method returns an AbortSignal that will automatically abort after a specified time
    /// The signal aborts with a TimeoutError DOMException on timeout.
    /// The "active" time in milliseconds before the returned AbortSignal will abort
    #[js_method]
    pub fn timeout(ctx: JSContext, time: u64) -> JSResult<JSObject> {
        let mut signal = Self::new();
        let timeout_error = get_reason_or_dom_exception(&ctx, None, DOMExceptionName::TIMEOUT_ERR)?;
        signal.reason = Some(timeout_error);

        let instance = Class::get::<AbortSignal>(&ctx)?.instance(signal);

        // Clone necessary values for the async block
        let instance_clone = instance.clone();

        // Spawn a new task that will abort the signal after the timeout
        ctx.spawn_local(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(time)).await;

            let mut borrow = instance_clone.borrow_mut::<AbortSignal>()?;
            borrow.aborted = true;
            drop(borrow);

            let _ = Self::do_emit(This(instance_clone), EventKey::from("abort"), Rest(vec![]));
            Ok(())
        });

        Ok(instance)
    }

    /// send abort signal to this
    pub fn send_aborted(ctx: &JSContext, this: This<JSObject>) -> JSResult<()> {
        let mut borrow = this.borrow_mut::<AbortSignal>()?;
        borrow.aborted = true;
        let reason =
            get_reason_or_dom_exception(ctx, borrow.reason.as_ref(), DOMExceptionName::ABORT_ERR)?;
        borrow.reason = Some(reason.clone());
        borrow.send(reason)?;
        drop(borrow); // drop for do_emit
        Self::do_emit(this, EventKey::from("abort"), Rest(vec![]))?;
        Ok(())
    }
}

impl Default for AbortSignal {
    fn default() -> Self {
        Self::new()
    }
}

fn get_reason_or_dom_exception(
    ctx: &JSContext,
    reason: Option<&JSValue>,
    name: DOMExceptionName,
) -> JSResult<JSValue> {
    let reason = if let Some(reason) = reason {
        reason.clone()
    } else {
        DOMException::create(ctx, "", name)?.into_jsvalue()
    };
    Ok(reason)
}

impl Emitter for AbortSignal {
    fn get_event_emitter(&self) -> &EventEmitter {
        &self.emitter
    }

    fn get_mut_event_emitter(&mut self) -> &mut EventEmitter {
        &mut self.emitter
    }
}

use dom_exception::{DOMException, DOMExceptionName};
use event::{Emitter, EmitterExt, EventEmitter, EventKey};
use rusty_js::{function::*, *};
use std::rc::Rc;
use std::sync::Mutex;
use tokio::sync::watch;

// The AbortSignal interface represents a signal object that allows you to communicate
// with an asynchronous operation (such as a fetch request) and abort it if required
// via an AbortController object
#[js_export]
pub struct AbortSignal {
    inner: Rc<Mutex<AbortSignalInner>>,
}

struct AbortSignalInner {
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
    pub fn new() -> Self {
        let (sender, _) = watch::channel(None);
        Self {
            inner: Rc::new(Mutex::new(AbortSignalInner {
                emitter: EventEmitter::new(),
                aborted: false,
                reason: None,
                sender,
            })),
        }
    }

    /// Creates a new receiver
    /// Each receiver subscribes to the same sender, implementing a one-to-many
    /// communication pattern
    #[must_use]
    pub fn subscribe(&self) -> AbortReceiver {
        let inner = self.inner.lock().unwrap();
        AbortReceiver {
            inner: inner.sender.subscribe(),
        }
    }

    /// Sends an abort signal
    /// If the signal hasn't been sent yet, it will be sent to all subscribed receivers
    /// Uses watch channel to ensure all receivers receive the same signal
    pub fn notify_abort(&self, abort: JSValue) -> JSResult<()> {
        let inner = self.inner.lock().unwrap();
        if inner.sender.borrow().is_none() {
            // Check if there are any active receivers before sending
            if inner.sender.receiver_count() > 0 {
                inner.sender.send(Some(abort)).into_result()?;
            }
        }
        Ok(())
    }

    fn get_inner_emitter(&self) -> &EventEmitter {
        unsafe { &*(&self.inner.lock().unwrap().emitter as *const EventEmitter) }
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

#[js_class]
impl AbortSignal {
    #[js_method(constructor)]
    fn constructor() -> JSResult<()> {
        Err(RustyJSError::TypeError(
            "Failed to construct 'AbortSignal': Illegal constructor".to_string(),
        ))
    }

    #[js_method(getter, enumerable, rename = "onabort")]
    fn get_on_abort(&self) -> Option<JSFunc> {
        let inner = self.inner.lock().ok()?;
        inner.emitter.get_listener(&EventKey::from("abort"))
    }

    #[js_method(setter, rename = "onabort")]
    fn set_on_abort(&self, this: This<JSObject>, listener: JSFunc) -> JSResult<()> {
        let key = EventKey::from("abort");
        Self::add_event_listener(this, key, listener, false, false)?;
        Ok(())
    }

    #[js_method(getter, enumerable)]
    pub fn aborted(&self) -> bool {
        self.inner.lock().unwrap().aborted
    }

    #[js_method(getter, enumerable, rename = "reason")]
    pub(crate) fn get_reason(&self, ctx: JSContext) -> JSValue {
        let inner = self.inner.lock().unwrap();
        inner.reason.clone().unwrap_or(JSValue::undefined(&ctx))
    }

    #[js_method(setter, rename = "reason")]
    pub(crate) fn set_reason(&self, reason: Optional<JSValue>) {
        let mut inner = self.inner.lock().unwrap();
        match reason.0 {
            Some(new_reason) if !new_reason.is_undefined() => inner.reason.replace(new_reason),
            _ => inner.reason.take(),
        };
    }

    #[js_method(rename = "throwIfAborted")]
    fn throw_if_aborted(&self, ctx: JSContext) -> JSValue {
        let inner = self.inner.lock().unwrap();
        if inner.aborted && inner.reason.is_some() {
            ctx.throw(inner.reason.clone().unwrap())
        } else {
            JSValue::undefined(&ctx)
        }
    }

    /// static method takes an iterable of abort signals and returns an AbortSignal.
    /// The returned abort signal is aborted when any of the input iterable abort
    /// signals are aborted. The abort reason will be set to the reason of the first
    /// signal that is aborted. If any of the given abort signals are already aborted
    /// then so will be the returned AbortSignal.
    #[js_method]
    fn any(ctx: JSContext, signals: JSArray) -> JSResult<JSObject> {
        let new_signal = AbortSignal::new();
        let class = Class::get::<AbortSignal>(&ctx)?;
        let mut unaborted_signals = Vec::with_capacity(signals.len() as _);

        for item in signals.iter::<JSObject>() {
            let signal = item?;
            let borrow = signal.borrow_mut::<AbortSignal>()?;

            if borrow.aborted() {
                {
                    let mut inner = new_signal.inner.lock().unwrap();
                    inner.aborted = true;
                    inner.reason = Some(borrow.get_reason(ctx.clone()));
                }
                let new_signal = class.instance::<AbortSignal>(new_signal);
                return Ok(new_signal);
            } else {
                drop(borrow);
                unaborted_signals.push(signal);
            }
        }

        let new_signal = class.instance::<AbortSignal>(new_signal);

        for signal in unaborted_signals {
            let to_abort = new_signal.clone();
            let ctx_for_closure = ctx.clone();

            let notifier = JSFunc::new_once(&ctx, move |signal: This<JSObject>| -> JSResult<()> {
                let signal_obj = signal.borrow::<AbortSignal>()?;
                let reason = signal_obj.get_reason(ctx_for_closure.clone());
                drop(signal_obj);

                let to_abort_obj = to_abort.borrow_mut::<AbortSignal>()?;
                {
                    let mut inner = to_abort_obj.inner.lock().unwrap();
                    inner.aborted = true;
                    inner.reason = Some(reason);
                }
                drop(to_abort_obj);

                Self::broadcast_abort(&ctx_for_closure, This(to_abort))
            })?;
            Self::add_event_listener(This(signal), EventKey::from("abort"), notifier, false, true)?;
        }
        Ok(new_signal)
    }

    /// static method returns an AbortSignal that is already set as aborted, and
    /// which does not trigger an abort event
    #[js_method]
    fn abort(reason: Optional<JSValue>) -> JSResult<AbortSignal> {
        let signal = Self::new();
        signal.set_reason(reason);
        {
            let mut inner = signal.inner.lock().into_result()?;
            inner.aborted = true;
        }
        Ok(signal)
    }

    /// static method returns an AbortSignal that will automatically abort after a specified time
    /// The signal aborts with a TimeoutError DOMException on timeout.
    /// The "active" time in milliseconds before the returned AbortSignal will abort
    #[js_method]
    pub fn timeout(ctx: JSContext, time: u64) -> JSResult<JSObject> {
        let signal = Self::new();
        let timeout_error = get_reason_or_dom_exception(&ctx, None, DOMExceptionName::TIMEOUT_ERR)?;
        {
            let mut inner = signal.inner.lock().unwrap();
            inner.reason = Some(timeout_error);
        }

        let instance = Class::get::<AbortSignal>(&ctx)?.instance(signal);
        let instance_clone = instance.clone();

        ctx.spawn_local(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(time)).await;

            let signal = instance_clone.borrow_mut::<AbortSignal>()?;
            {
                let mut inner = signal.inner.lock().unwrap();
                inner.aborted = true;
            }
            drop(signal);

            let _ = Self::do_emit(This(instance_clone), EventKey::from("abort"), Rest(vec![]));
            Ok(())
        });

        Ok(instance)
    }

    /// send abort signal to this
    pub(crate) fn broadcast_abort(ctx: &JSContext, this: This<JSObject>) -> JSResult<()> {
        let borrow = this.borrow_mut::<AbortSignal>()?;
        let reason = {
            let mut inner = borrow.inner.lock().into_result()?;
            inner.aborted = true;
            let reason = get_reason_or_dom_exception(
                ctx,
                inner.reason.as_ref(),
                DOMExceptionName::ABORT_ERR,
            )?;
            inner.reason = Some(reason.clone());
            reason
        };
        borrow.notify_abort(reason)?;
        drop(borrow);
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
        self.get_inner_emitter()
    }
}

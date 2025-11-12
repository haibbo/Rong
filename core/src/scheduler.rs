use crate::{
    JSContext, JSContextImpl, JSFunc, JSObject, JSObjectOps, JSResult, JSRuntimeService,
    JSValueImpl, RongJSError,
};
use futures::Future;
use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::Arc,
};
use tokio::sync::{Mutex, mpsc, oneshot};

/// Runtime-level gate: hard serialization at engine entrance.
#[derive(Clone, Default)]
pub struct JsInvokeGate(pub Arc<Mutex<()>>);
impl JSRuntimeService for JsInvokeGate {}

/// Soft scheduler with priority and event coalescing, per JSRuntime.
#[derive(Clone)]
pub struct JsInvokeQueue {
    tx: mpsc::Sender<QueueItem>,
}

impl Default for JsInvokeQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl JsInvokeQueue {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel::<QueueItem>(1024);

        crate::rong::spawn(async move {
            let mut q_high: VecDeque<QueueItem> = VecDeque::new();
            let mut q_norm: VecDeque<QueueItem> = VecDeque::new();
            let mut q_event: VecDeque<QueueItem> = VecDeque::new();
            let mut event_gen: HashMap<String, u64> = HashMap::new();
            let mut next_gen: u64 = 1;

            loop {
                tokio::select! {
                    Some(mut item) = rx.recv() => {
                        match item.priority {
                            JsInvokePriority::High => q_high.push_back(item),
                            JsInvokePriority::Normal => q_norm.push_back(item),
                            JsInvokePriority::Event => {
                                if let Some(key) = item.dedup_key.clone() {
                                    let r#gen = next_gen; next_gen += 1;
                                    event_gen.insert(key, r#gen);
                                    item.generation = r#gen;
                                }
                                q_event.push_back(item);
                            }
                        }
                    }
                    else => break,
                }

                // Drain one item per tick: High -> Normal -> Event (last-wins for events)
                let mut next = q_high.pop_front().or_else(|| q_norm.pop_front());
                if next.is_none() {
                    while let Some(ev) = q_event.pop_front() {
                        if let Some(key) = &ev.dedup_key {
                            if let Some(&latest) = event_gen.get(key) {
                                if ev.generation != latest {
                                    continue;
                                }
                            }
                        }
                        next = Some(ev);
                        break;
                    }
                }

                if let Some(item) = next {
                    let fut = (item.cb)();
                    let res = fut.await;
                    if let Some(tx) = item.reply {
                        let _ = tx.send(res);
                    }
                }
            }
        });

        Self { tx }
    }
}
impl JSRuntimeService for JsInvokeQueue {}

/// Invocation priority
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsInvokePriority {
    High,
    Normal,
    Event,
}

type InvokeFuture = Pin<Box<dyn Future<Output = JSResult<()>> + 'static>>;
type InvokeFn = Box<dyn FnOnce() -> InvokeFuture + 'static>;

struct QueueItem {
    priority: JsInvokePriority,
    dedup_key: Option<String>,
    generation: u64,
    cb: InvokeFn,
    reply: Option<oneshot::Sender<JSResult<()>>>,
}

/// Enqueue a JS function invocation with priority/coalescing.
pub async fn enqueue_js_invoke<C>(
    ctx: &JSContext<C>,
    func: JSFunc<C::Value>,
    this_obj: Option<JSObject<C::Value>>,
    args_obj: Option<JSObject<C::Value>>,
    priority: JsInvokePriority,
    dedup_key: Option<String>,
    must_reply: bool,
) -> JSResult<()>
where
    C: JSContextImpl + 'static,
    C::Value: JSValueImpl + JSObjectOps + 'static,
    C::Runtime: 'static,
{
    // Get runtime-level services via context user_data (bound at context creation)
    let queue = ctx.runtime().get_or_init_service::<JsInvokeQueue>().clone();

    let (reply_tx, reply_rx) = if must_reply {
        let (tx, rx) = oneshot::channel();
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    let ctx_clone = ctx.clone();
    let cb: InvokeFn = Box::new(move || {
        Box::pin(
            async move { js_invoke_async::<C, ()>(&ctx_clone, func, this_obj, args_obj).await },
        )
    });

    let item = QueueItem {
        priority,
        dedup_key,
        generation: 0,
        cb,
        reply: reply_tx,
    };
    queue
        .tx
        .send(item)
        .await
        .map_err(|_| RongJSError::Error("scheduler queue closed".into()))?;

    if let Some(rx) = reply_rx {
        rx.await
            .unwrap_or_else(|_| Err(RongJSError::Error("scheduler reply dropped".into())))
    } else {
        Ok(())
    }
}

/// Dispatch a JS invocation immediately with hard gate (async form).
pub async fn js_invoke_async<C, R>(
    ctx: &JSContext<C>,
    func: JSFunc<C::Value>,
    this_obj: Option<JSObject<C::Value>>,
    args_obj: Option<JSObject<C::Value>>,
) -> JSResult<R>
where
    C: JSContextImpl,
    C::Value: JSValueImpl + JSObjectOps + 'static,
    C::Runtime: 'static,
    R: crate::FromJSValue<C::Value> + 'static,
{
    // Acquire runtime-level gate (shared across all contexts in this runtime)
    let gate = ctx.runtime().get_or_init_service::<JsInvokeGate>().clone();
    let _guard = gate.0.lock().await;

    match args_obj {
        Some(obj) => func
            .call_async::<_, R>(this_obj, (obj,))
            .await
            .map_err(|e| RongJSError::Error(e.to_string())),
        None => func
            .call_async::<_, R>(this_obj, ())
            .await
            .map_err(|e| RongJSError::Error(e.to_string())),
    }
}

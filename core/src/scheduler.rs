use crate::{
    HostError, JSContext, JSContextImpl, JSFunc, JSObject, JSObjectOps, JSResult, JSRuntimeService,
    JSValueImpl,
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

            const DRAIN_BATCH_LIMIT: usize = 64;

            loop {
                // Phase 1: Drain available incoming items (non-blocking, capped).
                // Cap prevents starvation of Phase 2 under sustained high throughput.
                for _ in 0..DRAIN_BATCH_LIMIT {
                    match rx.try_recv() {
                        Ok(item) => Self::enqueue_item(
                            item,
                            &mut q_high,
                            &mut q_norm,
                            &mut q_event,
                            &mut event_gen,
                            &mut next_gen,
                        ),
                        Err(mpsc::error::TryRecvError::Empty) => break,
                        Err(mpsc::error::TryRecvError::Disconnected) => return,
                    }
                }

                // Phase 2: Process one item from priority queues.
                if let Some(item) =
                    Self::dequeue_item(&mut q_high, &mut q_norm, &mut q_event, &event_gen)
                {
                    // Clean up the generation entry now that the event is consumed.
                    if let Some(key) = &item.dedup_key {
                        if let Some(&latest) = event_gen.get(key) {
                            if latest == item.generation {
                                event_gen.remove(key);
                            }
                        }
                    }

                    let fut = (item.cb)();
                    let res = fut.await;
                    if let Some(tx) = item.reply {
                        let _ = tx.send(res);
                    }
                    // After processing, loop back to drain more incoming + process more.
                    continue;
                }

                // Phase 3: Nothing queued — wait for the next incoming item.
                match rx.recv().await {
                    Some(item) => Self::enqueue_item(
                        item,
                        &mut q_high,
                        &mut q_norm,
                        &mut q_event,
                        &mut event_gen,
                        &mut next_gen,
                    ),
                    None => break, // Channel closed.
                }
            }
        });

        Self { tx }
    }

    /// Route an item into the correct priority queue.
    fn enqueue_item(
        mut item: QueueItem,
        q_high: &mut VecDeque<QueueItem>,
        q_norm: &mut VecDeque<QueueItem>,
        q_event: &mut VecDeque<QueueItem>,
        event_gen: &mut HashMap<String, u64>,
        next_gen: &mut u64,
    ) {
        match item.priority {
            JsInvokePriority::High => q_high.push_back(item),
            JsInvokePriority::Normal => q_norm.push_back(item),
            JsInvokePriority::Event => {
                if let Some(key) = item.dedup_key.clone() {
                    let generation = *next_gen;
                    *next_gen += 1;
                    event_gen.insert(key, generation);
                    item.generation = generation;
                }
                q_event.push_back(item);
            }
        }
    }

    /// Pop the highest-priority ready item. Events use last-wins dedup.
    fn dequeue_item(
        q_high: &mut VecDeque<QueueItem>,
        q_norm: &mut VecDeque<QueueItem>,
        q_event: &mut VecDeque<QueueItem>,
        event_gen: &HashMap<String, u64>,
    ) -> Option<QueueItem> {
        if let item @ Some(_) = q_high.pop_front() {
            return item;
        }
        if let item @ Some(_) = q_norm.pop_front() {
            return item;
        }
        // For events, skip stale entries (superseded by newer same-key events).
        while let Some(ev) = q_event.pop_front() {
            if let Some(key) = &ev.dedup_key {
                if let Some(&latest) = event_gen.get(key) {
                    if ev.generation != latest {
                        continue; // Stale — skip.
                    }
                }
            }
            return Some(ev);
        }
        None
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
        .map_err(|_| HostError::new(crate::error::E_INTERNAL, "scheduler queue closed"))?;

    if let Some(rx) = reply_rx {
        rx.await.unwrap_or_else(|_| {
            Err(HostError::new(crate::error::E_INTERNAL, "scheduler reply dropped").into())
        })
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
    let gate = ctx.runtime().get_or_init_service::<JsInvokeGate>().clone();
    let _guard = gate.0.lock().await;

    match args_obj {
        Some(obj) => func.call_async::<_, R>(this_obj, (obj,)).await,
        None => func.call_async::<_, R>(this_obj, ()).await,
    }
}

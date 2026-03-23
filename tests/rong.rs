use rong::{
    JSFunc, JSObject, JSResult, JsInvokePriority, Rong, RongJS, Source, WorkerMessage,
    enqueue_js_invoke,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::test]
async fn test_block_on_simple() -> JSResult<()> {
    let rong = Rong::<RongJS>::builder().build();
    let result = rong.block_on(|_runtime, _receiver| async {
        let value = 10 + 20;
        Ok(value)
    })?;
    println!("[Test Main] Result: {:?}", result);
    assert_eq!(result, 30);
    Ok(())
}

#[tokio::test]
async fn test_post_usize_message() -> JSResult<()> {
    let rong = Rong::<RongJS>::builder().build();
    let worker = rong.get_worker().await?;
    let worker_clone = worker.clone();

    // Use Arc<Mutex<Option<usize>>> to get the result back from the spawned future
    let received_value = Arc::new(Mutex::new(None::<usize>));
    let received_value_clone = received_value.clone();

    worker_clone.spawn_future::<_, _, ()>(async move |_runtime, mut receiver| {
        println!("[Test Worker] Waiting for message...");
        if let Some(msg) = receiver.recv().await {
            match msg {
                WorkerMessage::Usize(val) => {
                    println!("[Test Worker] Received usize: {}", val);
                    let mut guard = received_value_clone.lock().unwrap();
                    *guard = Some(val); // Store the received value
                }
                _ => {
                    println!("[Test Worker] Received unexpected message type");
                }
            }
        } else {
            println!("[Test Worker] Channel closed unexpectedly");
        }
        Ok(())
    })?;

    // Give the worker thread a moment to start and listen
    tokio::time::sleep(Duration::from_millis(50)).await;

    let sent_value = 123;
    println!("[Test Main] Posting usize: {}", sent_value);
    worker.post_message(WorkerMessage::Usize(sent_value))?;

    rong.join_all().await?; // Waits for all workers to become free

    // Check if the value was received
    let final_value = *received_value.lock().unwrap();
    assert_eq!(final_value, Some(sent_value));

    Ok(())
}

#[tokio::test]
async fn test_post_string_message() -> JSResult<()> {
    let rong = Rong::<RongJS>::builder().build();
    let worker = rong.get_worker().await?;
    let worker_clone = worker.clone();
    let received_value = Arc::new(Mutex::new(None::<String>));
    let received_value_clone = received_value.clone();

    worker_clone.spawn_future::<_, _, ()>(async move |_runtime, mut receiver| {
        if let Some(WorkerMessage::String(val)) = receiver.recv().await {
            println!("[Test Worker] Received string: {}", val);
            *received_value_clone.lock().unwrap() = Some(val);
        }
        Ok(())
    })?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    let sent_value = "hello worker".to_string();
    println!("[Test Main] Posting string: {}", sent_value);
    worker.post_message(WorkerMessage::String(sent_value.clone()))?;

    rong.join_all().await?;

    let final_value = received_value.lock().unwrap().clone();
    assert_eq!(final_value, Some(sent_value));

    Ok(())
}

#[tokio::test]
async fn test_post_custom_message() -> JSResult<()> {
    #[derive(Debug, PartialEq, Clone)]
    struct MyCustomData {
        id: i32,
        name: String,
    }

    let rong = Rong::<RongJS>::builder().build();
    let worker = rong.get_worker().await?;
    let worker_clone = worker.clone();
    let received_value = Arc::new(Mutex::new(None::<MyCustomData>));
    let received_value_clone = received_value.clone();

    worker_clone.spawn_future::<_, _, ()>(async move |_runtime, mut receiver| {
        if let Some(WorkerMessage::Custom(val_box)) = receiver.recv().await {
            if let Ok(downcasted_val) = val_box.downcast::<MyCustomData>() {
                println!("[Test Worker] Received custom: {:?}", *downcasted_val);
                *received_value_clone.lock().unwrap() = Some(*downcasted_val);
            } else {
                println!("[Test Worker] Failed to downcast custom message");
            }
        }
        Ok(())
    })?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    let sent_value = MyCustomData {
        id: 99,
        name: "test data".to_string(),
    };
    println!("[Test Main] Posting custom: {:?}", sent_value);
    worker.post_message(WorkerMessage::Custom(Box::new(sent_value.clone())))?;

    rong.join_all().await?;

    let final_value = received_value.lock().unwrap().clone();
    assert_eq!(final_value, Some(sent_value));

    Ok(())
}

#[tokio::test]
async fn test_worker_termination() -> JSResult<()> {
    let rong = Rong::<RongJS>::builder().build();
    let worker = rong.get_worker().await?;
    let worker_clone = worker.clone();

    // Flag to check if the task started execution
    let task_started = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let task_started_clone = task_started.clone();

    println!(
        "[Test Main] Spawning sleeping task on Worker {}",
        worker_clone.id()
    );
    let worker_id = worker_clone.id(); // Get ID before the closure
    worker_clone.spawn_future::<_, _, ()>(async move |_runtime, _receiver| {
        println!("[Test Worker {}] Task started, sleeping...", worker_id); // Use captured ID
        task_started_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        tokio::time::sleep(Duration::from_secs(5)).await;
        // This part should ideally not be reached if termination works correctly
        println!(
            "[Test Worker {}] Task finished sleeping (should have been terminated)",
            worker_id
        );
        Ok(())
    })?;

    // Wait until the task has started inside the worker
    println!("[Test Main] Waiting for task to start...");
    while !task_started.load(std::sync::atomic::Ordering::SeqCst) {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    println!("[Test Main] Task confirmed started.");

    // Give it a tiny bit more time just in case, then terminate
    tokio::time::sleep(Duration::from_millis(50)).await;
    println!("[Test Main] Terminating Worker {}", worker.id());
    worker.terminate()?;

    // Wait for the worker pool to become idle (should happen quickly after termination)
    // Use a timeout to prevent the test from hanging indefinitely if termination fails
    println!("[Test Main] Waiting for join_all()...");
    match tokio::time::timeout(Duration::from_secs(2), rong.join_all()).await {
        Ok(Ok(_)) => {
            println!("[Test Main] join_all() completed successfully after termination.");
            // Check worker state after join_all confirms it's free
            // assert_eq!(worker.state().await, rong::WorkerState::Free);
            Ok(())
        }
        Ok(Err(e)) => {
            panic!("[Test Main] join_all() failed after termination: {:?}", e);
        }
        Err(_) => {
            panic!(
                "[Test Main] Timed out waiting for join_all() after termination. Worker might not have terminated correctly."
            );
        }
    }
}

#[tokio::test]
async fn test_enqueue_js_invoke_queue() -> JSResult<()> {
    let rong = Rong::<RongJS>::builder().build();
    rong.block_on(|runtime, _receiver| async move {
        let ctx = runtime.context();
        let script = r#"(() => {
            globalThis.__invoke_counter = 0;
            return function () {
                globalThis.__invoke_counter += 1;
                return globalThis.__invoke_counter;
            };
        })()"#;
        let js_fn: JSFunc = ctx.eval(Source::from_bytes(script))?;

        enqueue_js_invoke(
            &ctx,
            js_fn.clone(),
            None,
            None,
            JsInvokePriority::Normal,
            None,
            true,
        )
        .await?;
        enqueue_js_invoke(
            &ctx,
            js_fn.clone(),
            None,
            None,
            JsInvokePriority::High,
            None,
            true,
        )
        .await?;
        enqueue_js_invoke(
            &ctx,
            js_fn,
            None,
            None,
            JsInvokePriority::Normal,
            None,
            true,
        )
        .await?;

        let final_value: i32 = ctx.global().get("__invoke_counter")?;
        assert_eq!(final_value, 3);

        Ok(())
    })?;
    Ok(())
}

#[tokio::test]
async fn test_get_worker_wait_eventually_returns_free_worker() -> JSResult<()> {
    let rong = Rong::<RongJS>::builder().with_num_workers(1).build();

    for _ in 0..32 {
        let worker = rong.get_worker().await?;
        worker.spawn_future::<_, _, ()>(async move |_runtime, _receiver| {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(())
        })?;

        assert!(
            rong.get_worker().await.is_err(),
            "worker should still be busy"
        );

        let waited_worker = tokio::time::timeout(Duration::from_secs(1), rong.get_worker_wait())
            .await
            .expect("get_worker_wait timed out")?;
        assert_eq!(waited_worker.id(), worker.id());

        waited_worker.spawn_future::<_, _, ()>(async move |_runtime, _receiver| Ok(()))?;
        rong.join_all().await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_block_on_reentrant_inside_worker_returns_error() -> JSResult<()> {
    let rong = Rong::<RongJS>::builder().with_num_workers(1).build();
    let rong_clone = rong.clone();

    let nested_error = Arc::new(Mutex::new(None::<String>));
    let nested_error_clone = nested_error.clone();

    rong.block_on(move |_runtime, _receiver| async move {
        let err = rong_clone
            .block_on(|_runtime, _receiver| async move { Ok::<_, rong::RongJSError>(123) })
            .expect_err("reentrant block_on should fail");
        *nested_error_clone.lock().unwrap() = Some(err.to_string());
        Ok(())
    })?;

    let message = nested_error.lock().unwrap().clone().unwrap_or_default();
    assert!(
        message.contains("reentrant block_on") || message.contains("inside a worker thread"),
        "unexpected reentrant error: {message}"
    );

    Ok(())
}

#[tokio::test]
async fn test_scheduler_makes_progress_under_burst_enqueue() -> JSResult<()> {
    let rong = Rong::<RongJS>::builder().build();
    let observed = Arc::new(AtomicUsize::new(0));
    let observed_clone = observed.clone();

    rong.block_on(|runtime, _receiver| async move {
        let ctx = runtime.context();
        let script = r#"(() => {
            globalThis.__burst_counter = 0;
            return function () {
                globalThis.__burst_counter += 1;
                return globalThis.__burst_counter;
            };
        })()"#;
        let js_fn: JSFunc = ctx.eval(Source::from_bytes(script))?;

        for _ in 0..256 {
            enqueue_js_invoke(
                &ctx,
                js_fn.clone(),
                None,
                None,
                JsInvokePriority::Normal,
                None,
                false,
            )
            .await?;
        }

        tokio::time::timeout(
            Duration::from_secs(1),
            enqueue_js_invoke(
                &ctx,
                js_fn.clone(),
                None,
                None,
                JsInvokePriority::High,
                None,
                true,
            ),
        )
        .await
        .expect("high-priority invoke timed out")?;

        let value: i32 = ctx.global().get("__burst_counter")?;
        observed_clone.store(value as usize, Ordering::SeqCst);
        Ok(())
    })?;

    assert!(
        observed.load(Ordering::SeqCst) > 0,
        "scheduler failed to make progress during burst enqueue"
    );

    Ok(())
}

#[tokio::test]
async fn test_scheduler_does_not_block_on_pending_js_promise() -> JSResult<()> {
    let rong = Rong::<RongJS>::builder().build();
    let started = Arc::new(AtomicUsize::new(0));
    let observed = Arc::new(AtomicUsize::new(0));
    let started_clone = started.clone();
    let observed_clone = observed.clone();

    rong.block_on(|runtime, _receiver| async move {
        let ctx = runtime.context();

        ctx.global().set(
            "sleepMs",
            JSFunc::new(&ctx, |ms: i32| async move {
                tokio::time::sleep(Duration::from_millis(ms as u64)).await;
            })?,
        )?;

        let slow: JSFunc = ctx.eval(Source::from_bytes(
            r#"(async function () {
                globalThis.__scheduler_started = 1;
                await sleepMs(200);
            })"#,
        ))?;

        let fast = JSFunc::new(&ctx, move || {
            observed_clone.store(1, Ordering::SeqCst);
        })?;

        enqueue_js_invoke(
            &ctx,
            slow,
            None,
            None,
            JsInvokePriority::Normal,
            None,
            false,
        )
        .await?;

        tokio::time::timeout(Duration::from_millis(100), async {
            while started_clone.load(Ordering::SeqCst) == 0 {
                if let Ok(started_flag) = ctx.global().get::<_, i32>("__scheduler_started")
                    && started_flag == 1
                {
                    started.store(1, Ordering::SeqCst);
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("slow invoke did not start");

        tokio::time::timeout(
            Duration::from_millis(50),
            enqueue_js_invoke(&ctx, fast, None, None, JsInvokePriority::High, None, true),
        )
        .await
        .expect("high-priority invoke timed out")?;

        Ok(())
    })?;

    assert_eq!(observed.load(Ordering::SeqCst), 1);

    Ok(())
}

#[tokio::test]
async fn test_scheduler_event_async_handler_keeps_last_wins_ordering() -> JSResult<()> {
    let rong = Rong::<RongJS>::builder().build();

    rong.block_on(|runtime, _receiver| async move {
        let ctx = runtime.context();

        ctx.global().set(
            "sleepMs",
            JSFunc::new(&ctx, |ms: i32| async move {
                tokio::time::sleep(Duration::from_millis(ms as u64)).await;
            })?,
        )?;

        ctx.eval::<()>(Source::from_bytes(r#"globalThis.__event_value = "unset";"#))?;

        let handler: JSFunc = ctx.eval(Source::from_bytes(
            r#"(async function (event) {
                await sleepMs(event.delay);
                globalThis.__event_value = event.label;
            })"#,
        ))?;

        let old_event = JSObject::new(&ctx);
        old_event.set("label", "old")?;
        old_event.set("delay", 75)?;

        let new_event = JSObject::new(&ctx);
        new_event.set("label", "new")?;
        new_event.set("delay", 0)?;

        enqueue_js_invoke(
            &ctx,
            handler.clone(),
            None,
            Some(old_event),
            JsInvokePriority::Event,
            Some("view:update".to_string()),
            false,
        )
        .await?;

        enqueue_js_invoke(
            &ctx,
            handler,
            None,
            Some(new_event),
            JsInvokePriority::Event,
            Some("view:update".to_string()),
            true,
        )
        .await?;

        tokio::time::sleep(Duration::from_millis(120)).await;

        let final_value: String = ctx.global().get("__event_value")?;
        assert_eq!(final_value, "new");

        Ok(())
    })?;

    Ok(())
}

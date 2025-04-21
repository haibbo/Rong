use rong::{JSResult, Rong, RongJS, RongJSError, Source, WorkerMessage};
use std::error::Error;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let num_workers = 4;
    let rong = Rong::<RongJS>::builder()
        .with_num_workers(num_workers)
        .build();
    println!("Rong instance created with {} workers.", num_workers);

    let mut workers = Vec::with_capacity(num_workers);
    for _ in 0..num_workers {
        match rong.get_worker().await {
            Ok(w) => workers.push(w),
            Err(e) => {
                eprintln!("Failed to acquire all workers: {:?}", e);
                return Ok(());
            }
        }
    }
    println!("Acquired all {} workers.", workers.len());

    // Designate Worker 0 for setInterval, others for expressions
    let interval_worker = workers[0].clone();
    let interval_worker_id = interval_worker.id();
    println!(
        "Designating Worker {} for the interval task.",
        interval_worker_id
    );

    interval_worker.spawn_future(async move |runtime, _receiver| -> JSResult<()> {
        println!("[Worker {} Interval Task] Started.", interval_worker_id);
        let ctx = runtime.context();
        // Optional: Initialize modules if needed
        let _ = rong_modules::init(&ctx).map_err(|e| {
            eprintln!(
                "[Worker {} Interval Task] Failed to init rong_modules (ignoring): {:?}",
                interval_worker_id, e
            );
            // Don't return error, just log
        });

        let js_code = format!(
            r#"
                console.log("[Worker {} JS] Setting up interval...");
                setInterval(() => {{ console.log("[Worker {} JS] Interval Tick!"); }}, 500);
                console.log("[Worker {} JS] Interval setup complete.");
            "#,
            interval_worker_id, interval_worker_id, interval_worker_id
        );

        ctx.eval::<()>(Source::from_bytes(js_code.as_bytes()))
            .map_err(|e| RongJSError::Error(format!("JS Eval failed: {:?}", e)))?;

        println!(
            "[Worker {} Interval Task] JS interval evaluated, Rust task sleeping...",
            interval_worker_id
        );
        tokio::time::sleep(Duration::from_secs(60)).await; // Keep worker alive for interval
        println!(
            "[Worker {} Interval Task] Rust task finished sleep (unexpected!).",
            interval_worker_id
        );
        Ok(())
    })?;
    println!(
        "[Main] Interval task spawned on Worker {}.",
        interval_worker_id
    );

    // Spawn expression evaluation tasks on other workers
    for worker in workers.iter().skip(1) {
        let worker = worker.clone();
        let worker_id = worker.id();
        worker.spawn_future(async move |runtime, mut receiver| -> JSResult<()> {
            println!("[Worker {} Expr Task] Waiting for expression...", worker_id);
            let ctx = runtime.context();

            if let Some(WorkerMessage::String(expr_str)) = receiver.recv().await {
                println!(
                    "[Worker {} Expr Task] Received expression: '{}'",
                    worker_id, expr_str
                );
                match ctx.eval::<i32>(Source::from_bytes(expr_str.as_bytes())) {
                    Ok(result) => {
                        println!(
                            "[Worker {} Expr Task] Evaluated '{}' = {}",
                            worker_id, expr_str, result
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "[Worker {} Expr Task] Failed to evaluate expression '{}': {:?}",
                            worker_id, expr_str, e
                        );
                        return Err(RongJSError::Error(format!("Eval failed: {:?}", e)));
                    }
                }
            } else {
                println!(
                    "[Worker {} Expr Task] Channel closed unexpectedly",
                    worker_id
                );
            }
            println!("[Worker {} Expr Task] Finished.", worker_id);
            Ok(())
        })?;
        println!("[Main] Expression task spawned on Worker {}.", worker_id);
    }

    // Give tasks time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send expressions ONLY to workers 1, 2, 3
    println!("[Main] Sending expressions to workers 1..{}", workers.len());
    for (i, worker) in workers.iter().enumerate().skip(1) {
        let expr = format!("{}+{}", i * 2 + 1, i * 2 + 2);
        println!(
            "[Main] Sending expression '{}' to Worker {}",
            expr,
            worker.id()
        );
        if let Err(e) = worker.post_message(WorkerMessage::String(expr)) {
            eprintln!(
                "[Main] Failed to send message to Worker {}: {:?}",
                worker.id(),
                e
            );
        }
    }

    println!(
        "[Main] Waiting to observe interval logs from Worker {}...",
        interval_worker_id
    );
    tokio::time::sleep(Duration::from_secs(3)).await;

    println!(
        "[Main] Terminating ONLY Worker {} (interval task)...",
        interval_worker_id
    );
    interval_worker.terminate()?;

    println!("[Main] Waiting for ALL workers termination/idle (join_all)...",);
    rong.join_all().await?;
    println!("[Main] All workers idle. Example complete.");

    Ok(())
}

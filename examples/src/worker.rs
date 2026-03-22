use rong::{JSResult, Rong, RongJS, Source, WorkerMessage};
use std::error::Error;
use std::time::Duration;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let num_workers = 4;
    let rong = Rong::<RongJS>::builder()
        .with_num_workers(num_workers)
        .build();
    info!(num_workers, "Rong instance created");

    let mut workers = Vec::with_capacity(num_workers);
    for _ in 0..num_workers {
        match rong.get_worker().await {
            Ok(w) => workers.push(w),
            Err(e) => {
                error!(error = ?e, "Failed to acquire all workers");
                return Ok(());
            }
        }
    }
    info!(count = workers.len(), "Acquired all workers");

    // Designate Worker 0 for setInterval, others for expressions
    let interval_worker = workers[0].clone();
    let interval_worker_id = interval_worker.id();
    info!(
        worker_id = interval_worker_id,
        "Designating worker for interval task"
    );

    interval_worker.spawn_future(async move |runtime, _receiver| -> JSResult<()> {
        info!(worker_id = interval_worker_id, "Interval task started");
        let ctx = runtime.context();
        // Optional: Initialize modules if needed
        let _ = rong_modules::init(&ctx).map_err(|e| {
            warn!(worker_id = interval_worker_id, error = ?e, "Failed to init rong_modules (ignoring)");
        });

        let js_code = format!(
            r#"
                console.log("[Worker {} JS] Setting up interval...");
                setInterval(() => {{ console.log("[Worker {} JS] Interval Tick!"); }}, 500);
                console.log("[Worker {} JS] Interval setup complete.");
            "#,
            interval_worker_id, interval_worker_id, interval_worker_id
        );

        ctx.eval::<()>(Source::from_bytes(js_code.as_bytes()))?;

        info!(worker_id = interval_worker_id, "JS interval evaluated, sleeping");
        tokio::time::sleep(Duration::from_secs(60)).await;
        info!(worker_id = interval_worker_id, "Interval task finished sleep (unexpected)");
        Ok(())
    })?;
    info!(worker_id = interval_worker_id, "Interval task spawned");

    // Spawn expression evaluation tasks on other workers
    for worker in workers.iter().skip(1) {
        let worker = worker.clone();
        let worker_id = worker.id();
        worker.spawn_future(async move |runtime, mut receiver| -> JSResult<()> {
            info!(worker_id, "Expr task waiting for expression");
            let ctx = runtime.context();

            if let Some(WorkerMessage::String(expr_str)) = receiver.recv().await {
                info!(worker_id, expr = %expr_str, "Received expression");
                match ctx.eval::<i32>(Source::from_bytes(expr_str.as_bytes())) {
                    Ok(result) => {
                        info!(worker_id, expr = %expr_str, result, "Evaluated expression");
                    }
                    Err(e) => {
                        error!(worker_id, expr = %expr_str, error = ?e, "Failed to evaluate expression");
                        return Err(e);
                    }
                }
            } else {
                warn!(worker_id, "Channel closed unexpectedly");
            }
            info!(worker_id, "Expr task finished");
            Ok(())
        })?;
        info!(worker_id = worker.id(), "Expression task spawned");
    }

    // Give tasks time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send expressions ONLY to workers 1, 2, 3
    info!(count = workers.len() - 1, "Sending expressions to workers");
    for (i, worker) in workers.iter().enumerate().skip(1) {
        let expr = format!("{}+{}", i * 2 + 1, i * 2 + 2);
        info!(worker_id = worker.id(), expr = %expr, "Sending expression");
        if let Err(e) = worker.post_message(WorkerMessage::String(expr)) {
            error!(worker_id = worker.id(), error = ?e, "Failed to send message");
        }
    }

    info!(
        worker_id = interval_worker_id,
        "Waiting to observe interval logs"
    );
    tokio::time::sleep(Duration::from_secs(3)).await;

    info!(
        worker_id = interval_worker_id,
        "Terminating interval worker"
    );
    interval_worker.terminate()?;

    info!("Waiting for all workers termination/idle");
    rong.join_all().await?;
    info!("All workers idle. Example complete.");

    Ok(())
}

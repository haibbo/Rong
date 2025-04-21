use rong::{JSResult, Rong, RongJS, RongJSError, Source, WorkerMessage};
use std::error::Error;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let rong = Rong::<RongJS>::builder().with_num_workers(4).build();
    let worker_count = rong.total_workers_count().await;
    println!("Rong instance created with {} workers.", worker_count);

    let mut workers = Vec::new();
    for i in 0..worker_count {
        match rong.get_worker().await {
            Ok(worker) => {
                workers.push(worker);
            }
            Err(e) => {
                eprintln!("Failed to acquire worker {}: {:?}. Adjusting count.", i, e);
                break;
            }
        }
    }

    if workers.len() != worker_count {
        eprintln!(
            "Could not acquire all expected workers. Acquired {}, expected {}. Exiting.",
            workers.len(),
            worker_count
        );
        return Ok(());
    }
    println!("All workers acquired and tasks spawned.");

    for worker in &workers {
        let worker_for_task = worker.clone();
        let worker_id = worker.id();

        worker_for_task.spawn_future(async move |runtime, mut receiver| -> JSResult<()> {
            let ctx = runtime.context();

            if let Some(msg) = receiver.recv().await {
                match msg {
                    WorkerMessage::String(expr_str) => {
                        match ctx.eval::<i32>(Source::from_bytes(expr_str.as_bytes())) {
                            Ok(result) => {
                                println!(
                                    "[Worker {}] Evaluated '{}' = {}",
                                    worker_id, expr_str, result
                                );
                            }
                            Err(e) => {
                                eprintln!(
                                    "[Worker {}] Failed to evaluate expression '{}': {:?}",
                                    worker_id, expr_str, e
                                );
                                return Err(RongJSError::Error(format!("Eval failed: {:?}", e)));
                            }
                        }
                    }
                    _ => {
                        println!("[Worker {}] Received unexpected message type", worker_id);
                    }
                }
            } else {
                println!(
                    "[Worker {}] Channel closed unexpectedly before receiving message",
                    worker_id
                );
            }
            Ok(())
        })?;
    }

    tokio::time::sleep(Duration::from_millis(50)).await;

    println!("Sending expressions to workers...");
    for (i, worker) in workers.iter().enumerate() {
        let expr = format!("{}+{}", i * 2 + 1, i * 2 + 2);
        println!(
            "[Main] Sending expression '{}' to Worker {}",
            expr,
            worker.id()
        );
        match worker.post_message(WorkerMessage::String(expr)) {
            Ok(_) => {}
            Err(e) => {
                eprintln!(
                    "[Main] Failed to send message to Worker {}: {:?}",
                    worker.id(),
                    e
                );
            }
        }
    }

    println!("Waiting for all workers to complete tasks...");
    rong.join_all().await?;
    println!("All workers finished. Example complete.");

    Ok(())
}

use rong::{Rong, RongExecutor, RongJS, Source};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let executor = RongExecutor::builder()
        .threads(2)
        .thread_name("rong-example-host")
        .build()?;
    executor.clone().install_global()?;

    let host_task = RongExecutor::global().spawn(async { "host work complete" });

    let rong = Rong::<RongJS>::builder().shared().workers(2).build()?;

    let js_result: i32 = rong
        .call(|runtime, _receiver| async move {
            let ctx = runtime.context();
            ctx.eval(Source::from_bytes(b"21 * 2"))
        })
        .await?;

    let host_message = host_task.await?;

    println!("JS result: {js_result}");
    println!("Executor result: {host_message}");
    Ok(())
}

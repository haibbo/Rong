use rusty_js::*;
use std::time::Duration;
use tokio::time::sleep;

fn main() {
    let rt = RustyJS::runtime();
    let ctx = RustyJS::context(&rt);

    ctx.global().set(
        "print",
        JSFunc::new(&ctx, |msg: String| println!("{}", msg)),
    );
    timer::init(&ctx).unwrap();

    rt.block_on(async move {
        let current_dir = std::env::current_dir().unwrap();
        let js_path = current_dir.join("examples/timer_script.js");
        println!("Looking for JS file at: {}", js_path.display());

        ctx.eval::<()>(Source::from_path(js_path).await.unwrap())
            .unwrap();

        println!("Timers set up. Waiting for 5 seconds...");
        sleep(Duration::from_millis(5500)).await;

        println!("Program ending...");
        Ok(())
    })
    .unwrap();
}

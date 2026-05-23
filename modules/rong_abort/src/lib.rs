mod abort_controller;
mod abort_signal;

use rong::*;
use rong_event::EmitterExt;

pub use abort_controller::AbortController;
pub use abort_signal::{AbortReceiver, AbortSignal};

pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<AbortSignal>()?;
    AbortSignal::add_web_event_target_prototype(ctx)?;

    ctx.register_class::<AbortController>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;
    use tokio::sync::mpsc;

    #[test]
    fn test_abort_receiver() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx)?;

            ctx.global().set(
                "print",
                JSFunc::new(&ctx, |msg: String| println!("{}", msg)),
            )?;

            let (tx, mut rx) = mpsc::channel(1);

            let callback = JSFunc::new(&ctx, move |obj: JSObject| {
                let abort = obj.borrow::<AbortSignal>().unwrap();
                let mut recv = abort.subscribe();

                let tx = tx.clone();
                spawn_local(async move {
                    let value = recv.recv().await;
                    let reason: String = value.to_rust().unwrap();
                    println!("Got reason:{}", reason);
                    tx.send(reason).await.unwrap();
                });
            });

            ctx.global().set("rust", callback)?;

            ctx.eval::<()>(Source::from_bytes(
                r#"
                    let controller=new AbortController();

                    // js abort listener
                    controller.signal.onabort=() => {
                        print("JS: Got Abort Signal");
                    };

                    // add rust receiver
                    rust(controller.signal);

                    controller.abort('Aborted');
                "#,
            ))?;

            let reason = rx.recv().await.unwrap();
            assert_eq!(reason, "Aborted");

            Ok(())
        });
    }

    #[test]
    fn test_abort() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx)?;
            rong_assert::init(&ctx)?;
            rong_event::init(&ctx)?;
            rong_exception::init(&ctx)?;
            rong_timer::init(&ctx)?;
            rong_console::init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "abort.js")
                .await?
                .run()
                .await?;
            assert!(passed);
            Ok(())
        });
    }

    // Regression: AbortSignal.timeout used to set `inner.aborted = true`
    // directly and emit only the JS `abort` event, leaving Rust subscribers
    // on the internal watch channel permanently pending. Verify Rust-side
    // `subscribe()` receivers wake when the timeout fires.
    #[test]
    fn test_timeout_notifies_rust_subscribers() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx)?;
            rong_exception::init(&ctx)?;

            let (tx, mut rx) = mpsc::channel(1);

            let callback = JSFunc::new(&ctx, move |obj: JSObject| {
                let abort = obj.borrow::<AbortSignal>().unwrap();
                let mut recv = abort.subscribe();
                let tx = tx.clone();
                spawn_local(async move {
                    let _ = recv.recv().await;
                    tx.send(()).await.unwrap();
                });
            });

            ctx.global().set("rust", callback)?;
            ctx.eval::<()>(Source::from_bytes(
                r#"
                    const signal = AbortSignal.timeout(50);
                    rust(signal);
                "#,
            ))?;

            tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
                .await
                .expect("rust subscriber should receive timeout abort within 2s")
                .expect("forwarder should not drop sender before notifying");

            Ok(())
        });
    }
}

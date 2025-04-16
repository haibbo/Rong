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
                let ctx = obj.get_ctx();

                let tx = tx.clone();
                ctx.spawn_local(async move {
                    let value = recv.recv().await;
                    let reason: String = value.try_into()?;
                    println!("Got reason:{}", reason);
                    tx.send(reason).await.unwrap();
                    Ok(())
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
}

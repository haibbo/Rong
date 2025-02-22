mod abort_controller;
mod abort_signal;

use event::EmitterExt;
use rusty_js::*;

pub use abort_controller::AbortController;
pub use abort_signal::AbortSignal;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<AbortSignal>()?;
    AbortSignal::add_web_event_target_prototype(ctx)?;

    ctx.register_class::<AbortController>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;
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
            ctx.global().set(
                "print",
                JSFunc::new(&ctx, |msg: String| println!("{}", msg)),
            )?;

            ctx.eval::<()>(Source::from_bytes(
                r#"
                    const console={
                        log: function(...args){
                            print(args.join(' '))
                        },
                        error: function(...args){
                            print(args.join(' '))
                        }
                    }
                "#,
            ))?;

            init(&ctx)?;
            assert::init(&ctx)?;
            event::init(&ctx)?;
            dom_exception::init(&ctx)?;

            let current_dir = std::env::current_dir().unwrap();

            let runner = current_dir.join("../../tests/unit/test-runner.js");
            let source = Source::from_path(runner).await.unwrap();
            ctx.eval_async::<()>(source).await?;

            let test = current_dir.join("../../tests/unit/abort.js");
            let source = Source::from_path(test).await.unwrap();
            ctx.eval_async::<()>(source).await?;

            let result: JSObject = ctx
                .eval_async(Source::from_bytes("runner.report()"))
                .await?;

            let failed: u32 = result.get("failed")?;
            let passed: u32 = result.get("passed")?;

            assert!(
                failed == 0,
                "Path tests passed: {}, failed: {}",
                failed,
                passed
            );
            Ok(())
        });
    }
}

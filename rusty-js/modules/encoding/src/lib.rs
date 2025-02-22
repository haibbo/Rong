use rusty_js::*;

mod text_decoder;
mod text_encoder;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    text_encoder::init(ctx)?;
    text_decoder::init(ctx)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;

    #[test]
    fn test_encoding() {
        async_run!(|ctx: JSContext| async move {
            ctx.global().set(
                "print",
                JSFunc::new(&ctx, |msg: String| println!("JS: {}", msg)),
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
            init(&ctx).unwrap();

            let passed = UnitJSRunner::load_script(&ctx, "encoding.js")
                .await?
                .run()
                .await?;
            assert!(passed);
            Ok(())
        });
    }
}

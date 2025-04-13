use rusty_js::*;
use std::env;
use std::process;

fn exit(status: u32) {
    process::exit(status as i32);
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let rong = ctx.rong();

    let args = env::args().skip(2).collect::<Vec<String>>();
    rong.set("args", args)?;

    let exit = JSFunc::new(ctx, exit)?.name("exit")?;
    rong.set("exit", exit)?;

    Ok(())
}

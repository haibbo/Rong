use rusty_js::*;
use std::env;
use std::process;

fn exit(status: u32) {
    process::exit(status as i32);
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let danity = ctx.dainty();

    let args = env::args().skip(2).collect::<Vec<String>>();
    danity.set("args", args)?;

    let exit = JSFunc::new(ctx, exit)?.name("exit")?;
    danity.set("exit", exit)?;

    Ok(())
}

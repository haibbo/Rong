use rong::*;
use std::env;
use std::process;

fn exit(status: u32) {
    process::exit(status as i32);
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let rong = ctx.host_namespace();

    let args = env::args().collect::<Vec<String>>();
    let rong_args = match args.get(1).map(|s| s.as_str()) {
        Some("compile") => args.into_iter().skip(2).collect::<Vec<String>>(),
        _ => args.into_iter().skip(1).collect::<Vec<String>>(),
    };
    rong.set("args", rong_args)?;

    let exit = JSFunc::new(ctx, exit)?.name("exit")?;
    rong.set("exit", exit)?;

    Ok(())
}

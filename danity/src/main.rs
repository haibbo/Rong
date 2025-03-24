use rusty_js::*;
use std::env;
use std::path::PathBuf;

mod danity;
mod repl;

#[derive(Debug)]
enum Command {
    Run(PathBuf),
    Compile { input: PathBuf, output: PathBuf },
    Help,
    Version,
}

fn usage() {
    println!(
        r#"
Usage:
  dainty run <file.js>
  dainty compile <input.js> <output.danity>
  dainty -v | --version
  dainty -h | --help

Commands:
  run         Execute a JavaScript file
  compile     Compile JavaScript to bytecode

Options:
  -v, --version     Print version information
  -h, --help        Print this help message
"#
    );
}

fn parse_args() -> Result<Command, String> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Ok(Command::Help);
    }

    match args[1].as_str() {
        "-h" | "--help" => Ok(Command::Help),
        "-v" | "--version" => Ok(Command::Version),
        "run" => {
            if args.len() < 3 {
                return Err("Missing file argument for run command".to_string());
            }
            Ok(Command::Run(args[2].clone().into()))
        }
        "compile" => {
            if args.len() < 4 {
                return Err("Missing input/output arguments for compile command".to_string());
            }
            Ok(Command::Compile {
                input: args[2].clone().into(),
                output: args[3].clone().into(),
            })
        }
        _ => Err(format!("Unknown command: {}", args[1])),
    }
}

async fn run_file(ctx: &JSContext, path: PathBuf) -> Result<(), RustyJSError> {
    let source = Source::from_path(ctx, &path).await?;
    ctx.eval_async::<()>(source).await
}

async fn compile_file(
    ctx: &JSContext,
    input: PathBuf,
    output: PathBuf,
) -> Result<(), RustyJSError> {
    // 1. Load the source file
    let source = Source::from_path(ctx, &input).await?;

    // 2. Check source kind and get JavaScript code
    let js_code = match source.kind() {
        SourceKind::JavaScript(code) => code,
        SourceKind::ByteCode(_) => {
            return Err(RustyJSError::Error(
                "Cannot compile already compiled bytecode".to_string(),
            ));
        }
    };

    // 3. Compile to bytecode
    let bytecode_source = ctx.compile_to_bytecode(js_code)?;

    // 4. Save the bytecode
    bytecode_source.save_bytecode(ctx, output).await
}

fn main() -> Result<(), RustyJSError> {
    let command = match parse_args() {
        Ok(cmd) => cmd,
        Err(err) => {
            eprintln!("Error: {}", err);
            usage();
            return Ok(());
        }
    };

    let rt = RustyJS::runtime();
    let ctx = RustyJS::context(&rt);

    // Initialize all modules
    danity_modules::init(&ctx)?;
    danity::init(&ctx)?;

    match command {
        Command::Run(path) => rt.block_on(async move { run_file(&ctx, path).await })?,
        Command::Compile { input, output } => {
            rt.block_on(async move { compile_file(&ctx, input, output).await })?
        }
        Command::Help => {
            usage();
        }
        Command::Version => {
            println!("dainty v0.1.0");
        }
    }

    Ok(())
}

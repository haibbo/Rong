use rong::*;
use std::env;
use std::path::PathBuf;

mod extension;
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
  rong run <file.js>
  rong compile <input.js> <output.rong>
  rong -v | --version
  rong -h | --help

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

async fn run_file(ctx: &JSContext, path: PathBuf) -> Result<(), RongJSError> {
    let source = Source::from_path(ctx, &path).await?;
    ctx.eval_async::<()>(source).await
}

async fn compile_file(ctx: &JSContext, input: PathBuf, output: PathBuf) -> Result<(), RongJSError> {
    // 1. Load the source file
    let source = Source::from_path(ctx, &input).await?;

    // 2. Check source kind and get JavaScript code
    let js_code = match source.kind() {
        SourceKind::JavaScript(code) => code,
        SourceKind::ByteCode(_) => {
            return Err(HostError::new(
                rong::error::E_INVALID_ARG,
                "Cannot compile already compiled bytecode",
            )
            .with_name("TypeError")
            .into());
        }
    };

    // 3. Compile to bytecode
    let bytecode_source = ctx.compile_to_bytecode(js_code)?;

    // 4. Save the bytecode
    bytecode_source.save_bytecode(ctx, output).await
}

fn main() -> Result<(), RongJSError> {
    let command = match parse_args() {
        Ok(cmd) => cmd,
        Err(err) => {
            eprintln!("Error: {}", err);
            usage();
            return Ok(());
        }
    };

    // Create a single Rong instance for all commands
    if matches!(command, Command::Help | Command::Version) {
        // No need to create a Rong instance for these simple commands
        match command {
            Command::Help => {
                usage();
            }
            Command::Version => {
                println!("rong v0.1.0");
            }
            _ => unreachable!(),
        }
    } else {
        // For commands that need JS execution, use a single Rong instance
        Rong::<RongJS>::builder()
            .build()
            .block_on(async |runtime, _receiver| {
                let ctx = runtime.context();
                // Initialize all modules
                rong_modules::init(&ctx)?;
                extension::init(&ctx)?;

                // Process the command with the initialized context
                match command {
                    Command::Run(path) => run_file(&ctx, path).await,
                    Command::Compile { input, output } => compile_file(&ctx, input, output).await,
                    _ => unreachable!(), // Help and Version already handled
                }
            })?
    }

    Ok(())
}

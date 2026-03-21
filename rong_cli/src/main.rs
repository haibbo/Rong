use rong::*;
use std::env;
use std::path::PathBuf;

mod completer;
mod extension;
mod repl;

#[derive(Debug)]
enum Command {
    Run(PathBuf),
    Compile { input: PathBuf, output: PathBuf },
    Repl,
    Help,
    Version,
}

fn usage() {
    println!(
        r#"
Usage:
  rong                      Start REPL (or run stdin when piped)
  rong <file.js|file.rong>
  rong compile <input.js> <output.rong>
  rong -v | --version
  rong -h | --help

Commands:
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
        return Ok(Command::Repl);
    }

    match args[1].as_str() {
        "-h" | "--help" => Ok(Command::Help),
        "-v" | "--version" => Ok(Command::Version),
        "compile" => {
            if args.len() < 4 {
                return Err("Missing input/output arguments for compile command".to_string());
            }
            Ok(Command::Compile {
                input: args[2].clone().into(),
                output: args[3].clone().into(),
            })
        }
        _ => {
            // If the argument looks like a file path (ends with .js or .rong), execute it.
            let arg = &args[1];
            if arg.ends_with(".js") || arg.ends_with(".rong") {
                Ok(Command::Run(arg.clone().into()))
            } else {
                Err(format!("Unknown command: {}", args[1]))
            }
        }
    }
}

async fn run_file(ctx: &JSContext, path: PathBuf) -> Result<(), RongJSError> {
    let source = Source::from_path(ctx, &path).await?;

    // For JavaScript files, wrap top-level await in async IIFE
    match source.kind() {
        rong::SourceKind::JavaScript(code) => {
            let code_str = String::from_utf8_lossy(code);
            let wrapped = if code_str.contains("await ")
                && !code_str.contains("async function")
                && !code_str.contains("async (")
            {
                format!("(async () => {{ {} }})();", code_str)
            } else {
                code_str.to_string()
            };
            ctx.eval_async::<()>(Source::from_bytes(wrapped)).await
        }
        _ => ctx.eval_async::<()>(source).await,
    }
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
                println!("rong v{}", env!("CARGO_PKG_VERSION"));
            }
            _ => unreachable!(),
        }
    } else {
        // Give workers the same module set as the main context
        rong_modules::worker::set_initializer(|ctx| rong_modules::init(ctx));

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
                    Command::Repl => repl::run(&ctx).await,
                    Command::Run(path) => run_file(&ctx, path).await,
                    Command::Compile { input, output } => compile_file(&ctx, input, output).await,
                    _ => unreachable!(), // Help and Version already handled
                }
            })?
    }

    Ok(())
}

use rusty_js::*;
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use ureq::Agent;

mod danity;
mod repl;

#[derive(Debug)]
enum SourceLocation {
    File(PathBuf),
    Url(String),
}

fn usage() {
    println!(
        r#"
Usage:
  dainty <file_or_url>
  dainty -v | --version
  dainty -h | --help

Options:
  -v, --version     Print version information
  -h, --help        Print this help message

Arguments:
  file_or_url       Path to a local JavaScript file or a URL to fetch from
"#
    );
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

async fn load_source(location: SourceLocation) -> Result<Source, RustyJSError> {
    match location {
        SourceLocation::File(path) => Source::from_path(&path).await.map_err(|e| {
            RustyJSError::TypeError(format!("Failed to read file '{}': {}", path.display(), e))
        }),
        SourceLocation::Url(url) => {
            let config = Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(5)))
                .build();

            let agent: Agent = config.into();
            let bytes: Vec<u8> = agent
                .get(&url)
                .call()
                .into_result()?
                .body_mut()
                .read_to_vec()
                .into_result()?;

            // Create source with URL as name
            Ok(Source::from_bytes(bytes).with_name(url))
        }
    }
}

fn main() -> Result<(), RustyJSError> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        usage();
        return Ok(());
    }

    match args[1].as_str() {
        "-h" | "--help" => {
            usage();
            return Ok(());
        }
        "-v" | "--version" => {
            println!("dainty v0.1.0");
            return Ok(());
        }
        _ => {}
    }

    let source_location = if is_url(&args[1]) {
        SourceLocation::Url(args[1].clone())
    } else {
        SourceLocation::File(args[1].clone().into())
    };

    let rt = RustyJS::runtime();
    let ctx = RustyJS::context(&rt);

    // Initialize all modules
    danity_modules::init(&ctx)?;

    // Initialize CLI-specific functionality
    danity::init(&ctx)?;

    rt.block_on(async move {
        let source = load_source(source_location).await?;
        ctx.eval_async::<()>(source)
            .await
            .map_err(|e| RustyJSError::TypeError(format!("Failed to execute JavaScript: {}", e)))
    })?;

    Ok(())
}

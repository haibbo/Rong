use crate::completer::{self, ReplHelper};
use rong::*;
use rustyline::Editor;
use rustyline::error::ReadlineError;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

fn print_help() {
    println!(
        r#"
Rong REPL commands:
  .help               Show this help
  .exit / .quit       Exit the REPL
  .clear              Clear the screen
  .load <file.js>     Load and execute a JavaScript file in the current context

Notes:
  - Top-level `await` is supported (promise results are awaited).
  - The last evaluation result is available as global `_`.
  - Global `let/const` redeclarations behave like normal JS (may throw on re-define).
  - Use Ctrl+D to exit, Ctrl+C to cancel current input.
  - Arrow keys, backspace, and history (up/down arrows) are fully supported.
"#
    );
}

fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

fn is_complete_js(input: &str) -> bool {
    // Minimal heuristics for multiline entry:
    // - Track (), {}, [] balance
    // - Track string literals ('", `) with escaping
    let mut paren = 0i32;
    let mut brace = 0i32;
    let mut bracket = 0i32;

    let mut in_single = false;
    let mut in_double = false;
    let mut in_template = false;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_single || in_double || in_template {
            match ch {
                '\\' => escaped = true,
                '\'' if in_single => in_single = false,
                '"' if in_double => in_double = false,
                '`' if in_template => in_template = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '\'' => in_single = true,
            '"' => in_double = true,
            '`' => in_template = true,
            '(' => paren += 1,
            ')' => paren -= 1,
            '{' => brace += 1,
            '}' => brace -= 1,
            '[' => bracket += 1,
            ']' => bracket -= 1,
            _ => {}
        }
    }

    let balanced = paren <= 0 && brace <= 0 && bracket <= 0;
    balanced && !in_single && !in_double && !in_template
}

fn render_error(ctx: &JSContext, err: RongJSError) -> String {
    let fallback = err.to_string();
    let js_value = err.into_catch_value::<JSEngineValue>(ctx);
    let rendered = rong_console::inspect_value(js_value);
    if rendered.is_empty() {
        fallback
    } else {
        rendered
    }
}

/// Check if code contains top-level await
fn has_top_level_await(code: &str) -> bool {
    code.contains("await ") || code.contains("for await")
}

/// Extract simple variable names from let/const/var declarations
fn extract_declarations(code: &str) -> Vec<String> {
    let mut vars = Vec::new();
    // Simple pattern: let/const/var followed by identifier
    // This won't handle destructuring, but covers common REPL usage
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }
        for keyword in &["async function ", "function ", "class "] {
            if let Some(rest) = trimmed.strip_prefix(keyword) {
                let ident: String = rest
                    .trim_start()
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
                    .collect();
                if !ident.is_empty() {
                    vars.push(ident);
                }
            }
        }
        for keyword in &["let ", "const ", "var "] {
            if let Some(rest) = trimmed.strip_prefix(keyword) {
                // Handle simple comma-separated declarations: let a = 1, b = 2;
                for part in rest.split(',') {
                    let ident: String = part
                        .trim_start()
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
                        .collect();
                    if !ident.is_empty() {
                        vars.push(ident);
                    }
                }
            }
        }
    }
    vars
}

fn wrap_top_level_await(code: &str) -> String {
    if !has_top_level_await(code) {
        return code.to_string();
    }

    let vars = extract_declarations(code);
    if vars.is_empty() {
        return format!("(async () => {{ {} }})();", code);
    }

    let mut expose = String::new();
    for v in &vars {
        expose.push_str("globalThis.");
        expose.push_str(v);
        expose.push_str(" = ");
        expose.push_str(v);
        expose.push_str("; ");
    }
    format!("(async () => {{ {} {} }})();", code, expose)
}

async fn eval_and_print(ctx: &JSContext, code: &str) -> JSResult<()> {
    let wrapped_code = wrap_top_level_await(code);

    let value = ctx
        .eval_async::<JSValue>(Source::from_bytes(wrapped_code.as_bytes()))
        .await?;

    ctx.global().set("_", value.clone())?;

    if value.is_undefined() {
        return Ok(());
    }

    let rendered = rong_console::inspect_value(value);
    println!("{}", rendered);
    Ok(())
}

async fn load_file(ctx: &JSContext, path: &PathBuf) -> JSResult<()> {
    let source = Source::from_path(ctx, path).await?;
    let value = match source.kind() {
        rong::SourceKind::JavaScript(code) => {
            let code_str = String::from_utf8_lossy(code);
            let wrapped = wrap_top_level_await(&code_str);
            ctx.eval_async::<JSValue>(Source::from_bytes(wrapped))
                .await?
        }
        _ => ctx.eval_async::<JSValue>(source).await?,
    };
    ctx.global().set("_", value.clone())?;
    if !value.is_undefined() {
        println!("{}", rong_console::inspect_value(value));
    }
    Ok(())
}

fn try_run_stdin_noninteractive(ctx: &JSContext) -> JSResult<Option<String>> {
    let _ = ctx;
    if io::stdin().is_terminal() {
        return Ok(None);
    }

    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).map_err(|e| {
        let err = HostError::new(rong::error::E_IO, format!("failed reading stdin: {e}"));
        RongJSError::from(err)
    })?;
    if buf.trim().is_empty() {
        return Ok(Some(String::new()));
    }
    Ok(Some(buf))
}

pub async fn run(ctx: &JSContext) -> JSResult<()> {
    // Handle piped input
    if let Some(stdin_code) = try_run_stdin_noninteractive(ctx)? {
        if stdin_code.trim().is_empty() {
            return Ok(());
        }
        return eval_and_print(ctx, &stdin_code).await;
    }

    println!("Welcome to Rong v{}.", env!("CARGO_PKG_VERSION"));
    println!("Type \".help\" for more information.");

    // Set up completion state
    let completion_state = completer::new_completion_state();
    completer::update_completions(ctx, &completion_state);

    let helper = ReplHelper::new(ctx.clone(), completion_state.clone());
    let mut rl = Editor::new().map_err(|e| {
        HostError::new(
            rong::error::E_IO,
            format!("failed to initialize editor: {e}"),
        )
    })?;
    rl.set_helper(Some(helper));

    // Load history from home directory
    let history_path = dirs::home_dir().map(|mut p| {
        p.push(".rong_history");
        p
    });

    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    let mut buf = String::new();

    loop {
        let prompt = if buf.is_empty() { "rong> " } else { "...> " };

        let readline = rl.readline(prompt);

        match readline {
            Ok(line) => {
                let trimmed = line.trim_end_matches(&['\r', '\n'] as &[char]);

                // Handle REPL commands only at the start of input
                if buf.is_empty() {
                    let cmd = trimmed.trim();
                    if cmd.starts_with('.') {
                        let mut parts = cmd.split_whitespace();
                        let name = parts.next().unwrap_or("");
                        match name {
                            ".help" => {
                                print_help();
                                continue;
                            }
                            ".exit" | ".quit" => {
                                break;
                            }
                            ".clear" => {
                                clear_screen();
                                continue;
                            }
                            ".load" => {
                                if let Some(path) = parts.next() {
                                    if let Err(e) = load_file(ctx, &PathBuf::from(path)).await {
                                        eprintln!("Uncaught {}", render_error(ctx, e));
                                    }
                                } else {
                                    eprintln!("Usage: .load <file.js>");
                                }
                                // Refresh completions in case the file added globals.
                                completer::update_completions(ctx, &completion_state);
                                continue;
                            }
                            "." => {
                                continue;
                            }
                            _ => {
                                eprintln!("Unknown REPL command: {name} (try `.help`)");
                                continue;
                            }
                        }
                    }
                }

                buf.push_str(trimmed);
                buf.push('\n');

                // Check if the input is complete
                if !is_complete_js(&buf) {
                    continue;
                }

                let code = buf.trim().to_string();
                buf.clear();

                if code.is_empty() {
                    continue;
                }

                // Add to history
                rl.add_history_entry(&code).ok();

                // Evaluate and print
                if let Err(e) = eval_and_print(ctx, &code).await {
                    eprintln!("Uncaught {}", render_error(ctx, e));
                }

                // Update completions with any new globals
                completer::update_completions(ctx, &completion_state);
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C
                println!("^C");
                buf.clear();
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl+D
                println!();
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    // Save history
    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_top_level_await_exposes_simple_declarations() {
        let wrapped = wrap_top_level_await(
            "const client = 1;\nasync function boot() {}\nclass Demo {}\nawait Promise.resolve();",
        );
        assert!(wrapped.contains("globalThis.client"));
        assert!(wrapped.contains("globalThis.boot"));
        assert!(wrapped.contains("globalThis.Demo"));
    }
}

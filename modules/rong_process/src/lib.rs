//! Process module - Node.js compatible global `process` object
//!
//! This module provides a Node.js compatible `process` global object with:
//! - Environment information (platform, arch, version, pid)
//! - Environment variables (process.env)
//! - Command line arguments (process.argv)
//! - Working directory operations (cwd, chdir)
//! - Process control (exit, uptime, hrtime)
//! - Event loop control (nextTick)
//! - Standard I/O (stdin as ReadableStream, stdout/stderr with write method)
//! - EventEmitter interface (on, emit, off, etc.)

use rong::{
    HostError, IntoJSValue, JSArray, JSContext, JSFunc, JSObject, JSResult, JSValue, Source,
    function::{Optional, Rest},
    js_class, js_export, js_method,
};
use rong_event::{Emitter, EmitterExt, EventEmitter};
use rong_stream::JSReadableStream;
use std::env;
use std::sync::OnceLock;
use std::time::Instant;

/// Global start time for uptime calculation
static START_TIME: OnceLock<Instant> = OnceLock::new();

/// Get the start time, initializing if needed
fn get_start_time() -> &'static Instant {
    START_TIME.get_or_init(Instant::now)
}

/// Get the platform string (Node.js compatible)
fn get_platform() -> &'static str {
    match env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

/// Process class implementing Node.js compatible process object
#[js_export]
pub struct Process {
    events: EventEmitter,
}

impl Process {
    pub fn new() -> Self {
        // Ensure start time is initialized
        let _ = get_start_time();
        Self {
            events: EventEmitter::new(),
        }
    }
}

#[js_class]
impl Process {
    #[js_method(constructor)]
    pub fn constructor() -> JSResult<Self> {
        rong::illegal_constructor("Process cannot be constructed directly. Use globalThis.process.")
    }

    // Static properties as getters

    #[js_method(getter)]
    pub fn platform(&self) -> &'static str {
        get_platform()
    }

    #[js_method(getter)]
    pub fn arch(&self) -> &'static str {
        env::consts::ARCH
    }

    #[js_method(getter)]
    pub fn version(&self) -> &'static str {
        "v1.0.0" // Starfire version
    }

    #[js_method(getter)]
    pub fn pid(&self) -> u32 {
        std::process::id()
    }

    #[js_method(getter)]
    pub fn argv(&self, ctx: JSContext) -> JSResult<JSValue> {
        let args: Vec<String> = env::args().collect();
        let array = JSArray::new(&ctx)?;
        for arg in args {
            array.push(arg)?;
        }
        Ok(array.into_js_value(&ctx))
    }

    #[js_method]
    pub fn cwd(&self) -> JSResult<String> {
        env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .map_err(|e| {
                HostError::new(rong::error::E_IO, format!("Failed to get cwd: {}", e)).into()
            })
    }

    #[js_method]
    pub fn chdir(&self, path: String) -> JSResult<()> {
        env::set_current_dir(&path).map_err(|e| {
            HostError::new(
                rong::error::E_IO,
                format!("Failed to chdir to '{}': {}", path, e),
            )
            .into()
        })
    }

    #[js_method]
    pub fn exit(&self, code: Optional<i32>) {
        let exit_code = code.unwrap_or(0);
        std::process::exit(exit_code);
    }

    #[js_method]
    pub fn uptime(&self) -> f64 {
        get_start_time().elapsed().as_secs_f64()
    }

    /// Returns high-resolution real time.
    /// - Without argument: returns [seconds, nanoseconds] since arbitrary time in the past
    /// - With argument [prev_sec, prev_nano]: returns difference from that time
    #[js_method]
    pub fn hrtime(&self, ctx: JSContext, prev: Optional<JSValue>) -> JSResult<JSValue> {
        let elapsed = get_start_time().elapsed();
        let secs = elapsed.as_secs();
        let nanos = elapsed.subsec_nanos();

        let (result_secs, result_nanos) = if let Some(prev_val) = prev.0 {
            // Calculate difference from previous hrtime
            if let Some(arr) = prev_val.into_object().and_then(JSArray::from_object) {
                let prev_secs = arr.get_opt::<u64>(0)?.unwrap_or(0);
                let prev_nanos = arr.get_opt::<u32>(1)?.unwrap_or(0);

                // Calculate difference, handling nanosecond underflow
                let total_nanos = secs as i128 * 1_000_000_000 + nanos as i128;
                let prev_total = prev_secs as i128 * 1_000_000_000 + prev_nanos as i128;
                let diff = total_nanos - prev_total;

                if diff <= 0 {
                    (0, 0)
                } else {
                    let diff_secs = (diff / 1_000_000_000) as u64;
                    let diff_nanos = (diff % 1_000_000_000) as u32;
                    (diff_secs, diff_nanos)
                }
            } else {
                (secs, nanos)
            }
        } else {
            (secs, nanos)
        };

        let array = JSArray::new(&ctx)?;
        array.push(result_secs)?;
        array.push(result_nanos)?;
        Ok(array.into_js_value(&ctx))
    }

    #[js_method(rename = "nextTick")]
    pub fn next_tick(&self, ctx: JSContext, callback: JSFunc, args: Rest<JSValue>) -> JSResult<()> {
        // Use Promise.resolve().then() to schedule callback as a microtask
        // This matches Node.js nextTick behavior - runs after current sync code, before I/O
        // This approach is cross-engine compatible (QuickJS, JSCore, ArkJS)
        let args_vec = args.0;

        // Create: Promise.resolve().then(() => callback.apply(null, args))
        let promise_ctor: JSObject = ctx.global().get("Promise")?;
        let resolve_fn: JSFunc = promise_ctor.get("resolve")?;
        let promise: JSObject = resolve_fn.call(Some(promise_ctor.clone()), ())?;

        // Wrap callback with args into a thunk
        let then_fn: JSFunc = promise.get("then")?;
        let thunk = JSFunc::new(&ctx, move || {
            // Call callback with captured arguments (Vec<JSValue> as a spread)
            callback.call::<_, ()>(None, (args_vec.clone(),))?;
            Ok(())
        })?;

        then_fn.call::<_, ()>(Some(promise), (thunk,))?;
        Ok(())
    }

    #[js_method(gc_mark)]
    fn gc_mark_with<F>(&self, mark_fn: F)
    where
        F: FnMut(&JSValue),
    {
        self.events.gc_mark_with(mark_fn);
    }
}

impl Default for Process {
    fn default() -> Self {
        Self::new()
    }
}

impl Emitter for Process {
    fn get_event_emitter(&self) -> EventEmitter {
        self.events.clone()
    }
}

/// Create stdout object with write method
fn create_stdout(ctx: &JSContext) -> JSResult<JSObject> {
    let stdout = JSObject::new(ctx);

    let write_fn = JSFunc::new(ctx, |data: String| {
        print!("{}", data);
        Ok(true)
    })?;

    stdout.set("write", write_fn)?;
    stdout.set(
        "isTTY",
        std::io::IsTerminal::is_terminal(&std::io::stdout()),
    )?;

    Ok(stdout)
}

/// Create stderr object with write method
fn create_stderr(ctx: &JSContext) -> JSResult<JSObject> {
    let stderr = JSObject::new(ctx);

    let write_fn = JSFunc::new(ctx, |data: String| {
        eprint!("{}", data);
        Ok(true)
    })?;

    stderr.set("write", write_fn)?;
    stderr.set(
        "isTTY",
        std::io::IsTerminal::is_terminal(&std::io::stderr()),
    )?;

    Ok(stderr)
}

fn create_env(ctx: &JSContext) -> JSResult<JSObject> {
    // Use a null-prototype object so env keys like "__proto__" behave like normal data
    // properties instead of triggering the magic Object.prototype accessor.
    let env_obj = ctx.eval::<JSObject>(Source::from_bytes("Object.create(null)"))?;
    for (key, value) in env::vars() {
        env_obj.set(key.as_str(), value.as_str())?;
    }
    Ok(env_obj)
}

const STDIN_CHUNK_SIZE: usize = 8192;

/// Create stdin object as a ReadableStream
fn create_stdin(ctx: &JSContext) -> JSResult<JSObject> {
    // Create a ReadableStream from tokio stdin
    let stdin = tokio::io::stdin();
    let stream = JSReadableStream::from_async_reader(ctx, stdin, STDIN_CHUNK_SIZE)?;
    let obj = stream.into_object();

    // Add isTTY property
    obj.set("isTTY", std::io::IsTerminal::is_terminal(&std::io::stdin()))?;

    Ok(obj)
}

/// Initialize the process module and inject into global scope
pub fn init(ctx: &JSContext) -> JSResult<()> {
    // Initialize stream module for stdin
    rong_stream::init(ctx)?;

    // Register the Process class without exposing a global constructor.
    ctx.register_hidden_class::<Process>()?;

    // Add EventEmitter methods to Process prototype
    Process::add_node_event_target_prototype(ctx)?;

    // Create the singleton process instance
    let process = Process::new();
    let process_obj: JSObject = JSValue::from_rust(ctx, process).to_rust()?;

    // Attach static singletons (stdin, stdout, stderr, env) to the process instance
    // This ensures identity equality (process.env === process.env)
    process_obj.set("stdin", create_stdin(ctx)?)?;
    process_obj.set("stdout", create_stdout(ctx)?)?;
    process_obj.set("stderr", create_stderr(ctx)?)?;
    process_obj.set("env", create_env(ctx)?)?;

    // Mount to global
    ctx.global().set("process", process_obj)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong::JSContext;
    use rong_test::*;

    #[test]
    fn test_process() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx)?;
            rong_timer::init(&ctx)?;
            rong_console::init(&ctx)?;
            rong_assert::init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "process.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}

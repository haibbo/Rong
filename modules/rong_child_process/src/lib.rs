//! Child Process module - Node.js compatible child process spawning
//!
//! Provides:
//! - `spawn(command, args?, options?)` - Spawn a new process (async, non-blocking)
//! - `exec(command, options?)` - Execute a shell command
//! - `execFile(file, args?, options?)` - Execute a file directly

use rong::{
    HostError, JSArray, JSContext, JSFunc, JSObject, JSResult, JSValue, Promise,
    function::{Optional, Rest, This},
    js_class, js_export, js_method,
};
use rong_event::{Emitter, EmitterExt, EventEmitter};
use rong_stream::{JSReadableStream, JSWritableStream};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::Notify;

#[cfg(windows)]
use tokio::sync::mpsc;

#[cfg(windows)]
enum ChildCommand {
    Kill,
}

fn type_error(message: impl Into<String>) -> HostError {
    HostError::new(rong::error::E_TYPE, message).with_name("TypeError")
}

/// Options for spawn/exec/execFile
#[derive(Default, Clone)]
pub struct SpawnOptions {
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub shell: Option<bool>,
    pub timeout: Option<u64>, // timeout in milliseconds
}

impl SpawnOptions {
    fn from_js_object(_ctx: &JSContext, obj: &JSObject) -> JSResult<Self> {
        let mut opts = SpawnOptions::default();
        if obj.has("cwd") {
            let cwd = obj
                .get::<_, String>("cwd")
                .map_err(|_| type_error("options.cwd must be a string"))?;
            opts.cwd = Some(cwd);
        }
        if obj.has("shell") {
            let shell = obj
                .get::<_, bool>("shell")
                .map_err(|_| type_error("options.shell must be a boolean"))?;
            opts.shell = Some(shell);
        }
        if obj.has("env") {
            let env_obj = obj
                .get::<_, JSObject>("env")
                .map_err(|_| type_error("options.env must be an object of string values"))?;
            let entries = env_obj
                .entries_as::<String, String>()
                .map_err(|_| type_error("options.env must be an object of string values"))?;
            let mut env_map = HashMap::with_capacity(entries.len());
            for (k, v) in entries {
                env_map.insert(k, v);
            }
            opts.env = Some(env_map);
        }
        if obj.has("timeout") {
            let timeout = obj
                .get::<_, f64>("timeout")
                .map_err(|_| type_error("options.timeout must be a non-negative number"))?;
            if !timeout.is_finite() || timeout < 0.0 {
                return Err(type_error("options.timeout must be a non-negative number").into());
            }
            opts.timeout = Some(timeout as u64);
        }
        Ok(opts)
    }
}

/// ChildProcess class representing a spawned child process
#[js_export]
pub struct ChildProcess {
    events: EventEmitter,
    pid: Option<u32>,
    exit_code: Arc<Mutex<Option<i32>>>,
    exit_notify: Arc<Notify>,
    exited: Arc<AtomicBool>,
    #[cfg(windows)]
    kill_tx: Option<mpsc::Sender<ChildCommand>>,
}

#[js_class]
impl ChildProcess {
    #[js_method(constructor)]
    pub fn new() -> Self {
        Self {
            events: EventEmitter::new(),
            pid: None,
            exit_code: Arc::new(Mutex::new(None)),
            exit_notify: Arc::new(Notify::new()),
            exited: Arc::new(AtomicBool::new(false)),
            #[cfg(windows)]
            kill_tx: None,
        }
    }

    #[js_method(getter)]
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    #[js_method(getter, rename = "exitCode")]
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code.lock().ok().and_then(|g| *g)
    }

    /// Kill the child process with optional signal.
    /// Supported signals: SIGTERM (default), SIGKILL, SIGINT, SIGHUP, SIGUSR1, SIGUSR2
    /// Returns true if the signal was sent successfully.
    #[js_method]
    pub fn kill(&self, signal: Optional<String>) -> bool {
        let Some(pid) = self.pid else {
            return false;
        };

        #[cfg(unix)]
        {
            let sig = match signal.0.as_deref() {
                None | Some("SIGTERM") => libc::SIGTERM,
                Some("SIGKILL") => libc::SIGKILL,
                Some("SIGINT") => libc::SIGINT,
                Some("SIGHUP") => libc::SIGHUP,
                Some("SIGUSR1") => libc::SIGUSR1,
                Some("SIGUSR2") => libc::SIGUSR2,
                Some("SIGQUIT") => libc::SIGQUIT,
                Some("SIGSTOP") => libc::SIGSTOP,
                Some("SIGCONT") => libc::SIGCONT,
                // Also support numeric signals or without SIG prefix
                Some(s) => {
                    if let Some(stripped) = s.strip_prefix("SIG") {
                        match stripped {
                            "TERM" => libc::SIGTERM,
                            "KILL" => libc::SIGKILL,
                            "INT" => libc::SIGINT,
                            "HUP" => libc::SIGHUP,
                            "USR1" => libc::SIGUSR1,
                            "USR2" => libc::SIGUSR2,
                            "QUIT" => libc::SIGQUIT,
                            "STOP" => libc::SIGSTOP,
                            "CONT" => libc::SIGCONT,
                            _ => return false,
                        }
                    } else if let Ok(num) = s.parse::<i32>() {
                        num
                    } else {
                        return false;
                    }
                }
            };
            // SAFETY: pid is valid process id obtained from spawn
            unsafe { libc::kill(pid as i32, sig) == 0 }
        }

        #[cfg(windows)]
        {
            let _ = pid;
            let _ = signal;
            if let Some(tx) = &self.kill_tx {
                return tx.try_send(ChildCommand::Kill).is_ok();
            }
            false
        }
    }

    /// Wait for the process to exit and return the exit code.
    #[js_method]
    pub async fn wait(&self) -> JSResult<Option<i32>> {
        loop {
            let notified = self.exit_notify.notified();
            if self.exited.load(Ordering::SeqCst) {
                break;
            }
            notified.await;
        }
        Ok(self.exit_code.lock().ok().and_then(|g| *g))
    }

    #[js_method(gc_mark)]
    fn gc_mark_with<F>(&self, mark_fn: F)
    where
        F: FnMut(&JSValue),
    {
        self.events.gc_mark_with(mark_fn);
    }
}

impl Default for ChildProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl Emitter for ChildProcess {
    fn get_event_emitter(&self) -> EventEmitter {
        self.events.clone()
    }
}

/// Result of exec command
#[js_export]
pub struct ExecResult {
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

#[js_class]
impl ExecResult {
    #[js_method(constructor)]
    pub fn new() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            code: None,
        }
    }

    #[js_method(getter)]
    pub fn stdout(&self) -> String {
        self.stdout.clone()
    }

    #[js_method(getter)]
    pub fn stderr(&self) -> String {
        self.stderr.clone()
    }

    #[js_method(getter)]
    pub fn code(&self) -> Option<i32> {
        self.code
    }

    #[js_method(gc_mark)]
    fn gc_mark_with<F>(&self, _mark_fn: F)
    where
        F: FnMut(&JSValue),
    {
    }
}

impl Default for ExecResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape a string for safe use in shell commands (Unix)
#[cfg(not(target_os = "windows"))]
fn shell_escape(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Read process.env from the global scope and convert to HashMap
/// This allows JS modifications to process.env to propagate to child processes
fn get_process_env(ctx: &JSContext) -> Option<HashMap<String, String>> {
    let global = ctx.global();
    let process: JSObject = global.get("process").ok()?;
    let env_obj: JSObject = process.get("env").ok()?;

    let mut env_map = HashMap::new();
    if let Ok(entries) = env_obj.entries_as::<String, String>() {
        for (k, v) in entries {
            env_map.insert(k, v);
        }
    }
    Some(env_map)
}

/// Build a Command from the given parameters
fn build_command(
    command: &str,
    args: &[String],
    options: &SpawnOptions,
    use_shell: bool,
) -> Command {
    let mut cmd = if use_shell {
        #[cfg(target_os = "windows")]
        {
            let mut c = Command::new("cmd");
            c.arg("/C");
            let full_cmd = if args.is_empty() {
                command.to_string()
            } else {
                let escaped_args: Vec<String> = args
                    .iter()
                    .map(|a| {
                        if a.contains(|c: char| c.is_whitespace() || "\"&|<>^".contains(c)) {
                            format!("\"{}\"", a.replace('"', "\"\""))
                        } else {
                            a.clone()
                        }
                    })
                    .collect();
                format!("{} {}", command, escaped_args.join(" "))
            };
            c.arg(full_cmd);
            c
        }
        #[cfg(not(target_os = "windows"))]
        {
            let mut c = Command::new("sh");
            c.arg("-c");
            let full_cmd = if args.is_empty() {
                command.to_string()
            } else {
                let escaped_args: Vec<String> = args.iter().map(|a| shell_escape(a)).collect();
                format!("{} {}", command, escaped_args.join(" "))
            };
            c.arg(full_cmd);
            c
        }
    } else {
        let mut c = Command::new(command);
        c.args(args);
        c
    };

    if let Some(ref cwd) = options.cwd {
        cmd.current_dir(cwd);
    }

    if let Some(ref env) = options.env {
        cmd.env_clear();
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    cmd
}

/// Helper to parse args from Optional<JSValue>
fn parse_args(args: &Optional<JSValue>) -> JSResult<Vec<String>> {
    if let Some(ref args_val) = args.0 {
        if args_val.is_null() || args_val.is_undefined() {
            return Ok(vec![]);
        }
        if let Some(arr) = args_val
            .clone()
            .into_object()
            .and_then(JSArray::from_object)
        {
            let mut result = Vec::new();
            for i in 0..arr.len()? {
                let val = arr
                    .get_opt::<String>(i)?
                    .ok_or_else(|| type_error("args must be an array of strings"))?;
                result.push(val);
            }
            return Ok(result);
        }
        return Err(type_error("args must be an array of strings").into());
    }
    Ok(vec![])
}

const STREAM_CHUNK_SIZE: usize = 8192;

async fn read_all(
    mut stdout: Option<impl tokio::io::AsyncRead + Unpin>,
) -> Result<Vec<u8>, std::io::Error> {
    use tokio::io::AsyncReadExt;

    let mut buf = Vec::new();
    if let Some(ref mut out) = stdout {
        out.read_to_end(&mut buf).await?;
    }
    Ok(buf)
}

async fn run_command_with_output(
    mut child: Child,
    timeout: Option<u64>,
) -> JSResult<(std::process::ExitStatus, Vec<u8>, Vec<u8>)> {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_task = tokio::task::spawn_local(async move { read_all(stdout).await });
    let stderr_task = tokio::task::spawn_local(async move { read_all(stderr).await });

    let status = if let Some(timeout_ms) = timeout {
        match tokio::time::timeout(Duration::from_millis(timeout_ms), child.wait()).await {
            Ok(res) => res.map_err(|e| HostError::new(rong::error::E_IO, e.to_string()))?,
            Err(_) => {
                let _ = child.start_kill();
                let _ = child.wait().await;
                let _ = stdout_task.await;
                let _ = stderr_task.await;
                return Err(HostError::new(rong::error::E_TIMEOUT, "Command timed out").into());
            }
        }
    } else {
        child
            .wait()
            .await
            .map_err(|e| HostError::new(rong::error::E_IO, e.to_string()))?
    };

    let stdout_bytes = stdout_task
        .await
        .map_err(|e| HostError::new(rong::error::E_INTERNAL, e.to_string()))?
        .map_err(|e| HostError::new(rong::error::E_IO, e.to_string()))?;
    let stderr_bytes = stderr_task
        .await
        .map_err(|e| HostError::new(rong::error::E_INTERNAL, e.to_string()))?
        .map_err(|e| HostError::new(rong::error::E_IO, e.to_string()))?;

    Ok((status, stdout_bytes, stderr_bytes))
}

/// spawn(command, args?, options?) - Spawn a new process and return ChildProcess
/// This is now truly async - returns immediately with streaming stdout/stderr
fn spawn(
    ctx: JSContext,
    command: String,
    args: Optional<JSValue>,
    options: Optional<JSObject>,
) -> JSResult<JSObject> {
    let args_vec = parse_args(&args)?;

    let mut opts = if let Some(ref opts_obj) = options.0 {
        SpawnOptions::from_js_object(&ctx, opts_obj)?
    } else {
        SpawnOptions::default()
    };

    // If no explicit env option, use process.env from JS (allows JS modifications to propagate)
    if opts.env.is_none() {
        opts.env = get_process_env(&ctx);
    }

    let use_shell = opts.shell.unwrap_or(false);
    let mut cmd = build_command(&command, &args_vec, &opts, use_shell);

    let mut child = cmd
        .spawn()
        .map_err(|e| HostError::new(rong::error::E_IO, e.to_string()))?;

    let pid = child.id();

    // Take stdin/stdout/stderr for streaming
    let stdin_writer = child.stdin.take();
    let stdout_reader = child.stdout.take();
    let stderr_reader = child.stderr.take();

    // Create ChildProcess instance
    let mut child_process = ChildProcess::new();
    child_process.pid = pid;

    // Clones for the background wait task.
    let exit_code = child_process.exit_code.clone();
    let exit_notify = child_process.exit_notify.clone();
    let exited = child_process.exited.clone();

    #[cfg(windows)]
    let mut kill_rx = {
        let (tx, rx) = mpsc::channel::<ChildCommand>(4);
        child_process.kill_tx = Some(tx);
        rx
    };

    // Create the JS object
    let child_obj = JSValue::from(&ctx, child_process);
    let child_obj: JSObject = child_obj.try_into()?;

    // Create WritableStream for stdin
    if let Some(stdin) = stdin_writer {
        let stdin_stream = JSWritableStream::from_async_writer(&ctx, stdin)?;
        child_obj.set("stdin", stdin_stream.into_object())?;
    } else {
        child_obj.set("stdin", JSValue::null(&ctx))?;
    }

    // Create ReadableStream for stdout
    if let Some(stdout) = stdout_reader {
        let stdout_stream = JSReadableStream::from_async_reader(&ctx, stdout, STREAM_CHUNK_SIZE)?;
        child_obj.set("stdout", stdout_stream.into_object())?;
    } else {
        child_obj.set("stdout", JSValue::null(&ctx))?;
    }

    // Create ReadableStream for stderr
    if let Some(stderr) = stderr_reader {
        let stderr_stream = JSReadableStream::from_async_reader(&ctx, stderr, STREAM_CHUNK_SIZE)?;
        child_obj.set("stderr", stderr_stream.into_object())?;
    } else {
        child_obj.set("stderr", JSValue::null(&ctx))?;
    }

    // Start background wait task to emit 'exit' and resolve waiters
    let ctx_for_exit = ctx.clone();
    let child_obj_for_exit = child_obj.clone();

    rong::spawn(async move {
        let emit_exit = |code: Option<i32>| {
            if let Ok(mut ec) = exit_code.lock() {
                *ec = code;
            }
            exited.store(true, Ordering::SeqCst);
            exit_notify.notify_waiters();

            let code_val = match code {
                Some(code) => JSValue::from(&ctx_for_exit, code),
                None => JSValue::null(&ctx_for_exit),
            };
            let _ = ChildProcess::do_emit(
                This(child_obj_for_exit.clone()),
                rong_event::EventKey::from("exit"),
                Rest(vec![code_val]),
            );
        };

        #[cfg(windows)]
        {
            loop {
                tokio::select! {
                    status = child.wait() => {
                        let code = status.ok().and_then(|s| s.code());
                        emit_exit(code);
                        break;
                    }
                    cmd = kill_rx.recv() => {
                        match cmd {
                            Some(ChildCommand::Kill) => {
                                let _ = child.start_kill();
                            }
                            None => {
                                break;
                            }
                        }
                    }
                }
            }

            if !exited.load(Ordering::SeqCst) {
                let status = child.wait().await;
                let code = status.ok().and_then(|s| s.code());
                emit_exit(code);
            }
        }

        #[cfg(not(windows))]
        {
            let status = child.wait().await;
            let code = status.ok().and_then(|s| s.code());
            emit_exit(code);
        }
    });

    Ok(child_obj)
}

/// exec(command, options?) - Execute a shell command and return a promise
fn exec(ctx: JSContext, command: String, options: Optional<JSObject>) -> JSResult<Promise> {
    let mut opts = if let Some(ref opts_obj) = options.0 {
        SpawnOptions::from_js_object(&ctx, opts_obj)?
    } else {
        SpawnOptions::default()
    };

    // If no explicit env option, use process.env from JS (allows JS modifications to propagate)
    if opts.env.is_none() {
        opts.env = get_process_env(&ctx);
    }

    let cwd = opts.cwd.clone();
    let env = opts.env.clone();
    let timeout = opts.timeout;

    Promise::from_future(&ctx, None, async move {
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(&command);
            c
        };

        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.arg("-c").arg(&command);
            c
        };

        if let Some(ref cwd) = cwd {
            cmd.current_dir(cwd);
        }

        if let Some(ref env) = env {
            cmd.env_clear();
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| HostError::new(rong::error::E_IO, e.to_string()))?;
        let (status, stdout, stderr) = run_command_with_output(child, timeout).await?;

        Ok(ExecResult {
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
            code: status.code(),
        })
    })
}

/// execFile(file, args?, options?) - Execute a file directly
fn exec_file(
    ctx: JSContext,
    file: String,
    args: Optional<JSValue>,
    options: Optional<JSObject>,
) -> JSResult<Promise> {
    let args_vec = parse_args(&args)?;

    let mut opts = if let Some(ref opts_obj) = options.0 {
        SpawnOptions::from_js_object(&ctx, opts_obj)?
    } else {
        SpawnOptions::default()
    };

    // If no explicit env option, use process.env from JS (allows JS modifications to propagate)
    if opts.env.is_none() {
        opts.env = get_process_env(&ctx);
    }

    let cwd = opts.cwd.clone();
    let env = opts.env.clone();
    let timeout = opts.timeout;

    Promise::from_future(&ctx, None, async move {
        let mut cmd = Command::new(&file);
        cmd.args(&args_vec);

        if let Some(ref cwd) = cwd {
            cmd.current_dir(cwd);
        }

        if let Some(ref env) = env {
            cmd.env_clear();
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| HostError::new(rong::error::E_IO, e.to_string()))?;
        let (status, stdout, stderr) = run_command_with_output(child, timeout).await?;

        Ok(ExecResult {
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
            code: status.code(),
        })
    })
}

/// Initialize the child_process module
pub fn init(ctx: &JSContext) -> JSResult<()> {
    rong_stream::init(ctx)?;

    ctx.register_class::<ChildProcess>()?;
    ctx.register_class::<ExecResult>()?;

    ChildProcess::add_node_event_target_prototype(ctx)?;

    let child_process = JSObject::new(ctx);

    child_process.set("spawn", JSFunc::new(ctx, spawn)?)?;
    child_process.set("exec", JSFunc::new(ctx, exec)?)?;
    child_process.set("execFile", JSFunc::new(ctx, exec_file)?)?;

    ctx.global().set("child_process", child_process)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong::JSContext;
    use rong_test::*;

    #[test]
    fn test_child_process() {
        async_run!(|ctx: JSContext| async move {
            rong_process::init(&ctx)?;
            rong_encoding::init(&ctx)?;
            init(&ctx)?;
            rong_timer::init(&ctx)?;
            rong_console::init(&ctx)?;
            rong_assert::init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "child_process.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}

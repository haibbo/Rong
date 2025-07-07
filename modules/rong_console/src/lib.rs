use rong::{function::*, *};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, IsTerminal, Write};

thread_local! {
    static CONSOLE_WRITER: RefCell<Option<Box<dyn ConsoleWriter>>> = RefCell::new(None);
}

#[derive(Debug)]
pub enum LogLevel {
    Verbose,
    Error,
    Warn,
    Info,
    Debug,
}

pub trait ConsoleWriter: Send + Sync + fmt::Debug {
    fn write(&self, level: LogLevel, message: String);
    fn is_tty(&self) -> bool;
}

#[derive(Debug)]
struct DefaultWriter;

impl ConsoleWriter for DefaultWriter {
    fn write(&self, level: LogLevel, message: String) {
        match level {
            LogLevel::Verbose | LogLevel::Info => {
                println!("{}", message);
            }
            LogLevel::Debug => {
                println!("DEBUG: {}", message);
            }
            LogLevel::Error => {
                eprintln!("ERROR: {}", message);
            }
            LogLevel::Warn => {
                eprintln!("WARN: {}", message);
            }
        }
    }

    fn is_tty(&self) -> bool {
        io::stdout().is_terminal()
    }
}

/// Set a custom console writer for the current thread.
pub fn set_writer(writer: Box<dyn ConsoleWriter>) {
    CONSOLE_WRITER.with(|w| {
        *w.borrow_mut() = Some(writer);
    });
}

/// Write a message using the thread-local console writer
fn write_console(level: LogLevel, message: String) {
    CONSOLE_WRITER.with(|w| {
        let mut writer = w.borrow_mut();
        if writer.is_none() {
            *writer = Some(Box::new(DefaultWriter));
        }
        writer.as_ref().unwrap().write(level, message);
    });
}

/// Check if the console writer is a TTY
fn console_writer_is_tty() -> bool {
    CONSOLE_WRITER.with(|w| {
        let mut writer = w.borrow_mut();
        if writer.is_none() {
            *writer = Some(Box::new(DefaultWriter));
        }
        writer.as_ref().unwrap().is_tty()
    })
}

/// Initialize the console module
pub fn init(ctx: &JSContext) -> JSResult<()> {
    let console = JSObject::new(ctx);

    console
        .set("clear", JSFunc::new(ctx, clear)?)?
        .set("log", JSFunc::new(ctx, verbose)?)?
        .set("error", JSFunc::new(ctx, error)?)?
        .set("warn", JSFunc::new(ctx, warn)?)?
        .set("info", JSFunc::new(ctx, info)?)?
        .set("debug", JSFunc::new(ctx, debug)?)?;

    ctx.register_class::<Console>()?;
    ctx.global().set("console", console)?;
    Ok(())
}

fn log_message(level: LogLevel, message: String) {
    write_console(level, message);
}

fn clear() {
    if console_writer_is_tty() {
        // ANSI clear screen sequence
        print!("\x1B[2J\x1B[1;1H");
        // Ensure immediate output flush
        io::stdout().flush().unwrap();
    } else {
        // In non-terminal environment, print a newline
        println!();
    }
}

fn verbose(ctx: JSContext, args: Rest<JSValue>) {
    let message = format_args(&ctx, args);
    log_message(LogLevel::Info, message);
}

fn error(ctx: JSContext, args: Rest<JSValue>) {
    let message = format_args(&ctx, args);
    log_message(LogLevel::Error, message);
}

fn warn(ctx: JSContext, args: Rest<JSValue>) {
    let message = format_args(&ctx, args);
    log_message(LogLevel::Warn, message);
}

fn info(ctx: JSContext, args: Rest<JSValue>) {
    let message = format_args(&ctx, args);
    log_message(LogLevel::Info, message);
}

fn debug(ctx: JSContext, args: Rest<JSValue>) {
    let message = format_args(&ctx, args);
    log_message(LogLevel::Debug, message);
}

fn format_args(_ctx: &JSContext, args: Rest<JSValue>) -> String {
    let mut result = String::new();
    format_values_internal(&mut result, args);
    result
}

fn format_values_internal(result: &mut String, args: Rest<JSValue>) {
    let size = args.len();
    let mut iter = args.0.into_iter().enumerate().peekable();

    while let Some((index, arg)) = iter.next() {
        // Handle formatted strings
        if index == 0 && size > 1 {
            if let Ok(format_str) = arg.clone().try_into::<String>() {
                let mut chars = format_str.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '%' {
                        match chars.next() {
                            Some('s') => {
                                if let Some((_, next_arg)) = iter.next() {
                                    if let Ok(str) = next_arg.try_into::<String>() {
                                        result.push_str(&str);
                                        continue;
                                    }
                                    continue;
                                }
                            }
                            Some('d') | Some('i') => {
                                if let Some((_, next_arg)) = iter.next() {
                                    if let Ok(num) = next_arg.try_into::<f64>() {
                                        result.push_str(&num.trunc().to_string());
                                        continue;
                                    }
                                }
                            }
                            Some('f') => {
                                if let Some((_, next_arg)) = iter.next() {
                                    if let Ok(num) = next_arg.try_into::<f64>() {
                                        result.push_str(&num.to_string());
                                        continue;
                                    }
                                }
                            }
                            Some('o') | Some('O') => {
                                if let Some((_, next_arg)) = iter.next() {
                                    format_raw_inner(result, next_arg, &mut HashSet::default(), 0);
                                    continue;
                                }
                            }
                            Some('%') => {
                                result.push('%');
                                continue;
                            }
                            _ => {}
                        }
                    }
                    result.push(c);
                }
                continue;
            }
        }

        // Non-formatted string regular argument
        if index != 0 {
            result.push(' ');
        }

        // handle next arg
        format_raw_inner(result, arg, &mut HashSet::default(), 0);
    }
}

fn format_raw_inner(
    result: &mut String,
    value: JSValue,
    visited: &mut HashSet<usize>,
    depth: usize,
) {
    if depth > 8 {
        result.push_str("[Maximum recursion depth exceeded]");
        return;
    }

    match value.type_of() {
        JSValueType::Undefined => result.push_str("undefined"),
        JSValueType::Null => result.push_str("null"),

        JSValueType::Boolean => {
            if let Ok(b) = value.try_into::<bool>() {
                result.push_str(if b { "true" } else { "false" });
            }
        }

        JSValueType::Number => {
            if let Ok(n) = value.try_into::<f64>() {
                result.push_str(&n.to_string());
            }
        }

        JSValueType::BigInt => {
            if let Ok(s) = value.try_into::<i64>() {
                result.push_str(&s.to_string());
            }
        }

        JSValueType::String => {
            if let Ok(s) = value.try_into::<String>() {
                if depth > 0 {
                    result.push('"');
                    result.push_str(&escape_string(&s));
                    result.push('"');
                } else {
                    result.push_str(&s);
                }
            }
        }

        JSValueType::Date => {
            if let Ok(s) = value.try_into::<String>() {
                result.push_str(&s);
            }
        }

        JSValueType::Object | JSValueType::Array => {
            let obj: JSObject = value.clone().into();
            let hash = default_hash(&value);
            if visited.contains(&hash) {
                result.push_str("[Circular]");
                return;
            }
            visited.insert(hash);

            if let Some(array) = JSArray::from_object(obj.clone()) {
                format_array(result, array, visited, depth);
            } else {
                format_object(result, obj, visited, depth);
            }
            visited.remove(&hash);
        }

        JSValueType::Function => {
            let obj: JSObject = value.into();
            let mut fn_info = Vec::new();

            if let Ok(name) = obj.get::<_, String>("name") {
                if !name.is_empty() {
                    fn_info.push(format!("Function: {}", name));
                } else {
                    fn_info.push("anonymous".to_string());
                }
            } else {
                fn_info.push("anonymous".to_string());
            }

            if let Ok(length) = obj.get::<_, f64>("length") {
                fn_info.push(format!("length: {}", length as usize));
            }

            result.push('[');
            result.push_str(&fn_info.join(", "));
            result.push(']');
        }

        JSValueType::Symbol => {
            let symbol = JSSymbol::from_object(value.into()).unwrap();
            if let Ok(description) = symbol.descripiton() {
                if !description.is_empty() {
                    result.push_str(&format!("Symbol({})", description));
                } else {
                    result.push_str("Symbol()");
                }
            }
        }

        JSValueType::Promise => {
            let obj: JSObject = value.into();
            if let Ok(state) = obj.get::<_, String>("state") {
                result.push_str(&format!("Promise <{}>", state));
            }
        }

        JSValueType::Constructor => {
            let obj: JSObject = value.into();
            if let Ok(name) = obj.get::<_, String>("name") {
                if !name.is_empty() {
                    result.push_str(&format!("[class {}]", name));
                    return;
                }
            }

            if let Ok(prototype) = obj.get::<_, JSObject>("prototype") {
                if let Ok(constructor_name) = prototype.get::<_, String>("constructor") {
                    if !constructor_name.is_empty() {
                        result.push_str(&format!("[class {}]", constructor_name));
                    }
                }
            }
        }

        JSValueType::Error | JSValueType::Exception => {
            let obj: JSObject = value.into();
            let mut error_parts = Vec::new();

            if let Ok(name) = obj.get::<_, String>("name") {
                error_parts.push(name);
            } else {
                error_parts.push("Error".to_string());
            }

            if let Ok(message) = obj.get::<_, String>("message") {
                if !message.is_empty() {
                    error_parts.push(message);
                }
            }

            result.push_str(&error_parts.join(": "));

            if depth == 0 {
                if let Ok(stack) = obj.get::<_, String>("stack") {
                    if !stack.is_empty() {
                        error_parts.push(stack);
                    }
                }
            }
        }

        JSValueType::ArrayBuffer => {
            let obj: JSObject = value.into();
            format_array_buffer(result, obj);
        }

        JSValueType::Unknown => {
            result.push_str("[Unknown]");
        }
    }
}

fn format_array(result: &mut String, array: JSArray, visited: &mut HashSet<usize>, depth: usize) {
    result.push_str("[ ");
    for (index, item) in array.iter::<JSValue>().flatten().enumerate() {
        if index > 0 {
            result.push_str(", ");
        }
        format_raw_inner(result, item, visited, depth + 1);
    }
    result.push_str(" ]");
}

fn format_object(result: &mut String, obj: JSObject, visited: &mut HashSet<usize>, depth: usize) {
    result.push('{');
    let mut first = true;

    if let Ok(entries) = obj.entries() {
        for entry in entries {
            if !first {
                result.push_str(", ");
            }
            first = false;

            if let Ok(key_str) = entry.key().clone().try_into::<String>() {
                if needs_quotes(&key_str) {
                    result.push('"');
                    result.push_str(&escape_string(&key_str));
                    result.push('"');
                } else {
                    result.push_str(&key_str);
                }
                result.push_str(": ");

                format_raw_inner(result, entry.value().clone(), visited, depth + 1);
            }
        }
    }

    result.push('}');
}

fn format_array_buffer(result: &mut String, obj: JSObject) {
    if let Some(buffer) = JSArrayBuffer::<u8>::from_object(obj.clone()) {
        let len = buffer.len();

        // For small ArrayBuffer, display its content
        if len <= 50 && len > 0 {
            result.push_str("ArrayBuffer { ");
            result.push_str(&format!("byteLength: {}", len));

            // Try to get and display the byte content
            if let Some(bytes) = buffer.as_bytes() {
                result.push_str(", bytes: [");

                for (i, byte) in bytes.iter().enumerate() {
                    if i > 0 {
                        result.push_str(", ");
                    }
                    result.push_str(&format!("0x{:02x}", byte));
                }

                result.push(']');
            }

            result.push_str(" }");
        } else {
            // For large ArrayBuffer, only display the length
            result.push_str(&format!("ArrayBuffer {{ byteLength: {} }}", len));
        }
    }
}

fn needs_quotes(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }

    let first_char = s.chars().next().unwrap();
    if !first_char.is_ascii_alphabetic() && first_char != '_' && first_char != '$' {
        return true;
    }

    !s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

fn escape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\x08' => result.push_str("\\b"),
            '\x0c' => result.push_str("\\f"),
            c if c.is_ascii_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

#[js_export]
pub struct Console {}

#[js_class]
impl Console {
    #[js_method(constructor)]
    fn new() -> Self {
        Self {}
    }

    #[js_method]
    pub fn log(&self, ctx: JSContext, args: Rest<JSValue>) {
        verbose(ctx, args);
    }

    #[js_method]
    pub fn error(&self, ctx: JSContext, args: Rest<JSValue>) {
        error(ctx, args);
    }

    #[js_method]
    pub fn warn(&self, ctx: JSContext, args: Rest<JSValue>) {
        warn(ctx, args);
    }

    #[js_method]
    pub fn info(&self, ctx: JSContext, args: Rest<JSValue>) {
        info(ctx, args);
    }

    #[js_method]
    pub fn debug(&self, ctx: JSContext, args: Rest<JSValue>) {
        debug(ctx, args);
    }

    #[js_method]
    pub fn clear() {
        clear();
    }
}

#[inline]
pub fn default_hash<T: Hash + ?Sized>(v: &T) -> usize {
    let mut state = DefaultHasher::default();
    v.hash(&mut state);
    state.finish() as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::run;
    use std::sync::Mutex;

    static TEST_OUTPUT: Mutex<String> = Mutex::new(String::new());

    fn clear_test_output() {
        let mut output = TEST_OUTPUT.lock().unwrap();
        *output = String::new();
    }

    fn get_test_output() -> String {
        TEST_OUTPUT.lock().unwrap().clone()
    }

    fn append_test_output(message: &str) {
        let mut output = TEST_OUTPUT.lock().unwrap();
        output.push_str(message);
        output.push('\n');
    }

    #[derive(Debug)]
    struct TestConsoleWriter;

    impl ConsoleWriter for TestConsoleWriter {
        fn write(&self, _level: LogLevel, message: String) {
            println!("{}", message);
            append_test_output(&message);
        }

        fn is_tty(&self) -> bool {
            false
        }
    }

    #[test]
    fn test_console_log_formatted_string() {
        run(|ctx| {
            clear_test_output();
            // Reset thread-local storage
            CONSOLE_WRITER.with(|w| {
                *w.borrow_mut() = None;
            });
            init(ctx)?;
            set_writer(Box::new(TestConsoleWriter));

            let js_code = r#"console.log("Name: %s, Age: %d", "Alice", 30);"#;
            ctx.eval::<()>(Source::from_bytes(js_code))?;

            let output = get_test_output().trim().to_string();
            assert_eq!(
                output, "Name: Alice, Age: 30",
                "Output should match formatted string"
            );
            Ok(())
        });
    }

    #[test]
    fn test_console_log_circular_reference() {
        run(|ctx| {
            clear_test_output();
            // Reset thread-local storage
            CONSOLE_WRITER.with(|w| {
                *w.borrow_mut() = None;
            });
            init(ctx)?;
            set_writer(Box::new(TestConsoleWriter));

            let js_code = r#"
                // Create an object with circular reference
                const obj = { name: "Circular Object" };
                obj.self = obj;

                console.log("Circular object:", obj);
            "#;
            ctx.eval::<()>(Source::from_bytes(js_code))?;

            let output = get_test_output().trim().to_string();
            assert!(
                output.contains("[Circular]"),
                "Output '{}' should contain circular reference marker",
                output
            );
            Ok(())
        });
    }

    #[test]
    fn test_console_log_max_depth() {
        run(|ctx| {
            clear_test_output();
            // Reset thread-local storage
            CONSOLE_WRITER.with(|w| {
                *w.borrow_mut() = None;
            });
            init(ctx)?;
            set_writer(Box::new(TestConsoleWriter));

            let js_code = r#"
                // Function to create a deeply nested object
                function createDeepObject(depth) {
                    if (depth <= 0) return {};
                    return { child: createDeepObject(depth - 1) };
                }

                // Create an object that exceeds maximum recursion depth
                const deepObj = createDeepObject(15);

                console.log("Deep object:", deepObj);
            "#;
            ctx.eval::<()>(Source::from_bytes(js_code))?;

            let output = get_test_output().trim().to_string();
            assert!(
                output.contains("[Maximum recursion depth exceeded]"),
                "Output '{}' should contain recursion depth warning",
                output
            );
            Ok(())
        });
    }
}

use rong::*;
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::cell::RefCell;
use std::rc::Rc;

/// JavaScript keywords for completion
const JS_KEYWORDS: &[&str] = &[
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "null",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "undefined",
    "var",
    "void",
    "while",
    "with",
    "yield",
    "async",
];

/// Shared state for completions
#[derive(Default)]
pub struct CompletionState {
    globals: Vec<String>,
}

/// REPL helper for tab completion
pub struct ReplHelper {
    state: Rc<RefCell<CompletionState>>,
}

impl ReplHelper {
    pub fn new(state: Rc<RefCell<CompletionState>>) -> Self {
        Self { state }
    }

    /// Get properties for well-known objects
    fn get_object_properties(&self, obj_name: &str) -> Vec<String> {
        match obj_name {
            "console" => vec!["clear", "debug", "error", "info", "log", "warn"],
            "Math" => vec![
                "abs", "acos", "acosh", "asin", "asinh", "atan", "atan2", "atanh", "cbrt", "ceil",
                "clz32", "cos", "cosh", "exp", "expm1", "floor", "fround", "hypot", "imul", "log",
                "log10", "log1p", "log2", "max", "min", "pow", "random", "round", "sign", "sin",
                "sinh", "sqrt", "tan", "tanh", "trunc", "E", "LN2", "LN10", "LOG2E", "LOG10E",
                "PI", "SQRT1_2", "SQRT2",
            ],
            "JSON" => vec!["parse", "stringify"],
            "Object" => vec![
                "assign",
                "create",
                "defineProperty",
                "defineProperties",
                "entries",
                "freeze",
                "fromEntries",
                "getOwnPropertyDescriptor",
                "getOwnPropertyDescriptors",
                "getOwnPropertyNames",
                "getOwnPropertySymbols",
                "getPrototypeOf",
                "is",
                "isExtensible",
                "isFrozen",
                "isSealed",
                "keys",
                "preventExtensions",
                "seal",
                "setPrototypeOf",
                "values",
            ],
            "Array" => vec!["from", "isArray", "of"],
            "Promise" => vec!["all", "allSettled", "any", "race", "reject", "resolve"],
            "Number" => vec![
                "isFinite",
                "isInteger",
                "isNaN",
                "isSafeInteger",
                "parseFloat",
                "parseInt",
                "EPSILON",
                "MAX_SAFE_INTEGER",
                "MAX_VALUE",
                "MIN_SAFE_INTEGER",
                "MIN_VALUE",
                "NaN",
                "NEGATIVE_INFINITY",
                "POSITIVE_INFINITY",
            ],
            "String" => vec!["fromCharCode", "fromCodePoint", "raw"],
            "globalThis" => {
                let state = self.state.borrow();
                return state
                    .globals
                    .iter()
                    .filter(|g| !g.starts_with("__"))
                    .cloned()
                    .collect();
            }
            _ => vec![],
        }
        .into_iter()
        .map(|s| s.to_string())
        .collect()
    }
}

impl Helper for ReplHelper {}
impl Hinter for ReplHelper {
    type Hint = String;
}
impl Highlighter for ReplHelper {}
impl Validator for ReplHelper {}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line_to_cursor = &line[..pos];

        // Find the start of the current identifier (including dots for property access)
        let word_start = line_to_cursor
            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '$' && c != '.')
            .map(|i| i + 1)
            .unwrap_or(0);

        let word = &line_to_cursor[word_start..];

        // Don't complete on empty input
        if word.is_empty() {
            return Ok((0, vec![]));
        }

        // Check if we're completing a property (e.g., "console.")
        if let Some(dot_pos) = word.rfind('.') {
            let obj_name = &word[..dot_pos];
            let prop_prefix = &word[dot_pos + 1..];

            // Get properties for known objects
            let props = self.get_object_properties(obj_name);
            let matches: Vec<Pair> = props
                .into_iter()
                .filter(|p| p.starts_with(prop_prefix))
                .map(|p| Pair {
                    display: p.clone(),
                    replacement: p,
                })
                .collect();

            return Ok((word_start + dot_pos + 1, matches));
        }

        // Complete globals and keywords
        let state = self.state.borrow();
        let mut matches: Vec<Pair> = Vec::new();

        // Add matching globals (filter out internal names starting with __)
        for global in &state.globals {
            if global.starts_with(word) && !global.starts_with("__") {
                matches.push(Pair {
                    display: global.clone(),
                    replacement: global.clone(),
                });
            }
        }

        // Add matching keywords
        for &kw in JS_KEYWORDS {
            if kw.starts_with(word) {
                matches.push(Pair {
                    display: kw.to_string(),
                    replacement: kw.to_string(),
                });
            }
        }

        // Sort and deduplicate
        matches.sort_by(|a, b| a.display.cmp(&b.display));
        matches.dedup_by(|a, b| a.display == b.display);

        Ok((word_start, matches))
    }
}

/// Update completion state with globals from JS context
pub fn update_completions(ctx: &JSContext, state: &Rc<RefCell<CompletionState>>) {
    let Ok(mut globals) = ctx.eval::<Vec<String>>(Source::from_bytes(
        b"Object.getOwnPropertyNames(globalThis)",
    )) else {
        return;
    };

    globals.sort_unstable();
    globals.dedup();

    state.borrow_mut().globals = globals;
}

/// Create a new completion state
pub fn new_completion_state() -> Rc<RefCell<CompletionState>> {
    Rc::new(RefCell::new(CompletionState::default()))
}

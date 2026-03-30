use serde::Serialize;
use std::time::Instant;

pub type AsyncTestFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>>>>;
pub type AsyncTestFn = fn() -> AsyncTestFuture;

/// A single test entry: name + a function that runs it.
pub struct TestEntry {
    pub name: &'static str,
    pub run: TestFn,
}

/// Sync or async test function.
pub enum TestFn {
    Sync(fn() -> Result<(), String>),
    Async(AsyncTestFn),
}

/// Outcome of a single test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    Pass,
    Fail,
    Crash,
}

/// Result of running a single test.
#[derive(Debug, Serialize)]
pub struct TestCaseResult {
    pub name: String,
    pub status: TestStatus,
    pub elapsed_ms: u64,
    pub error: Option<String>,
}

/// Summary report of a full test run.
#[derive(Debug, Serialize)]
pub struct TestReport {
    pub ok: bool,
    pub filter: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub crashed: usize,
    pub elapsed_ms: u64,
    pub cases: Vec<TestCaseResult>,
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Info,
    Error,
}

/// Runs a set of tests with optional filtering and log output.
pub fn run_tests<F>(tests: &[TestEntry], filter: &str, log: F) -> TestReport
where
    F: Fn(LogLevel, &str),
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");

    let overall_start = Instant::now();
    let mut cases = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut crashed = 0usize;

    for test in tests {
        if !filter.is_empty() && !test.name.contains(filter) {
            continue;
        }

        log(LogLevel::Info, &format!("[test] START {}", test.name));
        let start = Instant::now();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match &test.run {
            TestFn::Sync(f) => f(),
            TestFn::Async(f) => rt.block_on(f()),
        }));

        let elapsed_ms = start.elapsed().as_millis() as u64;

        let case = match result {
            Ok(Ok(())) => {
                passed += 1;
                log(
                    LogLevel::Info,
                    &format!("[test] PASS  {} ({} ms)", test.name, elapsed_ms),
                );
                TestCaseResult {
                    name: test.name.to_string(),
                    status: TestStatus::Pass,
                    elapsed_ms,
                    error: None,
                }
            }
            Ok(Err(e)) => {
                failed += 1;
                log(
                    LogLevel::Error,
                    &format!("[test] FAIL  {} ({} ms): {}", test.name, elapsed_ms, e),
                );
                TestCaseResult {
                    name: test.name.to_string(),
                    status: TestStatus::Fail,
                    elapsed_ms,
                    error: Some(e),
                }
            }
            Err(panic) => {
                crashed += 1;
                let msg = extract_panic_message(&panic);
                log(
                    LogLevel::Error,
                    &format!("[test] CRASH {} ({} ms): {}", test.name, elapsed_ms, msg),
                );
                TestCaseResult {
                    name: test.name.to_string(),
                    status: TestStatus::Crash,
                    elapsed_ms,
                    error: Some(msg),
                }
            }
        };
        cases.push(case);
    }

    let total = passed + failed + crashed;
    let elapsed_ms = overall_start.elapsed().as_millis() as u64;
    log(
        LogLevel::Info,
        &format!(
            "[test] SUMMARY total={} passed={} failed={} crashed={} elapsed={}ms",
            total, passed, failed, crashed, elapsed_ms
        ),
    );

    TestReport {
        ok: failed == 0 && crashed == 0,
        filter: filter.to_string(),
        total,
        passed,
        failed,
        crashed,
        elapsed_ms,
        cases,
    }
}

fn extract_panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

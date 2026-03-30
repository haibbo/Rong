pub mod prelude;

/// Generated test modules — each corresponds to a `tests/*.rs` file.
/// Created by build.rs at compile time.
mod generated {
    include!(concat!(env!("OUT_DIR"), "/tests/mod.rs"));
}

/// Generated test registry.
mod registry {
    include!(concat!(env!("OUT_DIR"), "/registry.rs"));
}

pub use rong_test_harness::TestEntry;

/// Returns all test entries discovered from `tests/*.rs` by build.rs.
pub fn all_tests() -> Vec<TestEntry> {
    registry::all_tests()
}

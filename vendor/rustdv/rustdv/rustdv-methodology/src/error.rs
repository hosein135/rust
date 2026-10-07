//! Test failure value.
//!
//! `Err` fails the test (design-doc §0.6): `Result` for *checks*, panics for
//! *testbench bugs* (§7.3 failure taxonomy).
//!
//! This lived in `rustdv-runner` until step 4. `Component::run` returns it
//! (D46/D47), and `rustdv-methodology` sits below the runner, so the type had to
//! move down. `rustdv-runner` re-exports it, and `::rustdv::TestError`
//! resolves exactly as before.
//!
//! **The `kind` carries a machine-readable cause** so a test can declare
//! `#[rustdv::test(expect_error = "config_not_found")]` and pass only if it
//! fails *that* way (D68). Without it the runner sees a flat string and
//! `expect_fail` accepts any failure at all — including a panic from
//! somewhere unrelated. pyuvm gets this from exception *types*
//! (`expect_error=UVMConfigItemNotFound`); rustdv errors are values, so the
//! cause travels as a field.

use std::fmt;

use rustdv_sim::executor::TaskError;
use rustdv_sim::{HandleError, ValueError};

use crate::config::ConfigError;
use crate::sequence::SeqError;

#[derive(Debug, Clone)]
pub struct TestError {
    msg: String,
    kind: Option<&'static str>,
}

impl TestError {
    /// An unclassified failure — the common case for a check that failed.
    pub fn new(msg: impl Into<String>) -> TestError {
        TestError {
            msg: msg.into(),
            kind: None,
        }
    }

    /// A failure with a cause the runner can match against `expect_error`.
    /// Kinds are stable strings; see [`crate::config::ConfigError::kind`].
    pub fn with_kind(msg: impl Into<String>, kind: &'static str) -> TestError {
        TestError {
            msg: msg.into(),
            kind: Some(kind),
        }
    }

    pub fn message(&self) -> &str {
        &self.msg
    }

    /// The machine-readable cause, if this failure has one.
    pub fn kind(&self) -> Option<&'static str> {
        self.kind
    }
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl std::error::Error for TestError {}

impl From<&str> for TestError {
    fn from(s: &str) -> Self {
        TestError::new(s)
    }
}
impl From<String> for TestError {
    fn from(s: String) -> Self {
        TestError::new(s)
    }
}
impl From<HandleError> for TestError {
    fn from(e: HandleError) -> Self {
        TestError::new(e.to_string())
    }
}
impl From<ValueError> for TestError {
    fn from(e: ValueError) -> Self {
        TestError::new(e.to_string())
    }
}
impl From<SeqError> for TestError {
    fn from(e: SeqError) -> Self {
        TestError::new(e.to_string())
    }
}
impl From<TaskError> for TestError {
    fn from(e: TaskError) -> Self {
        TestError::new(e.to_string())
    }
}

/// Configuration failures keep their cause, so `?` in a test body still
/// lets `expect_error` distinguish "nothing was set" from "wrong type".
impl From<ConfigError> for TestError {
    fn from(e: ConfigError) -> Self {
        TestError::with_kind(e.to_string(), e.kind())
    }
}

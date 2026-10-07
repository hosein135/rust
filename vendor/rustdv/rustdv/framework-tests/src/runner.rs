//! `runner_` — the harness itself.
//!
//! Everything else in the regression trusts the runner to say PASS only when
//! a test passed. These tests are the ones that check the checker: a hang
//! must be reported as a timeout rather than looking like a slow test, an
//! `expect_error` must insist on the *named* cause, and each test must start
//! from a clean ConfigDb rather than inheriting the last one's state.
//!
//! Several of these are meant to fail, and pass because they failed. That
//! reads oddly in the transcript and it is the only way to test a failure
//! path from inside the thing that reports failures.

use rustdv::prelude::*;

// A test that hangs is reported as a timeout, not as a hang.
//
// Without the timeout attribute this test would run until vvp was killed and
// the regression would have no result at all — which is what "reported as a
// timeout" is worth.
#[rustdv::test(timeout_time = 100, timeout_unit = "ns", expect_fail)]
async fn runner_timeout_is_reported(_ctx: RustdvCtx) -> Result<(), TestError> {
    Timer::ns(1_000_000).await;
    Err(TestError::new("the timeout never fired"))
}

// `expect_fail` accepts any failure at all — which is exactly why
// `expect_error` exists.
#[rustdv::test(expect_fail)]
async fn runner_expect_fail_accepts_any_failure(_ctx: RustdvCtx) -> Result<(), TestError> {
    Err(TestError::new("a deliberate failure"))
}

// `expect_error` passes only when the failure carries the named cause (D68).
#[rustdv::test(expect_error = "config_not_found")]
async fn runner_expect_error_matches_the_cause(_ctx: RustdvCtx) -> Result<(), TestError> {
    // `?` on a ConfigDb miss carries the cause through into TestError.
    let _v: u32 = ConfigDb::get(None, "", "NOTHING_SET_THIS_KEY")?;
    Err(TestError::new("the missing key was found"))
}

// A panic in the test body fails the test rather than taking down the
// simulator — the task boundary catches it (§0.6, panic = "unwind").
#[rustdv::test(expect_fail)]
async fn runner_panic_fails_the_test(_ctx: RustdvCtx) -> Result<(), TestError> {
    Timer::ns(1).await;
    panic!("a deliberate panic in the test body");
}

// A panic in a *spawned* task fails the test too, even though the body
// returned Ok. cocotb's rule, and the one that stops a broken monitor from
// passing quietly in the background.
#[rustdv::test(expect_fail)]
async fn runner_background_panic_fails_the_test(_ctx: RustdvCtx) -> Result<(), TestError> {
    spawn(async {
        Timer::ns(1).await;
        panic!("a deliberate panic in a background task");
    });
    Timer::ns(10).await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Per-test freshness
// ---------------------------------------------------------------------------
//
// These two are a pair and the order matters: tests run in (file, line)
// order, so the writer below runs before the reader. pyuvm's `run_test` does
// the same clearing, and D101 leans on it now that the BFM lives in the
// ConfigDb rather than in a singleton — a test that inherited the previous
// test's BFM would inherit its half-drained queues with it.

#[rustdv::test]
async fn runner_configdb_writer(_ctx: RustdvCtx) -> Result<(), TestError> {
    ConfigDb::set(None, "*", "FRESHNESS_CANARY", 1234u32);
    let back: u32 = ConfigDb::get(None, "", "FRESHNESS_CANARY")?;
    check!(
        back == 1234,
        "the canary did not survive its own test: {back}"
    );
    Ok(())
}

#[rustdv::test]
async fn runner_configdb_is_cleared_between_tests(_ctx: RustdvCtx) -> Result<(), TestError> {
    let leaked: Result<u32, _> = ConfigDb::get(None, "", "FRESHNESS_CANARY");
    check!(
        leaked.is_err(),
        "the previous test's ConfigDb entry survived into this one: {leaked:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Logging configuration is cleared too
// ---------------------------------------------------------------------------
//
// The other half of `run_one`'s per-test reset, and the harder half to
// observe: nothing exposes the effective level for a test to read back. A
// file handler is the way in — it turns "was this message emitted?" into a
// file on disk, so both leaks become readable facts:
//
//   * a level left hostile by the previous test would suppress the second
//     test's info message, and it would be missing from the second file;
//   * a file handler left attached would receive the second test's message,
//     and it would appear in the *first* file.
//
// Like the ConfigDb pair above, these two run in (file, line) order.

fn canary_log(which: &str) -> String {
    // Process-unique so two regressions on one machine cannot read each
    // other's file, and under the temp dir so nothing lands in the repo.
    std::env::temp_dir()
        .join(format!(
            "rustdv-log-canary-{}-{}.log",
            std::process::id(),
            which
        ))
        .to_string_lossy()
        .into_owned()
}

fn read_log(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

#[rustdv::test]
async fn runner_log_config_writer(ctx: RustdvCtx) -> Result<(), TestError> {
    let first = canary_log("first");
    // An empty prefix covers every path, so this handler would catch the
    // *next* test's messages if it survived into it.
    log::add_file_for("", &first, false).map_err(|e| TestError::new(e.to_string()))?;
    ctx.info("canary-a");

    check!(
        read_log(&first).contains("canary-a"),
        "the file handler received nothing, so this test proves nothing about the next one"
    );

    // Leave the level hostile on the way out. If it survives, the next
    // test's info message is suppressed.
    log::set_level(log::Level::Critical);
    Ok(())
}

#[rustdv::test]
async fn runner_log_config_is_cleared_between_tests(ctx: RustdvCtx) -> Result<(), TestError> {
    let first = canary_log("first");
    let second = canary_log("second");
    log::add_file_for("", &second, false).map_err(|e| TestError::new(e.to_string()))?;

    ctx.info("canary-b");

    check!(
        read_log(&second).contains("canary-b"),
        "an info message was suppressed — the previous test's Critical level leaked"
    );
    check!(
        !read_log(&first).contains("canary-b"),
        "the previous test's file handler is still attached and received this test's output"
    );

    let _ = std::fs::remove_file(&first);
    let _ = std::fs::remove_file(&second);
    Ok(())
}

// Each test gets its own seed, derived from RUSTDV_RANDOM_SEED, and the same
// seed gives the same numbers — the reproducibility every transcript in the
// book rests on.
#[rustdv::test]
async fn runner_rng_is_reproducible(ctx: RustdvCtx) -> Result<(), TestError> {
    let mut a = ctx.rng();
    let mut b = ctx.rng();
    let one: Vec<u8> = (0..8).map(|_| a.u8()).collect();
    let two: Vec<u8> = (0..8).map(|_| b.u8()).collect();
    check!(
        one == two,
        "two generators from the same seed disagreed:\n  {one:?}\n  {two:?}"
    );
    check!(
        one.iter().any(|v| *v != one[0]),
        "the generator returned the same byte eight times: {one:?}"
    );
    Ok(())
}

//! Chapter 15 simulator figures. Build and run with:
//!
//!     sim-common/run_sim.sh ch15_async_await_executor playground
//!
//! Each `#[rustdv::test]` function below is a numbered figure in the book.

use rustdv::prelude::*;

rustdv::vpi_bootstrap!();

// Chapter 15, Figure 1: Hello world as a test
#[rustdv::test]
async fn hello_world(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Say hello!
    log::info("Hello, world.");
    Ok(())
}

// Chapter 15, Figure 7: Rust waits for 2 nanoseconds
#[rustdv::test]
async fn wait_2ns(_ctx: RustdvCtx) -> Result<(), TestError> {
    // Waits for two ns then prints
    Timer::ns(2).await;
    log::info("I am DONE waiting!");
    Ok(())
}

//! A misspelled port name must not compile.
//!
//! `#[derive(Component)]` generates a `PortName` constant per port field, so
//! a connection names a **constant** rather than a string. The typo that
//! pyuvm discovers at run time — if that connection is ever exercised — is
//! discovered here by the compiler, which also suggests the right spelling.

use rustdv::prelude::*;

#[derive(Component, Default)]
struct Consumer {
    #[port(get)]
    items: GetPort<u8>,
}

impl Component for Consumer {}

pub fn misspell_it() {
    let fifo: TlmFifo<u8> = TlmFifo::unbounded();
    let consumer = Consumer::default();

    // The constant is `ITEMS`. There is no `ITMES`, and there never will be
    // one by accident.
    fifo.get_export().connect(&consumer, Consumer::ITMES);
}

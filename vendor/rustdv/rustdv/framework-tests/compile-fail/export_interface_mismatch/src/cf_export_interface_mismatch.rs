//! A `get` export aimed at a `put` port must not compile.
//!
//! `PortName<I>` carries the interface in its type, so wiring an export to
//! the wrong kind of port is a type error at the connection, not a run-time
//! surprise the first time an item moves. SystemVerilog catches this at
//! elaboration and pyuvm cannot catch it at all — its connect is a name and
//! a hope.
//!
//! This is one of the claims the book makes in prose. Here it is as
//! something the compiler has to keep agreeing with.

use rustdv::prelude::*;

#[derive(Component, Default)]
struct Consumer {
    #[port(put)]
    outbox: PutPort<u8>,
}

impl Component for Consumer {}

pub fn wire_it_backwards() {
    let fifo: TlmFifo<u8> = TlmFifo::unbounded();
    let consumer = Consumer::default();

    // `get_export()` produces a `GetExport<u8>`, whose `connect` wants a
    // `PortName<dyn GetIf<u8>>`. `Consumer::OUTBOX` is a
    // `PortName<dyn PutIf<u8>>`. Nothing here can be made to line up, which
    // is the point.
    fifo.get_export().connect(&consumer, Consumer::OUTBOX);
}

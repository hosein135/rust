//! `#[port(..)]` on a field that is not a port must not compile.
//!
//! The attribute is a request for an interface, and the derive generates the
//! binding code that satisfies it. A `u32` cannot be bound to anything, so
//! the generated code has nothing to call — and the mistake is caught where
//! it was made rather than at the connection, or at the first item.

use rustdv::prelude::*;

#[derive(Component, Default)]
struct Muddled {
    // Someone meant `PutPort<u32>` and wrote the payload type instead.
    #[port(put)]
    count: u32,
}

impl Component for Muddled {}

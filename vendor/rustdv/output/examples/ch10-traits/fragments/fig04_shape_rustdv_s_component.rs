// Rust for RTL Verification — Chapter 10, Figure 4
// "The shape of rustdv's Component trait (preview — signatures only)"


pub trait Component {
    fn start(&mut self, ctx: &mut RustdvCtx) {}          // default: nothing to run
    fn check(&mut self, errors: &mut CheckSink) {}    // default: do nothing
    fn report(&self) {}                               // default: do nothing
}

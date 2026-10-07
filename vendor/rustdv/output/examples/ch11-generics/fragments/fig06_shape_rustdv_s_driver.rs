// Rust for RTL Verification — Chapter 11, Figure 6
// "The shape of rustdv's driver (preview — signatures only)"


pub struct SeqItemPort<REQ, RSP = REQ> { /* channel endpoints — elided */ }

pub struct AluDriver {                       // your driver: a plain struct...
    seq_item_port: SeqItemPort<AluCommand>,  // ...that owns a typed port
    // ...
}

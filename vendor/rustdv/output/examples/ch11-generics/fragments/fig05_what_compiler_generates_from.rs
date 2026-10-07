// Rust for RTL Verification — Chapter 11, Figure 5
// "What the compiler generates from figure 2 (conceptually — you never see this)"


fn largest_u8(list: &[u8]) -> &u8 {
    // ... same body, with T = u8 throughout
}

fn largest_u16(list: &[u16]) -> &u16 {
    // ... same body, with T = u16 throughout
}

fn largest_str(list: &[&str]) -> &&str {
    // ... same body, with T = &str throughout
}

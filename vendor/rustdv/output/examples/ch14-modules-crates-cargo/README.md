# Chapter 14 — figures

Source: `book-pdf/src/chapter-14-*.md`

| Figure | Title | Behavior | File | How to run | Try it |
|---|---|---|---|---|---|
| 1 | A module declared inline, in the middle of main.rs | runs | `src/bin/ch14_fig01_module_declared_inline_middle.rs` | `cargo run --bin ch14_fig01_module_declared_inline_middle` | [▶ playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=//%20Rust%20for%20RTL%20Verification%20%E2%80%94%20Chapter%2014%2C%20Figure%201%0A//%20%22A%20module%20declared%20inline%2C%20in%20the%20middle%20of%20main.rs%22%0A//%20Run%20with%3A%20cargo%20run%20--bin%20ch14_fig01_module_declared_inline_middle%0A//%0A//%20Expected%20output%3A%0A//%20%20%200xFF%20%2B%200x01%20%3D%200x0100%0A%0A%0Amod%20predictor%20%7B%0A%20%20%20%20%23%5Bderive%28Clone%2C%20Copy%2C%20Debug%2C%20PartialEq%29%5D%0A%20%20%20%20pub%20enum%20Ops%20%7B%20Add%20%3D%201%2C%20And%20%3D%202%2C%20Xor%20%3D%203%2C%20Mul%20%3D%204%20%7D%0A%0A%20%20%20%20pub%20fn%20alu_prediction%28a%3A%20u8%2C%20b%3A%20u8%2C%20op%3A%20Ops%29%20-%3E%20u16%20%7B%0A%20%20%20%20%20%20%20%20match%20op%20%7B%0A%20%20%20%20%20%20%20%20%20%20%20%20Ops%3A%3AAdd%20%3D%3E%20a%20as%20u16%20%2B%20b%20as%20u16%2C%0A%20%20%20%20%20%20%20%20%20%20%20%20Ops%3A%3AAnd%20%3D%3E%20%28a%20%26%20b%29%20as%20u16%2C%0A%20%20%20%20%20%20%20%20%20%20%20%20Ops%3A%3AXor%20%3D%3E%20%28a%20%5E%20b%29%20as%20u16%2C%0A%20%20%20%20%20%20%20%20%20%20%20%20Ops%3A%3AMul%20%3D%3E%20a%20as%20u16%20%2A%20b%20as%20u16%2C%0A%20%20%20%20%20%20%20%20%7D%0A%20%20%20%20%7D%0A%7D%0A%0Afn%20main%28%29%20%7B%0A%20%20%20%20let%20sum%20%3D%20predictor%3A%3Aalu_prediction%280xFF%2C%200x01%2C%20predictor%3A%3AOps%3A%3AAdd%29%3B%0A%20%20%20%20println%21%28%220xFF%20%2B%200x01%20%3D%20%7Bsum%3A%2306x%7D%22%29%3B%0A%7D%0A) |
| 2 | use brings names into scope, like Python's from-import | runs | `src/bin/ch14_fig02_use_brings_names_into.rs` | `cargo run --bin ch14_fig02_use_brings_names_into` | [▶ playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=//%20Rust%20for%20RTL%20Verification%20%E2%80%94%20Chapter%2014%2C%20Figure%202%0A//%20%22use%20brings%20names%20into%20scope%2C%20like%20Python%27s%20from-import%22%0A//%20Run%20with%3A%20cargo%20run%20--bin%20ch14_fig02_use_brings_names_into%0A//%0A//%20Expected%20output%3A%0A//%20%20%20AND%3A%200x0030%0A//%20%20%20XOR%3A%200x00cc%0A%0A%0Amod%20predictor%20%7B%0A%20%20%20%20%23%5Bderive%28Clone%2C%20Copy%2C%20Debug%2C%20PartialEq%29%5D%0A%20%20%20%20pub%20enum%20Ops%20%7B%20Add%20%3D%201%2C%20And%20%3D%202%2C%20Xor%20%3D%203%2C%20Mul%20%3D%204%20%7D%0A%0A%20%20%20%20pub%20fn%20alu_prediction%28a%3A%20u8%2C%20b%3A%20u8%2C%20op%3A%20Ops%29%20-%3E%20u16%20%7B%0A%20%20%20%20%20%20%20%20match%20op%20%7B%0A%20%20%20%20%20%20%20%20%20%20%20%20Ops%3A%3AAdd%20%3D%3E%20a%20as%20u16%20%2B%20b%20as%20u16%2C%0A%20%20%20%20%20%20%20%20%20%20%20%20Ops%3A%3AAnd%20%3D%3E%20%28a%20%26%20b%29%20as%20u16%2C%0A%20%20%20%20%20%20%20%20%20%20%20%20Ops%3A%3AXor%20%3D%3E%20%28a%20%5E%20b%29%20as%20u16%2C%0A%20%20%20%20%20%20%20%20%20%20%20%20Ops%3A%3AMul%20%3D%3E%20a%20as%20u16%20%2A%20b%20as%20u16%2C%0A%20%20%20%20%20%20%20%20%7D%0A%20%20%20%20%7D%0A%7D%0A%0Ause%20predictor%3A%3A%7Balu_prediction%2C%20Ops%7D%3B%0A%0Afn%20main%28%29%20%7B%0A%20%20%20%20println%21%28%22AND%3A%20%7B%3A%2306x%7D%22%2C%20alu_prediction%280xF0%2C%200x3C%2C%20Ops%3A%3AAnd%29%29%3B%0A%20%20%20%20println%21%28%22XOR%3A%20%7B%3A%2306x%7D%22%2C%20alu_prediction%280xF0%2C%200x3C%2C%20Ops%3A%3AXor%29%29%3B%0A%7D%0A) |
| 3 | Private by default — the underscore convention, enforced | **compile error on purpose** (E0603) | `compile-fail/fig03_private_default_underscore_convention/src/main.rs` | `cd compile-fail/fig03_private_default_underscore_convention && cargo build` — expect E0603 | [▶ playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=//%20Rust%20for%20RTL%20Verification%20%E2%80%94%20Chapter%2014%2C%20Figure%203%0A//%20%22Private%20by%20default%20%E2%80%94%20the%20underscore%20convention%2C%20enforced%22%0A//%20NOTE%3A%20this%20figure%20FAILS%20TO%20COMPILE%20ON%20PURPOSE%20%E2%80%94%20the%20error%20is%20the%20lesson.%0A//%20Build%20it%20and%20read%20the%20error%3A%20cargo%20build%20%20%20%28see%20EXPECTED.txt%29%0A//%0A//%20Expected%20compiler%20error%3A%0A//%20%20%20error%5BE0603%5D%3A%20function%20%60widen%60%20is%20private%0A//%20%20%20%20%20--%3E%20src/main.rs%3A20%3A24%0A//%20%20%20%20%20%20%7C%0A//%20%20%2020%20%7C%20%20%20%20%20let%20w%20%3D%20predictor%3A%3Awiden%280xFF%29%3B%20%20//%20reaching%20for%20a%20private%20helper%0A//%20%20%20%20%20%20%7C%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%20%5E%5E%5E%5E%5E%20private%20function%0A//%20%20%20%20%20%20%7C%0A//%20%20%20note%3A%20the%20function%20%60widen%60%20is%20defined%20here%0A//%20%20%20%20%20--%3E%20src/main.rs%3A5%3A5%0A//%20%20%20%20%20%20%7C%0A//%20%20%205%20%20%7C%20%20%20%20%20fn%20widen%28x%3A%20u8%29%20-%3E%20u16%20%7B%20%20%20%20%20//%20no%20pub%3A%20private%20to%20this%20module%0A//%20%20%20%20%20%20%7C%20%20%20%20%20%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%5E%0A%0A%0Amod%20predictor%20%7B%0A%20%20%20%20pub%20enum%20Ops%20%7B%20Add%20%3D%201%2C%20And%20%3D%202%2C%20Xor%20%3D%203%2C%20Mul%20%3D%204%20%7D%0A%0A%20%20%20%20fn%20widen%28x%3A%20u8%29%20-%3E%20u16%20%7B%20%20%20%20%20//%20no%20pub%3A%20private%20to%20this%20module%0A%20%20%20%20%20%20%20%20x%20as%20u16%0A%20%20%20%20%7D%0A%0A%20%20%20%20pub%20fn%20alu_prediction%28a%3A%20u8%2C%20b%3A%20u8%2C%20op%3A%20Ops%29%20-%3E%20u16%20%7B%0A%20%20%20%20%20%20%20%20match%20op%20%7B%0A%20%20%20%20%20%20%20%20%20%20%20%20Ops%3A%3AAdd%20%3D%3E%20widen%28a%29%20%2B%20widen%28b%29%2C%0A%20%20%20%20%20%20%20%20%20%20%20%20Ops%3A%3AAnd%20%3D%3E%20widen%28a%20%26%20b%29%2C%0A%20%20%20%20%20%20%20%20%20%20%20%20Ops%3A%3AXor%20%3D%3E%20widen%28a%20%5E%20b%29%2C%0A%20%20%20%20%20%20%20%20%20%20%20%20Ops%3A%3AMul%20%3D%3E%20widen%28a%29%20%2A%20widen%28b%29%2C%0A%20%20%20%20%20%20%20%20%7D%0A%20%20%20%20%7D%0A%7D%0A%0Afn%20main%28%29%20%7B%0A%20%20%20%20let%20w%20%3D%20predictor%3A%3Awiden%280xFF%29%3B%20%20//%20reaching%20for%20a%20private%20helper%0A%20%20%20%20println%21%28%22%7Bw%7D%22%29%3B%0A%7D%0A) |
| 4 | The file-to-module mapping | shell transcript (see below) | `(chapter README)` | shell transcript | — |
| 5 | Adding a dependency with cargo add | shell transcript (see below) | `(chapter README)` | shell transcript | — |
| 6 | Unit tests live beside the code they test | unit tests (`cargo test`) | `src/lib.rs` | `cargo test -p ch14_modules_crates_cargo` | — |
| 7 | Running the unit tests | shell transcript (see below) | `(chapter README)` | shell transcript | — |
| 8 | A failing test names the culprit | shell transcript (see below) | `(chapter README)` | shell transcript | — |

## Shell-transcript figures

### Figure 4: The file-to-module mapping

```text

alu_playground/
├── Cargo.toml
└── src/
    ├── main.rs          <-- contains the line: mod predictor;
    └── predictor.rs     <-- the body of the module: Ops, alu_prediction
--
% cargo run
   Compiling alu_playground v0.1.0
    Finished `dev` profile
     Running `target/debug/alu_playground`
AND: 0x0030
XOR: 0x00cc
```

### Figure 5: Adding a dependency with cargo add

```text

% cargo add rand
    Updating crates.io index
      Adding rand v0.9.1 to dependencies
--
# Cargo.toml, after:

[package]
name = "alu_playground"
version = "0.1.0"
edition = "2024"

[dependencies]
rand = "0.9.1"
```

### Figure 7: Running the unit tests

```text

% cargo test
   Compiling alu_playground v0.1.0
    Finished `test` profile [unoptimized + debuginfo]
     Running unittests src/main.rs
--
running 4 tests
test predictor::tests::add_carries_into_bit_eight ... ok
test predictor::tests::and_masks_operands ... ok
test predictor::tests::mul_needs_the_full_result_bus ... ok
test predictor::tests::xor_finds_differing_bits ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Figure 8: A failing test names the culprit

```text

% cargo test
--
running 4 tests
test predictor::tests::add_carries_into_bit_eight ... FAILED
test predictor::tests::and_masks_operands ... ok
test predictor::tests::mul_needs_the_full_result_bus ... ok
test predictor::tests::xor_finds_differing_bits ... ok

failures:

---- predictor::tests::add_carries_into_bit_eight stdout ----
assertion `left == right` failed
  left: 254
 right: 510

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured
```

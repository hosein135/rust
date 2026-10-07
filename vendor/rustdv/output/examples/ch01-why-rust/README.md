# Chapter 1 — figures

Source: `book-pdf/src/chapter-01-*.md`

| Figure | Title | Behavior | File | How to run | Try it |
|---|---|---|---|---|---|
| 1 | Creating our first program | shell transcript (see below) | `(chapter README)` | shell transcript | — |
| 2 | The classic first program | runs | `src/bin/ch01_fig02_classic_first_program.rs` | `cargo run --bin ch01_fig02_classic_first_program` | [▶ playground](https://play.rust-lang.org/?version=stable&mode=debug&edition=2024&code=//%20Rust%20for%20RTL%20Verification%20%E2%80%94%20Chapter%201%2C%20Figure%202%0A//%20%22The%20classic%20first%20program%22%0A//%20Run%20with%3A%20cargo%20run%20--bin%20ch01_fig02_classic_first_program%0A//%0A//%20Expected%20output%3A%0A//%20%20%20%25%20cargo%20run%0A//%20%20%20%20%20%20Compiling%20hello%20v0.1.0%0A//%20%20%20%20%20%20%20Finished%20%60dev%60%20profile%0A//%20%20%20%20%20%20%20%20Running%20%60target/debug/hello%60%0A//%20%20%20Hello%2C%20world.%0A//%20%20%20--%0A//%20%20%20Hello%2C%20world.%0A%0A%0Afn%20main%28%29%20%7B%0A%20%20%20%20println%21%28%22Hello%2C%20world.%22%29%3B%0A%7D%0A) |

## Shell-transcript figures

### Figure 1: Creating our first program

```text

% cargo new hello
    Creating binary (application) `hello` package
```

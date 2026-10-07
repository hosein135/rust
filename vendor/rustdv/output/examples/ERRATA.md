# Errata found while making the figures runnable

**Status: all three fixed in `book-pdf/src` on 2026-07-08.** The examples
now match the manuscript verbatim again (no marked deviations remain). Kept
as a record of what changed and why. Remember to rebuild the book
(`mdbook build`) and regenerate the PDF.

## 1. Chapter 9, Figure 2 — the divide-by-zero doesn't panic; it doesn't compile

The book shows `let divisor = 0;` followed by `3 / divisor` panicking at
runtime. rustc const-propagates the zero and rejects the program at *compile
time* with `error: this operation will panic at runtime` (the deny-by-default
`unconditional_panic` lint).

That is arguably an even better story for this chapter (the compiler catches
what Python/SystemVerilog leave to runtime), but the shown transcript is
wrong for the shown code. Options for the book: (a) keep the code and show
the compile error instead, or (b) make the divisor opaque so the panic really
happens — the example uses `let divisor: i32 = "0".parse().unwrap();`
(`parse` was introduced in Chapter 3, Figure 8).

## 2. Chapter 12, Figure 6 — `ii.pow(3)` fails type inference (E0689)

Despite the `HashMap<u32, u32>` annotation on `cubes`, rustc cannot resolve
`.pow` on the closure parameter: method calls need the receiver's concrete
type at check time, and inference hasn't flowed back through
`collect()`/`map` yet. The figure as printed fails with `error[E0689]: can't
call method pow on ambiguous numeric type {integer}`.

Fix for the book: annotate the closure parameter — `.map(|ii: u32| (ii,
ii.pow(3)))` — or write the range as `(0u32..4)`. The example uses the former.

## 3. Chapter 8, Figures 1 and 4 — the printed Debug output is wrong

The transcripts show `op: Ops::Add`, but `#[derive(Debug)]` prints enum
variants *without* the type path — the real output is `op: Add`. Three lines
to fix in `chapter-08-collections.md`, all in transcripts, none in code:

- line 43 (Figure 1's output): `first: AluCommand { a: 5, b: 3, op: Ops::Add }`
- lines 140–141 (Figure 4's output): `op: Ops::Add` and `op: Ops::Mul`

Everything else that writes `Ops::` is correct: source code constructs
variants with the full path, and rustc itself prints full paths in error
messages (Figure 2's E0382 quote, Chapter 7's E0004 `Ops::Sub`). Figure 7's
transcript (`Mul: 2`) and Chapter 10 Figure 6's (`op=Xor`) already have it
right.

## Not errata, but noted

- Chapters 4 and 12 figures that print with `print!("{} ", n)` emit a
  trailing space the book's transcripts trim. Harmless; `check.sh` ignores it.
- Several figures produce `dead_code` warnings (unused enum variants/fields).
  Expected — the figures define more than they use.

## 4. Ch 10 Fig 4 / Ch 11 Fig 6 — previews updated to match the implemented rustdv (2026-07-11)

When rustdv was implemented (see `/STATUS.md`), two Part I previews drifted
from the real API and were corrected in the book source and here:

- **Ch 10 Fig 4** (`Component` trait): `start` was shown as a *required*
  method. In the implementation every lifecycle method has a default empty
  body — components override only what they use (pyuvm's no-op-phase
  pattern) — so a structural env can `impl Component` with an empty block.
- **Ch 11 Fig 6** (driver preview): shown as a library-provided generic
  `Driver<REQ, RSP>` struct. The implementation follows the review-memo's
  mechanism/methodology split: the *port* is the library's generic type
  (`SeqItemPort<REQ, RSP = REQ>`); your driver is an ordinary struct that
  owns one. The figure now shows both lines.

Both are `fragments/` (non-runnable previews), so no build behavior changed.

## 5. Book-wide: figure captions in rust blocks were invisible when rendered (2026-07-11)

Figure captions were written as `# Figure N: ...` inside ```rust code
blocks. `#` is not a Rust comment — and worse, mdBook treats `#`-prefixed
lines in rust blocks as *hidden* ("boring") lines, so every rust figure's
caption was missing from the rendered HTML/PDF even though the prose
references the numbers. All 106 rust-block captions across the 14 chapters
and the Interlude are now `// Figure N: ...` (a real, visible Rust
comment). Captions in ```text and ```python blocks keep `#`, which is
visible and idiomatic there. `regression/regress.py`'s FIG_RE accepts both
markers; book-sync passes 107/107 after the change. Reported by Ray.

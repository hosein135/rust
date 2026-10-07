# Building *Rust for RTL Verification*

This directory is an [mdBook](https://rust-lang.github.io/mdBook/) project.
The chapter sources live in `src/` (ordered by `src/SUMMARY.md`); everything
under `book/` is generated output and is gitignored — never edit it by hand.

## Prerequisites

Both tools install through cargo, so you need a Rust toolchain first
(the one in `output/environment-setup.md` works):

```sh
cargo install mdbook       # the book builder
cargo install mdbook-pdf   # the PDF backend
```

**Offline / sandbox VM:** `cargo install` needs the network, which the VM does
not have. `mdbook` ships prebuilt in `toolchain-drop/` instead:

```sh
tar -xzf toolchain-drop/mdbook-v*.tar.gz -C /tmp/rust/bin mdbook
```

That gives the HTML build, which is enough to check `SUMMARY.md` and catch an
orphaned chapter. `mdbook-pdf` is *not* available there — it drives a headless
Chromium the VM lacks — so `[output.pdf]` is marked `optional = true` in
`book.toml` and is skipped with a warning rather than failing the build.

`mdbook-pdf` produces the PDF by driving a headless Chrome/Chromium, so a
Chrome or Chromium installation must be present on the machine.

## Build

```sh
cd book-pdf
mdbook build
```

Because two backends are configured in `book.toml` (`[output.html]` and
`[output.pdf]`), the outputs land in per-backend subdirectories:

| Output | Location |
|---|---|
| Website | `book/html/index.html` |
| PDF | `book/pdf/rustdvbook.pdf` |

## Live preview while writing

```sh
cd book-pdf
mdbook serve --open
```

This rebuilds on every save and refreshes the browser. `serve` runs only
the HTML backend — run `mdbook build` when you want a fresh PDF.

## Styling

- `theme/` — mdBook theme overrides (index.hbs, css, fonts, favicons).
- `github-markdown.css` — additional stylesheet listed in `book.toml`.

## Adding a chapter

1. Create `src/chapter-NN-short-name.md`.
2. Add it to `src/SUMMARY.md` in reading order — a chapter missing from
   SUMMARY.md is not built.
3. `mdbook build`.

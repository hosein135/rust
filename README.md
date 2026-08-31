# Verilog IDE

Desktop IDE for **Verilog** HDL and **testbenches**, written in Rust with [iced](https://github.com/iced-rs/iced) (CPU software renderer — no GPU required).

## Features

- Project explorer for `.v` / `.sv` / `.vh` files
- Multi-tab code editor with line numbers and syntax highlighting
- New module / new testbench templates
- Sample counter + testbench under `samples/`
- **Run** button (F5): simulate Verilog + testbench with [xezim](https://github.com/aionhw/xezim) and write a `.vcd` waveform
- **Waveform / Text Editor** modes: click a `.vcd` to view traces (via [wellen](https://github.com/ekiwi/wellen) from [Surfer](https://gitlab.com/surfer-project/surfer)) or the dump as text
- Console + problems panel, find, save shortcuts

## Setup and run (Linux / macOS / WSL)

```bash
chmod +x run.sh
./run.sh
```

What `run.sh` does (same pattern as [jadex_django/run.sh](../jadex_django/run.sh)):

1. Install **curl** (static binary) if missing
2. Install **Nix** (official installer) if missing
3. Enter a **nixpkgs 25.05** dev shell from `devops/flake.nix` (Rust **1.92.0**, iced + xezim + wellen/Surfer build deps)
4. Cache the Nix environment under `~/.cache/verilog-ide/` for fast later runs
5. `cargo run` the IDE inside that shell

## Reproducibility

- **Nix**: `devops/flake.lock` pins nixpkgs and rust-overlay; the shell uses Rust **1.92.0** exactly
- **Cargo**: committed `Cargo.lock` pins crates.io; xezim and Surfer are vendored under `vendor/` so `cargo build` does not fetch their git repos
- **Toolchain file**: `rust-toolchain.toml` selects 1.92.0 for rustup-based builds outside Nix

Useful flags:

| Flag | Meaning |
|------|---------|
| `--prep-only` | Install/cache Nix env only; do not launch |
| `--build` | `cargo build` only (no run) |
| `--release` | Release build / run |
| `--force-setup` | Re-fetch Nix packages into the system cache |

```bash
./run.sh --prep-only
./run.sh --release
./run.sh --build --release
./run.sh --force-setup
```

## Simulate (VCD)

Open a folder that contains RTL plus a testbench (`*_tb.v`), then click **▶ Run** (or press **F5**).

The IDE saves dirty files and runs the bundled [xezim](https://github.com/aionhw/xezim) simulator **in-process** (vendored at `vendor/xezim`, compiled with `cargo build` / `cargo run`). Waveform support is enabled the same way as xezim `--wave`. The sample `samples/counter_tb.v` already calls `$dumpfile` / `$dumpvars`; the waveform lands at `counter.vcd` in the project folder.

## Waveforms (Surfer / wellen)

[Surfer](https://gitlab.com/surfer-project/surfer) is vendored at `vendor/surfer` (`v0.7.0`). Its parser, [wellen](https://github.com/ekiwi/wellen), is a Cargo dependency and is compiled with the rest of the crate (same pattern as xezim).

Use **View → Waveform** or the **Wave** chip in the menu bar, then click a `.vcd` (also `.fst` / `.ghw`) in the explorer. Traces open in the editor pane (scroll to zoom, drag to pan, click to place a cursor).

Use **View → Text Editor** or the **Text** chip, then click the same file to see the dump as text.

After **▶ Run**, if Waveform mode is on, the new `.vcd` opens as traces automatically.

## Manual run (inside Nix shell)

```bash
nix develop devops
cargo run
# or
cargo run --release
```

## Layout

```
src/              Rust IDE sources (iced)
src/sim.rs        in-process xezim runner (VCD)
src/waveform.rs   Surfer/wellen waveform pane
vendor/xezim      Vendored [xezim](https://github.com/aionhw/xezim) (no git fetch)
vendor/xezim-core Vendored xezim-core (xezim path/patch dep)
vendor/surfer     Vendored [Surfer](https://gitlab.com/surfer-project/surfer) v0.7.0
samples/          Example counter + testbench
Cargo.lock        Pinned Rust crate graph
rust-toolchain.toml  Rust 1.92.0 (rustup)
devops/flake.nix  Nix dev shell (nixpkgs 25.05)
devops/flake.lock Pinned Nix inputs
run.sh            Bootstrap Nix + run inside flake env
```

## GUI stack

- [iced](https://github.com/iced-rs/iced) with the **tiny-skia** software renderer (CPU-only, no Vulkan/OpenGL/nixGL)
- `ICED_BACKEND=tiny-skia` is set in `.cargo/config.toml` and the Nix shell hook

Works on non-NixOS Linux without [nixGL](https://github.com/nix-community/nixGL) or GPU drivers.

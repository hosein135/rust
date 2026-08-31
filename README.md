# Verilog IDE

Desktop IDE for **Verilog** HDL and **testbenches**, written in Rust with [iced](https://github.com/iced-rs/iced) (CPU software renderer — no GPU required).

## Features

- Project explorer for `.v` / `.sv` / `.vh` files
- Multi-tab code editor with line numbers and syntax highlighting
- New module / new testbench templates
- Sample counter + testbench under `samples/`
- **Run** button (F5): simulate Verilog + testbench with [xezim](https://github.com/aionhw/xezim) and write a `.vcd` waveform
- Console + problems panel, find, save shortcuts

## Setup and run (Linux / macOS / WSL)

```bash
chmod +x run.sh
./run.sh
```

What `run.sh` does (same pattern as [jadex_django/run.sh](../jadex_django/run.sh)):

1. Install **curl** (static binary) if missing
2. Install **Nix** (official installer) if missing
3. Enter a **nixpkgs 25.05** dev shell from `devops/flake.nix` (Rust **1.92**, iced + xezim build deps)
4. Cache the Nix environment under `~/.cache/verilog-ide/` for fast later runs
5. `cargo run` the IDE inside that shell

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

The IDE saves dirty files and runs the bundled [xezim](https://github.com/aionhw/xezim) simulator **in-process** (Cargo git dependency, compiled with `cargo build` / `cargo run`). Waveform support is enabled the same way as xezim `--wave`. The sample `samples/counter_tb.v` already calls `$dumpfile` / `$dumpvars`; the waveform lands at `counter.vcd` in the project folder.

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
samples/          Example counter + testbench
devops/flake.nix  Nix dev shell (nixpkgs 25.05)
run.sh            Bootstrap Nix + run inside flake env
```

## GUI stack

- [iced](https://github.com/iced-rs/iced) with the **tiny-skia** software renderer (CPU-only, no Vulkan/OpenGL/nixGL)
- `ICED_BACKEND=tiny-skia` is set in `.cargo/config.toml` and the Nix shell hook

Works on non-NixOS Linux without [nixGL](https://github.com/nix-community/nixGL) or GPU drivers.

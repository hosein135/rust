# Verilog IDE

Desktop IDE for **Verilog** HDL and **testbenches**, written in Rust (`egui` / `eframe`).

## Features

- Project explorer for `.v` / `.sv` / `.vh` files
- Multi-tab editor with Verilog syntax highlighting
- New module / new testbench templates
- Sample counter + testbench under `samples/`
- Console + problems panel, find, save shortcuts (`Ctrl+S`, `Ctrl+O`, `Ctrl+F`)

## Toolchain (pinned)

See [`.vfox.toml`](.vfox.toml):

```toml
[tools]
rust = "1.98.0"
```

`vfox-run.ps1` installs **only this single Rust stable** (current stable release). Downloads show a live progress bar (bytes, %, speed). Failed downloads retry the same version — there is no multi-version fallback.
## Windows: setup and run

Double-click `run.cmd`, or from an **elevated** PowerShell:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\vfox-run.ps1
```

What the script does (winget → vfox → rust stack):

1. Install **winget** only if missing (AppX deps: VCLibs, UI.Xaml, DesktopAppInstaller)
2. Install **vfox** via winget if missing
3. `vfox add rust` + download/install Rust **1.98.0** from `.vfox.toml` (progress bar + retries)
4. Install **VS 2022 Build Tools** (MSVC) if `link.exe` is missing
5. `cargo run` the IDE

Useful flags:

| Flag | Meaning |
|------|---------|
| `-PrepOnly` | Install toolchain only; do not launch |
| `-Build` | `cargo build` then run |
| `-Release` | Release build / run |
| `-ForceSetup` | Reinstall / re-download even if present |

```powershell
.\vfox-run.ps1 -PrepOnly
.\vfox-run.ps1 -Release
```

## Manual run (after toolchain is ready)

```powershell
vfox activate pwsh
vfox use -p rust@1.98.0
cargo run
```

## Layout

```
src/           Rust IDE sources
samples/       Example counter + testbench
.vfox.toml     Pinned Rust version for vfox
vfox-run.ps1   Windows bootstrap + run
run.cmd        CMD launcher for vfox-run.ps1
```

# Environment Setup for the rustdv Implementation Sprint

**Goal:** give Claude's Cowork sandbox what it needs to write the complete
rustdv code base, compile it, and run it against the TinyALU on Icarus
Verilog — autonomously, without waiting on you.

**Why this is needed:** Cowork runs shell commands in an isolated Linux VM.
Today that VM has **no Rust toolchain and no Icarus**, has **no root access**
(so `apt install` is impossible), and its network allowlist **blocks every
install source** Claude tried: crates.io, rustup, PyPI, npm, GitHub, conda,
and the Ubuntu archives. Tools installed on your Mac don't help — the VM is a
separate machine.

**Sandbox facts that matter** (probed 2026-07-11):

| Fact | Value | Consequence |
|---|---|---|
| Architecture | **aarch64** (Apple Silicon) | all downloads must be arm64/aarch64 builds |
| OS | Ubuntu 22.04.5 | standard glibc; prebuilt Linux binaries work |
| Root/sudo | none | everything must install to the home directory |
| Compilers present | gcc, g++, make | but **no flex/bison/gperf/autoconf** → building Icarus from source won't work; prebuilt binaries required |
| Disk | ~9 GB free in home | enough for Rust (~1.5 GB) + oss-cad-suite (~2 GB) |

Pick **one** of the three paths below. Path A is the least work for you.

---

## Path A (recommended): open network access for the sandbox

Claude can then install everything itself, in user space, with no further
help from you.

1. In a **web browser**, go to **claude.ai → Settings → Capabilities**
   (direct link: <https://claude.ai/settings/capabilities>) and find the
   internet / network access control for code execution. This setting is on
   the claude.ai website, **not** in the desktop app's local settings —
   Cowork's sandbox follows whatever egress policy you set there. (On
   Team/Enterprise plans the same control is owner-managed under
   Organization settings → Capabilities.)
2. Either select the broadest access level you're comfortable with, or
   allowlist these specific domains:

   | Purpose | Domains |
   |---|---|
   | Rust toolchain (rustup) | `sh.rustup.rs`, `static.rust-lang.org` |
   | Icarus Verilog (prebuilt oss-cad-suite from GitHub releases) | `github.com`, `api.github.com`, `codeload.github.com`, `objects.githubusercontent.com`, `release-assets.githubusercontent.com` |
   | cargo dependencies (only if rustdv ever takes external deps — the plan is a zero-dependency workspace, so this row is optional insurance) | `crates.io`, `index.crates.io`, `static.crates.io` |

3. **Start a new conversation.** Per Anthropic's docs, network-setting
   changes do **not** apply to sessions that are already open — they take
   effect only in sessions created afterward.
4. Reconnect the `rustdv` project folder in the new conversation.
5. Paste the kickoff prompt from the bottom of this document.

You can tighten the network setting back down after the toolchain is
installed — the VM persists per session, but a fresh session needs to
reinstall, so leaving at least these domains open avoids repeated setup.

---

## Path B: offline toolchain drop (no settings change)

You download two files on your Mac, drop them into the project folder, and
Claude installs from there. No network change, fully offline in the VM.

1. Create a folder `toolchain-drop/` at the root of the rustdv project
   (Claude will add it to `.gitignore`).
2. Download these files into it (**arm64/aarch64 builds — the VM is not
   x86**):
   - **Rust standalone installer:**
     `https://static.rust-lang.org/dist/rust-1.89.0-aarch64-unknown-linux-gnu.tar.xz`
     (any recent stable version works; keep the `aarch64-unknown-linux-gnu`
     part; ~450 MB). If the exact version 404s, browse
     `https://forge.rust-lang.org/infra/other-installation-methods.html`
     for the current standalone-installer link.
   - **oss-cad-suite (contains Icarus Verilog + Verilator, prebuilt):**
     the latest `oss-cad-suite-linux-arm64-<date>.tgz` asset from
     `https://github.com/YosysHQ/oss-cad-suite-build/releases/latest`
     (~1.5 GB).
   - **mdbook (builds/verifies the book in `book-pdf/`):**
     `mdbook-v0.5.4-aarch64-unknown-linux-musl.tar.gz` from
     `https://github.com/rust-lang/mdBook/releases/latest` (~5 MB). Already
     committed in `toolchain-drop/`. Note: this builds the book's **HTML**;
     the PDF backend (`mdbook-pdf`) also needs a Chromium, which is not
     installed in the VM — render the PDF on a full workstation with
     `mdbook build book-pdf`.
3. Tell Claude the files are there. Installation from local tarballs needs
   no network and no root; Claude handles it.

Note: Path B commits us to a **zero-external-dependency** rustdv (std-only,
no crates.io) because cargo can't fetch anything. That is the current plan
anyway — it's a feature for the book (nothing to install, no magic) — but it
does rule out later convenience deps without revisiting setup.

---

## Path C (fallback): you run the builds

No setup at all: Claude writes the entire code base, build scripts, and CI;
you run `cargo build` / `sim/run_smoke.sh icarus` in the repo's devcontainer
or Codespaces (which already has Rust + Icarus) and paste the output back.
Claude iterates off your logs. This works but turns a minutes-long
compile-fix loop into a per-message round trip — expect it to be much slower
and to consume more of the five-hour usage windows for the same progress.

---

## How Claude verifies the environment (start of next session)

```sh
rustc --version && cargo --version      # expect 1.7x+, aarch64
iverilog -V | head -1 && vvp -V | head -1
cd sim && ./run_smoke.sh icarus         # expect "SMOKE: PASS"
```

If all three pass, no further questions — implementation starts immediately.

---

## Kickoff prompt for the new session

> Connect the rustdv project folder. Environment is set up per
> `output/environment-setup.md` (Path A/B — say which). Verify the
> toolchain, then implement the complete rustdv code base per
> `output/.design-doc.md` and demonstrate it against the TinyALU in
> `sim/hdl/tinyalu.sv` on Icarus: full testbench (BFM, driver, monitors,
> scoreboard, coverage, env, sequences, tests) per design-doc §7. Work
> autonomously; document deviations from the design doc in a STATUS.md;
> only stop for must-answer questions or usage limits.

---

*Sources: [Claude Cowork architecture overview](https://support.claude.com/en/articles/14479288-claude-cowork-architecture-overview) (sandbox isolation, egress proxy, allowlist), [Use Claude Cowork safely](https://support.claude.com/en/articles/13364135-use-claude-cowork-safely) (network egress settings and scope). Sandbox facts verified by direct probing in-session.*

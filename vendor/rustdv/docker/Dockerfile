# Reproducible environment for Rust for RTL Verification:
# Rust toolchain + Icarus Verilog + Verilator + Python.
#
# Used by the Codespaces devcontainer (.devcontainer/devcontainer.json)
# and for local work:
#
#   docker build -t rustdv docker/
#   docker run -it --rm -v "$PWD":/work -w /work rustdv bash
#   python3 output/regression/regress.py
#
# CI installs the same tools via apt directly (see .github/workflows/ci.yml)
# so there is no image-registry dependency; keep the two lists in sync.

FROM rust:1-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
        iverilog \
        verilator \
        python3 \
        git \
    && rm -rf /var/lib/apt/lists/*

# Sanity: every tool the regression system reaches for.
RUN cargo --version && rustc --version && iverilog -V | head -1 \
    && verilator --version && python3 --version

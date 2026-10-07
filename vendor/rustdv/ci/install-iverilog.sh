#!/usr/bin/env bash
# Build the Icarus release used by the simulator regression.
# Installs into a caller-owned prefix, so CI and containers need no sudo.
set -euo pipefail

VERSION=13.0
TAG=v13_0
SHA256=c897bbfa9848688982c6d5c30529fc29d68df0b9ff22ffa73bad89db73a7ce49
PREFIX="${1:-/tmp/rustdv-$(id -u)/iverilog-$VERSION}"

if [ -x "$PREFIX/bin/iverilog" ]; then
    installed="$("$PREFIX/bin/iverilog" -V 2>/dev/null | sed -n 's/^Icarus Verilog version \([^ ]*\).*/\1/p')"
    if [ "$installed" = "$VERSION" ]; then
        echo "Icarus Verilog $VERSION already installed at $PREFIX"
        exit 0
    fi
fi

missing=()
for tool in curl tar autoconf make flex bison gperf; do
    command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done
if [ "${#missing[@]}" -gt 0 ]; then
    echo "Icarus Verilog build dependencies missing: ${missing[*]}" >&2
    exit 2
fi

SCRATCH="/tmp/rustdv-$(id -u)/iverilog-install"
ARCHIVE="$SCRATCH/iverilog-v$VERSION.tar.gz"
mkdir -p "$SCRATCH" "$PREFIX"

if [ ! -f "$ARCHIVE" ]; then
    curl --fail --location --silent --show-error \
        "https://github.com/steveicarus/iverilog/archive/refs/tags/$TAG.tar.gz" \
        --output "$ARCHIVE"
fi

if command -v shasum >/dev/null 2>&1; then
    actual="$(shasum -a 256 "$ARCHIVE" | awk '{print $1}')"
elif command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$ARCHIVE" | awk '{print $1}')"
else
    echo "Icarus Verilog installer needs shasum or sha256sum" >&2
    exit 2
fi
if [ "$actual" != "$SHA256" ]; then
    echo "Icarus Verilog archive checksum mismatch: got $actual, expected $SHA256" >&2
    exit 1
fi

SOURCE="$(mktemp -d "$SCRATCH/source.XXXXXX")"
trap 'rm -rf "$SOURCE"' EXIT
tar -xzf "$ARCHIVE" -C "$SOURCE" --strip-components=1

jobs="${ICARUS_BUILD_JOBS:-2}"
(
    cd "$SOURCE"
    # The release driver uses _NSGetExecutablePath without including its
    # declaration. Supply the Apple header when compiling on macOS.
    if [ "$(uname -s)" = Darwin ]; then
        export CFLAGS="${CFLAGS:--g -O2} -include mach-o/dyld.h"
    fi
    sh autoconf.sh
    ./configure --prefix="$PREFIX"
    make -j "$jobs"
    make install
)

"$PREFIX/bin/iverilog" -V

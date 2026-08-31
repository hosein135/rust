{
  description = "Verilog IDE development environment (nixpkgs 25.05)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, nixpkgs, rust-overlay }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
          rustToolchain = pkgs.rust-bin.stable."1.88.0".default;
          # xezim (https://github.com/aionhw/xezim) requires rustc 1.92+.
          xezimRust = (pkgs.rust-bin.stable."1.92.0" or pkgs.rust-bin.stable.latest).default;
          linuxRuntimeLibs = pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.libxkbcommon
            pkgs.fontconfig
            pkgs.freetype
            pkgs.dbus
            pkgs.libdrm
            pkgs.systemd
            pkgs.wayland
            pkgs.wayland-protocols
            pkgs.xorg.libX11
            pkgs.xorg.libXext
            pkgs.xorg.libxcb
            pkgs.xorg.libXcursor
            pkgs.xorg.libXi
            pkgs.xorg.libXrandr
            pkgs.xorg.libXfixes
            pkgs.xorg.libXrender
            pkgs.xorg.libxshmfence
          ];
          # Runtime wrapper: first invocation clones + cargo-builds xezim into
          # ~/.cache/verilog-ide/xezim-build, later invocations exec the binary.
          # Building at runtime (not in the Nix sandbox) is required because
          # upstream does not ship Cargo.lock.
          xezim = pkgs.writeShellApplication {
            name = "xezim";
            runtimeInputs = [
              xezimRust
              pkgs.git
              pkgs.cmake
              pkgs.pkg-config
              pkgs.openssl
              pkgs.libffi
              pkgs.cacert
              pkgs.clang
              pkgs.stdenv.cc
              pkgs.gnumake
              pkgs.perl
            ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
            text = ''
              set -euo pipefail
              REV="265595e6da3764682bb1a64151c2debb7fd6ba20"
              CACHE="''${XDG_CACHE_HOME:-$HOME/.cache}/verilog-ide/xezim-build"
              BIN="$CACHE/bin/xezim"
              STAMP="$CACHE/rev"
              RUSTV="$(rustc -V)"
              NEED="$REV $RUSTV"

              export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
              export GIT_TERMINAL_PROMPT=0
              export CARGO_TERM_COLOR=never
              export CARGO_HOME="$CACHE/cargo"
              export CARGO_TARGET_DIR="$CACHE/target"
              export CARGO_NET_GIT_FETCH_WITH_CLI=true
              export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
              export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.libffi.dev}/lib/pkgconfig:''${PKG_CONFIG_PATH:-}"

              if [ -x "$BIN" ] && [ -f "$STAMP" ] && [ "$(cat "$STAMP")" = "$NEED" ]; then
                exec "$BIN" "$@"
              fi

              echo "[verilog-ide] Building xezim ($REV) — first time can take several minutes..." >&2
              mkdir -p "$CACHE/src" "$CACHE/bin"
              SRC="$CACHE/src/xezim"
              if [ ! -d "$SRC/.git" ]; then
                rm -rf "$SRC"
                git clone --depth 1 https://github.com/aionhw/xezim.git "$SRC"
              fi
              git -C "$SRC" fetch --depth 1 origin "$REV"
              git -C "$SRC" checkout --force --detach FETCH_HEAD

              cargo build --release --manifest-path "$SRC/Cargo.toml" --bin xezim
              if [ ! -x "$CARGO_TARGET_DIR/release/xezim" ]; then
                echo "[verilog-ide] cargo built but $CARGO_TARGET_DIR/release/xezim is missing" >&2
                exit 1
              fi
              cp -f "$CARGO_TARGET_DIR/release/xezim" "$BIN"
              chmod +x "$BIN"
              printf '%s\n' "$NEED" > "$STAMP"
              exec "$BIN" "$@"
            '';
          };
        in
        {
          default = pkgs.mkShell {
            name = "verilog-ide";
            packages = with pkgs; [
              rustToolchain
              xezim
              pkg-config
              openssl
              clang
              llvmPackages.clang
              llvmPackages.libclang
              cmake
              git
              curl
              perl
              zenity
              xdg-desktop-portal-gtk
            ];

            buildInputs = linuxRuntimeLibs;

            shellHook = ''
              export VERILOG_IDE_NIX=1
              export RUST_BACKTRACE=1
              export ICED_BACKEND=tiny-skia
              export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
              ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
                export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath linuxRuntimeLibs}:$LD_LIBRARY_PATH"
              ''}
            '';
          };
        }
      );
    };
}

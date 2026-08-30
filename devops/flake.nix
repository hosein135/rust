{
  description = "Verilog IDE development environment (nixpkgs 25.05)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  };

  outputs =
    { self, nixpkgs }:
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
          pkgs = nixpkgs.legacyPackages.${system};
          linuxGraphics = pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.libxkbcommon
            pkgs.fontconfig
            pkgs.freetype
            pkgs.alsa-lib
            pkgs.vulkan-loader
            pkgs.vulkan-tools
            pkgs.libGL
            pkgs.mesa
            pkgs.wayland
            pkgs.wayland-protocols
            pkgs.xorg.libxshmfence
            pkgs.dbus
            pkgs.nss
            pkgs.atk
            pkgs.at-spi2-atk
            pkgs.glib
            pkgs.cairo
            pkgs.pango
            pkgs.gdk-pixbuf
            pkgs.gtk3
            pkgs.xorg.libX11
            pkgs.xorg.libXcb
            pkgs.xorg.libXcursor
            pkgs.xorg.libXi
            pkgs.xorg.libXrandr
            pkgs.xorg.libXfixes
            pkgs.xorg.libXrender
          ];
        in
        {
          default = pkgs.mkShell {
            name = "verilog-ide";
            packages = with pkgs; [
              rustc
              cargo
              rustfmt
              clippy
              pkg-config
              openssl
              clang
              llvmPackages.clang
              llvmPackages.libclang
              cmake
              git
              curl
              perl
            ]
            ++ linuxGraphics;

            shellHook = ''
              export VERILOG_IDE_NIX=1
              export RUST_BACKTRACE=1
              export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
            '';
          };
        }
      );
    };
}

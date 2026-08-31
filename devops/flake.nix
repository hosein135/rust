{
  description = "Verilog IDE development environment (nixpkgs 25.05)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
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
          linuxWindowing = pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.libxkbcommon
            pkgs.fontconfig
            pkgs.freetype
            pkgs.wayland
            pkgs.wayland-protocols
            pkgs.xorg.libxshmfence
            pkgs.xorg.libX11
            pkgs.xorg.libxcb
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
              rustToolchain
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
            ++ linuxWindowing;

            shellHook = ''
              export VERILOG_IDE_NIX=1
              export RUST_BACKTRACE=1
              export ICED_BACKEND=tiny-skia
              export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
            '';
          };
        }
      );
    };
}

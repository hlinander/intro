{
  inputs = 
  {
   # nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.11";
   nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
   # rust-overlay.url = "github:oxalica/rust-overlay";
    fenix = {
          url = github:nix-community/fenix;
          inputs.nixpkgs.follows = "nixpkgs";
        };
   flake-utils.url  = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, fenix, flake-utils, ... }:
    let
      system = "x86_64-linux";
      rustToolchain = fenix.packages."${system}".complete;
      pkgs = (import nixpkgs {
        inherit system;
      });
      program = (pkgs.makeRustPlatform {
        cargo = rustToolchain.toolchain;
        rustc = rustToolchain.toolchain;
      }).buildRustPackage {
        pname = "lol64k";
        version = "0.1.0";
        src = ./rust;
        cargoLock.lockFile = ./rust/Cargo.lock;
      };
      buildInputs = with pkgs; [
          glfw3
          glm
          xorg.libX11
          xorg.libpthreadstubs
          xorg.libXau
          xorg.libXdmcp
          pipewire
      ];
      nativeInputs = with pkgs; [
          helix
          nixfmt
          (python3.withPackages (p: [
            p.python-lsp-server
            p.numpy
            p.ipython
            p.black
            p.flake8
          ]))
          valgrind
          kcachegrind
          cacert
          git
          gdb
          pkgconfig
          (rustToolchain.withComponents [ "cargo" "rustc" "rust-src" "rustfmt" "clippy"])
          fenix.packages."${system}".rust-analyzer
          llvmPackages_14.llvm
          upx
        ];
    in {
      packages.x86_64-linux.default = program;
      devShells.x86_64-linux.default = (pkgs.mkShell.override { stdenv = pkgs.llvmPackages_14.stdenv; }) {
        buildInputs = buildInputs;
        nativeBuildInputs = nativeInputs;
        shellHook = ''
          export LIBCLANG_PATH="${pkgs.llvmPackages_14.libclang.lib}/lib"
          '';
         };
    };
}

{
  inputs = 
  {
   nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.11";
   #nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
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
      inherit (pkgs) lib;
      stdenv = pkgs.llvmPackages_14.stdenv;
      program = (pkgs.makeRustPlatform {
        cargo = rustToolchain.toolchain;
        rustc = rustToolchain.toolchain;
      }).buildRustPackage {
        pname = "lol64k";
        version = "0.1.0";
        src = ./rust;
        cargoLock.lockFile = ./rust/Cargo.lock;
        buildInputs = buildInputs;
        nativeBuildInputs = nativeInputs;
        LIBCLANG_PATH = "${pkgs.llvmPackages_14.libclang.lib}/lib";

        preBuild = ''
            # From: https://github.com/NixOS/nixpkgs/blob/1fab95f5190d087e66a3502481e34e15d62090aa/pkgs/applications/networking/browsers/firefox/common.nix#L247-L253
            # Set C flags for Rust's bindgen program. Unlike ordinary C
            # compilation, bindgen does not invoke $CC directly. Instead it
            # uses LLVM's libclang. To make sure all necessary flags are
            # included we need to look in a few places.
            export BINDGEN_EXTRA_CLANG_ARGS="$(< ${stdenv.cc}/nix-support/libc-crt1-cflags) \
              $(< ${stdenv.cc}/nix-support/libc-cflags) \
              $(< ${stdenv.cc}/nix-support/cc-cflags) \
              $(< ${stdenv.cc}/nix-support/libcxx-cxxflags) \
              ${lib.optionalString stdenv.cc.isClang "-idirafter ${stdenv.cc.cc}/lib/clang/${lib.getVersion stdenv.cc.cc}/include"} \
              ${lib.optionalString stdenv.cc.isGNU "-isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc} -isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc}/${stdenv.hostPlatform.config} -idirafter ${stdenv.cc.cc}/lib/gcc/${stdenv.hostPlatform.config}/${lib.getVersion stdenv.cc.cc}/include"} \
            "
          '';
      };
      buildInputs = with pkgs; [
          glfw3
          glm
          xorg.libX11
          xorg.libpthreadstubs
          xorg.libXau
          xorg.libXdmcp
          xorg.libXrandr
          xorg.libXinerama
          xorg.libXcursor
          xorg.libXi
          libGL
          libGLU
          pipewire
          SDL2
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
          cmake
          (rustToolchain.withComponents [ "cargo" "rustc" "rust-src" "rustfmt" "clippy"])
          fenix.packages."${system}".rust-analyzer
          llvmPackages_14.llvm
          upx
        ];
      # crateOverrides = rustPlatform.defaultCrateOverrides // {
            # glfw = attrs: {
              # nativeBuildInputs = with pkgs; [ pkg-config ];
              # buildInputs = with pkgs; [ glfw x11 ];
            # };
          # };
    in {
      packages.x86_64-linux.default = program;
      devShells.x86_64-linux.default = (pkgs.mkShell.override { stdenv = pkgs.llvmPackages_14.stdenv; }) {
        buildInputs = buildInputs;
        nativeBuildInputs = nativeInputs;
        shellHook = ''
          export LIBCLANG_PATH="${pkgs.llvmPackages_14.libclang.lib}/lib"
          export LD_LIBRARY_PATH=/run/opengl-driver/lib/:${pkgs.lib.makeLibraryPath ([pkgs.libGL pkgs.libGLU pkgs.xorg.libX11])}
          export X11_X11_INCLUDE_PATH="${pkgs.xorg.libX11}/include"
          export X11_X11_LIB=${pkgs.lib.makeLibraryPath ([pkgs.xorg.libX11])}

          '';
         };
    };
}

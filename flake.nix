{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      utils,
      naersk,
      ...
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = callPackage naersk { };

        inherit (pkgs)
          mkShell
          rustPlatform
          callPackage
          lib
          ;
      in
      {
        defaultPackage = naersk-lib.buildPackage ./.;
        devShells.default = mkShell {
          nativeBuildInputs = with pkgs; [
            cargo
            rustc
            fuse-overlayfs
            libarchive
            openssl
            pkg-config
            mold-wrapped
            cargo-tarpaulin
            cargo-i18n
          ];

          packages = with pkgs; [
            # Tools
            bacon
            diesel-cli
            cargo-info
            rustPackages.clippy
            rustfmt
            rust-analyzer
          ];
          env = {
            RUST_SRC_PATH = rustPlatform.rustLibSrc;
            RUSTFLAGS = "-C link-arg=-fuse-ld=mold"; # Use mold linker
            LD_LIBRARY_PATH = "$LD_LIBRARY_PATH:${
              with pkgs;
              lib.makeLibraryPath [
                wayland
                libxkbcommon
                fontconfig
                libGL
                dbus
              ]
            }";
          };
        };
      }
    );
}

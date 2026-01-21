{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  outputs =
    { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = lib.platforms.linux; # Only support linux
      forEachSystem = fn: lib.genAttrs systems (system: fn system nixpkgs.legacyPackages.${system});
    in
    {
      packages = forEachSystem (
        system: pkgs: rec {
          barnacle = pkgs.callPackage ./nix/package.nix { };
          default = barnacle;
        }
      );

      devShells = forEachSystem (
        system: pkgs: {
          default = import ./nix/shell.nix {
            inherit pkgs;
            inherit (self.packages.${system}) barnacle;
          };
        }
      );
    };
}

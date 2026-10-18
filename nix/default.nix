{
  flake-parts,
  ...
}@inputs:
flake-parts.lib.mkFlake { inherit inputs; } {
  systems = [
    "x86_64-linux"
    "aarch64-linux"
    "aarch64-darwin"
  ];
  imports = [
    inputs.treefmt-nix.flakeModule
  ];

  perSystem =
    { pkgs, ... }:
    let
      craneLib = inputs.crane.mkLib pkgs;
    in
    {
      packages = rec {
        default = twrit;
        twrit = pkgs.callPackage ./pkgs/twrit.nix { inherit craneLib; };
      };

      devShells.default = pkgs.callPackage ./dev.nix {
        inherit inputs;
        twrit = pkgs.callPackage ./pkgs/twrit.nix { inherit craneLib; };
        fenix = inputs.fenix.packages.${pkgs.stdenv.hostPlatform.system};
        tomet = inputs.tomet.packages.${pkgs.stdenv.hostPlatform.system}.tomet;
        tomet-lsp = inputs.tomet.packages.${pkgs.stdenv.hostPlatform.system}.tomet-lsp;
        tomet-shelf = inputs.tomet-shelf.packages.${pkgs.stdenv.hostPlatform.system}.tomet-shelf;
      };

      treefmt = import ./formatter.nix {
        tomet = inputs.tomet.packages.${pkgs.stdenv.hostPlatform.system}.tomet;
      };
    };
}

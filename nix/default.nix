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
      system = pkgs.stdenv.hostPlatform.system;
      craneLib = inputs.crane.mkLib pkgs;
      sisho = inputs.sisho.packages.${system};
    in
    {
      packages = rec {
        default = twrit;
        twrit = pkgs.callPackage ./pkgs/twrit.nix { inherit craneLib; };
      };

      devShells.default = pkgs.callPackage ./dev.nix {
        inherit inputs;
        twrit = pkgs.callPackage ./pkgs/twrit.nix { inherit craneLib; };
        fenix = inputs.fenix.packages.${system};
        # The other tools; twrit itself is the one built from this checkout.
        tools = inputs.sisho.legacyPackages.${system}.combine [
          sisho.sisho
          sisho.tomet
          sisho.tomet-lsp
          sisho.tomet-shelf
        ];
      };

      treefmt = import ./formatter.nix {
        tomet = sisho.tomet;
      };
    };
}

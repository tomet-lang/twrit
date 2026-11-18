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
      tometPkgs = inputs.tomet.packages.${system};
      shelfPkgs = inputs.tomet-shelf.packages.${system};
      catastoPkgs = inputs.catasto.packages.${system};
      sishoPkgs = inputs.sisho.packages.${system};
    in
    {
      packages = rec {
        default = twrit;
        twrit = pkgs.callPackage ./pkgs/twrit.nix { inherit craneLib; };

        # What CI builds and sends to Cachix (.github/workflows/nix-ci.yml).
        ci = pkgs.linkFarm "ci" [
          {
            name = "twrit";
            path = twrit;
          }
        ];
      };

      devShells.default = pkgs.callPackage ./dev.nix {
        inherit inputs;
        twrit = pkgs.callPackage ./pkgs/twrit.nix { inherit craneLib; };
        fenix = inputs.fenix.packages.${system};
        # The other tools; twrit itself is the one built from this checkout.
        tools = pkgs.symlinkJoin {
          name = "tomet-toolchain";
          paths = [
            sishoPkgs.sisho
            tometPkgs.tomet
            tometPkgs.tomet-lsp
            shelfPkgs.tomet-shelf
            # `sisho check` runs `catasto check` beside `tomet` and `twrit`.
            catastoPkgs.catasto
          ];
        };
      };

      treefmt = import ./formatter.nix {
        inherit (tometPkgs) tomet;
      };
    };
}

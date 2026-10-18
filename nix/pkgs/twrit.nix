{
  craneLib,
}:
let
  src = ../..;

  commonArgs = {
    inherit src;

    pname = "twrit";
    version = "0.2.0";
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage (
  commonArgs
  // {
    inherit cargoArtifacts;

    cargoExtraArgs = "-p twrit-cli";

    doCheck = true;
  }
)

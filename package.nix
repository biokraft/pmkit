{
  lib,
  rustPlatform,
}:

let
  manifest = builtins.fromTOML (builtins.readFile ./Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = manifest.package.name;
  inherit (manifest.package) version;

  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;
  doCheck = false;

  meta = {
    inherit (manifest.package) description homepage;
    license = lib.licenses.mit;
    mainProgram = "pmkit";
  };
}

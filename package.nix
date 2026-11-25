{ lib, rustPlatform }:
let
  package = (lib.importTOML ./eyd/Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = package.name;
  inherit (package) version;
  src = ./eyd;
  cargoLock = {
    lockFile = ./eyd/Cargo.lock;
  };
  meta = {
    license = lib.licenses.mit;
    mainProgram = "eyd";
    platforms = lib.platforms.unix;
  };
}

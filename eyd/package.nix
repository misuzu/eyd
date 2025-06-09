{ rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "eyd";
  version = "0.2.0";
  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  meta.mainProgram = "eyd";
}

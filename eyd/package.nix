{ rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "eyd";
  version = "0.1.0";
  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  meta.mainProgram = "eyd";
}

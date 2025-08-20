{ rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "eyd";
  version = "0.3.0";
  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  meta.mainProgram = "eyd";
}

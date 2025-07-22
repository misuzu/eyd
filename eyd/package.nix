{ rustPlatform }:
rustPlatform.buildRustPackage {
  pname = "eyd";
  version = "0.2.1";
  src = ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  meta.mainProgram = "eyd";
}

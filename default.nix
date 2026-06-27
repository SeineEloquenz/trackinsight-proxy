{
  lib,
  rustPlatform,
}:

rustPlatform.buildRustPackage {
  pname = "trackinsight-proxy";
  version = "0.1.0";

  src = ./.;

  cargoLock.lockFile = ./Cargo.lock;

  meta = {
    description = "Trackinsight reverse proxy";
    homepage = "https://github.com/SeineEloquenz/trackinsight-proxy";
    license = lib.licenses.gpl3Plus;
    mainProgram = "trackinsight-proxy";
  };
}

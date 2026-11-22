{
  lib,
  rustPlatform,
}:

rustPlatform.buildRustPackage {
  pname = "trackinsight-proxy";
  version = "0.2.0";

  src = ./.;

  cargoLock.lockFile = ./Cargo.lock;


  meta = {
    description = "TrackInsight API for Ghostfolio backed by the holdings fund issuers publish";
    homepage = "https://github.com/SeineEloquenz/trackinsight-proxy";
    license = lib.licenses.gpl3Plus;
    mainProgram = "trackinsight-proxy";
  };
}

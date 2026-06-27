{
  dockerTools,
  trackinsight-proxy,
}:

# Tiny image: just the Rust binary. Chromium runs as a separate sidecar
# (see docker-compose.yml); this service connects to it over CDP.
dockerTools.buildLayeredImage {
  name = "trackinsight-proxy";
  tag = "latest";

  contents = [ trackinsight-proxy ];

  config = {
    Cmd = [ "${trackinsight-proxy}/bin/trackinsight-proxy" ];
    Env = [
      "SOLVER_PORT=8191"
      "CHROME_URL=http://chrome:9222"
      "RUST_LOG=info"
    ];
    ExposedPorts = {
      "8191/tcp" = { };
    };
  };
}

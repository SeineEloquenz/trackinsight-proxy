{
  dockerTools,
  cacert,
  trackinsight-proxy,
}:

dockerTools.buildLayeredImage {
  name = "trackinsight-proxy";
  tag = "latest";

  contents = [
    trackinsight-proxy
    cacert
  ];

  config = {
    Cmd = [ "${trackinsight-proxy}/bin/trackinsight-proxy" ];
    Env = [
      "LISTEN_HOST=0.0.0.0"
      "LISTEN_PORT=8191"
      "RUST_LOG=info"
    ];
    ExposedPorts = {
      "8191/tcp" = { };
    };
  };
}

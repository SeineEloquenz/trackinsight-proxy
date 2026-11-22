self:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.trackinsight-proxy;
in
{
  options.services.trackinsight-proxy = {
    enable = lib.mkEnableOption "the TrackInsight API backed by fund issuer data";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      defaultText = lib.literalExpression "trackinsight-proxy.packages.\${system}.default";
      description = "The trackinsight-proxy package to use.";
    };

    host = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "Address the proxy listens on.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8191;
      description = "Port the proxy listens on.";
    };

    environment = lib.mkOption {
      type = with lib.types; attrsOf str;
      default = { };
      example = {
        RUST_LOG = "debug";
      };
      description = "Extra environment, such as `RUST_LOG`.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.trackinsight-proxy = {
      description = "TrackInsight API backed by fund issuer data";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];

      environment = {
        LISTEN_HOST = cfg.host;
        LISTEN_PORT = toString cfg.port;
        NO_COLOR = "1";
      }
      // cfg.environment;

      serviceConfig = {
        ExecStart = lib.getExe cfg.package;
        Restart = "on-failure";
        DynamicUser = true;
        CapabilityBoundingSet = "";
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        NoNewPrivileges = true;
        PrivateDevices = true;
        PrivateTmp = true;
        PrivateUsers = true;
        ProtectClock = true;
        ProtectControlGroups = true;
        ProtectHome = true;
        ProtectHostname = true;
        ProtectKernelLogs = true;
        ProtectKernelModules = true;
        ProtectKernelTunables = true;
        ProtectProc = "invisible";
        ProtectSystem = "strict";
        RestrictAddressFamilies = [
          "AF_UNIX"
          "AF_INET"
          "AF_INET6"
        ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [
          "@system-service"
          "~@privileged"
        ];
      };
    };
  };
}

self:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.trackinsight-proxy;

  hardening = {
    DynamicUser = true;
    LockPersonality = true;
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
    RestrictRealtime = true;
    SystemCallArchitectures = "native";
    CapabilityBoundingSet = "";
  };
in
{
  options.services.trackinsight-proxy = {
    enable = lib.mkEnableOption "the TrackInsight reverse proxy";

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

    chromeUrl = lib.mkOption {
      type = lib.types.str;
      default = "http://127.0.0.1:${toString cfg.chromium.port}";
      defaultText = lib.literalExpression ''"http://127.0.0.1:''${toString config.services.trackinsight-proxy.chromium.port}"'';
      description = "DevTools endpoint of the Chrome instance the proxy drives.";
    };

    environment = lib.mkOption {
      type = with lib.types; attrsOf str;
      default = { };
      example = {
        RUST_LOG = "debug";
      };
      description = "Extra environment, such as `TRACKINSIGHT_ORIGIN`, `WARM_URL` or `RUST_LOG`.";
    };

    chromium = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Run a headless Chromium on localhost for the proxy.";
      };

      package = lib.mkPackageOption pkgs "chromium" { };

      port = lib.mkOption {
        type = lib.types.port;
        default = 9222;
        description = "DevTools port of the local Chromium.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.trackinsight-proxy-chromium = lib.mkIf cfg.chromium.enable {
      description = "Headless Chromium for trackinsight-proxy";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];

      script = ''
        exec ${lib.getExe cfg.chromium.package} \
          --headless=new --no-sandbox --disable-gpu --disable-dev-shm-usage \
          --enable-unsafe-swiftshader \
          --remote-debugging-address=127.0.0.1 \
          --remote-debugging-port=${toString cfg.chromium.port} \
          --user-data-dir="$STATE_DIRECTORY"
      '';

      serviceConfig = hardening // {
        Restart = "on-failure";
        StateDirectory = "trackinsight-proxy-chromium";
        CacheDirectory = "trackinsight-proxy-chromium";
      };
    };

    systemd.services.trackinsight-proxy = {
      description = "TrackInsight reverse proxy";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ] ++ lib.optional cfg.chromium.enable "trackinsight-proxy-chromium.service";
      wants = [ "network-online.target" ] ++ lib.optional cfg.chromium.enable "trackinsight-proxy-chromium.service";

      environment = {
        SOLVER_HOST = cfg.host;
        SOLVER_PORT = toString cfg.port;
        CHROME_URL = cfg.chromeUrl;
        NO_COLOR = "1";
      } // cfg.environment;

      serviceConfig = hardening // {
        ExecStart = lib.getExe cfg.package;
        Restart = "on-failure";
        MemoryDenyWriteExecute = true;
        SystemCallFilter = [
          "@system-service"
          "~@privileged"
        ];
      };
    };
  };
}

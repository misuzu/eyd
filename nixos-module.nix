{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.boot.initrd.eyd;
in
{
  options.boot.initrd.eyd = {
    enable = lib.mkEnableOption "eyd";
    package = lib.mkPackageOption pkgs "eyd" { };
    keep = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      example = [
        "/etc/ssh"
        "/home"
        "/root"
        "/var/log/btmp"
        "/var/log/journal"
        "/var/log/lastlog"
        "/var/log/wtmp"
      ];
      description = "Keep only these paths.";
    };
    defaultKeep = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [
        "/boot"
        "/etc/group"
        "/etc/machine-id"
        "/etc/nixos"
        "/etc/NIXOS"
        "/etc/passwd"
        "/etc/shadow"
        "/etc/subgid"
        "/etc/subuid"
        "/nix"
        "/run"
        "/var/empty"
        "/var/lib/nixos"
      ];
      description = "Paths kept by default, do not touch if not sure.";
    };
    retain = lib.mkOption {
      type = lib.types.ints.unsigned;
      default = 5;
      description = "How many previous roots to retain, zero to retain all.";
    };
  };
  config = lib.mkIf cfg.enable {
    boot.initrd.systemd.storePaths = [ cfg.package ];
    boot.initrd.systemd.services.eyd = {
      after = [ "initrd-fs.target" ];
      before = [ "initrd.target" ];
      wantedBy = [ "initrd.target" ];
      description = "Erase your darlings";
      serviceConfig = {
        ExecStart = "${lib.getExe cfg.package} ${
          lib.escapeShellArgs (
            [
              "/sysroot"
              "/oldroot"
              (toString cfg.retain)
            ]
            ++ cfg.defaultKeep
            ++ cfg.keep
          )
        }";
        RemainAfterExit = true;
        Restart = "no";
        TimeoutSec = "infinity";
        Type = "oneshot";
      };
      unitConfig.DefaultDependencies = "no";
    };
    # make sure we don't garbage collect this package
    system.extraDependencies = [ cfg.package ];
  };
}

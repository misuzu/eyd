{
  config,
  lib,
  pkgs,
  ...
}:
let
  eyd = pkgs.pkgsStatic.callPackage ./eyd/package.nix { };
  cfg = config.boot.initrd.eyd;
in
{
  options.boot.initrd.eyd = {
    enable = lib.mkEnableOption "eyd";
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
    assertions = [
      {
        assertion = config.boot.initrd.systemd.enable;
        message = "`boot.initrd.systemd.enable = true;` is required.";
      }
    ];
    boot.initrd.systemd.storePaths = [ eyd ];
    boot.initrd.systemd.services.eyd = {
      after = [ "sysroot.mount" ];
      before = [ "initrd-fs.target" ];
      requiredBy = [ "initrd-fs.target" ];
      wantedBy = [ "initrd.target" ];
      description = "Erase your darlings";
      serviceConfig = {
        ExecStart = "${lib.getExe eyd} ${
          lib.escapeShellArgs [
            "/sysroot"
            "/oldroot"
            (toString cfg.retain)
            (builtins.toJSON (cfg.defaultKeep ++ cfg.keep))
          ]
        }";
        RemainAfterExit = true;
        TimeoutSec = "infinity";
        Type = "oneshot";
      };
      unitConfig.DefaultDependencies = "no";
    };
  };
}

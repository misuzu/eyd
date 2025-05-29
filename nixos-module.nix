{
  config,
  lib,
  pkgs,
  ...
}:
let
  pythonEnv = pkgs.python3.withPackages (ps: with ps;  [ psutil ]);
  eyd = pkgs.writeScriptBin "eyd" ''
    #!${lib.getExe pythonEnv}
    ${builtins.readFile ./eyd.py}
  '';
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
  };
  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = config.boot.initrd.systemd.enable;
        message = "`boot.initrd.systemd.enable = true;` is required.";
      }
    ];
    boot.initrd.systemd.storePaths = [
      eyd
      pythonEnv
      pkgs.python3
      pkgs.python3Packages.psutil
    ];
    boot.initrd.systemd.services.eyd = {
      after = [ "sysroot.mount" ];
      before = [ "initrd-switch-root.target" ];
      wantedBy = [ "initrd.target" ];
      description = "Erase your darlings";
      serviceConfig = {
        Type = "oneshot";
        ExecStart = "${lib.getExe eyd} ${
          lib.escapeShellArgs [
            "/sysroot"
            (builtins.toJSON (cfg.defaultKeep ++ cfg.keep))
          ]
        }";
      };
      unitConfig.DefaultDependencies = "no";
    };
  };
}

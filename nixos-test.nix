{
  name = "eyd";

  nodes = {
    simple = {
      imports = [ ./nixos-module.nix ];

      boot.initrd.systemd.enable = true;
      boot.initrd.eyd.enable = true;
      boot.initrd.eyd.keep = [
        "/etc/ssh/ssh_host_ed25519_key"
        "/etc/ssh/ssh_host_ed25519_key.pub"
        "/etc/ssh/ssh_host_rsa_key"
        "/etc/ssh/ssh_host_rsa_key.pub"
        "/home"
        "/root"
      ];
      boot.supportedFilesystems = [ "ext4" ];

      services.sshd.enable = true;

      virtualisation.emptyDiskImages = [ 128 ];
      virtualisation.fileSystems."/oldroot" = {
        autoFormat = true;
        neededForBoot = true;
        device = "/dev/vdb";
        fsType = "ext4";
      };
    };
  };

  testScript = ''
    simple.start()

    simple.wait_for_console_text("Finished Erase your darlings")
    simple.wait_for_unit("default.target")

    simple.succeed("test -e /oldroot/0000000000000001")
    simple.fail("test -e /oldroot/0000000000000002")

    simple.shutdown()
    simple.start()

    simple.wait_for_console_text("Finished Erase your darlings")
    simple.wait_for_unit("default.target")

    simple.succeed("test -e /oldroot/0000000000000001")
    simple.succeed("test -e /oldroot/0000000000000002")

    simple.succeed("test -e /oldroot/0000000000000002/etc/ssh/ssh_config")
    simple.fail("test -e /oldroot/0000000000000002/etc/ssh/ssh_host_ed25519_key")
  '';
}

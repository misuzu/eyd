# Erase your darlings, but without filesystems tricks

## WARNING

I'm not responsible for any data loss, use at your own risk.

### Set up

```nix
inputs.eyd.url = "github:misuzu/eyd";

imports = [ inputs.eyd.nixosModules.default ];

boot.initrd.systemd.enable = true;
boot.initrd.eyd.enable = true;
boot.initrd.eyd.keep = [
  "/etc/docker"
  "/etc/ssh/ssh_host_ed25519_key"
  "/etc/ssh/ssh_host_ed25519_key.pub"
  "/etc/ssh/ssh_host_rsa_key"
  "/etc/ssh/ssh_host_rsa_key.pub"
  "/home"
  "/media"
  "/mnt"
  "/root"
  "/var/cache/fwupd"
  "/var/db/dhcpcd"
  "/var/lib/alsa"
  "/var/lib/bluetooth"
  "/var/lib/docker"
  "/var/lib/fwupd"
  "/var/lib/iwd"
  "/var/lib/logrotate.status"
  "/var/lib/NetworkManager"
  "/var/lib/private/yggdrasil"
  "/var/lib/systemd"
  "/var/lib/tailscale"
  "/var/lib/zerotier-one"
  "/var/log/btmp"
  "/var/log/journal"
  "/var/log/lastlog"
  "/var/log/wtmp"
];
```

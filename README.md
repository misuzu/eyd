# Erase your darlings, but without filesystems tricks

## WARNING

I'm not responsible for any data loss, use at your own risk.

## Features

- Easy to set up, just enable it and set paths to keep
- No snapshots, tmpfs as root or bind mounts are required, works with any filesystem
- Moves everything from root filesystem not in `boot.initrd.eyd.keep` to the `/oldroot` directory
- Retains data from previous boots (5 by default), so you have a chance to recover your data

## Drawbacks

- Doesn't touch anything besides the root filesystem
- A 800KB binary in initrd
- Only systemd initrd is supported

## Set it up

```nix
# in your flake
inputs.eyd.url = "github:misuzu/eyd";

# import the eyd module
imports = [ inputs.eyd.nixosModules.default ];

# the actual configuration
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
boot.initrd.eyd.retain = 3;
```

## Debugging

Run `journalctl -b 0 -u eyd` to see the logs


## Related projects

- [`impermanence`](https://github.com/nix-community/impermanence)
- [`preservation`](https://github.com/nix-community/preservation)

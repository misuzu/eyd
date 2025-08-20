# Erase your darlings[^1], but without filesystems tricks

## WARNING

I am not responsible for any data loss. Use at your own risk.

## Features

- Easy to set up: just enable it and specify the paths to keep
- No snapshots, tmpfs as root, or bind mounts are required - works with any filesystem
- Moves everything from the root filesystem not listed in `boot.initrd.eyd.keep` to the `/oldroot` directory
- Retains data from previous boots (5 by default), giving you a chance to recover your data

## Drawbacks

- Only affects the root filesystem
- Adds a ~600 KB binary to the initrd
- Supports only systemd initrd

## Setup

```nix
# in your flake
inputs.eyd.url = "github:misuzu/eyd/v0.3.0";

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
  "/root"
  "/var/db/dhcpcd"
  "/var/lib/alsa"
  "/var/lib/bluetooth"
  "/var/lib/docker"
  "/var/lib/logrotate.status"
  "/var/lib/NetworkManager"
  "/var/lib/private/yggdrasil"
  "/var/lib/systemd"
  "/var/lib/tailscale"
  "/var/log/btmp"
  "/var/log/journal"
  "/var/log/lastlog"
  "/var/log/wtmp"
];
boot.initrd.eyd.retain = 3;
```

## Debugging

Run `journalctl -b 0 -u eyd` to view logs


## Related projects

- [`impermanence`](https://github.com/nix-community/impermanence)
- [`preservation`](https://github.com/nix-community/preservation)

## License

This project is released under the MIT License. See [LICENSE](./LICENSE).

[^1]: https://grahamc.com/blog/erase-your-darlings/
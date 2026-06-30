# System
The provided ISO and raspberry pi images for the Arcader project

| Arch  | Kernel                | Output ISO                    |
|-------|-----------------------|-------------------------------|
| amd64 | `linux-image-amd64`   | `out/arcader-kiosk-amd64.iso` |
| i386  | `linux-image-686-pae` | `out/arcader-kiosk-i386.iso`  |

The build downloads the Arcader `.deb` for the target arch from the
[Arcader releases](https://github.com/ArcaderProject/Arcader/releases) and bakes
it into the image. The version defaults to the latest release; pin one with
`ARCADER_VERSION`.

## Build

Requires Docker (the build runs `--privileged` for chroot/loop devices).

```sh
./run.sh                       # both arches, latest Arcader release
./run.sh i386                  # 32-bit only
ARCADER_VERSION=1.2.2 ./run.sh # pin a specific Arcader version
```

## Deploy

```sh
sudo dd if=out/arcader-kiosk-amd64.iso of=/dev/sdX bs=4M status=progress conv=fsync
```

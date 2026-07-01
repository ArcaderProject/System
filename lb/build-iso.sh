#!/bin/bash
set -euo pipefail
cd /lb

ARCH="${ARCH:-amd64}"

case "$ARCH" in
  amd64)
    LINUX_IMAGE="linux-image-amd64"
    GRUB_EFI_PKG="grub-efi-amd64-bin"
    RUST_TARGET="x86_64-unknown-linux-gnu"
    ;;
  i386)
    LINUX_IMAGE="linux-image-686-pae"
    GRUB_EFI_PKG="grub-efi-ia32-bin"
    RUST_TARGET="i686-unknown-linux-gnu"
    ;;
  *)
    echo "ERROR: unsupported ARCH '$ARCH' (use amd64 or i386)" >&2
    exit 1
    ;;
esac

INSTALLER_SRC="/installer-gui"
[ -d "$INSTALLER_SRC" ] || INSTALLER_SRC="$(cd /lb/.. 2>/dev/null && pwd)/installer-gui"
if [ -d "$INSTALLER_SRC" ]; then
  echo ">> Building graphical installer ($RUST_TARGET)"
  ( cd "$INSTALLER_SRC" && cargo build --release --target "$RUST_TARGET" )
  install -Dm755 \
    "$INSTALLER_SRC/target/$RUST_TARGET/release/arcader-installer" \
    config/includes.chroot/usr/local/bin/arcader-installer
else
  echo "ERROR: installer-gui source not found (looked in /installer-gui and ../installer-gui)" >&2
  exit 1
fi

ARCADER_VERSION="${ARCADER_VERSION:-latest}"
if [ "$ARCADER_VERSION" = "latest" ]; then
  RELEASE_JSON="$(curl -fsSL https://api.github.com/repos/ArcaderProject/Arcader/releases/latest)"
  ARCADER_VERSION="$(printf '%s' "$RELEASE_JSON" \
    | awk -F'"' '/"tag_name"/{v=$4} END{sub(/^v/,"",v); print v}')"
fi
[ -n "$ARCADER_VERSION" ] || { echo "ERROR: could not resolve Arcader version" >&2; exit 1; }

DEB_FILE="arcader_${ARCADER_VERSION}_${ARCH}.deb"
DEB_URL="https://github.com/ArcaderProject/Arcader/releases/download/v${ARCADER_VERSION}/${DEB_FILE}"

echo ">> Building kiosk ISO for ARCH=$ARCH using $DEB_FILE"

cat > config/package-lists/arch.list.chroot <<EOF
$LINUX_IMAGE
$GRUB_EFI_PKG
EOF

mkdir -p config/includes.chroot/opt/arcader
DEB_DEST=config/includes.chroot/opt/arcader/arcader.deb
echo ">> Fetching $DEB_URL"
curl -fL --retry 3 --retry-delay 2 -o "$DEB_DEST" "$DEB_URL"

lb config noauto \
  --mode debian \
  --architectures "$ARCH" \
  --distribution bookworm \
  --binary-images iso-hybrid \
  --debian-installer false \
  --archive-areas "main contrib non-free non-free-firmware" \
  --apt-indices false \
  --apt-recommends false \
  --memtest none \
  --bootappend-live "boot=live components quiet splash loglevel=3 vt.global_cursor_default=0 rd.systemd.show_status=false systemd.show_status=false udev.log_level=3 rd.udev.log_level=3"

lb build

mkdir -p /out
OUT="/out/arcader-kiosk-${ARCH}.iso"
for f in live-image-*.hybrid.iso *.hybrid.iso; do
  cp -f "$f" "$OUT"
  echo "Wrote $OUT (from $f)"
  exit 0
done

echo "ERROR: no ISO produced" >&2
ls -la
exit 1

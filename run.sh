#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

ARCADER_VERSION="${ARCADER_VERSION:-latest}"
ARCHES=("$@")
[ "${#ARCHES[@]}" -eq 0 ] && ARCHES=(amd64 i386)

mkdir -p out
mkdir -p lb/cache installer-gui/target .cargo-registry

IMAGE_TAG="${IMAGE_TAG:-arcader-iso-builder:latest}"

if [ "${BUILD_IMAGE:-1}" != "0" ]; then
  docker build -t "$IMAGE_TAG" .
fi

for ARCH in "${ARCHES[@]}"; do
  case "$ARCH" in
    amd64|i386) ;;
    *) echo "ERROR: unsupported arch '$ARCH' (use amd64 or i386)" >&2; exit 1 ;;
  esac
  docker run --rm --privileged \
    -e ARCH="$ARCH" -e ARCADER_VERSION="$ARCADER_VERSION" \
    -v "$PWD/out:/out" \
    -v "$PWD/lb/cache:/lb/cache" \
    -v "$PWD/installer-gui/target:/installer-gui/target" \
    -v "$PWD/.cargo-registry:/opt/cargo/registry" \
    "$IMAGE_TAG"
done

echo
for ARCH in "${ARCHES[@]}"; do
  echo "out/arcader-kiosk-${ARCH}.iso"
done

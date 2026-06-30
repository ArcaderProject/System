FROM debian:bookworm

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        live-build \
        debootstrap \
        xorriso \
        isolinux \
        syslinux-common \
        squashfs-tools \
        ca-certificates \
        curl \
        gnupg \
        mtools \
        dosfstools \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /lb

COPY lb/ /lb/

RUN find /lb/config/hooks -type f -name '*.hook.chroot' -exec chmod +x {} \; \
    && chmod +x /lb/build-iso.sh

CMD ["/lb/build-iso.sh"]

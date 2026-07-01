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
        build-essential \
        pkg-config \
        gcc-multilib \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --profile minimal --default-toolchain stable \
    && rustup target add x86_64-unknown-linux-gnu i686-unknown-linux-gnu

WORKDIR /lb

COPY installer-gui/ /installer-gui/
COPY lb/ /lb/

RUN find /lb/config/hooks -type f -name '*.hook.chroot' -exec chmod +x {} \; \
    && chmod +x /lb/build-iso.sh

CMD ["/lb/build-iso.sh"]

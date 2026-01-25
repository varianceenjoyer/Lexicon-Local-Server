# Dockerfile for Cuttlefish Android Emulator
# Ubuntu 22.04 LTS - recommended for Cuttlefish compatibility
#
# This container runs Cuttlefish with KVM passthrough for Android emulation.
# Requires: --privileged or --device=/dev/kvm on the host

FROM ubuntu:22.04

LABEL maintainer="Lexicon Local Server"
LABEL description="Cuttlefish Android Emulator on Ubuntu 22.04"

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Cuttlefish environment variables
ENV CUTTLEFISH_HOME=/opt/cuttlefish
ENV ANDROID_HOST_OUT=/opt/cuttlefish
ENV PATH="/opt/cuttlefish/bin:${PATH}"

# Install base dependencies
RUN apt-get update && apt-get install -y \
    curl \
    wget \
    unzip \
    git \
    sudo \
    ca-certificates \
    gnupg \
    lsb-release \
    && rm -rf /var/lib/apt/lists/*

# Install KVM and virtualization packages
RUN apt-get update && apt-get install -y \
    qemu-kvm \
    qemu-system-x86 \
    libvirt-daemon-system \
    libvirt-clients \
    bridge-utils \
    cpu-checker \
    && rm -rf /var/lib/apt/lists/*

# Install Cuttlefish runtime dependencies
RUN apt-get update && apt-get install -y \
    f2fs-tools \
    libc++1 \
    libc++abi1 \
    libdrm2 \
    libfdt1 \
    libgbm1 \
    libgl1-mesa-glx \
    libwayland-client0 \
    libwayland-server0 \
    libx11-xcb1 \
    libxcb-randr0 \
    libxcb-render0 \
    libxcb-shape0 \
    libxcb-shm0 \
    libxcb-xfixes0 \
    libxcb1 \
    libxml2 \
    e2fsprogs \
    && rm -rf /var/lib/apt/lists/*

# Install Android tools (adb, fastboot)
RUN apt-get update && apt-get install -y \
    adb \
    fastboot \
    && rm -rf /var/lib/apt/lists/*

# Install additional tools for debugging
RUN apt-get update && apt-get install -y \
    vim \
    htop \
    net-tools \
    iproute2 \
    iputils-ping \
    procps \
    rsyslog \
    && rm -rf /var/lib/apt/lists/*

# Install build dependencies for Cuttlefish Debian packages
RUN apt-get update && apt-get install -y \
    devscripts \
    equivs \
    config-package-dev \
    debhelper-compat \
    golang \
    && rm -rf /var/lib/apt/lists/*

# Create cuttlefish user and add to required groups
RUN groupadd -r cvdnetwork || true && \
    groupadd -r render || true && \
    useradd -m -s /bin/bash -G kvm,cvdnetwork,render cuttlefish && \
    echo "cuttlefish ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Create directories
RUN mkdir -p /opt/cuttlefish /opt/android-images /var/run/cuttlefish && \
    chown -R cuttlefish:cuttlefish /opt/cuttlefish /opt/android-images /var/run/cuttlefish

# Copy entrypoint script
COPY docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Install capability query stub for VM manager detection
RUN mkdir -p /usr/lib/cuttlefish-common/bin
COPY capability_query.py /usr/lib/cuttlefish-common/bin/
RUN chmod +x /usr/lib/cuttlefish-common/bin/capability_query.py

# Switch to cuttlefish user
USER cuttlefish
WORKDIR /opt/cuttlefish

# Expose ports
# 6520: ADB
# 8443: WebRTC (optional)
# 15550-15560: VNC and other services
EXPOSE 6520 8443 15550-15560

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=120s --retries=3 \
    CMD adb connect localhost:6520 && adb -s localhost:6520 shell getprop sys.boot_completed 2>/dev/null | grep -q "1" || exit 1

ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["launch"]

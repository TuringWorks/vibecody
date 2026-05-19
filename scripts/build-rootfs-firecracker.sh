#!/usr/bin/env bash
#
# scripts/build-rootfs-firecracker.sh — Tier-3 (Firecracker) rootfs builder.
#
# Produces a small ext4 image (≤ 20 MB target) containing BusyBox + bash, the
# bare minimum to spawn `sh -c "<agent command>"` inside a Firecracker microVM.
# Slice F1 from docs/design/sandbox-tiers/03-firecracker-tier.md.
#
# Why so small: the image is bind-mounted read-only into the microVM; the
# workspace bind (slice F3, virtio-fs) and tmpfs overlay (slice F2) handle the
# writable surface. Bigger rootfs = slower microVM cold-start; we measure
# 5–8 ms boot today and want to keep that.
#
# Why Docker: building an ext4 image from scratch needs root + mkfs.ext4 +
# loop devices, none of which exist on a stock macOS dev machine. Docker
# gives us a deterministic Linux build environment that produces the same
# bytes whether the dev runs on macOS, Linux, or in CI.
#
# Output: target/firecracker-rootfs/rootfs.ext4 + rootfs.ext4.sha256
#
# Usage:
#   scripts/build-rootfs-firecracker.sh                  # default 20 MiB rootfs
#   ROOTFS_SIZE_MB=32 scripts/build-rootfs-firecracker.sh
#   ROOTFS_OUT_DIR=/tmp/foo scripts/build-rootfs-firecracker.sh

set -euo pipefail

# ── Inputs ────────────────────────────────────────────────────────────────────
ROOTFS_SIZE_MB="${ROOTFS_SIZE_MB:-20}"
ROOTFS_OUT_DIR="${ROOTFS_OUT_DIR:-$(pwd)/target/firecracker-rootfs}"
ROOTFS_IMG="${ROOTFS_OUT_DIR}/rootfs.ext4"
ROOTFS_SHA="${ROOTFS_OUT_DIR}/rootfs.ext4.sha256"
BUILDER_TAG="vibecody-firecracker-rootfs-builder:latest"
ALPINE_VERSION="${ALPINE_VERSION:-3.20}"

# ── Preflight ─────────────────────────────────────────────────────────────────
if ! command -v docker >/dev/null 2>&1; then
    echo "✗ Docker is required to build the rootfs (cross-platform mkfs.ext4 + loop)."
    echo "  Install Docker Desktop on macOS/Windows or 'docker' on Linux."
    exit 1
fi

echo "→ Firecracker rootfs builder"
echo "  size:    ${ROOTFS_SIZE_MB} MiB"
echo "  out:     ${ROOTFS_IMG}"
echo "  alpine:  ${ALPINE_VERSION}"

mkdir -p "${ROOTFS_OUT_DIR}"

# ── Build context ─────────────────────────────────────────────────────────────
# Self-contained build dir so we don't ship the whole repo to Docker. The
# Dockerfile + a setup script that runs inside the container are all we need.
BUILD_CTX="$(mktemp -d)"
trap 'rm -rf "${BUILD_CTX}"' EXIT

# Setup script: runs inside the alpine container under bash, populates
# /sysroot with the minimal rootfs surface. Using bash (not /bin/sh)
# because we explicitly want brace expansion + arrays.
cat >"${BUILD_CTX}/build-sysroot.sh" <<'INNER_EOF'
#!/usr/bin/env bash
set -euo pipefail

mkdir -p \
    /sysroot/bin \
    /sysroot/sbin \
    /sysroot/etc \
    /sysroot/proc \
    /sysroot/sys \
    /sysroot/dev \
    /sysroot/tmp \
    /sysroot/root \
    /sysroot/work \
    /sysroot/run \
    /sysroot/lib \
    /sysroot/usr/bin \
    /sysroot/usr/sbin \
    /sysroot/usr/lib \
    /sysroot/usr/share/ca-certificates \
    /sysroot/usr/share/zoneinfo \
    /sysroot/var/log \
    /sysroot/var/run

# Resolve actual busybox path (Alpine 3.20+ puts it at /bin/busybox; older
# images put it at /bin/busybox.suid; symlinks may vary).
BUSYBOX="$(command -v busybox)"
test -n "${BUSYBOX}" || { echo "busybox not found"; exit 1; }
cp -aL "${BUSYBOX}" /sysroot/bin/busybox

# bash + interpreter
cp -aL "$(command -v bash)" /sysroot/bin/bash
cp -aL /usr/bin/env /sysroot/usr/bin/env

# Resolve every applet busybox knows into /sysroot/bin
for applet in $(/sysroot/bin/busybox --list); do
    ln -sf busybox /sysroot/bin/${applet}
done

# Re-shim /bin/sh to bash so any "#!/bin/sh" script inside still gets
# a proper shell — busybox sh is fine for trivial commands but bash
# is what most agent tools assume.
ln -sf bash /sysroot/bin/sh

# Account & group files — without these, even `whoami` will fail.
cp -aL /etc/passwd /etc/group /sysroot/etc/

# Musl loader + libc — without these, dynamically-linked binaries
# (everything from Alpine) fail at exec().
for lib in /lib/ld-musl-*.so.* /lib/libc.musl-*.so.*; do
    [ -e "${lib}" ] && cp -aL "${lib}" /sysroot/lib/
done

# Readline + ncursesw — bash links these dynamically.
for lib in /usr/lib/libreadline.so.* /usr/lib/libncursesw.so.*; do
    [ -e "${lib}" ] && cp -aL "${lib}" /sysroot/usr/lib/
done

# TLS root certs so the agent can validate broker / outbound HTTPS
# certificates from inside the microVM.
if [ -d /etc/ssl ]; then
    cp -aL /etc/ssl /sysroot/etc/
fi
if [ -f /etc/ca-certificates.conf ]; then
    cp -aL /etc/ca-certificates.conf /sysroot/etc/
fi

# Provenance marker — used by integration tests + `vibecli doctor`
# to verify the rootfs at run time.
echo "vibecody-firecracker-rootfs:${ALPINE_VERSION:-unknown}" \
    > /sysroot/etc/vibe-rootfs-id
date -u +"%Y-%m-%dT%H:%M:%SZ" > /sysroot/etc/vibe-rootfs-built

# Drop any /sysroot/proc, /sys, /dev contents — mounted at runtime.
INNER_EOF
chmod +x "${BUILD_CTX}/build-sysroot.sh"

# ── Dockerfile ────────────────────────────────────────────────────────────────
# Two-stage build:
#   sysroot  → install BusyBox + bash + minimal libs into /sysroot
#   packer   → pack /sysroot into a Debian-built ext4 image via mke2fs -d
cat >"${BUILD_CTX}/Dockerfile" <<EOF
# syntax=docker/dockerfile:1.6

FROM alpine:${ALPINE_VERSION} AS sysroot
ARG ALPINE_VERSION=${ALPINE_VERSION}
ENV ALPINE_VERSION=\${ALPINE_VERSION}
RUN apk add --no-cache busybox bash ca-certificates
COPY build-sysroot.sh /usr/local/bin/build-sysroot.sh
RUN bash /usr/local/bin/build-sysroot.sh

FROM debian:bookworm-slim AS packer
RUN apt-get update -qq && \\
    apt-get install -y --no-install-recommends e2fsprogs && \\
    rm -rf /var/lib/apt/lists/*
COPY --from=sysroot /sysroot /sysroot
ARG SIZE_MB=20
WORKDIR /out
# mke2fs -d packs a directory into an ext4 image *without* needing a loop
# device — works inside Docker on macOS / Windows. -m 0 disables
# reserved-blocks (saves ~5% for tiny images).
RUN mke2fs -t ext4 -d /sysroot -m 0 -L vibe-rootfs /out/rootfs.ext4 \${SIZE_MB}M
EOF

# ── Build ─────────────────────────────────────────────────────────────────────
echo "→ Building Docker stages..."
docker build \
    --build-arg "ALPINE_VERSION=${ALPINE_VERSION}" \
    --build-arg "SIZE_MB=${ROOTFS_SIZE_MB}" \
    -t "${BUILDER_TAG}" \
    -f "${BUILD_CTX}/Dockerfile" \
    "${BUILD_CTX}"

# ── Extract the image bytes ───────────────────────────────────────────────────
echo "→ Extracting rootfs.ext4..."
container_id="$(docker create "${BUILDER_TAG}")"
trap 'docker rm -f "${container_id}" >/dev/null 2>&1 || true; rm -rf "${BUILD_CTX}"' EXIT
docker cp "${container_id}:/out/rootfs.ext4" "${ROOTFS_IMG}"

# ── Sign + report ─────────────────────────────────────────────────────────────
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${ROOTFS_IMG}" | awk '{print $1}' > "${ROOTFS_SHA}"
elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "${ROOTFS_IMG}" | awk '{print $1}' > "${ROOTFS_SHA}"
else
    echo "⚠ No sha256sum/shasum found — skipping signature"
fi

actual_size_bytes="$(wc -c < "${ROOTFS_IMG}" | tr -d '[:space:]')"
actual_size_mib="$(( (actual_size_bytes + 1024 * 1024 - 1) / (1024 * 1024) ))"

echo ""
echo "✓ Firecracker rootfs built"
echo "  path:    ${ROOTFS_IMG}"
echo "  size:    ${actual_size_mib} MiB  (${actual_size_bytes} bytes)"
if [ -f "${ROOTFS_SHA}" ]; then
    echo "  sha256:  $(cat "${ROOTFS_SHA}")"
fi
echo ""
echo "→ Next step: feed this image to FirecrackerSandbox::rootfs_image()"
echo "  (slice F2 — microVM lifecycle)"

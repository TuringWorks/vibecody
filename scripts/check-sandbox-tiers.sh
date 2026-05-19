#!/usr/bin/env bash
#
# scripts/check-sandbox-tiers.sh — probe the host for VibeCody sandbox-tier
# availability.
#
# Mirrors the logic that `vibecli doctor` will report once the H6 daemon
# wiring lands (currently blocked by the Metal-toolchain `cargo build`
# issue on macOS dev boxes). Until then, this script gives operators a
# usable preview of which tiers will work on a given host.
#
# Exit code:
#   0 — at least Tier-0 (Native) works
#   1 — no usable tier (should never happen on a supported OS)
#
# Output:
#   one line per tier, plus a JSON object at the end so callers can pipe
#   `scripts/check-sandbox-tiers.sh --json` into jq.
#
# Slice: companion to `docs/design/sandbox-tiers/04-hyperlight-tier.md` §H6
# and `docs/design/sandbox-tiers/03-firecracker-tier.md` §F2-onwards.

set -euo pipefail

JSON_OUT=0
if [[ "${1:-}" == "--json" ]]; then
    JSON_OUT=1
fi

OS="$(uname -s)"
ARCH="$(uname -m)"

# ── Tier-0 Native ─────────────────────────────────────────────────────────────
tier_native="unsupported"
tier_native_backend=""
tier_native_note=""
case "${OS}" in
    Linux)
        if command -v bwrap >/dev/null 2>&1; then
            tier_native="ok"
            tier_native_backend="bwrap"
            tier_native_note="$(bwrap --version 2>&1 | head -1)"
        else
            tier_native="missing"
            tier_native_note="bwrap not installed — apt install bubblewrap"
        fi
        ;;
    Darwin)
        if command -v sandbox-exec >/dev/null 2>&1; then
            tier_native="ok"
            tier_native_backend="sandbox-exec"
            tier_native_note="macOS Seatbelt"
        else
            tier_native="missing"
            tier_native_note="sandbox-exec not found (unusual for macOS)"
        fi
        ;;
    MINGW*|MSYS*|CYGWIN*)
        # Best-effort detection — actual AppContainer probe needs a Win32
        # API call; the daemon will do the real check.
        tier_native="probable"
        tier_native_backend="AppContainer"
        tier_native_note="Win32 AppContainer (real probe requires daemon)"
        ;;
    *)
        tier_native_note="unknown OS ${OS}"
        ;;
esac

# ── Tier-1 WASI (Wasmtime, in-process) ────────────────────────────────────────
# Always available where Rust is — but reporting that means rustc isn't
# the right gate (the binary embeds wasmtime). Report ok on any supported
# OS+arch combination.
tier_wasi="ok"
tier_wasi_note="in-process; H5 fuel+epoch enforced"
case "${ARCH}" in
    x86_64|aarch64|arm64) ;;
    *) tier_wasi="degraded"; tier_wasi_note="arch ${ARCH} not validated" ;;
esac

# ── Tier-2 Hyperlight (Linux KVM/mshv, Windows WHP) ───────────────────────────
tier_hyperlight="unsupported"
tier_hyperlight_note=""
case "${OS}" in
    Linux)
        if [[ -e /dev/kvm ]]; then
            if [[ -r /dev/kvm && -w /dev/kvm ]]; then
                tier_hyperlight="ok"
                tier_hyperlight_note="/dev/kvm readable"
            else
                tier_hyperlight="needs-perm"
                tier_hyperlight_note="/dev/kvm exists but not r/w by current user — usermod -aG kvm"
            fi
        elif [[ -e /dev/mshv ]]; then
            tier_hyperlight="ok"
            tier_hyperlight_note="Hyper-V mshv driver"
        else
            tier_hyperlight="missing"
            tier_hyperlight_note="no /dev/kvm or /dev/mshv — install qemu-kvm or run on bare metal"
        fi
        ;;
    MINGW*|MSYS*|CYGWIN*)
        tier_hyperlight="probable"
        tier_hyperlight_note="Windows Hypervisor Platform (real probe requires daemon)"
        ;;
    Darwin)
        tier_hyperlight="unsupported"
        tier_hyperlight_note="upstream Hyperlight doesn't target Hypervisor.framework; falls back to Tier-1"
        ;;
esac

# ── Tier-3 Firecracker (Linux KVM only) ───────────────────────────────────────
tier_firecracker="unsupported"
tier_firecracker_note=""
case "${OS}" in
    Linux)
        firecracker_bin="$(command -v firecracker 2>/dev/null || true)"
        if [[ -n "${firecracker_bin}" ]] && [[ -e /dev/kvm ]] && [[ -r /dev/kvm && -w /dev/kvm ]]; then
            tier_firecracker="ok"
            tier_firecracker_note="$(${firecracker_bin} --version 2>&1 | head -1)"
        elif [[ -z "${firecracker_bin}" ]]; then
            tier_firecracker="missing"
            tier_firecracker_note="firecracker binary not installed — github.com/firecracker-microvm/firecracker/releases"
        else
            tier_firecracker="needs-perm"
            tier_firecracker_note="/dev/kvm not r/w by current user"
        fi
        ;;
    Darwin|MINGW*|MSYS*|CYGWIN*)
        tier_firecracker="unsupported"
        tier_firecracker_note="Firecracker is Linux-only; falls back to Tier-0"
        ;;
esac

# Also check if the rootfs has been built (F1 artifact).
rootfs_status="absent"
rootfs_path=""
candidate="${PWD}/target/firecracker-rootfs/rootfs.ext4"
if [[ -f "${candidate}" ]]; then
    rootfs_status="present"
    rootfs_path="${candidate}"
fi

# ── Output ────────────────────────────────────────────────────────────────────
if [[ "${JSON_OUT}" -eq 1 ]]; then
    cat <<EOF
{
  "os":       "${OS}",
  "arch":     "${ARCH}",
  "tiers": {
    "native": {
      "status":  "${tier_native}",
      "backend": "${tier_native_backend}",
      "note":    "$(printf '%s' "${tier_native_note}" | sed 's/"/\\"/g')"
    },
    "wasi": {
      "status":  "${tier_wasi}",
      "note":    "$(printf '%s' "${tier_wasi_note}" | sed 's/"/\\"/g')"
    },
    "hyperlight": {
      "status":  "${tier_hyperlight}",
      "note":    "$(printf '%s' "${tier_hyperlight_note}" | sed 's/"/\\"/g')"
    },
    "firecracker": {
      "status":  "${tier_firecracker}",
      "note":    "$(printf '%s' "${tier_firecracker_note}" | sed 's/"/\\"/g')",
      "rootfs":  "${rootfs_status}",
      "rootfs_path": "${rootfs_path}"
    }
  }
}
EOF
else
    echo "VibeCody sandbox-tier probe"
    echo "  OS:                ${OS} (${ARCH})"
    echo ""
    printf "  Tier-0 Native:     %-12s — %s%s\n" \
        "${tier_native}" \
        "$( [[ -n "${tier_native_backend}" ]] && printf '%s — ' "${tier_native_backend}" )" \
        "${tier_native_note}"
    printf "  Tier-1 WASI:       %-12s — %s\n" "${tier_wasi}" "${tier_wasi_note}"
    printf "  Tier-2 Hyperlight: %-12s — %s\n" "${tier_hyperlight}" "${tier_hyperlight_note}"
    printf "  Tier-3 Firecracker:%-12s — %s\n" "${tier_firecracker}" "${tier_firecracker_note}"
    echo ""
    printf "  Firecracker rootfs: %s%s\n" "${rootfs_status}" \
        "$( [[ -n "${rootfs_path}" ]] && printf ' (%s)' "${rootfs_path}" )"
fi

# Exit non-zero only if Tier-0 is unusable.
case "${tier_native}" in
    ok|probable) exit 0 ;;
    *) exit 1 ;;
esac

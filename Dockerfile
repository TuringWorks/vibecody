# ── VibeCLI Multi-stage Dockerfile ──────────────────────────────────────────
# Build a statically-linked musl binary for minimal container images.
#
# Usage:
#   docker build -t vibecli .
#   docker run -p 7878:7878 vibecli serve --provider ollama --port 7878
#
# With Ollama sidecar:
#   docker compose up

# 1.96, not 1.88: yrs 0.27 (transitive via vibe-collab) uses `if let` guards,
# stabilised after 1.88, so the older image fails with
#   error[E0658]: `if let` guards are experimental
# 1.96 is what CI's `stable` toolchain resolves to and what the workspace is
# developed against. Pinned rather than `rust:1` so an image rebuild cannot
# silently change compilers — but that means this line needs bumping when the
# workspace starts relying on something newer.
FROM rust:1.96-bookworm AS builder

# Install musl cross-compilation tools
RUN apt-get update -qq && \
    apt-get install -y --no-install-recommends musl-tools pkg-config cmake perl && \
    rustup target add x86_64-unknown-linux-musl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependency builds: copy only manifests first.
# IMPORTANT: keep this list in sync with [workspace] members in /Cargo.toml —
# cargo refuses to resolve the workspace if any declared member is missing.
COPY Cargo.toml ./
COPY vibecli/vibecli-cli/Cargo.toml vibecli/vibecli-cli/Cargo.toml
COPY vibecli/crates/vibe-sandbox/Cargo.toml vibecli/crates/vibe-sandbox/Cargo.toml
COPY vibecli/crates/vibe-sandbox-native/Cargo.toml vibecli/crates/vibe-sandbox-native/Cargo.toml
COPY vibecli/crates/vibe-sandbox-firecracker/Cargo.toml vibecli/crates/vibe-sandbox-firecracker/Cargo.toml
COPY vibecli/crates/vibe-sandbox-hyperlight/Cargo.toml vibecli/crates/vibe-sandbox-hyperlight/Cargo.toml
COPY vibecli/crates/vibe-broker/Cargo.toml vibecli/crates/vibe-broker/Cargo.toml
COPY vibecoder/crates/vibe-core/Cargo.toml vibecoder/crates/vibe-core/Cargo.toml
COPY vibecoder/crates/vibe-ai/Cargo.toml vibecoder/crates/vibe-ai/Cargo.toml
COPY vibecoder/crates/vibe-infer/Cargo.toml vibecoder/crates/vibe-infer/Cargo.toml
COPY vibecoder/crates/vibe-lsp/Cargo.toml vibecoder/crates/vibe-lsp/Cargo.toml
COPY vibecoder/crates/vibe-extensions/Cargo.toml vibecoder/crates/vibe-extensions/Cargo.toml
COPY vibecoder/crates/vibe-collab/Cargo.toml vibecoder/crates/vibe-collab/Cargo.toml
COPY vibecoder/src-tauri/Cargo.toml vibecoder/src-tauri/Cargo.toml
COPY vibeaichat/src-tauri/Cargo.toml vibeaichat/src-tauri/Cargo.toml
COPY vibe-indexer/Cargo.toml vibe-indexer/Cargo.toml
COPY vibe-memory/Cargo.toml vibe-memory/Cargo.toml
COPY vibedesk/src-tauri/Cargo.toml vibedesk/src-tauri/Cargo.toml
COPY crates/vibe-desktop-settings/Cargo.toml crates/vibe-desktop-settings/Cargo.toml
COPY crates/vibe-desktop-voice/Cargo.toml crates/vibe-desktop-voice/Cargo.toml
COPY crates/vibe-profile-store/Cargo.toml crates/vibe-profile-store/Cargo.toml
COPY crates/vibe-embed/Cargo.toml crates/vibe-embed/Cargo.toml
COPY crates/vibe-eval/Cargo.toml crates/vibe-eval/Cargo.toml
COPY crates/vibe-sync-ext/Cargo.toml crates/vibe-sync-ext/Cargo.toml
COPY crates/vibe-alloc-count/Cargo.toml crates/vibe-alloc-count/Cargo.toml
COPY crates/vibe-http-pool/Cargo.toml crates/vibe-http-pool/Cargo.toml
COPY kodegraph/Cargo.toml kodegraph/Cargo.toml
COPY skilllensai-rs/Cargo.toml skilllensai-rs/Cargo.toml
COPY skilloptai-rs/Cargo.toml skilloptai-rs/Cargo.toml
COPY skillforgeai-rs/Cargo.toml skillforgeai-rs/Cargo.toml
COPY fluxo/fluxo-core/Cargo.toml fluxo/fluxo-core/Cargo.toml
COPY fluxo/fluxo-store/Cargo.toml fluxo/fluxo-store/Cargo.toml
COPY fluxo/fluxo-engine/Cargo.toml fluxo/fluxo-engine/Cargo.toml
COPY fluxo/fluxo-server/Cargo.toml fluxo/fluxo-server/Cargo.toml
COPY fluxo/fluxo-worker/Cargo.toml fluxo/fluxo-worker/Cargo.toml
COPY fluxo/fluxo-cli/Cargo.toml fluxo/fluxo-cli/Cargo.toml

# Create stub lib.rs / main.rs for each crate so cargo can resolve the dep graph
RUN mkdir -p vibecli/vibecli-cli/src && echo 'fn main() {}' > vibecli/vibecli-cli/src/main.rs && \
    mkdir -p vibecli/crates/vibe-sandbox/src && echo '' > vibecli/crates/vibe-sandbox/src/lib.rs && \
    mkdir -p vibecli/crates/vibe-sandbox-native/src && echo '' > vibecli/crates/vibe-sandbox-native/src/lib.rs && \
    mkdir -p vibecli/crates/vibe-sandbox-firecracker/src && echo '' > vibecli/crates/vibe-sandbox-firecracker/src/lib.rs && \
    mkdir -p vibecli/crates/vibe-sandbox-hyperlight/src && echo '' > vibecli/crates/vibe-sandbox-hyperlight/src/lib.rs && \
    mkdir -p vibecli/crates/vibe-broker/src && echo '' > vibecli/crates/vibe-broker/src/lib.rs && \
    mkdir -p vibecoder/crates/vibe-core/src && echo '' > vibecoder/crates/vibe-core/src/lib.rs && \
    mkdir -p vibecoder/crates/vibe-ai/src && echo '' > vibecoder/crates/vibe-ai/src/lib.rs && \
    mkdir -p vibecoder/crates/vibe-infer/src && echo '' > vibecoder/crates/vibe-infer/src/lib.rs && \
    mkdir -p vibecoder/crates/vibe-lsp/src && echo '' > vibecoder/crates/vibe-lsp/src/lib.rs && \
    mkdir -p vibecoder/crates/vibe-extensions/src && echo '' > vibecoder/crates/vibe-extensions/src/lib.rs && \
    mkdir -p vibecoder/crates/vibe-collab/src && echo '' > vibecoder/crates/vibe-collab/src/lib.rs && \
    mkdir -p vibecoder/src-tauri/src && echo '' > vibecoder/src-tauri/src/lib.rs && \
    mkdir -p vibeaichat/src-tauri/src && echo '' > vibeaichat/src-tauri/src/lib.rs && \
    mkdir -p vibe-indexer/src && echo 'fn main() {}' > vibe-indexer/src/main.rs && \
    mkdir -p vibe-memory/src && echo '' > vibe-memory/src/lib.rs && \
    mkdir -p vibedesk/src-tauri/src && echo '' > vibedesk/src-tauri/src/lib.rs && \
    mkdir -p vibedesk/src-tauri/src && echo 'fn main() {}' > vibedesk/src-tauri/src/main.rs && \
    mkdir -p crates/vibe-desktop-settings/src && echo '' > crates/vibe-desktop-settings/src/lib.rs && \
    mkdir -p crates/vibe-desktop-voice/src && echo '' > crates/vibe-desktop-voice/src/lib.rs && \
    mkdir -p crates/vibe-profile-store/src && echo '' > crates/vibe-profile-store/src/lib.rs && \
    mkdir -p crates/vibe-embed/src && echo '' > crates/vibe-embed/src/lib.rs && \
    mkdir -p crates/vibe-eval/src && echo '' > crates/vibe-eval/src/lib.rs && \
    mkdir -p crates/vibe-sync-ext/src && echo '' > crates/vibe-sync-ext/src/lib.rs && \
    mkdir -p crates/vibe-alloc-count/src && echo '' > crates/vibe-alloc-count/src/lib.rs && \
    mkdir -p crates/vibe-http-pool/src && echo '' > crates/vibe-http-pool/src/lib.rs && \
    mkdir -p kodegraph/src && echo '' > kodegraph/src/lib.rs && \
    mkdir -p skilllensai-rs/src && echo '' > skilllensai-rs/src/lib.rs && \
    mkdir -p skilloptai-rs/src && echo '' > skilloptai-rs/src/lib.rs && \
    mkdir -p skillforgeai-rs/src && echo '' > skillforgeai-rs/src/lib.rs && \
    mkdir -p fluxo/fluxo-core/src && echo '' > fluxo/fluxo-core/src/lib.rs && \
    mkdir -p fluxo/fluxo-store/src && echo '' > fluxo/fluxo-store/src/lib.rs && \
    mkdir -p fluxo/fluxo-engine/src && echo '' > fluxo/fluxo-engine/src/lib.rs && \
    mkdir -p fluxo/fluxo-server/src && echo '' > fluxo/fluxo-server/src/lib.rs && \
    mkdir -p fluxo/fluxo-server/src && echo 'fn main() {}' > fluxo/fluxo-server/src/main.rs && \
    mkdir -p fluxo/fluxo-worker/src && echo '' > fluxo/fluxo-worker/src/lib.rs && \
    mkdir -p fluxo/fluxo-cli/src && echo 'fn main() {}' > fluxo/fluxo-cli/src/main.rs

# Pre-build dependencies (cached layer)
RUN cargo build --release --package vibecli --target x86_64-unknown-linux-musl 2>/dev/null || true

# Now copy actual source over the stubs.
# IMPORTANT: any workspace member listed above must have its real
# src/ copied here, otherwise vibecli imports against the empty stub.
COPY vibecli/ vibecli/
COPY vibecoder/crates/ vibecoder/crates/
COPY vibecoder/src-tauri/src/ vibecoder/src-tauri/src/
COPY vibeaichat/src-tauri/src/ vibeaichat/src-tauri/src/
COPY vibe-indexer/src/ vibe-indexer/src/
COPY vibe-memory/src/ vibe-memory/src/
COPY vibedesk/src-tauri/src/ vibedesk/src-tauri/src/
COPY crates/vibe-desktop-settings/src/ crates/vibe-desktop-settings/src/
COPY crates/vibe-desktop-voice/src/ crates/vibe-desktop-voice/src/
COPY crates/vibe-profile-store/src/ crates/vibe-profile-store/src/
COPY crates/vibe-embed/src/ crates/vibe-embed/src/
COPY crates/vibe-eval/src/ crates/vibe-eval/src/
COPY crates/vibe-sync-ext/src/ crates/vibe-sync-ext/src/
COPY crates/vibe-alloc-count/src/ crates/vibe-alloc-count/src/
COPY crates/vibe-http-pool/src/ crates/vibe-http-pool/src/
COPY kodegraph/src/ kodegraph/src/
COPY skilllensai-rs/src/ skilllensai-rs/src/
COPY skilloptai-rs/src/ skilloptai-rs/src/
COPY skillforgeai-rs/src/ skillforgeai-rs/src/
COPY fluxo/fluxo-core/src/ fluxo/fluxo-core/src/
COPY fluxo/fluxo-store/src/ fluxo/fluxo-store/src/
COPY fluxo/fluxo-engine/src/ fluxo/fluxo-engine/src/
COPY fluxo/fluxo-server/src/ fluxo/fluxo-server/src/
COPY fluxo/fluxo-worker/src/ fluxo/fluxo-worker/src/
COPY fluxo/fluxo-cli/src/ fluxo/fluxo-cli/src/

# Build the real binary
RUN cargo build --release --package vibecli --target x86_64-unknown-linux-musl && \
    strip target/x86_64-unknown-linux-musl/release/vibecli

# ── Runtime stage: distroless-compatible scratch image ──────────────────────
FROM alpine:3.20 AS runtime

RUN addgroup -S vibecli && adduser -S vibecli -G vibecli

COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/vibecli /usr/local/bin/vibecli

# Copy skills library for agent use
COPY vibecli/vibecli-cli/skills/ /usr/share/vibecli/skills/

# Default config directory
RUN mkdir -p /home/vibecli/.vibecli && chown -R vibecli:vibecli /home/vibecli

USER vibecli
WORKDIR /workspace

ENV VIBECLI_SKILLS_DIR=/usr/share/vibecli/skills
ENV RUST_LOG=info

EXPOSE 7878

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -q --spider http://localhost:7878/health || exit 1

ENTRYPOINT ["vibecli"]
CMD ["serve", "--port", "7878"]

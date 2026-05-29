# ── VibeCLI Multi-stage Dockerfile ──────────────────────────────────────────
# Build a statically-linked musl binary for minimal container images.
#
# Usage:
#   docker build -t vibecli .
#   docker run -p 7878:7878 vibecli serve --provider ollama --port 7878
#
# With Ollama sidecar:
#   docker compose up

FROM rust:1.88-bookworm AS builder

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
COPY vibeui/crates/vibe-core/Cargo.toml vibeui/crates/vibe-core/Cargo.toml
COPY vibeui/crates/vibe-ai/Cargo.toml vibeui/crates/vibe-ai/Cargo.toml
COPY vibeui/crates/vibe-infer/Cargo.toml vibeui/crates/vibe-infer/Cargo.toml
COPY vibeui/crates/vibe-lsp/Cargo.toml vibeui/crates/vibe-lsp/Cargo.toml
COPY vibeui/crates/vibe-extensions/Cargo.toml vibeui/crates/vibe-extensions/Cargo.toml
COPY vibeui/crates/vibe-collab/Cargo.toml vibeui/crates/vibe-collab/Cargo.toml
COPY vibeui/src-tauri/Cargo.toml vibeui/src-tauri/Cargo.toml
COPY vibeapp/src-tauri/Cargo.toml vibeapp/src-tauri/Cargo.toml
COPY vibe-indexer/Cargo.toml vibe-indexer/Cargo.toml
COPY vibe-memory/Cargo.toml vibe-memory/Cargo.toml

# Create stub lib.rs / main.rs for each crate so cargo can resolve the dep graph
RUN mkdir -p vibecli/vibecli-cli/src && echo 'fn main() {}' > vibecli/vibecli-cli/src/main.rs && \
    mkdir -p vibecli/crates/vibe-sandbox/src && echo '' > vibecli/crates/vibe-sandbox/src/lib.rs && \
    mkdir -p vibecli/crates/vibe-sandbox-native/src && echo '' > vibecli/crates/vibe-sandbox-native/src/lib.rs && \
    mkdir -p vibecli/crates/vibe-sandbox-firecracker/src && echo '' > vibecli/crates/vibe-sandbox-firecracker/src/lib.rs && \
    mkdir -p vibecli/crates/vibe-sandbox-hyperlight/src && echo '' > vibecli/crates/vibe-sandbox-hyperlight/src/lib.rs && \
    mkdir -p vibecli/crates/vibe-broker/src && echo '' > vibecli/crates/vibe-broker/src/lib.rs && \
    mkdir -p vibeui/crates/vibe-core/src && echo '' > vibeui/crates/vibe-core/src/lib.rs && \
    mkdir -p vibeui/crates/vibe-ai/src && echo '' > vibeui/crates/vibe-ai/src/lib.rs && \
    mkdir -p vibeui/crates/vibe-infer/src && echo '' > vibeui/crates/vibe-infer/src/lib.rs && \
    mkdir -p vibeui/crates/vibe-lsp/src && echo '' > vibeui/crates/vibe-lsp/src/lib.rs && \
    mkdir -p vibeui/crates/vibe-extensions/src && echo '' > vibeui/crates/vibe-extensions/src/lib.rs && \
    mkdir -p vibeui/crates/vibe-collab/src && echo '' > vibeui/crates/vibe-collab/src/lib.rs && \
    mkdir -p vibeui/src-tauri/src && echo '' > vibeui/src-tauri/src/lib.rs && \
    mkdir -p vibeapp/src-tauri/src && echo '' > vibeapp/src-tauri/src/lib.rs && \
    mkdir -p vibe-indexer/src && echo 'fn main() {}' > vibe-indexer/src/main.rs && \
    mkdir -p vibe-memory/src && echo '' > vibe-memory/src/lib.rs

# Pre-build dependencies (cached layer)
RUN cargo build --release --package vibecli --target x86_64-unknown-linux-musl 2>/dev/null || true

# Now copy actual source
COPY vibecli/ vibecli/
COPY vibeui/crates/ vibeui/crates/
COPY vibeui/src-tauri/src/ vibeui/src-tauri/src/
COPY vibeapp/src-tauri/src/ vibeapp/src-tauri/src/
COPY vibe-indexer/src/ vibe-indexer/src/

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

# syntax=docker/dockerfile:1.4
#
# Containerfile for Stroma - Privacy-first decentralized trust network
# Compatible with both Docker and Podman

# ============================================================================
# Stage 1: Builder - Create static MUSL binary
# ============================================================================
FROM rust:1.93-alpine AS builder

# Install musl-dev and required build dependencies
# clang18-static + llvm18-dev provide libclang.a for wasmer's bindgen dependency
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static \
    clang18-static \
    llvm18-dev

# Set LIBCLANG_STATIC_PATH for clang-sys (wasmer -> bindgen -> clang-sys)
ENV LIBCLANG_STATIC_PATH=/usr/lib/llvm18/lib

# Create non-root user for build process
RUN addgroup -g 1000 builder && \
    adduser -D -u 1000 -G builder builder

# Set working directory
WORKDIR /build

# Switch to non-root user
USER builder

# Copy dependency manifests first for better layer caching
COPY --chown=builder:builder Cargo.toml Cargo.lock* ./

# Create minimal dummy source to cache dependencies
# Note: Only lib.rs and main.rs are needed for the stroma binary
# Spike exploration binaries are NOT included in container builds
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn placeholder() {}" > src/lib.rs

# Build dependencies only (this layer will be cached)
# ONLY build the main stroma binary, not spike explorations
RUN --mount=type=cache,target=/home/builder/.cargo/registry,uid=1000,gid=1000 \
    --mount=type=cache,target=/home/builder/.cargo/git,uid=1000,gid=1000 \
    --mount=type=cache,target=/build/target,uid=1000,gid=1000 \
    cargo build --release --target x86_64-unknown-linux-musl --bin stroma && \
    rm -rf src

# Copy actual source code
# .dockerignore filters out docs/spike/ to avoid including exploration code
COPY --chown=builder:builder . .

# Build the actual binary
# - Static linking with MUSL for portability
# - Target matches distroless/static architecture
# - All symbols stripped in Cargo.toml profile
# - ONLY builds stroma binary (--bin stroma), not spike explorations
RUN --mount=type=cache,target=/home/builder/.cargo/registry,uid=1000,gid=1000 \
    --mount=type=cache,target=/home/builder/.cargo/git,uid=1000,gid=1000 \
    --mount=type=cache,target=/build/target,uid=1000,gid=1000 \
    cargo build --release --target x86_64-unknown-linux-musl --bin stroma && \
    cp target/x86_64-unknown-linux-musl/release/stroma /tmp/stroma

# Verify static linking (should show "statically linked")
RUN file /tmp/stroma && \
    ldd /tmp/stroma 2>&1 | grep -q "not a dynamic executable" || \
    (echo "ERROR: Binary is not statically linked" && exit 1)

# ============================================================================
# Stage 2: Runtime - Minimal distroless image
# ============================================================================
FROM gcr.io/distroless/static:nonroot

# Metadata labels following OCI image spec
# Update these via CI when building releases
LABEL org.opencontainers.image.title="stroma" \
      org.opencontainers.image.description="Privacy-first decentralized trust network for Signal groups" \
      org.opencontainers.image.url="https://github.com/roder/stroma" \
      org.opencontainers.image.source="https://github.com/roder/stroma" \
      org.opencontainers.image.vendor="roder" \
      org.opencontainers.image.licenses="AGPL-3.0-or-later"

# Copy static binary from builder
# distroless/static:nonroot runs as uid/gid 65532 by default
COPY --from=builder --chown=65532:65532 /tmp/stroma /usr/local/bin/stroma

# distroless images don't have a shell, so ENTRYPOINT is the only way to run
ENTRYPOINT ["/usr/local/bin/stroma"]

# Default command arguments (can be overridden)
CMD []

# Expose no ports by default (add EXPOSE directives based on actual service needs)
# Security: Only expose what's necessary

# Health check (commented - implement based on actual service requirements)
# HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
#   CMD ["/usr/local/bin/stroma", "health"]

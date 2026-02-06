# Container Build Guide

**Status**: Dockerfile and Containerfile are identical (symlink) for Docker/Podman compatibility.

## Quick Start

### Docker Users
```bash
docker build -t stroma:latest .
docker run -d -v stroma-data:/data stroma:latest
```

### Podman Users
```bash
podman build -t stroma:latest .
podman run -d -v stroma-data:/data stroma:latest
```

Both commands work identically - `Containerfile` is a symlink to `Dockerfile` for universal compatibility.

## Architecture

### Two-Stage Build

**Stage 1: Builder (rust:1.93-alpine)**
- Static MUSL binary compilation
- Dependency caching for fast rebuilds
- Non-root builder user (uid/gid 1000)
- **Only builds `stroma` binary** - spike explorations excluded

**Stage 2: Runtime (distroless/static:nonroot)**
- Minimal attack surface (no shell, no package manager)
- Non-root execution (uid/gid 65532)
- Only contains the static `stroma` binary
- ~5MB final image size

## Build Options

### Local Development Build
```bash
# Docker
docker build -t stroma:dev .

# Podman
podman build -t stroma:dev .
```

### Production Build with Metadata
```bash
# Docker
docker build \
  --label org.opencontainers.image.version="1.0.0" \
  --label org.opencontainers.image.revision="$(git rev-parse HEAD)" \
  --label org.opencontainers.image.created="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
  -t stroma:1.0.0 \
  .

# Podman (identical)
podman build \
  --label org.opencontainers.image.version="1.0.0" \
  --label org.opencontainers.image.revision="$(git rev-parse HEAD)" \
  --label org.opencontainers.image.created="$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
  -t stroma:1.0.0 \
  .
```

### Multi-Platform Build (Docker Buildx)
```bash
# Create builder
docker buildx create --name multiarch --use

# Build for multiple platforms
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t ghcr.io/roder/stroma:latest \
  --push \
  .
```

**Note**: Podman buildah has similar multi-arch capabilities via `buildah manifest`.

## Security Features

### Build-Time Security
- ✅ Non-root builder user (least privilege)
- ✅ Dependency caching (reproducible builds)
- ✅ Static linking verification (no dynamic dependencies)
- ✅ Cargo.toml profile: `strip = true` (remove debug symbols)

### Runtime Security
- ✅ Distroless base (no shell, no package manager)
- ✅ Non-root execution (uid/gid 65532)
- ✅ Read-only root filesystem (add `--read-only` flag when running)
- ✅ No exposed ports by default
- ✅ Static MUSL binary (no libc dependencies)

### Recommended Runtime Flags

**Docker:**
```bash
docker run -d \
  --read-only \
  --cap-drop=ALL \
  --security-opt=no-new-privileges:true \
  -v stroma-data:/data \
  stroma:latest
```

**Podman:**
```bash
podman run -d \
  --read-only \
  --cap-drop=ALL \
  --security-opt=no-new-privileges:true \
  -v stroma-data:/data \
  stroma:latest
```

## Image Metadata

**OCI Labels** (org.opencontainers.image.*):
- `title`: "stroma"
- `description`: "Privacy-first decentralized trust network for Signal groups"
- `url`: https://github.com/roder/stroma
- `source`: https://github.com/roder/stroma
- `vendor`: "roder" (GitHub account)
- `licenses`: "AGPL-3.0-or-later"

**Additional labels** (set via CI/CD at build time):
- `version`: Semantic version (e.g., "1.0.0")
- `revision`: Git commit SHA
- `created`: ISO-8601 timestamp

## CI/CD Integration

The GitHub Actions release workflow (`.github/workflows/release.yml`) builds:
1. **Static binary** via `cargo build --release --target x86_64-unknown-linux-musl`
2. **Container image** via this Dockerfile/Containerfile

Both artifacts use the **same build configuration**:
- Rust 1.93+ with MUSL target
- Static linking verification
- Symbol stripping (Cargo.toml profile)
- Release optimizations (`opt-level = "z"`, `lto = true`)

## Docker vs Podman Compatibility

**100% compatible** - No differences in build or runtime behavior.

**File Structure**:
- `Dockerfile` - Primary file (tracked in git)
- `Containerfile` - Symlink to `Dockerfile` (also tracked in git)
- `.dockerignore` - Shared by both tools

**Why symlink approach**:
- ✅ Maintains single source of truth (DRY principle)
- ✅ No duplication or drift between files
- ✅ Podman users can run `podman build .` (auto-detects `Containerfile`)
- ✅ Docker users can run `docker build .` (auto-detects `Dockerfile`)
- ✅ Both communities feel welcome

**Tool-specific conventions**:
- Docker: Prefers `Dockerfile` (but supports `Containerfile`)
- Podman: Prefers `Containerfile` (but supports `Dockerfile`)
- OCI spec: Neutral (both names valid)

## Troubleshooting

### Build Cache Issues
```bash
# Docker: Clear cache and rebuild
docker build --no-cache -t stroma:latest .

# Podman: Clear cache and rebuild
podman build --no-cache -t stroma:latest .
```

### Static Linking Verification Failure
If you see: `ERROR: Binary is not statically linked`

**Diagnosis**:
```bash
# Check binary type
file target/x86_64-unknown-linux-musl/release/stroma

# Should show: "statically linked"
# If it shows "dynamically linked", you have a dependency issue
```

**Common causes**:
- Missing `musl-dev` in builder stage
- Wrong target (should be `x86_64-unknown-linux-musl`)
- C dependency not configured for static linking

### Build Performance

**First build**: 5-10 minutes (downloads all dependencies)
**Subsequent builds**: 1-2 minutes (cached dependencies)

**Optimization tips**:
- Use BuildKit: `DOCKER_BUILDKIT=1 docker build .`
- Mount caches: Already configured in Dockerfile via `--mount=type=cache`
- Parallel builds: Use `docker buildx` or `podman build --jobs`

## Verification

### Verify Image Contents
```bash
# Docker
docker run --rm stroma:latest --version

# Podman
podman run --rm stroma:latest --version
```

### Inspect Image Metadata
```bash
# Docker
docker inspect stroma:latest | jq '.[0].Config.Labels'

# Podman
podman inspect stroma:latest | jq '.[0].Labels'
```

### Verify Static Binary
```bash
# Docker
docker run --rm stroma:latest sh -c "file /usr/local/bin/stroma"
# Note: This will fail (distroless has no shell), which proves security

# Better approach: Check during build stage
docker build --target builder -t stroma:builder .
docker run --rm stroma:builder file /tmp/stroma
```

## Size Comparison

| Component | Size |
|-----------|------|
| Builder stage | ~1.5GB (Rust toolchain + dependencies) |
| Final runtime image | ~5MB (distroless + static binary) |
| Static binary alone | ~3MB (MUSL, stripped, LTO) |

**Storage saved**: The multi-stage build discards 99.7% of build artifacts.

## References

- **Dockerfile Syntax**: https://docs.docker.com/reference/dockerfile/
- **Containerfile (OCI)**: https://github.com/containers/common/blob/main/docs/Containerfile.5.md
- **Distroless Images**: https://github.com/GoogleContainerTools/distroless
- **OCI Image Spec**: https://github.com/opencontainers/image-spec
- **Rust MUSL Target**: https://rust-lang.github.io/rustup/installation/other.html

---

**Last Updated**: 2026-02-06

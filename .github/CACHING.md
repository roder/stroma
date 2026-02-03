# GitHub Actions Caching Strategy

This document describes the caching strategy used across all GitHub Actions workflows.

## Overview

All workflows use [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache) v2 for Rust dependency caching. This action automatically caches:
- Cargo registry index
- Cargo registry cache
- Cargo git dependencies
- Compiled dependencies (target/debug/deps, target/release/deps)

## Shared Key Strategy

Each workflow uses a unique `shared-key` to scope its cache while sharing within the workflow:

| Workflow | Shared Key | Rationale |
|----------|-----------|-----------|
| CI | `ci-stable` | Shared across all CI jobs (format, lint, test, coverage) for maximum reuse |
| Security | `security-binary-size` | Isolated for binary size monitoring to ensure consistent measurements |
| Release | `release-stable` | Isolated for release builds to avoid interference from debug builds |

## Why Swatinem Only?

We previously used manual `actions/cache@v3` entries for registry and index caching, but this is redundant. Swatinem/rust-cache handles all Rust caching needs automatically and more efficiently.

**Do not add manual cache steps** - they will conflict with Swatinem's automatic caching.

## Cache Isolation

Different workflows use different shared keys to prevent cache pollution:
- **CI builds** use debug mode and need fast incremental compilation
- **Security builds** need consistent binary sizes for monitoring
- **Release builds** use release mode with different optimization levels

Sharing caches between these contexts would cause unnecessary recompilation and unreliable measurements.

## Cache Lifetime

GitHub Actions caches expire after 7 days of inactivity. Active branches maintain their caches through regular commits.

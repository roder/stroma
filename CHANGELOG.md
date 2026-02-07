# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Property tests for attestation module
- Comprehensive test coverage for matchmaker module
- Property tests for crypto module (98.84% coverage)
- Test coverage for gatekeeper module (93-100%)
- Test coverage for audit_trail module
- EncryptedTrustNetworkState module
- /mesh replication command with real persistence queries
- /vouch flow admission checks
- /reject-intro command for Signal integration
- Audit logging for config change proposals
- Signal integration via presage (forked libsignal-service-rs with poll support)
- Bridge problem test for tight cluster separation

### Changed
- Canonicalized 'assessor' terminology throughout codebase
- Enabled presage dependency for Signal integration

### Fixed
- Format long format! macro call across multiple lines
- Added protoc to Format & Lint CI workflow

### Security
- Security constraint: DO NOT USE presage-store-sqlite (stores ALL messages - server seizure risk)
- Replaced unmaintained dependencies:
  - paste → pastey fork (RUSTSEC-2024-0436)
  - rustls-pemfile → rustls-pki-types (RUSTSEC-2025-0134)
  - bincode → bincode2 fork (RUSTSEC-2025-0141)

## [0.1.0] - Unreleased

Initial development version of stroma - a ZK-proof based trust network system built on Freenet.

### Core Features
- Trust network state management with encrypted persistence
- ZK-STARK based attestation system
- Signal integration for secure communications
- Mesh replication commands
- Audit trail logging
- Gatekeeper admission control
- Matchmaker module for trust network graph operations

### Infrastructure
- Comprehensive test coverage with property-based testing
- CI/CD pipeline with format and lint checks
- Release workflow with static binary builds for Linux x86_64
- SHA256 checksums for binary verification

[Unreleased]: https://github.com/stromarig/stromarig/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/stromarig/stromarig/releases/tag/v0.1.0

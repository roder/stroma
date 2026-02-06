# Rustls-Pemfile Crate Patch

## Purpose

This is a local Cargo patch that resolves **RUSTSEC-2025-0134** by replacing the unmaintained `rustls-pemfile` crate with the actively maintained `rustls-pki-types` crate.

## Background

- The `rustls-pemfile` crate (v1.0.4) was archived in August 2025
- It's a transitive dependency through: `stroma → freenet → hickory-proto → rustls-pemfile`
- [rustls-pki-types](https://crates.io/crates/rustls-pki-types) 1.9+ includes the same PEM parsing functionality that was in rustls-pemfile
- The `rustls-pemfile` crate itself became a thin wrapper around `rustls-pki-types` in version 2.2.0

## Implementation

Since Cargo's patch mechanism requires matching package names, we created a local wrapper crate that:

1. **Name**: `rustls-pemfile` (matches the crate being patched)
2. **Dependency**: `rustls-pki-types = "1.9"` (the maintained replacement)
3. **API**: Provides the old `rustls-pemfile` API on top of `rustls-pki-types`

This allows Cargo to patch all usages of `rustls-pemfile` throughout the dependency tree with our wrapper, which uses `rustls-pki-types` internally.

## API Mapping

The wrapper provides these common `rustls-pemfile` functions:

- `certs()` - Extract certificates from PEM
- `pkcs8_private_keys()` - Extract PKCS8 private keys
- `rsa_private_keys()` - Extract PKCS1/RSA private keys
- `ec_private_keys()` - Extract SEC1/EC private keys
- `read_one()` - Read a single PEM item
- `read_all()` - Read all PEM items

All functions are implemented using the `PemObject::pem_slice_iter()` API from `rustls-pki-types`.

## Verification

Confirm the patch is active:

```bash
cargo tree | grep rustls-pemfile
```

Should show:
```
rustls-pemfile v1.0.4 (/path/to/patches/rustls-pemfile)
  └── rustls-pki-types v1.9.x
```

Verify RUSTSEC-2025-0134 is resolved:

```bash
cargo deny check advisories
```

Should NOT report any issues related to `rustls-pemfile`.

## References

- [RUSTSEC-2025-0134](https://rustsec.org/advisories/RUSTSEC-2025-0134.html)
- [rustls-pki-types repository](https://github.com/rustls/pki-types)
- [rustls-pemfile migration guide](https://github.com/rustls/pemfile)
- [Cargo patch documentation](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html)

## Maintenance

This patch can be removed once:
1. `hickory-proto` migrates to `rustls-pki-types` directly, OR
2. A newer version of `freenet` uses an updated `hickory-proto` that doesn't depend on `rustls-pemfile`

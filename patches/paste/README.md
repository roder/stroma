# Paste Crate Patch

## Purpose

This is a local Cargo patch that resolves **RUSTSEC-2024-0436** by replacing the unmaintained `paste` crate with the actively maintained `pastey` fork.

## Background

- The `paste` crate (v1.0.15) was archived by its maintainer in October 2024
- It's a transitive dependency through: `stroma → freenet → wasmer → paste`
- [pastey](https://github.com/AS1100K/pastey) is an actively maintained fork and drop-in replacement

## Implementation

Since Cargo's patch mechanism requires matching package names, we created a local wrapper crate that:

1. **Name**: `paste` (matches the crate being patched)
2. **Dependency**: `pastey = "0.2"` (the maintained fork)
3. **Export**: Re-exports `pastey::paste!` macro

This allows Cargo to patch all usages of `paste` throughout the dependency tree with our wrapper, which in turn uses `pastey`.

## Verification

Confirm the patch is active:

```bash
cargo tree | grep paste
```

Should show:
```
paste v1.0.15 (/path/to/patches/paste)
  └── pastey v0.2.1 (proc-macro)
```

Verify RUSTSEC-2024-0436 is resolved:

```bash
cargo deny check advisories
```

Should NOT report any issues related to `paste`.

## References

- [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436)
- [pastey repository](https://github.com/AS1100K/pastey)
- [Cargo patch documentation](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html)

## Maintenance

This patch can be removed once wasmer migrates to `pastey` or an equivalent maintained alternative upstream.

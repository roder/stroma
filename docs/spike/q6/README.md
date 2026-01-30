# Q6: Proof Storage Decision

**Question**: Should we store STARK proofs in contract state?

**Status**: DECISION (based on Q4)

## Background

Q6 is not a test - it's a design decision that depends on Q4's answer.

The decision matrix:

| Q4 Result | Q6 Decision | Rationale |
|-----------|-------------|-----------|
| Contract verifies | Don't store proofs | Contract verifies on demand |
| Client/Bot verifies | Store outcomes only | Bot verifies, contract trusts |
| Partial (slow Wasm) | Store proofs temporarily | Verify once, then delete |

## Q4 Result

**Q4 Decision**: PARTIAL (Bot-side verification)

Winterfell Wasm support is experimental. Bot verifies proofs natively.

## Q6 Decision

**Store outcomes only** - Proofs are ephemeral, contract stores only the verified outcome.

See `RESULTS.md` for detailed rationale.

## Files

- `README.md` - This file (context)
- `RESULTS.md` - Decision and rationale

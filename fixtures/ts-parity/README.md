# TS Parity Fixtures

Fixtures in this folder are generated from upstream ghost-cursor (TS) to compare with Rust output.

## Source of Truth
- Upstream repo: Xetera/ghost-cursor
- Pinned commit/version are recorded in `index.json`.

## Generate
1. Build upstream once:
   - `cd vendor/ghost-cursor-ts`
   - `npx tsc -p tsconfig.build.json`
2. Generate fixtures from repo root:
   - `node scripts/ts-parity/generate-fixtures.js`

## Files
- `index.json`: generator metadata and fixture inventory
- `*.json`: per-case fixture data

## Notes
- These fixtures are deterministic on TS side using a seeded `Math.random` override.
- Rust side currently uses thread RNG, so strict point-by-point parity is not guaranteed for random-spread cases.
- Use these fixtures for invariant checks first (counts/endpoints/timestamp monotonicity), then tighten over time.

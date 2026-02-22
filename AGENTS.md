# AGENTS.md

## Purpose
This file defines local testing expectations for changes in this repository.
If you modify code, run the relevant checks below before asking for review or merge.

## CI-Parity Checks (Required)
CI currently validates frontend tests and TypeScript compilation separately. Run both locally:

```bash
npm run test
npx tsc
```

Treat both as mandatory for TypeScript/frontend changes.

## Change-Based Test Matrix
Use this matrix to decide what to run in addition to the CI-parity checks.

### TypeScript / UI / App Logic (`src/**`, `index.html`, `style.css`)
Run:

```bash
npm run test
npx tsc
```

### WASM / Rust (`wasm/**`)
Run:

```bash
cd wasm
HOST_TARGET=$(rustc -vV | sed -n 's|host: ||p')
cargo test --target "$HOST_TARGET"
wasm-pack test --headless --chrome
cd ..
npm run build:wasm:prod
npm run test
npx tsc
```

### Full Production Readiness Check
Use this when touching both frontend and wasm, or before a release:

```bash
npm run build:wasm:prod
npm run test
npx tsc
npx vite build
```

## Notes
- Do not skip `npx tsc` just because tests pass; type errors can still fail CI.
- Prefer small, focused tests for new behavior and bugfixes.

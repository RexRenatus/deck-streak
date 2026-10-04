---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# CI builds the module and holds it to 8000000 bytes, and the page admits WebAssembly compilation

## Context and Problem Statement

ADR-336 sets the web engine's budget at 8000000 bytes gzip -9 for the shipped module plus its JS
bindings, and requires that the page's content security policy admit WebAssembly compilation and
nothing else new. Neither exists: no job builds the module, and `web/app/svelte.config.js`'s
`kit.csp` sets `script-src` to `self` and Telegram only, which refuses `WebAssembly.compile` and
`instantiate`. Two decisions follow: where and how the budget is measured, and how the policy
admits the module.

## Decision Drivers

- The gate measures the files a browser downloads, after every size step, by the arithmetic
  ADR-336 names.
- A missing or empty file is never a pass.
- The tools the measurement uses are pinned, so a figure is reproducible.
- The local gate stays runnable without a wasm32 toolchain or browsers.
- The policy grows by the one source the module needs.

## Considered Options (the alternatives it was chosen against)

### Where the budget is measured

- A `web-engine` job in `.github/workflows/ci.yml` that builds the module, runs the size gate and the browser tests, and joins the `ci` aggregate: chosen because CI then holds every change to the budget, and the job installs the wasm32 target, `wasm-bindgen`, `wasm-opt` and the browsers only where they are used.
- A stage of `scripts/check.sh`: rejected because every local gate run would then need the wasm32 target, the two CLIs and two browsers, for a measurement CI already holds.
- A job on the fork: rejected because the fork carries no workflow (ADR-346), and the module that ships is DeckStreak's crate, not the engine alone.

### What is measured, and how

- Each file after `wasm-bindgen --target web` and `wasm-opt -Oz`, compressed by `gzip -9` alone, the two sizes summed and compared with `<=` 8000000; brotli at quality 11 printed beside each, not gated: chosen because these are the bytes a browser downloads, by ADR-336's arithmetic, and brotli is the figure a server that serves it would see.
- The cargo `cdylib` before the size pass: rejected because it is not what ships, and it reads larger than the shipped module.
- One archive of both files: rejected because a browser fetches them separately, and an archive's compression shares a dictionary they never share.
- Gate on brotli: rejected because ADR-336 accepted the budget in gzip -9, and the host may serve either.

### How the policy admits the module

- `'wasm-unsafe-eval'` added to `kit.csp`'s `script-src`, pinned by `csp.test.ts`: chosen because it admits WebAssembly compilation and nothing else, and the page's script policy already lives there.
- `'unsafe-eval'`: rejected because it also admits `eval` and `new Function`.
- A `script-src` in the host's response header: rejected because the header is deploy-class configuration, which carries no `script-src` today, and two policies on one page intersect, so both would need the source.

## Decision Outcome

Chosen options: the first under each heading above.

- **The job.** `web-engine` runs on `ubuntu-latest`: it adds the `wasm32-unknown-unknown` target,
  installs `protoc`, `wasm-bindgen-cli` at the crate's exact version and `wasm-opt` by checksum,
  runs `scripts/web-engine-build.sh`, then `scripts/web-engine-size.py`, then the browser tests
  (`pnpm test:engine`) in Chromium and WebKit. The `ci` aggregate needs it.
- **The build.** `cargo build --release --target wasm32-unknown-unknown -p deck-streak-web-engine`,
  then `wasm-bindgen --target web`, then `wasm-opt -Oz` with the post-MVP features the module uses.
  The two files are `deck_streak_web_engine_bg.wasm` and `deck_streak_web_engine.js`.
- **The gate.** `scripts/web-engine-size.py` prints each file's raw, `gzip -9` and brotli sizes and
  the `gzip -9` total against 8000000. It exits 0 when the total is at most the budget, 1 above it,
  and 2 (VOID) when a file is missing or empty.
- **The policy.** `script-src` reads `'self' https://telegram.org 'wasm-unsafe-eval'`; every other
  directive is unchanged.

### Consequences

- Good, because a change that grows the module past its budget fails by name in CI.
- Good, because the browser tests run over the module the gate measured.
- Bad, because the job adds a wasm32 build of the engine to every pull request's CI time.
- Bad, because the local gate does not measure the module; a builder measures it with the same
  two scripts.

### Confirmation

SPEC-338's A3 to A6 and A16, and its section 7's size figures.

## What would make this wrong

- The module's size moves with the toolchain rather than the code, so the figure is not
  reproducible from the pins.
- A browser stops honouring `'wasm-unsafe-eval'` in `script-src`, which the browser tests would
  show as a refused module.

## More Information

ADR-336 (the budget), ADR-346, ADR-348; SPEC-335, SPEC-338. CSP Level 3's `'wasm-unsafe-eval'`
(https://w3c.github.io/webappsec-csp/#wasm-unsafe-eval) and SvelteKit's `kit.csp`
(https://svelte.dev/docs/kit/configuration#csp).

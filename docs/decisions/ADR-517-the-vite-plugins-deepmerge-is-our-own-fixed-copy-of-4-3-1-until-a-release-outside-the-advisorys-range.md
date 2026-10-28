---
status: "accepted"
---

# ADR-517: the Vite plugin's deepmerge is our own fixed copy of 4.3.1 until a release outside the advisory's range

GHSA-gcqx-w48m-w2h4 affects `deepmerge` 4.2.0 to 4.3.1, and no release fixes it. SvelteKit 3
requires `@sveltejs/vite-plugin-svelte` 7, every release of which declares `deepmerge ^4.3.1`, so
the web advisory audit cannot clear by an update. This decision takes the plugin's `deepmerge`
from our own fixed copy of 4.3.1 in `vendor/deepmerge`, through one override scoped to the
plugin's major, until upstream releases a version outside the advisory's range (SPEC-499).

## Context and Problem Statement

- In `mergeObject`, deepmerge 4.3.1 assigns each key with `destination[key] = ...`. An own
  `__proto__` key in a merged source, the shape `JSON.parse` makes, therefore sets the merged
  object's prototype when the target is a primitive or has a null prototype.
- The plugin merges SvelteKit's options with deepmerge once, in `mergeConfigs`, at `vite dev` and
  `vite build`. Its inputs are this repository's own configuration, and the built Mini App does
  not contain deepmerge; that reach does not make the advisory acceptable.
- Upstream, a pull request that drops the key, TehShrike/deepmerge#274, was closed unmerged. The
  open TehShrike/deepmerge#277 keeps the key as an own enumerable, writable and configurable data
  property through a `setProperty` helper used at the three assignments in `mergeObject`. No
  release carries either.

## Decision Drivers

- The web advisory audit is a gate at level low: an advisory is fixed, never ignored, excepted or
  graded down.
- The fix upstream is most likely to release is preferred over one written here, so the copy
  behaves as that release will.
- An override names its parent's major, as ADR-005's amendments require, so it never reaches a
  later major or another consumer.
- The copy must leave when upstream releases.

## Decision

- D1. `vendor/deepmerge` holds the registry's `deepmerge@4.3.1` files `package.json`,
  `license.txt`, `index.js`, `index.d.ts`, `dist/cjs.js` and `dist/umd.js`, with the change of
  TehShrike/deepmerge#277: line for line in `index.js`, and in the built files' own style in
  `dist/cjs.js`, the package's `main`, and `dist/umd.js`. Nothing else in them changes.
- D2. `pnpm-workspace.yaml`'s `overrides:` gains
  `'@sveltejs/vite-plugin-svelte@7>deepmerge': 'file:./vendor/deepmerge'`, and the lock resolves
  the plugin's deepmerge as `deepmerge@file:vendor/deepmerge`.
- D3. `vendor/deepmerge/PATCHED.md` names the release and the advisory, holds the diff, names the
  fix it mirrors and the red-first test, and states the rule for leaving.
- D4. When upstream releases a version outside the advisory's range, the override entry and the
  copy go together, the lock takes the release from the registry, and the red-first test stays.

## Considered Options (the alternatives it was chosen against)

- Our fixed copy mirroring TehShrike/deepmerge#277, through a scoped override: chosen, because it
  clears the advisory while keeping every key, and behaves as the fix upstream has open.
- Removing the dependency: rejected because SvelteKit 3 requires the plugin at 7, and every 7.x
  release requires `deepmerge ^4.3.1`.
- Replacing it with a maintained package: rejected because the plugin calls the CommonJS default
  export as `deepmerge(target, source, { arrayMerge })`, and `@fastify/deepmerge` and
  `deepmerge-ts` publish other call shapes, so any adapter would be code of our own as well.
- A copy that drops an own `__proto__` key: rejected because it loses the key's data and
  upstream closed that shape unmerged (TehShrike/deepmerge#274).
- A `pnpm patch` of the registry's 4.3.1: rejected because the lock would still name
  `deepmerge@4.3.1`, the name and version the advisory's range matches.
- A global `deepmerge` override: rejected because it would also force any other consumer, and a
  later plugin major, onto the copy.
- An audit ignore or exception: rejected because it leaves the advisory unfixed.

## Decision Outcome

Chosen option: our fixed copy through a scoped override, because no release exists to move to,
and the copy takes the change upstream has open without dropping data.

### Consequences

- Good, because the lock holds no affected deepmerge, and the plugin merges its options through a
  copy that keeps an own `__proto__` key as data.
- Bad, because we own a 19-line change in each of three files until a release, and a reader of the
  lock sees `file:vendor/deepmerge` where a version was; `PATCHED.md` says why.
- Bad, because the copy keeps the version string 4.3.1 in its `package.json`, so a scanner that
  read the installed package's own `package.json` instead of the lock would still name 4.3.1.

### Confirmation

`pnpm exec vitest run web/app/src/lib/deepmerge-prototype.test.ts` passes T1 to T6, of which T1,
T2 and T6 fail on the registry's 4.3.1; the lock holds no `deepmerge@4.3.1`; and CI's audit-web
stage reads the lock at level low.

## What would make this wrong

- `pnpm audit` reports the copy as 4.3.1 through the lock's `file:` entry. Then this decision
  does not clear the audit, and a new decision is due; an ignore, an exception or a lower level
  stays refused.
- Upstream releases a fixed version: then D4 applies and this decision is superseded.
- Upstream reworks TehShrike/deepmerge#277 into another shape before releasing it: then the copy
  follows that change.

## More Information

- Amends ADR-005's override rule. ADR-005's amendments move a development-only dependency to a
  release that fixes its advisory; this entry takes our copy instead, because no such release
  exists, and its selector still names its parent's major. ADR-005 gains a section naming this
  decision.
- SPEC-499 holds the measurements and the red-first test.

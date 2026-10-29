# SPEC-499: the Vite plugin's deepmerge is our fixed copy of 4.3.1, so an own `__proto__` key never sets a prototype

- **Issue:** #1033, which the maintainer's daily dependency scan filed for GHSA-gcqx-w48m-w2h4;
  this delivery refers to it and closes nothing. **Context(s):** the Mini App's build tooling
  (`pnpm-workspace.yaml`, `pnpm-lock.yaml`), a new `vendor/deepmerge`, and one test in `web/app`;
  no crate, and no file the built Mini App contains, changes.
- **Decided by:** ADR-517 (our fixed copy through one override scoped to the plugin's major, until
  a release outside the advisory's range), which amends ADR-005's override rule.
- **Schematic:** none. No component, data flow or state machine of the app changes: the Vite
  plugin calls deepmerge as before, and only the files pnpm installs for that call change.
- **Status:** delivered by the pull request that adds this file, with its test and
  `docs/red-first/SPEC-499.md`. It is stacked on #1036 (SvelteKit 3), whose lock it changes.
  **Mutation band:** none (section 8). **Model:** none (section 8).

## 1. The problem, measured

- **The advisory has no fixed release.** GHSA-gcqx-w48m-w2h4 (GitHub's grade low) affects
  `deepmerge` `>= 4.2.0, <= 4.3.1` and names no patched version; the registry's latest release is
  4.3.1. #1036's changelog fragment records it as the one web advisory its audit leaves: "no fixed
  release and stays" (`changelog.d/deps-sveltekit-3-20261028.md`).
- **One dependent holds it.** At #1036's head, `pnpm-lock.yaml` resolves `deepmerge@4.3.1` once,
  for `@sveltejs/vite-plugin-svelte` 7.3.1 (`git grep -n deepmerge 1ac1a7b1 -- pnpm-lock.yaml`:
  the package at line 1738, the plugin's snapshot at 3543 and its own snapshot at 4134).
- **No update removes it.** SvelteKit 3.1.0 peers the plugin at `^7.0.0`, and every release of the
  plugin's major 7, 7.0.0 to 7.3.1, declares `deepmerge ^4.3.1` (the registry's metadata).
- **Where it runs.** The plugin calls deepmerge once, in `mergeConfigs`
  (`src/utils/options.js:185`, the call at `:189` in 7.3.1), merging SvelteKit's options at
  `vite dev` and `vite build`. The built Mini App does not contain it.
- **The behaviour.** Reached as the plugin resolves it, at #1036's head: T1 and T2 read
  `expected 'yes' to be undefined` and T6 reads `expected undefined to be defined`; T3 to T5
  pass. T7 to T9, which followed the copy, fail there too, by assertion
  (`docs/red-first/SPEC-499.md`).

## 2. Requirements

- **R1.** `pnpm-lock.yaml` resolves no registry `deepmerge`: the plugin's `deepmerge` is
  `deepmerge@file:vendor/deepmerge`, a directory resolution, and no other package moves.
- **R2.** `vendor/deepmerge` holds the registry's `deepmerge@4.3.1` files `package.json`,
  `license.txt`, `index.js`, `index.d.ts`, `dist/cjs.js` and `dist/umd.js`, changed only by the
  change TehShrike/deepmerge#277 proposes: a `setProperty` helper used at the three assignments in
  `mergeObject`, 19 changed lines in each of the three JS files.
- **R3.** `vendor/deepmerge/PATCHED.md` names upstream `deepmerge@4.3.1` and GHSA-gcqx-w48m-w2h4,
  holds the diff, names TehShrike/deepmerge#277 as the fix it mirrors, names this SPEC's test file
  by its path, and states the rule for leaving: an upstream release outside the advisory's range.
- **R4.** `pnpm-workspace.yaml` gains one override,
  `'@sveltejs/vite-plugin-svelte@7>deepmerge': 'file:./vendor/deepmerge'`, whose selector names the
  plugin's major, and one comment sentence naming ADR-517.
- **R5.** No advisory is ignored or excepted, and the audit's level is not lowered:
  `AUDIT_WEB_LEVEL` in `scripts/check.sh` stays `low`.
- **R6.** The test reaches deepmerge through the resolution web/app's declared dependency (the
  Vite plugin) itself gets, never by declaring deepmerge in web/app.

## 3. Acceptance criteria

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | a nested own `__proto__` key merged over a primitive leaves the result's prototype `Object.prototype` and adds no inherited property (R1, R2, R6) | red at #1036's head: `expected 'yes' to be undefined` | `T1 a nested own __proto__ key merged over a primitive leaves the prototype alone` |
| A2 | a top-level own `__proto__` key merged over a null-prototype target does the same (R1, R2, R6) | red at #1036's head: `expected 'yes' to be undefined` | `T2 a top-level own __proto__ key merged over a null prototype leaves the prototype alone` |
| A3 | the plugin's own array option still replaces arrays while objects merge deeply (R2) | not red: a control any fix keeps | `T3 the plugin's array option replaces arrays and objects still merge deeply` |
| A4 | a deep merge leaves both of its inputs unchanged (R2) | not red: a control any fix keeps | `T4 a deep merge leaves both of its inputs unchanged` |
| A5 | `Object.prototype` gains no property from the merges (R2) | not red: a control any fix keeps | `T5 Object.prototype gains no property from the merges` |
| A6 | an own `__proto__` key stays an own data property of the result, cloned, with the result's prototype unchanged, the shape of TehShrike/deepmerge#277 (R2, R6) | red at #1036's head: `expected undefined to be defined` | `T6 an own __proto__ key stays an own data property, cloned, and the prototype unchanged` |
| A6b | an own `__proto__` key of a null-prototype target stays an own data property of the result, cloned, with the result's prototype unchanged: the null-prototype-target case of upstream pull request 277 (R2, R6) | red at #1036's head: `expected { admin: true } to be { …(12) } // Object.is equality` | `T7 an own __proto__ key of a null-prototype target stays an own data property, cloned` |
| A6c | the kept own `__proto__` key is enumerable, writable and configurable (R2, R6) | red at #1036's head: `expected undefined to deeply equal { value: { admin: true }, …(3) }` | `T8 the kept own __proto__ key is enumerable, writable and configurable` |
| A6d | an own `__proto__` key on both target and source merges into one own data property, with the default options and with an `isMergeableObject` option that declines the target, so that the deep-merge branch alone writes the key (R2, R6) | red at #1036's head: `expected { a: 1, b: 2 } to be { …(12) } // Object.is equality` | `T9 an own __proto__ key on both target and source merges into one own data property` |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/deepmerge-prototype.test.ts -t "T1 a nested own __proto__ key"
A2: pnpm exec vitest run web/app/src/lib/deepmerge-prototype.test.ts -t "T2 a top-level own __proto__ key"
A3: pnpm exec vitest run web/app/src/lib/deepmerge-prototype.test.ts -t "T3 the plugin's array option"
A4: pnpm exec vitest run web/app/src/lib/deepmerge-prototype.test.ts -t "T4 a deep merge leaves"
A5: pnpm exec vitest run web/app/src/lib/deepmerge-prototype.test.ts -t "T5 Object.prototype gains"
A6: pnpm exec vitest run web/app/src/lib/deepmerge-prototype.test.ts -t "T6 an own __proto__ key"
A6b: pnpm exec vitest run web/app/src/lib/deepmerge-prototype.test.ts -t "T7 an own __proto__ key of a null-prototype target"
A6c: pnpm exec vitest run web/app/src/lib/deepmerge-prototype.test.ts -t "T8 the kept own __proto__ key"
A6d: pnpm exec vitest run web/app/src/lib/deepmerge-prototype.test.ts -t "T9 an own __proto__ key on both"
```

All nine run in CI's `web` job, whose `pnpm -r test` runs web/app's suite.

## 4. File manifest

| path | part | change |
|---|---|---|
| `web/app/src/lib/deepmerge-prototype.test.ts` | Mini App tests | added: T1 to T9 |
| `vendor/deepmerge/package.json` | the copy | added: the release's bytes |
| `vendor/deepmerge/license.txt` | the copy | added: the release's bytes (MIT) |
| `vendor/deepmerge/index.d.ts` | the copy | added: the release's bytes |
| `vendor/deepmerge/index.js` | the copy | added: the release's file with the change of TehShrike/deepmerge#277 |
| `vendor/deepmerge/dist/cjs.js` | the copy | added: the same change, in the built file's style |
| `vendor/deepmerge/dist/umd.js` | the copy | added: the same change, in the built file's style |
| `vendor/deepmerge/PATCHED.md` | the copy | added: what it patches, the diff, the fix it mirrors, the test and the rule for leaving |
| `pnpm-workspace.yaml` | workspace | changed: one override entry and one comment sentence |
| `pnpm-lock.yaml` | workspace | changed: regenerated lockfile-only, +5 -4 lines |
| `REUSE.toml` | licensing | changed: an override annotating the copy's upstream files MIT under upstream's holders, then an annotation keeping `vendor/deepmerge/PATCHED.md` DeckStreak's own text |
| `LICENSES/MIT.txt` | licensing | added: the MIT text the copy's annotation names |
| `docs/specs/SPEC-499-the-vite-plugins-deepmerge-is-our-fixed-copy-of-4-3-1-so-an-own-proto-key-never-sets-a-prototype.md` | docs | added: this file |
| `docs/decisions/ADR-517-the-vite-plugins-deepmerge-is-our-own-fixed-copy-of-4-3-1-until-a-release-outside-the-advisorys-range.md` | docs | added |
| `docs/decisions/ADR-005-mini-app-front-end-sveltekit.md` | docs | changed: an amendment section appended, naming ADR-517 |
| `docs/red-first/SPEC-499.md` | docs | added: A1 to A6, then A6b to A6d |
| `changelog.d/deps-deepmerge-copy-20261028.md` | docs | added: the changelog fragment |

## 5. What this does NOT do

- It does not change #1036: the SvelteKit 3 move, its other lock moves and its fragment stay
  that pull request's. This delivery's fragment supersedes that fragment's deepmerge line (#1036).
- It sends nothing upstream. TehShrike/deepmerge#277 is open, and the copy mirrors it; the copy
  leaves at a release outside the advisory's range, which the scan behind #1033 reports.
- It does not judge in advance how `pnpm audit` reads a `file:` package; CI's `web` job, whose
  audit-web stage reads the lock at level low, is the first measurement (#1033).

## 6. Risks

- **audit-web cannot see the copy.** `pnpm audit` does not audit a `file:` package, so
  audit-web's zero for deepmerge is structural: at this pull request's head the audit reads
  `No known vulnerabilities found`, against 1 low at #1036's head (GHSA-gcqx-w48m-w2h4, through
  the plugin). After landing, the guards are the red-first test and the maintainer's dependency
  scan's rule for a repository's own patched copy; never an ignore, an exception or a lower
  level (ADR-517).
- **Upstream reworks its fix before a release.** The copy then differs from the release it waits
  for. The rule for leaving takes the release once it is outside the range, and T1 to T9 must
  still pass on it; until then the copy follows an upstream change to the same lines.
- **A later plugin major.** The selector names major 7, so a major 8 is not overridden and the
  audit judges whatever it resolves.
- **pnpm hard-links a `file:` directory on this host (inode-shared with the vendored file), not a
  copy.** After an install, `stat -c '%i %h'` reads one inode and 2 links for the plugin's
  installed `deepmerge/dist/cjs.js` and for `vendor/deepmerge/dist/cjs.js`, so an in-place edit
  to `vendor/deepmerge` reaches `node_modules` at once, with no install between; CI's frozen
  install reads the committed copy.

## 7. What only CI proves

A1 to A6 and A6b to A6d on GitHub's runner, the build that merges SvelteKit's options through the
copy, and audit-web's reading of the lock with the copy in it, all in the `web` job.

## 8. Formal model and mutation rows

None. The change adds no actor, shared state or write path. No production file under
`web/app/src` changes, and the copy is a dependency that StrykerJS's `mutate` globs do not reach
(`web/app/stryker.config.json`); T1 to T9 pin its behaviour.

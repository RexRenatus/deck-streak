# SPEC-425: the web sink census holds six more shapes at none, and the type check refuses a switch fall-through

- **Wave:** the app's hardening, outside the campaign rows. **Issue:** none; the pull request
  closes no issue. **Context(s):** `miniapp` (`web/app`), with one repository test under
  `scripts/tests`.
- **Decided by:** ADR-439 (this SPEC's own: the six arms, the walk, the flag, the kill criteria)
  and ADR-352 (the card frame is the one way card HTML reaches the page, which SPEC-341 R7's
  census holds). It amends SPEC-341 R7 insert-only, by SPEC-341's section 8.
- **Schematic:** `docs/schematics/web-sink-census.md` (this delivery adds it).
- **Status:** this delivery builds it, with its tests and `docs/red-first/SPEC-425.md`.
  **Mutation band:** S42500-S42599 (rows S42501 to S42504).

## 1. The problem, measured

Every figure is read at dev `ac7ebd75`.

- **The census.** `web/app/src/lib/card/card-sinks.test.ts` (SPEC-341 R7, A7) holds seven arms
  (`:16-27`) by a pattern each over a file's raw text, so a comment counts (`:10`). Its walk reads
  every `.ts`, `.js` and `.svelte` file under `web/app/src`, skipping the generated messages
  directory and every `.test.` or `.spec.` file (`:46-56`). It prints its examined count and
  refuses zero (`:39-43`), and proves each arm against a planted spelling (`:75-87`). By
  `git ls-tree -r --name-only ac7ebd75 web/app/src`, filtered as the walk filters, it reads 111
  tracked files (48 `.svelte`, 60 `.ts`, 3 `.js`). The one shipped file it skips is the page
  shell, `src/app.html`.
- **The six shapes.** By `git grep -P` with each arm's pattern at `ac7ebd75`, filtered to the
  walk's files and `src/app.html`: `eval(` 0, the `Function` constructor 0, a string timer 0, a
  blank target without `rel` 0, a `postMessage` to `'*'` 0, a `javascript:` literal 0. Outside
  the walk the shapes appear on purpose: the planted card harness holds a blank target
  (`web/app/tests-card/planted.ts:171`), two posts to `'*'` (`:77-78`, mirrored in
  `web/app/tests-card/harness/planted/bridge.js:7-8`) and a `javascript:` link (`:185`), and a
  test names the scheme to prove its refusal (`web/app/src/lib/startapp.test.ts:53`).
- **The near misses.** A policy string holds `'wasm-unsafe-eval'`
  (`web/app/src/lib/telegram-launch.ts:29`); three timers take a function
  (`web/app/src/lib/study/ReviewScreen.svelte:150`, `web/app/src/lib/sync/BackupList.svelte:34`,
  `web/app/src/lib/telegram-launch.ts:65`); and three `postMessage` calls post to a port, a
  broadcast channel and the engine Worker's scope with no wildcard
  (`web/app/src/lib/engine/client.ts:72`, `web/app/src/lib/engine/credential.ts:268`,
  `web/app/src/lib/engine/worker.ts:80`).
- **The type check.** The app's `check` script runs `svelte-check --tsconfig ./tsconfig.json
  --fail-on-warnings` (`web/app/package.json:12`); `scripts/check.sh`'s `stage_web` runs it as
  `pnpm -r check` (`:157-164`); and CI's web job runs that stage. `web/app/tsconfig.json`
  (`:1-20`) extends the generated `.svelte-kit/tsconfig.json` and sets `strict`, which does not
  include `noFallthroughCasesInSwitch`, and that flag is off by default. The type-checked tree
  holds five `switch` statements (`web/app/src/lib/engine/session.ts:300` and `:372`,
  `web/app/src/lib/law/LawBlock.svelte:18`, `web/app/src/lib/sync/SyncScreen.svelte:90`,
  `web/app/src/lib/sync/status.ts:33`), found by `git grep -n -E '\bswitch[[:space:]]*\('` over the
  tree, and every non-empty case ends in a `return`: 0 fall-throughs.

## 2. Requirements

R1. The census holds six arms of its own, beside SPEC-341's seven, each by the name below and each
    at none over the walk of R4, with no expected site and no allow-list:

| arm | reads |
|---|---|
| `eval(` | `eval` then `(`, where the character before `eval` is not a letter, digit, `_` or `$` |
| `new Function(` | `Function` then `(`, with or without `new`, under the same character rule |
| `string timer` | `setTimeout` or `setInterval` whose first argument opens with a quote or a backtick |
| `blank target without rel` | a `target` of `_blank` (any quote, braces or case, or the DOM property) whose own tag holds no `rel` naming both `noopener` and `noreferrer` |
| `postMessage to '*'` | a `postMessage` call whose arguments hold `'*'` after a comma, or a `targetOrigin` of `'*'` |
| `javascript: URL` | a quote, a backtick or `=`, optional space, then `javascript:` in any case |

R2. Each arm reads each of its plants exactly once under its own name, and every near miss reads
    none; the plant set equals the arm set by name. A matcher that cannot read its plant fails,
    and so does one that reads a near miss.

| arm | plants (each reads that arm once) | near misses (each reads none) |
|---|---|---|
| `eval(` | `const run = eval(card);`, `window.eval(card);`, `// a comment naming eval(card) counts too` | `"script-src 'self' 'wasm-unsafe-eval'"`, `page.evaluate(fn);`, `page.$eval(sel, fn);` |
| `new Function(` | `const run = new Function('return ' + card);`, `Function('return 1')();` | `isFunction(value);`, `typeof x === 'function';` |
| `string timer` | `setTimeout('render()', 0);`, ``setInterval(`tick()`, 10);`` | `setTimeout(() => input.blur(), 0);`, `window.setTimeout(next, 0);` |
| `blank target without rel` | `<a href={url} target="_blank">open</a>`, `<a target='_blank' rel="noopener" href={url}>`, `<a href={url} target={'_blank'}>`, `link.target = '_blank';`, `<a rel="noopener noreferrer" onclick={() => go()} target="_blank">` | `<a href={url} target="_blank" rel="noopener noreferrer">`, `<a rel="noreferrer noopener" href={url} target="_blank">`, `<a onclick={() => go()} target="_blank" rel="noopener noreferrer">` |
| `postMessage to '*'` | `parent.postMessage('card', '*');`, `frame.contentWindow.postMessage(card, { targetOrigin: '*' });` | `scope.postMessage(reply, transferred(reply));`, `channel.postMessage(null);` |
| `javascript: URL` | `<a href="javascript:void 0">run</a>`, `location.href = 'JavaScript:void 0';`, `<a href=javascript:void(0)>`, `if (url.protocol === 'javascript:') refuse();` | `// the page's own JavaScript runs from its own files` |

R3. Where text is ambiguous the arms read a hit, never a miss. A comment counts, as it does for
    SPEC-341's seven. A blank target's tag is the text between the nearest `<` or `>` before the
    target and the nearest after it, so a tag the census misreads hides its `rel` and reads as a
    hit (R2's fifth `blank target` plant). A guard that names the `javascript:` scheme reads as a
    hit (R2's fourth `javascript: URL` plant).
R4. One walk serves all thirteen arms: every `.ts`, `.js`, `.svelte` and `.html` file under
    `web/app/src`, never a `.test.` or `.spec.` file and never the generated messages directory.
    Its kinds are exactly those four extensions; it holds `src/app.html`,
    `src/lib/card/CardFrame.svelte`, `src/lib/card/frame-host.ts` and `src/lib/card/policy.js`;
    it holds no test file and no message file; and it prints its examined count and refuses zero.
    SPEC-341's A7 reads the same walk with its expected census unchanged.
R5. `web/app/tsconfig.json` sets `"noFallthroughCasesInSwitch": true` in its own
    `compilerOptions`, on the line after `"strict": true`. The app's `check` script runs the type
    check on that file (`svelte-check --tsconfig ./tsconfig.json`), and `scripts/check.sh`'s
    `stage_web` runs the `check` script (`pnpm -r check`).
R6. Each arm and the flag is held by ADR-439 D5's kill criteria: a demotion is proposed on the
    measured precision of the last 30 merged pull requests, and it lands only as the owner's
    signed ruling.

## 3. Acceptance criteria of SPEC-425

| id | criterion | decided by |
|---|---|---|
| A1 | the census holds the six refused shapes at none over the walk, reads each plant once under its arm, reads no near miss, and its plant set equals its arm set | `web/app/src/lib/card/card-sinks.test.ts` "shipped code holds none of the six refused shapes" |
| A2 | the walk reads exactly four kinds, holds one anchor per kind and no test or message file, and refuses zero | `web/app/src/lib/card/card-sinks.test.ts` "the census reads every shipped kind, the page shell included" |
| A3 | the app's own config sets the fall-through flag, the type check reads that config, and the web stage runs the type check | `scripts/tests/test_web_type_check.py` `test_the_type_check_reads_no_fallthrough_from_the_apps_own_config` |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/card/card-sinks.test.ts -t "shipped code holds none of the six refused shapes"
A2: pnpm exec vitest run web/app/src/lib/card/card-sinks.test.ts -t "the census reads every shipped kind, the page shell included"
A3: python3 -m unittest discover -s scripts/tests -p test_web_type_check.py -k test_the_type_check_reads_no_fallthrough_from_the_apps_own_config
```

A1 and A2 sit in one `describe` block beside SPEC-341's, and each assertion comes before its
`examined()` call. SPEC-341's A7, "card HTML reaches the page only through the card frame", keeps
its name, its body and its fence line; only its walk gains `src/app.html`. A1 is red with an empty
arms table (its first plant reads `{}`), A2 is red at the old walk (its kinds lack `.html`), and
A3 is red with the flag absent.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-425-the-web-sink-census-holds-six-more-shapes-at-none-and-the-type-check-refuses-a-switch-fall-through.md` | docs | added |
| `docs/decisions/ADR-439-the-six-shapes-join-the-card-sink-census-as-text-arms-over-one-widened-population-and-the-apps-own-config-refuses-a-switch-fall-through.md` | docs | added |
| `docs/schematics/web-sink-census.md` | docs | added |
| `docs/specs/SPEC-341-a-card-face-renders-on-the-web-in-a-sandboxed-frame-that-reaches-neither-the-app-nor-the-network.md` | docs | changed: section 8 appended, insert-only |
| `docs/red-first/SPEC-425.md` | docs | added |
| `web/app/src/lib/card/card-sinks.test.ts` | `miniapp` | changed (A1, A2): the six arms and their plants, the shared matcher, the walk reads `.html` |
| `web/app/tsconfig.json` | `miniapp` | changed (A3): `noFallthroughCasesInSwitch` |
| `scripts/tests/test_web_type_check.py` | scripts | added (A3); it loads no code through `importlib`, `runpy` or `exec` |
| `scripts/mutation-rows.d/S42500-S42599.json` | scripts | added: rows S42501 to S42504 |
| `changelog.d/web-sink-census-425.md` | docs | added |

## 5. What this does NOT cover

- The URL-attribute arm, an `href` or `src` read from a variable: its two hits are constants
  (`web/app/src/routes/about/+page.svelte:26-27`) that a rule for admitting a site would have to
  triage first (#851).
- The iframe-without-`sandbox` arm: its one string is the host document's inner frame
  (`web/app/src/lib/card/frame-host.ts:45`), whose outer frame `CardFrame.svelte:19` sandboxes,
  and it needs its own triage (#852).

## 6. Risks

- **An arm reads too little.** Each arm's false negatives are named in ADR-439 D2: an alias or
  indirect `eval`, a constructor reached by reflection, a timer string held in a variable, a
  `window.open` with no `target`, an origin held in a constant, a scheme split across strings.
  Detected by a plant that a new spelling adds to R2; the arm then widens with it.
- **An arm reads too much.** The fail-closed reads of R3 are false positives by design, and a
  comment that names a shape counts. Detected by ADR-439 D5's precision over the last 30 merged
  pull requests, which proposes a demotion to the owner.
- **The walk shrinks.** A walk that stops reading one kind would still examine more than zero.
  Detected by A2's kinds and anchors.
- **A sibling adds a shape or a fall-through.** The census and the type check read the whole
  tree, so a change merged first is read by the second. Detected at land: the land re-runs both
  on the merged tree.
- **The type check stops reading the app's config,** or a framework writes options after it.
  Detected by A3's chain; the flag's effect is also measured once at build by a planted
  fall-through.
- **The config gains a comment or a trailing comma** the structural test cannot parse. Detected:
  A3 fails loudly on a parse error rather than reading the flag as set.

## 7. Mutation rows

The band file `scripts/mutation-rows.d/S42500-S42599.json` holds four `SCRIPT_MUTATIONS` rows, each
killed by A3's test,
`test_web_type_check.TheTypeCheckRefusesAFallThrough.test_the_type_check_reads_no_fallthrough_from_the_apps_own_config`:

| row | target | mutant |
|---|---|---|
| S42501-THE-FLAG-IS-ON | `web/app/tsconfig.json` | the flag's value `true` becomes `false` |
| S42502-THE-FLAG-IS-SET | `web/app/tsconfig.json` | the flag's line is removed, so the compiler's default, off, returns |
| S42503-THE-CHECK-READS-THE-APPS-OWN-CONFIG | `web/app/package.json` | `--tsconfig ./tsconfig.json` becomes `--tsconfig ./.svelte-kit/tsconfig.json`, the generated parent that lacks the flag |
| S42504-THE-WEB-STAGE-RUNS-THE-TYPE-CHECK | `scripts/check.sh` | `stage_web`'s `pnpm -r check &&` line is removed |

The six arms are proved by their plants (R2), not by rows: StrykerJS mutates no test file
(`web/app/stryker.config.json:11-12`), and the rows runner kills only with a unittest or a cargo
killer. No production web file changes, so the mutation verdict's web class reads not-applicable by
name; every row above is proved by the rows job.

## 8. Formal: not applicable by surface

No actor, shared state or write path is added. The census and A3's test each read the tree and
write nothing; the type check reads its config; no check is followed by an act another actor can
change. The rows runner's install and restore of the four rows' targets is an existing actor whose
protocol this delivery does not change. The arms are a pure function of a file's text in a `.ts`
test file, which no `@phx covers` path can name, and their proof is their plants. None of the 251
`@phx covers` lines under `formal/` at `ac7ebd75` names a path this delivery edits.

## 9. Kill criteria

ADR-439 D5 holds them: live precision over the last 30 merged pull requests, below 0.8 with at
least 5 hits; 2 or more false hits among a new arm's first 5; a demotion is the owner's signed
ruling; and an `examined 0`, or a tool missing or off its pin, is VOID, which blocks.

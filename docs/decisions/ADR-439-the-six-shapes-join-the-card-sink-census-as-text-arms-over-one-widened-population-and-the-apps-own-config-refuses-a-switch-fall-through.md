---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The six shapes join the card sink census as text arms over one widened population, and the app's own config refuses a switch fall-through

## Context and Problem Statement

SPEC-341 R7 holds the Mini App's raw-HTML sinks and window message listeners at none with one
census, `web/app/src/lib/card/card-sinks.test.ts`. It reads the text of every shipped `.ts`, `.js`
and `.svelte` file under `web/app/src`, holds seven arms (`:16-27`), prints and refuses a zero
examined count (`:39-43`), and proves each arm against a planted spelling (`:75-87`). Six more
shapes have no holder: code run from text (`eval(`, the `Function` constructor, a string handed to
`setTimeout` or `setInterval`), a link that opens a blank target without `rel="noopener
noreferrer"`, a `postMessage` to the origin `'*'`, and a `javascript:` URL literal. None of them
appears in the shipped code at `ac7ebd75`, so a rule holding each at none costs no cleanup today
and refuses the first one a change adds.

The type check, `svelte-check --tsconfig ./tsconfig.json --fail-on-warnings`
(`web/app/package.json:12`), lets a non-empty `switch` case fall through to the next one, since
`noFallthroughCasesInSwitch` is off by default and is not part of `strict`. The type-checked tree
holds five `switch` statements and no fall-through at `ac7ebd75`.

SPEC-425 is the delivery. This ADR records its eight decisions.

## Decision Drivers

- A refused shape is refused where the existing census already looks, by the method it already
  uses, so one reader holds every arm and one examined count covers them.
- A census that examines less than it should must fail, never read green.
- Every rule is held at none: no arm admits a listed site.
- A flag nobody tests can be dropped by any later edit while every check stays green.
- A rule that misfires must have a way out that a reader can measure, and the way out is a
  weakening the owner signs.

## Decisions, and the alternatives each was chosen against

### D1. The six shapes extend the card sink census; there is no second census

The six arms live in `card-sinks.test.ts`, beside SPEC-341's seven, in a second arms table with a
test of its own, and reuse the census's one matcher (`sinksIn`), its one walk (`sources`) and its
one examined helper (`examined`). SPEC-341's A7 keeps its test, its name, its seven arms and its
expected census. SPEC-341 R7 is amended insert-only by its section 8, which names the new arms and
the widened walk (D3); SPEC-425 cites R7 as the census it extends.

Chosen against:

- A Python census over the same files: rejected because it would read the same files twice under
  two matchers, so the two could disagree about one file and nobody compares them.
- The six arms folded into SPEC-341's own `SINKS` table and A7's test: rejected because A7 states
  that card HTML reaches the page only through the card frame, and none of the six is a card HTML
  sink, so A7 would hold a second claim under a name that does not say it.

### D2. Six named arms, each read from raw text, each held at none

Each arm is matched over a file's raw text, the census's existing method, so a comment counts
(`card-sinks.test.ts:10`). The arms and how each matches:

`eval(` reads `eval` followed by an opening parenthesis, where the character before `eval` is not
a letter, digit, `_` or `$`; `window.eval(` counts. It reads no `'unsafe-eval'` or
`'wasm-unsafe-eval'` policy keyword (`web/app/src/lib/telegram-launch.ts:29` holds one), no
`evaluate(` and no `$eval(`. A false positive is another object's method named `eval`; a false
negative is an alias (`const run = eval`), an indirect call (`(0, eval)(text)`) or a bracket read
(`globalThis['eval']`).

`new Function(` reads the `Function` constructor called with or without `new`, under the same
character rule, so `isFunction(` and the lower-case keyword never count. A false negative is a
constructor reached by reflection, such as an async function's `constructor`.

`string timer` reads `setTimeout` or `setInterval` whose first argument opens with a quote or a
backtick. The three timers in the shipped code each take a function (`ReviewScreen.svelte:150`,
`BackupList.svelte:34`, `telegram-launch.ts:65`) and read none. A false negative is a string held
in a variable or built by concatenation that does not open with a literal; the type checker does
not refuse one, since a timer's handler type admits a string.

`blank target without rel` reads each `target` set to `_blank` (any quote, Svelte braces, any
case, or the DOM property) and counts it unless its own tag carries a `rel` whose quoted value
names both `noopener` and `noreferrer`, in either order. The tag is the text between the nearest
`<` or `>` before the target and the nearest after it, so a misread tag hides the `rel` and reads
as a hit, never as a miss. Its known false positive is that fail-closed read: a `rel` placed before
an attribute whose expression holds `=>`, or a `>` inside an earlier quoted value. Its false
negatives are a `window.open` call, which takes no `target` attribute, and a `rel`-shaped string
inside another attribute's value of the same tag.

`postMessage to '*'` reads a `postMessage` call whose arguments, up to its closing parenthesis and
through two levels of nested parentheses, hold `'*'` (any quote) right after a comma, or an options
object whose `targetOrigin` is `'*'`. The six calls in the shipped code read none (a port, a
broadcast channel and the engine Worker's scope, `engine/client.ts:72`, `engine/credential.ts:268`,
`engine/worker.ts:80`). A false negative is an origin held in a constant, deeper nesting before the
origin argument, or a call reached by bracket access, `call` or `apply`.

`javascript: URL` reads a quote, a backtick or an `=` followed by optional space and `javascript:`
in any case. Its known false positive is a guard that names the scheme to refuse it (a test does
so at `web/app/src/lib/startapp.test.ts:53`, outside the population); it reads as a hit by design,
because a guard should allow its safe schemes rather than name the one it refuses. A false
negative is a scheme split across strings, entity-encoded, or broken by a tab or newline the URL
parser strips.

Every arm is proved by its plants: each planted spelling reads exactly its arm once, each near
miss reads none, and the plant set equals the arm set by name (SPEC-425 R2). Every count is 0 at
`ac7ebd75`; an arm that reads above 0 at the build's cut is a question for the architect, never a
cleanup folded into this delivery.

Chosen against:

- A parse of each file (the TypeScript and Svelte compilers' syntax trees): rejected because it
  needs two parsers and their version-bound APIs, and it would stop counting a comment, unlike
  the census's seven arms, so one census would hold two rules about what counts.
- A token-only arm for the blank target (any `_blank` at all): rejected because it would refuse a
  correct link with its `rel`, the one form the rule asks for.
- An arm per shape that admits listed sites, as `srcdoc` admits its two: rejected because an
  admitted site turns a refusal into a review of each listed site, and every one of the six has
  none to admit.

### D3. One walk for every arm, widened to the page shell, held by its kinds and anchors

Both tests read the census's one walk. It reads every `.ts`, `.js`, `.svelte` and `.html` file
under `web/app/src`, never a `.test.` or `.spec.` file and never the generated messages
directory: 112 tracked files at `ac7ebd75`, where it read 111 before `src/app.html` joined.
SPEC-341's A7 reads the same walk, and its expected census is unchanged, since `src/app.html`
holds none of its seven sinks. SPEC-425's A2 holds the walk: its kinds equal exactly those four
extensions; it holds four anchors, one per kind (`src/app.html`, `src/lib/card/CardFrame.svelte`,
`src/lib/card/frame-host.ts`, `src/lib/card/policy.js`); it holds no test file and no message
file; and it prints its examined count and refuses zero.

Chosen against:

- A walk that silently shrinks, held only by `examined > 0`: rejected because a walk that stopped
  reading one kind of file would still examine more than zero and read green over less.
- An exact examined count: rejected because every shipped file a change adds or removes would
  fail the census for a reason no sink gives, and the cure, editing the number, trains a reader
  to edit it.
- Leaving `src/app.html` unread: rejected because it is the one shipped page shell, and an unread
  shipped file is where a blank target or a script URL would sit unseen.
- A wider walk (the tests, the card harness, the build configs): rejected because the planted card
  harness holds a blank target, two posts to `'*'` and a script URL on purpose
  (`web/app/tests-card/planted.ts:77-78`, `:171`, `:185`), so the census would refuse the proofs it
  depends on, and a build config never reaches the page.
- A second walk for the six arms: rejected because two walks can drift apart and print two
  examined counts that nobody compares.

### D4. The flag sits in the app's own config, held by a test of the chain the type check reads

`web/app/tsconfig.json` sets `"noFallthroughCasesInSwitch": true` in its own `compilerOptions`,
on the line after `"strict": true`. That file is the one the type check names
(`svelte-check --tsconfig ./tsconfig.json`), and its options override the generated
`.svelte-kit/tsconfig.json` it extends, whose only in-tree source, `web/app/svelte.config.js`'s
`typescript.config` hook, adds include paths and no option. CI's web job runs
`scripts/check.sh`'s web stage, whose `stage_web` runs `pnpm -r check`, which runs that type
check; the job's stage list is already held by `scripts/tests/test_ci_workflows.py`'s stage
layout. With the flag on, the compiler reports error 7029 ("Fallthrough case in switch") for a
non-empty case that reaches the next, and still admits empty cases grouped over one body. At
`ac7ebd75` the type-checked tree holds five `switch` statements and every non-empty case ends in
a `return`: 0 fall-throughs.

The red-first shape is a structural test, `scripts/tests/test_web_type_check.py`, that reads the
three links of that chain from the tree: the flag's value in the app's config, the type check's
`--tsconfig` argument in the app's `check` script, and the type check in `stage_web`. Each link
has a mutation row (D7).

Chosen against:

- The flag with no test: rejected because today's tree holds no fall-through to refuse, so a later
  edit that drops the flag would leave every check green.
- A fixture the type check refuses, compiled by the compiler's API in Vitest: rejected because
  it re-tests the compiler's own semantics, pays a full library load on every run, and
  the rows runner kills only with a unittest or cargo killer, so its rows could not be proved; the
  build measures the flag's effect once instead, by a planted fall-through it restores.
- The flag set through the generated parent config by `svelte.config.js`'s hook: rejected because
  the generated file is not in the tree, so the test would read what a build step writes, and the
  app's own config overrides it anyway.
- A linter rule for fall-through: rejected because the app runs no linter beyond the type check,
  and adding a tool is a different delivery.

### D5. Kill criteria

Each of the six arms and the flag is held by these criteria. A hit is a pull request in which the
census reads an arm above none or the type check reports a fall-through; a fix removes the shape;
an admission is any edit that makes the census expect a site or narrows an arm's pattern, and any
edit that turns the flag off or sets it aside for a file.

Live precision is fixes divided by fixes plus admissions over the last 30 merged pull requests.
Below 0.8 with at least 5 hits, a demotion is proposed. For a new arm that has read zero until now,
if 2 or more of its first 5 hits are false, a demotion is proposed. A demotion is a weakening, and
it goes to the owner as a signed ruling: no arm is narrowed, and the flag is not removed, without
one. Tooling: an `examined 0`, or a tool that is missing or off its pin, is VOID, which blocks; it
is never a pass. A kill window reads "the last N merged pull requests", never a number of days.

Chosen against:

- No kill criteria: rejected because an arm that misfires would then be narrowed by whoever it
  blocks, which is a weakening no one signed.
- A window of days: rejected because the count of merged pull requests in a span of days varies,
  so one threshold would read different populations.

### D6. Two neighbouring arms stay out

The URL-attribute arm (an `href` or `src` read from a variable) has two hits, both constants
(`web/app/src/routes/about/+page.svelte:26-27`), and the iframe-without-`sandbox` arm has one string
to triage, the host document's inner frame (`web/app/src/lib/card/frame-host.ts:45`), whose outer
frame `CardFrame.svelte:19` sandboxes. Neither is in this delivery; each is tracked by the issue
SPEC-425 section 5 cites.

Chosen against:

- Folding both in: rejected because each has hits today, so each needs a triage of its hits and a
  rule for admitting a site, which D2 declines for this census.

### D7. Tests red first, four rows, and one push

A1 (the six arms, their plants and their near misses) is red with an empty arms table: the first
plant reads `{}`. A2 (the walk) is red at the old walk: its kinds lack `.html`. A3 (the chain) is
red with the flag absent. Each red is seen on the builder's own tree, so the delivery needs ONE
push. The rows are S42501 to S42504 in `scripts/mutation-rows.d/S42500-S42599.json`, each a
`SCRIPT_MUTATIONS` row killed by A3's test: the flag set to `false`, the flag's line removed, the
type check pointed at the generated parent config, and the type check's line removed from
`stage_web`. The six arms are proved by their plants, not by rows: StrykerJS mutates no test file
(`web/app/stryker.config.json:11-12`), and the rows runner kills only with a unittest or cargo
killer. No production web file changes, so the mutation verdict's web class reads not-applicable
by name. Formal: not applicable by surface (SPEC-425 section 8).

Chosen against:

- Rows over the census's patterns: rejected because no killer the rows runner can run observes a
  Vitest assertion, so each such row would be VOID; the plants are the arms' kill proof.
- Two pushes, the first carrying the red alone for CI to read: rejected because every red here
  runs on the builder's tree, and a second push spends a CI run proving what the red-first record
  already holds.

### D8. One delivery, and SPEC-341 amended insert-only

SPEC-425 and this ADR are the whole record. SPEC-341 gains an appended section 8 and loses no
byte. ADR-352 is not amended: none of its eight decisions changes, since no card channel is added
or closed and the census is SPEC-341 R7's test rather than one of ADR-352's layers.

Chosen against:

- An amendment of ADR-352 too: rejected because it would restate this ADR in a record whose
  decisions it does not touch.
- Two deliveries, the census and the flag apart: rejected because both are zero-hit rules over the
  Mini App with one red-first shape each, and two deliveries would pay the record twice for one
  change of rules.

## Decision Outcome

The card sink census holds thirteen arms over one walk of 112 shipped files, six of them new and
held at none with no admitted site, and the app's type check refuses a switch case that falls
through, held by a structural test of the chain that reads the flag.

### Consequences

- A change that adds one of the six shapes to shipped code fails the census by the arm's name and
  the file that holds it.
- A change that adds a fall-through fails the type check with error 7029.
- The census examines one more file, `src/app.html`, under SPEC-341's A7 too.
- A guard that names the `javascript:` scheme reads as a hit, so a scheme guard allows its safe
  schemes instead.

### Confirmation

SPEC-425's A1 to A3 run in CI's web job and its unittest stage on every pull request. The build
measures the flag once: it plants a fall-through in a shipped `switch`, reads the type check's
error 7029, and restores the file from git.

## What would make this wrong

- A planted spelling of a shape the browser runs that no arm reads: the arm's pattern is too
  narrow and gets a plant and a wider pattern.
- An arm's precision falls below D5's bar: its pattern is wrong for this tree, and a demotion is
  proposed to the owner.
- The type check stops reading the app's own config (a changed `--tsconfig`, or a framework that
  writes options after it): A3 fails, and the flag moves to wherever the type check reads.

## More Information

- SPEC-341 R7 and A7 (the census this extends); ADR-352 (the card frame is the one way card HTML
  reaches the page).
- The schematic `docs/schematics/web-sink-census.md` draws the walk, the arms and the type check's
  chain before and after.

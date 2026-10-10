---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The frame policy is in force from the card document's creation, through a host document it inherits from

Filed as `docs/decisions/ADR-421-the-frame-policy-is-in-force-from-the-card-documents-creation-through-a-host-document-it-inherits-from.md`.
It decides SPEC-407 (#787). Citations are read at DeckStreak `dev` `a68db18aed95338ebdebbec0374ba320be9f4471`.

## Context and Problem Statement

ADR-352 gives the web card frame four layers: W1 the sandbox with no token, W2 the frame policy (a
`Content-Security-Policy` meta element first in the card document's head), W3 the strip, and W4 the
page's `frame-src 'none'` (ADR-352 D7). ADR-416 made the strip remove every `srcset`, planted the two
image-set forms (`img srcset` and `picture source`) as their own `srcset` card, and gave that card no
layer alone: "W3 holds the image-set forms beside W2" (ADR-416 D4). In Firefox that table rested on
one reading: with W3 off, "the form stayed closed in #766's first reading with W1, W2 and W4 on"
(`docs/schematics/card-frame-channels.md:183-184`).

#787 reports the Firefox case `W3 off: srcset stays closed` (`web/app/tests-card/card.spec.ts:133`)
reading `/srcset/1` and `/srcset/2` on `dev` and on pull request runs. Its terms: until the cause is
decided, a red from this case is never admitted green, no assertion of the planted suite is narrowed
in any engine, and the case is not retried into green.

The measurement (SPEC-407 section 1): eight failing `card-sandbox` jobs and one passing job, with
the same web tree (`git rev-parse 32f62172:web` equals `a68db18a:web`), the same workflow file and
the same browser builds. Each failing job fails only this one Firefox case, by
`srcset opened with W3 off in firefox` at `card.spec.ts:137`; six read both paths once each, and two
read `/srcset/1` alone (jobs 114225529870 and 114227082352).

ADR-412 D6 already holds the engine reading this needs: Firefox's HTML parser fetches images ahead
of its tree builder, a policy meta element it meets ahead is kept only as a preload policy, and an
image-set candidate fetched ahead is typed as an image set, not a preload type, so only the
document's own policy is consulted, and the meta element is not part of it until the tree builder
applies it. The schematic's own lines say W1 and W4 stop no fetch (`card-frame-channels.md:130-135`),
and neither the page policy P (`web/app/svelte.config.js:24-31`) nor the edge's header
(`deploy/caddy/deck-streak.caddy:19`) names an image source. So in Firefox, with W3 off, no layer
holds the image-set forms by what it does: whether they arrive is the engine's fetch order.

## Decision Drivers

- ADR-416's own driver: a layer holds a form in every engine by what it does, never by an engine's fetch order (ADR-416 line 25).
- #787's terms: no retry, no admitted red, no narrowed assertion in any engine.
- The layer table stays as it is: `srcset` held by W3 and W2 with no layer alone, and every other channel's "layer alone" unchanged.
- The page policy, the edge's header and W1's one attribute stay as they are.
- A red-first record that reads red deterministically, in every engine, before the fix.

## Decisions, and the alternatives each was chosen against

### D1. The cause is a containment finding of the fetch-order kind, not a counting race

In Firefox, with the strip off, the meta element alone does not hold an image-set candidate the
parser fetches ahead of its tree builder, and nothing else in the frame names an image source, so
the two forms arrive whenever the fetch comes first. The evidence:

| fact | where it is read |
|---|---|
| the paths `/srcset/1` and `/srcset/2` are named only by the `srcset` card | `web/app/tests-card/planted.ts:101-111` |
| every other `srcset` visit has W3 on (no candidate left) except the reference visit | `planted.ts:204`, `web/app/tests-card/harness/main.ts:62-73` |
| every visit opens a fresh browser context and resets every count first | `card.spec.ts:58-74`, `web/app/tests-card/listeners.ts` `reset()` |
| in job 114224746266 the reference visit ran minutes earlier, with 87 tests between, each resetting the counts and each reading nothing or its own card's paths | the job's log, tests 301 to 387 |
| the arrivals are exactly the two image-set forms, never another path | the eight artifacts' `card-W3-off-srcset-stays-closed-firefox/error-context.md` |
| pass and fail share the web tree, the workflow file and the browser builds; the arrival set varies (both paths in six jobs, `/srcset/1` alone in two) | SPEC-407 section 1 |
| the case's duration is the same in pass and fail | the nine logs |

Chosen against:
- A counting race (an arrival from another visit landing in this one): rejected because no other visit in the run can name a `/srcset/` path with a candidate left, every visit resets the counts and opens a fresh context, and the reference visit ran 87 resets earlier.
- A late load of this visit's own forms after the reading: rejected because a late load would add an arrival to a reading, never move a form that the layer under test holds; the arrivals are counted only because no layer refused them.
- A change in the tree or the runner between the passing and the failing jobs: rejected because the web tree and the workflow are byte-identical across them and every job ran the same browser builds; only the arrival set varies, which is the engine's fetch order.

### D2. W2 holds the image-set forms from the card document's creation: the card document inherits the host policy from a host document

`CardFrame.svelte`'s frame no longer holds the card document directly. Its `srcdoc` is a host
document, composed by `frameHost` in the new `web/app/src/lib/card/frame-host.ts`: its head is a
meta element carrying the host policy and a style that makes its one frame fill it, and its body is
exactly one `iframe` with a `title` and, as its `srcdoc`, the frame document `frameDocument` writes,
unchanged. A `srcdoc` document's policy container is a copy of its parent's at creation, so the host
policy is in force in the card document before its first byte is parsed, in every engine, and an
image-set candidate fetched ahead is refused by the document's own policy. The card document keeps
its own meta element (the frame policy, unchanged), so W2 is delivered at two points: inherited from
the host, and first in the card document's head.

`HOST_POLICY`, appended to `web/app/src/lib/card/policy.js` after line 26, is
`img-src data:; script-src 'none'; object-src 'none'; base-uri 'none'`: each directive is one the
frame policy already enforces (its `img-src data:`, its `default-src 'none'` for scripts and objects,
its `base-uri 'none'`), so the card document admits nothing it did not admit before. It names no
`default-src`, `frame-src` or `child-src`, so the card frame's own navigation is still checked
against the page's `frame-src` (W4), which the host inherits from the page, and `nav-self` and
`download` keep W4 as their layer alone.

The host's frame carries no `sandbox` attribute of its own: a nested frame inherits its parent's
sandboxing flags, so W1 stays the one attribute `CardFrame.svelte` sets. The host is parsed again
as the frame will parse it, and refused as `escaped` unless its head reads back as written and its
one frame gives back exactly the card document and the title; `CardFrame` then renders a refused
frame as it does today.

The harness builds every single-layer variant through the host: W1 off is the host with no sandbox,
W2 off removes both of W2's points (the host carries no policy and the card document no meta), W3
off is the raw card under the frame policy meta inside the host, and W4 off mounts the shipped
`CardFrame`. The reference frame (`open.html`) and the scripts-on measurement stay single frames:
the reference has every layer off by definition, and the bridge measurement reads
`event.source === frame()`. One new planted case proves the host's point alone: with the strip and
the card document's own meta both off, every other layer on, neither form arrives in any engine,
after its reference visit reaches both. It is a new test; no existing assertion of the planted suite
changes in any engine, and the layer table's rows are unchanged.

Chosen against:
- Retrying the case: rejected because #787 forbids it, and a retry admits a red that the engine's fetch order decided as green, so the suite would pass with no layer holding the forms.
- A longer fixed wait before the reading: rejected because the arrivals land inside the current settle window, so a longer wait changes no layer and can only read the same arrivals later.
- Marking the cell blind in Firefox: rejected because blind means the reference frame cannot see the channel open, and Firefox's reference reaches both forms; #787 refuses it.
- Narrowing the expectation, or a per-engine table in which Firefox's W3-off variant "opens": rejected because #787 forbids narrowing, and an "opens" cell would read by fetch order too, red whenever the meta wins the race.
- Relabelling the `srcset` removal as part of W2: rejected because it hides the finding instead of holding the forms, and it loses Chromium's and WebKit's reading that the meta alone holds them.
- `img-src` in the page policy, which the card document already inherits: rejected because it changes the app page's own policy, an `https:` card URL can pass a scheme source the page needs, and with W2 off the inherited page policy would still hold `img`, `css-url` and `nested-frame`, moving their layer alone off W2 in every engine.
- The whole frame policy as the host policy: rejected because its `default-src 'none'` falls back into `frame-src` in the host, so the host would refuse the card frame's own navigation and `nav-self` and `download` would leave W4 alone.
- The iframe `csp` attribute (embedded enforcement): rejected because the embedded document must agree to it and not every engine in the suite implements it, so it holds nothing where the finding is.
- A `sandbox` attribute on the host's frame too: rejected because W1 would become two attributes in two places, and the inherited flags already sandbox the card document.
- A frame fetched from a `blob:` or `data:` URL whose response carries the policy: rejected because it is fetched, so the page's `frame-src 'none'` (W4) refuses it.
- Changing the planted markup (`loading="lazy"`) or a browser preference to stop the fetch-ahead: rejected because it measures a weaker plant or a browser no learner runs, and holds nothing in the frame.
- Counting each visit's arrivals by a per-visit token: rejected because D1 found no counting race; the arrivals are this visit's own.

### D3. The sink census names the host's frame, and the reads of the card document go through the host; neither is a weakening

The card's HTML now reaches the page through two `srcdoc` writes, both the card frame's: the host as
`CardFrame`'s `srcdoc`, and the card document as the `srcdoc` of the host's one frame, written by
`frame-host.ts`. The census in `web/app/src/lib/card/card-sinks.test.ts:68` stays an exact equality
and names both, `{ 'src/lib/card/CardFrame.svelte': { srcdoc: 1 }, 'src/lib/card/frame-host.ts': { srcdoc: 1 } }`,
so a sink in any other file, or a second one in either, still fails it. The tests that read the card
document from `CardFrame`'s `srcdoc` (`card-frame.test.ts:27` and `:47`,
`src/lib/study/late-line.test.ts:133`, `review-screen.test.ts:177`, `review-templates.test.ts:242`,
`src/routes/study.test.ts:67`) read it from the host's one frame instead, and assert the same facts.

Chosen against:
- An exception list in the census for the host's file: rejected because a list of allowed files is weaker than an exact population, and admits a second sink in either file.
- Building the host through DOM calls instead of a string: rejected because `setAttribute('srcdoc', ...)` is still a sink to the census, and the re-parse check would have no string to read back.
- A test-only reader exported for the study tests: rejected because `hosted()` is the reader `frameHost`'s own read-back uses, so it is shipped code under the census and StrykerJS, not a test fixture.

### D4. The red-first record: a deterministic red in every engine, read in CI by its check-run

The case `W3 off: srcset stays closed` reads red only when Firefox's fetch comes first, so it is no
red-first test. The record names criteria that read red deterministically before the fix:

| criterion | its test | red before the fix because | read by |
|---|---|---|---|
| A1, A2 | `frame-host.test.ts` | the stub `frameHost` returns the card document as it is | `web` (and locally) |
| A3 | `policy.test.ts` | `HOST_POLICY` is the empty string | `web` (and locally) |
| A4 | `card-frame.test.ts` | `CardFrame`'s `srcdoc` is still the frame document | `web` (and locally) |
| A5 | `card-sinks.test.ts` | the census reads one sink, not two | `web` (and locally) |
| A6 | the planted case, `card.spec.ts` | the stub host passes no policy, so with the strip and the meta off both forms arrive in every engine, with no race | `card-sandbox` in Chromium, WebKit and Firefox (Chromium also locally) |
| A7 | the whole planted suite | not red: the Firefox cell read red only by fetch order | `card-sandbox` |

The record writes A7 as `not red`, naming the CI reds that opened #787 by job id (114200425555,
114216666394, 114215917120, 114224746266, 114225289308, 114225399072, 114225529870, 114227082352)
and the passing job 114191481550, as the observation, never as red-first evidence.

Chosen against:
- The Firefox case itself as the red: rejected because it read green in job 114191481550 at the same tree, and a red that reads only some of the time proves nothing about the fix.
- A test that forces Firefox's fetch-ahead to win: rejected because no harness controls the order of an engine's parser and its tree builder, and A6 reads red in every engine without one.
- A local-only red: rejected because WebKit and Firefox run only in CI here, so the red commit is pushed alone and read in `card-sandbox`.

### D5. FORMAL is not applicable, by surface

No TLA+ model or Lean proof under `formal/` cites the planted suite's counting or the card frame's
layers: the census over `formal/` finds only an unrelated "arrival order" in a reconciliation proof
and an unrelated "planted failure" in a sync model, and no `@phx covers` line names a `web/` path.
The change is a pure string composition with a read-back check, and the interleaving that caused
the red is inside the engine (its parser and its tree builder), an actor this repository does not
write; the fix removes the dependence on that order rather than ordering it. The suite's counter
(the listener and the test) is unchanged, and D1 found no race in it.

Chosen against:
- A TLA+ model of the listener and the test: rejected because the counter is unchanged and the measured arrivals are this visit's own, so the model would check a protocol the fix does not touch.
- A Lean proof of the host's escaping round trip: rejected because the round trip is decided by the engine's HTML parser at run time, and the read-back check refuses any host whose frame does not give back the card document.

### D6. Order, drift and two pushes

This delivery lands first: `dev` itself reads red and every land waits on it. #661's follow-up
starts after it lands, because it edits `web/app/tests-card/card.spec.ts` and
`docs/schematics/card-frame-channels.md` too; it then re-measures section 3 of the schematic (this
delivery edits the W2 row and adds a subsection there, and adds no channel row and no layer row).
#778 shares four study test files and re-measures after this lands. The threat model's citations do not move:
`policy.js:11`, `:18` and `:26` stay (the host policy is appended after line 26),
`card-frame.test.ts:19` stays (nothing is inserted above it), `policy.test.ts:36` stays (the import
line keeps its count, the new test is appended), and `csp.test.ts:103` is not touched.

Two pushes: the red commit alone first, because A6's red in WebKit and Firefox is read only in CI,
then the fix and the record.

Chosen against:
- One push with the red, the fix and the record: rejected because A6's red in WebKit and Firefox would never be read, so its red-first line would rest on Chromium alone.
- Three pushes, the record on its own: rejected because the record changes no test, and every push re-runs the whole battery.
- Landing #661 first: rejected because `dev` stays red for every delivery until this fix is on it.

### D7. No mutation row: StrykerJS proves the changed web files

No row in `scripts/mutation-rows.d/` is anchored in the planted suite or the frame; the `card-sandbox`
rows anchor lines of `.github/workflows/ci.yml`, which this delivery does not touch, so no anchor
moves. `mutation-web` runs StrykerJS over each changed production file whole, with a break threshold
of 100 (`web/app/stryker.config.json`): `frame-host.ts`, `policy.js` and `CardFrame.svelte`.

Chosen against:
- Hand rows in band S40700-S40799 on `frame-host.ts`: rejected because web code with no row table is proved by StrykerJS, and a hand row would prove one mutant of a file the tool already mutates whole.

## Decision Outcome

Chosen: D1 to D7. The card document inherits the host policy from a host document, so W2 holds the
image-set forms from the card document's creation in every engine, the layer table stands as
ADR-416 wrote it, and a new planted case proves the host's point alone.

### Consequences

- Good, because the `srcset` row's "W3, W2" now holds in Firefox by what W2 does, not by fetch order.
- Good, because no planted assertion changes, the page policy and the edge header are untouched, and W1 stays one attribute.
- Neutral, because the card frame nests one frame deeper; the render proof compares it with the reference frame pixel for pixel, unchanged.
- Bad, because the sink census gains a named second sink and six test reads go through the host.

### Confirmation

- `card-sandbox` reads 150 tests per engine, 450 in all, every one passing, `W3 off: srcset stays closed` in Firefox and the new case in every engine included.
- `web` reads A1 to A5 green; `mutation-web` reads no survivor in the three changed production files; `web-engine` reads the study suite green.

## What would make this wrong

- The new case reads an arrival in any engine at the fix: that engine does not put an inherited policy in force before its fetch-ahead, the design is wrong, and the layer table is reopened, never narrowed.
- Firefox reads an arrival in `W3 off: srcset stays closed` at the fix: the cause D1 names is incomplete.
- A single-layer variant opens a set the table does not give it, or the render proof tells the shipped frame from the reference: the host changed what W1, W4 or the learner sees.

## More Information

- Issue #787; #771 and ADR-416 (the strip of every `srcset`); #766, #652, SPEC-398 and ADR-412 (Firefox in the card suite, D6's engine reading); ADR-352 and SPEC-341 (the layers). ADR-416 and ADR-412 are not edited: this ADR records the amendment of their Firefox reading.
- SPEC-407 and `docs/red-first/SPEC-407.md`; the schematic `docs/schematics/card-frame-channels.md` (sections 2, 3 and 9).

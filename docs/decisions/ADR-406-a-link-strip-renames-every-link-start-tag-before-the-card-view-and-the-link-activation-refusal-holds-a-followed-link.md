---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A link strip renames every link start tag before the iPhone and iPad card view, and the link-activation refusal holds a followed link's connection

## Context and Problem Statement

Two residuals of the iPhone and iPad card view are open. #664 asks that a card holding `link`
elements reach the card view with none of them, that a planted `preconnect` card produce no
connection in the probe, and that the strip carry a mutation row whose killer is its test. #677
asks for the cure for the one connection a followed link opened before the navigation gate refused
it, so that the planted suite asserts zero connections for every card.

Measured at `164ac206`:

- The factory hands a card to the view as it came, behind L12's policy only.
  `makeCardWebView(html:)` compiles the rule list and builds
  (`ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift:25-28`), `build` loads the
  card (`:47`), and `load` prefixes the policy (`:59`) and hands the string with no base URL
  (`:64`). Nothing on the native path removes an element. The engine's own document wrapper writes
  no `<link` (`crates/ffi/tests/face.rs:136-153` counts it 0 for its fixture), so every `link`
  element in the card view is the card's own.
- The rule list (L3) already holds a `link` hint. The schematic's section 6 records `preconnect`
  and `shadow-link` at one connection from the reference view and none from the card view, opened
  by removing L3 alone (`docs/schematics/card-frame-channels.md:241-242`). A strip is therefore
  depth beside L3, not the only hold.
- #677's channel is closed in the tree already. SPEC-361 added L10, the link-activation refusal,
  and L11, the page guard. The suite asserts zero connections for every planted card from the card
  view (`ios/CardProbeTests/PlantedCardTests.swift:582-583`), with the planted control that
  `nav-self` and `nav-blank` each open one from that view with L10 removed (`:584-594`), recorded
  green (`docs/red-first/SPEC-361.md:22-23`) and described in the schematic
  (`card-frame-channels.md:461-465`). #677 is still open, and the schematic's section 6 still
  carries the residual (`:232-234`), as does ADR-360 D8's tolerance of one connection.
- The web card frame already removes `link`, `meta`, `base` and `template` before the frame
  (`web/app/src/lib/card/frame-document.ts:17`, `:38-51`), proved by
  `frame-document.test.ts:49` and, for a card whose markup re-parses differently, by its refusal
  (`:75-81`).

## Decision Drivers

- The parser makes a `link` element only from a start tag named `link`. A tag name is the
  characters after `<` up to whitespace, `/` or `>`, with ASCII capitals lowered and nothing else
  folded. So text that holds no `<link`, in any ASCII case, holds no `link` start tag.
- A layer's place decides what the probe can measure alone. The planted suite hands every view that
  carries L12 through `load` (`PlantedCardTests.swift:466-468`, `:346-348`, `:366-368`).
- Lines that other documents cite must keep their content. The campaign's stride table cites
  `CardWebViewFactory.swift:200`, `:204` and `:210`, `CardScripts.swift:7`, and
  `scripts/tests/test_card_web_view_layers.py:178` and `:504`.
- Swift runs only in CI, and the planted suite runs only in CI's `harness` job.

## Decisions, and the alternatives each was chosen against

### D1. L14, a link strip in the factory's build, renames every `<link` to `<wbr` before the card's policy and load

`LinkStrip.stripped(_:)` lives in `ios/CardIsolation/Sources/CardIsolation/LinkStrip.swift`. It
walks the card's UTF-8 bytes and replaces every `<link`, its four letters in any ASCII case, with
`<wbr`, wherever the five characters stand; every other byte is kept, so a card holding none is
returned byte for byte. `build(html:ruleList:switchedOn:)` calls
`load(LinkStrip.stripped(html), into: view)` on the line that loads today
(`CardWebViewFactory.swift:47`), so the strip runs before L12's prefix and L7's string load, and
the shipped view is the only view the factory strips. The planted probe composes it by name.

Every `rel` is covered, because the rename reads no attribute. `wbr` is the target because it is
void, so a renamed tag never wraps the markup after it, and because it has no attribute of its
own, so a renamed tag's attributes grant nothing any element does not already have. The web card
frame needs nothing: W3 already removes every `link` and refuses a document that re-parses with
one. The proof is SPEC-392's: host goldens of the strip, a census that the factory hands every
card through it, and two planted readings (no linked card's card view holds a `link` element; the
reference view with only L14 on reaches nothing and opens no connection for every observable
linked card).

- Chosen: the rename in the factory's build, because five fixed characters need no tokenizer and a rename never has to find where a tag ends.
- A Swift tokenizer that removes each whole `link` tag: rejected because it is a second tokenizer
  that must find a tag's end through quoted attributes, and a misjudged end leaves the tag or
  swallows the markup after it.
- Parsing the card with the view's own DOM and re-serialising it: rejected because a serialisation
  can re-parse into a different tree, the class the web's refusal exists for
  (`frame-document.test.ts:80-81`), and it needs an asynchronous page inside a synchronous build.
- A strip in the Rust engine serving both clients: rejected because the web already strips in W3,
  and it would move an iPhone and iPad layer out of the factory that ADR-360 D2 makes the one place
  every layer is set.
- A document-start observer that removes each `link` as it is inserted: rejected because a hint's
  early connection starts at insertion, before an observer's callback can run.
- A rule-list `css-display-none` on `link`: rejected because it hides an element and removes
  nothing.
- A strip of only the `rel` values that open a connection: rejected because it must read
  attributes, which needs the tokenizer this decision rejects.
- Stripping `meta`, `base` and `template` too, as W3 does: rejected for this delivery because each
  has measured holders on iPhone and iPad (L3, L5, L12) and #664 names `link` alone.
- No strip, relying on L3: rejected because #664's acceptance asks for the strip itself.
- Renaming to `meta`: rejected because a renamed `http-equiv` attribute would become a working
  refresh.
- Renaming to `source`: rejected because a renamed tag inside a media element would load its `src`.
- Renaming to an unknown name: rejected because an unknown element is not void, so it wraps every
  sibling after it and moves the card's layout.
- The strip inside `load(_:into:)`: rejected because the probe hands every view that carries L12
  through `load`, so the strip would ride on single-layer variants that do not name it and move
  their readings silently.
- L14 in `CardScripts.required` and the read-back: rejected because a read-back of the rewrite
  proves only that the handed text holds no `<link`, which a card without links also holds, and it
  would change SPEC-361's census of `required` (`CardScriptsTests.swift:18`, `:41`).

### D2. No new layer holds #677: L10 and L11 already hold a followed link, and this delivery closes the issue on the suite's reading

The browser engine starts a clicked link's early connection inside its link click handler
(`handleClick`), before any delegate is asked, and skips it when the click's default is cancelled
(SPEC-361 section 1, ADR-372 D2). L10 cancels that default from a script world the card cannot
reach, in the scripts-off view as in the scripted one (SPEC-361 R9, ADR-372 D6). The suite measured
the closure: zero connections for every planted card from the card view, and one or more from each
followed link with L10 removed. #677 names only the iPhone and iPad card view. The web suite counts
requests by path and points `nav-self` at its HTTP listener
(`web/app/tests-card/listeners.ts:45-55`, `web/app/tests-card/planted.ts:160`), so it cannot see a
followed link's bare connection; SPEC-392 section 5 records that gap with its owner.

- Chosen: keep L10 and L11 as SPEC-361 built them, because the suite already reads the zero this issue asks for, and close #677 on this delivery's own `harness` reading of it on both simulators. The schematic's section 10 records the closure; section 6's residual paragraph and ADR-360 D8's tolerance stand as history, superseded by that record.
- A navigation-gate change that decides before any request: rejected because the early connection
  starts in the click handler whatever the delegate answers, so no answer from L5 can hold it, as
  SPEC-349's run measured with the gate cancelling every followed link.
- A rule-list entry: rejected because the rule list is asked about loads and hints, not about a
  clicked link's early connection (SPEC-361's channel table).
- Rewriting links or their `href` before load: rejected because it needs the attribute tokenizer D1
  rejects, a script-added link escapes it, and L10 already holds every link a card can activate.
- The retired proxy hold, L9: rejected because SPEC-361 R2 retired it, and a proxy is no containment
  layer until it is measured per destination.
- The browser engine's private per-view switch for link preconnection: rejected because it is a
  private interface (ADR-372 D1).
- Leaving #677 open as a recorded residual: rejected because the measured residual no longer exists
  in the tree, and an open issue over a closed channel misreports the card view.

### D3. One delivery closes both issues

- Chosen: one SPEC and one pull request closing #664 and #677, because both are read by the same planted suite in one `harness` run and both change `PlantedCardTests.swift` and the schematic.
- Two deliveries, the strip and the closure apart: rejected because the closure's only evidence is
  a run of the same suite the strip's readings need, so a second delivery buys a second run of that
  suite and a second edit of the same files, and nothing else.

### D4. No formal model or proof: the strip is a pure transform in a language no formal cover can name

The change adds no actor, no shared state, no timer and no write path. The strip runs
synchronously inside `build`, on the main actor, before the load; L5, L10 and L11 are unchanged.
Its invariant, that no `<link` survives and that text without one is unchanged, is a property of a
total function, which the host goldens and the census decide. A formal cover names only a Rust,
Python or shell item, and of the 219 cover lines under `formal/` at `164ac206` none names a path
this delivery edits.

- Chosen: FORMAL not applicable, because no interleaving is added and the one invariant is a pure property of a Swift function that no cover can tie to a proof.
- A Lean proof of the strip's invariant over a model of the function: rejected because no cover can
  tie the model to the Swift item, so it would prove a copy that nothing keeps equal to the code.
- A TLA+ model of the click handler, the gate and L10: rejected because it would state the browser
  engine's ordering as an assumption, as ADR-372 D9 found, and prove no code the app owns.
- Moving the strip into Rust so a proof could cover it: rejected for D1's reasons for keeping the
  layer in the factory.

### D5. Two pushes: the first carries the tests and the identity stub, so CI reads every Swift red by name; the second carries the strip, the rows and the record

A red predicted and never watched is not a red-first record. Push 1 carries the tests and
`LinkStrip.stripped(_:)` as an identity stub wired into the factory's build, which is the "no
strip" mutant: it leaves every `<link` in place, so A1, A2, A5 and A6, and any other criterion the
stub makes red, fail for their criterion's reason, and CI reads each red by name, `card-isolation`
for the host tests and `harness` on each simulator destination for the planted ones. A4, the
Python census, needs no Swift, so its red is measured before the stub and its green once the
factory is wired. Push 2 carries the strip, the rows and `docs/red-first/SPEC-392.md`, which
writes each Swift red from push 1's run as `red at <sha>: <failure>` with the run, job and step it
was read in. Push 2's checks are read by name: `card-isolation` (A1 to A3 and the strip's Swift
mutants), `harness` (A5 to A8, both simulators in one run), `hygiene` (A4 and the stride table's
citations) and the job that sweeps the script rows (S39200 and S39201).

- Chosen: two pushes, because a red is evidence only when it is watched, and one run of the identity stub lets CI watch every Swift red before the strip exists.
- One push, with each Swift red predicted in the record: rejected because the red is never
  watched, so the record would state a failure nobody saw, and a stub that failed for another
  reason would read the same.
- Running the Swift tests before the first push: rejected because Swift runs only in CI.
- A mutation row killed by a planted simulator test: rejected because the native mutation sweep
  runs only the package's host tests, and a planted killer would cost a whole `harness` run per
  mutant; the identity stub is that mutant, and the record carries A5's and A6's red against it
  from push 1's run.

## Decision Outcome

Every card reaches the iPhone and iPad card view through L14, which renames every `<link` to `<wbr`
in the card's text before L12's policy is prefixed and the card is loaded, so the view holds no
`link` element from the card's markup; L3 still blocks every load a hint or a script-added link
would make. A followed link opens no connection, held by L10 and L11 as SPEC-361 built them; this
delivery re-reads that zero on both simulators and closes #677 on it. One pull request closes both
issues, with two pushes.

### Consequences

- A renamed `link` in the card's `head` ends the head where it stood: what follows it is placed in
  the body, where `style`, `meta`, `base` and `title` are still processed. L12's policy stays first
  in the head, because `load` prefixes it ahead of the stripped card. A card's own policy `meta`
  after a renamed link stops applying; it is no layer of the card view.
- The strip renames the five characters inside text too. A card that shows them in a text area, a
  title, raw text, a comment or an attribute value shows `<wbr`; a card writes them as text with
  `&lt;link`, which the strip leaves.
- The probe gains `referenceWith`, a view the factory does not build whose layers are the set it
  names, and a count of `link` elements read through the app's script.
- No positive planted reading passes through the strip. L14 is in none of the base's single-layer
  variants (`Planted.swift:238-241`); the only static `<link` in the planted cards are the six
  linked cards' and the lookup card's (`Planted.swift:166-177`, `:506`), and every view derived
  from every layer reads those cards at zero already.

### Confirmation

SPEC-392's acceptance criteria A1 to A8, its red-first record, and its rows S39200, S39201 and
SW39200 to SW39204.

## What would make this wrong

- The parser makes a `link` element from text holding no `<link`: L14's premise fails, and a
  planted card of that text must join the linked set.
- The browser engine starts a connection for a `wbr`: the target is wrong.
- A followed link opens a connection from the card view again (A7 red): #677 reopens, and L10 is
  where to look.
- The browser engine gains a public per-view switch for link preconnection: L10 and L11 should
  defer to it (ADR-372).

## More Information

- ADR-360, ADR-366, ADR-372, SPEC-349, SPEC-361, #619, #664, #677, #651.

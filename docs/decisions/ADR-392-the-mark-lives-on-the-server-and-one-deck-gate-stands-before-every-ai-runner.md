---
status: proposed
decision-makers: "the DeckStreak architect"
---

# ADR-392: the mark lives on the server, and one deck gate stands before every AI runner

Decides SPEC-381 (issue #751): a per-deck setting, off by default, that keeps a deck's cards away
from every AI duty and fails closed. It works under SPEC-043's duty run and CHARTER rules 13, 16 and
17, and it amends none of them.

## Context and Problem Statement

At `e7ecf10d` no deck carries a setting of DeckStreak's own, and the engine's call table admits no
deck write (`crates/engine-core/src/table.rs:122-249`). One runner reaches a model, called once,
in `decide` (`crates/agent/src/duty.rs:165`, `:178`), and the duty's input carries card text with no
deck (`crates/agent/src/compose.rs:10-27`). No AI duty runs in production yet: the engine is built
only in tests (`crates/agent/tests/duty.rs:56`, `crates/agent/tests/redteam.rs:91`). The learner
needs to say "never let AI read this deck", the default must be off, and a setting that cannot be
read must stop every AI duty rather than let one through (CHARTER rule 16: the AI route fails
closed, and a silent skip is a defect).

## Decision Drivers

- A mark must hold from the moment it is saved, on every client, including clients that never
  learn of it.
- Exactly one rule decides whether a card is kept away, so two readers cannot disagree.
- Every AI duty, the ones not yet built included, must pass that rule, and a test must say so by
  name.
- No verdict, cause or column the closed sets fix may change.
- The learner's data rights hold for the mark as for every other record (CHARTER rule 13).

## Considered Options (the alternatives each was chosen against)

### D1. Where the mark lives, its name, its default, and what an older client reads

- Chosen: a server table, `sensitive_decks`, owned by `ingest`, because every AI duty reads from the
  server, and a mark there holds at its own commit. One row per marked deck, keyed by the collection's
  deck id; no row is "off", so every deck starts readable. The code calls it `sensitive`; the screen
  calls it "keep away from AI". An older client never reads it, so it cannot weaken it: the server
  is the only reader.
- Chosen against, a field on the deck's record in the synced collection: rejected, because the
  engine's table admits no deck write, and the mark would hold only after both a client sync and
  the server's ingest had run, a window in which a deck the learner marked stays readable.
- Chosen against, the deck's options preset: rejected, because a preset is shared by every deck that
  uses it, so marking one deck would mark its siblings, or force a preset per deck.
- Chosen against, a store on the client only: rejected, because the server's duties cannot read it,
  and every device the learner did not set would read every deck.

### D2. The one gate every AI duty passes, and how it fails closed

- Chosen: one rule, `ingest::sensitive::admits`, asked in two places, because the selection skips the
  deck and the gate in `decide` stops whatever the selection did not. The day set holds back a
  kept-away card (`hold_back_sensitive`). `decide` asks the `DeckGate` port, wired in
  `crates/daemon/src/wiring.rs` over the same rule, after the route check and before the input gate,
  `compose` and the runner. The set unreadable, a deck unresolved, or cards with an empty scope each
  read as kept away. A refusal is `withheld` with class `deck-sensitive` or `deck-unreadable`, through
  the `withhold` the input gate already uses, so it raises the one alert and is never silent.
- Chosen against, the selection alone: rejected, because a mark made between the selection and the
  run would not hold, and a later duty that gathers its cards elsewhere would never meet it.
- Chosen against, the gate alone: rejected, because a run is withheld whole, so one kept-away card
  would stop the reading of every other deck that day.
- Chosen against, a new verdict or cause: rejected, because `agent_runs`'s CHECK fixes the verdict
  set, so a new value means rebuilding the table, while `withheld` with a class already records an
  input refusal (`duty.rs:169-173`).
- Chosen against, a gate inside the runner: rejected, because the runner sees a composed prompt, not
  decks, and by then `compose` has joined the kept-away text to the rest.
- Chosen against, the census as a key of `ai-safety.json`: rejected, because that manifest refuses
  unknown keys, so the AI safety check would read red.
- Chosen against, a gate that skips the run with no record: rejected, because CHARTER rule 16 calls a
  silent skip a defect.

### D3. Where the learner sets it, its words, the locales, and the native clients

- Chosen: one screen, `/study/ai-decks`, because a setting changed rarely belongs beside the deck
  list, not in the study loop; the deck list links to it as it links to the mapping screen.
  One native switch per deck, off unless marked. A deck under a marked deck shows on, cannot be
  changed, and names its marked ancestor. The English words are SPEC-381 R9's; the build writes the
  six other locales, and a test holds every key in every locale.
- Chosen against, a switch on the review screen: rejected, because the review is the study loop,
  and a setting there competes with the grade buttons.
- Chosen against, the words "sensitive" or "private" on screen: rejected, because "sensitive" labels
  the learner's content and "private" reads as hidden from other people; "keep away from AI" says
  what the switch does.
- Chosen against, a mark per card or per note: rejected, because the learner asked per deck, and the
  deck is the unit the learner already arranges.
- Chosen against, the native clients now: rejected, because the native app holds no client of
  DeckStreak's API, and its sign-in to that API is #58's work. The gate still holds every deck the
  learner marked, whichever client studies it.

### D4. How it is tested, and the formal decision

- Chosen: a red-first test per criterion, because a red that is a build failure proves nothing;
  each red is read against a stub that keeps every input and lacks only the behaviour. The census is committed alone first. Rows `S38100` to `S38110` plant one mutant per
  behaviour, and the web files are judged by the web mutation run at a break of 100.
- Chosen against, one end-to-end test through the daemon: rejected, because no AI duty runs in
  production on `dev`, so the test would have to build a duty that does not exist.
- Chosen against, a formal decision of "not applicable": rejected, because a mark, a selection and a
  gate are actors over one set with a check before an act, and `admits` is a total function; the
  first takes a TLA+ entry and the second a Lean entry.

### D5. The shape of the delivery

- Chosen: one delivery under SPEC-381 and this record, because the mark, its rule, its gate and its
  screen are useless apart, and together they are one behaviour.
- Chosen against, an insert-only amendment of SPEC-043: rejected, because a table, two routes, a
  screen and a rule are more than an insert can carry, and SPEC-043's sections stay true as written.
- Chosen against, two deliveries, server then web: rejected, because the server half alone gives the
  learner no way to set a mark, so its gate would refuse nothing a learner chose.

### D6. Export and erase

- Chosen: the marks are exported and erased by delete, because CHARTER rule 13 makes export and
  erase symmetric for every record but two. After an erase every deck is readable again until the
  learner keeps one away, and `PRIVACY.md` says so.
- Chosen against, a reset in place to "every deck kept away": rejected, because rule 13 resets
  singletons, and a per-deck set is not one; it would also mark decks the learner never chose.
- Chosen against, an exemption: rejected, because rule 13 names two exemptions only, the cron-fire
  ledger and the schema version table.

### D7. A deck the server has not seen yet

- Chosen: the PUT accepts any positive deck id, because a deck the learner creates and keeps away at
  once must hold from that moment, before the server's next ingest brings the deck in.
- Chosen against, refusing an id the server has not ingested: rejected, because the server's first
  ingest of a new deck would then make it readable before the learner could mark it.

## Decision Outcome

D1 to D7 as chosen above. `ingest` gains `sensitive_decks` and `admits`; `agent` gains `DeckScope`,
`DeckGate` and the gate in `decide`; `readings` gains `hold_back_sensitive`; `coordination` reads the
set before a day resolves and serves the routes' use cases; `api` gains the two routes; `daemon`
wires the gate; the web gains `/study/ai-decks`; `formal/` gains `SensitiveDeckGate` and
`SensitiveDeck`; and `scripts/mutation-rows.d/S38100-S38199.json` holds the rows.

## Consequences

- Good: a mark holds at its commit, for every client, and the default is off.
- Good: a duty built later cannot reach a model without the census naming it.
- Good: no closed set changes; a refusal is a `withheld` run with a class and an alert.
- Bad: a run whose gate read the set before a mark committed may send that deck's cards once; the
  TLA+ entry states the window.
- Bad: a caller that declares a scope narrower than its cards is out of the census's reach; each
  later duty's SPEC owes that test.
- Bad: the native clients show no setting until they have a client of the API (#58).

### Confirmation

SPEC-381 A1 to A15, the TLA+ entry's witnesses caught and its fixed model clean, the Lean theorems
proved with no `sorry`, the rows killed by hand, and the web mutation run at a break of 100.

## What would make this wrong

- A duty that reaches a model by a shape the census does not list (a network client, say): the
  census then needs that shape, in its own SPEC.
- A learner who expects a mark to follow a deck into another collection client: the mark lives on
  the server only.
- A measured need to mark single cards: a per-card mark would join this rule, never a second one.

## More Information

The flow is `docs/schematics/web-study-screens.md`'s last section. The rows are SPEC-381 section 9.

---
status: accepted
date: "2026-10-01"
decision-makers: "@RexRenatus (owner, at #514), the DeckStreak architect"
---

# DeckStreak writes to the collection only through declared write classes, each with its own ADR in ADR-089's form

## Context and Problem Statement

CHARTER constraint 4 says "The skip day is the ONLY write back to Anki". ADR-089 opened that one
path under the owner's guardrails and kept ADR-037's condition (a), no upload path, whole for every
other path. Two more clauses tie the limit to the sync. ADR-089's guardrail (iii) runs the write
only on the owner's explicit skip declaration. ADR-037's condition (b) allows one scheduled sync per
study day, plus the owner's explicit triggers.

A change that would make study more efficient, whether to a card, a note or a preset, is a write
to the collection, and constraint 4 as written forbids it. The owner has decided to widen the
limit under guard rails (#514), and answered each question it raised:
- the charter: "Sign, with this never-list (Recommended)";
- the writer: "DeckStreak only (Recommended)";
- the target: efficiency only, with no exam-date target;
- derived card data: "Keep, under data-rights (Recommended)";
- card text to a model: "Yes, through the proxy (Recommended)";
- how far a write's power reaches: "Autonomous within guard rails".

Which writes may DeckStreak make, and under which guard rails? What proves each one, and how does a
write earn the right to run without the owner?

## Decision Drivers

- The owner's decision at #514, with the answers above.
- The owner's review history cannot be replaced. No write may destroy a record of what the owner
  did, and every write can be undone.
- A write is the riskiest act DeckStreak has (ADR-089). Its proof is a criterion planned red first
  against the recording fake sync server, and that proof extends to every class.
- One writer. Two automated writers of one collection would race, and neither could undo the
  other's changes.
- An objective that a write can move without any learning is no objective. Suspending a card, or a
  refit judged on its own data, must not count as remembering more.
- The repository is public and the learner is private (CHARTER 11 and 18).

## Considered Options (the alternatives it was chosen against)

- Declared write classes, each with its own ADR in ADR-089's form, promoted from advisory through the owner's approval to autonomous only by a pre-registered trial — chosen because it is the owner's decision at #514, and each write gets its own proof, undo and kill switch before it ever runs without the owner.
- Advisory-only: DeckStreak proposes every change and the owner makes each one in Anki — rejected because the owner chose "Autonomous within guard rails", and a proposal that waits for the owner's hand moves no card on a day the owner does not act, so no trial can measure what the change does.
- Approval-only: every batch waits for the owner's approval — rejected because the owner chose autonomy within guard rails, and an approval that never comes leaves the batch unrun; the approval rung stays as the step every class passes before a trial can promote it.
- A second scheduled sync for writes — rejected because it breaks the intent of ADR-037 (b), one scheduled sync per study day, so DeckStreak would load the sync server twice a day and could contend with another client's sync.
- Supersede condition (a) for every path — rejected because only a declared write class needs an upload, as ADR-089 already found for the skip day, so every other path keeps the proof that it records zero uploads.
- Keep CHARTER constraint 4 as written and widen ADR-089 alone — rejected because a charter constraint binds every delivery and no ADR can widen it; the change is the owner's, and a signed ruling records it.

## Decision Outcome

Chosen option: "Declared write classes", because it is the owner's decision at #514.

**The rule.** DeckStreak writes to the collection only through declared write classes, each with
its own ADR in ADR-089's form.

- **A write** is any change the next sync would carry to the server. That covers a card's due
  date, queue, deck or flag; a filtered deck built, rebuilt or emptied; a preset's options or
  parameters; a note's fields, tags or type; and a card or note added, suspended or deleted. A duty
  whose output needs such a change makes it through a declared write class. Otherwise it does not
  make the change, and proposes it to the owner instead.
- **A write class** is one named kind of write: the skip day's reschedule, for example. Its own
  ADR, in ADR-089's form, states:
  - the exact changes it may make and their exact inverse;
  - its guardrails, each an acceptance criterion of its SPEC, planned red first against the
    recording fake sync server;
  - its undo, and its rung on the ladder of (c);
  - its own formal-methods decision under the repository's formal check (ADR-295): a model or a
    proof, or not applicable with the reason.

### The amended clauses

- **CHARTER constraint 4**, its earlier sentence kept, now adds: DeckStreak writes to the
  collection only through declared write classes, each with its own ADR in ADR-089's form.
- **ADR-037 (a)** is superseded for each declared write class's path only, as ADR-089 did for the
  skip day. Every other path keeps the zero-upload proof against the recording fake sync server, and
  each class's ADR extends that proof to its exact changes and their inverse.
- **ADR-037 (b)** stands: one scheduled sync per study day, plus the owner's explicit triggers. An
  autonomous write class rides the study day's one scheduled sync and adds no scheduled sync, so
  the cadence is unchanged. A batch at the approval rung rides an owner trigger, as the skip day
  does.
- **ADR-089 (i)** now reads: the only writes ever made are each declared write class's exact
  changes and their exact inverses, and every other path records zero uploads against the
  recording fake sync server.
- **ADR-089 (ii)** binds every class: incremental sync only. On any full or one-way sync demand,
  the write aborts, writes nothing and tells the owner.
- **ADR-089 (iii)**: the owner's explicit declaration becomes the approval rung of (c). At the
  autonomous rung, three things replace it: promotion by a passed trial, the guard metric's own
  undo and the kill switch.
- **ADR-089 (iv)** generalises. Each batch records the prior state of everything it changes and
  previews the batch. Its undo writes only cards whose current state still equals what the batch
  wrote, and a card changed since then is skipped and listed to the owner, never overwritten.
- **ADR-089 (v)** stands, and each class's ADR extends its recording-server proof to that class's
  exact changes and their inverse.
- **The skip day is the first declared write class**, and ADR-089 is its ADR. It sits at the
  approval rung for good, because the owner's skip declaration approves each batch. Parts (a) to
  (f) bind it like any other class. Its planned SPEC-083 takes (b)'s backup, restore drill and
  counts through its own amendment before its write is built (#108). This ADR changes neither
  ADR-089's design of the skip day nor SPEC-083.

### (a) The never-list

No write class makes any of these writes, at any rung, and no owner approval admits one. Each
entry names what it protects.

| # | a write class never | what it protects |
|---|---|---|
| 1 | edit review history | the record of every answer the owner gave, from which XP, streaks, the scheduler's fit and every trial's score are computed |
| 2 | forget or reset a card | the memory state the scheduler built for that card from the owner's own reviews |
| 3 | delete a preset | the options and the scheduler parameters fitted to the owner's reviews, and every deck that uses them |
| 4 | force a full sync | the server's copy and every other client's unsynced changes, which a full upload would overwrite (ADR-089 (ii)) |
| 5 | mass-reschedule | the scheduler's own spacing of every card a batch did not mean to move, and the owner's daily review load |
| 6 | set a per-card due-date policy | the scheduler as the one authority on when a card is due, which a standing per-card rule would override unseen |
| 7 | change a reviewed note's type | the reviewed note's cards and their history, and the incremental sync, since Anki asks for a one-way sync after a note's type changes |
| 8 | delete a reviewed card or note | the card's review history and the observed outcome that (e) scores, which a deletion would hide |

- **Reviewed** means that one of the note's cards has a review (the LEXICON's review: a revlog row
  of type 0 to 3 with ease 1 or more). A note is unreviewed only when none of its cards has one.
  Editing a note edits every card of it.
- A note that reached the collection in a package (ADR-151) is changed at its package's source and
  reaches Anki in a newer package, never through a write in the collection. A write there would be
  overwritten by the next newer package, or would leave the note forked from its source.

### (b) The backup and the batch

- Every write batch takes a whole-collection backup first. A restore drill proves the backup
  before the class's first batch, and again whenever the way the backup is taken changes: it
  restores the backup into a throwaway collection and compares its counts with the source's.
- A batch is all-or-nothing. It makes every change it previewed, or none.
- A batch records counts before and after it runs: the cards, the notes, the review rows, and the
  cards in each state the batch touches. Its class's ADR names the counts the batch may move. Any
  other count that moves stops the class, and is reported to the owner.
- A batch is undoable. Its undo is the batch's exact inverse, under the rule of ADR-089 (iv) as
  amended above.
- Every edit to a reviewed note writes a change point to DeckStreak's ledger: the note, the
  fields before and after, and the class that made the edit. A trial can then separate an edit's
  effect from everything else.

### (c) The promotion ladder

Every write class climbs three rungs, one at a time, and starts on the first.

1. **Advisory.** The class proposes its batch to the owner and writes nothing. Every class starts
   here, and stays here until its own ADR and SPEC are accepted.
2. **Approval.** The owner approves each batch, and the batch rides that owner trigger
   (ADR-037 (b)).
3. **Autonomous.** The batch runs without the owner, on the study day's one scheduled sync. A
   class reaches this rung only after a pre-registered n-of-1 trial passes. The trial's
   hypothesis, arms, objective (e), guard metrics, length and pass rule are all committed before
   it starts, and none of them changes while it runs.

- A trip of a guard metric undoes the batch on its own, by its exact inverse under the rule of
  ADR-089 (iv) as amended. It returns the class to the approval rung and tells the owner.
- A kill switch the owner holds stops every class at once. While it is set, no batch starts, a
  running batch aborts and writes nothing more, and every class falls back to advisory.
- A score that the class's own fit produced is never its guard. A fitted parameter is judged on
  reviews made after the fit (a hold-out split by time), never on the reviews it was fitted to.
- Each class's ADR states its change budget, its dwell time between changes, and the band a
  result must clear before a change is reversed. While a trial runs, no class changes what the
  trial's arms share. A change that must run anyway is recorded as a ledger change point, and the
  analysis takes it as a covariate.

### (d) The single writer

- DeckStreak becomes the only automated writer of the collection.
- The predecessor's skip-day write retires once DeckStreak's skip day lands (SPEC-083), so the
  two never write the same cards.
- Cards from every other generator reach Anki only through DeckStreak, as a declared write class,
  or as a package the owner imports (ADR-151). The import is the owner's own act, and a package's
  notes are edited at their source, never in the collection.

### (e) The objective

- A write class is scored on cards remembered per minute studied, over observed outcomes: the
  answers the owner actually gives, never the scheduler's predicted recall.
- The denominator is the set of cards a trial scores, and it is frozen at trial start. A suspended
  or deleted card counts as a fail, and a card split in two counts as one item.
- There is no exam-date target. The objective is efficiency alone, and no write class plans
  toward a date.
- XP and the game score are never the objective, and no class is scored on them. The game's math
  (CHARTER 8) is unchanged.

### (f) Card text, derived data and persona review

- Card text reaches a model only through the deployment's one AI route (CHARTER 16, ADR-054), the
  owner's subscription proxy. No class, script or generator sends card text to any other model or
  service.
- Embeddings and similarity edges derived from card text may be kept in DeckStreak's ledger. Each
  table holding them carries a data-rights entry with a retention limit, and is exported and
  erased with the rest (CHARTER 13, SPEC-021). Any other data a class derives about how the owner
  studies is kept under the same rule.
- Card text or card ids that leave the collection for any other store, the vault included, need
  their own ADR and data-rights entry first.
- A persona's review of new cards follows CHARTER 18: public templates, private roster. The
  repository names no persona, and the review's output passes the packs' gate before it reaches
  the owner (CHARTER 17).

### (g) What it was chosen against

- Advisory-only and approval-only, each rejected in the considered options above. Advisory-only
  never moves a card the owner does not move, so no trial can measure it. Approval-only never runs
  a batch the owner does not approve. The ladder keeps both as rungs, and adds the autonomous rung
  above them, behind a trial, a guard metric and a kill switch.

### Consequences

- Good, because a write that makes study more efficient can run, each class under its own proof,
  undo and kill switch, and no class can destroy the record of what the owner did.
- Good, because the sync cadence is unchanged. An autonomous write rides the study day's one
  scheduled sync, and an approved batch rides the owner's own trigger.
- Good, because the objective cannot be gamed. A suspended or deleted card is a fail, and a refit
  is never judged on its own data.
- Bad, because DeckStreak can now change the owner's collection on more than one path, so a defect
  in any class reaches the owner's cards. The never-list, the backup, the all-or-nothing batch, the
  undo, the guard metric and the kill switch bound it, and no class writes before its own ADR and
  SPEC are accepted.
- Bad, because an undo still cannot see a change that another client made between the undo's sync
  and its push (ADR-089's consequences). Every class inherits that limit, and its ADR states its own
  form of it.
- Bad, because five documents still state the old limit in their own words: ADR-083, ADR-151,
  SPEC-001's gate-6 amendment, and the planned SPEC-083 and SPEC-151. They are read under this ADR.
- Open questions for the first write class's ADR:
  - a spend ceiling across every class's runs per study day, read from each run's recorded cost
    (`agent_runs`), with a fall-back to writing nothing when the AI route is absent;
  - whether the engine's due-date write adds a review-log row, and how (e) and a trial then count
    it;
  - what computing embeddings costs, measured before any class adopts them.

### Confirmation

- SPEC-301's criteria A1 to A7, each recorded red then green in `docs/red-first/SPEC-301.md`, pin
  the rule, the parts (a) to (g), the never-list, the options and the three amendment notes.
- Each write class's own SPEC proves that class against the recording fake sync server, red first.
  It proves the class's exact changes and their inverse, zero uploads on every other path, the
  abort on a full or one-way sync demand, the backup and its restore drill, the counts, the undo,
  the guard metric's own undo and the kill switch.
- The owner's signed ruling, `docs/rulings/OWNER-RULING-2026-10-01-deck-writes.md`, records the
  charter change.

## What would make this wrong

- The owner withdraws autonomy. The ladder then stops at the approval rung, and the autonomous
  rung of (c) is struck.
- A class's trial passes while its observed outcomes fall. The objective is then mis-specified, and
  every autonomous class drops to the approval rung until (e) is amended.
- A proof shows a write outside a class's declared changes, or an upload on any other path. Every
  class then drops to advisory until that proof is green.
- The owner's sync server demands a full sync so often that most batches abort. Autonomous writes
  then go back to the owner.

## More Information

CHARTER constraints 4, 8, 11, 13, 16, 17 and 18; ADR-037; ADR-089; ADR-083; ADR-151; ADR-054;
ADR-295; SPEC-301; SPEC-083; SPEC-021; SPEC-001 §14; #514; #108.

This delivery builds no write path and touches no interleaving or invariant surface, so it carries
no model and no proof. Each write class's ADR states its own formal-methods decision.

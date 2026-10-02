---
status: accepted
date: "2026-10-02"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A staged duty run guards each operation against the journal when it is applied, and a refusal stops the run

## Context and Problem Statement

The three-verb executor (`crates/vault/src/staged.rs`, SPEC-042 R4) applies a staged duty run: it
checks the run, asks the gate, checks it again, and then applies the run's operations one by one,
the folders a target needs, a capture renamed into place and a note's bytes. The owner's devices
keep writing the vault while the gate runs, so a duty's folder can become a link into a journal
folder after the checks have passed. SPEC-118 R5 forbids any vault write under a journal folder as
the file system resolves the path when the write runs (#56), and SPEC-118 section 11 named the
executor's `rename` and `create_dir` as calls outside the journal guard.

Measured before the guard: at the census's red commit,
`every_folder_and_rename_call_of_the_vault_is_guarded_or_named` listed `staged.rs`'s `rename` in
`apply_op` and its `create_dir` in `create_folders` as neither guarded nor named, and at the red
commit each of the four `staged_guard` tests read "the run reached the journal through the link",
after its control without the link had applied both operations.

## Decision Drivers

- R5's rule holds for every write a staged run applies, as the path resolves at that write.
- The checks before and after the gate stay as they are: they refuse a run that names a journal
  folder, or whose folder resolves into one when they run.
- Nothing a refused run leaves behind is under a journal folder, and the owner can see how far the
  run got.

## Considered Options (the alternatives it was chosen against)

- Guard each operation when it is applied, resolving its paths at that operation: a move's source
  first, then each folder the target needs before it is created, then the note's bytes or the
  rename; a refusal stops the run and the operations before it stay applied: chosen, because
  `NoJournalWrite` holds in `formal/tla/StagedNoJournal/` over every run of two operations of every
  kind over every folder with up to three changes by another writer, and each option below
  violates it in a witness (#56).
- Trust the checks alone, applying the run unguarded after the re-check: rejected, because a link
  the vault gains after the re-check carries the write into the journal; the witness
  `an-apply-that-trusts-the-check-alone.cfg` violates `NoJournalWrite`, and the four `staged_guard`
  tests were red for exactly this (#56).
- Resolve every target once before the first write: rejected, because a folder that becomes a link
  after the first write is never resolved again for a later operation; the witness
  `targets-resolved-once-before-the-first-write.cfg` violates `NoJournalWrite` (#56).
- Roll back the operations applied before a refusal: rejected, because undoing a rename or a write
  is itself a write into a vault the owner's devices are changing, so it needs the same guard and
  can itself be refused, leaving a run half undone; the operations before a refusal were each
  checked and each admitted by the guard, so the run stops and says how many were applied (#56).
- Guard the byte writes only, leaving `create_dir` and `rename` as they were: rejected, because a
  folder created or a capture renamed through a link lands in the journal; the witness
  `a-guard-over-byte-writes-only.cfg` violates `NoJournalWrite` (#56).
- Compare the paths' names without resolving them: rejected, because a link's own name is not
  under a journal folder while the folder it resolves to is; the witness
  `a-guard-that-compares-names.cfg` violates `NoJournalWrite` (#56).

## Decision Outcome

Chosen option: the guard at apply time over every kind of operation, with each operation's paths
resolved at that operation.

`staged.rs::guard` builds, for each operation, a `JournalGuard` over the journal folders of the
run's own layout, wrapping `staged.rs::Borrowed`, which passes every call straight through to the
executor's own file system. `apply_op` refuses a move's source first, through
`atomic::refuse_journal_resolved`; `create_folders` then refuses each folder the target needs
before it creates it; then the note's bytes go through `atomic::write` over the guard, or the
target is refused and the rename goes through the guard. That source-before-folders order means a
move whose source is refused creates no folder. A refusal is `VaultError::JournalRefused`, and
`apply` stops the run there as `RunOutcome::Stopped { applied, refusal }`, counting the operations
applied before it, which stay applied.

### Consequences

- Good, because R5's rule now holds for the staged duty runs' writes as well as the inbox
  capture's, and `journal_guard_census.rs` holds every folder, rename and removal call of
  `crates/vault/src` on the guard or named, so a new call outside the guard fails a test.
- Good, because a stopped run reports how far it got, and nothing it wrote is under a journal
  folder.
- Bad, because a stopped run leaves its earlier operations applied: the owner sees a partial run,
  each of whose applied operations was checked and admitted.
- Bad, because each operation resolves its paths again, a few more file system calls per
  operation.
- The delivery's census amendments, made at its green commit: `one_router.rs` counts
  `COMMAND_REPLIES` 14 to 15, `COMMAND_CALLERS` 25 to 27 and `REQUEST_SITES` 21 to 22 for the
  bot's media capture; `log_capture_class.rs` counts the routed log calls 18 to 19; and
  `atomic.rs` gains `PORT_DELEGATION`, `("staged.rs", "create_new(", 2)`. That last is an
  admission, a named delegation class with an exact count, and not a loosening: `Borrowed` names
  the port's `create_new` once and passes it through once, and a third mention in that file is
  refused by `every_vault_file_write_is_the_atomic_writer` as any other call outside the writer is.

### Confirmation

- `formal/tla/StagedNoJournal/`: `NoJournalWrite` holds at the bound `NumOps=2 MaxEnv=3`, and each
  of the four witnesses violates it.
- `crates/vault/tests/staged_guard.rs`: a note, a folder, a capture filed into the journal and one
  taken out of it, each through a link the vault gains after the checks, is refused when applied;
  each test is red at the red commit and green at the green commit.
- `crates/vault/tests/journal_guard_census.rs`: the census over the vault's folder, rename and
  removal calls, and its control, which moves one guarded call onto the executor's own file system
  and is flagged once.
- Mutation rows S11818 to S11824 on `crates/vault/src/staged.rs`, each killed by a `staged_guard`
  test.

## What would make this wrong

- A vault writer outside `crates/vault/src`: the census reads that tree, so a writer in another
  crate is outside its population.
- A new kind of operation in a staged run, a removal say: the model's kinds are `dir`, `rename`
  and `write`, so a new kind needs its own arm in the model and in `apply_op`. The covers of
  `apply_op` and `create_folders` read STALE on such an edit.
- A run whose failure needs more than two operations, or more than three changes by another
  writer, to show: the bound would not reach it.
- A stopped run whose applied operations leave the vault in a state the curator cannot read: the
  stop would then need a recovery step (#56).

## More Information

Issues #154 and #56; SPEC-118 sections 11 and 12; SPEC-042 R4; ADR-042; ADR-118;
`formal/tla/StagedNoJournal`; `formal/tla/CaptureOnce`.

# ADR-500: the later-day refund property moves to block only when the formal checker admits its streak, and the port test keeps the shared key

- **Status:** accepted; it decides SPEC-483 (issue #549, the coin wallet of #106).
- **Read at:** dev `36c4783a`; every `path:line` below is at that commit unless it names another.
- **Amends:** nothing. SPEC-082, ADR-308 and `docs/schematics/coin-wallet-and-its-ports.md` keep
  their text; `docs/schematics/formal-check-settings.md` gains one appended section (SPEC-483
  section 4).

## Context

`formal/tla/WalletFloor/WalletFloor.tla` models the coin wallet's ports over one ledger (#106). Its
header declares five properties, each at `ramp=report` (`:13-17`), and one witness for each, which
switches one defect on (`:18-22`). `RefundWritesItsOwnMovement` is declared at `:17` and defined at
`:284-286`. Its witness, `witness/a-refund-guarded-once-ever-over-every-day.cfg`, sets
`RefundKeyedPerDay = FALSE` (`:6`) and checks that property alone (`:12`). `MCWalletFloor.cfg`
checks `TypeOK` and all five properties (`:15`) with every fix switch on (`:6-10`), at three tasks,
two study days and amounts up to two (`:3`, `:11-13`), over a floor of 180580 states (`:4`).

The property entered in report mode, as every new property does, in the review round of #543 that
added it with its witness. #549 records what follows from that. In report mode the property cannot
fail the formal check until it is promoted to block. Until then the port test
`wallet_ports::a_refund_on_a_later_day_of_a_held_key_writes_its_own_movement`
(`crates/economy/tests/wallet_ports.rs:215`) holds the later-day refund. And because the model gives
a refund its own source (`WalletFloor.tla:56`, `:80`), a deposit and a refund that share one key stay
outside the model, held by that test alone. The test's three cases are a deposit (K-009), a once-ever
deposit (K-021) and a refund (K-033) on day 30, each followed by a refund of the same source and
reference on day 31 (`:225-227`, `:232-234`, `:243`), and each must write two movements (`:253`). It
was added after its code as mutation coverage, and it was proved by routing `refund_on` through the
once-ever guard (`docs/red-first/SPEC-082.md:86-95`).

SPEC-389 states this repository's rule for a ramp
(`docs/specs/SPEC-389-a-lean-entry-proves-confirmed-xp-is-never-below-the-xp-shown.md:251-252`): a
property moves from `ramp=report` to `ramp=block` only by the streak of clean landings the checker
counts. At `36c4783a` no `@phx property` line anywhere reads `ramp=block`; a search of the tree for
`ramp=block` finds only that sentence. No script or test in this repository reads a ramp: a search
for `ramp=` or `@phx property` lists 48 files, 7 SPECs and 41 formal entries, and none under
`scripts/` or `crates/`. No CI job runs a formal check (`docs/schematics/formal-check-settings.md:91-93`,
read at `32f6217`; #806). The formal verdict at a head is the checker's own check, run by hand at the
delivery's head, at its review and at its landing, as SPEC-389 section 8 states for its own entry.

## Decision

### D1. What a ramp is, who reads it, and which run fails on it

- The formal checker, outside this repository, reads each property's ramp from its header line
  (`WalletFloor.tla:13-17`) when it builds the entry's registry. Nothing in this tree reads it.
- At `report`, a `VIOLATION`, a `STALE` cover or a `TIMEOUT` on the property is printed on the
  check's card and never moves the exit of a check that no single property narrows. At `block`, the
  same finding fails the entry's gate check: the property must be clean at its state floor. A check
  narrowed to one property exits non-zero on any unclean reading, whatever the ramp.
- The streak's length is `config/formal.json`'s `k` (`:2`, 20; pinned by
  `scripts/tests/test_formal_config.py:25`). The checker counts the first-parent landings, back from
  the merge-base, at which the entry's inputs are unchanged: its files, the toolchain identity
  (`config/formal.json:32`) and every span its covers name (`WalletFloor.tla:2-11`, nine items of
  `crates/economy/src/wallet.rs` and `write` in `crates/kernel/src/db.rs`). A re-stamp, or any move
  of a covered span, restarts the count at that landing. The checker derives the count from the base
  branch's history at each reading and prints it on the card beside the ramp and the clean state; no
  file in this tree keeps it.
- The checker's ramp reading names the verdict on a flip. A flip it reads admitted (a streak of `k`
  or more, the property clean, and no other change to the entry's inputs) may land. A flip below
  `k`, a flip bundled with another change to the entry's inputs, or a flip of a property that is not
  clean is a premature promotion, which fails the check in either mode. Moving a property back from
  `block` to `report` is a weakening that only an owner-signed ruling admits.
- In this repository today, no CI run reads a ramp, so no CI run fails on a violated `block`
  property. The run that fails on it is the formal check at a delivery's head and at its landing,
  against live dev. At `report`, that check's exit passes over the same violation, which it only
  reports.
- Not measured by this decision: the entry's streak at `36c4783a`; whether the property is clean
  there (it was clean at the round-2 head of #543, with the other 21 properties of that check); and
  the exact words the ramp reading prints. The delivery reads all three before it flips (SPEC-483
  section 3a).

### D2. The promotion, and what proves it

This delivery moves `RefundWritesItsOwnMovement`, and only it, to `ramp=block`, as one change to
`WalletFloor.tla:17` and to no other byte of the entry, and only when the checker reads the flip
admitted against live dev. Any other reading stops the delivery before its push, and #549 stays open
until the streak is earned. So the promotion takes the rule's own path (SPEC-389:251), an earned
flip, never a step around it. Nor is it a no-op: at `block`, the formal check at landing fails on the
violation that it now only reports.

What proves the property at `block`:

1. the checker's ramp reading of the flip, against live dev: admitted;
2. the entry's gate check at the head, against live dev: every property of `MCWalletFloor.cfg` clean
   at its floor, each of the five witnesses caught, no stale cover and no premature promotion;
3. a planted violation, in a scratch clone only: with `RefundKeyedPerDay = FALSE` committed in
   `MCWalletFloor.cfg` on top of the flip, the gate check fails on the property's violation; the same
   plant over live dev, where the property reads `report`, passes with the violation reported; and
   the unplanted head passes (item 2);
4. the pin A1 of SPEC-483, which CI reads by name: the committed entry declares the property once, at
   `block`.

What stays outside the model, held by the port test alone: a refund on a later study day of a source
and reference that a deposit (K-009) or a once-ever deposit (K-021) moved earlier. The model's refund
writes only the `"refund"` source (`WalletFloor.tla:56`, `:80`), so its witness reaches the
refund-then-refund case (K-033) and no case in which a refund shares a deposit's key. The test and its
file are unchanged.

### D3. Which file holds each reading

| reading | file | kind | read by | this delivery |
|---|---|---|---|---|
| the ramp | `formal/tla/WalletFloor/WalletFloor.tla:17` | formal source | the formal checker | `report` to `block`, the entry's one changed line |
| the configuration's invariants | `formal/tla/WalletFloor/MCWalletFloor.cfg:15` | formal source | the formal checker | unchanged |
| the five witnesses | `formal/tla/WalletFloor/witness/*.cfg`, each at `:12` | formal source | the formal checker | unchanged; none dropped |
| the later-day refund | `crates/economy/tests/wallet_ports.rs:215` | Rust test | the Rust tests in CI | unchanged |
| the ramp's pin | `scripts/tests/test_wallet_refund_ramp.py` | Python test | the Python tests in CI | added |
| `k` and the budgets | `config/formal.json` | settings | the formal checker; `scripts/tests/test_formal_config.py` | unchanged |

No property, invariant or configuration is loosened, no witness is dropped, and no state floor or
budget falls. The pin reads only the header line's text, never the model, so the invariants and the
witnesses keep one reader, the checker, whose own rules already refuse dropping a property, a witness
or a configuration.

### D4. One delivery

One pull request into dev under SPEC-483 and this decision, in the band `S48300-S48399`: the flip,
the pin, its rows, its red-first record, its changelog fragment and one appended section of
`docs/schematics/formal-check-settings.md`. SPEC-082 states no ramp (its R7 and A19 stand as written)
and the wallet's schematic states no model, so neither is amended. A CI job that reads ramps is
#806's work, never this delivery's.

### D5. Tests, rows and the push

- A1, red first: the pin reads the committed entry. It is red at the commit that adds it, where `:17`
  reads `report`, by an assertion on the reader's verdict, and green at the flip.
- A2, mutation coverage: the same reader, over a fixture tree, refuses a `report` line, a missing line
  and a doubled line, and admits the `block` line. It reads fixtures, so it passes when written; its
  reds are rows S48302 and S48303.
- Rows: S48301 on the ramp line (`block` back to `report`, killed by A1); S48302 and S48303 on the
  reader's two refusal arms (each killed by A2).
- The checker's readings (D2, items 1 to 3) are made by hand at the head, never in CI, and are
  recorded in the red-first record's prose.
- Every red is read before the push, so the delivery pushes once.

### D6. What this delivery leaves out

- The other four properties at `report` (`FloorHolds`, `OneMovementPerKey`, `DepositOnceEver` and
  `SettledMintNeverFalls`, `WalletFloor.tla:13-16`). They share the entry's streak, and each moves by
  its own earned flip.
- A CI job that checks formal entries (#806).
- A model of a deposit and a refund that share one key (D2), and a mutation row whose killer is the
  later-day refund test.
- Anything only a later CI run measures: the pin's and the rows' readings in CI, read by name after
  the push.

## What it was chosen against

### D1

- Reading the ramp in a repository script: no script here reads one, and a second reader of the
  header beside the checker would decide nothing the checker does not.
- Treating a `block` property as a CI gate today: no CI job runs a formal check (#806), so only the
  check at a delivery's head and at its landing fails on it.

### D2

- A ruling that #549 is not needed: the promotion is the one act #549 tracks, and without it the
  property can stay in report mode with no record that it should move.
- A flip now, whatever the streak: a flip below `k` is a premature promotion, which fails the
  landing's own formal check in either mode.
- Waiting for the CI job of #806 before the flip: the check at landing already fails on a `block`
  property, so a CI job adds a reader and changes nothing that `block` decides.
- Modelling the shared key in this delivery: any edit to the entry's files moves its inputs, so the
  flip would read bundled and premature, and every property's streak would restart.
- Promoting all five properties at once: #549 tracks the refund property alone, and each earned flip
  is judged on its own.

### D3

- The pin in `scripts/tests/test_formal_config.py`: that module pins the settings file, not an entry,
  and an open pull request changes it.
- A Python reader of the invariants and the witness configurations: it would be a second gate over
  the model, beside the checker that already refuses dropping any of them.
- Lowering the state floor or the entry's budget so the plant runs faster: each is a weakening, and
  nothing here needs one.

### D4

- A new schematic: the settings schematic already maps `config/formal.json` to the witness kill, and
  `k` is a setting of that file, so the ramp's step belongs there.
- Two deliveries, the pin first: a pin that asserts `block` cannot land before the flip, and a pin
  that admits either ramp decides nothing.
- An amendment of SPEC-082: it states no ramp, so nothing it says changes.

### D5

- The two-push form: every red here is read before the push, and none only in CI.
- A planted violation in the committed tree: it would land a violated model and move the entry's
  inputs.
- A fence line for the checker's readings: a public acceptance fence holds test commands only.

### D6

- Folding the four other flips in here: each is its own earned flip, outside #549.

## Consequences

- At `block`, a violation, a stale cover or a timeout on the refund property fails the formal check
  at a delivery's head and at its landing. A delivery that moves any covered span (the nine
  `wallet.rs` items of `WalletFloor.tla:2-10`, or `write` in `crates/kernel/src/db.rs`) re-reads and
  re-stamps the entry in the same pull request, or its landing stops.
- The property cannot move back to `report` without an owner-signed ruling.
- CI still runs no formal check. The pin is the only reader of the ramp in CI, and it reads text only.
- The streak is read again at landing, against the live dev. A landing in between that moves a
  covered span restarts it, and the landing waits until the streak is earned again.

## What would make this wrong

- A check that judges this repository's formal entries against a base other than live dev, such as
  the release branch, where `formal/tla/WalletFloor/` is absent at `3d77726e`: there the flip would
  read premature.
- The checker counting the streak, or reading `block`, other than as D1 states: the delivery's
  readings come before the flip and stop it.
- A CI job (#806) that reads ramps: then the pin duplicates it, and its retirement is that job's own
  decision.
- A refund port keyed by its source and reference over every day: then the model's own source for
  refunds no longer matches the code, and the boundary of D2 moves.

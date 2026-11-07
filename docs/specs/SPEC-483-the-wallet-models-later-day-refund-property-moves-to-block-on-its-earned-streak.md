# SPEC-483: the wallet model's later-day refund property moves to block on its earned streak

- **Issue:** #549 (the coin wallet of #106). **Context(s):** the formal entry `tla/WalletFloor`;
  the Python tests under `scripts/tests`.
- **Decided by:** ADR-500.
- **Prerequisites:** SPEC-082 (the wallet, its model and the later-day refund port test) and SPEC-389
  (the rule that a ramp moves only by the checker's streak). **Mutation band:** `S48300-S48399`.
- **Status:** proposed, for one pull request into dev that closes #549. The ramp moves only when the
  formal checker reads the flip admitted (section 3a); any other reading stops the delivery before
  its push.

## 1. The problem, measured

Each reading is at dev `36c4783a`.

- `git show 36c4783a:formal/tla/WalletFloor/WalletFloor.tla | grep -n '@phx property'` prints five
  lines, `:13` to `:17`, each at `ramp=report`. Line `:17` is `RefundWritesItsOwnMovement`, the
  property that a refund on a later study day of a source already held writes its own movement.
- `git grep -n 'ramp=block' 36c4783a -- .` prints one line,
  `docs/specs/SPEC-389-a-lean-entry-proves-confirmed-xp-is-never-below-the-xp-shown.md:251`, which is
  prose: no property anywhere is at `block`.
- `git grep -l -E 'ramp=|@phx property' 36c4783a -- .` lists 48 files, 7 under `docs/specs/` and 41
  under `formal/`, and none under `scripts/` or `crates/`: no test in this repository reads a ramp.
- `formal/tla/WalletFloor/MCWalletFloor.cfg:15` checks `TypeOK` and the five properties, and each of
  the five configurations under `witness/` checks one of them (`:12`).
- So at `report` a violation of the refund property is reported and fails nothing (#549). The
  later-day refund is held by `crates/economy/tests/wallet_ports.rs:215` alone, and that test is also
  the only holder of a refund that shares a deposit's key (`:225-226`), which the model cannot
  express because it gives a refund its own source (`WalletFloor.tla:56`, `:80`).

## 2. Requirements

R1. `formal/tla/WalletFloor/WalletFloor.tla:17` reads
    `\* @phx property RefundWritesItsOwnMovement ramp=block`, and it is the only line of the entry
    this delivery changes: the other four property lines, the covers, the witness lines, the model's
    definitions, `MCWalletFloor.cfg` and the five witness configurations are byte-identical to dev.
R2. The flip lands only when the formal checker reads it admitted against live dev: the entry's
    streak at `config/formal.json`'s `k` or more, the property clean, and no other change to the
    entry's inputs.
R3. At `block`, the entry's gate check at the head is clean: every property of `MCWalletFloor.cfg`
    clean at its state floor, each of the five witnesses caught, and no stale cover.
R4. A violation of the property fails the gate check at `block`: `RefundKeyedPerDay = FALSE` planted
    in `MCWalletFloor.cfg`, in a scratch clone of the head, fails it, while the same plant over live
    dev, at `report`, passes with the violation reported.
R5. A test under `scripts/tests` reads the committed entry and passes only when exactly one
    `@phx property` line names `RefundWritesItsOwnMovement` and that line reads `ramp=block`. Over a
    fixture tree it refuses a `report` line, a missing line and a doubled line, each by its own
    verdict, and it reports how many plants it examined, refusing fewer than all of them.
R6. No byte under `crates/` and no byte of `config/formal.json` changes: the later-day refund port
    test, its three cases and the wallet's code stand as on dev.
R7. Rows S48301 to S48303 hold the ramp line and the reader's two refusal arms, each killed by its
    named test.

## 3. Acceptance criteria of SPEC-483

| id | criterion | decided by |
|---|---|---|
| A1 | the committed entry declares `RefundWritesItsOwnMovement` on exactly one property line, at `ramp=block` (R1, R5) | `test_wallet_refund_ramp.WalletRefundRamp.test_the_refund_property_is_declared_once_at_block` |
| A2 | over a fixture tree the reader refuses a `report` line, a missing line and a doubled line, admits the `block` line, and reports every plant examined (R5) | `test_wallet_refund_ramp.WalletRefundRamp.test_the_reader_refuses_each_planted_ramp` |

```acceptance
A1: python3 scripts/tests/test_wallet_refund_ramp.py WalletRefundRamp.test_the_refund_property_is_declared_once_at_block
A2: python3 scripts/tests/test_wallet_refund_ramp.py WalletRefundRamp.test_the_reader_refuses_each_planted_ramp
```

## 3a. What the formal checker judges, by hand at the head and at landing

These have no line in the fence, because a public acceptance fence holds test commands only. Each
is read at the delivery's head and again at its landing, against live dev, and recorded in
`docs/red-first/SPEC-483.md`.

| id | criterion | decided by |
|---|---|---|
| F1 | the flip is admitted: the entry's streak at `k` or more, the property clean, and nothing else of the entry's inputs moved (R1, R2) | the checker's ramp reading of `tla/WalletFloor`, property `RefundWritesItsOwnMovement` |
| F2 | the entry's gate check at the head is clean: five properties clean at the floor, five witnesses caught, no stale cover (R3) | the checker's gate check of `tla/WalletFloor` |
| F3 | the planted violation fails the gate check at `block` and passes it, reported, at `report` (R4) | the same gate check, over two scratch clones |
| F4 | the entry's change is one line: `git diff --numstat` against live dev over `formal/tla/WalletFloor/` reads one added and one removed line of `WalletFloor.tla`, and the diff names no path under `crates/` and not `config/formal.json` (R1, R6) | the diff |

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-483-the-wallet-models-later-day-refund-property-moves-to-block-on-its-earned-streak.md` | this SPEC | added |
| `docs/decisions/ADR-500-the-later-day-refund-property-moves-to-block-only-when-the-checker-admits-its-streak.md` | its decision | added |
| `docs/schematics/formal-check-settings.md` | the ramp's step, appended after the last line | changed, insert-only |
| `formal/tla/WalletFloor/WalletFloor.tla` | line 17, `ramp=report` to `ramp=block` | changed, one line |
| `scripts/tests/test_wallet_refund_ramp.py` | A1 and A2, and the reader they share | added |
| `scripts/mutation-rows.d/S48300-S48399.json` | rows S48301 to S48303 | added |
| `docs/red-first/SPEC-483.md` | the red-first record and the checker's readings F1 to F4 | added |
| `changelog.d/wallet-refund-ramp-483.md` | the changelog fragment | added |

## 5. What this does NOT cover

- A CI job that checks formal entries, and so reads a ramp in CI: #806. Until it lands, `block` is
  read by the formal check at a delivery's head and at its landing.
- The wallet's code and its port tests, including the later-day refund test and its three cases: they
  stay as SPEC-082 delivered them for #106.

## 6. Risks

- The streak restarts between the head and the landing: a landing that moves a covered span or
  re-stamps the entry. F1, read again at landing, then reads premature, and the landing waits until
  the streak is earned again.
- A check that judges the formal entries against the release branch, where the entry is absent,
  would read the flip premature. The delivery reads the configuration of every run that judges the
  entries before its push, and stops if one judges them against the release branch.
- After the flip, a delivery that moves a covered span must re-read and re-stamp the entry in the same
  pull request, or its landing's formal check fails on a stale cover of a blocking property. That
  check detects it.
- The pin passes while the checker reads the ramp differently: the pin reads text only, and F2 is the
  reading of record.
- A row's mutant left installed: each proof restores its file and compares its sha256 before the next
  step, and the record quotes both hashes.

## 7. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S48301-THE-REFUND-PROPERTY-BLOCKS` | `formal/tla/WalletFloor/WalletFloor.tla` | the refund property's line reads `ramp=block` | A1 |
| `S48302-A-DOUBLED-RAMP-LINE-IS-REFUSED` | `scripts/tests/test_wallet_refund_ramp.py` | exactly one line declares the property | A2 |
| `S48303-A-REPORT-RAMP-IS-REFUSED` | `scripts/tests/test_wallet_refund_ramp.py` | a line that does not read `ramp=block` is refused | A2 |

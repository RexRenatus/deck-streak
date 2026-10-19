# The formal check's settings: from the settings file to the witness kill

- **Kind:** data flow.
- **Read at:** the development branch's commit `32f6217`; every `path:line` below is at that commit.
- **Decided by:** ADR-418 (SPEC-404, issue #703). It amends nothing; SPEC-295 and ADR-295 keep their
  own sections and gain the insert-only notes SPEC-404 R3 names.

## The flow

The settings file is the one place the slot setting is stated. Two readers take it: the formal
checker, outside this repository, which turns it into the model checker's slot and runs the model
and its witnesses; and this repository's settings test, which holds the file to the reading the
checker's reader takes. Solid edges are the agree path; dotted edges are the ways the two readings
can part, each ending where it is observed.

```mermaid
flowchart TD
    FILE["config/formal.json<br/>tlc_slot: capacity, wait_seconds"]

    subgraph TESTS["this repository's settings test (hygiene job, python stage)"]
        TREAD["read(doc)<br/>the declared fields, each a positive integer"]
        TSLOT["slot_reading(doc)<br/>both keys named, each within the reader's range"]
        A1["A1: the reading is the file's own two values"]
        A2["A2: each bound admitted, the next integer refused"]
        TRED["the test fails, naming the key"]
    end

    subgraph CHECKER["the formal checker (outside this repository)"]
        CREAD["the settings reader<br/>takes tlc_slot from the file"]
        CDEF["the compiled default<br/>read only for a key the file omits"]
        CREF["the file refused whole<br/>not an object, not a positive integer, past the range"]
        CCMP["a comparing checker<br/>file capacity against a compiled one"]
        SLOT["a turn at the model checker's slot<br/>waits up to wait_seconds"]
        MC["MCEveryLegCounted.cfg<br/>PassMeansEveryLegCounted, PassMeansThePartition"]
        W1["witness/PassMeansEveryLegCounted.cfg<br/>CountReported = TRUE"]
        W2["witness/PassMeansThePartition.cfg<br/>TrustThePlan = TRUE"]
        CARD["the check's card<br/>each property clean, each witness caught"]
        VOIDC["FORMAL VOID CONFIG<br/>no property checked"]
    end

    REC["docs/red-first/SPEC-404.md<br/>the local reading at the committed head"]

    FILE --> TREAD --> TSLOT
    TSLOT --> A1
    TSLOT --> A2
    TSLOT -.->|a key omitted, or a value past the range| TRED

    FILE --> CREAD
    CREAD -->|both keys named, in range| SLOT
    CREAD -.->|a key omitted| CDEF
    CDEF -.->|the reading is not the file's| SLOT
    CREAD -.->|a value past the range| CREF
    FILE -.->|read by a checker that compares| CCMP
    CCMP -.->|the values differ| VOIDC
    CCMP -->|the values are equal| SLOT
    SLOT --> MC
    SLOT --> W1
    SLOT --> W2
    MC -->|each invariant holds| CARD
    W1 -->|violated, so caught| CARD
    W2 -->|violated, so caught| CARD
    CARD --> REC
    VOIDC -.->|a stop, never a reading| REC
```

## Agree and disagree

| case | the settings test | the formal checker | observed by |
|---|---|---|---|
| both keys named, each a positive integer within the reader's range | admits it; the slot reading is the file's own two values | reads the file's own two values, takes a turn, runs the model and both witnesses | A1 and A2 green; the local check's card |
| `tlc_slot` or one of its keys omitted | refuses it by the slot rule, naming what the file does not name | reads its compiled default for the omitted key, so the reading is no longer the file's | A1, before any check runs |
| a value past the reader's range (a capacity above 4294967295, a wait above 18446744073709551615) | refuses it by the slot rule's range arm, naming the key | refuses the file whole; no model runs | A2, before any check runs |
| a value that is not a positive integer | refuses it by `read`, as before this delivery | refuses the file whole | SPEC-295's planted faults |
| a checker that compares the file's capacity with a compiled value that differs | admits the file | reads `FORMAL VOID CONFIG`; no property is checked | only the local check (SPEC-404 section 4), which stops on it |

## Where each step lives

| step | where, at `32f6217` |
|---|---|
| the setting | `config/formal.json:26-29` |
| the test's expected table, field table and integer rule | `scripts/tests/test_formal_config.py:45`, `:58-59`, `:125-126` |
| the test's reader | `scripts/tests/test_formal_config.py:150-189` (`read`) |
| the slot rule | `scripts/tests/test_formal_config.py`, `slot_reading`, added by SPEC-404 |
| the job that runs the test | `.github/workflows/ci.yml:298` (`hygiene`), its step at `:351`; `scripts/check.sh:178-195` (the `python` stage, discovery at `:186`) |
| the model and its configuration | `formal/tla/EveryLegCounted/EveryLegCounted.tla`, `formal/tla/EveryLegCounted/MCEveryLegCounted.cfg` |
| the witnesses | `formal/tla/EveryLegCounted/witness/PassMeansEveryLegCounted.cfg`, `formal/tla/EveryLegCounted/witness/PassMeansThePartition.cfg` |
| the entry's register lines | `formal/tla/EveryLegCounted/EveryLegCounted.tla:68-77` |
| the row on the setting | `S29530` in `scripts/mutation-rows.d/S29500-S29599.json` |
| the rows on the slot rule | `scripts/mutation-rows.d/S40400-S40499.json`, added by SPEC-404 |

The formal check is a local check: a search of `.github/workflows/` at `32f6217` for `formal` finds
no line. So the checker's half of this flow is read only by a local check, and its reading is recorded
in `docs/red-first/SPEC-404.md`.

## The ramp: from a property's header line to the run that fails on it

- **Kind:** data flow.
- **Read at:** dev `36c4783a`; every `path:line` in this section is at that commit.
- **Decided by:** ADR-500 (SPEC-483, issue #549). It amends nothing above: the settings flow, its
  agree and disagree table and its step table stand as written.

`config/formal.json`'s `k` (`:2`) is the setting this section follows: the length of the streak of
clean landings a property needs before it may block. The entry is `tla/WalletFloor`, the model of the
coin wallet's ports (#106), and the property is `RefundWritesItsOwnMovement`. Solid edges are the
path a reading takes; dotted edges are what does not read it.

```mermaid
flowchart TD
    HDR["WalletFloor.tla:17<br/>@phx property RefundWritesItsOwnMovement ramp=block"]
    KSET["config/formal.json:2<br/>k, the streak's length"]
    INPUTS["the entry's inputs<br/>its files, the toolchain identity, the covered spans of WalletFloor.tla:2-11"]

    subgraph CHECKER["the formal checker (outside this repository)"]
        REG["the registry<br/>each property line and its ramp"]
        STREAK["the streak<br/>first-parent landings with the inputs unchanged"]
        FLIP["the ramp reading<br/>admitted, or a premature promotion"]
        MC["MCWalletFloor.cfg:15<br/>TypeOK and the five properties, every fix on"]
        WIT["five witness configurations, one defect each<br/>the refund's: RefundKeyedPerDay = FALSE"]
        CARD["the card<br/>each property: its ramp, streak and clean state"]
        REPORTED["at report: printed, the exit unmoved"]
        FAILED["at block: the gate check fails"]
    end

    subgraph RUNS["the runs"]
        HAND["the formal check at the head and at landing, against live dev"]
        NOCI["CI: no formal job"]
    end

    subgraph PIN["the pin, in the Python tests CI runs"]
        A1["A1: one line declares the property, at ramp=block"]
        A2["A2: a fixture tree's report, missing and doubled lines refused"]
    end

    PORT["wallet_ports.rs:215<br/>the later-day refund: K-009, K-021, K-033"]
    OUTSIDE["outside the model<br/>a refund that shares a deposit's key: K-009, K-021"]

    HDR --> REG
    KSET --> STREAK
    INPUTS --> STREAK
    REG --> FLIP
    STREAK --> FLIP
    REG --> MC
    REG --> WIT
    HAND --> MC
    HAND --> WIT
    HAND --> FLIP
    MC -->|each invariant holds| CARD
    WIT -->|violated, so caught| CARD
    FLIP --> CARD
    CARD -.->|a violation at report| REPORTED
    CARD -->|a violation at block| FAILED
    NOCI -.->|reads no ramp| CARD
    HDR --> A1
    A2 -.->|guards the reader A1 uses| A1
    PORT --> OUTSIDE
```

### Where each step of the ramp lives, and what reads it red

| step | where, at `36c4783a` | what reads it red | row |
|---|---|---|---|
| the ramp line | `formal/tla/WalletFloor/WalletFloor.tla:17` | A1, red at the commit that adds the pin, where the line reads `ramp=report` | `S48301` |
| one line per property | the pin's reader, `scripts/tests/test_wallet_refund_ramp.py` | A2's doubled-line plant | `S48302` |
| a line that is not `block` | the pin's reader | A2's `report` plant | `S48303` |
| the streak and the flip | the checker's ramp reading, with `config/formal.json:2` | F1 of SPEC-483: any reading but admitted stops the delivery | none: a checker reading |
| the invariants | `formal/tla/WalletFloor/MCWalletFloor.cfg:15` | F2: the head's gate check | none: unchanged |
| the refund witness | `formal/tla/WalletFloor/witness/a-refund-guarded-once-ever-over-every-day.cfg:6`, `:12` | F2: caught | none: unchanged |
| the other four witnesses | `witness/a-floor-read-outside-the-write.cfg`, `witness/a-once-ever-guard-read-on-its-own-day.cfg`, `witness/a-ledger-with-no-unique-key.cfg` and `witness/a-closed-day-mint-that-follows-its-base.cfg`, each checking its own property at `:12` | F2: each caught | none: unchanged |
| a violation at block | `MCWalletFloor.cfg:9` in a scratch clone only, never committed | F3: fails at `block`, passes reported at `report` | none: a scratch plant |
| the later-day refund | `crates/economy/tests/wallet_ports.rs:215` | the plant of `docs/red-first/SPEC-082.md:86-95` | none in this band |

The model gives a refund its own source (`WalletFloor.tla:56`, `:80`), so its witness reaches a refund
followed by a refund (K-033) and never a refund that shares a deposit's key (K-009, K-021): those two
cases are held by the port test alone. No CI job runs the formal checker (#806), so the checker's half of
this flow is read by hand at a delivery's head and at its landing, and recorded in
`docs/red-first/SPEC-483.md`; the pin's half is read in CI.

# Red-first record: SPEC-483

SPEC-483 (R1 to R7, A1 and A2, F1 to F4). The SPEC, ADR-500 and the schematic section were
committed first, then the pin, then the flip. The pin was committed at 36dc73dc while the
refund property's header line still read `ramp=report`, so A1 failed by its assertion on the
reader's verdict. The flip was committed at 6b5f0018, the entry's one changed line, and A1 and
A2 read green there.

```text
A1: python3 scripts/tests/test_wallet_refund_ramp.py WalletRefundRamp.test_the_refund_property_is_declared_once_at_block
A2: python3 scripts/tests/test_wallet_refund_ramp.py WalletRefundRamp.test_the_reader_refuses_each_planted_ramp
```

```red-first
A1: red at 36dc73dc: AssertionError: 'refused: RefundWritesItsOwnMovement reads ramp=report' != 'ok: RefundWritesItsOwnMovement reads ramp=block'
A1: green at 6b5f0018
A2: not red: the reader's planted controls read a fixture tree, so they pass when written; its reds are rows S48302 and S48303, each killed by A2
```

Each run was over a fixture tree archived from the commit, holding only the support module, the
pin and the entry. At 36dc73dc the run printed `Ran 2 tests` and `FAILED (failures=1)`, and A2
printed `examined 4 plants`. At 6b5f0018 it printed `Ran 2 tests` and `OK`, and A2 printed
`examined 4 plants` again. The pin, its support module and the entry are byte-identical between
6b5f0018 and the head that holds this record (`git diff --quiet` over the three paths read rc 0),
so the green at the flip is the green CI runs.

## The local formal reading (SPEC-483 section 3a)

Read at the head, with the paired checker and the development branch's commit at the cut as the
base, one check at a time, after the slot verb read `FORMAL SLOT OK capacity=4 taken=0`.

Before the flip, at the cut, the ramp reading of `RefundWritesItsOwnMovement` printed
`"flip": "unmoved"`, `"eligible": true`, `"streak": 20` and `"k": 20` with no finding, and the
entry's card showed all five properties at ramp `report`, clean `true`, streak 20.

F1, the ramp reading of the flip against the cut's base, printed `"base_ramp": "report"`,
`"eligible": true`, `"flip": "admitted"`, `"k": 20`, `"ramp": "block"`, `"streak": 20`, no
finding, and `FORMAL OK exit=0`.

F2, the entry's gate check at the head, printed:

```text
FORMAL EXAMINED entries=1 properties=5 witnesses=5 configs=1 landings=20 rulings=0 deferred=0
FORMAL TOOLCHAIN compiled identity=a2518571360483e12161e99b64695e6c3e8845230129ce0ad179b5bddf9a8dc4
FORMAL SURFACES deny=0 contexts=0 rows=2689 land=0 retired=0
FORMAL OK exit=0
```

Its card read `RefundWritesItsOwnMovement` at ramp `block`, clean `true`, flip `admitted`, streak 20,
and the other four properties at ramp `report`, clean `true`, flip `unmoved`, streak 20. It had no
`WITNESS_SURVIVED`, `STALE`, `FLOOR`, `TIMEOUT`, `MODEL_ERROR` or `PREMATURE_PROMOTION` finding. The
ratchet-only check at the head printed:

```text
FORMAL EXAMINED entries=42 properties=134 witnesses=170 configs=34 landings=20 rulings=0 deferred=0
FORMAL SURFACES deny=0 contexts=0 rows=2689 land=0 retired=0
FORMAL OK exit=0
```

F3, the block arm, in a scratch clone detached at the flip, with `RefundKeyedPerDay = FALSE`
committed in `MCWalletFloor.cfg` and the flip as the base, exited 1. It printed
`FORMAL FAIL VIOLATION tla/WalletFloor/RefundWritesItsOwnMovement — MCWalletFloor.cfg violates
["RefundWritesItsOwnMovement"]` and `FORMAL FAIL exit=1`. Its card row for the property read ramp
`block`, clean `false`. The plant moved the state count, so the four `report` siblings each printed
`FORMAL REPORT FLOOR … MCWalletFloor.cfg explored 177444 distinct states, under its floor of 180580`,
which is expected and was acted on in neither arm.

F3, the report arm, in a scratch clone detached at the cut's base with the same plant and that base
as the base, exited 0. It printed `FORMAL REPORT VIOLATION tla/WalletFloor/RefundWritesItsOwnMovement
— MCWalletFloor.cfg violates ["RefundWritesItsOwnMovement"]` and `FORMAL OK exit=0`. Its card row
for the property read ramp `report`, clean `false`. Neither clone was pushed.

F4, `git diff --numstat` against the cut's base over `formal/tla/WalletFloor/` read
one added and one removed line of `formal/tla/WalletFloor/WalletFloor.tla` and nothing else, and the diff names no path under `crates/`
and not `config/formal.json`.

## The rows, each proved by one installed mutant

Each mutant was installed by a one-off script into a fixture tree holding the support module, the
pin and the entry, taken from the work tree; the killer was run by name, and the file was restored
and its sha256 compared.

| row | killer's failing assertion | sha256 before and after | sha256 of the mutant |
|---|---|---|---|
| `S48301-THE-REFUND-PROPERTY-BLOCKS` | A1: `'refused: RefundWritesItsOwnMovement reads ramp=report' != 'ok: RefundWritesItsOwnMovement reads ramp=block'` | `3d79040749f8c0c988f5628475e94797cfe6a237035f0efd878ea2a5224e7758` both | `6e95c808fe8e2a488a5d0c3b3a51248687c2dc102d91552fe6182ed6fbc2a862` |
| `S48302-A-DOUBLED-RAMP-LINE-IS-REFUSED` | A2, subtest `twice`: `'ok: RefundWritesItsOwnMovement reads ramp=block' != 'refused: 2 lines declare RefundWritesItsOwnMovement'` | `aee832c22b9165902843540f9466514dc9fdf00af7573f80942bf7cba82f2db5` both | `428033277e8fde1d4ea8a86e57c8a4a07978178858cb07d324ab7bf36b6d2645` |
| `S48303-A-REPORT-RAMP-IS-REFUSED` | A2, subtest `report`: `'ok: RefundWritesItsOwnMovement reads ramp=report' != 'refused: RefundWritesItsOwnMovement reads ramp=report'` | `aee832c22b9165902843540f9466514dc9fdf00af7573f80942bf7cba82f2db5` both | `c2bdc5cc09c05e150e31ebe58adab4b75ac172e9751c394a1983b76b0072a5ea` |

No mutation tool ran. CI's rows job reads each row by its stem.

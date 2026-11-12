# Red-first record: SPEC-404

SPEC-404 (R1 to R5, A1 to A3). The SPEC, ADR-418, the schematic, the two sections of SPEC-295 and
ADR-295, and the two tests with a stub of the slot rule were committed first
(9ef34e7040b33fc9055087ac23fef1707f59e1cd). The stub keeps every input and refuses nothing, so each
test failed by its own assertion: a document that omits a slot key, or holds a value past its
range, was admitted. The rule was committed next (28027c3b6c1a36e2fb07748346b4c8b556a3c51f), and
both tests read green. Each red below is quoted from the local run at its commit.

```red-first
A1: red at 9ef34e7040b33fc9055087ac23fef1707f59e1cd: AssertionError: Refused not raised : tlc_slot omitted was read as stated
A1: green at 28027c3b6c1a36e2fb07748346b4c8b556a3c51f
A2: red at 9ef34e7040b33fc9055087ac23fef1707f59e1cd: AssertionError: Refused not raised : capacity = 4294967296 was read
A2: green at 28027c3b6c1a36e2fb07748346b4c8b556a3c51f
A3: not red: SPEC-295's presence harness, green at the base; it holds the two new tests to its rule and gains one loader
```

The green run of A1 prints `examined 3 of 3 slot plants`, and the green run of A2 prints
`examined 4 of 4 slot values`. A3 prints `examined 6 tests that load the committed file`, one more
than at the base, because A1 loads the committed file.

One edit to a test followed its green commit. The message A1 prints when the committed file is
refused was reworded in the record commit, because its first wording repeated the exact line that
the row `S29513` anchors on, and that row's anchor must occur once. The assertion and the control
flow are unchanged, and A1 and A2 read green at the later head as at the green commit.

## The local formal reading (SPEC-404 section 4)

Read at the head that holds the rows, with the paired checker and the development branch's commit
at the cut as the base, one check at a time. The settings file's `tlc_slot` is unchanged, and the
checker's slot verb reads the slot directory as `FORMAL SLOT OK`.

The entry check `formal check --entry tla/EveryLegCounted` printed:

```text
FORMAL EXAMINED entries=1 properties=2 witnesses=2 configs=1 landings=20 rulings=0 deferred=0
FORMAL TOOLCHAIN compiled identity=a2518571360483e12161e99b64695e6c3e8845230129ce0ad179b5bddf9a8dc4
FORMAL SURFACES deny=0 contexts=0 rows=2390 land=0 retired=0
FORMAL OK exit=0
```

Its report read `"verdict": "OK"` and `"findings": []`, and each property row read `clean: true`
with `ramp: report` and `flip: unmoved`:

| property | clean | ramp |
|---|---|---|
| `PassMeansEveryLegCounted` | true | report |
| `PassMeansThePartition` | true | report |

Both witnesses were examined, and the output holds no `WITNESS_SURVIVED`, `VOID`, `TIMEOUT`,
`MODEL_ERROR` or `STALE` line.

The ratchet-only check at the same head printed:

```text
FORMAL EXAMINED entries=36 properties=122 witnesses=157 configs=32 landings=20 rulings=0 deferred=0
FORMAL SURFACES deny=0 contexts=0 rows=2390 land=0 retired=0
FORMAL OK exit=0
```

Its report read `"verdict": "OK"`, with no weakening.

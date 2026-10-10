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

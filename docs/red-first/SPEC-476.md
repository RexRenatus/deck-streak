# Red-first record: SPEC-476

SPEC-476 (R1 to R7, A1 to A6). The SPEC, ADR-493 and the runner schematic's new section were
committed first (50df4cd6), then the six tests and the harness change alone (eec7bce7), against
the runner as it stands at dev, then the two cures (fb18054e) and the rows (fedd4a2b). Every test
runs the real runner over the fixture tree, with `TMPDIR` and the working directory inside the
test's own temporary directory. Each line below runs from `scripts/tests`, and each read
`Ran 1 test`.

```text
A1: python3 -m unittest test_box_packs.TheRunnerCleansUpItsCards.test_a_decoy_sibling_and_the_parent_survive_every_end
A2: python3 -m unittest test_box_packs.TheRunnerCleansUpItsCards.test_an_empty_out_is_read_as_unset
A3: python3 -m unittest test_box_packs.TheRunnerCleansUpItsCards.test_a_keep_of_0_keeps_the_made_directory
A4: python3 -m unittest test_box_packs.TheRunnerCleansUpItsCards.test_an_empty_keep_removes_the_made_directory
A5: python3 -m unittest test_box_packs.TheRunnerSourceHoldsItsCures.test_every_runner_line_fits_100_columns
A6: python3 -m unittest test_box_packs.TheRunnerSourceHoldsItsCures.test_the_cards_directory_is_made_just_before_its_try
```

```red-first
A1: not red: pins the base's removal of exactly the made directory; its reds are hand rows S47601 and S47602, killed by A1
A2: not red: pins the base's reading of an empty BOX_PACKS_OUT as unset; its reds are hand rows S47603 and S47604, killed by A2
A3: not red: pins the base's keep for BOX_PACKS_KEEP=0; its red is hand row S47605, killed by A3
A4: not red: pins the base's removal under an empty BOX_PACKS_KEEP; its red is hand row S47606, killed by A4
A5: red at eec7bce70b79fd10532c1ae11c9987a899984280: AssertionError: Lists differ: [(262, 102), (264, 101), (267, 101)] != []
A5: green at fb18054e2f34345a3702f2864cd180fd265a8d7b
A6: red at eec7bce70b79fd10532c1ae11c9987a899984280: AssertionError: 2 != 0 : 2 statement(s) run between the make and the try
A6: green at fb18054e2f34345a3702f2864cd180fd265a8d7b
```

At eec7bce7, against the unchanged runner, A1 to A4 read `OK`, and A5 and A6 each read
`FAILED (failures=1)` by its assertion, with no error. The whole module read `Ran 22 tests` and
`FAILED (failures=2)`, the two being A5 and A6, and `test_box_scan_expected.py` read its 9 tests
`OK`. At fb18054e each of the six lines reads `OK`, and both modules read `OK`, 22 tests and 9.

The gap before the tests. At 50df4cd6, with the test module as it stands at dev, each of S47601 to
S47606 was planted in turn and the whole module run, `Ran 16 tests` each time. S47601 read
`FAILED (errors=3)` with no failure: each error a `FileNotFoundError` in
`test_the_created_cards_directory_is_gone_after_every_end`, one for each end (a pass, a failure
and an error), so the module saw a removal of the parent only by an error. S47602 to S47606 each
read `OK`: no test held them.

The planted reds. Each row was planted by a script that refuses unless the row's find occurs once
in the runner; each planted copy passed `bash -n`, and its heredoc's body parsed with `ast.parse`.
Its killer was then run alone twice, and the runner restored with `cp`, its sha256 compared with
the saved copy's and `git status --porcelain` read empty. Rows S47601 to S47606 were read at
eec7bce7, against the unchanged runner, and again at fedd4a2b, against the cured one; rows S47607
to S47610, whose finds exist only after the cures, at fedd4a2b. Every run read `Ran 1 test` and
the failures below, with no error:

- S47601: `FAILED (failures=3)`, one for each end, `the run removed the parent`.
- S47602: `FAILED (failures=3)`, one for each end, `the run removed a sibling cards directory`.
- S47603: `FAILED (failures=1)`, the cell beside `BOX_PACKS_KEEP=1`,
  `the run wrote into its working directory`.
- S47604: `FAILED (failures=1)`, the cell alone, `the run left the directory it made`.
- S47605: `FAILED (failures=1)`, `0 != 1 : []`: no made directory was kept.
- S47606: `FAILED (failures=1)`, `an empty BOX_PACKS_KEEP kept the directory the run made`.
- S47607: `FAILED (failures=1)`, `1 statement(s) run between the make and the try`.
- S47608, S47609 and S47610: `FAILED (failures=1)` each, `[(263, 102)]`, `[(268, 101)]` and
  `[(273, 101)]`, `a line of the runner runs past 100 columns`.

Every restore compared equal, the runner's sha256 beginning `e31838590c70` at eec7bce7 and
`c3dac8cbd3a9` at fedd4a2b.

The wrap. With the first cure alone in the work tree, the sha256 of the `ast.dump` of the
heredoc's body began `ccbc24e965b6`, as it did at eec7bce7: the three wrapped lines leave the
runner's syntax tree as it was.

The bytes CI runs. At this record's commit,
`git diff --quiet fb18054e HEAD -- scripts/box-packs.sh scripts/tests/test_box_packs.py scripts/tests/_support.py`
exits 0, so the green shas test the bytes CI runs at the head. The rows commit and this record's
commit edit no test module.

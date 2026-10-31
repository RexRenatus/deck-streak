# Red-first record: SPEC-435

SPEC-435 (R1 to R19, A1 to A13). The SPEC, ADR-449, the schematic, the three test files with
their registers (`ADMITTED`, `LISTED` and `ARMS`) and both fixtures were committed first and
alone, over stubs that keep every input and drop the behaviour: each census returns an empty
result without listing or reading any file, so it finds no site and refuses no empty population,
and each register check accepts whatever it is given. Each red below is therefore its criterion's
assertion and never a missing name.

A1 to A3, A5 to A7, A9 to A11 and A13 read a fixture or a temporary tree. Their reds were read by
cargo over the red commit, with the three tests that read the real tree left out by name, and
again in the `rust` job over the same commit. A4, A8 and A12 read the real tree, so they run in
CI alone: their reds are quoted from the `rust` job over the red commit, each on its register
assertion.

In CI, the `rust` job over the red commit was a job of 2242 s, and of its stages only `test`
failed: the suite ran 1909 tests, of which 1896 passed, 13 failed and 10 were skipped, and the
13 are exactly the tests of A1 to A13, with no failure in any other target. The `release` job's
run of the same suite failed the same 13 tests and no other. The red lines of A4, A8 and A12
quote the panic location that job printed for each test: each is the line of the test's register
assertion, after its census call and its judge assertion, so each failed on its register and
never on a read error or an empty selection.

Test edits after the red commit, each named here:
- The green commit, 381930a4bf5d8686ab67e11b06973c491ecc7b30, replaces the stubs of `census` and `judge` in
  `additive_migrations.rs`, of `sites_of`, `census` and `judge` in `bounded_arithmetic.rs`, and of
  `arms_of`, `census` and `judge` in `allow_list_arms.rs` with the censuses of R1 to R17, and
  removes each stub's lint expectation with it. It changes no test, no assertion, no register row
  and no fixture.
- The rows commit, 7882be8ec5dcd5456f3d1ec4a14210118e8fe9aa, and this record's commit edit no test file.

No CI run reads the green commit or the rows commit alone: every green below is read at the
second push's head, which carries the green commit, the rows and this record.

```red-first
A1: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: assertion `left == right` failed: each planted class is found in its own migration, and nothing in the additive control (left: [])
A1: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
A2: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: assertion `left == right` failed: only the statement split over lines 5 to 7 is a hit, at its first line (left: [])
A2: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
A3: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: assertion `left == right` failed: a hit no row lists (left: None, right: Some("unlisted: migrations/000001_rebuild.sql:3 renames"))
A3: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
A4: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: panicked at crates/kernel/tests/additive_migrations.rs:389:5, on the register assertion "the hits equal ADMITTED"
A4: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
A5: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: assertion `left == right` failed: an empty migrations/ is refused (left: None)
A5: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
A6: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: assertion `left == right` failed: each planted site by its innermost function, and none in `nothing` or the test module (left: [])
A6: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
A7: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: assertion `left == right` failed: a function no row lists (left: None, right: Some("unlisted: crates/planted/src/lib.rs spend 1"))
A7: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
A8: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: panicked at crates/economy/tests/bounded_arithmetic.rs:541:5, on the register assertion "the sites equal LISTED, function by function"
A8: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
A9: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: assertion `left == right` failed: six crates with no source are refused (left: None)
A9: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
A10: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: assertion `left == right` failed: each planted site by its line, and none in the use line, a comment, a literal or env! (left: [])
A10: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
A11: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: assertion `left == right` failed: a site no row lists (left: None, right: Some("unlisted: crates/planted/src/lib.rs:9 shell"))
A11: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
A12: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: panicked at crates/kernel/tests/allow_list_arms.rs:285:5, on the register assertion "the sites equal ARMS, file by file and arm by arm"
A12: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
A13: red at 4c5fb3c00836bcf92717cf827420e10bdf462e98: assertion `left == right` failed: a tree with no source is refused (left: None)
A13: green at 381930a4bf5d8686ab67e11b06973c491ecc7b30
```

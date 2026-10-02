# Red-first record: SPEC-302

The SPEC, its schematic and the two pointer edits in SPEC-090 and SPEC-095 were committed first.
The goldens and their registry followed, then the four tests against stubs of `pynum` that compile
and return a wrong value, so each test fails by assertion. The implementation turns them green.

```red-first
A1: red at d3c75b7: assertion `left == right` failed: sum of {"op":"sum","values":[1e+16,1.0,-1e+16]}: got 0e0, CPython 1e0
A1: green at 1fb30c4
A2: red at d3c75b7: assertion `left == right` failed: the percentile 0.9 of [5]: left Some(0), right Some(5)
A2: green at 1fb30c4
A3: red at d3c75b7: assertion `left == right` failed: draw 0 of seed 0: got 0e0, CPython 8.444218515250481e-1
A3: green at 1fb30c4
A4: red at d3c75b7: panicked: lgamma of 1 is a number (the stub returns None)
A4: green at 1fb30c4
```

The red run reads `test result: FAILED. 1 passed; 3 failed` for each of the two test files; the
passing test of each is the one that holds the empty and the undefined cases, which the stubs
happen to return. The green run reads `test result: ok. 4 passed` for each. The two rows of §7
were proved by the mutation-row verb after the green commit, each killed by its test.

Mutation coverage: the tests of `crates/kernel/tests/pynum_edges.rs` (CPython rows in
`crates/kernel/tests/fixtures/pynum_edges.txt`) were added after the GREEN commit to kill the
mutants the goldens missed; they pass at the implementation and are not a red-first claim. Round-boundary rows at ndigits -308 for x of 4.9e307 to 9e307 (the `ndigits < -308` guard of `round`) were added the same way, to kill the `<` to `<=` mutant. Rows for `round` of non-finite values, zeros and digit counts past 323 were added to kill the `||` to `&&` mutants of its first guard.

Round 1 fix: the verifier's round 1 found four functions that differ from CPython; each code fix
has a test that was red first, recorded here in prose because the criteria above already hold their
fence lines.

```text
mean: red at 97294d0be03e84ec57a4e5f970752853232aed54 in pynum_edges.rs
  (the_mean_of_every_edge_list_is_cpythons_to_the_last_bit): assertion `left == right` failed: mean
  of 7fe1ccf385ebc8a0,7fe1ccf385ebc8a0,fff0000000000000: left: 18444492273895866368 right:
  18442240474082181120
mean: green at bcda6848677c245eace8187de08cb7756100e0d6
lgamma: red at 1560f7b7e4cb2ffa801fc07c16b98e7cfa1283a5 in pynum_edges.rs
  (lgamma_of_a_finite_argument_whose_result_overflows_is_none): assertion `left == right` failed:
  lgamma of 7f57b236a943b4a5: left: Some(inf) right: None
lgamma: green at ca0c5a683ba76550d4102fa40367a395c43c6e6c
median: red at 3e92e50f707ad7f1cd59ad86e678adc8356ded2d in pynum_edges.rs
  (the_median_of_a_list_holding_a_nan_is_nan): median of [NaN, 1.0, 2.0] is 1
median: green at 16e4cf75b776aa1d1d1ee89cb637850ec5ac1ade
round: not red: a_round_beyond_the_largest_float_is_a_signed_infinity
  (e5073d0976354a7101251b2fcf9224dd735be740) pins a section 5 limit the code already had
```

Disclosure: two of these tests changed after the commits named above. The lgamma test gained a
positive assertion at 220308b25301b98aade66860183f422de76ad69a, after its loop: the argument
2.5e305 (`7f56c8e5ca239029`) reads CPython 3.12.12's `7fef3fc83052cbf5`. Its new body, run at its
red commit 1560f7b7e4cb2ffa801fc07c16b98e7cfa1283a5, fails at the same assertion:

```text
panicked at crates/kernel/tests/pynum_edges.rs:98:9:
assertion `left == right` failed: lgamma of 7f57b236a943b4a5
  left: Some(inf)
 right: None
```

The round pin test compares bit patterns since eb783283fdd68ba0854084977ddb971b7f6b70e3, where it
compared the floats; its six cases and their values are unchanged.

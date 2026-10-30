# Red-first record: SPEC-513

The SPEC, ADR-531 and the schematic were installed first (d46882bc). The tests came next, whole, over a stub that keeps
every shape but the behaviour (1d3170e4): the stub's hash is all zeros and reads nothing back, every name and every
bind and release is refused, the grammars name nothing, and the sweep keeps every file as live and lists nothing
missing. The Lean entries `MediaBind` and `MediaGc` and their vectors, written by their ports, came with the tests, and
so did the Kani harnesses under `cfg(kani)`. 9753a450 turns them green: the hash, the names, the bind machine, the
grammar, the CSS reading, the sweep and the index port, the ledger's rows and the context map's. 7a95f80b stamps the
entries' covers and rewrites their vectors' headers. Each red below is quoted from a run of the media crate's nine test
targets on the tree of 1d3170e4, each green from a run on the tree of 7a95f80b.

```red-first
A1: red at 1d3170e4: crates/media/tests/hash.rs:35: assertion `left == right` failed: the SHA-256 of "abc" (media_hash_of_matches_the_nist_sha256_vectors); hash.rs:54 (a_hash_reads_back_from_its_hex_form_and_its_blob_path)
A1: green at 7a95f80b
A2: red at 1d3170e4: crates/media/tests/name.rs:17: a certified name reads: NotPlain (a_certified_name_round_trips); name.rs:39 (a_mismatched_certified_name_is_refused); name.rs:62 (a_name_that_looks_certified_but_is_not_is_refused); name.rs:89 (an_imported_name_is_kept_unchanged)
A2: green at 7a95f80b
A3: red at 1d3170e4: crates/media/tests/bind.rs:46: assertion `left == right` failed, left: Err(Conflict), right: Ok(New) (bind_is_write_once); bind.rs:61, :76, :97 and :143 (the other four); crates/media/tests/formal_vectors_bind.rs:80: the first vector's slot after is None, the port's is live (step_and_release_step_answer_every_lean_vector)
A3: green at 7a95f80b
A4: red at 1d3170e4: crates/media/tests/scan_golden.rs:74: an uncited case differs: case 0 (scan_equals_the_engines_field_refs_golden_but_for_the_cited_divergences); scan_golden.rs:160 (scan_reads_the_engines_grammar_over_hand_cases)
A4: green at 7a95f80b
A5: red at 1d3170e4: crates/media/tests/css.rs:101: assertion `left == right` failed: a_font_the_css_names_is_inlined_for_a_reader_that_asks: every name (scan_css_keeps_every_font_case_of_the_face_suite); css.rs:151 (a_url_that_names_no_plain_file_is_no_name)
A5: green at 7a95f80b
A6: red at 1d3170e4: crates/media/tests/sweep.rs:48, :79, :93 and :138 (the four sweep tests); crates/media/tests/check_golden.rs:83: case 0: the sweep deletes the engine's unused files, left: [], right: ["ds_present.png"]; crates/media/tests/formal_vectors_gc.rs:59: the first vector's fate is live, the port's is delete
A6: green at 7a95f80b
```

## What each pair disclosed

- **The tests are byte-identical at 1d3170e4 and 7a95f80b.** No file under `crates/media/tests/` changes after the
  stub commit (`git diff --stat 1d3170e4 7a95f80b -- crates/media/tests` prints nothing).
- **Twenty-two tests are RED-FIRST:** each failed by name at 1d3170e4 (9 test binaries, 5 passed, 22 failed) and passed
  at 7a95f80b (9 binaries, 27 passed, 0 failed). The five that passed at the stub carry no criterion alone: the two
  golden controls (`a_golden_answer_changed_by_hand_is_caught`,
  `a_pinned_file_is_never_unused_and_a_planted_unused_answer_is_caught`), the face suite's census
  (`the_face_suite_still_holds_its_tests_and_every_font_case_copied_here`), and two refusal tests the stub's blanket
  refusal satisfies (`a_malformed_hex_form_or_blob_path_is_refused`, `a_name_that_is_not_plain_is_refused`).
- **A4's golden:** at 7a95f80b the grammar equals G-L5-1 in 88 of its 96 cases; the 8 others are the cases ADR-531 D4
  cites (24, and 64 to 68, 70 and 71), and each differs only by the names the engine derives.
- **A6's golden:** at 7a95f80b the sweep's deletions equal G-L5-2's unused names in all 64 cases, and the missing names
  equal it in all but the 8 `latex-notes` cases, which differ by 16 rendered names.
- **The Lean entries are green at 7a95f80b** under the formal checker's gate (`--kind lean --gate`, exit 0, 24
  entries, 69 properties, all 69 clean, 72 witnesses examined): the six `MediaBind` and two `MediaGc` theorems read
  clean, each refuted by its witness, and the four covers read FRESH.
- **Each Kani witness is killed by the plain tests too.** With each witness feature on, the crate's own tests fail:
  `kani-witness-bind` fails four tests of A3, `kani-witness-sweep` three of A6, `kani-witness-scan` both grammar
  tests of A4 and `kani-witness-blob-path` two hash tests of A1. No Kani run is pinned for this tree, so the harnesses
  are VOID.

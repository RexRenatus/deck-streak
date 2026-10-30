# SPEC-513: A media file is known by the SHA-256 of its bytes, and one grammar finds every media name a note writes

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | none filed; the pull request names this SPEC |
| decided by | ADR-531, under ADR-024 and ADR-029 |
| measured at | the delivery's base `554a223e` (every file this SPEC cites is byte-equal there), and the pinned engine `a078820f` |
| schematic | `docs/schematics/media-names-hashes-and-the-sweep.md` (new: component and binding state machine); `docs/schematics/context-map.md` (changed: the media crate's node) |
| mutation band | S51300-S51399 (no hand row: section 9) |

## 1. The problem, measured

DeckStreak has no media core of its own. What was read at `554a223e`:

- **A file has no identity but its name.** The engine core reads a face's media by name through its `Reader`
  (`crates/engine-core/src/media.rs:72-92`), and nothing in the workspace knows a media file by its bytes: no
  crate hashes a media file, so two files of one name, or one file under two names, cannot be told apart.
- **Two grammars read media names, neither DeckStreak's.** A note's text is read by the engine's own
  `replace_media_refs` (`crates/engine-core/src/media.rs:170-173`), whose expressions are the pinned engine's
  (`rslib/src/text.rs`: the tag expression, the sound tag expression and the remote check); a note type's CSS is read
  by the engine core's font pass (`crates/engine-core/src/media.rs:210-257`), which knows fonts alone. Nothing reads a
  note's text without the engine.
- **The engine's reading has goldens and no reader.** `tools/parity-oracle/goldens/anki.media.field_refs.json`
  (G-L5-1, SPEC-498) holds 96 cases, eight per class over nine classes (`quotes`, `case`, `percent-escape`,
  `url-schemes`, `sound-tag`, `nested-markup`, `broken-markup`, `underscore-names`, `latex`) and 24 seeded random
  cases, each a field's HTML and the names the engine's media check reads in it, each once, in byte order.
  `tools/parity-oracle/goldens/anki.media.check.json` (G-L5-2) holds 64 cases, eight per class over five classes
  (`underscore-files`, `latex-notes`, `missing-files`, `unused-files`, `css-fonts`) and 24 random ones, each a
  collection with its media folder and the engine's unused and missing names.
- **What the engine's check derives.** Measured over G-L5-1: the engine names a LaTeX image `latex-` and the SHA-1 of
  its expression, `.png` (`[latex]amber^2[/latex]` gives `latex-` and the SHA-1 of `amber^2`; `[$]basil[/$]`, of
  `$basil$`), and writes a `data:image/png;base64,` source to a file named `paste-` and the SHA-1 of its bytes; a
  `data:` source it cannot decode it keeps as written (case 27). A file whose name starts with `_` is never unused.
- **The face suite's font cases.** `crates/engine-core/tests/face.rs` holds 18 `#[test]`; five of them give a note
  type a CSS that names fonts (`a_font_the_css_names_is_inlined_for_a_reader_that_asks`,
  `a_refused_font_is_emptied_and_named_and_any_other_url_stays`,
  `the_css_stays_as_written_for_a_reader_that_does_not_ask`, `a_font_never_crowds_out_the_cards_own_media`,
  `every_font_type_is_inlined_under_its_own_type`).
- **No sweep exists.** Nothing decides which stored media file may be deleted.

## 2. Requirements

R1. **A file is known by its hash.** `crates/media`, package `deck-streak-media`, holds `MediaHash`, the SHA-256 of a
file's bytes (`MediaHash::of`), its lowercase hex form, and its blob path: the hex form's first two digits, `/`, and
the whole hex form, which reads back to its hash alone. SHA-256 comes from the workspace's `sha2` (ADR-024).

R2. **Certified names, and imported names unchanged.** `MediaName` is a certified name, `ds-` and the 64 lowercase hex
digits of its file's hash with an optional `.` and an extension of 1 to 10 lowercase ASCII letters or digits, or an
imported name, any other plain file name, kept as it was written and claiming no hash. A certified name is written
in one form, so it reads back to the same name; a file whose bytes hash to anything else is refused under it. A name
that starts like a certified one, `ds-` and 64 hex digits in any case, but is not one is refused.

R3. **Write-once bindings.** `step` binds a name's slot: an unbound name is bound live to the file's hash; a bound
name is admitted again only for its own hash (unchanged when live, revived when released); any other bind is a
conflict; a certified name's claim of another hash is a mismatch; a refusal leaves the slot as it was. `release_step`
releases a live binding and keeps its hash. `Bindings` holds a slot per name.

R4. **One grammar for a note's text.** `scan` gives every media name a note's text writes, by the pinned engine's
reading: a `src` or `data` attribute of an `img`, `audio`, `video`, `object` or `source` tag, tag and attribute names
in any case, the attribute where a word starts, quoted or unquoted, with a quoted `>` admitted in the tag's run of
attributes; and a `[sound:…]` tag, in lower case, up to its first `]`. A remote reference (`http://` or `https://`,
in any case) is dropped, a name's HTML entities are decoded as the engine decodes them, and the names come in the
engine's order: every tag's, then every sound tag's.

R5. **One grammar for a CSS.** `scan_css` gives every name a CSS writes in a `url(`: the argument up to the first
`)`, trimmed, unquoted, `%XX`-decoded and a plain file name, as the engine core's font pass reads it.

R6. **The sweep plan.** `plan_sweep` keeps a file a note, a template or a CSS names (live), a file whose name starts
with `_` (pinned), a file added within the grace window or after the sweep's instant (in grace) and a file an import
brought (imported), each for the first reason that holds, deletes every other file, and lists every name written
that no stored file has (missing). `fate` decides one file; `in_grace` reads the window through `i128`, so no instant
or window overflows.

R7. **The port a collection implements.** `MediaIndex` hands the core every text that names media, as HTML (a note's
field, a card template) or CSS (a note type's), and every stored file; `references` and `plan` read it.

R8. **A pure core.** The crate reads no file, clock, environment, network, process or thread, depends on `sha2`
alone, and has no workspace edge; the context map carries it.

## 3. Acceptance criteria of SPEC-513

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1: `MediaHash::of` gives the NIST SHA-256 examples; a hash reads back from its hex form and its blob path, and every malformed one is refused. K-blob-path: VOID (section 8) | the stub hashes every input to zeros and reads back nothing | ADR-531 D1 |
| A2 | R2: a certified name round-trips, a mismatched one is refused, a near-certified one is refused, an imported name is kept as written | the stub refuses every name | ADR-531 D2 |
| A3 | R3: the bind is write-once, idempotent, refuses a conflict and a mismatch, and revives only for the same hash; Lean `MediaBind` holds the machine and its vectors are byte-equal. K-bind: VOID (section 8) | the stub refuses every bind and release | ADR-531 D3 |
| A4 | R4: `scan` equals G-L5-1 in 88 of its 96 cases, and each of the 8 others is a case ADR-531 D4 cites, differing only by the names the engine derives; hand cases of the grammar. K-scan: VOID (section 8) | the stub names nothing | ADR-531 D4 |
| A5 | R5: `scan_css` keeps every font case of `crates/engine-core/tests/face.rs` (18 `#[test]`): each case's fonts, in order, and no name for a scheme, a `data:` URL or a path | the stub names nothing | ADR-531 D5 |
| A6 | R6, R7: `plan_sweep` keeps live, pinned, in-grace and imported files and deletes the rest; with `scan` it reproduces G-L5-2's unused names in all 64 cases and its missing names in all but the 8 `latex-notes` cases, which differ only by the LaTeX images ADR-531 D4 cites; Lean `MediaGc` holds the decision and its vectors are byte-equal. K-sweep: VOID (section 8) | the stub keeps every file and lists nothing missing | ADR-531 D6 |

```acceptance
A1: cargo test -p deck-streak-media --test hash -- --exact media_hash_of_matches_the_nist_sha256_vectors
A1: cargo test -p deck-streak-media --test hash -- --exact a_hash_reads_back_from_its_hex_form_and_its_blob_path
A1: cargo test -p deck-streak-media --test hash -- --exact a_malformed_hex_form_or_blob_path_is_refused
A2: cargo test -p deck-streak-media --test name -- --exact a_certified_name_round_trips
A2: cargo test -p deck-streak-media --test name -- --exact a_mismatched_certified_name_is_refused
A2: cargo test -p deck-streak-media --test name -- --exact a_name_that_looks_certified_but_is_not_is_refused
A2: cargo test -p deck-streak-media --test name -- --exact an_imported_name_is_kept_unchanged
A2: cargo test -p deck-streak-media --test name -- --exact a_name_that_is_not_plain_is_refused
A3: cargo test -p deck-streak-media --test bind -- --exact bind_is_write_once
A3: cargo test -p deck-streak-media --test bind -- --exact binding_the_same_hash_again_changes_nothing
A3: cargo test -p deck-streak-media --test bind -- --exact a_released_name_comes_back_only_for_its_own_hash
A3: cargo test -p deck-streak-media --test bind -- --exact a_bind_under_a_mismatched_certified_name_is_refused
A3: cargo test -p deck-streak-media --test bind -- --exact the_machine_answers_every_slot_claim_and_hash_by_its_rules
A3: cargo test -p deck-streak-media --test formal_vectors_bind -- --exact step_and_release_step_answer_every_lean_vector
A4: cargo test -p deck-streak-media --test scan_golden -- --exact scan_equals_the_engines_field_refs_golden_but_for_the_cited_divergences
A4: cargo test -p deck-streak-media --test scan_golden -- --exact scan_reads_the_engines_grammar_over_hand_cases
A5: cargo test -p deck-streak-media --test css -- --exact scan_css_keeps_every_font_case_of_the_face_suite
A5: cargo test -p deck-streak-media --test css -- --exact the_face_suite_still_holds_its_tests_and_every_font_case_copied_here
A5: cargo test -p deck-streak-media --test css -- --exact a_url_that_names_no_plain_file_is_no_name
A6: cargo test -p deck-streak-media --test sweep -- --exact plan_sweep_keeps_live_pinned_in_grace_and_imported_files
A6: cargo test -p deck-streak-media --test sweep -- --exact every_other_file_is_deleted_and_every_missing_name_is_listed
A6: cargo test -p deck-streak-media --test sweep -- --exact a_file_is_in_grace_until_the_window_has_passed
A6: cargo test -p deck-streak-media --test sweep -- --exact each_reason_is_asked_in_order_and_none_leaves_the_file_deleted
A6: cargo test -p deck-streak-media --test check_golden -- --exact plan_sweep_and_scan_reproduce_the_engines_media_check_golden_but_for_latex
A6: cargo test -p deck-streak-media --test formal_vectors_gc -- --exact fate_answers_every_lean_vector
```

The Lean halves of A3 and A6 are judged by the formal checker (`--kind lean`): `lean/MediaBind` and `lean/MediaGc`
build with no hole, each witness refutes its theorem, and each vectors file is byte-equal to its writer's output.
The Kani halves are VOID (section 8). The two controls (`a_golden_answer_changed_by_hand_is_caught` in
`scan_golden`, `a_pinned_file_is_never_unused_and_a_planted_unused_answer_is_caught` in `check_golden`) run with
their suites and show the goldens' comparisons can fail.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/media/Cargo.toml` | media | new: `sha2` its one dependency; the golden reader's two dev-dependencies; four witness features, off |
| `crates/media/src/lib.rs` | media | new: the crate's doc, its modules and re-exports |
| `crates/media/src/hash.rs` | media | new: `MediaHash`, its hex form and blob path |
| `crates/media/src/name.rs` | media | new: `MediaName`, `CertifiedName`, `NameRefusal`, `plain_name` |
| `crates/media/src/bind.rs` | media | new: `step`, `release_step`, `Bindings` and their outcomes |
| `crates/media/src/scan.rs` | media | new: `scan`, `is_remote`, `decode_entities` |
| `crates/media/src/css.rs` | media | new: `scan_css`, `decoded_name` |
| `crates/media/src/sweep.rs` | media | new: `fate`, `in_grace`, `pinned`, `plan_sweep` and their types |
| `crates/media/src/index.rs` | media | new: `MediaIndex`, `Source`, `references`, `plan` |
| `crates/media/src/kani_proofs.rs` | media | new: K-blob-path, K-bind, K-sweep and K-scan, under `cfg(kani)` |
| `crates/media/tests/hash.rs` | media tests | new: A1 |
| `crates/media/tests/name.rs` | media tests | new: A2 |
| `crates/media/tests/bind.rs` | media tests | new: A3 |
| `crates/media/tests/formal_vectors_bind.rs` | media tests | new: A3's vectors |
| `crates/media/tests/scan_golden.rs` | media tests | new: A4 |
| `crates/media/tests/css.rs` | media tests | new: A5 |
| `crates/media/tests/sweep.rs` | media tests | new: A6 |
| `crates/media/tests/check_golden.rs` | media tests | new: A6's golden |
| `crates/media/tests/formal_vectors_gc.rs` | media tests | new: A6's vectors |
| `formal/lean/Formal/MediaBind.lean` | formal | new: the bind machine's six theorems and their witnesses |
| `formal/lean/Formal/MediaBindVectors.lean` | formal | new: its vector writer |
| `formal/lean/Formal/MediaGc.lean` | formal | new: the sweep decision's two theorems and their witnesses |
| `formal/lean/Formal/MediaGcVectors.lean` | formal | new: its vector writer |
| `formal/lean/Formal/Vectors.lean` | formal | changed: the two entries' arms |
| `formal/vectors/media-bind.jsonl` | formal | new: written by the checker |
| `formal/vectors/media-gc.jsonl` | formal | new: written by the checker |
| `formal/obligations.json` | formal | changed: O13 lists the two Lean entries present with their vectors ties, and the four harnesses planned with where each sits; the entries' rows |
| `scripts/tests/test_formal_obligations.py` | formal census | changed: the two entries leave the planned list, join the new entries and O13's present names, and their ties join the committed ties |
| `Cargo.toml` | workspace | changed: the crate's workspace dependency entry; `cfg(kani)` declared to the `unexpected_cfgs` lint |
| `Cargo.lock` | workspace | changed: the crate's package |
| `docs/CONTEXT-MAP.md` | docs | changed: the crate's fence row and its prose |
| `docs/LEXICON.md` | docs | changed: the media terms |
| `docs/schematics/context-map.md` | docs | changed: the crate's node in the foundations |
| `docs/schematics/media-names-hashes-and-the-sweep.md` | docs | new: the component and the binding state machine |
| `docs/specs/SPEC-513-a-media-file-is-known-by-the-sha-256-of-its-bytes-and-one-grammar-finds-every-media-name-a-note-writes.md` | docs | new: this file |
| `docs/decisions/ADR-531-a-media-file-is-known-by-its-sha-256-its-names-are-bound-once-and-one-grammar-reads-its-references.md` | docs | new: the decisions |
| `docs/red-first/SPEC-513.md` | docs | new: the red-first record |
| `changelog.d/media-file-known-sha-513.md` | changelog | new: the fragment |

## 5. What this does NOT cover

- The store: no file is written, read, moved or deleted here; a sweep's plan is applied by its store, which is
  another delivery's, and so is the media sync model (`tla/MediaSync`) and its harnesses (K-evictable, K-put).
- The collection's implementation of `MediaIndex`, and the renderer's reading of names through this grammar: the
  engine core keeps its own font pass and the engine's text rewrite until the renderer depends on this crate.
- LaTeX: no image is rendered or named from a LaTeX expression (ADR-531 D4).
- An inline `data:image/…;base64,` image is not written to a file (ADR-531 D4).
- NFC normalization of a name, and the engine's named entities other than `&amp;`, `&lt;`, `&gt;`, `&quot;` and
  `&nbsp;` (ADR-531 D4); no case of either golden holds one.

## 6. Risks

- **A sweep that deletes a file a note still needs.** Every file a note, template or CSS names is live, every `_`
  file is pinned, every imported file stays, and a new file waits out the grace window; the plan lists, it never
  deletes. A LaTeX image the engine rendered is unnamed here, so a store that holds one must pin or import it until
  the renderer names it.
- **The grammar drifts from the engine's.** G-L5-1 holds it case by case; the hand cases hold the shapes the golden
  does not draw (a quoted `>`, the long s, an entity). A re-pin of the engine re-runs both.
- **A word boundary the engine reads otherwise.** The grammar's word characters are letters, numbers and `_`; the
  engine's are Unicode's word class, which also holds combining marks and connector punctuation. A tag name followed
  by a combining mark reads differently; neither golden holds one.
- **Cost.** A tag's run of attributes is searched depth first with one mark per position; a text of many unclosed
  media tags costs time in the square of its length.

## 7. What only CI proves

The whole workspace's clippy and tests (`rust`), the release profile's tests (`release`), and the mutation jobs over
the new files (`mutation-rust`), each read by name at the head. The engine and study web suites are not touched by
this change.

## 8. Formal model

- **Lean: `lean/MediaBind`, present.** Six theorems over `step` and `release_step`: a bound name keeps its hash
  through any bind; an admitted bind leaves the name live at its hash and a refusal leaves the slot as it was; an
  admitted bind repeated changes nothing; a released name is admitted only for its own hash; a claim of another hash
  is refused; a release keeps the hash. Each has a witness, the machine with that rule broken, which refutes it. The
  vectors (`formal/vectors/media-bind.jsonl`, 35 lines) are answered by `formal_vectors_bind`.
- **Lean: `lean/MediaGc`, present.** Two theorems over `fate` and `in_grace`: a live, pinned, young or imported file
  is never deleted, and every other file is. Each has a witness (a sweep blind to the window; a sweep that keeps
  every file). The vectors (`formal/vectors/media-gc.jsonl`, 244 lines) are answered by `formal_vectors_gc`.
- O13 lists both entries present, each tied by its vectors test; its status stays `planned`: `tla/MediaSync`,
  K-evictable and K-put are still planned, and so are the four harnesses below.
- **Kani: K-blob-path, K-bind, K-sweep, K-scan, VOID.** The harnesses and their four witness features are in the
  tree (`crates/media/src/kani_proofs.rs`), and no Kani run is pinned for this tree yet: each is unratcheted and
  nothing runs, and the ledger keeps each planned, naming where it sits. Their owner is the delivery that runs the
  formal checker's Kani kind over this tree.
- **TLA+: NOT APPLICABLE.** Each function here is pure; nothing interleaves.

## 9. Mutation rows

None. The new crate's code is generated by cargo-mutants, and its own tests (A1 to A6) are their killers. The band
S51300-S51399 holds no row.

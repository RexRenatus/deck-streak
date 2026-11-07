# Red-first record: SPEC-601

The SPEC, ADR-621, the schematic and the changelog fragment were installed first (7fa08005). The tests came next,
whole, in the commit that adds this record, over inert seams that compile and give a fixed wrong answer: the blob
store's `get` and `CasReader` read nothing, the open pass deletes nothing, `put_claimed` admits, the handed
constructor admits every pair, `rewrite` returns its input unchanged and `kani_proofs.rs` holds only an inert
`fn k_put() {}`. The migration, the media statements, the data rights entries, the privacy entry and the ledger's
`at` come only with the implementation, so the statement census and the data rights tests stay green beside the
reds and the seams run no SQL. Every red below is an assertion failure, quoted from a run of its test target on the
tree of the commit that adds this record. The red and green lines, each with its commit, are recorded once the
implementation is in.

## The reds of the tests commit

Each line names its criterion, the test file and line, the first assertion line and the test. All 32 named tests read
red.

```text
A1: crates/collection/tests/schema.rs:504: assertion `left == right` failed: the tables, left: ["media_file is missing", "media_file holds [], not [\"hash\", \"size\", \"kind\", \"state\", \"synced\", \"added_ms\", \"last_seen_ms\"]", "media_name is missing", ...] (every_table_of_the_schema_is_strict_with_its_columns, examined 23 tables)
A1: crates/collection/tests/schema.rs:528: assertion `left == right` failed: the checks, left: ["media_file.hash: refused for another reason: no such table: media_file", ...] (each_check_refuses_its_planted_row, examined 74 planted rows)
A1: crates/collection/tests/media_store.rs:82: assertion `left == right` failed, left: 1, right: 2 (a_store_at_head_reads_user_version_two)
A2: crates/collection/tests/media_store.rs:93: a put places its file (a_file_without_its_row_is_deleted_at_the_next_open)
A3: crates/collection/tests/media_store.rs:122: assertion failed: matches!(outcome, Err(MediaError::Blob(BlobError::Mismatch))), got Ok(()) (a_claimed_hash_with_other_bytes_is_refused)
A4: crates/collection/tests/media_store.rs:144: assertion `left == right` failed, left: None, right: Some([116, 104, 101, 32]) (get_returns_at_most_the_limit)
A4: crates/collection/tests/media_store.rs:157: a put places its file (a_size_mismatch_quarantines_the_file_and_reports_it_wanted)
A5: crates/engine-core/tests/face.rs:385: assertion `left == right` failed: Question: the image's src is the data: URL of the file's bytes (a_face_inlines_its_image_and_strips_its_av_tags)
A5: crates/engine-core/tests/face.rs:354: assertion `left == right` failed: the answer replays the question's clip, then its own (the_answer_replays_the_question_unless_the_preset_skips_it)
A5: crates/engine-core/tests/face.rs:437: assertion `left == right` failed: each refused reference's attribute is emptied, each admitted one inlined (a_reference_the_rules_refuse_is_omitted_and_emptied)
A5: crates/engine-core/tests/face.rs:510: assertion `left == right` failed (a_tts_tag_becomes_speech_in_the_platforms_terms)
A5: crates/engine-core/tests/face.rs:545: assertion `left == right` failed: the default preset, wished: the replay is full (autoplay_follows_the_preset_and_the_client)
A5: crates/engine-core/tests/face.rs:733: assertion `left == right` failed: each url( naming the font holds its data: URL, and the rest is as written (a_font_the_css_names_is_inlined_for_a_reader_that_asks)
A5: crates/engine-core/tests/face.rs:812: assertion `left == right` failed: nothing omitted (the_css_stays_as_written_for_a_reader_that_does_not_ask)
A5: crates/engine-core/tests/face.rs:842: assertion `left == right` failed: the three images are admitted (a_font_never_crowds_out_the_cards_own_media)
A5: crates/engine-core/tests/face.rs:913: assertion `left == right` failed: each font is inlined under its own type, and the rest is as written (every_font_type_is_inlined_under_its_own_type)
A5: crates/engine-core/tests/face.rs:1028: assertion `left == right` failed: the question's text ends with sign's element, then wave's (a_sound_tags_video_is_written_into_the_face_for_a_reader_that_asks)
A5: crates/engine-core/tests/face.rs:1059: assertion `left == right` failed: the answer ends with the question's videos, then its own (the_answer_shows_the_question_videos_then_its_own_unless_the_preset_skips)
A5: crates/engine-core/tests/face.rs:1100: assertion `left == right` failed: the question's text ends with small's element alone (a_video_over_the_file_cap_is_omitted_and_named_once)
A5: crates/engine-core/tests/face.rs:1126: assertion `left == right` failed: the question replays sign, then beep, as sounds (a_reader_that_does_not_ask_keeps_a_video_tag_as_a_sound)
A5: crates/collection/tests/media_store.rs:173: assertion `left == right` failed, left: None, right: Some([97, 32, 108, 105, 118, 101, 32, 102, 105, 108, 101]) (an_unbound_or_tombstoned_name_reads_nothing)
A6: crates/ffi/tests/store_media.rs:43: assertion `left == right` failed, left: None (a_put_file_reads_back_by_name)
A6: crates/ffi/tests/store_media.rs:54: assertion `left == right` failed, left: None (an_unbound_name_reads_nothing)
A6: crates/ffi/tests/store_media.rs:66: the put places the file at its hash's path (a_quarantined_name_is_wanted)
A6: crates/ffi/tests/store_media.rs:79: assertion failed: reader.inlines_fonts() (the_reader_inlines_fonts_and_shows_video)
A7: crates/web-engine/tests/handed.rs:14: assertion `left == right` failed, left: None (a_matching_pair_is_admitted_and_read_back_capped)
A7: crates/web-engine/tests/handed.rs:29: assertion `left == right` failed, left: [] (a_pair_whose_bytes_differ_by_one_byte_is_refused_and_listed)
A7: crates/web-engine/tests/handed.rs:41: assertion `left == right` failed, left: None (an_empty_set_admits_nothing)
A8: crates/collection/tests/media_store.rs:213: assertion `left == right` failed, left: None (a_file_lies_at_its_blob_path)
A8: crates/collection/tests/media_store.rs:257: assertion `left == right` failed: k_put, left: None (each_kani_harness_reads_as_it_was_written)
A9: crates/media/tests/rewrite.rs:52: assertion `left == right` failed: the cases whose visited names differ from the engine's, left: [0, 1, 2, ...] (61 cases), right: [] (rewrite_visits_the_engines_reference_names)
A9: crates/engine-core/tests/media_refs_census.rs:57: assertion `left == right` failed: the core's lines naming the engine media rewrite, left: ["src/media.rs:8", "src/media.rs:171"], right: [] (the_engine_core_names_no_engine_media_rewrite)
```

## What each criterion's reds say

- A1: the schema tests name the three tables and their columns and plant one row per new check; with the migration
  absent, each table is missing and each plant meets no table. A store at head reads `user_version` 1.
- A2: the inert `put` places no file, so the crash test fails before its crash.
- A3: the inert `put_claimed` admits bytes that are not the claimed hash's.
- A4: the inert `get` reads nothing, and the quarantine test's put places no file.
- A5: 13 of the 18 face tests read red: each expects a media file's bytes (a `data:` URL or a sound clip's bytes),
  and the inert `CasReader` behind both test readers reads nothing. The five that stay green expect no media file's
  bytes: `a_percent_escape_decodes_to_its_byte`,
  `a_field_the_frontend_completes_keeps_its_own_text_beside_the_front_side`,
  `a_face_carries_its_cards_template_ordinal`, `a_tts_tag_carries_its_voices_in_order` and
  `a_refused_font_is_emptied_and_named_and_any_other_url_stays` (its only `data:` URL is in its own CSS input). The
  media store's own reader test also reads a live name's bytes, which the inert reader misses.
- A6: the iOS reader's four tests read red against the inert seam, the unbound-name case included, because it also
  asserts that a live name reads its bytes.
- A7: the web core's three tests read red against the inert constructor, the empty-set case included, because it also
  asserts that a matching pair is admitted.
- A8: K-blob-path and K-put read VOID: the verifier is not installed (`command -v cargo-kani` exits 1), so neither
  harness ran and the ledger's `K-put` stays `planned`. The layout test reads red because the inert `put` places no
  file, and the text hold reads red because the seam's `k_put` is not the text it holds; both are admitted by name.
- A9: the golden holds 854 cases, 61 of them naming a media reference, and the test pins both counts before it
  compares. The inert `rewrite` visits no name, so those 61 cases differ, and the census finds the engine's rewrite
  named at `src/media.rs` lines 8 and 171.

## What stays green beside the reds

On the same tree: the collection store's `data_rights` (6 tests), `statement_census` (3) and `opener` (13), the
engine core's `review_templates` (1) and `graph` (4), and the adapter's `face` (6). The Python censuses of the context
map register, the context map schematic, the formal obligations, the privacy policy and the TestFlight workflows read
OK.

## The red and green lines

```red-first
A1: red at 817b416c: crates/collection/tests/schema.rs:504: assertion `left == right` failed: the tables
A1: green at 90fb1aec
A2: red at 817b416c: crates/collection/tests/media_store.rs:93: a put places its file
A2: green at 90fb1aec
A3: red at 817b416c: crates/collection/tests/media_store.rs:122: assertion failed: matches!(outcome, Err(MediaError::Blob(BlobError::Mismatch)))
A3: green at 90fb1aec
A4: red at 817b416c: crates/collection/tests/media_store.rs:144: assertion `left == right` failed
A4: green at 90fb1aec
A5: red at 817b416c: crates/engine-core/tests/face.rs:385: assertion `left == right` failed: Question: the image's src is the data: URL of the file's bytes
A5: green at 90fb1aec
A6: red at 817b416c: crates/ffi/tests/store_media.rs:43: assertion `left == right` failed
A6: green at 90fb1aec
A7: red at 817b416c: crates/web-engine/tests/handed.rs:14: assertion `left == right` failed
A7: green at 90fb1aec
A8: red at 817b416c: crates/collection/tests/media_store.rs:213: assertion `left == right` failed
A8: green at 90fb1aec
A9: red at 817b416c: crates/media/tests/rewrite.rs:52: assertion `left == right` failed: the cases whose visited names differ from the engine's
A9: green at 90fb1aec
```

## The greens

Each green line names the first commit whose tree reads its criterion's tests green, measured by `cargo test
--locked --offline` one test target at a time: the collection store's `schema`, `media_store`, `data_rights`,
`statement_census` and `opener`, the engine core's `face`, `media_refs_census`, `review_templates` and `graph`, the
media core's `rewrite`, the adapter's `store_media` and `face`, and the web core's `handed`. A9's rewrite test reads
0 divergences over the golden's 854 cases. A8's two harnesses stay VOID at 90fb1aec, because the verifier is not
installed, so its green line is its two tests, the layout test and the text hold; the ledger's `K-put` gains its `at`
and stays `planned`.

Between the reds and the greens, one test-only commit, 468654c5, keeps each test's predicate: two assertions in the
web core's handed tests take the lint's suggested form and its module comment names its criterion in words, one doc
comment in the media rewrite tests quotes a name in backticks, ten line citations in the stride schematic move down
one line, one mutation row's anchor follows the context map's new header commit, and the kernel's allow-list census
counts the six new source files. So the tests of the seventh and ninth criteria changed once between their red and
their green, each keeping its predicate; no other test file of these criteria changed between a red and its green.

Two side reds at 817b416c are not criteria of this record and carry no green line: the kernel's `allow_list_arms`
test `every_arm_site_on_the_tree_is_listed_with_its_reason`, the source-count pin, cured in 468654c5; and the
progression crate's `xp_census` test `only_progression_writes_xp_settlement_and_only_coordination_settles`, whose
cure a later commit owes.

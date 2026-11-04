# Red-first record: SPEC-602

The SPEC, ADR-622, ADR-623, the schematic and the changelog fragment came first. The fence's tests came next,
whole, over an inert crate that keeps every shape but the behaviour: its opener opens the source in place, with no
setting and no refusal; its readers answer no row, and its row types hold only the columns the tests name; its blob
codec decodes each view's default and encodes one fixed wrong byte; its field codec splits and joins to empty; and
its wire walk answers an empty list. The commit that carries those tests also carries the census edits of A9 and A10
and the testflight census line of A12, and this record. Each red below was measured on the tree of that commit, by
an assertion against the inert crate. A record cannot name the commit it is in, so the red lines that name that
commit by its sha, and each green, are appended by the record's last commit, once the commits that turn them green
exist.

## The reds, measured on the tests' commit

- A2: `crates/anki-format/tests/opener.rs:103`: assertion `left == right` failed: the settings on the copy;
  `opener.rs:114`: a write is refused as read-only, not Ok(1); `opener.rs:134`: assertion `left == right` failed:
  the one col row is read; `opener.rs:185`: the damaged page is refused at the quick check, not an opened reader.
- A3: `crates/anki-format/tests/refusals.rs:42`: assertion `left == right` failed: the digest check;
  `refusals.rs:51`: the write-ahead log; `refusals.rs:62`: col.ver 17; `refusals.rs:69`: a table set without graves
  is refused; `refusals.rs:82`: the user version.
- A4: `crates/anki-format/tests/refusals.rs:97`: assertion `left == right` failed: the planted view;
  `refusals.rs:114`: the planted trigger; `refusals.rs:134`: the planted virtual table; `refusals.rs:148`: the
  planted table; `refusals.rs:167`: the bound; `refusals.rs:239`: the one col row, ver 18.
- A5: `crates/anki-format/tests/tables.rs:113`: assertion `left == right` failed: the columns kept;
  `tables.rs:154`: the counts.
- A6: `crates/anki-format/tests/collation.rs:63`: assertion `left == right` failed: the collation order;
  `collation.rs:87`: the crate's connection orders by the registration; `crates/anki-format/tests/blobs.rs:144`: the
  blob views; `blobs.rs:193`: the raw bytes kept.
- A8: `crates/anki-format/tests/collection_view.rs:172`: assertion `left == right` failed: the per-table views.
- A9: `crates/ingest/tests/collection_read.rs:301`: assertion `left == right` failed: the moved statements.
- A11: `crates/anki-format/tests/wire_walk.rs:27`: assertion `left == right` failed: [08];
  `wire_walk.rs:35`: [0b, 00]; `wire_walk.rs:49`: 70 bits; `wire_walk.rs:79`: a step that stays;
  `wire_walk.rs:184`: [08, 96, 01]; `crates/anki-format/tests/field_codec.rs:41`: "a"; `field_codec.rs:62`: [""];
  `field_codec.rs:69`: the joined text. The Kani harnesses of K-wire-walk and K-field-split are VOID here: the
  verifier is not installed.
- A12: `scripts/tests/test_context_map_schematic.py`: `test_every_crate_directory_is_one_node_named_by_it`
  (anki-format: no node) and `test_every_arrow_is_a_manifest_edge_the_fence_names` (anki-format: no fence row);
  `scripts/tests/test_testflight_workflows.py`:
  `test_each_lane_runs_on_its_one_trigger_and_nothing_a_pull_request_starts` (the push filter lacks
  `crates/anki-format/**`). The crate's build lines of A12 are not red: the inert crate builds.

```red-first
A1: not red: the crate is born with its dependencies; its planted manifest reads red
A10: not red: the inert opener's in-place open is a connection in opener.rs; the census is red at the base, where that file is absent
```

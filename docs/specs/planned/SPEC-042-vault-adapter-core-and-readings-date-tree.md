# SPEC-042: every vault write is atomic and railed, and the readings keep a byte-preserving date tree

- **Wave:** W1. **Issue:** #28 (epic #2). **Context(s):** `deck-streak-vault`.
- **Decided by:** ADR-002 (the vault is an anti-corruption layer that depends on the kernel only),
  ADR-011 (DeckStreak is the single writer of the readings folder), ADR-012 (the parity oracle),
  ADR-019 (the Mini App first, the vault an archive copy), and ADR-042 (the two write paths, the
  reading note's format and the rails).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-042.md` (ADR-016).

## 1. The problem, measured

- **The vault context is empty** (`crates/vault/src/` holds only `lib.rs`), and the readings'
  archive copy (SPEC-046), the owner's read tick (SPEC-047) and every later vault duty write
  through it.
- **What is ported.** The predecessor's readings notes (`reading_notes.py`): the atomic write under
  a temp name the sync bridge ignores (`_atomic_write`), the eight-key frontmatter
  (`_render_frontmatter`), the byte-preserving roll that patches exactly three keys
  (`_roll_note_text`), the archive path by calendar year, month and ISO week (`_archive_dir`), the
  roll-forward that archives everything the most recent day does not carry (`roll_forward`), and
  the exact-line Studied stamp (`stamp_studied`). The second brain's file contract fixes the rest:
  configured paths (C1), temp names the bridge ignores (C7), content rails (C8), the readings
  folder's single writer (C6).
- **A defect not to port.** The predecessor created the readings folder with `mkdir(parents=True)`;
  the vault root does not let the service create a top-level folder, so its first real night
  would have failed. Here the adapter never creates a top-level folder and says so when one is
  missing.
- **A rail that must change.** The predecessor's readings validator refused every `<` followed by
  a letter; the language mentors' Japanese readings need furigana ruby (`<ruby>`, `<rt>`), which
  the vault-duties pack's rails allow. The rails here are that pack's.
- **What the parity oracle proves.** `reading_notes.py:_roll_note_text` (line endings, a missing
  `first_generated`, an owner's tick) and `reading_notes.py:_archive_dir` (a day whose ISO week
  belongs to the next calendar year).
- **Prerequisites.** SPEC-020 (typed configuration, the clock, the study day) and SPEC-029 (the
  golden reader). It precedes SPEC-046, SPEC-047 and SPEC-048.

## 2. Requirements

R1. The vault root, the readings folder (example value `12-Readings`) and the archive folder inside
    it (example value `Archive`) are values of the kernel's typed configuration; no vault path is a
    literal in code. The vault features refuse to start when the root is unset or not a directory,
    or the readings folder is absent or not writable by the service. The adapter never creates a
    folder at the vault's top level.
R2. Every file the adapter writes lands through a temp file in the target's own directory, named
    `.<name>.<pid>.tmp` (it matches the bridge's ignore pattern
    `\.tmp\.\d+\.|\.tmp$|\.crswap$|^~|\.crdownload$`), which is written, fsynced, renamed over the
    target, and followed by an fsync of the directory. A failed step leaves the target as it was and
    removes the temp file.
R3. Every byte the adapter writes into the vault passes the content rails of the vault-duties
    pack's `no-executable` class, read from the vendored `rails.json`: fences off the allow-list,
    Templater's `<%` anywhere, Dataview inline queries, HTML tags or attributes off the allow-list,
    `javascript:`, `vbscript:`, `data:` and `obsidian:` links, MathJax `\href` and `\url`, and
    embedded `.base` views; plus NUL and other control characters. A refused write names the rail
    and never echoes the text, and nothing is sanitised.
R4. A staged duty run is a staging directory with a `phx.duty.vault.run.v1` record. Its executor
    applies only `create`, `update` and `move`, never a delete, only inside the duty's folders from
    the layout, never over an existing note (compared case-insensitively), and only after the
    vault-duties pack's blocking classes are green on the run. A red class discards the run and
    leaves the vault untouched.
R5. A reading note's path is `<readings>/<YYYY-MM-DD>/<file>`, where the day is the study day and
    the file is the topic key with `/` replaced by `-`, then `.md`. The topic key must match
    `^[a-z0-9]+(?:[-/][a-z0-9]+)*$`, and no frontmatter value may contain `\n`, `\r` or `]`.
R6. A reading note is written by the adapter, never by the agent, in the predecessor's format: the
    eight flat keys `type: reading`, `topic`, `date`, `first_generated`, `last_rolled`, `rolls: 0`,
    `digest`, `tags: [reading, <topic key>]` in that order, then `ai_generated: true` (the
    machine-readable mark of EU AI Act Art. 50(2)), then the reading's validated body, then
    `\n\n- [ ] Studied\n- [ ] I read it\n`. A create refuses an existing target.
R7. Rolling a note forward patches exactly `rolls` (plus one), `last_rolled` (the new study day)
    and `first_generated` (only when absent), keeps every other byte and every line ending, writes
    the destination atomically, reads it back and compares the bytes before it removes the source,
    and equals the golden of `reading_notes.py:_roll_note_text`. A note already at the destination
    refuses the roll.
R8. A roll-forward for a study day rolls, from the single most recent prior-day folder only, the
    notes of the topics the caller names as carried; it moves every other note of any prior-day
    folder unchanged to `<readings>/<archive>/<YYYY>/<MM>/W<WW>/<YYYY-MM-DD>/` (calendar year and
    month, zero-padded ISO week), equal to the golden of `reading_notes.py:_archive_dir`. It removes
    each emptied prior-day folder, reports a malformed note by path and a bounded reason without
    stopping its siblings, and changes nothing when called again on the same day. Archiving a
    superseded note whose name is taken in its archive folder appends `-2`, `-3` and so on before
    `.md`; nothing is ever overwritten.
R9. The Studied stamp replaces exactly the whole line `- [ ] Studied` with `- [x] Studied`, and the
    read tick replaces exactly the whole line `- [ ] I read it` with `- [x] I read it`. Neither
    touches the other's line, neither ever unticks, and a missing or doubled line is reported as
    `box_anchor_missing` or `box_anchor_ambiguous` with no write. The read tick has one caller,
    the owner's read-tap use case (SPEC-047).
R10. Replacing a reading's body (a regeneration of the same reading, SPEC-048) keeps the frontmatter
    and both box lines byte for byte, and refuses with `vault_note_edited` when the note's current
    body is not the body the adapter last wrote (the caller passes that body's hash), so the owner's
    own edits are never overwritten.
R11. A write whose resolved path (after `..` and symbolic links) leaves the configured readings
    folder, or a staged run's duty folders, is refused.
R12. The vault context owns no table and depends on the kernel only.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a write lands through a temp name matching the bridge's ignore pattern, then a file fsync, the rename and a directory fsync, in that order (a recording file-system fake) | `a_write_lands_through_an_ignored_temp_name_then_fsync_rename_and_dir_fsync` |
| A2 | every rail row of `rails.json` refuses its planted fixture and a clean reading note passes (examined count equals the rail rows, zero refused) | `every_rail_refuses_its_planted_fixture_and_a_clean_note_passes` |
| A3 | the adapter's rails and the vault-duties `no-executable` class agree on every planted fixture, each staged in a temporary run | vault-duties `no-executable`; `test_the_adapter_rails_agree_with_the_pack_on_every_fixture` |
| A4 | rolling a note forward equals the golden of `reading_notes.py:_roll_note_text` | `rolling_a_note_matches_the_parity_golden` |
| A5 | rolling a CRLF note whose owner ticked `I read it` keeps that line and every line ending byte for byte | `rolling_forward_keeps_the_owners_tick_byte_for_byte` |
| A6 | archive paths equal the golden of `reading_notes.py:_archive_dir`, a year-boundary ISO week included | `archive_paths_match_the_parity_golden` |
| A7 | a second roll-forward on the same study day changes nothing, and a malformed note is reported while its siblings still roll | `a_second_roll_forward_changes_nothing_and_a_malformed_note_is_reported` |
| A8 | a write outside the readings folder, through `..` or a symbolic link, or over an existing note is refused | `a_write_outside_the_readings_folder_or_over_a_note_is_refused` |
| A9 | the vault-duties rails rows (`write-confinement`, `no-executable`, `never-deletes`) are green over the committed synthetic run, with a non-zero examined count | vault-duties rails stage; `test_the_vault_duties_rails_rows_are_green_on_the_synthetic_run` |
| A10 | a staged run with a red blocking class is discarded and the vault's bytes are unchanged | `a_staged_run_with_a_red_class_leaves_the_vault_untouched` |
| A11 | a missing readings folder refuses the vault features at start and creates no folder | `a_missing_readings_folder_refuses_and_creates_nothing` |
| A12 | the Studied stamp never changes the `I read it` line, the read tick changes only its own line, and neither unticks | `each_box_write_changes_only_its_own_line` |
| A13 | a body replacement keeps the frontmatter and both box lines, and refuses a note whose body the owner edited | `a_body_replacement_keeps_the_boxes_and_refuses_an_edited_note` |

```acceptance
A1: cargo test -p deck-streak-vault --test atomic -- --exact a_write_lands_through_an_ignored_temp_name_then_fsync_rename_and_dir_fsync
A2: cargo test -p deck-streak-vault --test rails -- --exact every_rail_refuses_its_planted_fixture_and_a_clean_note_passes
A3: python3 -m unittest discover -s scripts/tests -p test_vault_rails_rows.py -k test_the_adapter_rails_agree_with_the_pack_on_every_fixture
A4: cargo test -p deck-streak-vault --test roll -- --exact rolling_a_note_matches_the_parity_golden
A5: cargo test -p deck-streak-vault --test roll -- --exact rolling_forward_keeps_the_owners_tick_byte_for_byte
A6: cargo test -p deck-streak-vault --test archive -- --exact archive_paths_match_the_parity_golden
A7: cargo test -p deck-streak-vault --test roll -- --exact a_second_roll_forward_changes_nothing_and_a_malformed_note_is_reported
A8: cargo test -p deck-streak-vault --test confinement -- --exact a_write_outside_the_readings_folder_or_over_a_note_is_refused
A9: python3 -m unittest discover -s scripts/tests -p test_vault_rails_rows.py -k test_the_vault_duties_rails_rows_are_green_on_the_synthetic_run
A10: cargo test -p deck-streak-vault --test staged -- --exact a_staged_run_with_a_red_class_leaves_the_vault_untouched
A11: cargo test -p deck-streak-vault --test confinement -- --exact a_missing_readings_folder_refuses_and_creates_nothing
A12: cargo test -p deck-streak-vault --test boxes -- --exact each_box_write_changes_only_its_own_line
A13: cargo test -p deck-streak-vault --test boxes -- --exact a_body_replacement_keeps_the_boxes_and_refuses_an_edited_note
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/vault/Cargo.toml` | `deck-streak-vault` | changed: the workspace dependencies it uses |
| `crates/vault/src/lib.rs` | `deck-streak-vault` | changed: the modules below |
| `crates/vault/src/config.rs` | `deck-streak-vault` | added: the configured paths and the start check |
| `crates/vault/src/fs.rs` | `deck-streak-vault` | added: the file-system port and its real implementation |
| `crates/vault/src/atomic.rs` | `deck-streak-vault` | added |
| `crates/vault/src/rails.rs` | `deck-streak-vault` | added: the rails, read from the vendored `rails.json` |
| `crates/vault/src/staged.rs` | `deck-streak-vault` | added: the run record, the three-verb executor, the gate call |
| `crates/vault/src/note.rs` | `deck-streak-vault` | added: the reading note's render, patch and box lines |
| `crates/vault/src/readings_tree.rs` | `deck-streak-vault` | added: paths, create, roll, roll-forward, archive, stamp, tick, body replacement |
| `crates/vault/tests/atomic.rs` | `deck-streak-vault` | added |
| `crates/vault/tests/rails.rs` | `deck-streak-vault` | added |
| `crates/vault/tests/fixtures/rails/` | `deck-streak-vault` | added: one planted fixture per rail row and one clean note, plain notes with no run record |
| `crates/vault/tests/roll.rs` | `deck-streak-vault` | added |
| `crates/vault/tests/archive.rs` | `deck-streak-vault` | added |
| `crates/vault/tests/boxes.rs` | `deck-streak-vault` | added |
| `crates/vault/tests/staged.rs` | `deck-streak-vault` | added |
| `crates/vault/tests/confinement.rs` | `deck-streak-vault` | added |
| `crates/vault/tests/runs/clean/duty-run.json` | `deck-streak-vault` | added: one synthetic run, green on every blocking vault-duties row |
| `crates/vault/tests/runs/clean/` | `deck-streak-vault` | added: that run's staged notes |
| `scripts/tests/test_vault_rails_rows.py` | repo | added |
| `tools/parity-oracle/registry/spec_042.py` | repo | added: registers `reading_notes.py:_roll_note_text` and `reading_notes.py:_archive_dir` (SPEC-029's registry) |
| `tools/parity-oracle/goldens/_roll_note_text.json` | repo | added |
| `tools/parity-oracle/goldens/_archive_dir.json` | repo | added |
| `Cargo.lock` | workspace | changed |
| `docs/schematics/vault-write-paths.md` | docs | added |
| `docs/specs/SPEC-042-vault-adapter-core-and-readings-date-tree.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-042-vault-write-paths-note-format-and-rails.md` | docs | added |
| `docs/red-first/SPEC-042.md` | docs | added |

Planted-defect runs are built in temporary directories by the tests and never committed: the pack's
walk from the root would find them and turn its rows red.

## 5. What this does NOT do

- It pre-creates no folder on the host and sets no group permission: the readings folder and its
  archive folder are made by the private operations rail before the first live night (#45).
- It writes no drill file and reads no drill answer (#136).
- It writes no stats file into the dashboard folder (#153).
- It files no inbox capture (#154), and captures no journal line (#56).
- It runs no agent vault duty; the daily note, the synthesis and the curator stage their runs
  through this executor later (#49, #50).
- It builds no bridge from the vault back to Anki (#65).

## 6. Risks

- **The owner's device changes a note while a roll reads it** (the sync bridge writes the replica).
  Detected by the read-back comparison before the source is removed (R7): the roll fails loud and
  the source stays.
- **The ported rails drift from the pack's `rails.json`.** Detected by A3, which runs the vendored
  probe on the same fixtures.
- **The readings folder is missing on the first live night.** Detected at start (A11) and paged as a
  broken rail by the readings health check (SPEC-050); the folder is an operations prerequisite
  (#45).
- **A second writer of the readings folder** (the predecessor's lane, which still fires nightly).
  Prevented by the one-writer prerequisite of the first live night (SPEC-053).
- **A case-insensitive file system on a device** merges two names. Detected by the case-insensitive
  collision check (R4, R8).

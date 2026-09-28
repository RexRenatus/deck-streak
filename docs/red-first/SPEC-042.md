# Red-first record: SPEC-042

The goldens of `reading_notes.py:_roll_note_text` and `reading_notes.py:_archive_dir` were registered
and generated first (c3deaec), and the schematic drawn (05755d5). Then the tests were committed
(2814116) beside an adapter whose public API was in place and whose behaviour was stubbed: the atomic
write renamed a temporary file without syncing it or its folder, the rails refused nothing, the topic
key and the start check accepted everything, the roll returned its note unchanged, the archive folder
was empty, the box writes and the body replacement wrote nothing, and the executor applied every
create without a check or a gate; the planted fixtures were committed with the tests, and the
synthetic run was not. Each criterion was run there, with the SPEC's own fenced command, and failed by
assertion for its own criterion, not by a compile error, a missing fixture or an empty selection. The
implementation and the synthetic run followed (abe9781). Between the two commits no test changed what
it asserts: two test files took lint fixes only (an allowance on the bridge's case-sensitive pattern,
a type alias), and one test was added for the live-note lookup.

2814116 was replayed from a `git archive` export with a target directory of its own and ran 13 of 13
red; abe9781 ran 13 of 13 green in the worktree, with nothing uncommitted. Failures below name no path
and no calendar date.

```red-first
A1: red at 2814116: assertion `left == right` failed: a temp file in the target's own directory, then its fsync, the rename and the dir fsync; left: CreateNew, Write, Rename; right: CreateNew, Write, SyncFile, Rename, SyncDir
A1: green at abe9781
A2: red at 2814116: assertion `left == right` failed: the planted fixture embed-base.md is refused by its own rail row dynamic_embed_extensions:.base, and by no other; left: {}, right: {"dynamic_embed_extensions:.base"}
A2: green at abe9781
```
```retired
A3: red at 2814116: AssertionError: Items in the second set but not the first: the adapter and the pack disagree on 22 of the 23 fixtures (failures=22; the clean note agreed)
A3: green at abe9781
```
```red-first
A4: red at 2814116: assertion `left == right` failed: the roll of the golden's first fresh case; left: the note unchanged, rolls 0; right: rolls 1 and last_rolled a study day later
A4: green at abe9781
A5: red at 2814116: assertion `left == right` failed: the notes rolled; left: [], right: the carried note's path in today's folder
A5: green at abe9781
A6: red at 2814116: assertion `left == right` failed: the archive folder of the golden's first case, a year-boundary day; left: "", right: the predecessor's archive folder
A6: green at abe9781
A7: red at 2814116: assertion `left == right` failed: the notes rolled; left: [], right: the carried note's path in today's folder
A7: green at abe9781
A8: red at 2814116: the topic key "../escape" was accepted
A8: green at abe9781
```
```retired
A9: red at 2814116: AssertionError: False is not true : the synthetic run is not committed
A9: green at abe9781
```
```red-first
A10: red at 2814116: the run was not discarded for its red note-links class: Applied { ops: 1 }
A10: green at abe9781
A11: red at 2814116: a missing readings folder started as Ok(())
A11: green at abe9781
A12: red at 2814116: assertion `left == right` failed: the Studied stamp's outcome; left: AlreadyTicked, right: Ticked
A12: green at abe9781
A13: red at 2814116: assertion `left == right` failed: the frontmatter and both box lines are kept byte for byte; left: the note with its first body, right: the note with the second
A13: green at abe9781
```

In A1 the recording file system saw no file sync and no folder sync. In A2 and A3 the stub rails
refused none of the 22 planted fixtures, which the pack's own probe refuses. In A4 the golden's first
case came back unrolled. In A5 and A7 the stub roll-forward moved nothing. In A6 the stub archive
folder was empty for the golden's first year-boundary day. In A8 the stub topic key accepted a key
that climbs out of its folder. In A9 the synthetic run the pack judges was not yet committed. In A10
the stub executor applied a run the pack's `note-links` class finds red. In A11 the stub start check
opened a vault with no readings folder. In A12 the stub stamp wrote nothing, and in A13 the stub body
replacement kept the old body.

Amendment (2026-09-28): the lines of A3 and A9 moved into a `` ```retired `` fence, by inserted
fence lines, because SPEC-056 retired those criteria when it removed their tests.

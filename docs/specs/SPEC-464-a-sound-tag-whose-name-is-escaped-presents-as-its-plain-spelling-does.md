# SPEC-464: a sound tag whose name is escaped presents as its plain spelling does

- **Issue:** #820 (Closes: the delivery decides it whole). **Context(s):** `engine-core`
  (`crates/engine-core`): `src/media.rs` at the video decision, and its tests.
- **Decided by:** ADR-481 (this SPEC's own: D1 the mechanism, D2 the cure, D3 the production change, D4 the
  shape and the two amendments, D5 the tests, rows, formal surface and push, D6 the exclusions). **Amends:**
  SPEC-410 R3 and ADR-424 D1, insert-only (section 9); SPEC-348 stands as it is (ADR-481 D4).
- **Schematic:** `docs/schematics/sound-tag-written-name-and-decoded-load.md` (this delivery adds it).
- **Status:** designed; one pull request, one push (ADR-481 D4, D5). **Mutation band:** `S46400-S46499`.
  **Changelog:** `changelog.d/sound-tag-escaped-dot-464.md`. **Red-first record:** `docs/red-first/SPEC-464.md`.

Refs read: `DEV` is DeckStreak `dev` `36c4783a29c44d2ffafc987803ccb1742a2c2cfc`. Every fact below is read at
DEV with `git show DEV:<path>`, at the lines beside it.

## 1. The problem, measured

| id | fact | where at DEV |
|---|---|---|
| M1 | The video decision is `self.reader.shows_video() && media_type(written, &VIDEO_TYPES).is_some()`: it reads the name as the card writes it | `crates/engine-core/src/media.rs:193-195` |
| M2 | `media_type` reads the text after the last literal `.`, compared ASCII-case-insensitively; a name with no `.` has no type | `crates/engine-core/src/media.rs:141-147` |
| M3 | The admit path decodes first: `decoded_name(written)?`, then `plain_name`, then the table's type and kind, then the read of the decoded name, then both caps | `crates/engine-core/src/media.rs:281-302` (`:287`, `:288`, `:291`, `:292`) |
| M4 | `decoded_name` turns each `%` and two hex digits into its byte and refuses bytes that are not UTF-8 | `crates/engine-core/src/media.rs:107-123` |
| M5 | The route sends a tag to `budget.video(name)` when `budget.shows_video(name)`, else to `budget.sound(name)`, whose clip is `Clip::Sound { name: name.clone(), bytes }`, the written name | `crates/engine-core/src/face.rs:164-178` (`:167`, `:171-174`) |
| M6 | `TYPES` reads `mp4` as `audio/mp4` and holds no `m4v` and no `mov`; `VIDEO_TYPES` holds `mp4`, `m4v` and `mov` | `crates/engine-core/src/media.rs:19-36`, `:52-56` |
| M7 | A refused name joins `omitted` once, under its written spelling | `crates/engine-core/src/media.rs:266-277` |
| M8 | The font pass decides on the decoded name, as `admit` does | `crates/engine-core/src/media.rs:241-257` (`:243-244`) |
| M9 | The ffi's `MediaFolder` answers `shows_video` with `true`; the trait's default, which the web's reader keeps, is `false` | `crates/ffi/src/face.rs:64-81`; `crates/engine-core/src/media.rs:89-91` |
| M10 | Every sound tag the media tests drive is spelt with a plain dot, and the test reader that asks for video is `Showing` | `crates/engine-core/tests/face.rs:66-71`, `:795`, `:897-898`, `:1061`, `:970-981` |
| M11 | Rows S41012 and S41013 anchor on `media.rs:194` and mutate its two operands; no row reads the escaped spelling | `scripts/mutation-rows.d/S41000-S41099.json:112-129` |
| M12 | `shows_video`, `VIDEO_TYPES`, `m4v`, `mov` and `quicktime` occur in no file under `ios/` or `web/` | a fenced `git grep` over the tree at DEV |
| M13 | No `@phx covers` line under `formal/` names `media.rs`, `face.rs` or `tests/face.rs` | the 254 cover lines at DEV (section 8) |
| M14 | `extract_av_tags` is the pinned engine's: `face.rs` imports it from `anki::card_rendering`, so its source is not in this tree | `crates/engine-core/src/face.rs:10`, `:197`, `:199` |
| M15 | `admit` decodes and then checks that the decoded name is plain, on two adjacent lines; the font pass decodes with the same first line | `crates/engine-core/src/media.rs:287-288`, `:243` |
| M16 | `decoded_name` rebinds `high` and `low` to values derived from them, under the crate's lints | `crates/engine-core/src/media.rs:112-113` |

So, at DEV, for a reader that asks, `[sound:clip%2Emp4]` crosses as `Clip::Sound` over `clip.mp4`'s bytes
while `[sound:clip.mp4]` is shown as a video, and `[sound:clip%2Em4v]` is refused before any read while
`[sound:clip.m4v]` is shown. ADR-481 D2 removes that split: the decision reads the decoded name, and this SPEC
states it and pins it.

## 2. Requirements

R1. `Budget::shows_video` decides on the name `admit` reads: the sound tag's name with its `%XX` escapes
decoded by `decoded_name`, the one decoder `admit` runs. For a reader whose `shows_video` is `true`, a sound
tag whose decoded extension is in `VIDEO_TYPES` is shown as a video, whatever escapes its written spelling
holds, and a name that does not decode is not. The body's line
`self.reader.shows_video() && media_type(written, &VIDEO_TYPES).is_some()` stays byte-identical, with
`written` bound to the decoded name above it.

R2. Over every extension of `TYPES` and `VIDEO_TYPES`, a sound tag whose dot is escaped, and one whose
extension's first letter is escaped, presents as its plain spelling does, for a reader that asks for video
and for one that does not: the same elements after the side's text, the same clips replayed and autoplayed
over the same bytes, the same names omitted and the same files read. A sound clip's `name` and an omitted
name keep the spelling as written.

R3. `admit`'s refusal set and both caps are unchanged. Under an escaped spelling, for both readers, a name
that does not decode, a name that decodes to one holding a separator, a backslash or a NUL, a dot name, a
file past `FILE_CAP`, a file the reader lacks and a file past `FACE_CAP` are each refused and named once in
`omitted` as written, and only a name that passes the name and type rules is read.

R4. Of `crates/*/src`, only `crates/engine-core/src/media.rs` changes, and there only `Budget::shows_video`'s
doc comment and body. `crates/engine-core/src/face.rs`, the ffi and the web engine do not change.

R5. Two hand rows in `scripts/mutation-rows.d/S46400-S46499.json` guard R1 and R3 (section 7), and rows
S41012 and S41013 keep their find and their killers.

R6. SPEC-410 R3 and ADR-424 D1 each gain the one paragraph section 9 inserts, and no line of either changes.

## 3. Acceptance criteria of SPEC-464

| id | criterion | decided by |
|---|---|---|
| A1 | Over every extension of `TYPES` and `VIDEO_TYPES`, each named once by a hand table that is checked against both tables, the question side of a note whose sound tags escape each name's dot (`x%2Emp4`) presents, for a reader that asks for video and for one that does not, what the hand table says the plain spelling presents, and the same note spelt plainly presents it too: the same elements after the side's text, the same clips replayed and autoplayed over the same bytes under the name as written, the same names omitted as written, and the same names read | `cargo test -p deck-streak-engine-core --test face -- --exact an_escaped_dot_presents_as_its_plain_spelling_over_the_whole_type_table` |
| A2 | As A1, for a note whose sound tags escape the first letter of each name's extension (`x.%6Dp4`) | `cargo test -p deck-streak-engine-core --test face -- --exact an_escaped_extension_letter_presents_as_its_plain_spelling_over_the_whole_type_table` |
| A3 | For a reader that asks and for one that does not, the question side of a note whose sound tags spell, with escapes, a name that does not decode, names that decode to a separator, a backslash or a NUL, a dot name, a file past the file cap beside its plain spelling, a file the reader lacks, and five sounds the fifth of which passes the face cap, shows no element, replays and autoplays the four sounds within the face cap, omits every refused name once as written, and asks its reader only for the names that pass the name and type rules | `cargo test -p deck-streak-engine-core --test face -- --exact escaped_names_admit_refuses_stay_refused_under_both_caps_for_both_readers` |

A1 and A2 are red first: at the base the reader that asks gets no element for the escaped `mp4`, `m4v` and
`mov` spellings (ADR-481 D5). A green at the base on either means the route already sees a decoded name, and
the delivery stops. A3 is MUTATION COVERAGE, recorded `not red`: it pins what the base already does, with its
hand row (section 7).

```acceptance
A1: cargo test -p deck-streak-engine-core --test face -- --exact an_escaped_dot_presents_as_its_plain_spelling_over_the_whole_type_table
A2: cargo test -p deck-streak-engine-core --test face -- --exact an_escaped_extension_letter_presents_as_its_plain_spelling_over_the_whole_type_table
A3: cargo test -p deck-streak-engine-core --test face -- --exact escaped_names_admit_refuses_stay_refused_under_both_caps_for_both_readers
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-464-a-sound-tag-whose-name-is-escaped-presents-as-its-plain-spelling-does.md` | docs | added: this SPEC |
| `docs/decisions/ADR-481-a-sound-tags-video-decision-reads-the-decoded-name-admit-reads.md` | docs | added: ADR-481 |
| `docs/schematics/sound-tag-written-name-and-decoded-load.md` | docs | added: the data flow of a sound tag's name |
| `docs/specs/SPEC-410-a-sound-tag-that-names-a-video-plays-as-video-in-the-iphone-and-ipad-card-view.md` | docs | R6: one paragraph inserted after R3 (section 9); no line changes |
| `docs/decisions/ADR-424-the-core-writes-a-sound-tags-video-into-the-face-as-a-data-url-video-element-for-a-reader-that-asks.md` | docs | R6: one paragraph inserted after D1's shape (section 9); no line changes |
| `crates/engine-core/src/media.rs` | engine-core | R1, R4: `Budget::shows_video` decodes the name with `decoded_name` above its unchanged line, and its doc comment says so |
| `crates/engine-core/tests/face.rs` | engine-core | R2, R3, A1 to A3: three tests, a hand table, a reader that records each name it is asked for, and their helpers, appended after the sound video tests |
| `scripts/mutation-rows.d/S46400-S46499.json` | rows | added: S46400 and S46401 (section 7) |
| `docs/red-first/SPEC-464.md` | docs | added: A1 and A2 red then green, A3 recorded `not red` |
| `changelog.d/sound-tag-escaped-dot-464.md` | docs | added: the fragment, citing #820 |

## 5. What this does NOT cover

- A sound clip's `name` and the omitted names in the decoded spelling: both keep the spelling as written
  (`face.rs:172`, `media.rs:273-274`), and `face.rs` is not changed (#820).
- The admit path's name rules, their order and both caps: unchanged, and A3 pins them (#820).
- Adding `m4v` or `mov` to `TYPES`, which would make the reader that does not ask hear them as audio (#632).
- The native client's player and its view of a sound clip's name: they receive what the core writes, as
  before (#632).
- The web client's faces: its reader does not ask for video, so every sound tag stays a sound there (#630).
- A device session: the element an escaped spelling now yields is its plain spelling's, which SPEC-410's
  session showed (#632).

## 6. Risks

- The route sees a decoded name, because the engine decodes a sound tag's name before `clips` does. A1 and A2
  then read green at the base; the builder stops and reports it, and the decision is made again on that
  measurement.
- Another delivery that changes `clips()`, `Budget::shows_video`, `admit`, `decoded_name` or the sound video
  tests' helpers lands first. The build re-measures each at its cut and stops on a changed fact.
- A row's find moves under a formatter, or the change makes another band's find ambiguous. Each find is
  written after `cargo fmt`, and every row on `media.rs` is counted once at the head.
- The crate's lints refuse the rebinding of `written`. Clippy runs with `-D warnings` on the change, and the
  same file's `decoded_name` rebinds `high` and `low` under those lints (M16).
- A3's face-cap fixture is large: its files pass the face cap only in total, and every failure message names
  the clips and never prints their bytes.

## 7. Mutation rows

The band file `scripts/mutation-rows.d/S46400-S46499.json` holds two `MUTATIONS` rows of seven cells (id,
crate, file relative to the crate, find, replace, killer, why), in the shape of
`scripts/mutation-rows.d/S41000-S41099.json`. Each find occurs exactly once in `crates/engine-core/src/media.rs`
at the head, and each replacement keeps every item used.

| id | table | target | the mutant | killer |
|---|---|---|---|---|
| `S46400-THE-VIDEO-DECISION-DECODES-THE-NAME` | `MUTATIONS` | engine-core `src/media.rs` | the decision's decode call is removed: `decoded` holds the written name | `face::an_escaped_dot_presents_as_its_plain_spelling_over_the_whole_type_table` |
| `S46401-ADMIT-CHECKS-THE-DECODED-NAME-IS-PLAIN` | `MUTATIONS` | engine-core `src/media.rs` | `admit`'s name rule reads the written name in place of the decoded one | `face::escaped_names_admit_refuses_stay_refused_under_both_caps_for_both_readers` |

S46400's find is the line the change adds, `let decoded = decoded_name(written);`. S46401's find is the
two-line span `media.rs:287-288`, whose first line also opens the font pass, so the span and not its first line
is the find. Rows S41012 and S41013 keep their find, `media.rs:194`, byte-identical, and are proved again over
the new body. No row sits on the failed-decode answer: every answer to it presents alike, so its mutant is
equivalent (ADR-481 D5).

## 8. Formal

NOT APPLICABLE by surface (ADR-481 D5). The change adds no actor, timer, lock, static, task or write path: it
changes the answer of a total function, `Budget::shows_video`, inside one face's completion, whose budget lives
and dies inside one `complete` call. Of the 254 cover lines under `formal/` at DEV, none names
`crates/engine-core/src/media.rs`, `crates/engine-core/src/face.rs` or `crates/engine-core/tests/face.rs`.

## 9. Amendments this delivery inserts

Each block below is inserted, every line strictly between its BEGIN and END lines, into the file its BEGIN line
names, immediately after the one line its AFTER line quotes, which occurs once in that file; no line of either
file changes.

AFTER `replay. The element carries no part of the written name.`
BEGIN docs/specs/SPEC-410-a-sound-tag-that-names-a-video-plays-as-video-in-the-iphone-and-ipad-card-view.md

Amended by SPEC-464 (ADR-481 D2, #820): the extension R3 reads is that of the name `admit` reads, the sound
tag's name with its `%XX` escapes decoded by `decoded_name`, so `[sound:clip%2Emp4]` and `[sound:clip.%6Dp4]`
are shown as `[sound:clip.mp4]` is, and an escaped `m4v` or `mov` spelling as its plain spelling is. A name
that does not decode is not shown as a video: SPEC-348 R3's rules refuse it on the sound path before any read,
as on every path.
END

AFTER `` `complete` appends `videos.concat()` to the text. ``
BEGIN docs/decisions/ADR-424-the-core-writes-a-sound-tags-video-into-the-face-as-a-data-url-video-element-for-a-reader-that-asks.md

Amended by ADR-481 D2 (SPEC-464, #820): the decision reads the extension of the name `admit` reads.
`Budget::shows_video` binds `written` to the sound tag's name decoded by `decoded_name`, the decoder `admit`
runs, before the shape's `media_type(written, &VIDEO_TYPES)` line, which stays as written; a name that does
not decode is bound to the empty name, which has no type, so it stays a sound tag and `admit` refuses it
before any read. ADR-481 D2 names the options it was chosen against.
END

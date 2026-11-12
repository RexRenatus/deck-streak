# ADR-481: a sound tag's video decision reads the decoded name admit reads

- **Status:** proposed, with SPEC-464 (Closes #820).
- **Context(s):** `engine-core` (`crates/engine-core`): `src/media.rs` at the video decision, and its tests.
  `crates/engine-core/src/face.rs`, `ffi` (`crates/ffi`) and the web engine's reader are read and not changed.
- **Settles:** #820: whether `Budget::shows_video` reads a sound tag's decoded name, and the pins of the
  escaped spellings under that answer.
- **Amends:** SPEC-410 R3 and ADR-424 D1, each by one inserted paragraph that reads the extension from the
  decoded name (D4); no line of either changes. ADR-424 D1's shape line `media_type(written, &VIDEO_TYPES)`
  stays as written: the change binds `written` to the decoded name above it (D3).
- **Read at:** DeckStreak `dev` `36c4783a29c44d2ffafc987803ccb1742a2c2cfc` (DEV). Every `path:line` below is
  read there.

## Context

SPEC-410 (#808) taught the core to show a sound tag that names a video as a `data:` video element, for a
reader that asks. #820 records that the decision and the load read the name in two ways: the decision reads
the name as the card writes it, and the load reads it after `%XX` decoding. The steps, at DEV:

| step | what it reads | where |
|---|---|---|
| the side's sound tags | `extract_av_tags` gives each `[sound:...]` tag as `SoundOrVideo(name)`, `name` as the card writes it | `crates/engine-core/src/face.rs:197`, `:199` |
| the route | a tag goes to the video path when `budget.shows_video(name)`, else to the sound path | `crates/engine-core/src/face.rs:167-174` |
| the video decision | `self.reader.shows_video() && media_type(written, &VIDEO_TYPES).is_some()`: the name as written | `crates/engine-core/src/media.rs:193-195` |
| the extension | `media_type` takes the text after the LAST literal `.` and compares it with each table row ASCII-case-insensitively; a name with no `.` has no type | `crates/engine-core/src/media.rs:141-147` |
| the sound path | `take(written, &TYPES, "audio/")` | `crates/engine-core/src/media.rs:186-188` |
| the video path | `take(written, &VIDEO_TYPES, "video/")`, then the constant element | `crates/engine-core/src/media.rs:200-208` |
| the admit path | `decoded_name(written)?`, then `plain_name`, then the table's type and kind, then the read of the DECODED name, then both caps | `crates/engine-core/src/media.rs:281-302` (`:287`, `:288`, `:291`, `:292`) |
| the decode | each `%` followed by two hex digits becomes its byte; `None` when the bytes are not UTF-8 | `crates/engine-core/src/media.rs:107-123` |
| the omitted names | a refused name joins `omitted` once, under its written spelling | `crates/engine-core/src/media.rs:266-277` (`:273-274`) |
| the sound clip | `Clip::Sound { name: name.clone(), bytes }`: the written name, the decoded file's bytes | `crates/engine-core/src/face.rs:171-174` |
| a font | the font pass decides "is this a font" on the DECODED name, `plain_name(&name) && media_type(&name, &FONT_TYPES)` | `crates/engine-core/src/media.rs:241-257` (`:243-244`) |
| the tables | `TYPES` holds `mp4` as `audio/mp4` and has no `m4v` and no `mov`; `VIDEO_TYPES` holds `mp4`, `m4v`, `mov` | `crates/engine-core/src/media.rs:19-36`, `:52-56` |
| the readers | the ffi's `MediaFolder` answers `shows_video` with `true` and reads the name it is given from the opened collection's media folder; the web's reader keeps the default `false` | `crates/ffi/src/face.rs:64-81`; `crates/engine-core/src/media.rs:89-91` |

So, for a reader that asks: `[sound:clip.mp4]` is shown as a video; `[sound:clip%2Emp4]` has no literal `.`,
takes the sound path, decodes to `clip.mp4`, which `TYPES` reads as `audio/mp4`, and crosses as
`Clip::Sound` named `clip%2Emp4` over `clip.mp4`'s bytes. An escaped letter of the extension
(`[sound:clip.%6Dp4]`) takes the same path, because its written extension is `%6Dp4`. A case-folded extension
(`clip.MOV`) does not differ: both paths compare ASCII-case-insensitively. A name whose decode fails, or
whose decoded name is not plain, is refused by `admit` on either path and joins `omitted` once, so its face
does not differ by path.

One case changes more than the presentation. `[sound:clip%2Em4v]` takes the sound path, decodes to
`clip.m4v`, and `TYPES` has no `m4v`, so `admit` refuses it at `:291`, before the read at `:292`: no file is
read for it. Under a decision that read the decoded name, it would take the video path, and `admit` would
read `clip.m4v`. The same holds for `mov`. Today `clip.m4v` is reachable only by a spelling whose WRITTEN
extension is `m4v`.

No test drives an escaped sound tag: the media tests of `crates/engine-core/tests/face.rs` name every sound
tag with a plain dot (`:66-71`, `:795`, `:897-898`, `:1061`), and `media.rs` holds no tests of its own. No mutation row guards
the reading: rows S41012 and S41013 anchor on `media.rs:194` and mutate the two operands, each killed by a
plain-dot test (`scripts/mutation-rows.d/S41000-S41099.json:112-129`). No `@phx covers` line under `formal/`
names `media.rs`, `face.rs` or `tests/face.rs` (0 of the 254 cover lines there). No other client carries a
copy of the decision: `shows_video`, `VIDEO_TYPES`, `m4v`, `mov` and `quicktime` occur in Rust files and
documents alone, never under `ios/` or `web/`.

## Decisions, and the alternatives each was chosen against

### D1. The mechanism: the decision reads the written name, the load reads the decoded name, and three other sites read the written name

The video decision reads the name as written (`media.rs:194`), and the admit path reads it decoded
(`media.rs:287`). The other sites of the media path that read the written name are the route's guard
(`face.rs:167`), the sound clip's `name` (`face.rs:172`) and the omitted names (`media.rs:273-274`). The font
pass reads the decoded name for its own decision (`media.rs:243-244`), as `admit` does. The mismatch class is
a sound tag whose DECODED extension is in `VIDEO_TYPES` while its WRITTEN extension is not: an escaped dot,
an escaped letter of the extension, or both. The reverse cannot occur when the decode succeeds: a written
extension in `VIDEO_TYPES` holds no `%`, so its decoded extension is the same. `extract_av_tags` is the
pinned engine's (`face.rs:10` imports it from `anki::card_rendering`), so whether it decodes a `%XX` escape
before the route is not read from this tree: SPEC-464 A1 and A2 measure it at the base, where each must read
red, and a green there means the route already sees a decoded name.

**Chosen against:**

- naming only the escaped dot: because an escaped letter of the extension takes the same path for the same
  reason, so a pin of the dot alone leaves the class half pinned.
- reading the route from the issue's words alone: because the issue names `mp4`, and the `m4v` and `mov` case,
  where the paths differ in what is read, is found only by reading both tables beside `admit`.

### D2. The cure: the video decision reads the decoded name, through `admit`'s own decoder

`Budget::shows_video` decodes the sound tag's name with `decoded_name`, the one decoder `admit` runs
(`media.rs:107-123`, `:287`), and looks the decoded name's extension up in `VIDEO_TYPES`. The decision and
the action then read one form of the name: over every extension of `TYPES` and `VIDEO_TYPES`, a spelling that
escapes its dot or a letter of its extension presents exactly as its plain spelling does, for a reader that
asks for video and for one that does not, the clip's `name` and the omitted names keeping the spelling as
written. A name that does not decode answers `false`: it takes the sound path, where `admit` refuses it at its
first rule before any read and names it once in `omitted`, exactly as the video path would. A decoded name
that is not plain is decided by its extension like any other, and `admit` refuses it at `:288`, before any
read, on either path. `admit`'s rules, their order and both caps are unchanged. The names this newly routes to
a read, `[sound:clip%2Em4v]`, `%2Emov` and the escaped-letter spellings that decode to them, read only the
file their plain spelling already reads, under the same name rules and the same caps, so no file becomes
reachable that is not reachable at DEV.

**Chosen against:**

- keeping the decision on the name as written and pinning the escaped dot as a sound: because a decision on
  one form of a name and an action on another is the split #820 names, and a pin would record it as intended.
- decoding for an escaped `mp4` alone: because the predicate then needs three operands to say one rule, and an
  escaped `m4v` or `mov` still presents unlike its plain spelling, so the split stays for those two.
- decoding each sound tag's name before the route, in `face.rs`: because it is a second decode path beside
  `admit`'s, the clip's `name` and the omitted names would change spelling on both clients, and `face.rs` is a
  file another open change edits.
- a `plain_name` check inside the decision: because it is a second copy of `admit`'s name rule, and no face
  differs by it, since `admit` refuses a name that is not plain before any read on both paths.
- answering a failed decode from the written name: because it keeps a second reading of the name inside the
  decision, and it presents exactly as `false` does, so no face can tell the two apart.

### D3. Production or test: one production change, in `media.rs` at the video decision

The build changes one item under `crates/*/src`, `Budget::shows_video`: two lines are inserted above its
body's one line, which stays byte-identical, and its doc comment says the decision reads the decoded name.

```rust
    pub(crate) fn shows_video(&self, written: &str) -> bool {
        let decoded = decoded_name(written);
        let written = decoded.as_deref().unwrap_or_default();
        self.reader.shows_video() && media_type(written, &VIDEO_TYPES).is_some()
    }
```

`written` is bound to the decoded name, or to the empty name, which has no type, when the decode fails. The
last line is `media.rs:194` byte for byte, so rows S41012 and S41013 keep their find and their killers. The
file already rebinds a name to a value derived from it (`high` and `low` in `decoded_name`, `media.rs:112-113`)
under the same lints. No other client needs a change: the ffi's reader only answers `true` and reads the name
it is given, and the web's reader does not ask, so its faces do not change.

**Chosen against:**

- rewriting `media.rs:194` as one expression over `decoded_name(written)`: because the line rows S41012 and
  S41013 anchor on would change, and another delivery's band holds them.
- passing one decoded name from the route to the decision and both paths: because the route is in `face.rs`,
  which another open change edits, and `take` and `admit` would change their signatures for no face that
  differs.
- a cure or a pin in the native or the web client: because neither carries the decision; the native player
  receives a `Clip::Sound` or an element from the core, and the web's reader keeps every sound tag a sound.

### D4. Shape and split: one delivery, one pull request, two insert-only amendments

One delivery under SPEC-464, ADR-481 and band `S46400-S46499`, one pull request that says `Closes #820`, and
a new schematic, `docs/schematics/sound-tag-written-name-and-decoded-load.md`. SPEC-410 R3 and ADR-424 D1 each
take one paragraph inserted after an anchor line read at DEV, which reads the extension from the decoded name
and points here; no line of either document changes. SPEC-348 is not amended: its section 12 points to
SPEC-410 and ADR-424, which carry the change.

**Chosen against:**

- two deliveries, the pins first and the cure later: because the reds and the change that turns them green
  belong to one red-first record, and a pin-only delivery would record the split as intended.
- rewriting SPEC-410 R3's or ADR-424 D1's sentences in place: because an inserted paragraph keeps every line
  those documents were judged by, and its proof is a numstat that deletes nothing.
- appending a section to `docs/schematics/ios-review-screen.md`, the media path's schematic: because another
  open change edits that file, and a new schematic shares no file with it.

### D5. Tests, rows, formal and pushes

A1 and A2 are RED-FIRST. Each drives the 18 extensions of `TYPES` and `VIDEO_TYPES`, from a hand table that
names each once and is checked against both tables, through a note spelt plainly and a note spelt with one
escape: the dot (`x%2Emp4`) for A1, the extension's first letter (`x.%6Dp4`) for A2. Each note is read by a
reader that asks for video and by one that does not, both recording every name they are asked for, and each
face is held field by field to the hand table's answer: the elements after the side's text, the clips
replayed and autoplayed under the name as written over the file's bytes, the names omitted as written, and
the names read. At the base each fails on the reader that asks, where the escaped `mp4`, `m4v` and `mov`
spellings yield no element. A3 is MUTATION COVERAGE, recorded `not red`: names `admit` refuses, spelt with
escapes, stay refused under both caps for both readers, and only a name that passes the name and type rules
is read. Every expected value is written out by hand: each file's bytes are its plain name's, and the three
elements' base64 is spelt in the table.

Two hand rows in `scripts/mutation-rows.d/S46400-S46499.json`, in the band's `MUTATIONS` shape: S46400
removes the decision's decode call (A1 kills it: the escaped video spellings lose their elements), and S46401
has `admit`'s name rule read the written name (A3 kills it: the names that decode to a separator or a NUL are
read and shown). Rows S41012 and S41013 are proved again over the new body. No row sits on the failed-decode
answer: every answer to it presents alike, so its mutant is equivalent.

FORMAL: NOT APPLICABLE by surface. The change adds no actor, shared state, timer, lock or write path: it
changes a total function's answer inside one face's completion, and no `@phx covers` line names a file it
edits. One push: the reds are Rust tests read at the red commit, and the gate's red-first stage runs them at
the merge-base itself.

**Chosen against:**

- one test over both escapes: because a failure on one spelling would hide the other's answer.
- a test per extension: because one face per spelling reads every extension beside the others under one
  budget, and the hand table counts what it examined.
- a row that changes the sound clip's `name` in `face.rs`: because `face.rs` is a file another open change
  edits, so its anchor could move under the row before it lands.
- a Lean port of `shows_video`: because the function decides how a clip is presented, never a verdict, a
  ledger, money or a release, and the hand table's answers over every extension already close its answers.
- a second push to read a red on CI: because the reds are read locally at the red commit, and CI's red-first
  stage measures them at the merge-base without one.

### D6. Not in this delivery

The admit path's name rules, their order and both caps; a sound clip's `name` and the omitted names, which keep
the spelling as written; the web client's and the native client's media views; `m4v` or `mov` in `TYPES`; and
a device session, since the element an escaped spelling now yields is its plain spelling's. SPEC-464 section 5
names each with its issue.

**Chosen against:**

- reviewing the admit path's rules in the same delivery: because D2 keeps them whole and A3 only pins them, so
  a review of them is its own decision.

## Consequences

- A learner whose sound tag escapes its dot or a letter of its extension sees, on the native client, the
  video its plain spelling shows; the web client keeps every sound tag a sound, as before. SPEC-464 A1 and A2
  pin both readers over every extension.
- `[sound:clip%2Em4v]` and `[sound:clip%2Emov]` now read `clip.m4v` and `clip.mov`, the files their plain
  spellings read, under the same rules and caps, and such a file counts toward the face's cap as the plain
  spelling's does.
- A sound clip and an omitted name keep the spelling as written, so an escaped spelling's sound differs from
  its plain spelling's by its `name` alone.
- Rows S46400 and S46401 guard the decode call and the decoded name rule; S41012 and S41013 hold on the
  unchanged line.

## What would make this wrong

- `extract_av_tags` decoding a sound tag's name before the route: A1 and A2 then read green at the base, the
  split is not where D1 places it, and the build stops.
- A card writer that relies on an escaped spelling to keep a video file a sound: no test or fixture at DEV
  writes an escaped sound tag, so such a reliance would surface as a report against this change.
- A later delivery that adds `m4v` or `mov` to `TYPES`: the reader that does not ask would then hear them as
  audio, and A1's and A2's hand table changes with it.
- A change to `decoded_name`: it now moves the decision and `admit` together, which is D2's point, and A1, A2
  and A3 read it.

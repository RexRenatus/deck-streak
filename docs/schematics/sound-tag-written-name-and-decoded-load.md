# Schematic: a sound tag's name, from the card to its video decision and to the decoded load

- **For:** SPEC-464, ADR-481. **Kind:** data flow. **Read at:** DEV `36c4783a`.
- **Stands on:** `docs/schematics/ios-review-screen.md` section 7 (SPEC-410's route of a sound tag's video),
  which this schematic leaves as it is.

A sound tag's name reaches the route as the card writes it. SPEC-464 has the budget's video decision decode
it first, with `decoded_name`, the one decoder `admit` runs, so the decision and the load read one form of the
name: an escaped spelling and its plain spelling join at that decode, before the route, and take one path. A
name that does not decode becomes the empty name, which has no type, and stays a sound tag. The clip's name
and the omitted names keep the spelling as written.

```mermaid
flowchart TD
  CARD["the card's text: a sound tag"] --> TAGS["extract_av_tags: SoundOrVideo, the name as written"]
  TAGS -->|"x%2Emp4, x.%6Dp4, x.mp4"| DECODE["Budget::shows_video, first: decoded_name, the decoder admit runs (SPEC-464 R1, changed)"]
  DECODE -->|"one name: x.mp4"| ROUTE{"the reader asks AND the DECODED extension is in VIDEO_TYPES (media.rs:194, unchanged)"}
  DECODE -->|"no decode: the empty name, no type"| ROUTE
  ROUTE -->|"yes: x.mp4, x.m4v, x.mov in any spelling"| VPATH["video path: take with VIDEO_TYPES, kind video/"]
  ROUTE -->|"no: an audio or image type, a reader that does not ask, or no decode"| SPATH["sound path: take with TYPES, kind audio/"]
  VPATH --> ADMIT["admit: decoded_name, plain_name, the table's type, then the read, then both caps"]
  SPATH --> ADMIT
  ADMIT -->|"under VIDEO_TYPES, admitted"| ELEM["constant video element after the side's text: no part of the name"]
  ADMIT -->|"under TYPES, an audio type, admitted"| SOUND["Clip::Sound under the WRITTEN name, the decoded file's bytes"]
  ADMIT -->|"refused by a rule or a cap"| OMIT["omitted, once, under the written name"]
  ADMIT -->|"read through the Reader"| READER["MediaFolder or a test reader: the DECODED name"]
  ELEM --> FACE["Face: replay, autoplay, text, omitted"]
  SOUND --> FACE
  OMIT --> FACE
  A1["A1, A2: each escaped spelling presents as its plain one, for both readers, over every extension"] -.-> DECODE
  A1 -.-> FACE
  A3["A3: refused names stay refused under both caps, and only names past the name and type rules are read"] -.-> ADMIT
  A3 -.-> READER
  R0["S46400: the decode call removed, killed by A1"] -.-> DECODE
  R1["S46401: the name rule reads the written name, killed by A3"] -.-> ADMIT
```

| step | reads the name | what an escaped spelling takes | file:line at DEV | changed by SPEC-464 |
|---|---|---|---|---|
| tags | as written | `SoundOrVideo("x%2Emp4")` | `crates/engine-core/src/face.rs:197`, `:199` | no |
| decision | decoded, by `decoded_name` | `x.mp4`: shown as a video for a reader that asks, as `x.mp4` is | `crates/engine-core/src/media.rs:193-195`, `:107-123` | yes: two lines above `:194`, which stays byte-identical (R1; rows S46400, S41012, S41013) |
| route | the decision's answer | the video path | `crates/engine-core/src/face.rs:167-174` | no |
| admit | decoded | `x.mp4` under `VIDEO_TYPES`; a refused name at the rule that refuses it | `crates/engine-core/src/media.rs:281-302` (`:287`, `:288`, `:291`, `:292`) | no (pinned by A3; row S46401) |
| read | decoded | the file `x.mp4`, which the plain spelling reads too | `crates/engine-core/src/media.rs:292`; `crates/ffi/src/face.rs:66-72` | no |
| element | no part of the name | the plain spelling's element | `crates/engine-core/src/media.rs:58-67` | no |
| clip | as written | `Clip::Sound { name: "x%2Emp3", bytes }` for an audio type | `crates/engine-core/src/face.rs:171-174` | no |
| omitted | as written | `"x%2Epng"`, once | `crates/engine-core/src/media.rs:266-277` | no |

The cure is the decision on the decoded name (ADR-481 D2). The names it newly routes to a read, an escaped
`m4v` or `mov`, read only the file their plain spelling reads, under the same rules and caps.

Unchanged and outside the change: `crates/engine-core/src/face.rs`, `crates/ffi/src/face.rs`, `admit`,
`TYPES`, `VIDEO_TYPES`, both caps, every Swift and web file, and the web's reader, which keeps `shows_video` at
its default, so every sound tag stays a sound there.

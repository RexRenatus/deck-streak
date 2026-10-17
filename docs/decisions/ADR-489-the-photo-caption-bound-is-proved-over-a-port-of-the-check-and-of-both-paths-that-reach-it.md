# ADR-489: the photo caption bound is proved over a port of the check and of both paths that reach it, and tied to the code by vectors the crate reads

- **Status:** accepted for SPEC-472. **Issue:** #480 (refs #430, #457 and #468).
- **Context(s):** `deck-streak-notifications` (`crates/notifications/src/photo.rs` and
  `crates/notifications/src/router.rs`, read and never changed; one new test target); `formal/` (one
  new Lean entry, its writer and its vectors).
- **Read at:** DeckStreak `dev` `36c4783a`. Every `path:line` below is read there.

## Context

#457 (for #430) moved the photo caption's length check into one total function that the
notifications crate asks before any transport call. Its tests sample the claim: `share_caption.rs`
judges a generated population of 600 captions (eight character classes by three lengths by 25
forms, `crates/notifications/tests/share_caption.rs:124-150`, its count asserted at :178-183), and
`photo_render.rs`'s A16 counts 512 and 513 characters outside the basic plane (SPEC-132 A16). A
population can sample a property over every caption; it cannot prove it. #480 asks for a Lean entry
that states the property over every string and proves it against the crate's rule, including the
count of code units for characters outside the basic plane.

What the code does at `36c4783a`:

- `check_caption` (`photo.rs:89-94`) counts a caption by `caption.encode_utf16().count()` (:90),
  compares the count with `MAX_CAPTION_UTF16` (:18, `1_024`) by `>`, answers
  `Err(PhotoError::Caption)` over it (:91) and `Ok(())` at it and under it (:93).
- `Photo::new` (:118-133) checks the bytes first (:119-129: the byte bound, the format, the
  dimensions, the ratio), then calls `check_caption(&caption)?` (:131).
- `Router::prepare_share` (`router.rs:537`) calls `check_caption(caption)` first (:538) and answers
  `Prepared::Refused(refusal)` (:539) before it asks the transport (:541-543).
- The bot's own `MAX_CAPTION_UTF16` (`crates/bot/src/transport.rs:71`, 1024) bounds a document's
  caption by `chunk::visible_units` (:639) and leaves an over-long one off (:644); the bot's photo
  and share calls (:673, :768) add no bound of their own.
- No `@phx covers` line names `photo.rs`, and none names `prepare_share`: the eleven covers that name
  `router.rs` anchor `route`, `flush_with`, `take_lease`, `deliver`, `render_pushed`,
  `flush_reactions`, `after_claim`, `decide` and `send_bot`.
- The rows on the bound are SPEC-132's: `S13212-CAPTION-UTF16` (the count by characters),
  `S13213-THE-SHARE-ASKS-THE-BOUND`, `S13214-THE-SHARE-REFUSES-BY-NAME`,
  `S13215-THE-PHOTO-ASKS-THE-SAME-BOUND` and `S13216-A-CAPTION-AT-THE-LIMIT-IS-TAKEN`
  (`scripts/mutation-rows.d/S13200-S13299.json`).

## Decision

**D1. The mechanism.** The bound is one function with one inclusive limit, and both paths ask it
before any transport call. `check_caption` counts UTF-16 code units, one for a character of the
basic plane and two for any other, because the standard library's encoder yields a surrogate pair
for a character above U+FFFF; it refuses `Caption` when the count is over 1,024 and takes the
caption at 1,024 and under. Building a photo takes exactly the captions the check takes whenever
the image is within the photo's other bounds, since the caption is checked last (`photo.rs:131`)
and an image outside them is refused by its own bound whatever the caption. Preparing a share takes
exactly the captions the check takes, since the check comes first (`router.rs:538-539`) and a
refusal returns before the transport is asked. So the two paths take one set of captions. The bot's
own caption bound is not the same rule: it counts a document's caption after its markup and drops an
over-long caption instead of refusing it. How `chunk::visible_units` counts is not measured here.

**D2. The proof.** A new Lean entry, `lean/CaptionBound`, in `formal/lean/Formal/CaptionBound.lean`:

- **The port.** A caption is a list of characters, as a Rust `&str` is a sequence of Unicode scalar
  values and a Lean `Char` is one. `encodeUtf16` ports the encoder's branch for one character (one
  unit when `code & 0xFFFF == code`, else two); `countUnits` is the length of the units the caption's
  characters yield, the port of `.encode_utf16().count()`; `maxCaptionUtf16` is 1024;
  `checkCaption` ports `check_caption` branch for branch. `photoNew` ports `Photo::new`'s order: the
  image's own verdict, which reads only the bytes, is one input (`none` when the bytes pass), then
  the check. `prepareShare` ports `prepare_share`: the check, then the transport, as a `Share` that
  is either `refused` with the check's error or `transport`. The covers are `check_caption` and
  `MAX_CAPTION_UTF16` in `photo.rs` and `prepare_share` in `router.rs`.
- **The claims, in the issue's words.** `TakesExactlyTheCaptionsWithinTheLimit check`: for every
  caption, `check` answers `ok` exactly when its length in UTF-16 code units is within 1,024, a
  character outside the basic plane counting two code units, and `Caption` otherwise. The length is
  written in the claim by the plane rule, `utf16Length`, never by the port's encoder.
  `OneSetOfCaptions photo share`: for every caption, the photo over an image within its bounds and
  the share each take it exactly when its length is within 1,024; over the limit the photo answers
  `Caption` and the share is `refused` with `Caption`; and an image's own refusal stands whatever
  the caption.
- **The theorems.** `the_check_takes_a_caption_exactly_when_its_utf16_units_fit` is
  `TakesExactlyTheCaptionsWithinTheLimit checkCaption`, and
  `the_photo_and_the_share_take_one_set_of_captions` is `OneSetOfCaptions photoNew prepareShare`.
  Both enter at `ramp=report`.
- **The witnesses.** The code at the base already holds both claims, so each witness ports a
  mutant of the S13200 band that its row proves killed, a wrong reading the red-first record of
  SPEC-132 shows at an earlier commit. `a_caption_is_counted_in_utf16_units_not_in_characters` is
  the negation of the first claim for `S13212`'s count by characters
  (`caption.chars().count() > MAX_CAPTION_UTF16`), at 1,023 ASCII letters followed by one character
  outside the basic plane: 1,024 characters, 1,025 units. The record's A16 red at `cd05a8b0c3a1` is a
  photo that took 513 characters outside the basic plane, 1,026 units, as a count by characters
  would; what that commit's check counted is not read here.
  `the_share_asks_the_bound_of_its_own_caption` is the negation of the second for `S13213`'s share
  that asks the bound of the empty caption, so it takes every caption, as the share with no bound
  that A17's red measured at `cb142394e372` did, at 1,025 ASCII letters.
- **The vectors.** `formal/lean/Formal/CaptionBoundVectors.lean` writes
  `formal/vectors/caption-bound.jsonl`: one header line and 55 captions, each as runs of code points
  with the port's unit count and its answer. They are the empty caption, then nine characters
  (U+0061, U+00E9, U+65E5, U+D7FF, U+E000, U+FFFF, U+10000, U+1F600 and U+10FFFF) at 1,023, 1,024
  and 1,025 units, each filled with the character and with the character last after ASCII letters.
  So they hold captions at the limit, one over it, and ones whose last character lies outside the
  basic plane. A Rust test in the notifications crate reads every vector through `check_caption`,
  `Photo::new` and `Router::prepare_share`, so the port and the code stay tied.
- **TLA+ does not apply.** No two actors share state here: the check reads one borrowed caption that
  no other actor can change, and the share's check precedes its transport call inside one function.

**D3. Production or test: test only.** D1 finds one set of captions on both paths, so there is no
cure to make. No production line changes. No existing test line changes: `share_caption.rs`'s
population of 600, A16 to A18, and rows S13208 to S13216 keep every examined figure.

**D4. Shape: one delivery.** SPEC-472 and this ADR, the band `S47200-S47299`, and a new schematic,
`docs/schematics/the-photo-caption-bound-and-its-lean-proof.md`. SPEC-132 is not amended: the
delivery changes nothing it states. Its R3 still reads "1,024 characters after escaping"
(SPEC-132 lines 44-48), older wording that its A16 and its amendment for #430 already state in
UTF-16 units.

**D5. Tests and rows.** A1 and A2 are MUTATION COVERAGE: the bound is already on dev, so neither is
red at the base, and each records `not red` with its red named. A1's red is row `S47200`, A2's is row
`S47206`. Beside them, four plants of the S13200 band's mutants (`S13212`, `S13213`, `S13215` and
`S13216`), installed one at a time and restored, each turn A1 red, quoted in the record's prose.
Seven rows, `S47200` to `S47206`, mutate the vectors file, each killed by A1 or A2. The Lean entry
is this delivery's own formal surface. One push.

**D6. Not in this delivery.** The bot's own caption bound and message limits; what the platform does
with a caption it takes; the transport calls' sending and their refusal by default; and a CI job
that checks the Lean entry. SPEC-472's "What this does NOT cover" lists each with its issue.

## What it was chosen against

- D1, two bounds, one per path: one function already serves both paths (`photo.rs:131`,
  `router.rs:538`), and a second would be a second count to keep equal to the first.
- D1, counting the bot's document bound as the same rule: it counts after markup and drops the
  caption, so a proof of the photo's rule says nothing about it.
- D2, a claim over Lean's `String` and its library functions: the elaborator's `decide` cannot unfold
  them, and the claim would rest on library lemmas no DeckStreak code runs.
- D2, a port that models a caption as its unit count: it would assume the per-character count the
  issue asks to prove.
- D2, one claim joining the count and the two paths: each part has a counterexample of its own, so
  each takes its own theorem and witness.
- D2, a count by UTF-8 bytes as the declared witness: it errs the other way, refusing captions within
  the limit, and no row names it; the character count is row `S13212`'s mutant, and the byte count
  and the count that refuses at the limit are checked as scratch examples instead.
- D2, a cover on `Photo::new`: `photo.rs` defines `new` twice (`FileId::new` at :66, `Photo::new` at
  :118), so the anchor resolves to nothing; A18's source read, row `S13215` and A1 hold its call.
- D3, a production change to give the vectors test a red at the base: there is no defect to cure,
  and a change made for a red proves nothing about the code that runs.
- D3, widening `share_caption.rs`'s population with the vectors' captions: it moves A17's examined
  600 and the expected-size formula it asserts.
- D4, two deliveries, the Lean entry first and the vectors test second: a port whose vectors no test
  of the crate reads is tied to nothing, so the proof and the tie land together.
- D4, an insert-only amendment of SPEC-132: the delivery changes no behaviour SPEC-132 states.
- D4, an amendment of `docs/schematics/w7-image-pipeline-and-its-no-provider-path.md`: a new
  schematic keeps the proof's map in one file, and the pipeline's drawing stays as written.
- D5, committing the vectors test one commit before the vectors: it would be red for a missing file,
  never for the bound.
- D5, rows on `photo.rs` beside the S13200 band's: the delivery changes no line there, and those
  mutants already have their rows and killers.

## Consequences

- The caption bound is proved over every caption, for the port the covers tie to the three items.
  An edit to `check_caption`, `MAX_CAPTION_UTF16` or `prepare_share` makes the entry `STALE` until
  its port is re-read and re-stamped.
- The vectors test runs the crate's own code on every vector, so a port that drifts from the code,
  or code that drifts from the port, fails a cargo test in CI even though no CI job runs the Lean
  entry itself.
- The Lean entry is checked by the formal checker by hand at build, verify and land. DeckStreak CI
  runs no formal job.
- The theorems stay at `ramp=report` until their streak earns a flip, in a later delivery.

## What would make this wrong

- The standard library's encoder yielding another count than the port's branch: A1 compares each
  vector's count with `encode_utf16().count()` at the plane's edges, U+FFFF and U+10000 among them.
- The platform counting a caption in another unit than UTF-16 code units after its entities are
  parsed: the bound would then be wrong, and the proof would still hold of the code.
- An edit that moves `Photo::new`'s call of the check, which no cover holds: A18, row `S13215` and A1
  each fail on it.

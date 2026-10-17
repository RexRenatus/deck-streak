# SPEC-472: the photo caption bound is proved in Lean for every caption, and tied to the code by its vectors

- **Issue:** #480 (refs #430, #457 and #468). **Context(s):** `deck-streak-notifications` (a new test
  target that reads the Lean port's vectors; `photo.rs` and `router.rs` are read, never changed);
  `formal/` (the Lean entry `lean/CaptionBound`, its writer and its vectors).
- **Decided by:** ADR-489 (this SPEC's own: the port, the two claims, their witnesses, the vectors
  and the rows), ADR-295 (the repository's formal-check settings).
- **Prerequisites:** SPEC-132 with its amendment for #430 (the one bound function, `check_caption`,
  that `Photo::new` and `Router::prepare_share` both call) and SPEC-295 (`config/formal.json`). Both
  have landed. **Mutation band:** `S47200-S47299`.
- **Status:** in build, as one delivery that carries `docs/red-first/SPEC-472.md`.

## The problem, measured

Read at DeckStreak `dev` `36c4783a`.

- **The check counts code units, in one place.** `git show 36c4783a:crates/notifications/src/photo.rs | grep -n 'MAX_CAPTION_UTF16\|encode_utf16\|check_caption'`
  prints five lines: :17, a doc line naming the bot's constant, then
  `18:pub const MAX_CAPTION_UTF16: usize = 1_024;`,
  `89:pub fn check_caption(caption: &str) -> Result<(), PhotoError> {`,
  `90:    if caption.encode_utf16().count() > MAX_CAPTION_UTF16 {` and
  `131:        check_caption(&caption)?;`. The comparison is `>`, so 1,024 units are taken and 1,025 are
  refused `PhotoError::Caption` (:91); `Photo::new` (:118-133) checks the bytes at :119-129 before the
  caption at :131.
- **The share asks the same function first.** `git show 36c4783a:crates/notifications/src/router.rs | grep -n 'check_caption'`
  prints `40:use crate::photo::{FileId, Photo, check_caption};` and
  `538:        if let Err(refusal) = check_caption(caption) {`; :539 returns
  `Prepared::Refused(refusal)` before the transport is asked at :541-543.
- **The tests sample the property.** `crates/notifications/tests/share_caption.rs` judges 600
  captions (eight classes by three lengths by 25 forms, :124-150, the size asserted at :178-183);
  SPEC-132 A16 counts 512 and 513 characters outside the basic plane. No test or proof covers every
  caption.
- **No proof covers the crate's bound.** `git grep -n '@phx covers' 36c4783a -- formal` prints 254
  lines: none names `crates/notifications/src/photo.rs`, and none names `prepare_share`.
- **The vectors shape exists.** `git show 36c4783a:formal/lean/Formal/Vectors.lean | grep -c '^import'`
  prints 14: each Lean entry's writer is one import and one arm of `main` there, and each Rust reader
  is a `crates/<crate>/tests/formal_vectors_<entry>.rs` target.

## Requirements

- **R1.** The Lean entry `lean/CaptionBound`, `formal/lean/Formal/CaptionBound.lean`, ports
  `check_caption` and `MAX_CAPTION_UTF16` (`crates/notifications/src/photo.rs`) and
  `Router::prepare_share` (`crates/notifications/src/router.rs`) over a caption read as its
  characters, each character yielding one UTF-16 unit when its code satisfies `code & 0xFFFF == code`
  and two otherwise, as the standard library's encoder does; it covers those three items by name,
  and `Photo::new`'s order (the image's own bounds, then the check) is ported beside them.
- **R2.** Its theorem `the_check_takes_a_caption_exactly_when_its_utf16_units_fit` states and proves
  that for every caption the check takes it exactly when its length in UTF-16 code units is within
  the platform's caption limit of 1,024, a character outside the basic plane counting two code
  units, and refuses every other with `Caption`. The claim counts the length by that plane rule, never
  by the port's encoder. Its witness `a_caption_is_counted_in_utf16_units_not_in_characters` proves
  that a check counting characters (row `S13212`'s mutant) breaks the claim.
- **R3.** Its theorem `the_photo_and_the_share_take_one_set_of_captions` states and proves that for
  every caption, building a photo over an image within the photo's other bounds and preparing a
  share each take it exactly when its length is within 1,024; that over the limit the photo answers
  `Caption` and the share is refused with `Caption` before any transport call; and that an image's
  own refusal stands whatever the caption. So no caption either path takes is one the platform would
  cut. Its witness `the_share_asks_the_bound_of_its_own_caption` proves that a share asking the
  bound of the empty caption (row `S13213`'s mutant) breaks the claim.
- **R4.** The writer `formal/lean/Formal/CaptionBoundVectors.lean`, dispatched by one import and one
  arm of `formal/lean/Formal/Vectors.lean`, writes `formal/vectors/caption-bound.jsonl`: one header
  line, then the empty caption, then for each of U+0061, U+00E9, U+65E5, U+D7FF, U+E000, U+FFFF,
  U+10000, U+1F600 and U+10FFFF, at 1,023, 1,024 and 1,025 units, the caption filled with that
  character (padded with one U+0061 when the character is two units and the length is odd) and the
  caption of U+0061 letters with that character last. Each vector names its runs of code points, the
  port's unit count and the port's answer. The file is the writer's output, never edited by hand.
- **R5.** A test target of the notifications crate reads every vector through `check_caption`,
  `Photo::new` over an image within its bounds, and `Router::prepare_share` with a transport that
  counts its calls, and holds the vectors' classes.
- **R6.** No production line changes and no existing test changes: `photo.rs`, `router.rs`,
  `transport.rs`, the bot crate, `share_caption.rs`, `photo_render.rs` and `config/formal.json` keep
  every byte. The entry runs inside the formal settings' default budget.
- **R7.** Seven mutation rows, `S47200` to `S47206`, mutate the vectors file, each killed by A1 or A2.

## Acceptance criteria of SPEC-472

| id | criterion | decided by |
|---|---|---|
| A1 | for every caption of `formal/vectors/caption-bound.jsonl`, and as many as its header counts, the caption's `encode_utf16().count()` equals the vector's unit count; `photo::check_caption` takes it exactly when the vector does and refuses each other with `PhotoError::Caption`; `Photo::new` over an image within its bounds answers the same; and `Router::prepare_share` answers `Ready` after one transport call for each caption the vector takes and `Refused(PhotoError::Caption)` after none for each other. The header names the entry, the covered file and its anchor; the test holds the taken and refused counts and prints what it examined | `the_caption_bound_answers_every_lean_vector` |
| A2 | the vectors hold the captions that tell the code's count from a broken one: captions a count of characters would take over the limit, captions a count of UTF-8 bytes would refuse within it, captions at exactly the limit, captions whose last character lies outside the basic plane, and every character R4 names; the test holds each class's size | `the_lean_vectors_hold_each_caption_a_broken_count_misjudges` |

```acceptance
A1: cargo test -p deck-streak-notifications --test formal_vectors_caption_bound -- --exact the_caption_bound_answers_every_lean_vector
A2: cargo test -p deck-streak-notifications --test formal_vectors_caption_bound -- --exact the_lean_vectors_hold_each_caption_a_broken_count_misjudges
```

## What the formal check judges

DeckStreak CI runs no formal job. The formal checker judges these by hand, on the committed head,
before the pull request is marked ready and again when it lands; the readings are quoted in the
pull request and in `docs/red-first/SPEC-472.md`'s prose.

| id | criterion | decided by |
|---|---|---|
| F1 | R2's theorem is clean and its witness builds | `formal check --entry lean/CaptionBound` |
| F2 | R3's theorem is clean and its witness builds | `formal check --entry lean/CaptionBound` |
| F3 | every cover is stamped (no `STALE`), the vectors equal the writer's output byte for byte (no `DERIVED_DRIFT`), each theorem and witness uses only the settings' allowed axioms, and the ratchet reads no weakening | `formal check --entry lean/CaptionBound --gate`, then `formal check --ratchet-only` |

## File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-472-the-photo-caption-bound-is-proved-in-lean-for-every-caption-and-tied-to-the-code-by-its-vectors.md` | repo | added |
| `docs/decisions/ADR-489-the-photo-caption-bound-is-proved-over-a-port-of-the-check-and-of-both-paths-that-reach-it.md` | repo | added |
| `docs/schematics/the-photo-caption-bound-and-its-lean-proof.md` | repo | added |
| `formal/lean/Formal/CaptionBound.lean` | `formal` | added: the port, the two claims, the theorems and the witnesses (R1 to R3) |
| `formal/lean/Formal/CaptionBoundVectors.lean` | `formal` | added: the writer (R4) |
| `formal/lean/Formal/Vectors.lean` | `formal` | changed: one import and one arm of `main`, each in sorted place |
| `formal/vectors/caption-bound.jsonl` | `formal` | added: the writer's output |
| `crates/notifications/tests/formal_vectors_caption_bound.rs` | `deck-streak-notifications` | added: A1 and A2 |
| `scripts/mutation-rows.d/S47200-S47299.json` | repo | added: R7's rows |
| `docs/red-first/SPEC-472.md` | repo | added |
| `changelog.d/photo-caption-bound-472.md` | repo | added |

## What this does NOT cover

- It proves nothing about the bot's own caption bound or its message limits: the bot bounds a
  document's caption by its visible units and leaves an over-long one off
  (`crates/bot/src/transport.rs`, `MAX_CAPTION_UTF16` and `MAX_TEXT_UTF16`), a different rule from
  the photo's (#958).
- It bounds no caption by what the platform parses: the caption is counted as written, which only
  over-counts its markup, so a caption whose parsed text fits but whose markup does not is refused,
  as SPEC-132 decided (#430).
- It changes neither transport call, their sending, nor their refusal by default (SPEC-132 R1 and
  R2; #125).
- It adds no CI job that checks a Lean entry, and flips neither theorem from `ramp=report`: a flip
  waits for its streak of clean landings, in a delivery of its own (#806).

## Risks

- **A covered span moves.** An edit to `check_caption`, `MAX_CAPTION_UTF16` or `prepare_share`
  stales the entry until its port is re-read and re-stamped. An open change to `router.rs` that
  lands first and moves `prepare_share` is re-stamped at this delivery's cut; one that lands after
  re-stamps the entry itself. The formal checker's `STALE` finding detects it.
- **`Photo::new` has no cover.** `photo.rs` defines `new` twice, so the anchor would resolve to
  nothing. SPEC-132's A18, row `S13215` and A1 each fail if its call of the check moves or goes.
- **Two writers' arms meet.** Other entries add arms to `formal/lean/Formal/Vectors.lean`. One-line
  arms in sorted place merge by keeping both; a conflict there is read before the merge.
- **A package-root edit restarts the streak.** An edit to `formal/lean/lakefile.toml`,
  `lean-toolchain` or `lake-manifest.json` restarts every Lean entry's streak, this one's too.
- **The plane lemma.** The port's `code & 0xFFFF == code` must be proved equal to the plane rule by
  lemmas the elaborator and the kernel check; an evaluation that rests on an axiom outside the
  settings' allow-list is a hole the axiom audit refuses.
- **A census of formal vectors.** A tree test that counts Lean entries, writers or vectors readers
  reads this entry as one more; it is run before the push.

## Mutation rows

In `scripts/mutation-rows.d/S47200-S47299.json`, table `CARGO_KILLED_SCRIPT_MUTATIONS`, each on
`formal/vectors/caption-bound.jsonl`, killer crate `notifications`:

| row | what it guards | killer |
|---|---|---|
| `S47200-THE-LIMIT-TAKES-A-CAPTION-ENDING-OUTSIDE-THE-PLANE` | 1,022 letters and U+1F600, 1,024 units, are taken | `formal_vectors_caption_bound::the_caption_bound_answers_every_lean_vector` |
| `S47201-ONE-UNIT-OVER-IS-REFUSED` | 1,023 letters and U+10FFFF, 1,025 units, are refused | `formal_vectors_caption_bound::the_caption_bound_answers_every_lean_vector` |
| `S47202-A-CHARACTER-OUTSIDE-THE-PLANE-COUNTS-TWO` | 512 of U+1F600 and a letter count 1,025 units, never 513 | `formal_vectors_caption_bound::the_caption_bound_answers_every_lean_vector` |
| `S47203-THE-LAST-CHARACTER-OF-THE-PLANE-COUNTS-ONE` | 1,024 of U+FFFF count 1,024 units | `formal_vectors_caption_bound::the_caption_bound_answers_every_lean_vector` |
| `S47204-THE-FIRST-CHARACTER-PAST-THE-PLANE-COUNTS-TWO` | 512 of U+10000 count 1,024 units | `formal_vectors_caption_bound::the_caption_bound_answers_every_lean_vector` |
| `S47205-THE-HEADER-COUNTS-EVERY-VECTOR` | the header's count equals the vectors read | `formal_vectors_caption_bound::the_caption_bound_answers_every_lean_vector` |
| `S47206-THE-VECTORS-HOLD-A-CAPTION-A-CHARACTER-COUNT-TAKES` | the vectors keep 1,023 letters and U+1F600, which a count by characters takes | `formal_vectors_caption_bound::the_lean_vectors_hold_each_caption_a_broken_count_misjudges` |

## Parity goldens

None. The bound has no predecessor number to prove: the vectors are the port's, and the code
answers them.

## Tables and the v9 import

None. Nothing is stored.

## References

Issue #480; ADR-489; SPEC-132 and its red-first record; SPEC-295 and ADR-295;
`docs/schematics/the-photo-caption-bound-and-its-lean-proof.md`.

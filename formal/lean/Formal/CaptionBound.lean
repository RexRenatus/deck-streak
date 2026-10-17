-- @phx covers crates/notifications/src/photo.rs anchor=check_caption digest=sha256:c530f0a2abc6064204250253e049224649f06af8a8beaacfe00f8aa59414ba06
-- @phx covers crates/notifications/src/photo.rs anchor=MAX_CAPTION_UTF16 digest=sha256:e819493a6ea780adb17d3a47e76de1eadf8ea1385424a12bfb058b61bc819c76
-- @phx covers crates/notifications/src/router.rs anchor=prepare_share digest=sha256:4cd521f9b97a9992a38f25bf7169e138918dcaad8b087c7581f4d44af02c86d4
-- @phx vectors formal/vectors/caption-bound.jsonl
-- @phx cites #480
-- @phx theorem the_check_takes_a_caption_exactly_when_its_utf16_units_fit ramp=report
-- @phx witness a_caption_is_counted_in_utf16_units_not_in_characters kills=the_check_takes_a_caption_exactly_when_its_utf16_units_fit
-- @phx theorem the_photo_and_the_share_take_one_set_of_captions ramp=report
-- @phx witness the_share_asks_the_bound_of_its_own_caption kills=the_photo_and_the_share_take_one_set_of_captions

/-!
# Formal.CaptionBound

The photo caption's bound (#480; SPEC-472 R1 to R3; ADR-489 D2): `check_caption` and
`MAX_CAPTION_UTF16` in `crates/notifications/src/photo.rs`, and `Router::prepare_share` in
`crates/notifications/src/router.rs`, with `Photo::new`'s order in `photo.rs` beside them, as total
functions over a caption read as its characters.

**The claim.** For every caption:
- `the_check_takes_a_caption_exactly_when_its_utf16_units_fit`: the check takes the caption exactly
  when its length in UTF-16 code units is within the platform's caption limit of 1,024, a character
  outside the basic plane counting two units, and refuses every other with `Caption`;
- `the_photo_and_the_share_take_one_set_of_captions`: building a photo over an image within the
  photo's other bounds and preparing a share each take the caption exactly when its length is
  within 1,024; over the limit the photo answers `Caption` and the share is refused with `Caption`
  before any transport call; and an image's own refusal stands whatever the caption.

Both claims write the length by the plane rule, `utf16Length`: one unit below U+10000 and two from
it. They never read the port's encoder, and their limit is the literal 1024, never the port's
constant.

**What is ported.** A Rust `&str` is a sequence of Unicode scalar values, and a Lean `Char` is one,
so a caption is a `List Char`. `encodeUtf16` ports the standard library's `char::encode_utf16`,
which `check_caption` reaches through `str::encode_utf16`: one unit, the code itself, when
`code & 0xFFFF == code`, else the surrogate pair of `code - 0x10000`. `countUnits` is the length of
the units every character yields, the port of `.encode_utf16().count()`. `maxCaptionUtf16` is
`MAX_CAPTION_UTF16`, and `checkCaption` is `check_caption` branch for branch: `Caption` when the
count is over the constant, else `Ok`. `photoNew` is `Photo::new`'s order: the image's own verdict,
then the check. `prepareShare` is `Router::prepare_share`: the check first, its refusal returned as
`Prepared::Refused` before the transport is asked, else the transport, as a `Share`.

**What is not ported.** What the image's bounds read (the byte bound, the header's format, the
dimensions and the ratio): they read only the bytes, so they reach `photoNew` as one input, `none`
when the bytes pass. What the transport answers once it is asked, and the bot's own caption bound,
which is a different rule (#958).

**The witnesses** are mutants of the S13200 band, each read killed by its row. At `36c4783a`,
`scripts/mutation-rows.d/S13200-S13299.json:107-108` replaces the count of units by a count of
characters, `caption.chars().count() > MAX_CAPTION_UTF16` (row `S13212`): `checkByCharacters`
ports it, and `a_caption_is_counted_in_utf16_units_not_in_characters` refutes the first claim for
it at 1,023 letters and one character outside the basic plane, 1,024 characters and 1,025 units.
SPEC-132's record reads its A16 red at that kind of caption, `docs/red-first/SPEC-132.md:39`. At
`36c4783a`, `scripts/mutation-rows.d/S13200-S13299.json:116-117` has the share ask the bound of the
empty caption, `check_caption("")` (row `S13213`): `prepareShareAskingTheEmptyCaption` ports it,
and `the_share_asks_the_bound_of_its_own_caption` refutes the second claim for it at 1,025 letters.
SPEC-132's record reads its A17 red at a share that took every caption,
`docs/red-first/SPEC-132.md:41`.
-/

namespace Formal.CaptionBound

/-- `PhotoError`'s variants, in the order `photo.rs` declares them. -/
inductive PhotoError where
  | format
  | tooLarge
  | dimensions
  | ratio
  | caption
  | fileId
  deriving DecidableEq, Repr

/-- `MAX_CAPTION_UTF16`. -/
def maxCaptionUtf16 : Nat := 1024

/-- The standard library's `char::encode_utf16`: one unit when `code & 0xFFFF == code`, else the
surrogate pair of `code - 0x10000`. -/
def encodeUtf16 (c : Char) : List Nat :=
  if c.val &&& 0xFFFF = c.val then
    [c.val.toNat]
  else
    let code := c.val.toNat - 0x10000
    [0xD800 ||| (code >>> 10), 0xDC00 ||| (code &&& 0x3FF)]

/-- `caption.encode_utf16().count()`: the length of every character's units. -/
def countUnits (caption : List Char) : Nat :=
  (caption.flatMap encodeUtf16).length

/-- `check_caption`: `Caption` when the count is over `MAX_CAPTION_UTF16`, else `Ok`. -/
def checkCaption (caption : List Char) : Except PhotoError Unit :=
  if countUnits caption > maxCaptionUtf16 then .error .caption else .ok ()

/-- `Photo::new`'s order: the image's own verdict (`some` its refusal), then `check_caption`. The
image's bounds read only the bytes, so their verdict is one input. This is not a derived port: no
anchor names `new`, which `photo.rs` defines twice, so the ratchet's W3 reads it as part of the
theorem's key. -/
def photoNew (image : Option PhotoError) (caption : List Char) : Except PhotoError Unit :=
  match image with
  | some e => .error e
  | none => checkCaption caption

/-- What one prepared share came to before the transport answers: refused with the check's error,
or handed to the transport. -/
inductive Share where
  | refused (error : PhotoError)
  | transport
  deriving DecidableEq, Repr

/-- `Router::prepare_share`: the check first, its refusal returned before any transport call. -/
def prepareShare (caption : List Char) : Share :=
  match checkCaption caption with
  | .error e => .refused e
  | .ok () => .transport

/-- A caption's length in UTF-16 code units by the plane rule: one below U+10000, two from it. -/
def utf16Length : List Char → Nat
  | [] => 0
  | c :: rest => (if c.val.toNat < 0x10000 then 1 else 2) + utf16Length rest

/-- The check takes a caption exactly when its UTF-16 length is within 1,024, and refuses every
other with `Caption`. -/
def TakesExactlyTheCaptionsWithinTheLimit (check : List Char → Except PhotoError Unit) : Prop :=
  ∀ caption, check caption = if utf16Length caption ≤ 1024 then .ok () else .error .caption

/-- The photo over an image within its bounds and the share take one set of captions, those within
1,024 units; over it the photo answers `Caption` and the share is refused with `Caption`; and an
image's own refusal stands whatever the caption. -/
def OneSetOfCaptions (photo : Option PhotoError → List Char → Except PhotoError Unit)
    (share : List Char → Share) : Prop :=
  ∀ caption,
    (photo none caption = .ok () ↔ utf16Length caption ≤ 1024) ∧
    (share caption = .transport ↔ utf16Length caption ≤ 1024) ∧
    (1024 < utf16Length caption →
      photo none caption = .error .caption ∧ share caption = .refused .caption) ∧
    (∀ e, photo (some e) caption = .error e)

theorem utf16Length_append (xs ys : List Char) :
    utf16Length (xs ++ ys) = utf16Length xs + utf16Length ys := by
  induction xs with
  | nil => simp [utf16Length]
  | cons c rest ih => simp only [List.cons_append, utf16Length, ih]; omega

theorem utf16Length_replicate (n : Nat) (c : Char) :
    utf16Length (List.replicate n c) = n * (if c.val.toNat < 0x10000 then 1 else 2) := by
  induction n with
  | zero => simp [utf16Length]
  | succ n ih => rw [List.replicate_succ, utf16Length, ih, Nat.succ_mul]; omega

/-- The encoder's branch is the plane rule: `code & 0xFFFF == code` exactly below U+10000. -/
theorem one_unit_exactly_below_the_supplementary_planes (c : Char) :
    c.val &&& 0xFFFF = c.val ↔ c.val.toNat < 0x10000 := by
  rw [← UInt32.toNat_inj, UInt32.toNat_and]
  have hmask : (0xFFFF : UInt32).toNat = 2 ^ 16 - 1 := by decide
  rw [hmask, Nat.and_two_pow_sub_one_eq_mod]
  have hpow : (2 : Nat) ^ 16 = 0x10000 := by decide
  rw [hpow]
  constructor
  · intro h
    have := Nat.mod_lt c.val.toNat (by decide : 0x10000 > 0)
    omega
  · intro h
    exact Nat.mod_eq_of_lt h

theorem encodeUtf16_length (c : Char) :
    (encodeUtf16 c).length = if c.val.toNat < 0x10000 then 1 else 2 := by
  unfold encodeUtf16
  by_cases h : c.val.toNat < 0x10000
  · rw [ite_eq_left ((one_unit_exactly_below_the_supplementary_planes c).mpr h), ite_eq_left h]
    rfl
  · rw [ite_eq_right (mt (one_unit_exactly_below_the_supplementary_planes c).mp h),
      ite_eq_right h]
    rfl

theorem countUnits_eq_utf16Length (caption : List Char) :
    countUnits caption = utf16Length caption := by
  induction caption with
  | nil => rfl
  | cons c rest ih =>
    unfold countUnits at ih ⊢
    rw [List.flatMap_cons, List.length_append, ih, encodeUtf16_length]
    simp only [utf16Length]

theorem the_check_takes_a_caption_exactly_when_its_utf16_units_fit :
    TakesExactlyTheCaptionsWithinTheLimit checkCaption := by
  intro caption
  unfold checkCaption maxCaptionUtf16
  rw [countUnits_eq_utf16Length]
  split <;> split <;> first | rfl | omega

theorem the_photo_and_the_share_take_one_set_of_captions :
    OneSetOfCaptions photoNew prepareShare := by
  intro caption
  have hcheck := the_check_takes_a_caption_exactly_when_its_utf16_units_fit caption
  by_cases h : utf16Length caption ≤ 1024
  · rw [ite_eq_left h] at hcheck
    refine ⟨?_, ?_, ?_, ?_⟩
    · simp [photoNew, hcheck, h]
    · simp [prepareShare, hcheck, h]
    · intro hover
      omega
    · intro e
      rfl
  · rw [ite_eq_right h] at hcheck
    refine ⟨?_, ?_, ?_, ?_⟩
    · simp [photoNew, hcheck, h]
    · simp [prepareShare, hcheck, h]
    · intro _
      exact ⟨by simp [photoNew, hcheck], by simp [prepareShare, hcheck]⟩
    · intro e
      rfl

/-- Row `S13212`'s mutant: the caption counted by its characters. -/
def checkByCharacters (caption : List Char) : Except PhotoError Unit :=
  if caption.length > maxCaptionUtf16 then .error .caption else .ok ()

theorem a_caption_is_counted_in_utf16_units_not_in_characters :
    ¬ TakesExactlyTheCaptionsWithinTheLimit checkByCharacters := by
  intro hclaim
  have h := hclaim (List.replicate 1023 'a' ++ [Char.ofNat 0x1F600])
  have hlen : (List.replicate 1023 'a' ++ [Char.ofNat 0x1F600]).length = 1024 := by
    simp only [List.length_append, List.length_replicate, List.length_singleton]
  have hunits : utf16Length (List.replicate 1023 'a' ++ [Char.ofNat 0x1F600]) = 1025 := by
    have hs : utf16Length [Char.ofNat 0x1F600] = 2 := by decide
    rw [utf16Length_append, utf16Length_replicate, hs, ite_eq_left (by decide)]
  rw [checkByCharacters, hlen, hunits] at h
  simp [maxCaptionUtf16] at h

/-- Row `S13213`'s mutant: the share asks the bound of the empty caption, whatever its own. -/
def prepareShareAskingTheEmptyCaption (_caption : List Char) : Share :=
  match checkCaption [] with
  | .error e => .refused e
  | .ok () => .transport

theorem the_share_asks_the_bound_of_its_own_caption :
    ¬ OneSetOfCaptions photoNew prepareShareAskingTheEmptyCaption := by
  intro hclaim
  have h := (hclaim (List.replicate 1025 'a')).2.1
  have hshare : prepareShareAskingTheEmptyCaption (List.replicate 1025 'a') = .transport := by
    simp [prepareShareAskingTheEmptyCaption, checkCaption, countUnits, maxCaptionUtf16]
  have hunits : utf16Length (List.replicate 1025 'a') = 1025 := by
    rw [utf16Length_replicate, ite_eq_left (by decide)]
  have := h.mp hshare
  omega

end Formal.CaptionBound

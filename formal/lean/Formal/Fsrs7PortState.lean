-- @phx covers crates/engine-core/src/replay.rs anchor=fold digest=sha256:122756ece51ecabd1976ac9faf79cfcef9f1d26f2de8eeaf65480c9c8dfdb9c8
-- @phx covers crates/engine-core/src/replay.rs anchor=new_state digest=sha256:35a48ac904f8d50abd0d8125ef0a74c55aed72f2957f9b33babd1349b095a4a9
-- @phx covers crates/engine-core/src/replay.rs anchor=same_model digest=sha256:324176ec32f62da3764f9e5c512e07f26c95327905f3f816214c0053eedb99ea
-- @phx covers crates/engine-core/src/scheduler.rs anchor=check_event digest=sha256:2d0a5b91be534d46902e5856a2b462914874917958b7352f44a3e50cb25a53cb
-- @phx covers crates/engine-core/src/replay.rs anchor=checkpoint digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/engine-core/src/replay.rs anchor=restore digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/engine-core/src/replay.rs anchor=take digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/engine-core/src/replay.rs anchor=array digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/engine-core/src/replay.rs anchor=event digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/engine-core/src/replay.rs anchor=CHECKPOINT_MAGIC digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/engine-core/src/replay.rs anchor=CHECKPOINT_FORMAT digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/engine-core/src/replay.rs anchor=HEADER_LEN digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/engine-core/src/replay.rs anchor=EVENT_LEN digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/engine-core/src/replay.rs anchor=FSRS7_VERSION digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx cites #823
-- @phx theorem a_refused_event_leaves_the_state_unchanged ramp=report
-- @phx witness a_fold_that_writes_before_it_checks_keeps_a_refused_event kills=a_refused_event_leaves_the_state_unchanged
-- @phx theorem the_folded_keys_strictly_increase ramp=report
-- @phx witness a_check_that_admits_an_equal_key_folds_it_twice kills=the_folded_keys_strictly_increase
-- @phx theorem restoring_a_checkpoint_gives_the_state_back ramp=report
-- @phx witness a_restore_reading_another_format_refuses_the_new_states_checkpoint kills=restoring_a_checkpoint_gives_the_state_back
-- @phx vectors formal/vectors/fsrs7-port-state.jsonl

/-!
# Formal.Fsrs7PortState

The FSRS-7 implementation of the engine core's scheduling port where it has state (#823, Q1 and
Q3): `Fsrs7`'s `new_state`, `fold`, `same_model`, `checkpoint` and `restore` with the checkpoint's
reader and constants in `crates/engine-core/src/replay.rs`, and the port's `check_event` in
`crates/engine-core/src/scheduler.rs`.

**The claims.** For every parameter vector and every event:
- `a_refused_event_leaves_the_state_unchanged`: a fold that refuses its event, for a state made
  under other parameters, a key not after the last, a grade outside 1 to 4 or an answer time past
  the cap, returns the state it was given.
- `the_folded_keys_strictly_increase`: in every state a model's folds reach from its new state,
  the folded events' keys strictly increase, read as the signed 64-bit integers they are.
- `restoring_a_checkpoint_gives_the_state_back`: the same model restores every such state's
  checkpoint as the state it was, on a target whose largest `usize` is at least the checkpoint's
  length and below 2^64. The checkpoint holds no parameters: `restore` folds into the model's own
  new state, so the claim is per model.

**What is ported.** The state's events, last key and parameter bits, and each fold's checks in
the code's order, with every write after them. A 64-bit key is its bit pattern, below 2^64, read
signed as Rust's `i64` comparison reads it. The state's memo of replayed memory states is reset by
every fold and read only through `replayed`, so it is left out: two states with the same events,
last key and parameters replay alike. The checkpoint is a list of bytes: `to_le_bytes` is `le`,
an integer's low bytes first, and `from_le_bytes` is `readLe`, so the codec is proved over the
naturals, with no bit-vector decision procedure. `restore` keeps the code's order of checks, the
reader's `take`, `array` and `event` reads, and the `usize` arithmetic of the expected length,
whose largest value `usizeMax` is a parameter (64 bits on a desktop target, 32 on wasm).
-/

namespace Formal.Fsrs7PortState

/-- `ANSWER_MS_CAP`. -/
def answerMsCap : Nat := 3600000

/-- A review event: card, note, deck and preset as `i64`, the grade as `u8`, the answer time as
`u32`, the key as `i64` and the study day as `i32`, each as its bit pattern. -/
structure Event where
  card : Fin (2 ^ 64)
  note : Fin (2 ^ 64)
  deck : Fin (2 ^ 64)
  preset : Fin (2 ^ 64)
  grade : Fin (2 ^ 8)
  answerMs : Fin (2 ^ 32)
  key : Fin (2 ^ 64)
  studyDay : Fin (2 ^ 32)
  deriving DecidableEq, Repr

/-- An `i64` bit pattern read signed. -/
def signed (k : Fin (2 ^ 64)) : Int :=
  if k.val < 2 ^ 63 then k.val else (k.val : Int) - 2 ^ 64

/-- `FoldError`. -/
inductive FoldError where
  | otherModel
  | keyNotAfter (last key : Fin (2 ^ 64))
  | gradeOutOfRange (grade : Fin (2 ^ 8))
  | answerTooLong (answerMs : Fin (2 ^ 32))
  deriving DecidableEq, Repr

/-- A fold's answer: `Ok(())` or the error. -/
inductive Folded where
  | ok
  | err (e : FoldError)
  deriving DecidableEq, Repr

/-- `check_event`, refusing a key at or before the last (`strict`) or only one before it. -/
def checkEventWith (strict : Bool) (last : Option (Fin (2 ^ 64))) (ev : Event) : Folded :=
  match last with
  | some l =>
    if (if strict then signed ev.key ≤ signed l else signed ev.key < signed l) then
      .err (.keyNotAfter l ev.key)
    else if ev.grade.val < 1 ∨ 4 < ev.grade.val then .err (.gradeOutOfRange ev.grade)
    else if answerMsCap < ev.answerMs.val then .err (.answerTooLong ev.answerMs)
    else .ok
  | none =>
    if ev.grade.val < 1 ∨ 4 < ev.grade.val then .err (.gradeOutOfRange ev.grade)
    else if answerMsCap < ev.answerMs.val then .err (.answerTooLong ev.answerMs)
    else .ok

/-- `Fsrs7`: the bits of the parameters the model was made under. -/
structure Model where
  parameters : List (Fin (2 ^ 32))
  deriving DecidableEq, Repr

/-- `Fsrs7State`, its memo left out. -/
structure State where
  events : List Event
  last : Option (Fin (2 ^ 64))
  parameters : List (Fin (2 ^ 32))
  deriving DecidableEq, Repr

/-- `new_state`. -/
def newState (f : Model) : State := ⟨[], none, f.parameters⟩

/-- `same_model`. -/
def sameModel (f : Model) (st : State) : Folded :=
  if st.parameters = f.parameters then .ok else .err .otherModel

/-- `fold` on `&mut st`: the state after it, and its answer. With `checkFirst` the checks come
before the writes, as the code has them; without, the event is written first. -/
def foldWith (checkFirst strict : Bool) (f : Model) (st : State) (ev : Event) : State × Folded :=
  let pushed : State := ⟨st.events ++ [ev], some ev.key, st.parameters⟩
  let checked :=
    match sameModel f st with
    | .ok => checkEventWith strict st.last ev
    | .err e => .err e
  match checked with
  | .ok => (pushed, .ok)
  | .err e => (if checkFirst then st else pushed, .err e)

/-- The code's fold. -/
def fold : Model → State → Event → State × Folded := foldWith true true

/-- The states a model's folds reach from its new state. -/
inductive Reachable (fd : Model → State → Event → State × Folded) (f : Model) : State → Prop
  | new : Reachable fd f (newState f)
  | step {st st' : State} {ev : Event} : Reachable fd f st → fd f st ev = (st', .ok) →
      Reachable fd f st'

/-! ## The checkpoint -/

/-- A byte. -/
abbrev Byte := Fin 256

/-- `n`'s low `w` bytes, the lowest first: `to_le_bytes` of a `w`-byte integer's bit pattern. -/
def le : Nat → Nat → List Byte
  | 0, _ => []
  | w + 1, n => ⟨n % 256, Nat.mod_lt _ (by decide)⟩ :: le w (n / 256)

/-- Bytes read back lowest first: `from_le_bytes`, as a bit pattern. -/
def readLe : List Byte → Nat
  | [] => 0
  | b :: bs => b.val + 256 * readLe bs

/-- The bit pattern of `k` bits a natural number's low bits make. -/
def bits (k n : Nat) : Fin (2 ^ k) := ⟨n % 2 ^ k, Nat.mod_lt _ (Nat.two_pow_pos k)⟩

/-- `CHECKPOINT_MAGIC`, `b"DSSP"`. -/
def checkpointMagic : List Byte := [68, 83, 83, 80]

/-- `CHECKPOINT_FORMAT`. -/
def checkpointFormat : Byte := 1

/-- `FSRS7_VERSION`, `b"deck-streak-fsrs7 rev 4bc0a097\0\0"`. -/
def fsrs7Version : List Byte :=
  [100, 101, 99, 107, 45, 115, 116, 114, 101, 97, 107, 45, 102, 115, 114, 115, 55, 32, 114, 101,
    118, 32, 52, 98, 99, 48, 97, 48, 57, 55, 0, 0]

/-- `HEADER_LEN`. -/
def headerLen : Nat := 45

/-- `EVENT_LEN`. -/
def eventLen : Nat := 49

/-- One event's bytes in a checkpoint: card, note, deck and preset in 8 bytes each, the grade in
1, the answer time in 4, the key in 8 and the study day in 4. -/
def encEvent (e : Event) : List Byte :=
  le 8 e.card.val ++ le 8 e.note.val ++ le 8 e.deck.val ++ le 8 e.preset.val ++
    le 1 e.grade.val ++ le 4 e.answerMs.val ++ le 8 e.key.val ++ le 4 e.studyDay.val

/-- `checkpoint`: the magic, the format, the identifier, the event count as a `u64`, then each
event. -/
def checkpoint (st : State) : List Byte :=
  checkpointMagic ++ [checkpointFormat] ++ fsrs7Version ++ le 8 st.events.length ++
    st.events.flatMap encEvent

/-- `LoadError`. -/
inductive LoadError where
  | truncated
  | notACheckpoint
  | otherModel
  | refused (e : FoldError)
  deriving DecidableEq, Repr

/-- `restore`'s answer: the state, or the error. -/
inductive Loaded where
  | ok (st : State)
  | err (e : LoadError)
  deriving DecidableEq, Repr

/-- `Reader::take` (and `Reader::array`, the same read): the next `len` bytes and the rest, or
`none` when fewer remain, as `split_at_checked` answers. -/
def takeBytes (len : Nat) (bs : List Byte) : Option (List Byte × List Byte) :=
  if len ≤ bs.length then some (bs.take len, bs.drop len) else none

/-- A `w`-byte little-endian integer: `from_le_bytes(self.array()?)`. -/
def field (w : Nat) (bs : List Byte) : Option (Nat × List Byte) :=
  (takeBytes w bs).map fun p => (readLe p.1, p.2)

/-- `Reader::event`: the next event, field by field, or `none` when a whole one does not remain. -/
def readEvent (bs : List Byte) : Option (Event × List Byte) := do
  let (card, bs) ← field 8 bs
  let (note, bs) ← field 8 bs
  let (deck, bs) ← field 8 bs
  let (preset, bs) ← field 8 bs
  let (grade, bs) ← field 1 bs
  let (answerMs, bs) ← field 4 bs
  let (key, bs) ← field 8 bs
  let (studyDay, bs) ← field 4 bs
  pure (⟨bits 64 card, bits 64 note, bits 64 deck, bits 64 preset, bits 8 grade, bits 32 answerMs,
    bits 64 key, bits 32 studyDay⟩, bs)

/-- `usize::try_from(count)`, then `checked_mul(EVENT_LEN)`, then `checked_add(HEADER_LEN)`, on a
target whose largest `usize` is `usizeMax`. -/
def expectedLen (usizeMax count : Nat) : Option Nat :=
  if usizeMax < count then none
  else if usizeMax < count * eventLen then none
  else if usizeMax < count * eventLen + headerLen then none
  else some (count * eventLen + headerLen)

/-- `restore`'s loop: each event the reader yields is folded into the state until no whole event
remains, and a refused event ends it. The fuel is the checkpoint's length, more than the events it
holds, so the reader runs out first. -/
def foldEvents (f : Model) : Nat → State → List Byte → Loaded
  | 0, st, _ => .ok st
  | fuel + 1, st, bs =>
    match readEvent bs with
    | none => .ok st
    | some (ev, rest) =>
      match fold f st ev with
      | (st', .ok) => foldEvents f fuel st' rest
      | (_, .err e) => .err (.refused e)

/-- `restore`, its format byte `format`, in the code's order of checks. -/
def restoreWith (format : Byte) (usizeMax : Nat) (f : Model) (bytes : List Byte) : Loaded :=
  if bytes.take 4 ≠ checkpointMagic then .err .notACheckpoint
  else if bytes.length < headerLen then .err .truncated
  else
    match takeBytes 1 (bytes.drop 4) with
    | none => .err .notACheckpoint
    | some (fmt, r) =>
      if fmt ≠ [format] then .err .notACheckpoint
      else
        match takeBytes 32 r with
        | none => .err .truncated
        | some (version, r) =>
          if version ≠ fsrs7Version then .err .otherModel
          else
            match field 8 r with
            | none => .err .truncated
            | some (count, r) =>
              match expectedLen usizeMax count with
              | none => .err .notACheckpoint
              | some expected =>
                if bytes.length < expected then .err .truncated
                else if expected < bytes.length then .err .notACheckpoint
                else foldEvents f bytes.length (newState f) r

/-- The code's `restore`. -/
def restore : Nat → Model → List Byte → Loaded := restoreWith checkpointFormat

/-! ## The claims -/

/-- The refusal claim over a fold. -/
def RefusalRule (fd : Model → State → Event → State × Folded) : Prop :=
  ∀ f st ev st' e, fd f st ev = (st', .err e) → st' = st

/-- The key claim over a fold. -/
def KeysRule (fd : Model → State → Event → State × Folded) : Prop :=
  ∀ f st, Reachable fd f st → (st.events.map fun e => signed e.key).Pairwise (· < ·)

/-- The round-trip claim over a checkpoint writer and a reader, on a target whose largest `usize`
is `usizeMax`: every state a model's folds reach from its new state, whose checkpoint fits in a
`Vec`, is restored by the same model as it was. -/
def RoundTrip (cp : State → List Byte) (rs : Nat → Model → List Byte → Loaded) : Prop :=
  ∀ usizeMax f st, usizeMax < 2 ^ 64 → Reachable fold f st → (cp st).length ≤ usizeMax →
    rs usizeMax f (cp st) = .ok st

/-! ## The proofs -/

theorem a_refused_event_leaves_the_state_unchanged : RefusalRule fold := by
  intro f st ev st' e h
  simp only [fold, foldWith] at h
  split at h
  · simp at h
  · simp only [↓reduceIte, Prod.mk.injEq] at h
    exact h.1.symm

/-- A fold that answers `Ok` writes the event after the last, under the same parameters, and its
key is after the last. -/
theorem fold_ok {f : Model} {st st' : State} {ev : Event} (h : fold f st ev = (st', .ok)) :
    st' = ⟨st.events ++ [ev], some ev.key, st.parameters⟩ ∧
      ∀ l, st.last = some l → signed l < signed ev.key := by
  simp only [fold, foldWith] at h
  split at h
  · rename_i hc
    simp only [Prod.mk.injEq] at h
    refine ⟨h.1.symm, ?_⟩
    intro l hl
    revert hc
    unfold sameModel
    split
    · simp only [hl, checkEventWith, ↓reduceIte]
      split
      · simp
      · rename_i hk
        intro _
        grind
    · simp
  · simp at h

/-- What every reachable state keeps: its keys strictly increase, and each is at or before the
last key. -/
def Keyed (st : State) : Prop :=
  (st.events.map fun e => signed e.key).Pairwise (· < ·) ∧
    ∀ e ∈ st.events, ∃ l, st.last = some l ∧ signed e.key ≤ signed l

theorem reachable_keyed {f : Model} {st : State} (h : Reachable fold f st) : Keyed st := by
  induction h with
  | new => exact ⟨List.Pairwise.nil, fun e he => by simp [newState] at he⟩
  | step _ hf ih =>
    obtain ⟨e1, e2⟩ := fold_ok hf
    subst e1
    obtain ⟨p, b⟩ := ih
    refine ⟨?_, ?_⟩
    · simp only [List.map_append, List.map_cons, List.map_nil, List.pairwise_append,
        List.pairwise_singleton, List.mem_singleton, true_and]
      refine ⟨p, ?_⟩
      intro a ha b' hb
      subst hb
      simp only [List.mem_map] at ha
      obtain ⟨x, hx, rfl⟩ := ha
      obtain ⟨l, hl, hle⟩ := b x hx
      have := e2 l hl
      grind
    · intro x hx
      refine ⟨_, rfl, ?_⟩
      simp only [List.mem_append, List.mem_singleton] at hx
      rcases hx with hx | rfl
      · obtain ⟨l, hl, hle⟩ := b x hx
        have := e2 l hl
        grind
      · exact Int.le_refl _

theorem the_folded_keys_strictly_increase : KeysRule fold := by
  intro f st h
  exact (reachable_keyed h).1

theorem le_length (w n : Nat) : (le w n).length = w := by
  induction w generalizing n with
  | zero => rfl
  | succ w ih => simp [le, ih]

theorem readLe_le (w n : Nat) : readLe (le w n) = n % 256 ^ w := by
  induction w generalizing n with
  | zero => simp [le, readLe, Nat.mod_one]
  | succ w ih =>
    simp only [le, readLe, ih, Nat.pow_succ]
    rw [Nat.mul_comm (256 ^ w) 256, Nat.mod_mul]

theorem bits_readLe_le (k w : Nat) (hk : k = 8 * w) (x : Fin (2 ^ k)) :
    bits k (readLe (le w x.val)) = x := by
  subst hk
  apply Fin.ext
  have h : (256 : Nat) ^ w = 2 ^ (8 * w) := by
    rw [Nat.pow_mul]
  simp only [bits, readLe_le, h, Nat.mod_eq_of_lt x.isLt]

theorem takeBytes_append {l r : List Byte} {k : Nat} (h : l.length = k) :
    takeBytes k (l ++ r) = some (l, r) := by
  subst h
  simp [takeBytes, List.take_left' rfl, List.drop_left' rfl]

theorem field_le (w n : Nat) (r : List Byte) :
    field w (le w n ++ r) = some (readLe (le w n), r) := by
  simp [field, takeBytes_append (le_length w n)]

theorem readEvent_nil : readEvent [] = none := by
  simp [readEvent, field, takeBytes]

theorem readEvent_enc (e : Event) (r : List Byte) : readEvent (encEvent e ++ r) = some (e, r) := by
  cases e
  simp [readEvent, encEvent, List.append_assoc, field_le, bits_readLe_le]

theorem encEvents_length (es : List Event) :
    (es.flatMap encEvent).length = es.length * eventLen := by
  induction es with
  | nil => simp
  | cons e es ih =>
    simp [encEvent, le_length, ih, eventLen]
    omega

/-- Every event folded in turn, or `none` at the first refusal. -/
def foldAll (f : Model) : State → List Event → Option State
  | st, [] => some st
  | st, ev :: es =>
    match fold f st ev with
    | (st', .ok) => foldAll f st' es
    | (_, .err _) => none

theorem foldAll_append (f : Model) (xs ys : List Event) :
    ∀ st, foldAll f st (xs ++ ys) = (foldAll f st xs).bind fun t => foldAll f t ys := by
  induction xs with
  | nil => intro st; simp [foldAll]
  | cons x xs ih =>
    intro st
    rcases h : fold f st x with ⟨st', _ | e⟩
    · simp [foldAll, h, ih]
    · simp [foldAll, h]

/-- A reachable state is its own events folded into the model's new state. -/
theorem reachable_foldAll {f : Model} {st : State} (h : Reachable fold f st) :
    foldAll f (newState f) st.events = some st := by
  induction h with
  | new => rfl
  | @step st st' ev _ hf ih =>
    obtain ⟨e1, _⟩ := fold_ok hf
    have hev : st'.events = st.events ++ [ev] := by rw [e1]
    rw [hev, foldAll_append, ih]
    simp [foldAll, hf]

/-- The restore loop over the bytes of events that all fold. -/
theorem foldEvents_encEvents (f : Model) :
    ∀ (es : List Event) (st st' : State) (fuel : Nat), es.length ≤ fuel →
      foldAll f st es = some st' → foldEvents f fuel st (es.flatMap encEvent) = .ok st'
  | [], st, st', fuel, _, h => by
    simp only [foldAll, Option.some.injEq] at h
    subst h
    cases fuel with
    | zero => rfl
    | succ k => simp [foldEvents, readEvent_nil]
  | e :: es, st, st', fuel, hl, h => by
    cases fuel with
    | zero => simp at hl
    | succ k =>
      rcases hf : fold f st e with ⟨s1, _ | err⟩
      · simp only [foldAll, hf] at h
        simp only [foldEvents, List.flatMap_cons, readEvent_enc, hf]
        exact foldEvents_encEvents f es s1 st' k (by simp at hl; omega) h
      · simp [foldAll, hf] at h

theorem restoring_a_checkpoint_gives_the_state_back : RoundTrip checkpoint restore := by
  intro usizeMax f st hmax hr hlen
  have hbody := encEvents_length st.events
  have hcp : (checkpoint st).length = st.events.length * 49 + 45 := by
    simp [checkpoint, hbody, le_length, checkpointMagic, fsrs7Version, eventLen]
    omega
  have hn : st.events.length < 256 ^ 8 := by
    have : (256 : Nat) ^ 8 = 2 ^ 64 := by decide
    omega
  have hcount : readLe (le 8 st.events.length) = st.events.length := by
    rw [readLe_le, Nat.mod_eq_of_lt hn]
  have hexp : expectedLen usizeMax st.events.length = some (st.events.length * 49 + 45) := by
    have h1 : ¬ usizeMax < st.events.length := by omega
    have h2 : ¬ usizeMax < st.events.length * 49 := by omega
    have h3 : ¬ usizeMax < st.events.length * 49 + 45 := by omega
    simp only [expectedLen, eventLen, headerLen, h1, h2, h3, ↓reduceIte]
  have hloop := foldEvents_encEvents f st.events (newState f) st (checkpoint st).length
    (by omega) (reachable_foldAll hr)
  have htake : (checkpoint st).take 4 = checkpointMagic := by
    simp only [checkpoint, List.append_assoc]
    exact List.take_left' rfl
  have hdrop : (checkpoint st).drop 4 =
      [checkpointFormat] ++ (fsrs7Version ++ (le 8 st.events.length ++
        st.events.flatMap encEvent)) := by
    simp only [checkpoint, List.append_assoc]
    exact List.drop_left' rfl
  have hshort : ¬ (checkpoint st).length < headerLen := by
    simp only [headerLen]
    omega
  have hlt1 : ¬ (checkpoint st).length < st.events.length * 49 + 45 := by omega
  have hlt2 : ¬ st.events.length * 49 + 45 < (checkpoint st).length := by omega
  unfold restore restoreWith
  simp only [htake, hshort, ne_eq, not_true_eq_false, ↓reduceIte, hdrop]
  rw [takeBytes_append (l := [checkpointFormat]) (k := 1) rfl]
  simp only [not_true_eq_false, ↓reduceIte]
  rw [takeBytes_append (by rfl : fsrs7Version.length = 32)]
  simp only [not_true_eq_false, ↓reduceIte]
  rw [field_le, hcount]
  simp only [hexp, hlt1, hlt2, ↓reduceIte]
  exact hloop

/-! ## The witnesses -/

/-- A fold that writes the event before it checks it. -/
def foldWritingFirst : Model → State → Event → State × Folded := foldWith false true

/-- An event of grade 0 at key 5. -/
def gradeZero : Event := ⟨0, 0, 0, 0, 0, 0, 5, 0⟩

/-- A grade-0 event folded into a new state is refused, and the refused event stays written. -/
theorem a_fold_that_writes_before_it_checks_keeps_a_refused_event :
    ¬ RefusalRule foldWritingFirst := by
  intro h
  have := h ⟨[]⟩ (newState ⟨[]⟩) gradeZero _ _ rfl
  revert this
  decide +kernel

/-- A fold whose check refuses only a key before the last. -/
def foldAdmittingAnEqualKey : Model → State → Event → State × Folded := foldWith true false

/-- A Good event at key 5. -/
def goodAtFive : Event := ⟨0, 0, 0, 0, 3, 0, 5, 0⟩

/-- The same event folded twice: both are kept, at equal keys. -/
theorem a_check_that_admits_an_equal_key_folds_it_twice :
    ¬ KeysRule foldAdmittingAnEqualKey := by
  intro h
  have r1 : Reachable foldAdmittingAnEqualKey ⟨[]⟩ ⟨[goodAtFive], some 5, []⟩ :=
    Reachable.step (ev := goodAtFive) Reachable.new (by decide +kernel)
  have r2 : Reachable foldAdmittingAnEqualKey ⟨[]⟩ ⟨[goodAtFive, goodAtFive], some 5, []⟩ :=
    Reachable.step (ev := goodAtFive) r1 (by decide +kernel)
  have := h _ _ r2
  revert this
  decide +kernel

/-- A restore that expects format byte 2. -/
def restoreReadingFormatTwo : Nat → Model → List Byte → Loaded := restoreWith 2

/-- The new state's 45-byte checkpoint, read by a restore that expects another format byte, is
refused as not a checkpoint. -/
theorem a_restore_reading_another_format_refuses_the_new_states_checkpoint :
    ¬ RoundTrip checkpoint restoreReadingFormatTwo := by
  intro h
  have := h 45 ⟨[]⟩ (newState ⟨[]⟩) (by decide) Reachable.new (by decide +kernel)
  revert this
  decide +kernel

end Formal.Fsrs7PortState

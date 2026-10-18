-- @phx covers scripts/mutation-verdict.py anchor=rust_tokens digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers scripts/mutation-verdict.py anchor=block_end digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx vectors formal/vectors/token-progress.jsonl
-- @phx cites #822, #322
-- @phx theorem every_reading_ends_within_its_text ramp=report
-- @phx witness old_violates_on_a_brace kills=every_reading_ends_within_its_text
-- @phx witness old_violates_on_a_word kills=every_reading_ends_within_its_text

/-!
# Formal.TokenProgress

The mutation verdict's Rust tokenizer, `rust_tokens` (`scripts/mutation-verdict.py:513-541`),
which decides which Rust lines the mutation plan reads as test-only (#822, SPEC-480, ADR-497). Each
pass of its loop reads one token, or skips a space or a comment, and its progress guard refuses a
reading whose stop lies at or before its start, naming the position.

**The claim**, one `Prop` over a loop, `Claim`: for every literal reader, every space and word
predicate and every block comment end that lies past its start, and for every text, the loop run
with the text's length plus one passes ends, with its tokens, no more of them than the text has
characters, or with a refusal at a position. It never runs out of passes with its loop still
running.

**What is ported.** The loop, branch for branch: the space branch, the `//` branch (the first
newline at or after the position, else the text's length), the `/*` branch (`block_end`), the
literal reading, the word branch with the word predicate in both its compares, the default of one
character past the start, the guard and the append. A text is a `List Char` and a position an
index into it.

**What is not ported.** `rust_literal`, `block_end` and the two predicates are inputs here, each
`Helpers` field; the claim holds for every one of them whose block comment end lies past its
start, as `block_end` does (`:352-365`: from a `/*` it reads at least two characters, or returns
the text's length). Python's `str.isspace` and `str.isalnum` read Unicode; the claim needs nothing
of them.

The witnesses are the base loop, with no guard, under two rows' mutants: row S48001's `or` for
`and` at `scripts/mutation-verdict.py:532`, which leaves a punctuation reading at its start, read
on `{`; and row S48003's `>=` for `<` at `:534`, which leaves a word's reading at its start, read
on `a`. Each appends a token that ends where it starts on every pass, so the loop never ends:
the release run's six legs read this way (#822).
-/

namespace Formal.TokenProgress

/-- What the loop calls: the literal reader `rust_literal`, the block comment's end `block_end`,
and the space and word predicates, `str.isspace` and `str.isalnum() or == "_"`. -/
structure Helpers where
  literal : List Char → Nat → Option Nat
  blockEnd : List Char → Nat → Nat
  space : Char → Bool
  word : Char → Bool

/-- What a reading of a text comes to: its tokens, a refusal at a position, or a loop that has
not ended when its passes run out. -/
inductive Outcome where
  | tokens (ts : List (Nat × Nat))
  | refused (pos : Nat)
  | unfinished
  deriving DecidableEq, Repr

/-- `text.find("\n", at)`, read over the text from `at` on: the offset of the first newline, or
the rest's length when none is left, so `at` plus it is the newline's position or the text's
length. -/
def lineEnd : List Char → Nat
  | [] => 0
  | c :: cs => if c == '\n' then 0 else lineEnd cs + 1

/-- `while stop < len(text) and word(text[stop]): stop += 1` from `stop = at`, read over the text
from `at` on: how many word characters open it. -/
def wordRun (word : Char → Bool) : List Char → Nat
  | [] => 0
  | c :: cs => if word c then wordRun word cs + 1 else 0

/-- A reading's stop at `a`, where the character `c` and the text after it, `rest`, open no space
and no comment: the literal's end; else, when `c` is a word character, the word's end; else one
character past the start: `stop = at + 1 if stop is None else stop`. -/
def readingStop (h : Helpers) (t : List Char) (a : Nat) (c : Char) (rest : List Char) : Nat :=
  (match h.literal t a with
    | none => if h.word c then some (a + wordRun h.word (c :: rest)) else none
    | some s => some s).getD (a + 1)

/-- The loop that ships, with `fuel` passes left at position `a`, `acc` the tokens read so far:
`while at < len(text)` is a text left at `a`, and each branch is the loop's own, in its order. -/
def guarded (h : Helpers) (t : List Char) : Nat → Nat → List (Nat × Nat) → Outcome
  | 0, _, _ => .unfinished
  | n + 1, a, acc =>
    match t.drop a with
    | [] => .tokens acc
    | c :: rest =>
      if h.space c then guarded h t n (a + 1) acc
      else if c == '/' && rest.head? == some '/' then guarded h t n (a + lineEnd (c :: rest)) acc
      else if c == '/' && rest.head? == some '*' then guarded h t n (h.blockEnd t a) acc
      else
        let stop := readingStop h t a c rest
        if stop ≤ a then .refused a else guarded h t n stop (acc ++ [(a, stop)])

/-- `rust_tokens` over the text `t` with `fuel` passes. -/
def rustTokens (h : Helpers) (fuel : Nat) (t : List Char) : Outcome :=
  guarded h t fuel 0 []

/-- A reading's stop under row S48001's mutant, `if stop is None or (word)`: a reading that opens
no literal enters the word branch whatever its character, and a punctuation character opens no
word, so the stop stays at the start. -/
def readingStopUnderS48001 (h : Helpers) (t : List Char) (a : Nat) (c : Char)
    (rest : List Char) : Nat :=
  let literal := h.literal t a
  (if literal.isNone || h.word c then some (a + wordRun h.word (c :: rest)) else literal).getD
    (a + 1)

/-- The base loop, with no guard, under row S48001's mutant (`or` for `and`,
`scripts/mutation-verdict.py:532`): a reading that opens no literal still enters the word branch,
so a punctuation reading stays at its start. -/
def unguardedUnderS48001 (h : Helpers) (t : List Char) :
    Nat → Nat → List (Nat × Nat) → Outcome
  | 0, _, _ => .unfinished
  | n + 1, a, acc =>
    match t.drop a with
    | [] => .tokens acc
    | c :: rest =>
      if h.space c then unguardedUnderS48001 h t n (a + 1) acc
      else if c == '/' && rest.head? == some '/' then
        unguardedUnderS48001 h t n (a + lineEnd (c :: rest)) acc
      else if c == '/' && rest.head? == some '*' then
        unguardedUnderS48001 h t n (h.blockEnd t a) acc
      else
        let stop := readingStopUnderS48001 h t a c rest
        unguardedUnderS48001 h t n stop (acc ++ [(a, stop)])

/-- `rust_tokens` at the base under row S48001's mutant. -/
def rustTokensUnderS48001 (h : Helpers) (fuel : Nat) (t : List Char) : Outcome :=
  unguardedUnderS48001 h t fuel 0 []

/-- Row S48003's word loop, `while stop >= len(text) and word(text[stop])` from `stop = at`, read
over the text from `at` on: at a token start text is left, so its first compare is false and it
passes no character. -/
def wordRunUnderS48003 (_word : Char → Bool) (_rest : List Char) : Nat :=
  0

/-- A reading's stop under row S48003's mutant: its word loop passes no character. -/
def readingStopUnderS48003 (h : Helpers) (t : List Char) (a : Nat) (c : Char)
    (rest : List Char) : Nat :=
  (match h.literal t a with
    | none => if h.word c then some (a + wordRunUnderS48003 h.word (c :: rest)) else none
    | some s => some s).getD (a + 1)

/-- The base loop, with no guard, under row S48003's mutant (`>=` for `<`,
`scripts/mutation-verdict.py:534`): a word's reading stays at its start. -/
def unguardedUnderS48003 (h : Helpers) (t : List Char) :
    Nat → Nat → List (Nat × Nat) → Outcome
  | 0, _, _ => .unfinished
  | n + 1, a, acc =>
    match t.drop a with
    | [] => .tokens acc
    | c :: rest =>
      if h.space c then unguardedUnderS48003 h t n (a + 1) acc
      else if c == '/' && rest.head? == some '/' then
        unguardedUnderS48003 h t n (a + lineEnd (c :: rest)) acc
      else if c == '/' && rest.head? == some '*' then
        unguardedUnderS48003 h t n (h.blockEnd t a) acc
      else
        let stop := readingStopUnderS48003 h t a c rest
        unguardedUnderS48003 h t n stop (acc ++ [(a, stop)])

/-- `rust_tokens` at the base under row S48003's mutant. -/
def rustTokensUnderS48003 (h : Helpers) (fuel : Nat) (t : List Char) : Outcome :=
  unguardedUnderS48003 h t fuel 0 []

/-- A reading ends within its text: its tokens, no more of them than the text's length `len`, or
a refusal; never a loop still running when its passes run out. -/
def endsWithin (len : Nat) : Outcome → Bool
  | .tokens ts => decide (ts.length ≤ len)
  | .refused _ => true
  | .unfinished => false

/-- The claim of a loop: for every helpers whose block comment end lies past its start, and every
text, the loop run with the text's length plus one passes ends within the text. -/
def Claim (loop : Helpers → Nat → List Char → Outcome) : Prop :=
  ∀ h : Helpers, (∀ t a, h.blockEnd t a > a) →
    ∀ t : List Char, endsWithin t.length (loop h (t.length + 1) t) = true

/-- A `//` comment's line end lies past its start: the comment's first character is no newline. -/
theorem lineEnd_slash (rest : List Char) : lineEnd ('/' :: rest) = lineEnd rest + 1 := by
  simp [lineEnd]

/-- The guarded loop ends within its text from any position, with passes enough for the rest of
the text and one more, and no more tokens read than the position or the text's length. -/
theorem guarded_ends (h : Helpers) (hb : ∀ t a, h.blockEnd t a > a) (t : List Char) :
    ∀ (f a : Nat) (acc : List (Nat × Nat)), 1 ≤ f → t.length + 1 ≤ a + f →
      acc.length ≤ a → acc.length ≤ t.length →
        endsWithin t.length (guarded h t f a acc) = true := by
  intro f
  induction f with
  | zero => intro a acc hf; omega
  | succ n ih =>
    intro a acc _ hf ha hl
    simp only [guarded]
    split
    · simp [endsWithin, hl]
    · rename_i c rest hd
      have hlt : a < t.length := by
        have hlen := congrArg List.length hd
        simp at hlen
        omega
      split
      · exact ih (a + 1) acc (by omega) (by omega) (by omega) hl
      · split
        · rename_i hl1
          have hc : c = '/' := by simp at hl1; exact hl1.1
          subst hc
          rw [lineEnd_slash]
          exact ih _ acc (by omega) (by omega) (by omega) hl
        · split
          · have hgt := hb t a
            exact ih _ acc (by omega) (by omega) (by omega) hl
          · generalize readingStop h t a c rest = stop
            split
            · simp [endsWithin]
            · rename_i hle
              exact ih stop (acc ++ [(a, stop)]) (by omega) (by omega)
                (by simp; omega) (by simp; omega)

/-- The loop that ships ends within every text: its tokens, no more of them than the text's
length, or a refusal at the position where a reading stays. -/
theorem every_reading_ends_within_its_text : Claim rustTokens := by
  intro h hb t
  exact guarded_ends h hb t (t.length + 1) 0 [] (by omega) (by omega) (by simp) (by simp)

/-! ## The witnesses: the release run's runaway mutants, each without the guard -/

/-- The helpers each witness reads: no literal, as `rust_literal` reads a text with no quote,
apostrophe or `#`; a block comment's end two characters on; Lean's whitespace; and a letter, a
digit or an underscore as a word character. -/
def witnessHelpers : Helpers where
  literal := fun _ _ => none
  blockEnd := fun _ a => a + 2
  space := Char.isWhitespace
  word := fun c => c.isAlphanum || c == '_'

/-- Row S48001's mutant at `scripts/mutation-verdict.py:532`, read on `{` without the guard,
appends a token that ends where it starts on every pass, so the claim is false of it. -/
theorem old_violates_on_a_brace : ¬ Claim rustTokensUnderS48001 := by
  intro h
  have ends := h witnessHelpers (fun _ a => by show a + 2 > a; omega) "{".toList
  exact absurd ends (by decide +kernel)

/-- Row S48003's mutant at `scripts/mutation-verdict.py:534`, read on `a` without the guard,
appends a token that ends where it starts on every pass, so the claim is false of it. -/
theorem old_violates_on_a_word : ¬ Claim rustTokensUnderS48003 := by
  intro h
  have ends := h witnessHelpers (fun _ a => by show a + 2 > a; omega) "a".toList
  exact absurd ends (by decide +kernel)

end Formal.TokenProgress

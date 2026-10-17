-- @phx covers crates/agent/tests/runs.rs anchor=prune_pin_problems digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/agent/tests/runs.rs anchor=opens_with_a_shebang digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/agent/tests/verdict.rs anchor=opens_with_a_shebang digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/agent/tests/runs.rs anchor=is_punct digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/agent/tests/runs.rs anchor=name_of digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/agent/tests/runs.rs anchor=tokens_in digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/agent/tests/runs.rs anchor=is_doc_attribute digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/agent/tests/runs.rs anchor=text_of digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/agent/tests/runs.rs anchor=ends_in_the_query_macro digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/agent/tests/runs.rs anchor=opens_with_the_statement digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/agent/tests/runs.rs anchor=statements_run_in digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/agent/tests/runs.rs anchor=code_texts digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx vectors formal/vectors/agent-pin-groups.jsonl
-- @phx cites #474
-- @phx theorem a_source_is_refused_as_a_shebang_exactly_when_it_opens_with_one ramp=report
-- @phx witness a_check_that_reads_the_byte_order_mark_misses_a_shebang kills=a_source_is_refused_as_a_shebang_exactly_when_it_opens_with_one
-- @phx theorem the_run_count_and_the_word_count_descend_into_the_same_groups ramp=report
-- @phx witness a_word_count_that_reads_doc_attributes_parts_from_the_run_count kills=the_run_count_and_the_word_count_descend_into_the_same_groups

/-!
# Formal.AgentPinGroups

The prune pin's reader in `crates/agent/tests/runs.rs` (#474, SPEC-469, ADR-486), ported over a
token tree with the lexer taken as any function from text to an optional token list.

**The claim**, in #474's words: for every source text, after one byte order mark is removed, a
source that opens with `#!` and not `#![` is refused; otherwise the run count and the word count
descend into exactly the same token groups, so no count reads a group the other skips. It is two
theorems, each read from one `Prop`:
- `a_source_is_refused_as_a_shebang_exactly_when_it_opens_with_one`: for every lexer and every
  source, `prunePinProblems` answers `shebang` exactly when `ShebangSpec` holds, which is stated
  over list equations, apart from the port's prefix test;
- `the_run_count_and_the_word_count_descend_into_the_same_groups`: for every token list, the
  groups `statementsRunIn` descends into are the groups `codeTexts` descends into, in order.

**What is ported**, each named after its item, branch for branch. From
`crates/agent/tests/runs.rs`: `opens_with_a_shebang`, `is_punct`, `name_of`, `tokens_in`,
`is_doc_attribute`, `text_of`, `ends_in_the_query_macro` with its closure `is`,
`opens_with_the_statement`, `statements_run_in`, `code_texts` and `prune_pin_problems` with its
word count. From `crates/agent/tests/verdict.rs`: `opens_with_a_shebang`, whose body is the prune
pin's byte for byte, so one port covers both copies. Text is a list of characters. A token is an
identifier with its name as written, its raw prefix included; a punctuation character; a literal
with its kind and its cooked text; or a group with its delimiter and the tokens it holds. Each
walk returns its answer and the groups it descended into, each group named by its path of
positions from the top, from one recursion: a token's step and the list's step are one mutual
recursion over the token tree, as the code's `match` inside its `map` or its `for` is, and the
inner walk starts with no tokens before it, as `tokens[..at]` of a group's own tokens does.

**What is not ported.** The lexer: `prunePinProblems` takes it as a parameter and hands it the
source as it was handed in, the byte order mark included, so both theorems hold whatever tokens a
lexer returns. A literal's kind and cooked text: the token carries them as the lexer's and
`syn`'s answer. What rustc does after it lexes, expanding a macro or evaluating a `cfg` (#444).
The verdict pin's `declarations`, which reads every group (#474 names the prune pin's two counts).

**The witnesses** are variants of this delivery's rows. `prunePinProblemsReadingTheMark` runs the
prefix test on the source itself, the byte order mark kept (row S46901's mutant of the verdict
pin's copy), and misses the shebang of a source of U+FEFF then `#!`.
`codeTextsReadingDocAttributes` is the word count's walk with its doc guard dropped (row S46902's
mutant), and descends into a doc attribute group the run count skips.
-/

namespace Formal.AgentPinGroups

/-- A group's delimiter, as `proc_macro2::Delimiter` names it. -/
inductive Delim where
  | parenthesis
  | bracket
  | brace
  | none
  deriving DecidableEq, Repr

/-- A literal's kind, as `syn::Lit::new` reads it: a string, a byte string, a C string, or any
other literal. -/
inductive LitKind where
  | str
  | byteStr
  | cStr
  | other
  deriving DecidableEq, Repr

/-- A token tree, as `proc_macro2::TokenTree` holds it: an identifier with its name as written,
its raw prefix included; a punctuation character; a literal with its kind and its cooked text; or
a delimited group with the tokens it holds. -/
inductive Tok where
  | ident (name : List Char)
  | punct (ch : Char)
  | lit (kind : LitKind) (cooked : List Char)
  | group (delim : Delim) (inner : List Tok)

/-- `str::starts_with` over characters: whether `text` opens with `pre`. -/
def startsWith : List Char → List Char → Bool
  | _, [] => true
  | [], _ :: _ => false
  | c :: text, p :: pre => c == p && startsWith text pre

/-- `str::strip_prefix` of one character: the text after it, or nothing when the text does not
open with it. -/
def stripPrefix (ch : Char) : List Char → Option (List Char)
  | c :: rest => if c == ch then some rest else none
  | [] => none

/-- `opens_with_a_shebang`, in both pins: one byte order mark is removed if it heads the source,
then the text opens with `#!` and not with `#![`. -/
def opensWithAShebang (source : List Char) : Bool :=
  let text := (stripPrefix '﻿' source).getD source
  startsWith text ['#', '!'] && !startsWith text ['#', '!', '[']

/-- `is_punct`: whether `token` is the punctuation `ch`. -/
def isPunct : Option Tok → Char → Bool
  | some (.punct c), ch => c == ch
  | _, _ => false

/-- `name_of`: an identifier's name as rustc resolves it, a leading `r#` removed. -/
def nameOf : List Char → List Char
  | 'r' :: '#' :: rest => rest
  | name => name

/-- `tokens_in`: the tokens a delimited group holds. It is reducible, so a walk that descends
through it stays structural. -/
abbrev tokensIn (inner : List Tok) : List Tok := inner

/-- `text_of`: the cooked text of a string, byte-string or C-string literal, and nothing for any
other literal. -/
def textOf : LitKind → List Char → Option (List Char)
  | .str, cooked => some cooked
  | .byteStr, cooked => some cooked
  | .cStr, cooked => some cooked
  | .other, _ => none

/-- `is_doc_attribute`: whether a group of `delim` holding `inner`, after the tokens `before`, is
a doc attribute. When `before` holds at least two tokens and its last is `!`, the one before that
must be `#`; otherwise its last must be `#`. Then the delimiter is a bracket and the group's first
token is an identifier whose name, its raw prefix removed, is `doc`. -/
def isDocAttribute (before : List Tok) (delim : Delim) (inner : List Tok) : Bool :=
  let hash :=
    match before.reverse, isPunct before.getLast? '!' with
    | _ :: beforeBang :: _, true => isPunct (some beforeBang) '#'
    | _, _ => isPunct before.getLast? '#'
  delim == .bracket && hash &&
    match (tokensIn inner).head? with
    | some (.ident doc) => nameOf doc == ['d', 'o', 'c']
    | _ => false

/-- The closure `is` of `ends_in_the_query_macro`: an identifier whose name, its raw prefix
removed, is `text`, or a punctuation character written as `text`; a group or a literal never. -/
def isWritten : Tok → List Char → Bool
  | .ident name, text => nameOf name == text
  | .punct ch, text => [ch] == text
  | .group _ _, _ => false
  | .lit _ _, _ => false

/-- `ends_in_the_query_macro`: whether at least five tokens stand before, and the last five are
`sqlx`, `:`, `:`, `query` and `!`. -/
def endsInTheQueryMacro (before : List Tok) : Bool :=
  decide (before.length ≥ 5) &&
    (List.zip [['s', 'q', 'l', 'x'], [':'], [':'], ['q', 'u', 'e', 'r', 'y'], ['!']]
        (before.drop (before.length - 5))).all
      fun (text, token) => isWritten token text

/-- `PRUNE`: the tested statement. -/
def prune : List Char := "DELETE FROM agent_runs WHERE created_at < ?1".toList

/-- `opens_with_the_statement`: whether a group's tokens open with a string literal whose cooked
text is the statement, then nothing or a comma. -/
def opensWithTheStatement (inner : List Tok) : Bool :=
  match tokensIn inner with
  | .lit kind statement :: rest =>
    (kind == .str && statement == prune) &&
      match rest.head? with
      | none => true
      | some (.punct comma) => comma == ','
      | some _ => false
  | _ => false

mutual
/-- One token's step of `statements_run_in`, the `match` its `map` applies at `at`: a doc
attribute group counts nothing and is not entered; any other group is entered at its path, its
inner walk's runs counted, and is one run more when the tokens before it end in `sqlx::query!` and
it opens with the statement; any other token counts nothing. -/
def statementsRunInToken (path : List Nat) (before : List Tok) («at» : Nat) :
    Tok → Nat × List (List Nat)
  | .group delim inner =>
    if isDocAttribute before delim inner then (0, [])
    else
      let inside := statementsRunIn (path ++ [«at»]) [] 0 (tokensIn inner)
      (inside.1 + (if endsInTheQueryMacro before && opensWithTheStatement inner then 1 else 0),
        (path ++ [«at»]) :: inside.2)
  | .ident _ | .punct _ | .lit _ _ => (0, [])

/-- `statements_run_in`: the runs the tokens hold at any depth, summed over each token's step with
the tokens before it, and the paths of the groups it descended into, in order. -/
def statementsRunIn (path : List Nat) (before : List Tok) («at» : Nat) :
    List Tok → Nat × List (List Nat)
  | [] => (0, [])
  | token :: rest =>
    let here := statementsRunInToken path before «at» token
    let after := statementsRunIn path (before ++ [token]) («at» + 1) rest
    (here.1 + after.1, here.2 ++ after.2)
end

mutual
/-- One token's step of `code_texts`, the `match` its `for` runs at `at`: an identifier pushes
its name, a literal its cooked text if it has one, a group that is not a doc attribute is entered
at its path, and a doc attribute group and a punctuation push nothing. -/
def codeTextsOfToken (path : List Nat) (before : List Tok) («at» : Nat) :
    Tok → List (List Char) × List (List Nat)
  | .ident name => ([nameOf name], [])
  | .lit kind cooked => ((textOf kind cooked).toList, [])
  | .group delim inner =>
    if !isDocAttribute before delim inner then
      let inside := codeTexts (path ++ [«at»]) [] 0 (tokensIn inner)
      (inside.1, (path ++ [«at»]) :: inside.2)
    else ([], [])
  | .punct _ => ([], [])

/-- `code_texts`: every text the tokens write at any depth, in order, and the paths of the groups
it descended into, in order. -/
def codeTexts (path : List Nat) (before : List Tok) («at» : Nat) :
    List Tok → List (List Char) × List (List Nat)
  | [] => ([], [])
  | token :: rest =>
    let here := codeTextsOfToken path before «at» token
    let after := codeTexts path (before ++ [token]) («at» + 1) rest
    (here.1 ++ after.1, here.2 ++ after.2)
end

/-- Whether `ch` belongs to a word: an ASCII letter or digit, or `_`. -/
def isWordChar (ch : Char) : Bool :=
  ch.isAlphanum || ch == '_'

/-- The word count over one text: the text split at every character that is not a word
character, and the pieces equal to `delete`, ignoring ASCII case, counted. -/
def deleteWordsIn (text : List Char) : Nat :=
  ((text.splitOnP fun ch => !isWordChar ch).filter
    fun word => word.map Char.toLower == ['d', 'e', 'l', 'e', 't', 'e']).length

/-- `keywords` in `prune_pin_problems`: the word count summed over every text the code writes. -/
def keywords (texts : List (List Char)) : Nat :=
  (texts.map deleteWordsIn).sum

/-- What `prune_pin_problems` reads of a source: the shebang refusal, a source that does not lex,
or the run count and the word count, from which it writes its sentences. -/
inductive Outcome where
  | shebang
  | doesNotLex
  | read (run words : Nat)
  deriving DecidableEq, Repr

/-- `prune_pin_problems`, under any lexer `lex`: the shebang check first, then `lex` over the
source as it was handed in, then both counts over the one token list it returns. -/
def prunePinProblems (lex : List Char → Option (List Tok)) (source : List Char) : Outcome :=
  if opensWithAShebang source then .shebang
  else
    match lex source with
    | none => .doesNotLex
    | some tokens =>
      .read (statementsRunIn [] [] 0 tokens).1 (keywords (codeTexts [] [] 0 tokens).1)

/-- A source a pin must refuse as a shebang, over list equations: the text after one U+FEFF head
is removed, the source itself when no U+FEFF heads it, equals `#` then `!` then some rest, and
equals `#` then `!` then `[` then no rest. -/
def ShebangSpec (source : List Char) : Prop :=
  ∃ text, (source = '﻿' :: text ∨ (text = source ∧ ∀ t, source ≠ '﻿' :: t)) ∧
    (∃ rest, text = '#' :: '!' :: rest) ∧ ∀ rest', text ≠ '#' :: '!' :: '[' :: rest'

/-- The claim, stated once: whatever the lexer, the pin refuses a source as a shebang exactly when
the source is one. -/
def RefusesExactlyTheShebangs (pin : (List Char → Option (List Tok)) → List Char → Outcome) :
    Prop :=
  ∀ lex source, pin lex source = .shebang ↔ ShebangSpec source

/-- The claim, stated once: over every token list, the run count's groups are the word count's,
in order. -/
def SameGroups (run : List Nat → List Tok → Nat → List Tok → Nat × List (List Nat))
    (word : List Nat → List Tok → Nat → List Tok → List (List Char) × List (List Nat)) : Prop :=
  ∀ ts, (run [] [] 0 ts).2 = (word [] [] 0 ts).2

theorem startsWith_iff : ∀ text pre : List Char,
    startsWith text pre = true ↔ ∃ rest, text = pre ++ rest
  | text, [] => by simp [startsWith]
  | [], _ :: _ => by simp [startsWith]
  | c :: text, p :: pre => by
    simp only [startsWith, Bool.and_eq_true, beq_iff_eq, startsWith_iff text pre,
      List.cons_append, List.cons.injEq]
    constructor
    · rintro ⟨rfl, rest, h⟩
      exact ⟨rest, rfl, h⟩
    · rintro ⟨rest, h1, h2⟩
      exact ⟨h1, rest, h2⟩

theorem afterTheMark_iff (source text : List Char) :
    (source = '\uFEFF' :: text ∨ (text = source ∧ ∀ t, source ≠ '\uFEFF' :: t)) ↔
      (stripPrefix '\uFEFF' source).getD source = text := by
  cases source with
  | nil =>
    simp only [stripPrefix, Option.getD_none]
    constructor
    · rintro (h | ⟨rfl, _⟩)
      · cases h
      · rfl
    · rintro rfl
      exact Or.inr ⟨rfl, fun _ h => by cases h⟩
  | cons c rest =>
    by_cases hc : c = '\uFEFF'
    · subst hc
      have hs : stripPrefix '\uFEFF' ('\uFEFF' :: rest) = some rest := by simp [stripPrefix]
      rw [hs, Option.getD_some]
      constructor
      · rintro (h | ⟨_, h⟩)
        · exact (List.cons.inj h).2
        · exact absurd rfl (h rest)
      · rintro rfl
        exact Or.inl rfl
    · have hs : stripPrefix '\uFEFF' (c :: rest) = none := by simp [stripPrefix, hc]
      rw [hs, Option.getD_none]
      constructor
      · rintro (h | ⟨rfl, _⟩)
        · exact absurd (List.cons.inj h).1 hc
        · rfl
      · rintro rfl
        exact Or.inr ⟨rfl, fun _ h => hc (List.cons.inj h).1⟩

theorem opensWithAShebang_iff (source : List Char) :
    opensWithAShebang source = true ↔ ShebangSpec source := by
  unfold opensWithAShebang ShebangSpec
  generalize hT : (stripPrefix '\uFEFF' source).getD source = T
  simp only [afterTheMark_iff, hT]
  constructor
  · intro h
    simp only [Bool.and_eq_true, Bool.not_eq_true'] at h
    obtain ⟨h1, h2⟩ := h
    obtain ⟨rest, hrest⟩ := (startsWith_iff T _).mp h1
    refine ⟨T, rfl, ⟨rest, hrest⟩, fun rest' h' => ?_⟩
    have h3 : startsWith T ['#', '!', '['] = true := (startsWith_iff T _).mpr ⟨rest', h'⟩
    rw [h2] at h3
    cases h3
  · rintro ⟨text, rfl, ⟨rest, hrest⟩, hno⟩
    simp only [Bool.and_eq_true, Bool.not_eq_true']
    refine ⟨(startsWith_iff _ _).mpr ⟨rest, hrest⟩, ?_⟩
    cases h : startsWith T ['#', '!', '['] with
    | false => rfl
    | true =>
      obtain ⟨r, hr⟩ := (startsWith_iff _ _).mp h
      exact absurd hr (hno r)

theorem a_source_is_refused_as_a_shebang_exactly_when_it_opens_with_one :
    RefusesExactlyTheShebangs prunePinProblems := by
  intro lex source
  rw [← opensWithAShebang_iff]
  unfold prunePinProblems
  cases h : opensWithAShebang source with
  | true => simp
  | false =>
    simp only [Bool.false_eq_true, ↓reduceIte, iff_false]
    split <;> simp

/-- The variant of row S46901: the prefix test on the source itself, the byte order mark kept. -/
def opensWithAShebangReadingTheMark (source : List Char) : Bool :=
  startsWith source ['#', '!'] && !startsWith source ['#', '!', '[']

/-- `prunePinProblems` with the check that reads the byte order mark. -/
def prunePinProblemsReadingTheMark (lex : List Char → Option (List Tok)) (source : List Char) :
    Outcome :=
  if opensWithAShebangReadingTheMark source then .shebang
  else
    match lex source with
    | none => .doesNotLex
    | some tokens =>
      .read (statementsRunIn [] [] 0 tokens).1 (keywords (codeTexts [] [] 0 tokens).1)

theorem a_check_that_reads_the_byte_order_mark_misses_a_shebang :
    ¬ RefusesExactlyTheShebangs prunePinProblemsReadingTheMark := by
  intro h
  have hspec : ShebangSpec ['\uFEFF', '#', '!'] :=
    ⟨['#', '!'], Or.inl rfl, ⟨[], rfl⟩, fun _ h => by cases h⟩
  have h1 := (h (fun _ => none) ['\uFEFF', '#', '!']).mpr hspec
  revert h1
  decide

mutual
theorem statementsRunInToken_groups (path : List Nat) (before : List Tok) («at» : Nat) :
    ∀ token : Tok,
      (statementsRunInToken path before «at» token).2 = (codeTextsOfToken path before «at» token).2
  | .group delim inner => by
    have hin := statementsRunIn_groups (path ++ [«at»]) [] 0 inner
    simp only [statementsRunInToken, codeTextsOfToken]
    cases isDocAttribute before delim inner <;> simp [hin]
  | .ident _ => by simp [statementsRunInToken, codeTextsOfToken]
  | .punct _ => by simp [statementsRunInToken, codeTextsOfToken]
  | .lit _ _ => by simp [statementsRunInToken, codeTextsOfToken]

theorem statementsRunIn_groups : ∀ (path : List Nat) (before : List Tok) («at» : Nat)
    (ts : List Tok), (statementsRunIn path before «at» ts).2 = (codeTexts path before «at» ts).2
  | _, _, _, [] => by simp [statementsRunIn, codeTexts]
  | path, before, «at», token :: rest => by
    have hhere := statementsRunInToken_groups path before «at» token
    have hafter := statementsRunIn_groups path (before ++ [token]) («at» + 1) rest
    simp only [statementsRunIn, codeTexts, hhere, hafter]
end

theorem the_run_count_and_the_word_count_descend_into_the_same_groups :
    SameGroups statementsRunIn codeTexts :=
  fun ts => statementsRunIn_groups [] [] 0 ts

mutual
/-- The variant of row S46902: one token's step of `code_texts` with its doc guard dropped, so
every group is entered. -/
def codeTextsOfTokenReadingDocAttributes (path : List Nat) (_before : List Tok) («at» : Nat) :
    Tok → List (List Char) × List (List Nat)
  | .ident name => ([nameOf name], [])
  | .lit kind cooked => ((textOf kind cooked).toList, [])
  | .group _ inner =>
    let inside := codeTextsReadingDocAttributes (path ++ [«at»]) [] 0 (tokensIn inner)
    (inside.1, (path ++ [«at»]) :: inside.2)
  | .punct _ => ([], [])

/-- `code_texts` with its doc guard dropped. -/
def codeTextsReadingDocAttributes (path : List Nat) (before : List Tok) («at» : Nat) :
    List Tok → List (List Char) × List (List Nat)
  | [] => ([], [])
  | token :: rest =>
    let here := codeTextsOfTokenReadingDocAttributes path before «at» token
    let after := codeTextsReadingDocAttributes path (before ++ [token]) («at» + 1) rest
    (here.1 ++ after.1, here.2 ++ after.2)
end

theorem a_word_count_that_reads_doc_attributes_parts_from_the_run_count :
    ¬ SameGroups statementsRunIn codeTextsReadingDocAttributes := by
  intro h
  have h1 := h [.punct '#', .group .bracket [.ident "doc".toList, .punct '=', .lit .str "x".toList]]
  revert h1
  decide +kernel

end Formal.AgentPinGroups

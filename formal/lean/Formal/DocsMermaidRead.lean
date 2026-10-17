-- @phx vectors formal/vectors/docs-mermaid-read.jsonl
-- @phx cites #493, #383
-- @phx theorem every_line_that_may_open_a_mermaid_fence_is_read_or_refused_by_name ramp=report
-- @phx witness the_reader_without_its_opener_rule_leaves_a_mermaid_line_unnamed kills=every_line_that_may_open_a_mermaid_fence_is_read_or_refused_by_name
-- @phx witness the_reader_without_its_info_rule_leaves_a_mermaid_fence_unnamed kills=every_line_that_may_open_a_mermaid_fence_is_read_or_refused_by_name

/-!
# Formal.DocsMermaidRead

The docs check's reader (#493, #383), `readMermaid` in `web/app/src/lib/docs-mermaid-read.js`, by
its fence rules, over text read as characters. A bare `:n` below is a line of that file.

**What is ported.** The line split (`:197`, `LINE_END` at `:89`); the opener pattern (`OPENER`,
`:104`, with `FENCE_PREFIX` at `:58`); the pattern by which an info string may hold the word
(`MERMAID_INFO`, `:66`); the word alone with blanks around it (`MERMAID_WORD`, `:69`); the
not-blank test (`:291`); the classification of each fence the parse opened (`:286-294`); the rule
that names every line that may open a `mermaid` fence and was not read (`:295-301`); and the sort
by line that keeps the order of equal lines (`:302`).

**What is an argument.** The parse's fences (each its first line, its info string as the line holds
it, and its text) and the refusals of the reader's other rules (characters, depth and raw HTML).
The claim holds for every value of them, so it rests on no fact about the parser, as the chest
entry's claim holds for every draw.

**The claim**, `NamedReading`, for every text, fence list and list of the other rules' refusals:
every line that may open a `mermaid` fence is the first line of a block read or is refused at that
line by name; every fence whose info string may hold the word is read with its own line and text
or refused at its line by one of the two fence names; every block read is a fence whose info
string is the word alone and whose text is not blank; every refusal is one of the other rules' or
carries a name whose fact holds at its line; every refusal of the other rules is kept; and the
refusals are in line order.

**The witnesses** are the port with one rule removed, each at one text the reader meets: without
the opener rule, at a `mermaid` opener inside a tilde fence's body; without the info rule, at a
fence whose info string is `Mermaid`.
-/

namespace Formal.DocsMermaidRead

/-- A fence the parse opened (`:168`): its first line, counted from 1, its info string as the line
holds it, and its text. -/
structure Fence where
  line : Nat
  info : List Char
  literal : List Char
  deriving DecidableEq, Repr

/-- A block read (`:177`): its first line and the text the diagram is drawn from. -/
structure Block where
  line : Nat
  source : List Char
  deriving DecidableEq, Repr

/-- A form the reader refuses (`:180`): its line and its name. -/
structure Refusal where
  line : Nat
  form : List Char
  deriving DecidableEq, Repr

/-- What the reader returns (`:184`). -/
structure Reading where
  blocks : List Block
  refused : List Refusal
  deriving DecidableEq, Repr

/-- `:289`. -/
def infoForm : List Char := "a fence whose info string is not `mermaid` alone".toList

/-- `:291`. -/
def emptyForm : List Char := "a `mermaid` block that holds no diagram".toList

/-- `:299`. -/
def openerForm : List Char :=
  "a line that may open a `mermaid` fence and is not read as one".toList

/-- The line split (`:197`): at a carriage return and line feed, a carriage return or a line feed,
in that order (`LINE_END`, `:89`); `acc` holds the line read so far, reversed. -/
def splitLinesFrom : List Char → List Char → List (List Char)
  | acc, [] => [acc.reverse]
  | acc, '\r' :: '\n' :: rest => acc.reverse :: splitLinesFrom [] rest
  | acc, '\r' :: rest => acc.reverse :: splitLinesFrom [] rest
  | acc, '\n' :: rest => acc.reverse :: splitLinesFrom [] rest
  | acc, c :: rest => splitLinesFrom (c :: acc) rest

/-- `text.split(LINE_END)` (`:197`); the empty text is one empty line. -/
def splitLines (text : List Char) : List (List Char) :=
  splitLinesFrom [] text

/-- `[0-9]`. -/
def isDigit (c : Char) : Bool :=
  '0' ≤ c && c ≤ '9'

/-- `[ \t]`. -/
def isBlank (c : Char) : Bool :=
  c == ' ' || c == '\t'

/-- The lookahead `(?=[ \t])`: the next character is a space or a tab. -/
def blankNext : List Char → Bool
  | c :: _ => isBlank c
  | [] => false

mutual
/-- `FENCE_PREFIX` (`:58`), skipped greedily, its alternatives in order: a space, a tab or `>`;
`-`, `*` or `+` before a space or tab; one to nine digits, then `.` or `)` before a space or tab.
It returns the line from the first character the prefix does not take. -/
def skipPrefix : List Char → List Char
  | [] => []
  | c :: rest =>
    if c == ' ' || c == '\t' || c == '>' then skipPrefix rest
    else if (c == '-' || c == '*' || c == '+') && blankNext rest then skipPrefix rest
    else if isDigit c then skipDigits (c :: rest) 1 rest
    else c :: rest

/-- The digits alternative, `[0-9]{1,9}[.)](?=[ \t])`, after `n` digits: a tenth digit, or no `.`
or `)` before a space or tab, fails it, and the prefix stops at its first digit, `start`. -/
def skipDigits (start : List Char) (n : Nat) : List Char → List Char
  | [] => start
  | c :: rest =>
    if isDigit c then (if n < 9 then skipDigits start (n + 1) rest else start)
    else if (c == '.' || c == ')') && blankNext rest then skipPrefix rest
    else start
end

/-- The run of `ch` a list begins with: its length and the rest. -/
def runOf (ch : Char) : List Char → Nat × List Char
  | [] => (0, [])
  | c :: rest =>
    if c == ch then
      let run := runOf ch rest
      (run.1 + 1, run.2)
    else (0, c :: rest)

/-- The characters `.` does not match: a line feed, a carriage return, U+2028 and U+2029. -/
def lineTerminator (c : Char) : Bool :=
  c == '\n' || c == '\r' || c == ' ' || c == ' '

/-- `OPENER` (`:104`), `^FENCE_PREFIX(?:`{3,}|~{3,})(.*)$`: the prefix, then a maximal run of three
or more backticks or of three or more tildes, then the info string, the rest of the line, which
`.` must match whole. -/
def opener (line : List Char) : Option (List Char) :=
  match skipPrefix line with
  | [] => none
  | c :: rest =>
    if c == '`' || c == '~' then
      let run := runOf c (c :: rest)
      if 3 ≤ run.1 && !(run.2.any lineTerminator) then some run.2 else none
    else none

/-- The word. -/
def word : List Char := "mermaid".toList

/-- `c` matches `p` under `/i` with no `u` flag: `p` itself, or `p`'s capital when `c` is in A-Z. -/
def foldsTo (p c : Char) : Bool :=
  c == p || ('A' ≤ c && c ≤ 'Z' && c.toNat + 32 == p.toNat)

/-- The list begins with `w`, each character folded. -/
def startsFolded : List Char → List Char → Bool
  | [], _ => true
  | _ :: _, [] => false
  | p :: ps, c :: cs => foldsTo p c && startsFolded ps cs

/-- The list holds `w` at some place, each character folded. -/
def holdsFolded (w : List Char) : List Char → Bool
  | [] => startsFolded w []
  | c :: cs => startsFolded w (c :: cs) || holdsFolded w cs

/-- `MERMAID_INFO` (`:66`), `/mermaid|&/i`: the word with A-Z folded to a-z, or a `&`. -/
def mermaidInfo (info : List Char) : Bool :=
  holdsFolded word info || info.contains '&'

/-- `[ \t]*`, skipped. -/
def dropBlanks : List Char → List Char
  | [] => []
  | c :: rest => if isBlank c then dropBlanks rest else c :: rest

/-- The rest of the list after `w`, if it begins with `w`. -/
def afterExact : List Char → List Char → Option (List Char)
  | [], cs => some cs
  | _ :: _, [] => none
  | p :: ps, c :: cs => if p == c then afterExact ps cs else none

/-- `MERMAID_WORD` (`:69`), `/^[ \t]*mermaid[ \t]*$/`: blanks, the word, blanks. -/
def mermaidWord (info : List Char) : Bool :=
  match afterExact word (dropBlanks info) with
  | some rest => rest.all isBlank
  | none => false

/-- `\s` under the `u` flag: a tab, a line feed, a line tabulation, a form feed, a carriage
return, a space, U+00A0, U+1680, U+2000 to U+200A, U+2028, U+2029, U+202F, U+205F, U+3000 and
U+FEFF. -/
def jsBlank (c : Char) : Bool :=
  let n := c.toNat
  n == 9 || n == 10 || n == 11 || n == 12 || n == 13 || n == 32 || n == 0xA0 || n == 0x1680 ||
    (0x2000 ≤ n && n ≤ 0x200A) || n == 0x2028 || n == 0x2029 || n == 0x202F || n == 0x205F ||
    n == 0x3000 || n == 0xFEFF

/-- `/\S/u.test(literal)` (`:291`): some character outside that set. -/
def notBlank (literal : List Char) : Bool :=
  literal.any fun c => !jsBlank c

/-- Each fence the parse opened, in order (`:286-294`): a fence whose info string may hold the word
and is not the word alone is refused by `infoForm` (`:289`); one whose info string is the word
alone is read, unless its text is blank, when it is refused by `emptyForm` (`:291-292`); any
other fence is not the reader's. It returns the blocks read and the refusals, each in fence
order. -/
def classify : List Fence → List Block × List Refusal
  | [] => ([], [])
  | f :: fs =>
    let rest := classify fs
    if mermaidInfo f.info && !mermaidWord f.info then (rest.1, ⟨f.line, infoForm⟩ :: rest.2)
    else if mermaidWord f.info then
      if !notBlank f.literal then (rest.1, ⟨f.line, emptyForm⟩ :: rest.2)
      else (⟨f.line, f.literal⟩ :: rest.1, rest.2)
    else rest

/-- The opener rule (`:295-301`) from line index `n`: each line `OPENER` matches whose info string
may hold the word, and which is not the first line of a block read, is refused by
`openerForm`. -/
def openerRuleFrom (bs : List Block) : Nat → List (List Char) → List Refusal
  | _, [] => []
  | n, l :: ls =>
    match opener l with
    | some info =>
      if mermaidInfo info && !(bs.any fun b => b.line == n + 1) then
        ⟨n + 1, openerForm⟩ :: openerRuleFrom bs (n + 1) ls
      else openerRuleFrom bs (n + 1) ls
    | none => openerRuleFrom bs (n + 1) ls

/-- The opener rule over every line of the text (`:295-301`). -/
def openerRule (lines : List (List Char)) (bs : List Block) : List Refusal :=
  openerRuleFrom bs 0 lines

/-- One refusal put before the first of a sorted list whose line is not below its own. -/
def insertByLine (x : Refusal) : List Refusal → List Refusal
  | [] => [x]
  | y :: ys => if x.line ≤ y.line then x :: y :: ys else y :: insertByLine x ys

/-- `refused.sort((a, b) => a.line - b.line)` (`:302`): a stable sort by line. -/
def byLine : List Refusal → List Refusal
  | [] => []
  | x :: xs => insertByLine x (byLine xs)

/-- The reader (`:192-303`), given the other rules' refusals `others` and the parse's `fences`. -/
def readMermaid (others : List Refusal) (text : List Char) (fences : List Fence) : Reading :=
  let (bs, rs) := classify fences
  { blocks := bs, refused := byLine (others ++ rs ++ openerRule (splitLines text) bs) }

/-- The first witness's port: the reader with lines 295-301 removed. -/
def readMermaidWithoutOpenerRule (others : List Refusal) (_text : List Char)
    (fences : List Fence) : Reading :=
  let (bs, rs) := classify fences
  { blocks := bs, refused := byLine (others ++ rs) }

/-- The classification with the refusal at line 289 removed. -/
def classifyWithoutInfoRule : List Fence → List Block × List Refusal
  | [] => ([], [])
  | f :: fs =>
    let rest := classifyWithoutInfoRule fs
    if mermaidInfo f.info && !mermaidWord f.info then rest
    else if mermaidWord f.info then
      if !notBlank f.literal then (rest.1, ⟨f.line, emptyForm⟩ :: rest.2)
      else (⟨f.line, f.literal⟩ :: rest.1, rest.2)
    else rest

/-- The second witness's port: the reader with the refusal at line 289 removed. -/
def readMermaidWithoutInfoRule (others : List Refusal) (text : List Char) (fences : List Fence) :
    Reading :=
  let (bs, rs) := classifyWithoutInfoRule fences
  { blocks := bs, refused := byLine (others ++ rs ++ openerRule (splitLines text) bs) }

/-- The claim, stated once, of a reading function. -/
def NamedReading (read : List Refusal → List Char → List Fence → Reading) : Prop :=
  ∀ (others : List Refusal) (text : List Char) (fences : List Fence),
    let r := read others text fences
    (∀ (i : Nat) (l info : List Char), (splitLines text)[i]? = some l → opener l = some info →
        mermaidInfo info = true →
        (∃ b ∈ r.blocks, b.line = i + 1) ∨ (⟨i + 1, openerForm⟩ : Refusal) ∈ r.refused) ∧
      (∀ f ∈ fences, mermaidInfo f.info = true →
        (⟨f.line, f.literal⟩ : Block) ∈ r.blocks ∨ (⟨f.line, infoForm⟩ : Refusal) ∈ r.refused ∨
          (⟨f.line, emptyForm⟩ : Refusal) ∈ r.refused) ∧
      (∀ b ∈ r.blocks, ∃ f ∈ fences, f.line = b.line ∧ f.literal = b.source ∧
        mermaidWord f.info = true ∧ notBlank f.literal = true) ∧
      (∀ x ∈ r.refused, x ∈ others ∨
        (x.form = infoForm ∧ ∃ f ∈ fences, f.line = x.line ∧ mermaidInfo f.info = true ∧
          mermaidWord f.info = false) ∨
        (x.form = emptyForm ∧ ∃ f ∈ fences, f.line = x.line ∧ mermaidWord f.info = true ∧
          notBlank f.literal = false) ∨
        (x.form = openerForm ∧ 1 ≤ x.line ∧
          (∃ l info, (splitLines text)[x.line - 1]? = some l ∧ opener l = some info ∧
            mermaidInfo info = true) ∧
          ∀ b ∈ r.blocks, b.line ≠ x.line)) ∧
      (∀ x ∈ others, x ∈ r.refused) ∧
      (r.refused.map (·.line)).Pairwise (· ≤ ·)

theorem readMermaid_blocks (others : List Refusal) (text : List Char) (fences : List Fence) :
    (readMermaid others text fences).blocks = (classify fences).1 := rfl

theorem readMermaid_refused (others : List Refusal) (text : List Char) (fences : List Fence) :
    (readMermaid others text fences).refused =
      byLine (others ++ (classify fences).2 ++ openerRule (splitLines text) (classify fences).1) :=
  rfl

theorem classify_cons (f : Fence) (fs : List Fence) :
    classify (f :: fs) =
      ((classify [f]).1 ++ (classify fs).1, (classify [f]).2 ++ (classify fs).2) := by
  cases hI : mermaidInfo f.info <;> cases hW : mermaidWord f.info <;>
    cases hN : notBlank f.literal <;> simp [classify, hI, hW, hN]

theorem classify_eq (fs : List Fence) :
    classify fs = (fs.flatMap fun f => (classify [f]).1, fs.flatMap fun f => (classify [f]).2) := by
  induction fs with
  | nil => rfl
  | cons f fs ih => rw [classify_cons, ih]; simp

theorem classify_one_blocks (f : Fence) (b : Block) (h : b ∈ (classify [f]).1) :
    f.line = b.line ∧ f.literal = b.source ∧ mermaidWord f.info = true ∧
      notBlank f.literal = true := by
  cases hI : mermaidInfo f.info <;> cases hW : mermaidWord f.info <;>
    cases hN : notBlank f.literal <;> simp [classify, hI, hW, hN] at h ⊢ <;> subst h <;> simp_all

theorem classify_one_refused (f : Fence) (x : Refusal) (h : x ∈ (classify [f]).2) :
    (x.form = infoForm ∧ f.line = x.line ∧ mermaidInfo f.info = true ∧
        mermaidWord f.info = false) ∨
      (x.form = emptyForm ∧ f.line = x.line ∧ mermaidWord f.info = true ∧
        notBlank f.literal = false) := by
  cases hI : mermaidInfo f.info <;> cases hW : mermaidWord f.info <;>
    cases hN : notBlank f.literal <;> simp [classify, hI, hW, hN] at h ⊢ <;> subst h <;> simp

theorem classify_one_named (f : Fence) (h : mermaidInfo f.info = true) :
    (⟨f.line, f.literal⟩ : Block) ∈ (classify [f]).1 ∨
      (⟨f.line, infoForm⟩ : Refusal) ∈ (classify [f]).2 ∨
        (⟨f.line, emptyForm⟩ : Refusal) ∈ (classify [f]).2 := by
  cases hW : mermaidWord f.info <;> cases hN : notBlank f.literal <;>
    simp [classify, h, hW, hN]

theorem mem_insertByLine {x y : Refusal} {l : List Refusal} :
    y ∈ insertByLine x l ↔ y = x ∨ y ∈ l := by
  induction l with
  | nil => simp [insertByLine]
  | cons z zs ih =>
    unfold insertByLine
    split
    · simp
    · simp only [List.mem_cons, ih]
      constructor
      · intro h; rcases h with h | h | h <;> simp [h]
      · intro h; rcases h with h | h | h <;> simp [h]

theorem mem_byLine {y : Refusal} {l : List Refusal} : y ∈ byLine l ↔ y ∈ l := by
  induction l with
  | nil => simp [byLine]
  | cons x xs ih => simp [byLine, mem_insertByLine, ih]

theorem insertByLine_sorted (x : Refusal) (l : List Refusal)
    (h : l.Pairwise (fun a b => a.line ≤ b.line)) :
    (insertByLine x l).Pairwise (fun a b => a.line ≤ b.line) := by
  induction l with
  | nil => simp [insertByLine]
  | cons y ys ih =>
    rw [List.pairwise_cons] at h
    unfold insertByLine
    split
    · next hxy =>
      refine List.pairwise_cons.2 ⟨?_, List.pairwise_cons.2 h⟩
      intro z hz
      rcases List.mem_cons.1 hz with hz | hz
      · subst hz; exact hxy
      · exact Nat.le_trans hxy (h.1 z hz)
    · next hxy =>
      refine List.pairwise_cons.2 ⟨?_, ih h.2⟩
      intro z hz
      rcases mem_insertByLine.1 hz with hz | hz
      · subst hz; omega
      · exact h.1 z hz

theorem byLine_sorted (l : List Refusal) :
    (byLine l).Pairwise (fun a b => a.line ≤ b.line) := by
  induction l with
  | nil => simp [byLine]
  | cons x xs ih => exact insertByLine_sorted x _ ih

theorem openerRuleFrom_tail (bs : List Block) (n : Nat) (l : List Char) (ls : List (List Char))
    (x : Refusal) (h : x ∈ openerRuleFrom bs (n + 1) ls) : x ∈ openerRuleFrom bs n (l :: ls) := by
  unfold openerRuleFrom
  split
  · split
    · exact List.mem_cons_of_mem _ h
    · exact h
  · exact h

theorem openerRuleFrom_sound (bs : List Block) (ls : List (List Char)) :
    ∀ (n : Nat) (x : Refusal), x ∈ openerRuleFrom bs n ls →
      ∃ i l info, ls[i]? = some l ∧ opener l = some info ∧ mermaidInfo info = true ∧
        (∀ b ∈ bs, b.line ≠ n + i + 1) ∧ x = ⟨n + i + 1, openerForm⟩ := by
  induction ls with
  | nil => intro n x h; simp [openerRuleFrom] at h
  | cons l ls ih =>
    intro n x h
    have tail : ∀ y, y ∈ openerRuleFrom bs (n + 1) ls →
        ∃ i l' info, (l :: ls)[i]? = some l' ∧ opener l' = some info ∧
          mermaidInfo info = true ∧ (∀ b ∈ bs, b.line ≠ n + i + 1) ∧
            y = ⟨n + i + 1, openerForm⟩ := by
      intro y hy
      obtain ⟨i, l', info, h1, h2, h3, h4, h5⟩ := ih (n + 1) y hy
      refine ⟨i + 1, l', info, by simpa using h1, h2, h3, ?_, ?_⟩
      · intro b hb; have := h4 b hb; omega
      · rw [h5]; congr 1; omega
    unfold openerRuleFrom at h
    split at h
    · next info ho =>
      split at h
      · next hc =>
        rcases List.mem_cons.1 h with h | h
        · simp only [Bool.and_eq_true, Bool.not_eq_true', List.any_eq_false, beq_iff_eq] at hc
          exact ⟨0, l, info, rfl, ho, hc.1, fun b hb => by simpa using hc.2 b hb, by simpa using h⟩
        · exact tail x h
      · exact tail x h
    · exact tail x h

theorem openerRuleFrom_complete (bs : List Block) (ls : List (List Char)) :
    ∀ (n i : Nat) (l info : List Char), ls[i]? = some l → opener l = some info →
      mermaidInfo info = true → (∀ b ∈ bs, b.line ≠ n + i + 1) →
        (⟨n + i + 1, openerForm⟩ : Refusal) ∈ openerRuleFrom bs n ls := by
  induction ls with
  | nil => intro n i l info h; simp at h
  | cons l' ls ih =>
    intro n i l info hl ho hm hb
    cases i with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at hl
      subst hl
      have hc : (mermaidInfo info && !(bs.any fun b => b.line == n + 1)) = true := by
        simp only [Bool.and_eq_true, Bool.not_eq_true', List.any_eq_false, beq_iff_eq]
        exact ⟨hm, fun b hb' => by simpa using hb b hb'⟩
      simp [openerRuleFrom, ho, hc]
    | succ i =>
      apply openerRuleFrom_tail
      have := ih (n + 1) i l info (by simpa using hl) ho hm
        (fun b hb' => by have := hb b hb'; omega)
      have e : n + 1 + i + 1 = n + (i + 1) + 1 := by omega
      rw [e] at this
      exact this

theorem every_line_that_may_open_a_mermaid_fence_is_read_or_refused_by_name :
    NamedReading readMermaid := by
  intro others text fences
  dsimp only
  rw [readMermaid_blocks, readMermaid_refused]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · intro i l info hl ho hm
    by_cases hb : ∃ b ∈ (classify fences).1, b.line = i + 1
    · exact Or.inl hb
    · right
      rw [mem_byLine, List.mem_append]
      right
      have := openerRuleFrom_complete (classify fences).1 (splitLines text) 0 i l info hl ho hm
        (fun b hb' he => hb ⟨b, hb', by omega⟩)
      simpa [openerRule] using this
  · intro f hf hm
    rw [mem_byLine, mem_byLine, classify_eq]
    simp only [List.mem_append, List.mem_flatMap]
    rcases classify_one_named f hm with h | h | h
    · exact Or.inl ⟨f, hf, h⟩
    · exact Or.inr (Or.inl (Or.inl (Or.inr ⟨f, hf, h⟩)))
    · exact Or.inr (Or.inr (Or.inl (Or.inr ⟨f, hf, h⟩)))
  · intro b hb
    rw [classify_eq] at hb
    obtain ⟨f, hf, hb⟩ := List.mem_flatMap.1 hb
    obtain ⟨h1, h2, h3, h4⟩ := classify_one_blocks f b hb
    exact ⟨f, hf, h1, h2, h3, h4⟩
  · intro x hx
    rw [mem_byLine, List.mem_append, List.mem_append] at hx
    rcases hx with (hx | hx) | hx
    · exact Or.inl hx
    · rw [classify_eq] at hx
      obtain ⟨f, hf, hx⟩ := List.mem_flatMap.1 hx
      rcases classify_one_refused f x hx with ⟨h1, h2, h3, h4⟩ | ⟨h1, h2, h3, h4⟩
      · exact Or.inr (Or.inl ⟨h1, f, hf, h2, h3, h4⟩)
      · exact Or.inr (Or.inr (Or.inl ⟨h1, f, hf, h2, h3, h4⟩))
    · obtain ⟨i, l, info, hl, ho, hm, hb, hx⟩ :=
        openerRuleFrom_sound (classify fences).1 (splitLines text) 0 x hx
      subst hx
      refine Or.inr (Or.inr (Or.inr ⟨rfl, by simp, ⟨l, info, by simpa using hl, ho, hm⟩, ?_⟩))
      intro b hb'
      simpa using hb b hb'
  · intro x hx
    rw [mem_byLine]
    simp [hx]
  · rw [List.pairwise_map]
    exact byLine_sorted _

theorem the_reader_without_its_opener_rule_leaves_a_mermaid_line_unnamed :
    ¬ NamedReading readMermaidWithoutOpenerRule := by
  intro h
  have h1 := (h [] "~~~\n```mermaid\nflowchart TD\n```\n~~~\n".toList
    [⟨1, "".toList, "```mermaid\nflowchart TD\n```\n".toList⟩]).1 1 "```mermaid".toList
    "mermaid".toList (by decide +kernel) (by decide +kernel) (by decide +kernel)
  revert h1
  decide +kernel

theorem the_reader_without_its_info_rule_leaves_a_mermaid_fence_unnamed :
    ¬ NamedReading readMermaidWithoutInfoRule := by
  intro h
  have h2 := (h [] "```Mermaid\nflowchart TD\n```\n".toList
    [⟨1, "Mermaid".toList, "flowchart TD\n".toList⟩]).2.1 ⟨1, "Mermaid".toList,
      "flowchart TD\n".toList⟩ (by decide +kernel) (by decide +kernel)
  revert h2
  decide +kernel

end Formal.DocsMermaidRead

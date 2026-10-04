-- @phx covers tools/table-census/table_census.rs anchor=covering digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers tools/table-census/table_census.rs anchor=piece digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers tools/table-census/table_census.rs anchor=reach_piece digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers tools/table-census/table_census.rs anchor=reach digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers tools/table-census/table_census.rs anchor=refusals digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx vectors formal/vectors/table-census-reach.jsonl
-- @phx cites #604, #585
-- @phx theorem refused_within_dev_and_a_lone_character_spared ramp=report
-- @phx witness old_violates kills=refused_within_dev_and_a_lone_character_spared

/-!
# Formal.TableCensusReach

The table censuses' shared reader, `tools/table-census/table_census.rs`, as it judges a tree it has
read (#604, SPEC-331, ADR-331). A piece of one character leaves SPEC-324 R3's workspace-wide pool
and is judged only in a file's reach: the file's own pieces, the pieces of every file it includes,
transitively, and the pieces of every `const` or `static` item it names.

**The claim**, one `Prop` over a census, `Claim`, with both theorems read from it:
- whatever the tree, every file the census refuses, dev's reader refuses too (SPEC-331 R5), so no
  tree green at dev turns red;
- a lone file is not refused: its every piece is one character and no macro metavariable's item's,
  no file includes it and it includes none, every item it names is its own and no other file names
  one of its items, its own pieces cannot cover the name, and no word or fail-closed arm names it.
  That is SPEC-331 A1's third file, #600's `strip_prefix('M')` among them.

The second half needs the file to include nothing and to name only its own items. A file of lone
characters that names items holding the rest of the name joins them, and the reach refuses it, as
it must (SPEC-331 A2).

**What is ported.** `covering` branch for branch: the start, end and middle pieces, `reached` and
`finished` as the two loops fill them, and a piece on a path. The vectors are arrays the loops
index; here they are functions from the index, set one place at a time, which is what an array
write is. `piece`'s split at each brace, its fold to lower case, and its pool decision: a part of
fewer than two characters stays out unless a metavariable names its item. `reach` as its stack
walk over the includes, then the items its visited files name. `refusals`' composition: the pool's
covering, each file's reach's covering, the words that name the table and the fail-closed arms.
dev's `refusals` at the base, `refusalsOld`, pools every part. The walk takes one step of fuel per
pop, `included.length + 1` in all, which bounds the pops: each file is expanded once, and each
expansion pushes its own includes.

The names are ASCII, so a byte index into them is a character index, and a piece's byte length and
its character length agree wherever `covering` reads them: a piece that is not ASCII is no prefix
of an ASCII name, so its middle is none either way.

**What is not ported.** The lexer and the reader's walk of the source (`lex`, `read`, `follow`,
`word`, `items_of`): the tree is what they record, the literals with their files and items, the
files read, the includes, the names each file writes (its words and placeholder names), the files
a word names the table in, and the files a fail-closed arm refuses. A refusal is its file here;
the reader's message names the file.

The witness is dev's reader at `cff914e3`, `tools/table-census/table_census.rs:435-443` (`piece`
pools every part) and `:632` (the pool's covering alone), on SPEC-331's L1 tree, which dev refuses
by its lone `'M'` (#600, #604).
-/

namespace Formal.TableCensusReach

/-- One literal the reader read: its file, its decoded value, and the `const` or `static` item
whose initializer holds it, `$` for an item a macro's metavariable names. -/
structure Literal where
  file : String
  value : List Char
  item : Option String
deriving DecidableEq, Repr

/-- What the reader records of one tree, for one census's name. -/
structure Tree where
  name : List Char
  literals : List Literal
  files : List String
  included : List (String × String)
  mentions : List (String × String)
  named : List String
  failClosed : List String
deriving Repr

/-- One part of a literal, cut at its braces, with its file and item. -/
structure Part where
  file : String
  part : List Char
  item : Option String
deriving DecidableEq, Repr

/-- A pool: each piece, folded, with the files holding it. -/
abbrev Pool := List (List Char × List String)

/-- `to_ascii_lowercase`. -/
def lower (s : List Char) : List Char := s.map Char.toLower

/-- `value.split(['{', '}'])`, empty parts included. -/
def splitBraces : List Char → List (List Char)
  | [] => [[]]
  | c :: cs =>
    if c = '{' ∨ c = '}' then [] :: splitBraces cs
    else
      match splitBraces cs with
      | [] => [[c]]
      | p :: ps => (c :: p) :: ps

/-- `piece`'s parts of one literal: cut at each brace, the empty parts dropped. -/
def piece (l : Literal) : List Part :=
  ((splitBraces l.value).filter (· ≠ [])).map fun p => ⟨l.file, p, l.item⟩

/-- Every part the reader read. -/
def parts (t : Tree) : List Part := t.literals.flatMap piece

/-- `piece`'s pool decision: a part of fewer than two characters stays out of the pool, unless a
macro's metavariable names its item. -/
def pooled (p : Part) : Bool := !(decide (p.part.length < 2) && p.item != some "$")

/-- dev's pool: every part. -/
def poolOld (t : Tree) : Pool := (parts t).map fun p => (lower p.part, [p.file])

/-- The pool: the parts `piece` pools. -/
def pool (t : Tree) : Pool := ((parts t).filter pooled).map fun p => (lower p.part, [p.file])

/-- `piece.ends_with(&name[..at])`. -/
def startP (name piece : List Char) (pos : Nat) : Bool := (name.take pos).isSuffixOf piece

/-- `piece.starts_with(&name[at..])`. -/
def endP (name piece : List Char) (pos : Nat) : Bool := (name.drop pos).isPrefixOf piece

/-- The middle piece's index after it, if `piece` is exactly the part of `name` from `at`, short
of its end. -/
def middleP (name piece : List Char) (pos : Nat) : Option Nat :=
  if piece ≠ [] ∧ pos + piece.length < name.length ∧ piece.isPrefixOf (name.drop pos) = true then
    some (pos + piece.length)
  else none

/-- `piece.contains(name)`. -/
def holds (name : List Char) : List Char → Bool
  | [] => name.isPrefixOf []
  | c :: cs => name.isPrefixOf (c :: cs) || holds name cs

/-- One of `covering`'s two vectors, read by index. A structure, so a loop's step returns a value
rather than a function the compiler would re-run at every read. -/
structure Marks where
  get : Nat → Bool

/-- The vector before either loop: every place false. -/
def unmarked : Marks := ⟨fun _ => false⟩

/-- A vector write: `r` with place `i` set to `v`. Never inlined, so `v` is a value when the write
is made, as the vector's is. -/
@[noinline] def setAt (r : Marks) (i : Nat) (v : Bool) : Marks := ⟨fun j => if j = i then v else r.get j⟩

/-- The middles from `at`: each sets the place after it. -/
def middles (name : List Char) (pos : Nat) (keys : List (List Char)) (r : Marks) : Marks :=
  keys.foldl (fun r piece =>
    match middleP name piece pos with
    | some after => setAt r after true
    | none => r) r

/-- One step of the `reached` loop at `at`. -/
def reachedStep (name : List Char) (keys : List (List Char)) (r : Marks) (pos : Nat) : Marks :=
  let r := setAt r pos (r.get pos || keys.any fun piece => startP name piece pos)
  if r.get pos then middles name pos keys r else r

/-- `reached`: the loop over `1..size`, ascending. -/
def reached (name : List Char) (keys : List (List Char)) : Marks :=
  (List.range' 1 (name.length - 1)).foldl (reachedStep name keys) unmarked

/-- One step of the `finished` loop at `at`. -/
def finishedStep (name : List Char) (keys : List (List Char)) (f : Marks) (pos : Nat) : Marks :=
  setAt f pos (keys.any fun piece => endP name piece pos || (middleP name piece pos).any f.get)

/-- `finished`: the loop over `1..size`, descending. -/
def finished (name : List Char) (keys : List (List Char)) : Marks :=
  (List.range' 1 (name.length - 1)).reverse.foldl (finishedStep name keys) unmarked

/-- Whether `piece` is on a path that assembles `name`. -/
def onAPath (name : List Char) (r f : Marks) (piece : List Char) : Bool :=
  holds name piece || (List.range' 1 (name.length - 1)).any fun pos =>
    (startP name piece pos && f.get pos) || (r.get pos && endP name piece pos) ||
      (r.get pos && (middleP name piece pos).any f.get)

/-- `covering`: the files holding a piece on a path through `pool` that assembles `name`. -/
def covering (name : List Char) (pool : Pool) : List String :=
  pool.foldl (fun named entry =>
    if onAPath name (reached name (pool.map (·.1))) (finished name (pool.map (·.1))) entry.1 then
      named ++ entry.2
    else named) []

/-- `reach`'s stack walk over the includes: `pending`'s head is the stack's top. -/
def walk (t : Tree) : Nat → List String → List String → List String
  | 0, _, visited => visited
  | _ + 1, [], visited => visited
  | fuel + 1, next :: pending, visited =>
    if next ∈ visited then walk t fuel pending visited
    else
      walk t fuel (((t.included.filter (·.1 = next)).map (·.2)).reverse ++ pending)
        (next :: visited)

/-- The files `file`'s walk visits. -/
def visitedFrom (t : Tree) (file : String) : List String :=
  walk t (t.included.length + 1) [file] []

/-- `reach`: the pool of `file`'s reach, its visited files' own pieces, then the pieces of every
item a visited file names, each held by the file holding it. -/
def reach (t : Tree) (file : String) : Pool :=
  ((visitedFrom t file).flatMap fun next =>
      ((parts t).filter (·.file = next)).map fun p => (lower p.part, [next])) ++
    ((visitedFrom t file).flatMap fun holder =>
      (t.mentions.filter (·.1 = holder)).flatMap fun m =>
        ((parts t).filter (·.item = some m.2)).map fun p => (lower p.part, [p.file]))

/-- The files that have a reach: every file read, and every file holding a part. -/
def localKeys (t : Tree) : List String := t.files ++ (parts t).map (·.file)

/-- `refusals`: the fail-closed arms, the pool's covering, each reach's covering and the words. -/
def refusals (t : Tree) : List String :=
  t.failClosed ++ covering t.name (pool t) ++
    ((localKeys t).flatMap fun file => covering t.name (reach t file)) ++ t.named

/-- dev's `refusals` at `cff914e3`: the fail-closed arms, the covering of a pool of every part,
and the words. -/
def refusalsOld (t : Tree) : List String :=
  t.failClosed ++ covering t.name (poolOld t) ++ t.named

/-- A file's own pieces, held by it. -/
def ownPool (t : Tree) (f : String) : Pool :=
  ((parts t).filter (·.file = f)).map fun p => (lower p.part, [f])

/-- A lone file: SPEC-331 A1's third file, in general. -/
def Lone (t : Tree) (f : String) : Prop :=
  (∀ p ∈ parts t, p.file = f → p.part.length < 2 ∧ p.item ≠ some "$") ∧
  (∀ e ∈ t.included, e.2 ≠ f) ∧
  (∀ e ∈ t.included, e.1 ≠ f) ∧
  (∀ m ∈ t.mentions, ∀ p ∈ parts t, p.item = some m.2 → (m.1 = f ↔ p.file = f)) ∧
  f ∉ covering t.name (ownPool t f) ∧
  f ∉ t.named ∧
  f ∉ t.failClosed

instance (t : Tree) (f : String) : Decidable (Lone t f) := by unfold Lone; infer_instance

/-- The claim over a census: it refuses within dev's refusals, and it refuses no lone file. -/
def Claim (census : Tree → List String) : Prop :=
  (∀ t, ∀ x ∈ census t, x ∈ refusalsOld t) ∧ (∀ t f, Lone t f → f ∉ census t)

/-! ## Monotonicity of `covering` in its pool -/

/-- Bool implication, pointwise. -/
def Le (r s : Marks) : Prop := ∀ i, r.get i = true → s.get i = true

theorem setAt_le {r s : Marks} {v w : Bool} (pos : Nat) (h : Le r s) (hv : v = true → w = true) :
    Le (setAt r pos v) (setAt s pos w) := by
  intro i hi
  simp only [setAt] at hi ⊢
  by_cases e : i = pos
  · subst e
    simp only [ite_true] at hi ⊢
    exact hv hi
  · simp only [e, ite_false] at hi ⊢
    exact h i hi

theorem middles_apply (name : List Char) (pos : Nat) (keys : List (List Char)) (r : Marks)
    (i : Nat) :
    (middles name pos keys r).get i = (r.get i || keys.any fun piece => decide (middleP name piece pos = some i)) := by
  induction keys generalizing r with
  | nil => simp [middles]
  | cons piece rest ih =>
    unfold middles at *
    simp only [List.foldl_cons, List.any_cons]
    rw [ih]
    cases h : middleP name piece pos with
    | none => simp
    | some after =>
      simp only [setAt]
      by_cases e : i = after
      · subst e; simp
      · simp [e, Ne.symm e]

theorem any_mono {α : Type} {l₁ l₂ : List α} {p q : α → Bool} (hl : ∀ x ∈ l₁, x ∈ l₂)
    (hpq : ∀ x, p x = true → q x = true) : l₁.any p = true → l₂.any q = true := by
  simp only [List.any_eq_true]
  rintro ⟨x, hx, px⟩
  exact ⟨x, hl x hx, hpq x px⟩

theorem reachedStep_le (name : List Char) {k₁ k₂ : List (List Char)} (hk : ∀ x ∈ k₁, x ∈ k₂)
    {r s : Marks} (h : Le r s) (pos : Nat) :
    Le (reachedStep name k₁ r pos) (reachedStep name k₂ s pos) := by
  have hv : (r.get pos || k₁.any fun piece => startP name piece pos) = true →
      (s.get pos || k₂.any fun piece => startP name piece pos) = true := by
    simp only [Bool.or_eq_true]
    rintro (h1 | h1)
    · exact Or.inl (h pos h1)
    · exact Or.inr (any_mono hk (fun _ hx => hx) h1)
  have hset := setAt_le pos h hv
  intro i hi
  simp only [reachedStep] at *
  generalize hv₁ : (r.get pos || k₁.any fun piece => startP name piece pos) = v₁ at hi hset hv
  generalize hv₂ : (s.get pos || k₂.any fun piece => startP name piece pos) = v₂ at hset hv ⊢
  have ha₁ : (setAt r pos v₁).get pos = v₁ := by simp [setAt]
  have ha₂ : (setAt s pos v₂).get pos = v₂ := by simp [setAt]
  rw [ha₁] at hi
  rw [ha₂]
  cases v₁ with
  | false =>
    simp only [Bool.false_eq_true, ite_false] at hi
    have := hset i hi
    split
    · rw [middles_apply]; simp [this]
    · exact this
  | true =>
    simp only [ite_true] at hi
    have hv2 : v₂ = true := hv rfl
    subst hv2
    simp only [ite_true]
    rw [middles_apply] at hi ⊢
    simp only [Bool.or_eq_true] at hi ⊢
    rcases hi with hi | hi
    · exact Or.inl (hset i hi)
    · exact Or.inr (any_mono hk (fun _ hx => hx) hi)

theorem reached_fold_le (name : List Char) {k₁ k₂ : List (List Char)} (hk : ∀ x ∈ k₁, x ∈ k₂)
    (ps : List Nat) : ∀ {r s : Marks}, Le r s →
      Le (ps.foldl (reachedStep name k₁) r) (ps.foldl (reachedStep name k₂) s) := by
  induction ps with
  | nil => intro r s h; simpa using h
  | cons p ps ih =>
    intro r s h
    simp only [List.foldl_cons]
    exact ih (reachedStep_le name hk h p)

theorem reached_le (name : List Char) {k₁ k₂ : List (List Char)} (hk : ∀ x ∈ k₁, x ∈ k₂) :
    Le (reached name k₁) (reached name k₂) :=
  reached_fold_le name hk _ (fun _ h => h)

theorem finishedStep_le (name : List Char) {k₁ k₂ : List (List Char)} (hk : ∀ x ∈ k₁, x ∈ k₂)
    {f g : Marks} (h : Le f g) (pos : Nat) :
    Le (finishedStep name k₁ f pos) (finishedStep name k₂ g pos) := by
  apply setAt_le pos h
  apply any_mono hk
  intro piece hp
  simp only [Bool.or_eq_true] at hp ⊢
  rcases hp with hp | hp
  · exact Or.inl hp
  · right
    cases hm : middleP name piece pos with
    | none => simp [hm] at hp
    | some after => simp [hm] at hp ⊢; exact h after hp

theorem finished_fold_le (name : List Char) {k₁ k₂ : List (List Char)} (hk : ∀ x ∈ k₁, x ∈ k₂)
    (ps : List Nat) : ∀ {f g : Marks}, Le f g →
      Le (ps.foldl (finishedStep name k₁) f) (ps.foldl (finishedStep name k₂) g) := by
  induction ps with
  | nil => intro f g h; simpa using h
  | cons p ps ih =>
    intro f g h
    simp only [List.foldl_cons]
    exact ih (finishedStep_le name hk h p)

theorem finished_le (name : List Char) {k₁ k₂ : List (List Char)} (hk : ∀ x ∈ k₁, x ∈ k₂) :
    Le (finished name k₁) (finished name k₂) :=
  finished_fold_le name hk _ (fun _ h => h)

theorem onAPath_mono (name piece : List Char) {r s f g : Marks} (hr : Le r s) (hf : Le f g) :
    onAPath name r f piece = true → onAPath name s g piece = true := by
  unfold onAPath
  simp only [Bool.or_eq_true]
  rintro (h | h)
  · exact Or.inl h
  · right
    refine any_mono (fun x hx => hx) ?_ h
    intro pos hat
    simp only [Bool.or_eq_true, Bool.and_eq_true] at hat ⊢
    rcases hat with (⟨h1, h2⟩ | ⟨h1, h2⟩) | ⟨h1, h2⟩
    · exact Or.inl (Or.inl ⟨h1, hf pos h2⟩)
    · exact Or.inl (Or.inr ⟨hr pos h1, h2⟩)
    · refine Or.inr ⟨hr pos h1, ?_⟩
      cases hm : middleP name piece pos with
      | none => simp [hm] at h2
      | some after => simp [hm] at h2 ⊢; exact hf after h2

theorem mem_fold_named (c : List Char × List String → Bool) (x : String) :
    ∀ (pool : Pool) (acc : List String),
      x ∈ pool.foldl (fun named entry => if c entry then named ++ entry.2 else named) acc ↔
        x ∈ acc ∨ ∃ e ∈ pool, c e = true ∧ x ∈ e.2 := by
  intro pool
  induction pool with
  | nil => intro acc; simp
  | cons e rest ih =>
    intro acc
    simp only [List.foldl_cons]
    rw [ih]
    by_cases hc : c e = true
    · simp [hc, or_assoc]
    · simp [hc]

theorem mem_covering (name : List Char) (pool : Pool) (x : String) :
    x ∈ covering name pool ↔
      ∃ e ∈ pool, onAPath name (reached name (pool.map (·.1))) (finished name (pool.map (·.1)))
        e.1 = true ∧ x ∈ e.2 := by
  unfold covering
  rw [mem_fold_named]
  simp

theorem covering_mono (name : List Char) {p₁ p₂ : Pool} (h : ∀ e ∈ p₁, e ∈ p₂) (x : String) :
    x ∈ covering name p₁ → x ∈ covering name p₂ := by
  rw [mem_covering, mem_covering]
  rintro ⟨e, he, hpath, hx⟩
  have hk : ∀ k ∈ p₁.map (·.1), k ∈ p₂.map (·.1) := by
    intro k hk
    simp only [List.mem_map] at hk ⊢
    obtain ⟨e', he', rfl⟩ := hk
    exact ⟨e', h e' he', rfl⟩
  exact ⟨e, h e he, onAPath_mono name e.1 (reached_le name hk) (finished_le name hk) hpath, hx⟩

theorem mem_holders (name : List Char) (pool : Pool) (x : String) :
    x ∈ covering name pool → ∃ e ∈ pool, x ∈ e.2 := by
  rw [mem_covering]
  rintro ⟨e, he, _, hx⟩
  exact ⟨e, he, hx⟩

/-! ## The walk stays within a closed set -/

theorem walk_closed (t : Tree) (P : String → Prop) (hP : ∀ e ∈ t.included, P e.1 → P e.2) :
    ∀ (fuel : Nat) (pending visited : List String), (∀ v ∈ pending, P v) →
      (∀ v ∈ visited, P v) → ∀ v ∈ walk t fuel pending visited, P v := by
  intro fuel
  induction fuel with
  | zero => intro pending visited _ hv; simpa [walk] using hv
  | succ fuel ih =>
    intro pending visited hp hv
    cases pending with
    | nil => simpa [walk] using hv
    | cons next rest =>
      simp only [walk]
      split
      · exact ih rest visited (fun v h => hp v (List.mem_cons_of_mem _ h)) hv
      · apply ih
        · intro v h
          simp only [List.mem_append, List.mem_reverse, List.mem_map, List.mem_filter,
            decide_eq_true_eq] at h
          rcases h with ⟨e, ⟨he, rfl⟩, rfl⟩ | h
          · exact hP e he (hp _ List.mem_cons_self)
          · exact hp v (List.mem_cons_of_mem _ h)
        · intro v h
          rcases List.mem_cons.mp h with rfl | h
          · exact hp _ List.mem_cons_self
          · exact hv v h

theorem visited_closed (t : Tree) (P : String → Prop) (hP : ∀ e ∈ t.included, P e.1 → P e.2)
    (file : String) (h : P file) : ∀ v ∈ visitedFrom t file, P v :=
  walk_closed t P hP _ [file] [] (by simpa using h) (by simp)

/-! ## The claim -/

theorem pool_sub (t : Tree) : ∀ e ∈ pool t, e ∈ poolOld t := by
  intro e he
  simp only [pool, poolOld, List.mem_map, List.mem_filter] at he ⊢
  obtain ⟨p, ⟨hp, _⟩, rfl⟩ := he
  exact ⟨p, hp, rfl⟩

theorem reach_sub (t : Tree) (file : String) : ∀ e ∈ reach t file, e ∈ poolOld t := by
  intro e he
  simp only [reach, poolOld, List.mem_append, List.mem_flatMap, List.mem_map, List.mem_filter,
    decide_eq_true_eq] at he ⊢
  rcases he with ⟨next, _, p, ⟨hp, hf⟩, rfl⟩ | ⟨_, _, m, _, p, ⟨hp, _⟩, rfl⟩
  · exact ⟨p, hp, by rw [hf]⟩
  · exact ⟨p, hp, rfl⟩

theorem refusals_within_dev (t : Tree) : ∀ x ∈ refusals t, x ∈ refusalsOld t := by
  intro x hx
  simp only [refusals, refusalsOld, List.mem_append, List.mem_flatMap] at hx ⊢
  rcases hx with ((h | h) | ⟨file, _, h⟩) | h
  · exact Or.inl (Or.inl h)
  · exact Or.inl (Or.inr (covering_mono t.name (pool_sub t) x h))
  · exact Or.inl (Or.inr (covering_mono t.name (reach_sub t file) x h))
  · exact Or.inr h

theorem lone_spared (t : Tree) (f : String) (h : Lone t f) : f ∉ refusals t := by
  obtain ⟨hparts, hinto, hfrom, hnames, hown, hnamed, hfail⟩ := h
  intro hx
  simp only [refusals, List.mem_append, List.mem_flatMap] at hx
  rcases hx with ((h | h) | ⟨g, _, h⟩) | h
  · exact hfail h
  · obtain ⟨e, he, hfe⟩ := mem_holders t.name (pool t) f h
    simp only [pool, List.mem_map, List.mem_filter] at he
    obtain ⟨p, ⟨hp, hpool⟩, rfl⟩ := he
    simp only [List.mem_singleton] at hfe
    obtain ⟨hlen, hitem⟩ := hparts p hp hfe.symm
    simp [pooled, hlen, hitem] at hpool
  · -- the reach that holds `f` is `f`'s own
    have hg : g = f := by
      obtain ⟨e, he, hfe⟩ := mem_holders t.name (reach t g) f h
      have hvis : f ∈ visitedFrom t g := by
        simp only [reach, List.mem_append, List.mem_flatMap, List.mem_map, List.mem_filter,
          decide_eq_true_eq] at he
        rcases he with ⟨next, hn, p, ⟨_, _⟩, rfl⟩ | ⟨holder, hh, m, ⟨hm, hmh⟩, p, ⟨hp, hpi⟩, rfl⟩
        · simp only [List.mem_singleton] at hfe
          rw [hfe]; exact hn
        · simp only [List.mem_singleton] at hfe
          have := (hnames m hm p hp hpi).mpr hfe.symm
          rw [← this, hmh]; exact hh
      have := visited_closed t (fun v => v = g ∨ ∃ e ∈ t.included, e.2 = v)
        (fun e he _ => Or.inr ⟨e, he, rfl⟩) g (Or.inl rfl) f hvis
      rcases this with h | ⟨e, he, hef⟩
      · exact h.symm
      · exact absurd hef (hinto e he)
    subst hg
    apply hown
    refine covering_mono t.name ?_ g h
    intro e he
    have hvis : ∀ v ∈ visitedFrom t g, v = g :=
      visited_closed t (fun v => v = g) (fun e he h1 => absurd h1 (hfrom e he)) g rfl
    simp only [reach, List.mem_append, List.mem_flatMap, List.mem_map, List.mem_filter,
      decide_eq_true_eq] at he
    simp only [ownPool, List.mem_map, List.mem_filter, decide_eq_true_eq]
    rcases he with ⟨next, hn, p, ⟨hp, hpf⟩, rfl⟩ | ⟨holder, hh, m, ⟨hm, hmh⟩, p, ⟨hp, hpi⟩, rfl⟩
    · refine ⟨p, ⟨hp, ?_⟩, ?_⟩
      · rw [hpf]; exact hvis next hn
      · rw [hvis next hn]
    · have hpf : p.file = g := (hnames m hm p hp hpi).mp (by rw [hmh]; exact hvis holder hh)
      exact ⟨p, ⟨hp, hpf⟩, by rw [hpf]⟩
  · exact hnamed h

/-- The claim holds of the reader that ships. -/
theorem refused_within_dev_and_a_lone_character_spared : Claim refusals :=
  ⟨refusals_within_dev, lone_spared⟩

/-! ## The witness: SPEC-331's L1 tree, which dev refuses by its lone `'M'` -/

/-- The L1 tree for `xp_settlement`: two lone pieces and one lone `'M'` in three files. -/
def l1 : Tree where
  name := "xp_settlement".toList
  literals :=
    [⟨"crates/quests/src/l1_a.rs", "xp_settle".toList, none⟩,
     ⟨"crates/quests/src/l1_b.rs", "ent".toList, none⟩,
     ⟨"crates/streaks/src/l1_c.rs", "M".toList, none⟩]
  files := ["crates/quests/src/l1_a.rs", "crates/quests/src/l1_b.rs", "crates/streaks/src/l1_c.rs"]
  included := []
  mentions :=
    [("crates/quests/src/l1_a.rs", "starts_with"), ("crates/quests/src/l1_b.rs", "ends_with"),
     ("crates/streaks/src/l1_c.rs", "strip_prefix")]
  named := []
  failClosed := []

/-- dev's reader refuses a lone file, so the claim is false of it. -/
theorem old_violates : ¬ Claim refusalsOld := by
  intro h
  exact h.2 l1 "crates/streaks/src/l1_c.rs" (by decide +kernel) (by decide +kernel)

end Formal.TableCensusReach

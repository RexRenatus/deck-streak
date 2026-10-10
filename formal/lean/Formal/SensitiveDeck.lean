-- @phx covers crates/ingest/src/sensitive.rs anchor=admits digest=sha256:1cbdfb7d696284f79f5ba4277789db3790240ee866022892bbe2ef0fb40dd426
-- @phx covers crates/ingest/src/sensitive.rs anchor=under digest=sha256:9927ba2d923f9a116779603de23693a3597dbf70bb7e6582cfcf4fd0e9e0a218
-- @phx covers crates/ingest/src/settings.rs anchor=DECK_SEPARATOR digest=sha256:bd9858995e88958a33f54813dcd4393738b4d88fe946be1d9be17034144d5b6a
-- @phx vectors formal/vectors/sensitive-deck.jsonl
-- @phx cites #751
-- @phx theorem a_marked_deck_or_an_ancestor_refuses_the_card ramp=report
-- @phx witness admitting_every_card_lets_a_marked_deck_through kills=a_marked_deck_or_an_ancestor_refuses_the_card
-- @phx theorem marking_more_decks_never_admits_a_refused_card ramp=report
-- @phx witness admitting_every_card_lets_a_marked_home_deck_through kills=marking_more_decks_never_admits_a_refused_card
-- @phx theorem an_unresolved_deck_or_an_unreadable_set_is_refused ramp=report
-- @phx witness admitting_every_card_lets_an_unresolved_deck_through kills=an_unresolved_deck_or_an_unreadable_set_is_refused

/-!
# Formal.SensitiveDeck

The rule that keeps a deck's cards away from AI (#751; SPEC-381 R2 and section 8; ADR-392 D2):
`admits` in `crates/ingest/src/sensitive.rs`, with the name rule `under` beside it and the
separator `DECK_SEPARATOR` in `crates/ingest/src/settings.rs`, as a total function over the marked
set, the collection's deck tree and a card's two decks.

**The claim.** For every marked set, every deck tree and every card:
- `a_marked_deck_or_an_ancestor_refuses_the_card`: when both of the card's decks are in the tree,
  the card is kept away exactly when a marked deck in the tree is its home deck or its current
  deck, or an ancestor of either by name, and it is admitted exactly when none is;
- `marking_more_decks_never_admits_a_refused_card`: a card a smaller marked set refuses, a larger
  one refuses too, and a card whose own home or current deck is marked is kept away;
- `an_unresolved_deck_or_an_unreadable_set_is_refused`: a set that could not be read refuses every
  card, and a card whose home or current deck is not in the tree is refused as unresolved.

**What is ported.** `admits` reads the marked set as `Option<&BTreeSet<i64>>`, here
`Option (List Int)`: the rule only asks whether any marked deck in the tree is over the card, so
the set's order and its duplicates change no answer. The tree is a `BTreeMap<i64, String>`, here
any function from a deck id to its name, read as characters; every map is such a function. The
rule's branches are ported in order: an unreadable set, then an unresolved home or current deck,
then the marked decks the tree names, filtered and searched for one over either deck. `under`
strips the ancestor's name from the deck's and admits what is left only when it is empty or begins
with the separator; `str::strip_prefix` on two valid strings is a prefix test on their characters,
which `stripPrefix` does by structural recursion.

**What is not ported.** Reading the set from `sensitive_decks` and reading the tree from the
collection; a failed read reaches this rule as the absent set. Who asks the rule, and when, is
`tla/SensitiveDeckGate`'s.

**The witnesses** are the merge-base's behaviour: at `88c66092` no rule stood between a card and
the runner (`crates/agent/src/duty.rs:178` calls `self.runner.run` after the input gate alone) or
the day set (`crates/readings/src/day_set.rs:620-621` resolves every card's deck), so every card
was admitted, which the census's red at the delivery's first test commit measured.
-/

namespace Formal.SensitiveDeck

/-- `DECK_SEPARATOR`: the character between the parts of a deck's name. -/
def deckSeparator : Char := '\x1f'

/-- What `admits` decides for one card: `Admission`'s four variants, in their order. -/
inductive Admission where
  | admitted
  | keptAway
  | unresolved
  | unreadable
  deriving DecidableEq, Repr

/-- A deck tree: each deck id's name, as characters, or nothing for an id the tree lacks. -/
abbrev Tree := Int → Option (List Char)

/-- A rule over the marked set (absent when it could not be read), the tree, and a card's home
and current deck ids. -/
abbrev Rule := Option (List Int) → Tree → Int → Int → Admission

/-- `str::strip_prefix`: what is left of `name` after `ancestor`, when `name` begins with it. -/
def stripPrefix : List Char → List Char → Option (List Char)
  | name, [] => some name
  | [], _ :: _ => none
  | c :: cs, a :: as => if c = a then stripPrefix cs as else none

/-- `under`: the deck named `name` is the deck named `ancestor` or sits under it. -/
def under (name ancestor : List Char) : Bool :=
  match stripPrefix name ancestor with
  | none => false
  | some rest => rest.isEmpty || rest.head? == some deckSeparator

/-- `admits`: the one rule, branch for branch. -/
def admits (marked : Option (List Int)) (tree : Tree) (home current : Int) : Admission :=
  match marked with
  | none => .unreadable
  | some marked =>
    match tree home, tree current with
    | some home, some current =>
      if (marked.filterMap tree).any (fun m => under home m || under current m) then
        .keptAway
      else
        .admitted
    | _, _ => .unresolved

/-- The merge-base's behaviour: no rule stood before the runner or the day set, so every card was
admitted (`crates/agent/src/duty.rs:178` and `crates/readings/src/day_set.rs:620-621` at
`88c66092`). -/
def admitsAtTheBase : Rule := fun _ _ _ _ => .admitted

/-- A deck is under an ancestor, stated apart from `under`: it is the ancestor, or its name is the
ancestor's followed by the separator and the rest. -/
def IsUnder (name ancestor : List Char) : Prop :=
  name = ancestor ∨ ∃ rest, name = ancestor ++ deckSeparator :: rest

/-- Some marked deck in the tree is over the card's home deck or its current deck. -/
def MarkedOver (marked : List Int) (tree : Tree) (home current : List Char) : Prop :=
  ∃ d m, d ∈ marked ∧ tree d = some m ∧ (IsUnder home m ∨ IsUnder current m)

/-- The claim, stated once: a card both of whose decks resolve is kept away exactly when a marked
deck is over either of them, and admitted exactly when none is. -/
def RefusesByEitherDeck (rule : Rule) : Prop :=
  ∀ (marked : List Int) (tree : Tree) (home current : Int) (h c : List Char),
    tree home = some h → tree current = some c →
      (rule (some marked) tree home current = .keptAway ↔ MarkedOver marked tree h c) ∧
      (rule (some marked) tree home current = .admitted ↔ ¬ MarkedOver marked tree h c)

/-- The claim, stated once: marking more decks never admits a card the smaller set refused, and a
card whose own home or current deck is marked is kept away. -/
def MarkingMoreNeverAdmits (rule : Rule) : Prop :=
  (∀ (small big : List Int) (tree : Tree) (home current : Int),
    (∀ d, d ∈ small → d ∈ big) →
      rule (some small) tree home current ≠ .admitted →
        rule (some big) tree home current ≠ .admitted) ∧
  (∀ (marked : List Int) (tree : Tree) (home current : Int),
    (home ∈ marked ∨ current ∈ marked) → (tree home).isSome → (tree current).isSome →
      rule (some marked) tree home current = .keptAway)

/-- The claim, stated once: an unreadable set refuses every card, and a card with a deck the tree
lacks is refused as unresolved. -/
def UnresolvedOrUnreadableIsRefused (rule : Rule) : Prop :=
  ∀ (tree : Tree) (home current : Int),
    rule none tree home current = .unreadable ∧
      ∀ marked : List Int, (tree home = none ∨ tree current = none) →
        rule (some marked) tree home current = .unresolved

/-- `stripPrefix` answers the rest exactly when the name is the ancestor followed by it. -/
theorem stripPrefix_eq_some : ∀ (name ancestor rest : List Char),
    stripPrefix name ancestor = some rest ↔ name = ancestor ++ rest
  | name, [], rest => by simp [stripPrefix]
  | [], _ :: _, rest => by simp [stripPrefix]
  | c :: cs, a :: as, rest => by
    by_cases h : c = a
    · subst h
      simp [stripPrefix, stripPrefix_eq_some cs as rest]
    · simp [stripPrefix, h]

/-- `under` is `IsUnder`. -/
theorem under_iff (name ancestor : List Char) :
    under name ancestor = true ↔ IsUnder name ancestor := by
  unfold under IsUnder
  cases hs : stripPrefix name ancestor with
  | none =>
    simp only [Bool.false_eq_true, false_iff]
    rintro (h | ⟨rest, h⟩)
    · have := (stripPrefix_eq_some name ancestor []).2 (by simp [h])
      simp [hs] at this
    · have := (stripPrefix_eq_some name ancestor (deckSeparator :: rest)).2 h
      simp [hs] at this
  | some rest =>
    have hn := (stripPrefix_eq_some name ancestor rest).1 hs
    subst hn
    cases rest with
    | nil => simp
    | cons x r => simp

/-- The rule's search finds a marked deck over the card exactly when `MarkedOver` holds. -/
theorem any_iff_markedOver (marked : List Int) (tree : Tree) (h c : List Char) :
    (marked.filterMap tree).any (fun m => under h m || under c m) = true ↔
      MarkedOver marked tree h c := by
  simp only [List.any_eq_true, List.mem_filterMap, Bool.or_eq_true, under_iff, MarkedOver]
  constructor
  · rintro ⟨m, ⟨d, hd, ht⟩, hu⟩
    exact ⟨d, m, hd, ht, hu⟩
  · rintro ⟨d, m, hd, ht, hu⟩
    exact ⟨m, ⟨d, hd, ht⟩, hu⟩

theorem a_marked_deck_or_an_ancestor_refuses_the_card : RefusesByEitherDeck admits := by
  intro marked tree home current h c hh hc
  have hiff := any_iff_markedOver marked tree h c
  simp only [admits, hh, hc]
  by_cases hm : MarkedOver marked tree h c
  · have hany := hiff.2 hm
    simp [hany, hm]
  · have hany : (marked.filterMap tree).any (fun m => under h m || under c m) = false := by
      cases hb : (marked.filterMap tree).any (fun m => under h m || under c m)
      · rfl
      · exact absurd (hiff.1 hb) hm
    simp [hany, hm]

theorem marking_more_decks_never_admits_a_refused_card : MarkingMoreNeverAdmits admits := by
  constructor
  · intro small big tree home current hsub hsmall
    cases hh : tree home with
    | none => simp [admits, hh]
    | some h =>
      cases hc : tree current with
      | none => simp [admits, hh, hc]
      | some c =>
        have hs := (a_marked_deck_or_an_ancestor_refuses_the_card small tree home current h c hh hc).2
        have hb := (a_marked_deck_or_an_ancestor_refuses_the_card big tree home current h c hh hc).2
        intro hbig
        apply hsmall
        apply hs.2
        rintro ⟨d, m, hd, ht, hu⟩
        exact (hb.1 hbig) ⟨d, m, hsub d hd, ht, hu⟩
  · intro marked tree home current hmem hhs hcs
    obtain ⟨h, hh⟩ := Option.isSome_iff_exists.1 hhs
    obtain ⟨c, hc⟩ := Option.isSome_iff_exists.1 hcs
    apply ((a_marked_deck_or_an_ancestor_refuses_the_card marked tree home current h c hh hc).1).2
    rcases hmem with hm | hm
    · exact ⟨home, h, hm, hh, Or.inl (Or.inl rfl)⟩
    · exact ⟨current, c, hm, hc, Or.inr (Or.inl rfl)⟩

theorem an_unresolved_deck_or_an_unreadable_set_is_refused :
    UnresolvedOrUnreadableIsRefused admits := by
  intro tree home current
  refine ⟨rfl, ?_⟩
  intro marked hn
  rcases hn with hn | hn
  · simp [admits, hn]
  · cases hh : tree home <;> simp [admits, hh, hn]

/-- One deck, id 1, named `A`; no other id resolves. -/
def oneDeck : Tree := fun d => if d = 1 then some ['A'] else none

theorem admitting_every_card_lets_a_marked_deck_through :
    ¬ RefusesByEitherDeck admitsAtTheBase := by
  intro hclaim
  have h := (hclaim [1] oneDeck 1 1 ['A'] ['A'] (by decide) (by decide)).1
  have hm : MarkedOver [1] oneDeck ['A'] ['A'] :=
    ⟨1, ['A'], by simp, by decide, Or.inl (Or.inl rfl)⟩
  exact absurd (h.2 hm) (by decide)

theorem admitting_every_card_lets_a_marked_home_deck_through :
    ¬ MarkingMoreNeverAdmits admitsAtTheBase := by
  intro hclaim
  have h := hclaim.2 [1] oneDeck 1 1 (Or.inl (by simp)) (by decide) (by decide)
  exact absurd h (by decide)

theorem admitting_every_card_lets_an_unresolved_deck_through :
    ¬ UnresolvedOrUnreadableIsRefused admitsAtTheBase := by
  intro hclaim
  have h := (hclaim oneDeck 2 2).2 [] (Or.inl (by decide))
  exact absurd h (by decide)

end Formal.SensitiveDeck

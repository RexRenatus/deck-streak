-- @phx covers scripts/mutation_carry.py anchor=union_rule digest=sha256:2347dc5048bffb5776d6681ed521d2476ed92c58181f41f2b0155e5cec36b8c9
-- @phx covers scripts/mutation_carry.py anchor=holds digest=sha256:f023c3ca05fc30d08731d4457aae5e9c8886cae76dd166644827f7e0b9a68a4c
-- @phx covers scripts/mutation_carry.py anchor=held_to_listing digest=sha256:b65bd3f766fb99bd76aacd0fc8964ed69bbd30bdf4ac50ccdf17a92b1866ba17
-- @phx covers scripts/mutation_carry.py anchor=read_set digest=sha256:7b173cc8e9e79a1f3792b2b0493052e16c838097555b6b181d2d177ebb213a87
-- @phx covers scripts/mutation_carry.py anchor=changes digest=sha256:d5c9f42e2f40b611163d48cc47ff0929cfca3cd36f5fc855be516a1c0ebc4b8c
-- @phx vectors formal/vectors/release-carry.jsonl
-- @phx cites #822
-- @phx theorem the_listing_is_rerun_plus_carried_once ramp=report
-- @phx witness an_overlap_rule_lists_a_name_in_both kills=the_listing_is_rerun_plus_carried_once
-- @phx theorem the_verdict_reads_only_the_listing ramp=report
-- @phx witness an_own_file_rule_carries_a_changed_read_set kills=the_verdict_reads_only_the_listing

/-!
# Formal.ReleaseCarry

The release carry's two pure decisions (#822, SPEC-493, ADR-511), ported from
`scripts/mutation_carry.py`.

**The union rule** (`union_rule`, R10). The verdict's listing for a class is the re-run legs'
names plus the carried ones, as multisets, with no name in both. `union_rule` returns `None`
exactly when that holds: no name is missing, none is extra, and none is in both. `Accepts` states
it as a `Prop`, and `accepts` is the decision. The witness `acceptsOverlap` drops the "none in
both" conjunct, the shape a carry takes when it writes `carry.json` and leaves its carried lines in
`code` so a carried name is re-run too: it accepts a name counted once by the re-run legs and once
by the carry, which the listing then holds twice, and `an_overlap_rule_lists_a_name_in_both` proves
it violates the claim at `["m"]`.

**The carry predicate** (`holds`, R8). A file carries only when no path, probe or listing of its
whole read set changed. `holds` refuses a diff it cannot read, carries nothing changed when the
read set is the whole tree, and otherwise lets no binary, changed or listed path of the read set
through. `Carries` states that decision and `holds` decides it. The claim
`the_verdict_reads_only_the_listing` (SPEC-493 section 8) is that a file that carries has no
changed path in its read set, so the carry reads only the listing. The witness `holdsOwnFileOnly`
is the own-file-only predicate, which looks at the file's own members alone: it carries a mutant
whose read set changed through a probe, and `an_own_file_rule_carries_a_changed_read_set` proves it
violates the claim.

**What is ported.** `union_rule`'s three checks (missing, extra, both) as multiset counts over the
names present, and `holds`' branches with its inner `hit` over the read set's members, probes,
directory prefixes and globs. **What is not ported.** `read_set` (R6, R7) and `changes`, which
build the read set and the diff `holds` reads, and `held_to_listing`, which builds the listed,
re-run and carried multisets `union_rule` reads: each is an input here, covered so a move of its
span re-reads this port. Every glob the read set builds is `<probe>.*` (`read_set`), so `fnmatch`
reduces to a prefix test with the trailing `*` dropped.
-/

namespace Formal.ReleaseCarry

/-- A name is read as its characters, as `union_rule` compares `str` keys. -/
abbrev Name := List Char

/-! ## The union rule -/

/-- How many times `n` appears in a multiset of names, as a `Counter` counts it. -/
def cnt (n : Name) : List Name → Nat
  | [] => 0
  | m :: ms => (if m = n then 1 else 0) + cnt n ms

/-- The names present across the listing, the re-run legs and the carried ones. -/
def relevant (listed rerun carried : List Name) : List Name :=
  listed ++ rerun ++ carried

/-- `union_rule` returns `None` exactly here: over every name present, the listing's count is the
re-run count plus the carried count (no name missing or extra), and no re-run name is carried
(none in both). -/
def Accepts (listed rerun carried : List Name) : Prop :=
  (∀ n ∈ relevant listed rerun carried, cnt n listed = cnt n rerun + cnt n carried) ∧
    (∀ n ∈ rerun, cnt n carried = 0)

/-- The union rule that ships. -/
def accepts (listed rerun carried : List Name) : Bool :=
  decide (Accepts listed rerun carried)

/-- An overlap rule that drops "none in both": it accepts a listing even when a name is both
re-run and carried. -/
def AcceptsOverlap (listed rerun carried : List Name) : Prop :=
  ∀ n ∈ relevant listed rerun carried, cnt n listed = cnt n rerun + cnt n carried

/-- The weakened rule that ignores names in both. -/
def acceptsOverlap (listed rerun carried : List Name) : Bool :=
  decide (AcceptsOverlap listed rerun carried)

/-- The claim over a union rule: when it accepts, the listing is the re-run names plus the carried
ones, each counted once, with none in both. -/
def Claim (f : List Name → List Name → List Name → Bool) : Prop :=
  ∀ listed rerun carried, f listed rerun carried = true → Accepts listed rerun carried

/-- The union rule that ships lists each re-run name and each carried name once, none in both. -/
theorem the_listing_is_rerun_plus_carried_once : Claim accepts := by
  intro listed rerun carried h
  unfold accepts at h
  exact of_decide_eq_true h

/-! ### The witness: an overlap rule lists a name in both -/

/-- The overlap rule accepts the listing `["m", "m"]` built from the re-run leg `["m"]` and the
carry `["m"]`, so the name `"m"` is listed twice, in both: the claim is false of it. -/
theorem an_overlap_rule_lists_a_name_in_both : ¬ Claim acceptsOverlap := by
  intro h
  have hA := h [['m'], ['m']] [['m']] [['m']] (by decide +kernel)
  exact absurd hA.2 (by decide +kernel)

/-! ## The carry predicate -/

/-- A read set `S(f)`: its closure members, probe paths, directory trees, globs and listings, and
whether it is the whole tree. -/
structure ReadSet where
  members : List Name
  paths : List Name
  trees : List Name
  globs : List Name
  listings : List Name
  whole : Bool

/-- A diff: the sections a reader cannot read, the changed paths, the binary ones and the changed
listings. -/
structure Changes where
  unread : List Name
  changed : List Name
  binary : List Name
  listings : List Name

/-- `hit`'s tree arm: the path is the tree, the tree is the root `.`, or the tree is a directory
prefix of the path. -/
def prefixTree (path tree : Name) : Bool :=
  (path == tree) || (tree == ['.']) || (tree ++ ['/']).isPrefixOf path

/-- `hit`'s glob arm: every glob is `<probe>.*`, so `fnmatch` is the pattern without its `*` as a
prefix of the path. -/
def globHit (path g : Name) : Bool :=
  g.dropLast.isPrefixOf path

/-- `hit`: whether a path is a member, a probe, under a tree or under a glob of the read set. -/
def hit (rs : ReadSet) (path : Name) : Bool :=
  rs.members.contains path || rs.paths.contains path ||
    rs.trees.any (prefixTree path) || rs.globs.any (globHit path)

/-- Whether a changed path is in the read set: everything when the set is the whole tree, else
`hit`. -/
def inReadSet (rs : ReadSet) (path : Name) : Bool :=
  rs.whole || hit rs path

/-- `holds` carries the file exactly here: the diff reads; no changed path is in the read set (the
whole-tree case needs no changed path, since everything is in it); and, when the set is not the
whole tree, no binary path and no changed listing of the read set got through. -/
def Carries (rs : ReadSet) (ch : Changes) : Prop :=
  ch.unread = [] ∧
    (∀ p ∈ ch.changed, inReadSet rs p = false) ∧
    (rs.whole = true ∨
      ((∀ p ∈ ch.binary, hit rs p = false) ∧
        ∀ l ∈ ch.listings, rs.listings.contains l = false ∧ hit rs l = false))

/-- The carry predicate that ships. -/
def holds (rs : ReadSet) (ch : Changes) : Bool :=
  decide (Carries rs ch)

/-- The own-file-only predicate: it carries whenever no changed path is one of the file's own
members, ignoring the rest of the read set. -/
def holdsOwnFileOnly (rs : ReadSet) (ch : Changes) : Bool :=
  decide (ch.unread = [] ∧ ∀ p ∈ ch.changed, rs.members.contains p = false)

/-- The claim over a carry predicate: when it carries, no changed path is in the read set, so the
verdict reads only the listing. -/
def Reads (f : ReadSet → Changes → Bool) : Prop :=
  ∀ (rs : ReadSet) (ch : Changes), f rs ch = true → ∀ p ∈ ch.changed, inReadSet rs p = false

/-- The carry predicate that ships reads only the listing. -/
theorem the_verdict_reads_only_the_listing : Reads holds := by
  intro rs ch h p hp
  have hc : Carries rs ch := by
    unfold holds at h
    exact of_decide_eq_true h
  exact hc.2.1 p hp

/-! ### The witness: an own-file-only rule carries a changed read set -/

/-- The own file `scripts/a.py` is unchanged, but its probe `scripts/dep.py`, a path of the read
set, changed. -/
def witnessRS : ReadSet :=
  { members := [['a']], paths := [['d']], trees := [], globs := [], listings := [], whole := false }

/-- The diff: the probe changed. -/
def witnessCh : Changes :=
  { unread := [], changed := [['d']], binary := [], listings := [] }

/-- The own-file-only rule carries a mutant whose read set changed through its probe, so the claim
is false of it. -/
theorem an_own_file_rule_carries_a_changed_read_set : ¬ Reads holdsOwnFileOnly := by
  intro h
  have hbad := h witnessRS witnessCh (by decide +kernel) ['d'] (by decide +kernel)
  exact absurd hbad (by decide +kernel)

end Formal.ReleaseCarry

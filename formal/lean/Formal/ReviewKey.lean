-- @phx covers crates/sync/src/key.rs anchor=sort_keys digest=sha256:644c2b6438b8338ff7bf846e061575d0205c984134988fa7fb798535cbfd52ac
-- @phx covers crates/sync/src/key.rs anchor=compare digest=sha256:471a3c7435f96360232781932b72560b51c34f5ae5cbf7f9cfff937bd69457d3
-- @phx cites #15
-- @phx vectors formal/vectors/review-key.jsonl
-- @phx theorem fold_sorted_perm_invariant ramp=report
-- @phx witness a_card_folded_in_server_order kills=fold_sorted_perm_invariant

import Formal.SyncOrder

/-!
# Formal.ReviewKey

The card's canonical fold, written ahead of its code (SPEC-504, ADR-522). A card's counters and
due state are folded from its reviews in key order, (hlc, origin, seq), whatever order the server's
log delivered them in. `sortKeys` places each review by its key (`ins`), and the claim
`FoldPermInvariant` is that any fold over the sorted reviews gives one answer for every arrival
order of the same set of distinct reviews. `sortServerOrder`, which keeps the arrival order, is
refuted at two reviews delivered in both orders. The sync core's `sort_keys` and the key order it
sorts by, `compare` (SPEC-509), are the covered code, and its test reads
`formal/vectors/review-key.jsonl` byte-equal.
-/

namespace Formal.ReviewKey

open Formal.SyncOrder

/-- Place a review by its key in a key-ordered list. -/
def ins (a : Key) : List Key → List Key
  | [] => [a]
  | x :: s => if Key.lt a x then a :: x :: s else x :: ins a s

/-- The reviews in key order. -/
def sortKeys : List Key → List Key
  | [] => []
  | x :: s => ins x (sortKeys s)

/-- Any fold over the sorted reviews depends only on the set of reviews. -/
def FoldPermInvariant (sort : List Key → List Key) : Prop :=
  ∀ (step : Nat → Key → Nat) (init : Nat) (l1 l2 : List Key),
    l1.Perm l2 → l1.Nodup → (sort l1).foldl step init = (sort l2).foldl step init

theorem ins_comm (a b : Key) (hab : a ≠ b) : ∀ s, ins a (ins b s) = ins b (ins a s)
  | [] => by
    rcases Key.lt_total hab with h | h
    · simp [ins, h, Key.lt_asymm h]
    · simp [ins, h, Key.lt_asymm h]
  | x :: s => by
    by_cases hb : Key.lt b x = true
    · by_cases ha : Key.lt a x = true
      · rcases Key.lt_total hab with h | h
        · simp [ins, ha, hb, h, Key.lt_asymm h]
        · simp [ins, ha, hb, h, Key.lt_asymm h]
      · have hab' : Key.lt a b = false := by
          cases hlt : Key.lt a b
          · rfl
          · exact absurd (Key.lt_trans hlt hb) ha
        simp [ins, ha, hb, hab']
    · by_cases ha : Key.lt a x = true
      · have hba' : Key.lt b a = false := by
          cases hlt : Key.lt b a
          · rfl
          · exact absurd (Key.lt_trans hlt ha) hb
        simp [ins, ha, hb, hba']
      · simp [ins, ha, hb, ins_comm a b hab s]

theorem sortKeys_perm {l1 l2 : List Key} (p : l1.Perm l2) (hn : l1.Nodup) :
    sortKeys l1 = sortKeys l2 := by
  induction p with
  | nil => rfl
  | cons x _ ih =>
    simp only [sortKeys]
    rw [ih (List.nodup_cons.mp hn).2]
  | swap x y l =>
    have hyx : y ≠ x := by
      intro h
      subst h
      simp at hn
    simp only [sortKeys]
    exact ins_comm y x hyx _
  | trans p12 _ ih12 ih23 =>
    rw [ih12 hn, ih23 (p12.nodup_iff.mp hn)]

theorem fold_sorted_perm_invariant : FoldPermInvariant sortKeys := by
  intro step init l1 l2 p hn
  rw [sortKeys_perm p hn]

/-- The defect: the card folded in the order the server's log delivered its reviews. -/
def sortServerOrder (l : List Key) : List Key := l

theorem a_card_folded_in_server_order : ¬ FoldPermInvariant sortServerOrder := by
  intro h
  have := h (fun _ k => k.hlc) 0 [⟨1, 0, 0⟩, ⟨2, 0, 0⟩] [⟨2, 0, 0⟩, ⟨1, 0, 0⟩]
    (List.Perm.swap _ _ _) (by decide)
  simp [sortServerOrder] at this

end Formal.ReviewKey

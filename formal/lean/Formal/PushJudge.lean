-- @phx cites #15
-- @phx vectors formal/vectors/push-judge.jsonl
-- @phx theorem judge_idempotent ramp=report
-- @phx witness a_judge_with_no_origin_seq_key kills=judge_idempotent
-- @phx theorem no_review_mutation ramp=report
-- @phx witness a_judge_that_rewrites_a_committed_row kills=no_review_mutation

/-!
# Formal.PushJudge

The server's judge of one pushed operation, ahead of its code (SPEC-504, ADR-522). It decides the
review ledger that XP and coins read. The judge refuses an operation from a revoked device, answers
Duplicate for an operation whose (origin, seq) key the log already holds, and otherwise appends it.
The claims: judging the same operation again changes nothing and reads Duplicate
(`Idempotent`), and no verdict changes a row already in the log (`NoReviewMutation`, the log before
is a prefix of the log after). Each is refuted, at a concrete input, of the defect its witness
names: a judge that reads no key and appends again, and a judge that rewrites the row with the same
key. No code in the tree judges a push yet; the sync core that will gains a `covers` line on its
judge and reads `formal/vectors/push-judge.jsonl` byte-equal.
-/

namespace Formal.PushJudge

/-- A pushed operation: its origin device, its sequence number and its payload. -/
structure Op where
  origin : Nat
  seq : Nat
  body : Nat
  deriving DecidableEq, Repr

inductive Verdict where
  | accepted
  | duplicate
  | rejected
  deriving DecidableEq, Repr

/-- Two operations share the (origin, seq) key. -/
def keyed (o p : Op) : Bool := o.origin == p.origin && o.seq == p.seq

/-- The judge: the token, then the key, then the append. -/
def judge (revoked : List Nat) (log : List Op) (o : Op) : Verdict × List Op :=
  if revoked.contains o.origin then (.rejected, log)
  else if log.any (keyed o) then (.duplicate, log)
  else (.accepted, log ++ [o])

abbrev Judge := List Nat → List Op → Op → Verdict × List Op

/-- Judging the same operation twice: the second verdict is Duplicate or the refusal again, and
the log is unchanged by it. -/
def Idempotent (j : Judge) : Prop :=
  ∀ revoked log o,
    (j revoked (j revoked log o).2 o).2 = (j revoked log o).2 ∧
    (j revoked (j revoked log o).2 o).1 ≠ .accepted

/-- No verdict changes a row already in the log. -/
def NoReviewMutation (j : Judge) : Prop :=
  ∀ revoked log o, log <+: (j revoked log o).2

theorem keyed_self (o : Op) : keyed o o = true := by
  simp [keyed]

theorem any_append_self (log : List Op) (o : Op) : (log ++ [o]).any (keyed o) = true := by
  rw [List.any_append]
  simp only [List.any_cons, List.any_nil, keyed_self, Bool.or_false, Bool.or_true]

theorem judge_idempotent : Idempotent judge := by
  intro revoked log o
  by_cases hr : revoked.contains o.origin = true
  · simp only [judge, hr, ↓reduceIte]
    simp
  · by_cases hd : log.any (keyed o) = true
    · simp only [judge, hr, hd, ↓reduceIte]
      simp_all
    · simp only [judge, hr, hd]
      simp_all [keyed_self]

theorem no_review_mutation : NoReviewMutation judge := by
  intro revoked log o
  by_cases hr : revoked.contains o.origin = true
  · simp only [judge, hr, ↓reduceIte]
    exact List.prefix_refl log
  · by_cases hd : log.any (keyed o) = true
    · simp only [judge, hr, hd, ↓reduceIte]
      exact List.prefix_refl log
    · simp only [judge, hr, hd]
      exact List.prefix_append log [o]

/-- The defect: a judge that reads no (origin, seq) key and appends every push. -/
def judgeNoKey (revoked : List Nat) (log : List Op) (o : Op) : Verdict × List Op :=
  if revoked.contains o.origin then (.rejected, log) else (.accepted, log ++ [o])

/-- The defect: a judge that rewrites the row with the same key instead of answering Duplicate. -/
def judgeRewrite (revoked : List Nat) (log : List Op) (o : Op) : Verdict × List Op :=
  if revoked.contains o.origin then (.rejected, log)
  else if log.any (keyed o) then (.accepted, log.map fun p => if keyed o p then o else p)
  else (.accepted, log ++ [o])

theorem a_judge_with_no_origin_seq_key : ¬ Idempotent judgeNoKey := by
  intro h
  have := (h [] [] ⟨1, 1, 0⟩).1
  simp [judgeNoKey] at this

theorem a_judge_that_rewrites_a_committed_row : ¬ NoReviewMutation judgeRewrite := by
  intro h
  have := h [] [⟨1, 1, 0⟩] ⟨1, 1, 5⟩
  simp [judgeRewrite, keyed] at this

end Formal.PushJudge

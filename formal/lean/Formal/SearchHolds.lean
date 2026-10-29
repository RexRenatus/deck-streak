-- @phx covers crates/search/src/skip.rs anchor=query digest=sha256:039246c27c23186d74c044c0ed4ceaf6cc53c4d40d13702f0737c382428ddad2
-- @phx covers crates/search/src/skip.rs anchor=holds digest=sha256:f3d918c8a64a4cfc3872cef45bdacdeb29c0271c9283d5ad2336f49da243dd48
-- @phx cites #108
-- @phx theorem holds_inescapable ramp=report
-- @phx witness a_text_wrap_a_closing_parenthesis_escapes kills=holds_inescapable

/-!
# Formal.SearchHolds

The skip day's wrap (SPEC-514 R8 and A4; ADR-532 D4): `SkipQuery::query` in
`crates/search/src/skip.rs` joins the configured query and the holds `holds` as the operands of
one `and`, and `eval` in `crates/search/src/eval.rs` folds a card's answer for each term through
the connectives.

**The claim.** For every configured query and every card:
- `holds_inescapable`: a card the wrapped query selects meets every hold: it is a review card,
  due today, neither suspended nor buried, and in no filtered deck. No configured query, however
  it nests its negations, `and`s and `or`s, selects a card past them.

**What is ported.** A query is a term, a negation, or an `and` or `or` of its operands; the Rust
`And` and `Or` take a list, ported here as a right-nested pair, which `eval`'s `all` and `any`
fold to the same answer. A term's answer for one card is any function from the term to a
boolean, so the claim holds whatever the evaluator answers for each term: the six holds are six
atoms, and every configured term is an atom of its own. `holds` is the six holds in their written
order, and the wrap is the configured query `and` the holds.

**What is not ported.** The parse: that the text the engine runs reads back to the wrapped query,
and that a configured text which closes the wrap's group is refused before it is wrapped, are the
Rust tests' (`crates/search/tests/holds.rs`). What each term answers is the evaluator's, held to
the engine's goldens (`crates/search/tests/eval_golden.rs`).

**The witness** is the query a text wrap gives a configured text that closes the wrap's group,
`X) or (Y`: the parse of `(X) or (Y) -is:new ...` is `X` or (`Y` and the holds), so the holds bind
to the last disjunct only. At the merge-base the configured text was wrapped as text, guarded by a
balance check alone (`crates/ingest/src/skip.rs`, `is_one_expression`).
-/

namespace Formal.SearchHolds

/-- A term as the wrap sees it: one of the six holds' own terms, or a configured term. -/
inductive Atom where
  | newCard
  | learnCard
  | dueToday
  | suspended
  | buried
  | filtered
  | configured (n : Nat)

/-- `Query`: a term, its complement, and an `and` or an `or` of two operands. -/
inductive Query where
  | term : Atom → Query
  | not : Query → Query
  | and : Query → Query → Query
  | or : Query → Query → Query

/-- `eval` for one card: the card's answer for each term, folded through the connectives. -/
def eval (atom : Atom → Bool) : Query → Bool
  | .term a => atom a
  | .not q => !eval atom q
  | .and p q => eval atom p && eval atom q
  | .or p q => eval atom p || eval atom q

/-- `holds`: not new, not learning, due today, not suspended, not buried, in no filtered deck. -/
def holds : Query :=
  .and (.not (.term .newCard))
    (.and (.not (.term .learnCard))
      (.and (.term .dueToday)
        (.and (.not (.term .suspended))
          (.and (.not (.term .buried)) (.not (.term .filtered))))))

/-- `SkipQuery::query`: the configured query and the holds, the operands of one `and`. -/
def skipWrap (q : Query) : Query :=
  .and q holds

/-- The wrong variant: the parse of a text wrap of `X) or (Y`, with `X` and `Y` the same query;
the holds bind to the last disjunct only. -/
def textWrapOfAClosingParenthesis (q : Query) : Query :=
  .or q (.and q holds)

/-- The claim, stated once: whatever the configured query, a card the wrap selects meets every
hold. -/
def Inescapable (wrap : Query → Query) : Prop :=
  ∀ (atom : Atom → Bool) (q : Query), eval atom (wrap q) = true → eval atom holds = true

/-- A card that answers every configured term and no hold's term. -/
def configuredOnly : Atom → Bool
  | .configured _ => true
  | _ => false

theorem holds_inescapable : Inescapable skipWrap := by
  intro atom q h
  simp only [skipWrap, eval, Bool.and_eq_true] at h
  exact h.2

theorem a_text_wrap_a_closing_parenthesis_escapes :
    ¬ Inescapable textWrapOfAClosingParenthesis := by
  intro h
  have := h configuredOnly (.term (.configured 0)) (by decide)
  revert this
  decide

end Formal.SearchHolds

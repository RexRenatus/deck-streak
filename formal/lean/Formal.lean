import Formal.NextMilestone

/-!
# Formal

The root of the Lake package that holds DeckStreak's Lean proofs (#472). Lean proves pure
functions only, each as a faithful port that cites the source span it ports. An entry is a module
`Formal/<E>.lean` that carries header lines; the checker's registry derives it from the tree, and
`lakefile.toml`'s glob builds it without a typed list.
-/

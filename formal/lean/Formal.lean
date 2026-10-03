import Formal.NextMilestone
import Formal.RoadToC2
import Formal.Wallet

/-!
# Formal

The root of the Lake package that holds DeckStreak's Lean proofs: the next milestone's (#76), the
open lapse's (#472), the coin wallet's (#106) and Road to C2's band rules (#85). Lean proves pure
functions only, each as a faithful port that cites the source span it ports. An entry is a module
`Formal/<E>.lean` that carries header lines; the checker's registry derives it from the tree, and
`lakefile.toml`'s glob builds it without a typed list.
-/

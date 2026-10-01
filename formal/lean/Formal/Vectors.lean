import Formal.NextMilestoneVectors

/-!
# Formal.Vectors

The vector writer. Run from `formal/lean` as `lake env lean --run Formal/Vectors.lean <Entry>`, it
prints the entry's vectors as JSON lines on stdout: one header line naming the covered item and the
digest the entry recorded for it, then one line per input with the port's own answer.
`phxd formal check` byte-compares the output with the committed file, and `--write-vectors` writes
it; the file is never edited by hand.

Each entry's writer lives in a support module of its own, and `main` gains a one-line arm for it,
so two entries' arms merge by keeping both. This module carries no header line, so the registry
lists it as support.
-/

def main (args : List String) : IO UInt32 := do
  match args with
  | ["NextMilestone"] => Formal.NextMilestoneVectors.run
  | _ =>
    IO.eprintln s!"usage: lean --run Formal/Vectors.lean <Entry>; no vectors for {args}"
    return 2

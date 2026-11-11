### Changed

- The token compiler's tests now kill every mutant StrykerJS makes on its older lines but two
  equivalent ones: each refusal by its exact message, a small document's whole stylesheet, each
  Telegram theme parameter and CSS generic family by name, and the one read and one write of
  `writeTokens`. The colour-mode tests compile the tokens inside each test, so the compiler's mutants
  are no longer static (SPEC-636 A3).

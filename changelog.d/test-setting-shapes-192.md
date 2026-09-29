### Fixed

- Every setting's stated shape (the words a refused value is answered with) is now pinned by a test
  that spells it, so rewording one fails the build; a guard refuses a new setting (generic or macro-written
  included; comments of both forms and code after a test module do not count as a spelling) whose shape no test of its crate and no row on its own file spells, and one whose
  literal two settings of a crate share without a row on each file.

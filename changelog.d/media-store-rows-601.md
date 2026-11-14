### Changed

- Every changed line of the media store that no generated mutant reaches now carries a hand-written mutation row: the
  export, erase and statements of the three media tables, the second migration and the crate roots' declarations,
  each with a test that fails on its mutant (SPEC-601, ADR-621).

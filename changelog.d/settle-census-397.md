### Fixed

- The settle census (SPEC-072 A12, issue 397) now reads progression's own re-exports. A renamed
  `pub use` of `settle`, its request or its module, however grouped, nested or chained, and a
  crate alias or a type alias of them, no longer lets a caller outside coordination pass by
  naming only the new names; the refusal names the file, the alias and the original (ADR-197).

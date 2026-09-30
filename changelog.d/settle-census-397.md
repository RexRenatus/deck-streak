### Fixed

- The settle census (SPEC-072 A12, issue 397) now reads progression's own re-exports. A renamed
  `pub use` of `settle`, its request or its module, however grouped, nested or chained, a crate
  alias in each binding form the census's test generates (a `use` or `extern crate` alias, raw,
  grouped or chained, a glob, a manifest's rename), another member's re-export of the crate or its
  operation, or a type alias of them, no longer lets a caller outside coordination pass by naming
  only the new names, and no comment between a path's tokens hides one; the refusal names the file,
  the alias and the original, and a manifest the census cannot read is refused (ADR-197).

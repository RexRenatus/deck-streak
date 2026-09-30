### Fixed

- The settle census (SPEC-072 A12, issue 397) is now the compiler. Progression's `settle` carries a
  deprecation under a cfg that progression's build script sets only for the census, and the census
  has cargo check every target of every workspace package in four passes (debug assertions on and
  off in every package, each with the unwind and the abort panic strategy), and again over libraries
  and binaries alone when a member has a dev-dependency, with the deprecation forced to warn. So
  rustc names every caller outside progression however it reaches `settle`: through a re-export, an
  alias, a glob, a macro, an `include!`, a manifest's rename, a test, a bench, an example or a build
  script. A caller outside coordination is refused by file, and coordination's callers keep the
  owner's-correction rule. What the compiler is not asked to compile (a member's feature, a
  proc-macro member, a package outside the workspace, a cargo configuration) is refused by name
  (ADR-197).

- The census now reads the resolve graph (`cargo metadata --locked --offline`) and refuses, by name,
  every package that has a build script and can name `settle`: the package that defines it, or one
  that depends on it by a normal, a build or a dev edge. Progression's own script is admitted at
  one pinned SHA-256, and a graph cargo cannot give is a refusal. A build script's cfg in a package
  that cannot name `settle`, read by a macro that package exports, and an `include!` of a
  recompute file from outside the folder remain disclosed by kind (SPEC-072 section 12, ADR-197).
  The census's target is kept between CI runs, and the killer builds its stub once for each worker.

### Added

- Three fitness tests hold the architecture's standing facts to listed rows. Every internal
  dependency between workspace members, in each dependency table and target table, has one row in
  `config/crate-edges.json` that cites an ADR naming the edge. Every implemented Lean entry has a
  vectors test that reads its vectors, or a declared row in `config/vectors-parity.json`, and every
  vectors test names a live entry. Every production Rust file's clock reads equal its row's count in
  `config/clock-reads.json`. Each census prints what it examined and refuses an empty population
  (SPEC-431).

### Changed

- The workflow guards now read every suffix, spelling and word: each scan of the workflows reads
  both `.yml` and `.yaml` files (#393), the verdict test's judge-line reader stops at redirections
  and substitutions (#394), and the dispatch-shard guard finds `cargo +<toolchain> mutants`, the
  `cargo-mutants mutants` binary form and a second command on a line (#395). SPEC-038, SPEC-126
  and SPEC-129 each take an insert-only amendment.

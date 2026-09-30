### Changed

- The workflow guards now read every suffix, spelling and word: each scan of the workflows reads
  both `.yml` and `.yaml` files (#393), the verdict test's judge-line reader stops at redirections
  and substitutions, inside double quotes too (#394), and the dispatch-shard guard finds
  `cargo +<toolchain> mutants`, the `cargo-mutants mutants` binary form, a cargo flag's separate
  value and a second command on a line, because it reads each `run:` value as YAML and bash read
  it, or refuses it (#395, #447). SPEC-038, SPEC-126 and SPEC-129 each take an insert-only
  amendment against dev.
- The same guard refuses a computed word before the cargo bounds that bash could expand to exactly
  `--` (a word with an unquoted expansion, or an expansion and no literal besides `-`), and the
  weekly mutation sweep names its package in literal words, pinned by a test to select what it
  did before (#395, #447).

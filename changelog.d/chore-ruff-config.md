### Changed

- A `ruff.toml` sets 100-column Python lines. It keeps `ruff check --fix` from deleting an import
  or a variable that a red-first stub leaves unused, and it excludes the files whose bytes are pinned
  (the vendored probes, and the parity oracle's generator and registry), even when they are named
  directly. The project's own scripts are formatted once, so a formatter run on an unchanged file is
  now a no-op.

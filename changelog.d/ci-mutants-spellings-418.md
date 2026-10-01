### Changed

- Both scans of the workflow tests that look for `cargo mutants` now read each workflow through the
  one finder the dispatch-shard guard uses, so a command spelled with a toolchain selector, the
  hyphenated binary, repeated blanks, an option before the subcommand or at the end of a line is
  found by both. A planted workflow of each spelling must be found by each scan.
- A test double that cannot plant its seam now exits non-zero and names the failure, and runs
  nothing after it, where it used to run the real program. A text-only census lists each such
  fallback it reaches in the top-level `*.py` files of `scripts/tests/`, within 12 lines of the
  handler.

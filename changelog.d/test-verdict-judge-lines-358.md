### Changed

- The verdict test now reads each `mutation-verdict.py judge` command line by its `--class` and
  asserts the flags and paths of the `rust` and `oracle` lines separately, so a flag moved between
  the two lines or dropped from one no longer passes (#358). SPEC-126 takes an insert-only
  amendment that restates A4 and corrects two of its statements.

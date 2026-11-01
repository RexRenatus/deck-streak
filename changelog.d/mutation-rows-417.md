### Changed

- A pull request's hand-proved mutation rows are dealt into the fewest legs that each fit within
  the rows job's 90 minutes, at most 17, so a large selection is no longer cut at the job's limit.
  A selection that needs more legs is refused whole, by name, before any leg starts, and the
  verdict merges every leg's report and names a leg that sent none (SPEC-417).

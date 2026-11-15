### Changed

- The mutation plan job now names each priced Python file whose suite grew since its price run.
  A new verb, `mutation-verdict.py stale`, reads every line of `PYTHON_CONTROL_SECONDS` and
  `PYTHON_SCOPED_SECONDS`, counts the tests each file's mapped modules define from their syntax
  trees, and compares that count with the largest `ran` of the file's control records under the
  run its row names. A file whose pricing row is stale gets one line that names both counts, the
  run and the table, and the refresh by `mutation-prices.yml` before the release run; a row the
  tree cannot read is named unread with its reason. The step prints a summary and its examined
  count, reads VOID when it examined nothing, and never fails the job (SPEC-429).

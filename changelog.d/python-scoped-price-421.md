### Changed

- Each Python file's mutation price is now its scoped test cost, measured by a dispatch-only
  workflow, `mutation-prices.yml`, that runs one leg per file with `--scoped --sample 20`, and
  whose records each name the job that wrote them. The `prices --scoped` verb reads a price run's
  records into each file's control, sentinel and mean scoped mutant seconds, and reads VOID over a
  record with no run or no job, a whole run's mutant record, one path across two jobs or two
  attempts, and a download with no record. The plan prices a file by its triple in
  `PYTHON_SCOPED_SECONDS` when the legs trace, else by its whole price, else by the count rule,
  and deals its mutants by the triple's setup and mutant figure. The table is the verb's output
  over a committed price run's records (SPEC-421).

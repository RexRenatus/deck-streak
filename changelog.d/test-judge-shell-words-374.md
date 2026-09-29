### Changed

- The verdict test now reads each `mutation-verdict.py judge` command as the shell splits it: quotes
  removed, a `$` in single quotes or escaped kept apart from an expansion, continuations joined and
  `--flag=value` split. An equivalent spelling passes and every wrong path is still refused (#374).
  SPEC-126 takes an insert-only amendment that adds two criteria.

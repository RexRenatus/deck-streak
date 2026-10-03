### Fixed

- A change to a guard script's docstrings alone now reads a named `docstring-only` case in the mutation plan and verdict, never VOID: the plan compares each changed script's syntax tree with its base's once docstrings are set aside, and any other difference, or a script added, deleted, not UTF-8 or that does not parse, keeps the `scripts` class applying (#485, ADR-307).
- The mutation verdict script's description of its plan now names the two jobs whose skip the final check admits, and every step output the plan writes (#455).

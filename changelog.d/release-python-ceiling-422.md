### Changed

- The release pull request from `dev` into `main` is named by a fail-closed predicate in the
  mutation plan, and its Python legs may pass the share up to one ceiling constant,
  `PYTHON_RELEASE_CEILING`, that an owner-signed ruling file sets and the tree cites by path. A
  release listing past the ceiling is refused whole, by name, never capped. The legs past
  `PYTHON_MATRIX_LEGS` run in a second matrix job, `mutation-python-release`, and the verdict and
  `ci` judge both matrices; every other run keeps the share and both of its refusals. A
  workflow, `matrix-limit.yml`, measures the job limit per matrix, and a test reads its
  committed record (SPEC-422).

### Changed

- The release pull request's mutation run, and only that run, may plan up to 305 Python legs,
  above `PYTHON_SHARE_CAP` 128, and up to 192 Rust legs, above `LEG_CEILING["ci"]` 190, by a ruling
  the owner signs in `docs/rulings/` (#850). It supersedes the 303 Python figure. This pull request
  adds only that ruling and changes no code, test, row or assertion; the change that applies it
  lands with #849.

### Changed

- A release is published only when its release gates are on file and hold: the release job runs
  `scripts/release_gates.py` directly before its publish step, which reads `release-gates.json` at
  the tag's commit and refuses, one line each and by name, every gate that is missing or failed, and
  a record that is not on file. A refusal fails the job there, so the release stays a draft; every
  other release step runs as before. The Lean entry `ReleaseGates` proves the decision, and its
  vectors are read against the script.

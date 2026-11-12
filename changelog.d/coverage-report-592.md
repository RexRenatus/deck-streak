### Added

- An advisory coverage report: `.github/workflows/coverage.yml` measures the Rust workspace, the web
  app and the repository Python on every pull request into dev or main and on every push to dev, and
  prints each pull request changed-line figure as integer counts, "changed executable lines N,
  executed E, not executed: file:line ...". No ruleset and no `needs:` names its jobs, so it never
  gates a merge; a lane fails only when its instrument is broken (SPEC-592, ADR-613).
- `config/coverage.json`, the one exclusion list: each row holds one glob with its reason, and the
  deprecated Telegram code is excluded by rows citing ADR-589.
- The coverage provider dependencies are audited where every dependency is, under the advisories
  rule, not under a coverage figure.

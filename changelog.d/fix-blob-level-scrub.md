### Security

- The public scrub reads every blob of the history a change publishes. It refuses committed
  binaries and oversize files, and a shallow checkout reads VOID instead of green (SPEC-033).

### Fixed

- The committed release-tag ruleset uses only rules that GitHub enforces on this repository's plan.
  Dependabot now holds semver-major updates for cargo and npm until the house radar moves them.

### Added

- The parity oracle takes one registry module per SPEC, of three kinds (a function, an adapter
  that builds arguments JSON cannot carry, or constants), and its first golden: the predecessor's
  study day over 50 seeded synthetic cases at the rollover, before the Unix epoch and across UTC
  offsets. Every golden records the digests of the generator and of the registry module that
  built it, so a stale golden fails the gate, and no golden holds a date string.
- One Rust reader for the goldens, compiled into each proving crate's tests by path (ADR-029).
  `serde` and `serde_json` enter the workspace as test dependencies only.

### Fixed

- The oracle's generator tests remove the temporary directories they create.

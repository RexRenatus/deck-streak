### Fixed

- Every CI job that proves mutation rows now runs `cargo fetch --locked` after its cache restore and
  before `prove` (#606): a cache restored by key prefix can lack a package the current `Cargo.lock`
  adds, and the rows runner's offline census then read every row VOID. A test over the workflow
  files fails when a proving job lacks the step, orders it outside that window or lets it swallow
  its exit code (SPEC-332, ADR-333).

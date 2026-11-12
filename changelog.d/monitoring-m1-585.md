### Added

- A pager off the host: `docs/monitoring/pager.json` declares the pages a person receives (the
  alert template's line, a failed alert instance, a silent SLO evaluator and a late daily backup),
  the two channels a page reaches, the 14-day window of the shipped lines and the request counters,
  which carry only the unit, the route's pattern, the method and the status class (SPEC-585,
  ADR-604, #1069).
- The privacy page's `## Measuring use`: the server counts its requests without any account,
  address, device or content, the apps measure nothing about how they are used, and any later
  measurement is off until the learner turns it on by one switch.

### Changed

- Every alert of `deploy/slo.json` and its alert channel route to `pager`.
- SPEC-031's R2 and R3 are amended for the pager; SPEC-396 and ADR-410, the second route, are
  superseded by it, and the second route's script, model and tests stay until their retirement.

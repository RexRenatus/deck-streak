### Added

- The MCP server's unit has an SLO of its own, `mcp-availability` (0.95 over 28 days, the API's
  response-event indicator and alert windows), so the observability probe's `obs.slo-declared`
  finds it covered; `deploy/README.md`'s host budget names all four daemons whose `CPUQuota=` the
  share test sums (SPEC-333, issue #157).

### Changed

- The card frame's host now sandboxes its one inner frame with the card frame's own empty
  `sandbox`, so both frames carry the attribute, and the Mini App's sink census holds an `iframe`
  without `sandbox` at none in the shipped code under `web/app/src`, proved by its own plants and
  near misses (SPEC-425).

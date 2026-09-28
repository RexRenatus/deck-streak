### Security

- The Mini App's development tools no longer resolve `cookie` 0.6.0 or `qs` 6.15.1. Two scoped pnpm
  overrides move them to 0.7.2 and 6.16.0, which fix GHSA-pxg6-pf52-xh8x, and GHSA-q8mj-m7cp-5q26,
  GHSA-4mjr-xmp4-gh2g and GHSA-x5fp-wj9c-mxmx, because no update inside SvelteKit's or StrykerJS's
  major reaches them (ADR-005). Neither package reaches the built Mini App, whose output is
  unchanged.

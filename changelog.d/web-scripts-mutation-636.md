### Changed

- The token compiler `web/app/scripts/build-tokens.ts` joins the web mutation class by its exact
  path: a pull request that changes it gets a StrykerJS leg, the weekly battery mutates it, and the
  configuration check and the exclusions walk read it. The Mermaid recorder in the same folder stays
  outside, as a maintainer tool no Vitest test runs (SPEC-636).

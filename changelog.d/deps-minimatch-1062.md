### Security

- The web lock moves `minimatch` from 10.2.6 to 11.0.3, which clears GHSA-g3hv-75wg-m249, a denial
  of service through globparts amplification. StrykerJS 10's core declares `minimatch ~10.2.4`,
  which no update inside its major reaches past, so one pnpm override scoped to that major,
  `@stryker-mutator/core@10>minimatch`, takes `>=11.0.1 <12`. minimatch 11 adds
  `@isaacs/nonrecursive-glob-match` 1.0.3 to the lock; no `package.json` range moves.
- A lock check refuses any minimatch below 11.0.1 in `pnpm-lock.yaml`.

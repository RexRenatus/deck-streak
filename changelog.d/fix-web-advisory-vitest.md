### Fixed

- The web advisory audit reads clean again: `pnpm-lock.yaml` moves `vitest` and its `@vitest/*`
  packages from 4.1.11 to 4.1.12, the patch release that clears `@vitest/mocker`'s GHSA-qxg6-89mg-rqv8
  and GHSA-xp26-3454-mjgp, and the catalog's `vitest` range in `pnpm-workspace.yaml` moves from
  `^4.1.11` to `^4.1.12`. No `package.json` changes, and no other package moves.

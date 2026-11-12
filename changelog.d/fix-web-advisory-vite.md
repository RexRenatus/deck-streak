### Fixed

- The web advisory audit reads clean again: `pnpm-lock.yaml` moves `vite` (GHSA-587h-7wmj-h5pm,
  GHSA-698v-4r3x-3vcv and GHSA-r825-w8mm-5pxw) to the lowest patch release that clears them. No
  `package.json` changes, and no other package moves.

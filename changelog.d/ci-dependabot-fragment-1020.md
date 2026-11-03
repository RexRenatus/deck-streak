### Changed

- The `fragment` check passes a pull request that Dependabot opens when every file it changes is
  one that an ecosystem of `.github/dependabot.yml` updates: the Cargo manifests and lockfile, the
  pnpm manifests, lockfile and catalog, and the workflow files. Every other pull request, and a
  Dependabot pull request that changes any other file, still needs a fragment.

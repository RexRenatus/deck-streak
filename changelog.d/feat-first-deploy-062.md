### Added

- A tag on `main` now builds and publishes a release: `release.yml` attests the tarball's build
  provenance and attaches it with its `SHA256SUMS`.
- `deploy/deploy.sh` installs a verified tag on the host, switches `current` atomically and switches
  back from a release that does not become ready; `deploy/rollback.sh` returns to a kept release in
  one command. `deploy/scripts/render-caddy.py` renders the Caddy block for `deploy.sh
  caddy-install`, which validates a copy before it reloads.

### Changed

- The sync login is loaded by the sync job alone, through a drop-in for its instance; the liveness
  and maintenance instances no longer request it.

### Fixed

- `deploy.sh caddy-remove` now prints `deploy: refused` when the adapted configuration is refused, as it
  already did when the validation was refused, so the operator can tell which step stopped.

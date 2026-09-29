### Fixed

- `deploy.sh caddy-remove` now refuses in words of its own when its candidate Caddyfile cannot be
  written, and both of its refusals print `deploy: refused` even when the candidate file is absent.

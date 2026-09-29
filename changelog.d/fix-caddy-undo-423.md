### Fixed

- `deploy.sh caddy-install` now undoes every write it made and prints its refusal on every early exit; `caddy-remove` checks the same four paths as the install's guard, and both scripts check the Caddy directory and the live Caddyfile before they write (#423, #424).

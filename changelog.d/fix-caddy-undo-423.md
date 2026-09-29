### Fixed

- `deploy.sh caddy-install` now undoes every write it made and prints its refusal on every early exit, and `caddy-remove` refuses a candidate path that is a link before it writes (#423, #424).

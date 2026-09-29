### Fixed

- A Caddy reload that fails during `deploy.sh caddy-install` or `caddy-remove` now puts the previous site block and Caddyfile back, reloads them and exits non-zero naming the failed reload, so the files on the host match the configuration Caddy is running.

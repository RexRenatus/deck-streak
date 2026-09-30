### Fixed

- `deploy.sh` now ends a verb with one `deploy:` line naming the step, a non-zero exit and no write when a temporary path cannot be made (the release step's and the Caddy install's working directories, and the host script's check file, which is now made before the host script's first write), instead of the tool's bare message (#451).

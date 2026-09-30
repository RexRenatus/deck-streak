### Fixed

- `deploy.sh` now ends a verb with one `deploy:` line naming the step, a non-zero exit and no write when a temporary path cannot be made: the release step's and the Caddy install's working directories, the host script's check file, the unpack directory, the pid-named `current` link and the directories its unit files are written in. The host script checks those directories before its first write and stages the link before the unit files are installed, instead of leaving the tool's bare message (#451).

### Fixed

- The Caddy install and removal now tell Caddy to read the site file as a Caddyfile when they reload
  it, the restoring reload after a failed one included, so a Caddyfile kept under any file name is
  read as a Caddyfile and not as JSON (SPEC-127 A40, ADR-127, #674).

### Fixed

- The one-router census now follows what the compiler pulls in (#297). A file brought in by
  `include!`, `include_str!`, `include_bytes!` or `#[path]` is read under its own path with every
  rule of that path, and one the census cannot follow is refused. The walker reads SQL, so only
  the router's two migrations may name the feed and the held queue, and it refuses a symlink in a
  place it walks. A systemd unit or drop-in that runs a test file is refused, and so is a command
  reply visible outside the handler's module. Rows `S04182` to `S04194` pin each rule.

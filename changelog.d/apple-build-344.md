### Changed

- The Apple build (SPEC-344, ADR-355) now runs from one job body, `xcframework.yml`, on a pull
  request into `dev` that changes an Apple or FFI path, the workspace manifest or the toolchain
  pin, and on every release tag, through two small callers. It stays an advisory check, passes no
  secret and holds a read-only token. The CI pin test admits a call job's own local call and
  nothing else it refused before.

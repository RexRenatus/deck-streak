### Added

- The web engine (SPEC-338): Anki's engine, built for WebAssembly from the pinned fork's tag, runs
  in a dedicated Worker over the browser's private file system. The page's client opens a
  collection, takes the study queue, answers a card and undoes it; one tab at a time holds a
  collection, and a second tab is refused by name. Persistent storage is requested and its answer
  surfaced, and a context that refuses the private file system is refused by name. ADR-348 records
  the fork's patches and the Worker boundary.
- CI's `web-engine` job builds the module with its bindings, holds them to 8000000 bytes gzip -9
  with brotli printed beside it, and runs the browser tests over the module in Chromium and WebKit
  (ADR-349). The page's policy admits WebAssembly compilation (`'wasm-unsafe-eval'`) and nothing
  else new.

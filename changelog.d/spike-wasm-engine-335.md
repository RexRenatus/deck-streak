### Added

- The browser engine spike's record: SPEC-335 measures Anki's engine built for WebAssembly, opening,
  answering and undoing a card in a Worker over the browser's private file system in Chromium and
  WebKit, at 7529787 bytes gzip -9 shipped. ADR-346 records the fork's gates and the measuring
  harness, and ADR-336 takes GO as its decision outcome with a size budget of 8000000 bytes gzip -9.

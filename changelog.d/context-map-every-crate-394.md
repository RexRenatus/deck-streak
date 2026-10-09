### Added

- The context-map schematic draws every workspace crate and every dependency its manifest names,
  one arrow per edge in declared layers, and a census test holds the drawing to the manifests and
  the fence (SPEC-394, ADR-408; #689). The six crates and 29 edges the old drawing lacked are
  drawn; no manifest, fence line or product code changes.

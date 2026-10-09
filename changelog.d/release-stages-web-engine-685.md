### Fixed

- A release now carries the web engine's module and its bindings at `web/engine/`, built as CI builds them and held to the size budget before the release is drafted, so the published app's Worker finds its engine at `/engine/` (#685).

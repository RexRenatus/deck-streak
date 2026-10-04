### Added

- An FFI adapter puts the engine behind a five-call allow-list for a native client, and a workflow
  builds it as an arm64 XCFramework with a device slice and a simulator slice.

### Changed

- The settle census compiles the workspace under every combination of the features its members
  declare, so a call to `settle` under a feature is refused for the call, and a member that declares
  a feature is no longer refused for declaring it.

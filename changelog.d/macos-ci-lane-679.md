### Added

- A macOS CI lane: on each pull request into dev that changes an Apple or FFI path, a hosted macOS
  job builds the app for a macOS destination with the pinned Xcode and runs its unit tests there,
  its runner admitted by its workflow's file name (SPEC-679 section 9b, R7).

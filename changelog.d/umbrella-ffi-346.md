### Added

- The app links one Rust static library, the umbrella FFI crate (SPEC-346): `deck-streak-ffi` grows
  in place, UniFFI is named by it alone, and a census in the required CI refuses a second static
  library, a second UniFFI component, a remote or second binary target and a committed built
  library. The Apple job checks that its bindings hold one module and that each XCFramework slice
  holds one library, and the change caller now watches every crate the umbrella links. ADR-357
  records the choices and what each was chosen against.

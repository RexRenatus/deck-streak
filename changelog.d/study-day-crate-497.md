### Changed

- The study day, its instant and its ISO date error move out of the kernel into their own crate, `deck-streak-study-day`, which depends on nothing and reads no clock, file or network, so both clients can link the one rule the server counts days by. The kernel re-exports each item at the path it always had, so no importer changes, and the system clock stays in the kernel; CI builds the crate for the web client's wasm32 target and the native client's iOS target, and the context map carries the kernel's new edge (SPEC-497, ADR-515).

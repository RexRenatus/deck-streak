//! The FFI adapter for a native client (SPEC-336; ADR-345).
//!
//! A native client reaches Anki's engine through one entry point, [`engine::Engine::run`]: the
//! index of a backend service, the index of a method in it and the request's protobuf bytes go
//! in, and the response's protobuf bytes come out, the shape the engine's own clients use. A call
//! reaches the engine only when its pair is in [`allow_list::ALLOW_LIST`], a constant table; any
//! other call is answered with [`engine::EngineRefusal::NotAllowed`] before the engine sees it.
//!
//! - [`allow_list`]: the calls a native client may make, and the lookup that decides one (R2).
//! - [`engine`]: the engine handle, its typed refusal and the entry point (R1, R3).
//!
//! The adapter depends on the engine and on no other crate of this workspace (ADR-345 D1).

#![forbid(unsafe_code)]

uniffi::setup_scaffolding!();

pub mod allow_list;
pub mod engine;

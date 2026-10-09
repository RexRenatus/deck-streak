//! The web engine: Anki's own engine, built for `wasm32-unknown-unknown`, for the web
//! client's dedicated Worker (SPEC-338, ADR-348).
//!
//! Every dependency is `wasm32`-only, so a native build compiles this crate root alone and the
//! engine never joins the native graph through it.

pub mod files;
pub mod study;
pub mod synthetic;

#[cfg(target_arch = "wasm32")]
mod wasm;

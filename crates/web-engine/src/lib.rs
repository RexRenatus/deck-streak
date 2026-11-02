//! The web engine: Anki's own engine, built for `wasm32-unknown-unknown`, for the web
//! client's dedicated Worker (SPEC-338, ADR-348).
//!
//! Every dependency but one is `wasm32`-only, so a native build compiles this crate root alone and
//! the engine never joins the native graph through it. The one is the scheduling companion's
//! interface, which is `no_std`, depends on nothing and does no I/O: [`companion`] decides over the
//! owner's companion frames with the interface's own decision, so the native tests run it over the
//! same vectors as the native decision (SPEC-511, ADR-529 D4).

pub mod companion;
pub mod files;
pub mod study;
pub mod synthetic;

#[cfg(target_arch = "wasm32")]
mod wasm;

//! # deck-streak-media
//!
//! What this crate owns: how the product knows a media file and the names that point at it
//! (SPEC-513, ADR-531). A file is known by the SHA-256 of its bytes ([`MediaHash`]); a name is a
//! certified `ds-` name that carries that hash, or a name an import brought, kept unchanged
//! ([`MediaName`]); a name is bound to one hash, once ([`Bindings`]); one grammar finds every
//! media name a note's text writes ([`scan`]) and a CSS writes ([`scan_css`]); and a sweep keeps
//! every live, pinned, in-grace and imported file and deletes the rest ([`plan_sweep`]).
//!
//! What it does not own: the store. No source here reads a file, the clock, the environment, the
//! network, a process or a thread, and the crate depends on no workspace crate, so the server and
//! both clients link one rule. A collection implements [`MediaIndex`] to hand it texts and stored
//! files; the renderer reads names through the same grammar.

pub mod bind;
pub mod css;
pub mod hash;
pub mod index;
pub mod name;
pub mod scan;
pub mod sweep;

#[cfg(kani)]
mod kani_proofs;

pub use bind::{
    BindRefusal, BindState, Binding, Bindings, Bound, Release, ReleaseRefusal, release_step, step,
};
pub use css::scan_css;
pub use hash::MediaHash;
pub use index::{MediaIndex, Source, plan, references};
pub use name::{CertifiedName, MediaName, NameRefusal};
pub use scan::scan;
pub use sweep::{Facts, Fate, Keep, StoredFile, SweepPlan, SweepPolicy, fate, plan_sweep};

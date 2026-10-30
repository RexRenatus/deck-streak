//! The port a collection implements so the media core can read it (SPEC-513 R7; ADR-531 D6).
//!
//! The media core does no I/O and depends on no workspace crate. A collection hands it every text
//! that names media, as HTML (a note's field, a card template) or as CSS (a note type's), and
//! every file its store holds; the core finds the names and plans the sweep.

use std::collections::BTreeSet;

use crate::css::scan_css;
use crate::scan::scan;
use crate::sweep::{StoredFile, SweepPlan, SweepPolicy, plan_sweep};

/// One text that names media.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source<'a> {
    /// A note's field or a card template, read by [`scan`].
    Html(&'a str),
    /// A note type's CSS, read by [`scan_css`].
    Css(&'a str),
}

/// What a collection gives the media core.
pub trait MediaIndex {
    /// Every text that names media: each note's fields and each card template as HTML, and each
    /// note type's CSS as CSS.
    fn sources(&self) -> Vec<Source<'_>>;

    /// Every file the media store holds.
    fn stored(&self) -> Vec<StoredFile>;
}

/// Every name the index's texts write.
#[must_use]
pub fn references<I: MediaIndex + ?Sized>(index: &I) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for source in index.sources() {
        match source {
            Source::Html(text) => names.extend(scan(text)),
            Source::Css(css) => names.extend(scan_css(css)),
        }
    }
    names
}

/// The sweep plan for the index's store against every name its texts write.
#[must_use]
pub fn plan<I: MediaIndex + ?Sized>(index: &I, policy: SweepPolicy) -> SweepPlan {
    plan_sweep(&index.stored(), &references(index), policy)
}

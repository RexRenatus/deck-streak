//! Badges (SPEC-073 R1 to R8): the catalog of 40, the award port that writes each once, the study
//! conditions over a day's badge context, and the hour windows the context counts.
//!
//! The study badges are decided by coordination's badge step in phase 7 of the settle fold; a habit
//! or focus badge by its own context, and a band badge by the band-up, each through [`award`].

pub mod award;
pub mod catalog;
pub mod conditions;
pub mod hours;

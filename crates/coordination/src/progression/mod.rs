//! Coordination's reads over progression's XP (SPEC-072 R23, R24): the views the API and the bot
//! answer from, so the two surfaces show the same numbers; and over its badges, records and next
//! milestone (SPEC-073 R5, R13, R15).

pub mod badge_context;
pub mod badges_view;
pub mod law_tiers;
pub mod level_view;
pub mod milestone_view;
pub mod records_view;

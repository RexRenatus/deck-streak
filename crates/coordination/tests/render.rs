//! The two texts the coordination context hands a log line: a reason as the runner's ERROR line
//! writes it, and the obligations registry as a debug line names its sources (SPEC-057 R2: each
//! reads as its content, never as an empty string).

// An integration test is test code: it prints its examined counts on purpose.
#![allow(clippy::print_stdout)]

use deck_streak_coordination::obligations::{ObligationSource, Obligations};
use deck_streak_coordination::runner::Reason;
use deck_streak_ingest::gate::Deadline;
use deck_streak_kernel::{PortFuture, UtcMillis};

/// A source that names itself and holds no deadline.
struct Named(&'static str);

impl ObligationSource for Named {
    fn name(&self) -> &'static str {
        self.0
    }

    fn deadlines(&self, _now: UtcMillis) -> PortFuture<'_, Vec<Deadline>> {
        Box::pin(async { Ok(Vec::new()) })
    }
}

#[test]
fn a_reason_reads_its_code_then_each_figure_as_name_equals_value() {
    let bare = Reason::new("sync_dead").to_string();
    let figured = Reason::new("maintenance_drift")
        .with("skew_min", -45)
        .with("attempts", 3)
        .to_string();
    assert_eq!(bare, "sync_dead");
    assert_eq!(figured, "maintenance_drift skew_min=-45 attempts=3");
    println!("examined 2 reason(s)");
}

#[test]
fn a_registry_debug_line_names_its_sources_in_registration_order() {
    let empty = format!("{:?}", Obligations::new());
    let two = format!(
        "{:?}",
        Obligations::new()
            .with(Named("wagers"))
            .with(Named("spins"))
    );
    assert_eq!(empty, "Obligations { sources: [] }");
    assert_eq!(two, "Obligations { sources: [\"wagers\", \"spins\"] }");
    println!("examined 2 registr(ies)");
}

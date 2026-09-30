//! The API state's debug form names what it carries and whether the optional ports are wired,
//! and never prints a port itself (SPEC-047 R9).

use deck_streak_api::{ApiState, Readiness};

#[test]
fn the_debug_form_names_the_state_and_its_ports() {
    let shown = format!("{:?}", ApiState::new(Readiness::new()));
    assert!(shown.starts_with("ApiState {"), "{shown}");
    assert!(shown.contains("readiness"), "{shown}");
    assert!(shown.contains("law_tiers: false"), "{shown}");
    assert!(shown.contains("readings: false"), "{shown}");
}

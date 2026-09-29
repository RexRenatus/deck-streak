//! The literals the agent core is configured by (SPEC-043). Each is asserted against its value
//! written out here, because a test that reads a constant only through itself cannot see it change.

use std::time::Duration;

use deck_streak_agent::data_rights::AGENT_CONTEXT;
use deck_streak_agent::duty::{
    DAILY_READING, DAILY_READING_WALL_SECONDS, DEFAULT_MAX_BUDGET_MICRO_USD, DEFAULT_MAX_TURNS,
    DEFAULT_WALL_SECONDS,
};
use deck_streak_agent::gate::{CLASS_EMPTY, CLASS_VOID};
use deck_streak_agent::route::{AI_ROUTE, AiRoute};
use deck_streak_agent::runner::WALL_CLOCK_GRACE;
use deck_streak_agent::runs::AGENT_RUNS_TABLE;
use deck_streak_kernel::Setting;

#[test]
fn the_ai_route_setting_is_named_deckstreak_ai_route() {
    assert_eq!(AI_ROUTE, "DECKSTREAK_AI_ROUTE");
}

#[test]
fn the_ai_route_setting_states_its_shape() {
    assert_eq!(
        <AiRoute as Setting>::SHAPE,
        "proxy, or unset for no AI route"
    );
}

#[test]
fn the_default_turn_cap_is_thirty() {
    assert_eq!(DEFAULT_MAX_TURNS, 30);
}

#[test]
fn the_default_budget_cap_is_five_dollars() {
    assert_eq!(DEFAULT_MAX_BUDGET_MICRO_USD, 5_000_000);
}

#[test]
fn the_default_wall_clock_is_eighteen_hundred_seconds() {
    assert_eq!(DEFAULT_WALL_SECONDS, 1800);
}

#[test]
fn the_daily_reading_wall_clock_is_six_hundred_twenty_seconds() {
    assert_eq!(DAILY_READING_WALL_SECONDS, 620);
}

#[test]
fn the_daily_reading_is_recorded_under_its_name() {
    assert_eq!(DAILY_READING, "daily-reading");
}

#[test]
fn the_wall_clock_grace_is_forty_five_seconds() {
    assert_eq!(WALL_CLOCK_GRACE, Duration::from_secs(45));
}

#[test]
fn the_gate_names_a_void_and_an_empty_probe() {
    assert_eq!(CLASS_VOID, "void");
    assert_eq!(CLASS_EMPTY, "examined-nothing");
}

#[test]
fn the_agent_runs_table_and_context_are_named() {
    assert_eq!(AGENT_RUNS_TABLE, "agent_runs");
    assert_eq!(AGENT_CONTEXT, "agent");
}

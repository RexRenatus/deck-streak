#!/bin/sh
# DeckStreak's second route (SPEC-396; ADR-410): a stub that keeps the script's declared inputs and
# does nothing, so the tests that describe the route are red by assertion.
set -eu

readonly SECOND_ROUTE_CHECK_IN=second-route-check-in
readonly SECOND_ROUTE_REPORT=second-route-report
: "$SECOND_ROUTE_CHECK_IN" "$SECOND_ROUTE_REPORT"
exit 0

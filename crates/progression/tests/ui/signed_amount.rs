//! A debit written as an XP amount: it must not compile (SPEC-040 A6).

use deck_streak_progression::xp::XpAmount;

fn main() {
    let debit: i64 = -40;
    let _amount = XpAmount::new(debit);
}

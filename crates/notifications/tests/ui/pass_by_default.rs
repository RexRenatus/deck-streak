//! A delivery call outside the router module, with a pass made by `Default`: it must not compile
//! (SPEC-041 A2).

use deck_streak_notifications::{BotTransport, Pass};

async fn deliver(transport: &dyn BotTransport) {
    let _pushed = transport
        .push_message(&Pass::default(), "a line the router never decided")
        .await;
}

fn main() {
    drop(deliver);
}

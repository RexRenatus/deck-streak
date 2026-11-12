//! A bot transport that keeps the router's pass by cloning the one it borrows, so it could deliver
//! later, outside the router: it must not compile (SPEC-041 A2).

use std::sync::Mutex;

use deck_streak_notifications::{BotTransport, Pass, PushFuture, Pushed};

struct Keeper {
    kept: Mutex<Vec<Pass>>,
}

impl BotTransport for Keeper {
    fn push_message<'a>(&'a self, pass: &'a Pass, _text: &'a str) -> PushFuture<'a> {
        let kept: Pass = pass.clone();
        if let Ok(mut passes) = self.kept.lock() {
            passes.push(kept);
        }
        Box::pin(async { Pushed::Delivered })
    }
}

fn main() {
    drop(Keeper {
        kept: Mutex::new(Vec::new()),
    });
}

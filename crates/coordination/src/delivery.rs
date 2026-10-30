//! The `DeliveryMarker` port (SPEC-027 R6): the notifier's monotonic counts of attempted and
//! delivered sends, which the runner reads around a job to tell a failed delivery from a job that
//! never engaged the notifier (the predecessor's `pipeline_layers/ops.py:OpsLayer.cron_delivery_marker`
//! at `27ee2bc`).
//!
//! The bot's transport implements it in the composition root (SPEC-026). Until a notifier exists,
//! [`NoNotifier`] reports no send forever, which is the predecessor's own shape for a missing
//! notifier: a job that returns "not delivered" then keeps its claim.

/// The notifier's counts since its process started: sends it attempted, and sends that returned a
/// message id. Both only ever grow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeliveryCounts {
    /// Sends attempted.
    pub attempted: u64,
    /// Sends delivered: a message id came back.
    pub delivered: u64,
}

/// The port the notifier implements for the runner.
pub trait DeliveryMarker: Send + Sync {
    /// The counts now.
    fn counts(&self) -> DeliveryCounts;
}

/// No notifier: the counts never move.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoNotifier;

impl DeliveryMarker for NoNotifier {
    fn counts(&self) -> DeliveryCounts {
        DeliveryCounts::default()
    }
}

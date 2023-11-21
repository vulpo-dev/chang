use std::sync::OnceLock;
use tokio::sync::mpsc::UnboundedSender;

use crate::events::transform::Event;

type Sender = UnboundedSender<Event>;

static CHANNELS: OnceLock<Sender> = OnceLock::new();

pub fn init(sender: &Sender) {
    CHANNELS.set(sender.clone()).expect("To set sender");
}

pub fn sender() -> &'static Sender {
    CHANNELS.get().expect("Sender")
}

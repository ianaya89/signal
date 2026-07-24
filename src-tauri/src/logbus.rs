//! tracing → `EventBus` bridge: every log record becomes a `log:line` event
//! for the in-app viewer.

use signal_core::{EventBus, SignalEvent};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

pub struct BusLayer {
    events: EventBus,
}

impl BusLayer {
    pub fn new(events: EventBus) -> Self {
        Self { events }
    }
}

impl<S: Subscriber> Layer<S> for BusLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // ignore very chatty levels in the UI feed
        if *event.metadata().level() > Level::DEBUG {
            return;
        }

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        self.events.publish(SignalEvent::LogLine {
            level: event.metadata().level().to_string(),
            target: event.metadata().target().to_owned(),
            message: visitor.message,
        });
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            use std::fmt::Write as _;
            let _ = write!(self.message, " {}={value:?}", field.name());
        }
    }
}

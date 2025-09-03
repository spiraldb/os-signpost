//! Tracing subscriber layer for Apple's os_signpost integration.
//!
//! Provides a [`TracingSubscriber`] that can be used with the `tracing-subscriber`
//! crate to emit os_signpost intervals and events to be viewed in Apple's Instruments.

use crate::global_logger;
use crate::{SignpostId, SignpostType};
use dashmap::DashMap;
use tracing::{span, Event, Id, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

struct ActiveInterval {
    id: SignpostId,
    name: String,
}

/// A tracing subscriber layer that emits signposts for Apple's Instruments
pub struct TracingSubscriber {
    intervals: DashMap<Id, ActiveInterval>,
}

impl Default for TracingSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

impl TracingSubscriber {
    /// Create a new signpost tracing subscriber.
    pub fn new() -> Self {
        Self {
            intervals: DashMap::new(),
        }
    }
}

impl<S> Layer<S> for TracingSubscriber
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, _ctx: Context<'_, S>) {
        let logger = global_logger();
        if !logger.enabled() {
            return;
        }

        let mut visitor = MessageVisitor::new();
        attrs.record(&mut visitor);

        let name = format!(
            "{}::{}",
            attrs.metadata().module_path().unwrap_or_default(),
            attrs.metadata().name()
        );

        // Generate unique signpost ID for this span
        let signpost_id = SignpostId::generate(logger);

        logger.emit(
            signpost_id,
            &name,
            visitor.message.as_deref(),
            SignpostType::IntervalBegin,
        );

        // Store the interval. To be removed when the interval ends.
        self.intervals.insert(
            id.clone(),
            ActiveInterval {
                id: signpost_id,
                name,
            },
        );
    }

    fn on_record(&self, _id: &span::Id, _values: &span::Record<'_>, _ctx: Context<'_, S>) {
        // The os_signpost API doesn't have a direct way to add additional data
        // to an already-started interval.
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let logger = global_logger();
        if !logger.enabled() {
            return;
        }

        let mut visitor = MessageVisitor::new();
        event.record(&mut visitor);

        let name = format!(
            "{}::{}",
            event.metadata().module_path().unwrap_or_default(),
            ctx.current_span()
                .metadata()
                .map(|meta| meta.name())
                .unwrap_or_default(),
        );

        logger.emit(
            SignpostId::generate(logger),
            &name,
            visitor.message.as_deref(),
            SignpostType::Event,
        );
    }

    fn on_close(&self, id: Id, _ctx: Context<'_, S>) {
        let logger = global_logger();
        if !logger.enabled() {
            return;
        }

        // End the interval and remove it from the map.
        if let Some((_, interval)) = self.intervals.remove(&id) {
            logger.emit(interval.id, &interval.name, None, SignpostType::IntervalEnd);
        }
    }
}

/// Extracts message content from tracing span attributes and event fields.
///
/// Messages are extracted from log calls `info!("message")` as well
/// as annotated proc macros `#[instrument(fields(message = "message"))]`.
struct MessageVisitor {
    /// The captured message content from any "message" field.
    message: Option<String>,
}

impl MessageVisitor {
    /// Creates a new message visitor.
    fn new() -> Self {
        Self { message: None }
    }
}

impl tracing::field::Visit for MessageVisitor {
    /// Records string field values, capturing only "message" fields.
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        }
    }

    /// Records debug-formattable field values, capturing only "message" fields.
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        }
    }
}

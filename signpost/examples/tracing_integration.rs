//! This example shows how to use the Signpost tracing subscriber to automatically
//! create intervals as well as single point events that can be viewed in Instruments.
//!
//! To run: cargo run --example tracing_integration --features tracing
//! To view in Instruments: Create a new trace with "os_signpost" template.

use signpost::{categories, Signpost};
use std::thread;
use std::time::Duration;
use tracing::{info, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Registry};

fn main() {
    // Configure signpost at startup once.
    Signpost::configure("dev.vortex", categories::POINTS_OF_INTEREST);

    Registry::default()
        .with(signpost::TracingSubscriber::new())
        .with(tracing_subscriber::fmt::Layer::default().compact())
        .init();

    process_task();
}

#[instrument]
fn process_task() {
    info!("Starting task processing");
    perform_work(42);
    thread::sleep(Duration::from_millis(100));
    info!("Task processing completed");
}

#[instrument(fields(message = "Processing user data"))]
fn perform_work(value: u32) {
    info!("Performing work with value: {}", value);
    thread::sleep(Duration::from_millis(50));
    let result = value * 2;
    info!(result, "Work completed");
}

# os-signpost

A Rust wrapper for Apple's os_signpost API that integrates with Instruments for performance tracing.

## Installation

```toml
[dependencies]
signpost = "0.1.0"
```

## Quick Start

```rust
use signpost::{categories, signpost, Signpost};

fn main() {
    // Initialize the global signpost provider once.
    Signpost::configure("com.company.app", categories::POINTS_OF_INTEREST);
    let data = load_data();
    let result = process_data(&data);
    save_result(&result);
}

#[signpost]
fn load_data() -> Vec<i32> {
    signpost::event_with_message!("loading data", "in progress");
    vec![1, 2, 3, 4, 5]
}

fn process_data(data: &[i32]) -> Vec<i32> {
    let _interval = signpost::interval!("processing");

    data.iter()
        .map(|x| {
            signpost::event!("item_processed");
            x * 2
        })
        .collect()
}

#[signpost(message = "Saving results to disk")]
fn save_result(_data: &[i32]) {
    std::thread::sleep(std::time::Duration::from_millis(100));
}
```

## Signpost Types

**Intervals** represent periods of time with a beginning and end:

```rust
let interval = signpost::interval!("data_processing");
// .. process data
drop(interval); // Interval ends on drop.

let _guard = signpost::interval_with_message!("network_request", "GET /api/users");
// .. make request
```

**Events** mark single points in time:

```rust
signpost::event!("cache_miss");
signpost::event_with_message!("user_action", "button_clicked");
```

## Using OsLog Without Signpost

```rust
use signpost::{OsLog, SignpostId, categories};

let logger = OsLog::new("com.example.app".to_string(), categories::POINTS_OF_INTEREST);

if logger.enabled() {
    let id = SignpostId::generate(&logger);
    logger.event_with_message(id, "checkpoint", "Processing started");
    let interval = logger.interval_with_message(id, "processing", "Heavy computation");
}
```

## Integration with Instruments

1. Build your application with signpost instrumentation
2. Open Instruments.app on macOS
3. Create a new trace using the "os_signpost" instrument
4. Run your application
5. View the signpost intervals and events in the timeline

## Reference Docs

- [OS Signpost - Recording Performance Data](https://developer.apple.com/documentation/os/recording-performance-data)

# os-signpost

A Rust wrapper for Apple's os_signpost API that integrates with Instruments for performance tracing.

## Installation

```toml
[dependencies]
signpost = "0.1.0"
```

## Quick Start

```rust
use signpost::{categories, Signpost};

fn main() {
    // Initialize the global signpost provider once.
    Signpost::new("com.company.app", categories::POINTS_OF_INTEREST);
    let data = load_data();
    let result = process_data(&data);
    save_result(&result);
}

#[signpost]
fn load_data() -> Vec<i32> {
    signpost::event_with_message!("loading data", "in progress")
    vec![1, 2, 3, 4, 5]
}

fn process_data(data: &[i32]) -> Vec<i32> {
    let _interval = signpost::interval!("processing");

    data.iter().map(|x| {
        signpost::event!("item_processed");
        x * 2
    }).collect()
}

#[signpost(message = "Saving results to disk")]
fn save_result(data: &[i32]) {
    std::thread::sleep(std::time::Duration::from_millis(100));
}
```

## Core Concepts

### Signpost Types

**Intervals** represent periods of time with a beginning and end:

```rust
// Manual interval management
let interval = signpost::interval!("data_processing");
// ... do work ...
drop(interval); // Interval ends here

{
    let _guard = signpost::interval_with_message!("network_request", "GET /api/users");
    // ... make network request ...
} // Interval automatically ends when guard is dropped
```

**Events** mark single points in time:

```rust
signpost::event!("cache_miss");
signpost::event_with_message!("user_action", "button_clicked");
```

### Signpost IDs

Each signpost interval requires a unique ID to properly match begin/end markers. The library automatically generates these IDs, but you can also create custom ones:

```rust
use signpost::{SignpostId, global_logger};

let logger = global_logger();
let custom_id = SignpostId::from_pointer(some_ptr as *const _);
let interval = logger.interval(custom_id, "custom_operation");
```

### Signpost Categories

```rust
use signpost::categories;

SignpostTrace::new("com.example.app", categories::POINTS_OF_INTEREST);
SignpostTrace::new("com.example.app", categories::DYNAMIC_TRACING);
SignpostTrace::new("com.example.app", categories::DYNAMIC_STACK_TRACING);
```

### Manual Logger Management

```rust
use signpost::{OsLog, SignpostId, categories};

let logger = OsLog::new("com.example.app".to_string(), categories::POINTS_OF_INTEREST);

if logger.enabled() {
    let id = SignpostId::generate(&logger);
    logger.event_with_message(id, "checkpoint", "Processing started");
    let interval = logger.interval_with_message(id, "processing", "Heavy computation");
}
```

### Scoping

Signpost matching behavior can be configured with different scopes:

```rust
use signpost::{SignpostScope, categories, OsLog};

// Thread-wide: Matching restricted to single threads
let logger = OsLog::new("com.example.app".to_string(), categories::POINTS_OF_INTEREST)
    .with_scope(SignpostScope::Thread);

// Process-wide: Matching within single process (default)
let logger = OsLog::new("com.example.app".to_string(), categories::POINTS_OF_INTEREST)
    .with_scope(SignpostScope::Process);

// System-wide: Matching can span across processes
let logger = OsLog::new("com.example.app".to_string(), categories::POINTS_OF_INTEREST)
    .with_scope(SignpostScope::System);
```

## Integration with Instruments

1. Build your application with signpost instrumentation
2. Open Instruments.app on macOS
3. Create a new trace using the "os_signpost" instrument
4. Run your application
5. View the signpost intervals and events in the timeline

## License

This project is licensed under the Apache License, Version 2.0

## Reference Docs

- [os_signpost](https://developer.apple.com/documentation/os/recording-performance-data)

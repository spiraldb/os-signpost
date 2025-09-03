//! Basic example usage of the signpost API.

use os_signpost::{categories, Signpost};

fn main() {
    // Configure signpost at startup once.
    Signpost::configure("dev.vortex", categories::POINTS_OF_INTEREST);

    // Call signpost instrumented functions.
    let data = data::load();
    let result = data::process(&data);
    data::save(&result);
}

mod data {
    use os_signpost::signpost;
    use std::thread::sleep;
    use std::time::Duration;

    #[signpost]
    pub(super) fn load() -> Vec<i32> {
        os_signpost::event_with_message!("before", "started");
        sleep(Duration::from_millis(100));
        os_signpost::event_with_message!("after", "ended");
        vec![1, 2, 3, 4]
    }

    #[signpost(message = "process the data")]
    pub(super) fn process(items: &[i32]) -> Vec<i32> {
        let mut results = Vec::new();

        for item in items.iter() {
            let _guard = os_signpost::interval_with_message!("item", format!("{item}"));
            os_signpost::event!("another");
            sleep(Duration::from_millis(50));
            results.push(*item);
        }

        results
    }

    #[signpost]
    pub(super) fn save(_data: &[i32]) {
        os_signpost::event_with_message!("before", "started");
        sleep(Duration::from_millis(30));
        os_signpost::event_with_message!("after", "ended");
    }
}

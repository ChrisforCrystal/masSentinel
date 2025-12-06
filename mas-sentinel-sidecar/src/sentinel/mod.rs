pub mod rules;

use sentinel_core::EntryBuilder;

pub fn init_sentinel() {
    rules::load_hardcoded_rules();
}

pub fn check_flow(resource: &str) -> bool {
    // For MVP, we just check if entry creation succeeds.
    // In a real scenario, we might want to hold the entry for the duration of the connection
    // if we were doing concurrency limiting. For QPS, this is sufficient.
    let entry = EntryBuilder::new(resource.into()).build();
    entry.is_ok()
}

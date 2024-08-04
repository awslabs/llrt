use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

pub(crate) static TIME_ORIGIN: AtomicU64 = AtomicU64::new(0);

/// Get the current time in nanoseconds.
///
/// # Safety
/// - Good until the year 2554
/// - Always use a checked substraction since this can return 0
pub(crate) fn now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

// For accuracy reasons, this function should be executed when the vm is initialized
pub fn init() {
    if TIME_ORIGIN.load(Ordering::Relaxed) == 0 {
        let time_origin = now_nanos();
        TIME_ORIGIN.store(time_origin, Ordering::Relaxed)
    }
}

//! The time now: the system's clock, or the page's in the browser (where the system's is missing).
use std::time::SystemTime;

pub fn now() -> SystemTime {
    #[cfg(target_arch = "wasm32")]
    return crate::storage::from_ms(crate::web::now_ms());
    #[cfg(not(target_arch = "wasm32"))]
    SystemTime::now()
}

/// Milliseconds since `t` (0 if the clock went back).
pub fn since_ms(t: SystemTime) -> u128 {
    now().duration_since(t).map_or(0, |d| d.as_millis())
}

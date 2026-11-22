//! The time now: the system's clock, or the page's in the browser (where the system's is missing).
use std::time::SystemTime;

pub fn now() -> SystemTime {
    #[cfg(target_arch = "wasm32")]
    return crate::storage::from_ms(crate::web::now_ms());
    #[cfg(not(target_arch = "wasm32"))]
    SystemTime::now()
}

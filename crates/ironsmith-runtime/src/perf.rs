#[cfg(not(all(feature = "wasm", target_arch = "wasm32")))]
use std::time::Instant;

pub(crate) struct PerfTimer {
    #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
    started_at_ms: f64,
    #[cfg(not(all(feature = "wasm", target_arch = "wasm32")))]
    started_at: Instant,
}

impl PerfTimer {
    pub(crate) fn start() -> Self {
        Self {
            #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
            started_at_ms: js_sys::Date::now(),
            #[cfg(not(all(feature = "wasm", target_arch = "wasm32")))]
            started_at: Instant::now(),
        }
    }

    pub(crate) fn elapsed_ms(&self) -> f64 {
        #[cfg(all(feature = "wasm", target_arch = "wasm32"))]
        {
            js_sys::Date::now() - self.started_at_ms
        }

        #[cfg(not(all(feature = "wasm", target_arch = "wasm32")))]
        {
            self.started_at.elapsed().as_secs_f64() * 1000.0
        }
    }
}

#[cfg(not(all(any(feature = "wasm", feature = "wasm-lean"), target_arch = "wasm32")))]
use std::time::Instant;

pub(crate) struct PerfTimer {
    #[cfg(not(all(any(feature = "wasm", feature = "wasm-lean"), target_arch = "wasm32")))]
    started_at: Instant,
}

impl PerfTimer {
    pub(crate) fn start() -> Self {
        Self {
            #[cfg(not(all(
                any(feature = "wasm", feature = "wasm-lean"),
                target_arch = "wasm32"
            )))]
            started_at: Instant::now(),
        }
    }

    pub(crate) fn elapsed_ms(&self) -> f64 {
        #[cfg(all(any(feature = "wasm", feature = "wasm-lean"), target_arch = "wasm32"))]
        {
            // Avoid thousands of JS host calls from nested runtime timers in
            // legality/continuous-effect hot paths. The WASM wrapper still
            // records coarse dispatch and snapshot timing around public calls.
            0.0
        }

        #[cfg(not(all(any(feature = "wasm", feature = "wasm-lean"), target_arch = "wasm32")))]
        {
            self.started_at.elapsed().as_secs_f64() * 1000.0
        }
    }
}

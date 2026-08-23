#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

pub(crate) struct PerfTimer {
    #[cfg(not(target_arch = "wasm32"))]
    started_at: Instant,
}

impl PerfTimer {
    pub(crate) fn start() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            started_at: Instant::now(),
        }
    }

    pub(crate) fn elapsed_ms(&self) -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            // Avoid thousands of JS host calls from nested runtime timers in
            // legality/continuous-effect hot paths. The WASM wrapper still
            // records coarse dispatch and snapshot timing around public calls.
            0.0
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.started_at.elapsed().as_secs_f64() * 1000.0
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn maybe_grow<R>(red_zone: usize, stack_size: usize, callback: impl FnOnce() -> R) -> R {
    stacker::maybe_grow(red_zone, stack_size, callback)
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn maybe_grow<R>(
    _red_zone: usize,
    _stack_size: usize,
    callback: impl FnOnce() -> R,
) -> R {
    callback()
}

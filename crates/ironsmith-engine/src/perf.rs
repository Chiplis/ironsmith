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

/// `IRONSMITH_COW_PROBE=1`: count copy-on-write copies of shared game-state
/// parts by call site (`cow_copy_report`).
fn cow_probe_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("IRONSMITH_COW_PROBE").is_some())
}

thread_local! {
    static COW_COPIES: std::cell::RefCell<std::collections::HashMap<(&'static str, &'static std::panic::Location<'static>), u64>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

#[track_caller]
pub(crate) fn note_shared_copy(part: &'static str, copies: bool) {
    if copies && cow_probe_enabled() {
        let site = std::panic::Location::caller();
        COW_COPIES.with(|map| *map.borrow_mut().entry((part, site)).or_insert(0) += 1);
    }
}

/// Copy counts by (part, call site), most frequent first.
pub fn cow_copy_report() -> Vec<(String, u64)> {
    COW_COPIES.with(|map| {
        let mut out: Vec<_> = map
            .borrow()
            .iter()
            .map(|((part, site), count)| (format!("{part} {}:{}", site.file(), site.line()), *count))
            .collect();
        out.sort_by(|a, b| b.1.cmp(&a.1));
        out
    })
}

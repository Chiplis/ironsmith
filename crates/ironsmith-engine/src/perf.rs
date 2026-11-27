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

/// Exact work counters for throughput investigation (thread-local, cheap).
#[derive(Clone, Copy, Debug)]
pub enum Work {
    GameClone,
    StaticRegeneration,
    ReplacementRegeneration,
    BatchCharacteristicObjects,
    SingleCharacteristic,
    RefreshFull,
    RefreshFixedPoint,
    SnapshotBuild,
    Transaction,
    Menu,
}

const WORK_KINDS: usize = 10;
const WORK_NAMES: [&str; WORK_KINDS] = [
    "game clones",
    "static regenerations",
    "replacement regenerations",
    "batch characteristic objects",
    "single characteristic calcs",
    "full refreshes",
    "fixed-point refreshes",
    "snapshot builds",
    "transactions",
    "menus",
];

thread_local! {
    static WORK: std::cell::Cell<[u64; WORK_KINDS]> = const { std::cell::Cell::new([0; WORK_KINDS]) };
}

#[inline]
pub(crate) fn work(kind: Work) {
    work_n(kind, 1);
}

#[inline]
pub(crate) fn work_n(kind: Work, n: u64) {
    WORK.with(|cell| {
        let mut counts = cell.get();
        counts[kind as usize] += n;
        cell.set(counts);
    });
}

/// Counts since the last call, then reset.
pub fn take_work_counts() -> Vec<(&'static str, u64)> {
    WORK.with(|cell| {
        let counts = cell.replace([0; WORK_KINDS]);
        WORK_NAMES.iter().copied().zip(counts).collect()
    })
}

/// A field whose clone counts game-state clones.
#[derive(Debug, Default)]
pub(crate) struct CloneCounter;

impl Clone for CloneCounter {
    fn clone(&self) -> Self {
        work(Work::GameClone);
        CloneCounter
    }
}

thread_local! {
    static TIERS: std::cell::RefCell<std::collections::BTreeMap<(&'static str, &'static str), (u64, u64)>> =
        const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

/// `IRONSMITH_PAY_TIERS=1`: count which payment tier answers each query and
/// why the cheaper tiers declined. Never set on wasm (no environment), so the
/// clock below is never read there.
pub(crate) fn tiers_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("IRONSMITH_PAY_TIERS").is_some())
}

#[inline]
pub(crate) fn tier(group: &'static str, name: &'static str) {
    tier_n(group, name, 1);
}

pub(crate) fn tier_n(group: &'static str, name: &'static str, n: u64) {
    if tiers_enabled() {
        TIERS.with(|tiers| tiers.borrow_mut().entry((group, name)).or_default().0 += n);
    }
}

pub(crate) fn tier_clock() -> Option<std::time::Instant> {
    tiers_enabled().then(std::time::Instant::now)
}

pub(crate) fn tier_time(group: &'static str, name: &'static str, started: Option<std::time::Instant>) {
    if let Some(started) = started {
        let nanos = started.elapsed().as_nanos() as u64;
        TIERS.with(|tiers| {
            let mut tiers = tiers.borrow_mut();
            let entry = tiers.entry((group, name)).or_default();
            entry.0 += 1;
            entry.1 += nanos;
        });
    }
}

/// Payment tier counts since the last call: (group, name, count, nanoseconds).
pub fn take_tier_counts() -> Vec<(&'static str, &'static str, u64, u64)> {
    TIERS.with(|tiers| {
        std::mem::take(&mut *tiers.borrow_mut())
            .into_iter()
            .map(|((group, name), (count, nanos))| (group, name, count, nanos))
            .collect()
    })
}

thread_local! {
    static NOTES: std::cell::RefCell<std::collections::BTreeMap<(&'static str, String), u64>> =
        const { std::cell::RefCell::new(std::collections::BTreeMap::new()) };
}

/// Free-form diagnostic detail for one tier decision (which card, which rule).
pub(crate) fn tier_note(group: &'static str, detail: impl FnOnce() -> String) {
    if tiers_enabled() {
        NOTES.with(|notes| *notes.borrow_mut().entry((group, detail())).or_default() += 1);
    }
}

pub fn take_tier_notes() -> Vec<(&'static str, String, u64)> {
    NOTES.with(|notes| {
        std::mem::take(&mut *notes.borrow_mut())
            .into_iter()
            .map(|((group, detail), count)| (group, detail, count))
            .collect()
    })
}

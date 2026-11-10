//! TEMPORARY perf instrumentation (remove before commit).
use std::cell::RefCell;
use std::collections::HashMap;
thread_local! { static SITES: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new()); }
pub fn hit(kind: &str, loc: &'static std::panic::Location<'static>) {
    let file = loc.file().rsplit('/').next().unwrap_or("");
    add(&format!("{kind} {file}:{}", loc.line()), 1);
}
pub fn add(key: &str, n: u64) {
    SITES.with(|s| *s.borrow_mut().entry(key.to_string()).or_default() += n);
}
thread_local! { static PHASE: std::cell::Cell<&'static str> = const { std::cell::Cell::new("-") }; }
pub fn set_phase(p: &'static str) -> &'static str { PHASE.with(|c| c.replace(p)) }
pub fn phase() -> &'static str { PHASE.with(|c| c.get()) }
pub fn take() -> Vec<(String, u64)> {
    let mut v: Vec<_> = SITES.with(|s| s.borrow_mut().drain().collect());
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v
}

use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParseTraceReport {
    lines: Vec<TraceLine>,
}

impl ParseTraceReport {
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            for _ in 0..line.depth {
                out.push_str("  ");
            }
            out.push_str(&line.message);
            out.push('\n');
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TraceLine {
    depth: usize,
    message: String,
}

#[derive(Debug, Default)]
struct TraceState {
    depth: usize,
    lines: Vec<TraceLine>,
}

thread_local! {
    static TRACE_STATE: RefCell<Option<TraceState>> = const { RefCell::new(None) };
}

pub fn is_enabled() -> bool {
    TRACE_STATE.with(|state| state.borrow().is_some())
}

pub fn capture<T>(f: impl FnOnce() -> T) -> (T, ParseTraceReport) {
    let previous = TRACE_STATE.with(|state| state.replace(Some(TraceState::default())));
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    let trace = TRACE_STATE
        .with(|state| state.replace(previous))
        .unwrap_or_default();
    match result {
        Ok(result) => (result, ParseTraceReport { lines: trace.lines }),
        Err(payload) => panic::resume_unwind(payload),
    }
}

pub fn event(message: impl Into<String>) {
    let message = message.into();
    if message.trim().is_empty() {
        return;
    }
    TRACE_STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        let Some(state) = state.as_mut() else {
            return;
        };
        state.lines.push(TraceLine {
            depth: state.depth,
            message,
        });
    });
}

pub fn scope(message: impl Into<String>) -> TraceScope {
    let message = message.into();
    if message.trim().is_empty() {
        return TraceScope { active: false };
    }

    let mut active = false;
    TRACE_STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        let Some(state) = state.as_mut() else {
            return;
        };
        state.lines.push(TraceLine {
            depth: state.depth,
            message,
        });
        state.depth += 1;
        active = true;
    });

    TraceScope { active }
}

#[must_use]
pub struct TraceScope {
    active: bool,
}

impl Drop for TraceScope {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        TRACE_STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            if let Some(state) = state.as_mut() {
                state.depth = state.depth.saturating_sub(1);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_collects_indented_events() {
        let (_result, trace) = capture(|| {
            event("root");
            let _scope = scope("child");
            event("leaf");
            7
        });

        assert_eq!(trace.render(), "root\nchild\n  leaf\n");
    }
}

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::panic::{self, AssertUnwindSafe};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParseLossReport {
    diagnostics: Vec<ParseLossDiagnostic>,
}

impl ParseLossReport {
    pub fn is_lossy(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    pub fn count(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn diagnostics(&self) -> &[ParseLossDiagnostic] {
        &self.diagnostics
    }

    pub fn push(&mut self, diagnostic: ParseLossDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn push_reason(&mut self, code: impl Into<String>, message: impl Into<String>) {
        self.push(ParseLossDiagnostic::new(code, message));
    }

    pub fn reasons_text(&self) -> String {
        self.diagnostics
            .iter()
            .map(ParseLossDiagnostic::display_text)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseLossDiagnostic {
    pub code: String,
    pub message: String,
}

impl ParseLossDiagnostic {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    fn display_text(&self) -> String {
        if self.message.trim().is_empty() {
            self.code.clone()
        } else {
            format!("{}: {}", self.code, self.message)
        }
    }
}

thread_local! {
    static LOSS_STATE: RefCell<Option<Vec<ParseLossDiagnostic>>> = const { RefCell::new(None) };
}

pub fn is_enabled() -> bool {
    LOSS_STATE.with(|state| state.borrow().is_some())
}

pub fn capture<T>(f: impl FnOnce() -> T) -> (T, ParseLossReport) {
    let previous = LOSS_STATE.with(|state| state.replace(Some(Vec::new())));
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    let diagnostics = LOSS_STATE
        .with(|state| state.replace(previous))
        .unwrap_or_default();
    match result {
        Ok(result) => (result, ParseLossReport { diagnostics }),
        Err(payload) => panic::resume_unwind(payload),
    }
}

/// Run `f`, returning what it recorded alongside its result — while still
/// letting that loss reach whichever capture is already active.
///
/// [`capture`] isolates: the diagnostics it collects are hidden from the
/// enclosing capture. A memoized rule needs the opposite — the parse that
/// fills the cache must report its loss normally *and* leave a copy behind so
/// a later cache hit can [`replay`] it, because the hit skips the recording
/// code entirely.
pub fn observe<T>(f: impl FnOnce() -> T) -> (T, Vec<ParseLossDiagnostic>) {
    let previous = LOSS_STATE.with(|state| state.replace(Some(Vec::new())));
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    let observed = LOSS_STATE
        .with(|state| state.replace(previous))
        .unwrap_or_default();
    LOSS_STATE.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            state.extend(observed.iter().cloned());
        }
    });
    match result {
        Ok(result) => (result, observed),
        Err(payload) => panic::resume_unwind(payload),
    }
}

/// Record diagnostics an earlier, memoized run of the same rule produced.
pub fn replay(diagnostics: &[ParseLossDiagnostic]) {
    if diagnostics.is_empty() {
        return;
    }
    LOSS_STATE.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            state.extend(diagnostics.iter().cloned());
        }
    });
}

pub fn record(code: impl Into<String>, message: impl Into<String>) {
    let diagnostic = ParseLossDiagnostic::new(code, message);
    LOSS_STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        let Some(state) = state.as_mut() else {
            return;
        };
        state.push(diagnostic);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_returns_recorded_loss_diagnostics() {
        let (result, report) = capture(|| {
            record("suffix_object_filter_recovery", "kept suffix");
            7
        });

        assert_eq!(result, 7);
        assert!(report.is_lossy());
        assert_eq!(report.count(), 1);
        assert_eq!(
            report.reasons_text(),
            "suffix_object_filter_recovery: kept suffix"
        );
    }
}

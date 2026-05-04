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

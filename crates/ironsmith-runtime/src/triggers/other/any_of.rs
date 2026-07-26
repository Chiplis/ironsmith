use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{Trigger, TriggerEvent};

/// A union of alternative trigger events: "Whenever A or B, ...".
///
/// Matches when any branch matches. The display joins the branch surfaces
/// with " or ", keeping only the first branch's "Whenever "/"When " intro.
#[derive(Debug, Clone)]
pub struct AnyOfTrigger {
    pub branches: Vec<Trigger>,
}

impl TriggerMatcher for AnyOfTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.branches.iter().any(|branch| branch.matches(event, ctx))
    }

    fn uses_snapshot(&self) -> bool {
        // A snapshot must be captured whenever any branch wants one.
        self.branches.iter().any(Trigger::uses_snapshot)
    }

    fn display(&self) -> String {
        let mut parts = Vec::with_capacity(self.branches.len());
        for (idx, branch) in self.branches.iter().enumerate() {
            let display = branch.display();
            if idx == 0 {
                parts.push(display);
                continue;
            }
            let stripped = ["Whenever ", "When ", "At "]
                .into_iter()
                .find_map(|prefix| display.strip_prefix(prefix).map(str::to_string))
                .unwrap_or(display);
            parts.push(stripped);
        }
        parts.join(" or ")
    }
}

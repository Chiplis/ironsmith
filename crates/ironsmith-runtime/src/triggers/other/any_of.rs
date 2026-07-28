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
        self.branches
            .iter()
            .any(|branch| branch.matches(event, ctx))
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
            let stripped = if parts
                .first()
                .is_some_and(|first| first.contains("enchanted player"))
                && let Some(tail) = stripped.strip_prefix("enchanted player attacks")
            {
                format!("when they attack{tail}")
            } else {
                stripped
            };
            parts.push(stripped);
        }
        parts.join(" or ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::{ObjectFilter, PlayerFilter};

    #[test]
    fn repeated_enchanted_player_attack_branch_uses_pronoun_and_intro() {
        let enchanted = PlayerFilter::TaggedPlayer(crate::tag::TagKey::from("enchanted"));
        let mut your_attack = ObjectFilter::creature().you_control();
        your_attack.attacking_player_or_planeswalker_controlled_by = Some(enchanted.clone());
        let their_attack = ObjectFilter::creature().controlled_by(enchanted);
        let trigger = Trigger::new(AnyOfTrigger {
            branches: vec![
                Trigger::attacks_one_or_more(your_attack),
                Trigger::attacks_you_one_or_more(their_attack),
            ],
        });

        assert_eq!(
            trigger.display(),
            "Whenever you attack enchanted player or a planeswalker they control or when they attack you or a planeswalker you control"
        );
    }
}

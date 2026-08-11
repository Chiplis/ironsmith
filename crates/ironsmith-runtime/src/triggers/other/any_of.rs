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

impl AnyOfTrigger {
    fn passive_sacrificed_or_destroyed_display(&self) -> Option<String> {
        let [sacrificed, destroyed] = self.branches.as_slice() else {
            return None;
        };
        let sacrificed = sacrificed.downcast_ref::<super::PermanentSacrificedTrigger>()?;
        let destroyed = destroyed.downcast_ref::<super::PermanentDestroyedTrigger>()?;
        if sacrificed.filter != destroyed.filter {
            return None;
        }
        let sacrifice_text = sacrificed.display();
        let subject = sacrifice_text
            .strip_prefix("Whenever ")?
            .strip_suffix(" is sacrificed")?;
        Some(format!("Whenever {subject} is sacrificed or destroyed"))
    }
}

impl TriggerMatcher for AnyOfTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.branches
            .iter()
            .any(|branch| branch.matches(event, ctx))
    }

    fn subscribed_kinds(&self) -> Option<Vec<crate::events::EventKind>> {
        let mut kinds = Vec::new();
        for branch in &self.branches {
            for kind in branch.subscribed_kinds()? {
                if !kinds.contains(&kind) {
                    kinds.push(kind);
                }
            }
        }
        Some(kinds)
    }

    fn uses_snapshot(&self) -> bool {
        // A snapshot must be captured whenever any branch wants one.
        self.branches.iter().any(Trigger::uses_snapshot)
    }

    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        self.branches
            .iter()
            .any(|branch| branch.looks_back_for_source(event))
    }

    fn display(&self) -> String {
        if let Some(display) = self.passive_sacrificed_or_destroyed_display() {
            return display;
        }
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
        if parts.len() >= 3 {
            for shared_subject in ["an opponent ", "a player ", "you "] {
                let first_prefix = format!("Whenever {shared_subject}");
                if parts
                    .first()
                    .is_some_and(|first| first.starts_with(&first_prefix))
                    && parts
                        .iter()
                        .skip(1)
                        .all(|part| part.starts_with(shared_subject))
                {
                    let mut serial = vec![parts[0].clone()];
                    serial.extend(
                        parts
                            .iter()
                            .skip(1)
                            .map(|part| part[shared_subject.len()..].to_string()),
                    );
                    let last = serial
                        .pop()
                        .expect("a serial trigger union has at least three branches");
                    return format!("{}, or {last}", serial.join(", "));
                }
            }
        }
        parts.join(" or ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventKind, PermanentPhasedOutEvent};
    use crate::ids::{ObjectId, PlayerId};
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

    #[test]
    fn union_propagates_phase_out_subscription_and_source_lookback() {
        let source = ObjectId::from_raw(1);
        let alice = PlayerId::from_index(0);
        let trigger = AnyOfTrigger {
            branches: vec![
                Trigger::this_phases_out(),
                Trigger::new(crate::triggers::EventKindTrigger::new(
                    EventKind::ControlChanged,
                    "When control changes",
                )),
            ],
        };
        let event = TriggerEvent::new_with_provenance(
            PermanentPhasedOutEvent::new(source, alice, None),
            crate::provenance::ProvNodeId::default(),
        );

        let kinds = trigger
            .subscribed_kinds()
            .expect("all branches have typed event subscriptions");
        assert!(kinds.contains(&EventKind::PermanentPhasedOut));
        assert!(kinds.contains(&EventKind::ControlChanged));
        assert!(trigger.looks_back_for_source(&event));
    }

    #[test]
    fn serial_shared_opponent_union_uses_one_subject_and_oxford_or() {
        let mut attacking = ObjectFilter::creature().controlled_by(PlayerFilter::Opponent);
        attacking.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::You);
        attacking.targets_only_player = Some(PlayerFilter::You);
        attacking.set_union_one_or_more(true);
        let trigger = Trigger::new(AnyOfTrigger {
            branches: vec![
                Trigger::attacks_one_or_more_with_min_total(attacking, 2),
                Trigger::player_draws_nth_card_each_turn(PlayerFilter::Opponent, 2),
                Trigger::spell_cast_qualified(
                    None,
                    PlayerFilter::Opponent,
                    None,
                    None,
                    None,
                    Some(2),
                    false,
                ),
            ],
        });

        assert_eq!(
            trigger.display(),
            "Whenever an opponent attacks you with two or more creatures, draws their second card each turn, or casts their second spell each turn"
        );
    }
}

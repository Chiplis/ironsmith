//! "Whenever an ability of [filter] is activated" trigger.

use crate::events::EventKind;
use crate::events::spells::AbilityActivatedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct AbilityActivatedTrigger {
    pub activator: PlayerFilter,
    pub filter: ObjectFilter,
    pub non_mana_only: bool,
    pub loyalty_only: bool,
    pub activation_cost_has_tap: Option<bool>,
}

impl AbilityActivatedTrigger {
    pub fn new(activator: PlayerFilter, filter: ObjectFilter, non_mana_only: bool) -> Self {
        Self {
            activator,
            filter,
            non_mana_only,
            loyalty_only: false,
            activation_cost_has_tap: None,
        }
    }

    pub fn loyalty_only(mut self, loyalty_only: bool) -> Self {
        self.loyalty_only = loyalty_only;
        self
    }

    pub fn activation_cost_has_tap(mut self, activation_cost_has_tap: Option<bool>) -> Self {
        self.activation_cost_has_tap = activation_cost_has_tap;
        self
    }
}

fn activate_verb(subject: &str) -> &'static str {
    if subject.eq_ignore_ascii_case("you") || subject.eq_ignore_ascii_case("they") {
        "activate"
    } else {
        "activates"
    }
}

fn source_filter_phrase(filter: &ObjectFilter) -> String {
    let description = filter.description();
    let lower = description.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("target ")
        || lower.starts_with("each ")
    {
        return description;
    }
    let article = if lower
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {description}")
}

fn normalize_ability_marker(marker: &str) -> String {
    let normalized = marker.trim().to_ascii_lowercase();
    normalized
        .strip_suffix(" abilities")
        .or_else(|| normalized.strip_suffix(" ability"))
        .unwrap_or(&normalized)
        .trim()
        .to_string()
}

fn is_structural_ninjutsu_ability(ability: &crate::ability::Ability) -> bool {
    let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
        return false;
    };
    if ability.functional_zones != [Zone::Hand]
        || !activated.choices.is_empty()
        || !matches!(
            activated.timing,
            crate::ability::ActivationTiming::DuringCombat
        )
        || !activated.additional_restrictions.is_empty()
        || !activated.activation_restrictions.is_empty()
        || activated.activation_condition.is_some()
        || !activated.mana_usage_restrictions.is_empty()
    {
        return false;
    }

    let Some(costs) = activated.mana_cost.as_all() else {
        return false;
    };
    let has_ninjutsu_cost = costs.iter().any(|cost| {
        cost.effect_ref().is_some_and(|effect| {
            effect
                .downcast_ref::<crate::effects::NinjutsuCostEffect>()
                .is_some()
        })
    });
    let effects = activated.effects.flattened_default_effects();
    has_ninjutsu_cost
        && effects.len() == 1
        && effects[0]
            .downcast_ref::<crate::effects::NinjutsuEffect>()
            .is_some()
}

fn activated_ability_has_marker(ability: &crate::ability::Ability, marker: &str) -> bool {
    let marker = normalize_ability_marker(marker);
    if marker.is_empty() {
        return false;
    }
    let crate::ability::AbilityKind::Activated(activated) = &ability.kind else {
        return false;
    };

    let has_presentation_marker = activated.additional_restrictions.iter().any(|restriction| {
        let Some(label) = restriction.strip_prefix("__ironsmith_activation_label:") else {
            return false;
        };
        let label = normalize_ability_marker(label);
        label == marker || label.split_whitespace().next() == Some(marker.as_str())
    });
    has_presentation_marker || (marker == "ninjutsu" && is_structural_ninjutsu_ability(ability))
}

fn named_ability_phrase(marker: &str) -> String {
    let marker = normalize_ability_marker(marker);
    let article = if marker
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {marker} ability")
}

impl TriggerMatcher for AbilityActivatedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::AbilityActivated {
            return false;
        }
        let Some(e) = event.downcast::<AbilityActivatedEvent>() else {
            return false;
        };
        if self.non_mana_only && e.is_mana_ability {
            return false;
        }
        if self.loyalty_only && !e.is_loyalty_ability {
            return false;
        }
        if self.filter.has_x_in_cost && !e.activation_cost_has_x {
            return false;
        }
        if let Some(required) = self.activation_cost_has_tap
            && e.activation_cost_has_tap != required
        {
            return false;
        }
        if !self.activator.matches_player(e.activator, &ctx.filter_ctx) {
            return false;
        }

        let mut source_filter = self.filter.clone();
        source_filter.has_x_in_cost = false;
        if !source_filter.ability_markers.is_empty()
            || !source_filter.excluded_ability_markers.is_empty()
        {
            let Some(ability) = e.activated_ability.as_ref() else {
                return false;
            };
            if source_filter
                .ability_markers
                .iter()
                .any(|marker| !activated_ability_has_marker(ability, marker))
                || source_filter
                    .excluded_ability_markers
                    .iter()
                    .any(|marker| activated_ability_has_marker(ability, marker))
            {
                return false;
            }
            source_filter.ability_markers.clear();
            source_filter.excluded_ability_markers.clear();
        }
        if let Some(obj) = ctx.game.object(e.source) {
            source_filter.matches(obj, &ctx.filter_ctx, ctx.game)
        } else if let Some(snapshot) = e.snapshot.as_ref() {
            source_filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
        } else {
            false
        }
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::AbilityActivated])
    }

    fn display(&self) -> String {
        let subject = self.activator.description();
        let verb = activate_verb(&subject);
        let mut source_filter = self.filter.clone();
        source_filter.has_x_in_cost = false;
        let named_marker = match (
            source_filter.ability_markers.as_slice(),
            source_filter.excluded_ability_markers.as_slice(),
        ) {
            ([marker], []) => Some(marker.clone()),
            _ => None,
        };
        if named_marker.is_some() {
            source_filter.ability_markers.clear();
        }
        let ability = if self.loyalty_only {
            "a loyalty ability".to_string()
        } else if let Some(marker) = named_marker.as_deref() {
            named_ability_phrase(marker)
        } else {
            "an ability".to_string()
        };
        let mut text = if source_filter == ObjectFilter::default() {
            format!("Whenever {subject} {verb} {ability}")
        } else {
            format!(
                "Whenever {subject} {verb} {ability} of {}",
                source_filter_phrase(&source_filter)
            )
        };
        if self.filter.has_x_in_cost {
            text.push_str(" with an activation cost that contains {X}");
        }
        if self.non_mana_only && !self.loyalty_only {
            text.push_str(" that isn't a mana ability");
        }
        match self.activation_cost_has_tap {
            Some(true) => text.push_str(" with {T} in its activation cost"),
            Some(false) => text.push_str(" without {T} in its activation cost"),
            None => {}
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::provenance::ProvNodeId;
    use crate::types::CardType;

    #[test]
    fn test_display() {
        let trigger =
            AbilityActivatedTrigger::new(PlayerFilter::Any, ObjectFilter::default(), false);
        assert!(trigger.display().contains("activates"));
    }

    #[test]
    fn named_ninjutsu_trigger_renders_and_matches_the_activated_ability() {
        let trigger = AbilityActivatedTrigger::new(
            PlayerFilter::You,
            ObjectFilter::default().with_ability_marker("ninjutsu"),
            false,
        );
        assert_eq!(
            trigger.display(),
            "Whenever you activate a ninjutsu ability"
        );

        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Ninjutsu Source",
        )
        .card_types(vec![CardType::Creature])
        .ninjutsu(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .build();
        let ability = definition
            .abilities
            .iter()
            .find(|ability| matches!(ability.kind, crate::ability::AbilityKind::Activated(_)))
            .cloned()
            .expect("ninjutsu builder should add an activated ability");

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let ctx = TriggerContext::for_source(crate::ids::ObjectId::from_raw(999), alice, &game);
        let event = TriggerEvent::new_with_provenance(
            AbilityActivatedEvent::new(source, alice, false).with_activated_ability(Some(ability)),
            ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));

        let unspecified_ability_event = TriggerEvent::new_with_provenance(
            AbilityActivatedEvent::new(source, alice, false),
            ProvNodeId::default(),
        );
        assert!(!trigger.matches(&unspecified_ability_event, &ctx));
    }
}

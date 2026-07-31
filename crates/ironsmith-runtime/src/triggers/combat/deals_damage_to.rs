//! "Whenever [source filter] deals damage to [target filter]" trigger.

use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::EventKind;
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{SimultaneousTriggerKey, TriggerContext, TriggerMatcher};
use ironsmith_core::trigger_model::DamageSourceSurface;

#[derive(Debug, Clone, PartialEq)]
pub struct DealsDamageToTrigger {
    pub source_filter: ObjectFilter,
    pub target_filter: ObjectFilter,
    pub combat_only: bool,
    pub source_surface: DamageSourceSurface,
}

impl DealsDamageToTrigger {
    pub fn new(source_filter: ObjectFilter, target_filter: ObjectFilter) -> Self {
        Self::with_source_surface(source_filter, target_filter, DamageSourceSurface::Filter)
    }

    pub fn with_source_surface(
        source_filter: ObjectFilter,
        target_filter: ObjectFilter,
        source_surface: DamageSourceSurface,
    ) -> Self {
        Self {
            source_filter,
            target_filter,
            combat_only: false,
            source_surface,
        }
    }

    pub fn combat_only(source_filter: ObjectFilter, target_filter: ObjectFilter) -> Self {
        Self {
            source_filter,
            target_filter,
            combat_only: true,
            source_surface: DamageSourceSurface::Filter,
        }
    }

    fn cardinality_key(
        &self,
        source: crate::ids::ObjectId,
        target: crate::ids::ObjectId,
    ) -> Option<SimultaneousTriggerKey> {
        match (
            self.source_filter.union_is_one_or_more(),
            self.target_filter.union_is_one_or_more(),
        ) {
            (false, false) => None,
            (false, true) => Some(SimultaneousTriggerKey::DamageSource(source)),
            (true, false) => Some(SimultaneousTriggerKey::DamageTarget(DamageTarget::Object(
                target,
            ))),
            (true, true) => Some(SimultaneousTriggerKey::DamageBatch),
        }
    }
}

impl TriggerMatcher for DealsDamageToTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(damage) = event.downcast::<DamageEvent>() else {
            return false;
        };
        if self.combat_only && !damage.is_combat {
            return false;
        }
        let Some(source_obj) = ctx.game.object(damage.source) else {
            return false;
        };
        if !self
            .source_filter
            .matches(source_obj, &ctx.filter_ctx, ctx.game)
        {
            return false;
        }
        let DamageTarget::Object(target_id) = damage.target else {
            return false;
        };
        let Some(target_obj) = ctx.game.object(target_id) else {
            return false;
        };
        if !self
            .target_filter
            .matches(target_obj, &ctx.filter_ctx, ctx.game)
        {
            return false;
        }
        if self.combat_only
            && let Some(current_key) = self.cardinality_key(damage.source, target_id)
        {
            let already_matched = ctx.game.combat_damage_object_batch_hits().iter().any(
                |(prior_source, prior_target)| {
                    if self.cardinality_key(*prior_source, *prior_target) != Some(current_key) {
                        return false;
                    }
                    let Some(prior_source_object) = ctx.game.object(*prior_source) else {
                        return false;
                    };
                    let Some(prior_target_object) = ctx.game.object(*prior_target) else {
                        return false;
                    };
                    self.source_filter
                        .matches(prior_source_object, &ctx.filter_ctx, ctx.game)
                        && self.target_filter.matches(
                            prior_target_object,
                            &ctx.filter_ctx,
                            ctx.game,
                        )
                },
            );
            if already_matched {
                return false;
            }
        }
        true
    }

    fn simultaneous_trigger_key(&self, event: &TriggerEvent) -> Option<SimultaneousTriggerKey> {
        let damage = event.downcast::<DamageEvent>()?;
        let DamageTarget::Object(target) = damage.target else {
            return None;
        };
        self.cardinality_key(damage.source, target)
    }

    fn display(&self) -> String {
        let target = damage_target_description(&self.target_filter);
        let source = if self.source_surface == DamageSourceSurface::Source {
            super::deals_damage::generic_source_description(&self.source_filter)
        } else {
            self.source_filter.description()
        };
        let source = super::deals_damage::correct_damage_source_indefinite_article(source);
        if self.source_surface == DamageSourceSurface::PassiveBy {
            let damage_kind = if self.combat_only {
                "combat damage"
            } else {
                "damage"
            };
            return format!("Whenever {target} is dealt {damage_kind} by {source}");
        }
        if self.combat_only {
            format!("Whenever {source} deals combat damage to {target}")
        } else {
            format!("Whenever {source} deals damage to {target}")
        }
    }
}

fn damage_target_description(filter: &ObjectFilter) -> String {
    let one_or_more = filter.union_is_one_or_more();
    let mut surface_filter = filter.clone();
    surface_filter.set_union_one_or_more(false);
    let description = surface_filter.description();
    if one_or_more {
        return format!("one or more {}", pluralize_damage_recipient(&description));
    }
    match description.as_str() {
        "artifact" | "enchantment" => format!("an {description}"),
        "creature" | "land" | "permanent" | "planeswalker" | "battle" => format!("a {description}"),
        description => {
            // A qualified singular noun still takes its indefinite article
            // ("a non-Wall creature"); phrases that already open with a
            // determiner or quantifier are left alone.
            const DETERMINED: &[&str] = &[
                "a ", "an ", "the ", "each ", "all ", "any ", "target ", "another ", "that ",
                "this ", "one ", "two ", "three ", "up to ", "x ",
            ];
            const SINGULAR_NOUNS: &[&str] = &[
                " creature",
                " land",
                " permanent",
                " planeswalker",
                " battle",
                " artifact",
                " enchantment",
            ];
            let lower = description.to_ascii_lowercase();
            if !DETERMINED.iter().any(|prefix| lower.starts_with(prefix))
                && SINGULAR_NOUNS.iter().any(|noun| lower.ends_with(noun))
            {
                let article = if lower.starts_with(['a', 'e', 'i', 'o', 'u']) {
                    "an"
                } else {
                    "a"
                };
                format!("{article} {description}")
            } else {
                description.to_string()
            }
        }
    }
}

fn pluralize_damage_recipient(description: &str) -> String {
    let description = description
        .strip_prefix("a ")
        .or_else(|| description.strip_prefix("an "))
        .unwrap_or(description);
    if description.ends_with('s') {
        return description.to_string();
    }
    if let Some(stem) = description.strip_suffix('y')
        && !stem
            .chars()
            .last()
            .is_some_and(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        return format!("{stem}ies");
    }
    format!("{description}s")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn damage(source: ObjectId, target: ObjectId, is_combat: bool) -> DamageEvent {
        let cause = if is_combat {
            crate::events::cause::EventCause::combat_damage(source)
        } else {
            crate::events::cause::EventCause::effect()
        };
        DamageEvent::with_cause(source, DamageTarget::Object(target), 2, is_combat, cause)
    }

    #[test]
    fn test_matches_combat_damage_to_matching_target() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Source", alice);
        let target = create_creature(&mut game, "Target", bob);

        let trigger =
            DealsDamageToTrigger::combat_only(ObjectFilter::creature(), ObjectFilter::creature());
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            damage(source, target, true),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_noncombat_damage_when_combat_only() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Source", alice);
        let target = create_creature(&mut game, "Target", bob);

        let trigger =
            DealsDamageToTrigger::combat_only(ObjectFilter::creature(), ObjectFilter::creature());
        let ctx = TriggerContext::for_source(source, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            damage(source, target, false),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn generic_source_surface_renders_source_without_changing_filters() {
        let source_filter = ObjectFilter::default();
        let target_filter = ObjectFilter::creature();
        let trigger = DealsDamageToTrigger::with_source_surface(
            source_filter.clone(),
            target_filter.clone(),
            DamageSourceSurface::Source,
        );

        assert_eq!(trigger.source_filter, source_filter);
        assert_eq!(trigger.target_filter, target_filter);
        assert_eq!(
            trigger.display(),
            "Whenever a source deals damage to a creature"
        );
    }
}

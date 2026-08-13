//! Mechanical helpers used while materializing the normalized compiler model.
//!
//! Semantic correlation, reference repair, and post-lowering interpretation do
//! not belong in this layer.  Inputs reaching these helpers are already typed.

use super::*;

pub(crate) fn infer_triggered_ability_functional_zones_from_facts(
    trigger: &TriggerSpec,
    facts: &crate::runtime_backend::shared_types::TriggerFunctionalZoneFacts,
) -> Vec<Zone> {
    if let Some(explicit_zone) = &facts.explicit_zone {
        return vec![explicit_zone.clone()];
    }
    if facts.returns_self_from_graveyard {
        return vec![Zone::Graveyard];
    }
    if facts.discards_this_card {
        return vec![Zone::Hand];
    }
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => {
            infer_triggered_ability_functional_zones_from_facts(trigger, facts)
        }
        TriggerSpec::YouCastThisSpell => vec![Zone::Stack],
        TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            ..
        } => vec![Zone::Graveyard],
        _ => vec![Zone::Battlefield],
    }
}

pub(super) fn runtime_effects_to_costs(
    effects: Vec<crate::effect::Effect>,
) -> Result<Vec<crate::costs::Cost>, CardTextError> {
    effects
        .into_iter()
        .filter(|effect| !crate::costs::is_tagged_type_marker_effect(effect))
        .map(|effect| {
            crate::costs::payment_effect_to_cost(effect).map_err(CardTextError::InvariantViolation)
        })
        .collect()
}

pub(super) fn rewrite_finalize_lowered_card(
    builder: CardDefinitionBuilder,
    _state: &mut RewriteLoweredCardState,
) -> CardDefinitionBuilder {
    builder
}

#[cfg(test)]
mod chosen_type_search_destination_tests {
    use super::*;
    use crate::target::PlayerFilter;

    fn program(move_tag: &str, reveal: bool) -> crate::resolution::ResolutionProgram {
        let search = crate::effect::Effect::new(crate::effects::SearchLibraryEffect::to_hand(
            ObjectFilter::creature().in_zone(Zone::Library),
            PlayerFilter::You,
            reveal,
        ))
        .tag("searched");
        let move_card = crate::effect::Effect::move_to_zone(
            ChooseSpec::Tagged(TagKey::from(move_tag)),
            Zone::Battlefield,
            false,
        )
        .tag("moved");
        let rider = crate::effect::Effect::conditional(
            crate::effect::Condition::ThisSpellPaidLabel(crate::cost::OptionalCostRef::new(
                crate::cost::OptionalCostKind::Additional,
            )),
            vec![move_card],
            vec![],
        );
        crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![search]),
            crate::resolution::ResolutionSegment::from_effects(vec![rider]),
        ])
    }

    #[test]
    fn chosen_type_search_uses_one_library_zone_change() {
        let mut matching = program("searched", true);
        correlate_additional_cost_chosen_type_search_destination(&mut matching);
        assert_eq!(matching.segments.len(), 1);
        let [branch] = matching.segments[0].self_replacements.as_slice() else {
            panic!("expected one destination self-replacement: {matching:#?}");
        };
        let local = branch.replacement_effects[0]
            .downcast_ref::<crate::effects::LocalRewriteEffect>()
            .expect("replacement should wrap the original search");
        let [replacement] = local.zone_replacements.as_slice() else {
            panic!("expected one local zone replacement: {local:#?}");
        };
        assert_eq!(replacement.from_zone, Some(Zone::Library));
        assert_eq!(replacement.to_zone, Some(Zone::Hand));
        assert_eq!(replacement.replacement_zone, Zone::Battlefield);
        assert!(matches!(
            &replacement.target,
            ChooseSpec::Object(filter) if filter.chosen_creature_type
        ));
    }

    #[test]
    fn uncorrelated_or_unrevealed_searches_are_near_misses() {
        for mut near_miss in [program("other", true), program("searched", false)] {
            correlate_additional_cost_chosen_type_search_destination(&mut near_miss);
            assert_eq!(near_miss.segments.len(), 2, "{near_miss:#?}");
        }
    }
}

#[cfg(test)]
mod additional_cost_damage_replacement_tests {
    use super::*;

    fn base_target() -> crate::target::ChooseSpec {
        crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
            crate::target::ObjectFilter::any_of_types(&[
                crate::types::CardType::Creature,
                crate::types::CardType::Planeswalker,
            ]),
        ))
    }

    fn program(
        base_target: crate::target::ChooseSpec,
        replacement_target: crate::target::ChooseSpec,
        replacement_combat: bool,
    ) -> crate::resolution::ResolutionProgram {
        let base = crate::effect::Effect::new(crate::effects::DealDamageEffect::new(
            crate::effect::Value::Fixed(2),
            base_target,
        ));
        let mut replacement_damage = crate::effects::DealDamageEffect::new(
            crate::effect::Value::Fixed(4),
            replacement_target,
        );
        replacement_damage.source_is_combat = replacement_combat;
        let replacement = crate::effect::Effect::new(crate::effects::ConditionalEffect::if_only(
            crate::effect::Condition::ThisSpellPaidLabel(crate::cost::OptionalCostRef::new(
                crate::cost::OptionalCostKind::Additional,
            )),
            vec![crate::effect::Effect::new(replacement_damage)],
        ));
        crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![base]),
            crate::resolution::ResolutionSegment::from_effects(vec![replacement]),
        ])
    }

    #[test]
    fn refreshed_instead_correlates_only_matching_noncombat_implicit_damage_replacement() {
        let explicit = base_target();
        let implicit = crate::target::ChooseSpec::PlayerOrPlaneswalker(
            crate::target::PlayerFilter::Any,
        );
        let mut matching = program(explicit.clone(), implicit.clone(), false);
        correlate_additional_cost_damage_replacement(&mut matching);
        assert_eq!(matching.segments.len(), 1);
        let conditional = matching.segments[0].default_effects[0]
            .downcast_ref::<crate::effects::ConditionalEffect>()
            .expect("matching program should become one conditional");
        assert_eq!(
            conditional.if_true[0]
                .downcast_ref::<crate::effects::DealDamageEffect>()
                .expect("replacement damage")
                .target,
            explicit
        );

        let wrong_target = crate::target::ChooseSpec::Player(crate::target::PlayerFilter::Any);
        let mut target_near_miss = program(explicit.clone(), wrong_target, false);
        correlate_additional_cost_damage_replacement(&mut target_near_miss);
        assert_eq!(target_near_miss.segments.len(), 2);

        let mut source_near_miss = program(explicit, implicit, true);
        correlate_additional_cost_damage_replacement(&mut source_near_miss);
        assert_eq!(source_near_miss.segments.len(), 2);
    }
}

#[cfg(test)]
mod quantified_player_damage_value_tests {
    use super::*;

    fn program_with_damage_target(target: crate::target::PlayerFilter) -> ResolutionProgram {
        let amount =
            crate::effect::Value::HalfRoundedDown(Box::new(crate::effect::Value::LifeTotal(
                crate::target::PlayerFilter::Target(Box::new(crate::target::PlayerFilter::Any)),
            )));
        ResolutionProgram::from_effects(vec![crate::effect::Effect::new(
            crate::effects::ForPlayersEffect {
                filter: crate::target::PlayerFilter::Any,
                effects: vec![crate::effect::Effect::new(
                    crate::effects::DealDamageEffect::new(
                        amount,
                        crate::target::ChooseSpec::Player(target),
                    ),
                )],
                starting_with_controller: false,
                stop_after_first_happened: false,
            },
        )])
    }

    fn life_total_player(program: &ResolutionProgram) -> &crate::target::PlayerFilter {
        let for_players = program.segments[0].default_effects[0]
            .downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
            .expect("quantified player effect");
        let damage = for_players.effects[0]
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .expect("damage action");
        let crate::effect::Value::HalfRoundedDown(inner) = &damage.amount else {
            panic!("expected half-rounded-down amount: {damage:#?}");
        };
        let crate::effect::Value::LifeTotal(player) = inner.as_ref() else {
            panic!("expected life-total basis: {damage:#?}");
        };
        player
    }

    #[test]
    fn binds_that_players_life_total_to_the_quantified_damage_recipient() {
        let mut program = program_with_damage_target(crate::target::PlayerFilter::IteratedPlayer);
        bind_quantified_player_damage_values(&mut program);
        assert_eq!(
            life_total_player(&program),
            &crate::target::PlayerFilter::IteratedPlayer
        );
    }

    #[test]
    fn does_not_bind_a_value_when_damage_has_a_different_recipient() {
        let mut program = program_with_damage_target(crate::target::PlayerFilter::You);
        bind_quantified_player_damage_values(&mut program);
        assert_eq!(
            life_total_player(&program),
            &crate::target::PlayerFilter::Target(Box::new(crate::target::PlayerFilter::Any))
        );
    }
}

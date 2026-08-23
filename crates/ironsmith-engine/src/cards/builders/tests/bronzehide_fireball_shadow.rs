#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn transparent_provenance_leaf(effect: &Effect) -> &Effect {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return transparent_provenance_leaf(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return transparent_provenance_leaf(&tagged.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return transparent_provenance_leaf(&tagged.effect);
    }
    effect
}

#[test]
fn bronzehide_lion_keeps_atomic_aura_return_loss_and_granted_activation() {
    let definition = parse_oracle_card_definition("Bronzehide Lion");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Bronzehide Lion"]
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Bronzehide Lion should retain its dies trigger");
    let flattened = triggered.effects.flattened_default_effects();
    let attachment = flattened
        .iter()
        .find_map(|effect| {
            transparent_provenance_leaf(effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        })
        .expect("the returned Aura should choose its legal attachment");
    assert_eq!(attachment.filter.card_types, [CardType::Creature]);
    assert_eq!(attachment.filter.controller, Some(PlayerFilter::You));
    let returned = flattened
        .iter()
        .find_map(|effect| {
            transparent_provenance_leaf(effect)
                .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
        })
        .expect("the dies trigger should atomically return the source as an Aura");
    let aura = returned
        .as_aura
        .as_ref()
        .expect("the returned source should carry its Aura attachment specification");
    assert!(aura.remove_all_abilities);
    assert!(
        aura.attachment_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag == attachment.tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            })
    );
    assert!(flattened.iter().any(|effect| {
        transparent_provenance_leaf(effect)
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            .is_some_and(|apply| {
                matches!(
                    &apply.modification,
                    Some(crate::continuous::Modification::AddAbilityGeneric(_))
                )
            })
    }));
}

#[test]
fn fireball_keeps_even_rounded_down_distribution_and_source_line_order() {
    let definition = parse_oracle_card_definition("Fireball");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Fireball"]
    );

    let distributed = definition
        .spell_effect
        .as_ref()
        .expect("Fireball should retain a spell-resolution program")
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DealDistributedDamageEffect>())
        .expect("Fireball should lower to typed distributed damage");
    assert_eq!(
        distributed.distribution,
        crate::effects::DamageDistributionMode::EvenRoundedDown
    );
    assert!(matches!(distributed.amount.unhinted(), Value::X));
    assert_eq!(
        distributed.target.count(),
        crate::effect::ChoiceCount::any_number()
    );
    assert!(distributed.get_target_distribution_value().is_none());
}

#[test]
fn shadow_mana_source_condition_qualifies_the_affected_spell_filter() {
    let definition = parse_oracle_card_definition("Shadow the Hedgehog");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Shadow the Hedgehog"]
    );

    let filter = definition
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            let model = static_ability.compiled_model()?;
            match &model.payload {
                ironsmith_core::StaticAbilityPayload::GrantAbility(grant)
                    if grant.condition.is_none()
                        && matches!(
                            &grant.ability.kind,
                            ironsmith_core::AbilityKind::Static(granted)
                                if granted.id == Some(StaticAbilityId::SplitSecond)
                        ) =>
                {
                    Some(&grant.filter)
                }
                ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant)
                    if grant.condition.is_none()
                        && matches!(
                            &grant.ability.kind,
                            ironsmith_core::AbilityKind::Static(granted)
                                if granted.id == Some(StaticAbilityId::SplitSecond)
                        ) =>
                {
                    Some(&grant.filter)
                }
                _ => None,
            }
        })
        .expect("Shadow should retain a typed continuous split-second grant");
    let mana_source = filter
        .mana_from_source_spent_to_cast
        .as_deref()
        .expect("the affected spell filter should retain its mana-source predicate");
    assert!(
        mana_source.card_types == [CardType::Artifact]
            && filter.has_mana_source_spent_trailing_if_surface(),
        "the artifact-mana predicate must match each spell, not Shadow itself: {filter:#?}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Artifact Mana Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let land = CardDefinitionBuilder::new(CardId::new(), "Land Mana Source")
        .card_types(vec![CardType::Land])
        .build();
    let artifact = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
    let land = game.create_object_from_definition(&land, alice, Zone::Battlefield);
    let spell_definition = CardDefinitionBuilder::new(CardId::new(), "Shadow Grant Probe")
        .card_types(vec![CardType::Instant])
        .build();
    let artifact_spell = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
    let land_spell = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
    for (spell, mana_source) in [(artifact_spell, artifact), (land_spell, land)] {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(mana_source).expect("mana source exists"),
            &game,
        );
        game.object_mut(spell)
            .expect("spell exists")
            .cast_tagged_objects
            .insert(
                crate::tag::TagKey::from(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG),
                vec![snapshot],
            );
    }
    // `spells you cast` is evaluated from the live stack entry's caster. Keep
    // this fixture faithful to an actually cast spell instead of leaving two
    // unattached objects in the Stack zone.
    game.push_to_stack(crate::game_state::StackEntry::new(artifact_spell, alice));
    game.push_to_stack(crate::game_state::StackEntry::new(land_spell, alice));
    assert!(game.current_has_static_ability_id(artifact_spell, StaticAbilityId::SplitSecond));
    assert!(!game.current_has_static_ability_id(land_spell, StaticAbilityId::SplitSecond));
}

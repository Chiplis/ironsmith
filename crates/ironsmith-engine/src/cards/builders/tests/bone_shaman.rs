#![cfg(ironsmith_runtime_parser_tests)]

use super::*;

const BONE_SHAMAN_TEXT: &str = "{B}: Until end of turn, this creature gains \"Creatures dealt damage by this creature this turn can't be regenerated this turn.\"";

fn bone_shaman_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Bone Shaman")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Giant, Subtype::Shaman])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(BONE_SHAMAN_TEXT)
        .expect("Bone Shaman should parse")
}

#[test]
fn bone_shaman_keeps_its_quoted_source_relative_restriction_exactly() {
    let definition = bone_shaman_definition();

    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![BONE_SHAMAN_TEXT.to_string()]
    );
}

#[test]
fn bone_shaman_grant_tracks_only_creatures_damaged_by_its_source() {
    let definition = bone_shaman_definition();
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Bone Shaman should have an activated ability");
    let apply = activated
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ApplyContinuousEffect>())
        .expect("activation should grant a temporary static ability");
    assert_eq!(apply.until, crate::effect::Until::EndOfTurn);
    assert!(matches!(
        apply.modification.as_ref(),
        Some(crate::continuous::Modification::AddAbility(ability))
            if matches!(
                ability.compiled_model().map(|model| &model.payload),
                Some(ironsmith_core::StaticAbilityPayload::RuleRestriction {
                    restriction:
                        crate::effect::Restriction::BeRegenerated(filter),
                    ..
                }) if filter.dealt_damage_by_source_this_turn
                    == Some(ironsmith_core::DamagedBySource::ThisCreature)
            )
    ));

    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let bear = CardDefinitionBuilder::new(CardId::new(), "Test Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let damaged = game.create_object_from_definition(&bear, alice, Zone::Battlefield);
    let undamaged = game.create_object_from_definition(&bear, alice, Zone::Battlefield);

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Bone Shaman activation should resolve");
    }
    game.update_cant_effects();
    assert!(game.can_be_regenerated(damaged));
    assert!(game.can_be_regenerated(undamaged));

    let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Object(damaged),
            1,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&damage_event);
    game.update_cant_effects();

    assert!(!game.can_be_regenerated(damaged));
    assert!(game.can_be_regenerated(undamaged));
}

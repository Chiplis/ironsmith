#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Equipped creature gets +2/+1.\nWhen equipped creature dies, you may have this Equipment become a 2/1 Construct artifact creature with flying and ward {1}. If you do, it isn't an Equipment.\nEquip {1}";

fn dies_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Dancing Sword should have its equipped-creature dies trigger")
}

#[test]
fn dancing_sword_keeps_linked_optional_animation_and_subtype_removal() {
    let definition = parse_oracle_card_definition("Dancing Sword");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let effects = dies_trigger(&definition)
        .effects
        .flattened_default_effects();
    let [offer_effect, result_effect] = effects else {
        panic!("expected an optional animation and its linked result: {effects:#?}");
    };
    let offer = offer_effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("the optional animation should carry a result ID");
    let may = offer
        .effect
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("the controller should choose whether to animate the Equipment");
    let [animation_effect] = may.effects.as_slice() else {
        panic!("the optional branch should contain one animation: {may:#?}");
    };
    let animation = animation_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .and_then(|tagged| {
            tagged
                .effect
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        })
        .expect("the optional branch should retain its typed source animation");
    assert!(matches!(
        &animation.modification,
        Some(crate::continuous::Modification::AddCardTypes(types))
            if types.contains(&CardType::Artifact) && types.contains(&CardType::Creature)
    ));

    let result = result_effect
        .downcast_ref::<crate::effects::IfEffect>()
        .expect("the subtype removal should depend on accepting the animation");
    assert_eq!(result.condition, offer.id);
    assert_eq!(result.predicate, crate::effect::EffectPredicate::Happened);
    assert!(result.else_.is_empty());
    let [removal_effect] = result.then.as_slice() else {
        panic!("the successful result should contain one subtype removal: {result:#?}");
    };
    let removal = removal_effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("the result should be a continuous characteristic change");
    assert!(matches!(
        &removal.modification,
        Some(crate::continuous::Modification::RemoveSubtypes(subtypes))
            if subtypes.as_slice() == [Subtype::Equipment]
    ));
}

#[test]
fn accepting_dancing_sword_animation_removes_equipment_but_declining_does_not() {
    let definition = parse_oracle_card_definition("Dancing Sword");
    let program = dies_trigger(&definition).effects.clone();
    let alice = PlayerId::from_index(0);

    let mut accept_game = crate::tests::test_helpers::setup_two_player_game();
    let accepted = accept_game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut accept = crate::decision::SelectFirstDecisionMaker;
    let mut accept_ctx = crate::effects::ExecutionContext::new(accepted, alice, &mut accept);
    crate::game_loop::execute_resolution_program(
        &mut accept_game,
        &mut accept_ctx,
        alice,
        accepted,
        &program,
        None,
        &[],
    )
    .expect("accepting Dancing Sword's optional animation should resolve");
    let accepted_chars = accept_game
        .calculated_characteristics(accepted)
        .expect("accepted Dancing Sword characteristics");
    assert_eq!(
        (accepted_chars.power, accepted_chars.toughness),
        (Some(2), Some(1))
    );
    assert!(accepted_chars.card_types.contains(&CardType::Artifact));
    assert!(accepted_chars.card_types.contains(&CardType::Creature));
    assert!(accepted_chars.subtypes.contains(&Subtype::Construct));
    assert!(!accepted_chars.subtypes.contains(&Subtype::Equipment));

    let mut decline_game = crate::tests::test_helpers::setup_two_player_game();
    let declined =
        decline_game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut decline = crate::decision::AutoPassDecisionMaker;
    let mut decline_ctx = crate::effects::ExecutionContext::new(declined, alice, &mut decline);
    crate::game_loop::execute_resolution_program(
        &mut decline_game,
        &mut decline_ctx,
        alice,
        declined,
        &program,
        None,
        &[],
    )
    .expect("declining Dancing Sword's optional animation should resolve");
    let declined_chars = decline_game
        .calculated_characteristics(declined)
        .expect("declined Dancing Sword characteristics");
    assert_eq!(
        (declined_chars.power, declined_chars.toughness),
        (None, None)
    );
    assert!(declined_chars.card_types.contains(&CardType::Artifact));
    assert!(!declined_chars.card_types.contains(&CardType::Creature));
    assert!(declined_chars.subtypes.contains(&Subtype::Equipment));
    assert!(!declined_chars.subtypes.contains(&Subtype::Construct));
}

#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn triggered(definition: &crate::cards::CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected a spell-cast triggered ability")
}

fn plural_keyword_grants(
    triggered: &crate::ability::TriggeredAbility,
) -> Vec<&crate::effects::ApplyContinuousEffect> {
    triggered
        .effects
        .flattened_default_effects()
        .iter()
        .filter_map(|effect| effect.downcast_ref::<crate::effects::IfEffect>())
        .flat_map(|result| result.then.iter())
        .filter_map(|effect| effect.downcast_ref::<crate::effects::ApplyContinuousEffect>())
        .filter(|grant| {
            matches!(
                grant.modification.as_ref(),
                Some(crate::continuous::Modification::AddAbility(ability))
                    if ability.id() == StaticAbilityId::Wither
            )
        })
        .collect()
}

#[test]
fn spinerock_grants_wither_to_the_original_spell_and_its_copy() {
    let definition = parse_oracle_card_definition("Spinerock Tyrant");
    let triggered = triggered(&definition);
    let cast = triggered
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()
        .expect("expected a spell-cast trigger");
    assert_eq!(
        cast.filter.as_ref().and_then(|filter| filter.target_count),
        Some(ironsmith_core::ChoiceCount::exactly(1)),
        "the triggering spell must have exactly one target"
    );

    let grants = plural_keyword_grants(triggered);
    let [original_grant, copied_grant] = grants.as_slice() else {
        panic!(
            "the successful copy result must grant Wither to exactly two stack objects: {:#?}",
            triggered.effects
        );
    };
    assert!(matches!(
        original_grant.target_spec.as_ref().map(ChooseSpec::base),
        Some(ChooseSpec::Tagged(tag)) if tag.as_str() == "triggering"
    ));
    assert!(matches!(
        copied_grant.target_spec.as_ref().map(ChooseSpec::base),
        Some(ChooseSpec::Tagged(tag)) if tag.as_str() == "__copied_stack_object__"
    ));
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Flying\nWither\nWhenever you cast an instant or sorcery spell with a single target, you may copy it. If you do, those spells gain wither. You may choose new targets for the copy."
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let spell_definition = CardDefinitionBuilder::new(CardId::new(), "Wither Grant Probe")
        .card_types(vec![CardType::Instant])
        .build();
    let original = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
    let copied = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
    let mut context = crate::effects::ExecutionContext::new_default(source, alice);
    context.set_tagged_objects(
        "triggering",
        vec![crate::snapshot::ObjectSnapshot::from_object(
            game.object(original).expect("original spell"),
            &game,
        )],
    );
    context.set_tagged_objects(
        "__copied_stack_object__",
        vec![crate::snapshot::ObjectSnapshot::from_object(
            game.object(copied).expect("copied spell"),
            &game,
        )],
    );
    for grant in grants {
        crate::effects::execute_effect(
            &mut game,
            &crate::effect::Effect::new((*grant).clone()),
            &mut context,
        )
        .expect("the proven stack-object grant should execute");
    }
    assert!(game.current_has_static_ability_id(original, StaticAbilityId::Wither));
    assert!(game.current_has_static_ability_id(copied, StaticAbilityId::Wither));
}

#[test]
fn a_singular_post_copy_grant_is_not_expanded_to_the_copy() {
    let oracle = "Whenever you cast an instant or sorcery spell, you may copy it. If you do, that spell gains wither. You may choose new targets for the copy.";
    let definition = CardDefinitionBuilder::new(CardId::new(), "Singular Copy Grant Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("singular copy grant near miss should parse");
    let grants = plural_keyword_grants(triggered(&definition));
    assert_eq!(
        grants.len(),
        1,
        "a singular source reference must not grant the keyword to the copy"
    );
    assert!(
        !canonical_compiled_lines(&definition)
            .join("\n")
            .contains("those spells gain"),
        "the plural compactor must not claim a singular post-copy grant"
    );
}

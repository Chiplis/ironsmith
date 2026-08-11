#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn triggered_ability(definition: &CardDefinition) -> &TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected one triggered ability")
}

fn find_nested<T: Clone + 'static>(effect: &crate::effect::Effect) -> Option<T> {
    if let Some(found) = effect.downcast_ref::<T>() {
        return Some(found.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_nested::<T>(child);
        }
    });
    found
}

#[test]
fn tuktuk_keeps_successful_destruction_provenance_and_exact_surface() {
    let definition = parse_oracle_card_definition("Tuktuk Scrapper");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Tuktuk Scrapper"],
    );
    let triggered = triggered_ability(&definition);
    let destroy = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .filter_map(find_nested::<crate::effects::TagAllEffect>)
        .find(|tagged| find_nested::<crate::effects::DestroyEffect>(&tagged.effect).is_some())
        .expect("destroy outcome should retain an identity tag");
    assert!(
        find_nested::<crate::effects::DestroyEffect>(&destroy.effect).is_some(),
        "only successful DestroyEffect results may feed the controller damage: {destroy:#?}",
    );
    let damage_segment = &triggered.effects.segments[1];
    let [conditional_effect] = damage_segment.default_effects.as_slice() else {
        panic!("expected one typed destruction-result condition: {damage_segment:#?}");
    };
    let conditional = conditional_effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .expect("typed destruction-result condition");
    assert!(matches!(
        &conditional.condition,
        crate::effect::Condition::TaggedObjectMatches(tag, _)
            if tag == &destroy.tag
    ));
}

fn resolve_tuktuk_against(indestructible: bool) -> (i32, Zone) {
    let definition = parse_oracle_card_definition("Tuktuk Scrapper");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let ally = CardDefinitionBuilder::new(CardId::new(), "Another Ally")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Ally])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_definition(&ally, alice, Zone::Battlefield);
    let mut artifact = CardDefinitionBuilder::new(CardId::new(), "Scrap Target")
        .card_types(vec![CardType::Artifact]);
    if indestructible {
        artifact = artifact.with_ability(Ability::static_ability(
            crate::static_abilities::StaticAbility::indestructible(),
        ));
    }
    let target = game.create_object_from_definition(&artifact.build(), bob, Zone::Battlefield);

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &triggered_ability(&definition).effects,
        None,
        &[],
    )
    .expect("Tuktuk's optional destruction should resolve");
    (
        game.life_total(bob),
        game.object(target)
            .map(|object| object.zone)
            .unwrap_or(Zone::Graveyard),
    )
}

#[test]
fn tuktuk_damages_only_after_the_artifact_is_successfully_destroyed() {
    assert_eq!(resolve_tuktuk_against(false), (18, Zone::Graveyard));
    assert_eq!(
        resolve_tuktuk_against(true),
        (20, Zone::Battlefield),
        "an indestructible artifact must not be tagged as destroyed or deal follow-up damage",
    );
}

#[test]
fn locke_scopes_the_one_cast_permission_to_the_exact_milled_collection() {
    let definition = parse_oracle_card_definition("Locke, Treasure Hunter");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle_text_by_name()["Locke, Treasure Hunter"],
    );
    let triggered = triggered_ability(&definition);
    let mill_tag = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .filter_map(find_nested::<crate::effects::TagAllEffect>)
        .find(|tagged| find_nested::<crate::effects::MillEffect>(&tagged.effect).is_some())
        .expect("the exact cards milled this way should be tagged");
    let grant = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested::<crate::effects::GrantPlayTaggedEffect>)
        .expect("milled-card cast permission");
    assert_eq!(grant.tag, mill_tag.tag);
    assert_eq!(grant.max_plays, Some(1));
    assert!(!grant.allow_land);
    assert_eq!(
        grant
            .surface
            .as_ref()
            .and_then(|surface| surface.object.as_ref()),
        Some(&ironsmith_core::GrantPlayTaggedObjectSurface::SpellsFromAmongThoseCards),
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let spell = |name: &str| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .build()
    };
    let unrelated =
        game.create_object_from_definition(&spell("Old Graveyard Spell"), alice, Zone::Graveyard);
    let alice_milled =
        game.create_object_from_definition(&spell("Alice Milled Spell"), alice, Zone::Library);
    let bob_milled =
        game.create_object_from_definition(&spell("Bob Milled Spell"), bob, Zone::Library);
    let alice_stable = game.object(alice_milled).expect("Alice spell").stable_id;
    let bob_stable = game.object(bob_milled).expect("Bob spell").stable_id;

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Locke's attack program should resolve");
    let alice_milled = game
        .find_object_by_stable_id(alice_stable)
        .expect("Alice's milled spell");
    let bob_milled = game
        .find_object_by_stable_id(bob_stable)
        .expect("Bob's milled spell");
    let registry = &game.effect_store.grant_registry;
    let alice_grants = registry.get_grants_for_card(&game, alice_milled, Zone::Graveyard, alice);
    let bob_grants = registry.get_grants_for_card(&game, bob_milled, Zone::Graveyard, alice);
    assert!(!alice_grants.is_empty() && !bob_grants.is_empty());
    assert!(
        registry
            .get_grants_for_card(&game, unrelated, Zone::Graveyard, alice)
            .is_empty(),
        "an unrelated card already in a graveyard must not gain the permission",
    );
    assert_eq!(
        alice_grants[0].shared_usage_id, bob_grants[0].shared_usage_id,
        "all cards in the result collection must consume the same one-cast budget",
    );
    assert!(alice_grants[0].shared_usage_id.is_some());
}

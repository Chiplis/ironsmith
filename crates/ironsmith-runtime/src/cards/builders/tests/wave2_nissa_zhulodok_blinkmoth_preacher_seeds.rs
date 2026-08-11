#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn nissa_shadowed_boughs_keeps_the_owned_zone_choice_and_entry_counters() {
    let definition = parse_oracle_card_definition("Nissa of Shadowed Boughs");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Landfall — Whenever a land you control enters, put a loyalty counter on Nissa.",
            "+1: Untap target land you control. You may have it become a 3/3 Elemental creature with haste and menace until end of turn. It's still a land.",
            "−5: You may put a creature card with mana value less than or equal to the number of lands you control onto the battlefield from your hand or graveyard with two +1/+1 counters on it.",
        ]
    );

    let minus_five = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .last()
        .expect("Nissa must retain her −5 loyalty ability");
    let debug = format!("{:#?}", minus_five.effects);
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("additional_zones"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert!(debug.contains("ManaValue"), "{debug}");
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
    assert!(debug.contains("Fixed(2"), "{debug}");
}

fn nissa_test_land() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Nissa Test Land")
        .card_types(vec![CardType::Land])
        .build()
}

fn nissa_test_creature() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Nissa Test Creature")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(3),
        ]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[test]
fn nissa_can_put_the_eligible_card_from_either_owned_zone() {
    let definition = parse_oracle_card_definition("Nissa of Shadowed Boughs");
    let minus_five = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .last()
        .expect("Nissa −5");

    for origin in [Zone::Hand, Zone::Graveyard] {
        let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for _ in 0..3 {
            game.create_object_from_definition(&nissa_test_land(), alice, Zone::Battlefield);
        }
        let chosen = game.create_object_from_definition(&nissa_test_creature(), alice, origin);
        let chosen_stable = game.object(chosen).expect("chosen card").stable_id;

        let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut ctx,
            alice,
            source,
            &minus_five.effects,
            None,
            &[],
        )
        .expect("Nissa −5 should resolve");

        let moved_id = game
            .find_object_by_stable_id(chosen_stable)
            .expect("chosen card remains represented after its zone change");
        let moved = game.object(moved_id).expect("moved chosen card");
        assert_eq!(moved.zone, Zone::Battlefield, "origin {origin:?}");
        assert_eq!(
            moved
                .counters
                .get(&crate::object::CounterType::PlusOnePlusOne)
                .copied(),
            Some(2),
            "origin {origin:?}"
        );
    }
}

#[test]
fn blinkmoth_urn_keeps_the_event_player_as_recipient_and_artifact_controller() {
    const ORACLE: &str = "At the beginning of each player's first main phase, if this artifact is untapped, that player adds {C} for each artifact they control.";
    let definition = parse_oracle_card_definition("Blinkmoth Urn");
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Blinkmoth Urn must retain its main-phase trigger");
    assert!(
        triggered
            .intervening_if
            .as_ref()
            .is_some_and(|condition| match condition {
                crate::effect::Condition::SourceIsUntapped => true,
                crate::effect::Condition::Not(inner) => {
                    matches!(inner.as_ref(), crate::effect::Condition::SourceIsTapped)
                }
                _ => false,
            })
    );
    let [effect] = triggered.effects.flattened_default_effects() else {
        panic!("expected one scaled-mana effect: {triggered:#?}");
    };
    let add = effect
        .downcast_ref::<crate::effects::AddScaledManaEffect>()
        .expect("typed scaled mana");
    assert_eq!(add.player, PlayerFilter::IteratedPlayer);
    assert!(matches!(
        add.amount.unhinted(),
        crate::effect::Value::Count(filter)
            if filter.controller == Some(PlayerFilter::IteratedPlayer)
                && filter.card_types == [CardType::Artifact]
    ));
}

#[test]
fn preacher_keeps_both_distinct_most_life_attack_conditions() {
    let definition = parse_oracle_card_definition("Preacher of the Schism");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Deathtouch",
            "Whenever this creature attacks the player with the most life or tied for most life, create a 1/1 white Vampire creature token with lifelink.",
            "Whenever this creature attacks while you have the most life or are tied for most life, you draw a card and you lose 1 life.",
        ]
    );

    let triggered = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(triggered.len(), 2);
    let first = triggered[0]
        .trigger
        .downcast_ref::<crate::triggers::AttacksTrigger>()
        .expect("first ability keeps a typed attacked-player filter");
    assert_eq!(
        first.filter.attacking_player_or_planeswalker_controlled_by,
        Some(PlayerFilter::MostLifeTied)
    );
    assert_eq!(
        first.filter.targets_only_player,
        Some(PlayerFilter::MostLifeTied)
    );
    assert_eq!(
        triggered[1].intervening_if,
        Some(
            crate::effect::Condition::PlayerHasNoOpponentWithMoreLifeThan {
                player: PlayerFilter::You,
            }
        )
    );
}

#[test]
fn zhulodok_keeps_hand_origin_and_two_independent_cascade_grants() {
    let definition = parse_oracle_card_definition("Zhulodok, Void Gorger");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Colorless spells you cast from your hand with mana value 7 or greater have \"Cascade, cascade.\""
        ]
    );

    let grants = definition
        .abilities
        .iter()
        .filter_map(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            let model = static_ability.compiled_model()?;
            let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
                &model.payload
            else {
                return None;
            };
            Some(grant)
        })
        .collect::<Vec<_>>();
    assert_eq!(grants.len(), 2, "{:#?}", definition.abilities);
    assert_eq!(grants[0].filter, grants[1].filter);
    assert_eq!(grants[0].filter.zone, Some(Zone::Hand));
    assert_eq!(
        grants[0].filter.stack_kind,
        Some(crate::filter::StackObjectKind::Spell)
    );
    assert_eq!(grants[0].filter.cast_by, Some(PlayerFilter::You));
    assert!(grants[0].filter.colorless);
    assert_eq!(
        grants[0].filter.mana_value,
        Some(crate::filter::Comparison::GreaterThanOrEqual(7))
    );
    for grant in grants {
        let ironsmith_core::AbilityKind::Static(granted) = &grant.ability.kind else {
            panic!("cascade grant must remain static: {grant:#?}");
        };
        assert_eq!(
            granted.id,
            Some(crate::static_abilities::StaticAbilityId::Cascade)
        );
    }
}

fn artifact(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(mana_value),
        ]]))
        .card_types(vec![CardType::Artifact])
        .build()
}

#[test]
fn seeds_rewards_each_destroyed_artifacts_own_controller() {
    const ORACLE: &str = "Destroy all artifacts. They can't be regenerated. The controller of each of those artifacts gains life equal to its mana value.";
    let definition = parse_oracle_card_definition("Seeds of Innocence");
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);

    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.create_object_from_definition(&artifact("Alice Relic", 2), alice, Zone::Battlefield);
    game.create_object_from_definition(&artifact("Bob Relic", 4), bob, Zone::Battlefield);

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        definition
            .spell_effect
            .as_ref()
            .expect("Seeds spell program"),
        None,
        &[],
    )
    .expect("Seeds should resolve");

    assert_eq!(game.player(alice).expect("Alice").life, 22);
    assert_eq!(game.player(bob).expect("Bob").life, 24);
    assert!(
        game.objects_in_zone(Zone::Battlefield)
            .into_iter()
            .filter_map(|id| game.object(id))
            .all(|object| !object.card_types.contains(&CardType::Artifact))
    );
}

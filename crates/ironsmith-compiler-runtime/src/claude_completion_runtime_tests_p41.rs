//! Claude package 41 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

const WEDDING_OF_RIVER_SONG: &str = "Mana cost: {2}{W}\nType: Sorcery\nDraw two cards, then you may exile a nonland card from your hand with a number of time counters on it equal to its mana value. Then target opponent does the same. Cards exiled this way that don't have suspend gain suspend.\nTime travel. (For each suspended card you own and each permanent you control with a time counter on it, you may add or remove a time counter.)";

#[test]
fn wedding_of_river_song_replays_draw_and_optional_exile_for_the_target_opponent() {
    use ironsmith::card::CardBuilder;
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::object::CounterType;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};

    struct Accept;
    impl ironsmith::decision::DecisionMaker for Accept {
        fn decide_boolean(
            &mut self,
            _: &ironsmith::GameState,
            _: &ironsmith::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
        fn decide_objects(
            &mut self,
            game: &ironsmith::GameState,
            ctx: &ironsmith::decisions::context::SelectObjectsContext,
        ) -> Vec<ironsmith::ObjectId> {
            ironsmith::decision::DecisionMaker::decide_objects(
                &mut ironsmith::decision::SelectFirstDecisionMaker,
                game,
                ctx,
            )
        }
    }

    let definition =
        compile_to_runtime_definition("The Wedding of River Song", WEDDING_OF_RIVER_SONG, false)
            .unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let land = CardBuilder::new(CardId::new(), "Library land")
        .card_types(vec![CardType::Land])
        .build();
    for player in [alice, bob] {
        for _ in 0..3 {
            game.create_object_from_card(&land, player, Zone::Library);
        }
    }
    let alice_spell = CardBuilder::new(CardId::new(), "Alice spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
            ironsmith::ManaSymbol::Generic(3),
            ironsmith::ManaSymbol::White,
        ]))
        .build();
    let bob_spell = CardBuilder::new(CardId::new(), "Bob spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
            ironsmith::ManaSymbol::Generic(1),
            ironsmith::ManaSymbol::Blue,
        ]))
        .build();
    game.create_object_from_card(&alice_spell, alice, Zone::Hand);
    game.create_object_from_card(&bob_spell, bob, Zone::Hand);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let target_spec = definition
        .spell_effect
        .as_ref()
        .unwrap()
        .all_effects()
        .into_iter()
        .find_map(|effect| effect.0.get_target_spec())
        .expect("the replay announces its opponent target")
        .clone();
    let mut dm = Accept;
    let mut ctx = EffectContext::new(source, alice, &mut dm)
        .with_targets(vec![ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![ironsmith::game_state::TargetAssignment {
            spec: target_spec,
            range: 0..1,
        }]);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        // Time travel would let the controller re-tune the counters checked below.
        if effect
            .downcast_ref::<ironsmith::effects::ForEachCounterKindPutOrRemoveEffect>()
            .is_some()
        {
            continue;
        }
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);

    // Each player drew two lands and exiled their own nonland card.
    assert_eq!(game.player(alice).unwrap().hand.len(), 2);
    assert_eq!(game.player(bob).unwrap().hand.len(), 2);
    assert_eq!(game.player(alice).unwrap().library.len(), 1);
    assert_eq!(game.player(bob).unwrap().library.len(), 1);
    let exiled = |name: &str| {
        game.exile
            .iter()
            .copied()
            .find(|id| game.object(*id).is_some_and(|object| object.name == name))
            .unwrap_or_else(|| panic!("{name} is exiled"))
    };
    let alice_exiled = exiled("Alice spell");
    let bob_exiled = exiled("Bob spell");
    assert_eq!(game.object(alice_exiled).unwrap().owner, alice);
    assert_eq!(game.object(bob_exiled).unwrap().owner, bob);
    assert_eq!(game.counter_count(alice_exiled, CounterType::Time), 4);
    assert_eq!(game.counter_count(bob_exiled, CounterType::Time), 2);
}

const VOLRATHS_SHAPESHIFTER: &str = "Mana cost: {1}{U}{U}\nType: Creature — Phyrexian Shapeshifter\nPower/Toughness: 0/1\nAs long as the top card of your graveyard is a creature card, this creature has the full text of that card and has the text \"{2}: Discard a card.\" (This creature has that card's name, mana cost, color, types, abilities, power, and toughness.)\n{2}: Discard a card.";

#[test]
fn volraths_shapeshifter_tracks_the_top_creature_card_of_its_controllers_graveyard() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::static_abilities::StaticAbilityId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};

    let definition =
        compile_to_runtime_definition("Volrath's Shapeshifter", VOLRATHS_SHAPESHIFTER, false)
            .unwrap();
    let statics = definition
        .abilities
        .iter()
        .filter(|ability| {
            matches!(&ability.kind, AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::SourceHasFullTextOfTopGraveyardCard)
        })
        .count();
    assert_eq!(statics, 1, "{:#?}", definition.abilities);
    fn activated(abilities: &[ironsmith::ability::Ability]) -> usize {
        abilities
            .iter()
            .filter(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
            .count()
    }
    assert_eq!(activated(&definition.abilities), 1, "the printed discard ability");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let shapeshifter = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(shapeshifter), Some(0));
    assert_eq!(game.calculated_toughness(shapeshifter), Some(1));

    let giant = CardBuilder::new(CardId::new(), "Graveyard Giant")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 4))
        .build();
    let land = CardBuilder::new(CardId::new(), "Graveyard Land")
        .card_types(vec![CardType::Land])
        .build();
    // An opponent's graveyard never matters.
    game.create_object_from_card(&giant, bob, Zone::Graveyard);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(shapeshifter), Some(0));

    game.create_object_from_card(&giant, alice, Zone::Graveyard);
    game.refresh_continuous_state().unwrap();
    let chars = game.calculated_characteristics(shapeshifter).unwrap();
    assert_eq!(chars.name, "Graveyard Giant");
    assert_eq!(game.calculated_power(shapeshifter), Some(5));
    assert_eq!(game.calculated_toughness(shapeshifter), Some(4));
    assert_eq!(
        activated(&chars.abilities),
        1,
        "the copied text plus the quoted {{2}}: Discard a card."
    );
    // The overwrite removed its own static ability, yet it keeps applying.
    assert!(!chars.abilities.iter().any(|ability| matches!(&ability.kind,
        AbilityKind::Static(static_ability)
            if static_ability.id() == StaticAbilityId::SourceHasFullTextOfTopGraveyardCard)));
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(shapeshifter), Some(5), "stable on a second pass");

    // A noncreature card on top ends the overwrite.
    game.create_object_from_card(&land, alice, Zone::Graveyard);
    game.refresh_continuous_state().unwrap();
    let chars = game.calculated_characteristics(shapeshifter).unwrap();
    assert_eq!(chars.name, "Volrath's Shapeshifter");
    assert_eq!(game.calculated_power(shapeshifter), Some(0));
    assert_eq!(game.calculated_toughness(shapeshifter), Some(1));
}

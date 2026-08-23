use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{DecisionMaker, compute_legal_attackers};
use ironsmith::events::other::CardsDrawnEvent;
use ironsmith::object::AttachmentTarget;
use ironsmith::prevention::{DamageFilter, PreventionShield, PreventionTarget};
use ironsmith::rules::{StateBasedAction, check_state_based_actions};
use ironsmith::triggers::Trigger;
use ironsmith::{
    Ability, CardDefinition, CardId, CardType, ChooseSpec, Effect, EffectContext, GameState,
    ObjectFilter, ObjectId, PlayerFilter, PlayerFilterExt, PlayerId, PowerToughness, Subtype,
    Supertype, TriggerEvent, Zone, check_triggers, compute_legal_targets, execute_effect,
};

struct TestDecisionMaker;
impl DecisionMaker for TestDecisionMaker {}

fn players() -> Vec<PlayerId> {
    (0..5).map(PlayerId::from_index).collect()
}

fn limited_game(ranges: Vec<u8>) -> GameState {
    let mut game = GameState::new(
        vec![
            "Alice".into(),
            "Bob".into(),
            "Charlie".into(),
            "Diana".into(),
            "Eve".into(),
        ],
        20,
    );
    game.enable_limited_range_of_influence(players(), ranges)
        .expect("valid circular seating");
    game
}

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[test]
fn u067_range_is_asymmetric_self_inclusive_and_frozen_until_the_next_turn() {
    let [alice, bob, charlie, diana, eve] = players().try_into().expect("five players");
    let mut game = limited_game(vec![1, 2, 0, 1, 1]);

    assert!(game.player_is_within_range(alice, alice));
    assert!(game.player_is_within_range(alice, bob));
    assert!(game.player_is_within_range(alice, eve));
    assert!(!game.player_is_within_range(alice, charlie));
    assert!(game.player_is_within_range(bob, diana));
    assert!(!game.player_is_within_range(charlie, bob));

    game.player_mut(bob).expect("Bob").has_left_game = true;
    assert!(game.player_is_within_range(alice, bob));
    assert!(!game.player_is_within_range(alice, charlie));

    game.next_turn();
    assert!(game.player_is_within_range(alice, charlie));
    assert!(game.player_is_within_range(alice, eve));
    assert_eq!(
        game.closest_in_game_player_to_left_matching(alice, |_| true),
        Some(charlie)
    );
}

#[test]
fn u067_targets_filters_and_attack_options_exclude_out_of_range_players_and_objects() {
    let [alice, bob, charlie, _diana, eve] = players().try_into().expect("five players");
    let mut game = limited_game(vec![1; 5]);
    let source = game.create_object_from_definition(&creature("Source"), alice, Zone::Battlefield);
    game.remove_summoning_sickness(source);
    let bob_creature =
        game.create_object_from_definition(&creature("Bob Creature"), bob, Zone::Battlefield);
    let charlie_creature = game.create_object_from_definition(
        &creature("Charlie Creature"),
        charlie,
        Zone::Battlefield,
    );
    let siege = CardDefinitionBuilder::new(CardId::new(), "Distant Siege")
        .card_types(vec![CardType::Battle])
        .subtypes(vec![Subtype::Siege])
        .defense(4)
        .build();
    let siege = game.create_object_from_definition(&siege, charlie, Zone::Battlefield);
    assert!(game.set_battle_protector(siege, bob));

    let legal = compute_legal_targets(&game, &ChooseSpec::AnyTarget, alice, Some(source));
    assert!(legal.contains(&ironsmith::Target::Player(bob)));
    assert!(legal.contains(&ironsmith::Target::Player(eve)));
    assert!(!legal.contains(&ironsmith::Target::Player(charlie)));
    assert!(legal.contains(&ironsmith::Target::Object(bob_creature)));
    assert!(!legal.contains(&ironsmith::Target::Object(charlie_creature)));
    assert!(
        !legal.contains(&ironsmith::Target::Object(siege)),
        "a Battle's general object range follows its controller, not its protector"
    );

    let filter_ctx = game.filter_context_for(alice, Some(source));
    assert!(PlayerFilter::Any.matches_player(bob, &filter_ctx));
    assert!(!PlayerFilter::Any.matches_player(charlie, &filter_ctx));

    let attacker = compute_legal_attackers(&game, &CombatState::default())
        .into_iter()
        .find(|option| option.creature == source)
        .expect("source can attack an in-range opponent");
    assert!(attacker.valid_targets.contains(&AttackTarget::Player(bob)));
    assert!(attacker.valid_targets.contains(&AttackTarget::Player(eve)));
    assert!(
        attacker
            .valid_targets
            .contains(&AttackTarget::Battle(siege)),
        "Battle attack eligibility follows its in-range protector"
    );
    assert!(
        !attacker
            .valid_targets
            .contains(&AttackTarget::Player(charlie))
    );
}

#[test]
fn u067_trigger_events_must_be_entirely_in_range_but_planes_are_exempt() {
    let [alice, bob, charlie, _diana, _eve] = players().try_into().expect("five players");
    let mut game = limited_game(vec![1; 5]);
    let observer = CardDefinitionBuilder::new(CardId::new(), "Draw Observer")
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::triggered(
            Trigger::player_draws_card(PlayerFilter::Any),
            vec![Effect::gain_life(1)],
        ))
        .build();
    let observer = game.create_object_from_definition(&observer, alice, Zone::Battlefield);
    let card = ObjectId::from_raw(90_067);
    let event = |player| {
        TriggerEvent::new(
            CardsDrawnEvent::single(player, card, false),
            ironsmith::provenance::ProvNodeId::default(),
        )
    };
    assert_eq!(check_triggers(&game, &event(bob)).len(), 1);
    assert!(check_triggers(&game, &event(charlie)).is_empty());

    let plane = CardDefinitionBuilder::new(CardId::new(), "Omniscient Plane")
        .card_types(vec![CardType::Plane])
        .with_ability(Ability::triggered(
            Trigger::player_draws_card(PlayerFilter::Any),
            vec![Effect::gain_life(1)],
        ))
        .build();
    let plane = game.create_object_from_definition(&plane, alice, Zone::Battlefield);
    let out_of_range = check_triggers(&game, &event(charlie));
    assert!(out_of_range.iter().any(|entry| entry.source == plane));
    assert!(!out_of_range.iter().any(|entry| entry.source == observer));
}

#[test]
fn u067_out_of_range_attachments_fail_and_the_world_rule_is_local_and_asymmetric() {
    let [alice, bob, charlie, _diana, _eve] = players().try_into().expect("five players");
    let mut game = limited_game(vec![1, 0, 0, 0, 0]);
    let charlie_creature = game.create_object_from_definition(
        &creature("Distant Creature"),
        charlie,
        Zone::Battlefield,
    );
    let aura = CardDefinitionBuilder::new(CardId::new(), "Distant Aura")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .enchants(ObjectFilter::creature())
        .build();
    let aura = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
    game.object_mut(aura).expect("Aura").attached_to =
        Some(AttachmentTarget::Object(charlie_creature));
    game.object_mut(charlie_creature)
        .expect("creature")
        .attachments
        .push(aura);
    let equipment = CardDefinitionBuilder::new(CardId::new(), "Distant Equipment")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let equipment = game.create_object_from_definition(&equipment, alice, Zone::Battlefield);
    game.object_mut(equipment).expect("Equipment").attached_to =
        Some(AttachmentTarget::Object(charlie_creature));
    game.object_mut(charlie_creature)
        .expect("creature")
        .attachments
        .push(equipment);

    let world = |name: &str| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .supertypes(vec![Supertype::World])
            .card_types(vec![CardType::Enchantment])
            .build()
    };
    let alice_world =
        game.create_object_from_definition(&world("Alice World"), alice, Zone::Battlefield);
    let bob_world = game.create_object_from_definition(&world("Bob World"), bob, Zone::Battlefield);
    let _charlie_world =
        game.create_object_from_definition(&world("Charlie World"), charlie, Zone::Battlefield);

    let actions = check_state_based_actions(&game);
    assert!(actions.contains(&StateBasedAction::AuraFallsOff(aura)));
    assert!(actions.contains(&StateBasedAction::AttachmentBecomesUnattached(equipment)));
    assert!(actions.contains(&StateBasedAction::WorldRuleViolation {
        permanents: vec![alice_world],
    }));
    assert!(!actions.iter().any(|action| {
        matches!(action, StateBasedAction::WorldRuleViolation { permanents } if permanents.contains(&bob_world))
    }));
}

#[test]
fn u067_winning_only_removes_opponents_in_the_winners_range() {
    let [alice, bob, charlie, diana, eve] = players().try_into().expect("five players");
    let mut game = limited_game(vec![1; 5]);
    let source = game.create_object_from_definition(&creature("Winner"), alice, Zone::Battlefield);
    let mut decisions = TestDecisionMaker;
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);
    execute_effect(&mut game, &Effect::win_the_game(), &mut ctx).expect("win effect resolves");

    assert!(game.player(bob).expect("Bob").has_lost);
    assert!(game.player(eve).expect("Eve").has_lost);
    assert!(!game.player(charlie).expect("Charlie").has_lost);
    assert!(!game.player(diana).expect("Diana").has_lost);
}

#[test]
fn u067_a_draw_effect_only_draws_its_controller_and_players_in_range() {
    let [alice, bob, charlie, diana, eve] = players().try_into().expect("five players");
    let mut game = limited_game(vec![1; 5]);
    let source =
        game.create_object_from_definition(&creature("Draw Source"), alice, Zone::Battlefield);
    let mut decisions = TestDecisionMaker;
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);
    execute_effect(&mut game, &Effect::draw_the_game(), &mut ctx).expect("draw effect resolves");

    assert!(game.player(alice).expect("Alice").has_left_game);
    assert!(game.player(bob).expect("Bob").has_left_game);
    assert!(game.player(eve).expect("Eve").has_left_game);
    assert!(game.player(charlie).expect("Charlie").is_in_game());
    assert!(game.player(diana).expect("Diana").is_in_game());
}

fn damage_after_prevention(protected: PreventionTarget, damage_filter: DamageFilter) -> u32 {
    let [alice, bob, charlie, _diana, _eve] = players().try_into().expect("five players");
    let mut game = limited_game(vec![1; 5]);
    let shield_source =
        game.create_object_from_definition(&creature("Shield"), alice, Zone::Battlefield);
    let damage_source =
        game.create_object_from_definition(&creature("Distant Source"), charlie, Zone::Battlefield);
    game.effect_store.prevention_effects.add_shield(
        PreventionShield::prevent_all(shield_source, alice, protected).with_filter(damage_filter),
    );
    ironsmith::events::processing::process_damage_assignments_with_event(
        &mut game,
        damage_source,
        ironsmith::events::DamageTarget::Player(bob),
        3,
        false,
        ironsmith::events::cause::EventCause::from_effect(damage_source, charlie),
    )
    .assignments
    .iter()
    .map(|assignment| assignment.amount)
    .sum()
}

#[test]
fn u067_prevention_range_depends_on_whether_source_or_recipient_is_specified() {
    let source_filter = DamageFilter {
        from_card_types: Some(vec![CardType::Creature]),
        ..Default::default()
    };
    assert_eq!(
        damage_after_prevention(PreventionTarget::All, source_filter),
        3,
        "source-specified prevention cannot see an out-of-range source"
    );
    assert_eq!(
        damage_after_prevention(
            PreventionTarget::Player(PlayerId::from_index(1)),
            DamageFilter::default(),
        ),
        0,
        "recipient-specified prevention can stop damage from an out-of-range source"
    );
    assert_eq!(
        damage_after_prevention(PreventionTarget::All, DamageFilter::default()),
        3,
        "nonspecific prevention requires both source and recipient in range"
    );
}

#[test]
fn u067_choice_fallback_is_chooser_only_and_information_extrema_are_range_local() {
    let [alice, bob, charlie, _diana, _eve] = players().try_into().expect("five players");
    let mut game = limited_game(vec![0; 5]);
    let source =
        game.create_object_from_definition(&creature("Choice Source"), alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice")
        .hand
        .push(ObjectId::from_raw(90_068));
    game.player_mut(bob)
        .expect("Bob")
        .hand
        .extend([ObjectId::from_raw(90_069), ObjectId::from_raw(90_070)]);
    let ctx = EffectContext::new_default(source, alice);

    assert!(
        ironsmith::effects::helpers::resolve_player_filter(&game, &PlayerFilter::Opponent, &ctx,)
            .is_err(),
        "801.5c must not make an out-of-range opponent an effect recipient"
    );
    assert_eq!(
        ironsmith::effects::helpers::resolve_player_filter_as_chooser(
            &game,
            &PlayerFilter::Opponent,
            &ctx,
        )
        .expect("closest appropriate opponent makes the choice"),
        bob,
    );
    assert_eq!(
        ironsmith::effects::helpers::resolve_player_filter(
            &game,
            &PlayerFilter::MostCardsInHand,
            &ctx,
        )
        .expect("only in-range information is considered"),
        alice,
    );

    let distant_life = game.player(charlie).expect("Charlie").life;
    let mut decisions = TestDecisionMaker;
    let mut execution =
        EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);
    execute_effect(
        &mut game,
        &Effect::lose_life_player(3, PlayerFilter::Specific(charlie)),
        &mut execution,
    )
    .expect("the out-of-range instruction resolves as a no-op");
    assert_eq!(game.player(charlie).expect("Charlie").life, distant_life);
}

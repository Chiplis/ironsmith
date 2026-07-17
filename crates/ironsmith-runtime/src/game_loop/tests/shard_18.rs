use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
fn chain_lightning_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Chain Lightning")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Chain Lightning deals 3 damage to any target. Then that player or that permanent's controller may pay {R}{R}. If the player does, they may copy this spell and may choose a new target for that copy.",
        )
        .expect("Chain Lightning should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
fn chain_of_vapor_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Chain of Vapor")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Return target nonland permanent to its owner's hand. Then that permanent's controller may sacrifice a land of their choice. If the player does, they may copy this spell and may choose a new target for that copy.",
        )
        .expect("Chain of Vapor should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[derive(Debug)]
struct AcceptChainAndRetarget {
    target: Target,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for AcceptChainAndRetarget {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }

    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        assert!(
            ctx.requirements
                .iter()
                .any(|requirement| requirement.legal_targets.contains(&self.target)),
            "requested chain-copy target should be legal: {ctx:#?}"
        );
        vec![self.target]
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chain_lightning_declining_payment_does_not_create_a_copy() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let definition = chain_lightning_definition();
    let spell_id = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice).with_targets(vec![Target::Player(bob)]));

    resolve_stack_entry_with(&mut game, &mut AutoPassDecisionMaker)
        .expect("declined Chain Lightning should resolve");

    assert_eq!(game.player(bob).expect("bob exists").life, 17);
    assert!(
        game.stack.is_empty(),
        "declining payment must end the chain"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chain_lightning_payment_copies_for_the_payer_and_legally_retargets() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.player_mut(bob)
        .expect("bob exists")
        .mana_pool
        .add(ManaSymbol::Red, 2);

    let definition = chain_lightning_definition();
    let spell_id = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice).with_targets(vec![Target::Player(bob)]));
    let mut decisions = AcceptChainAndRetarget {
        target: Target::Player(alice),
    };

    resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("paid Chain Lightning should resolve and create a copy");

    assert_eq!(game.player(bob).expect("bob exists").life, 17);
    assert_eq!(game.player(bob).expect("bob exists").mana_pool.red, 0);
    let [copy_entry] = game.stack.as_slice() else {
        panic!(
            "accepted chain should leave exactly one copy on the stack: {:#?}",
            game.stack
        );
    };
    assert_eq!(
        copy_entry.controller, bob,
        "the paying player controls the copy"
    );
    assert_eq!(copy_entry.targets, vec![Target::Player(alice)]);
    assert!(
        game.object(copy_entry.object_id)
            .is_some_and(|object| matches!(object.kind, ObjectKind::SpellCopy)),
        "the continuation must create a stack spell copy"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chain_of_vapor_uses_the_bounced_permanents_last_known_controller() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let borrowed = CardBuilder::new(CardId::new(), "Borrowed Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let borrowed_id = game.create_object_from_card(&borrowed, alice, Zone::Battlefield);
    game.set_current_controller(borrowed_id, bob);

    let land = CardBuilder::new(CardId::new(), "Bob's Island")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land, bob, Zone::Battlefield);
    let next_target = CardBuilder::new(CardId::new(), "Next Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let next_target_id = game.create_object_from_card(&next_target, alice, Zone::Battlefield);

    let definition = chain_of_vapor_definition();
    let spell_id = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, alice).with_targets(vec![Target::Object(borrowed_id)]),
    );
    let mut decisions = AcceptChainAndRetarget {
        target: Target::Object(next_target_id),
    };

    resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Chain of Vapor should use target last-known information");

    assert_eq!(
        game.object(borrowed_id)
            .expect("borrowed relic exists")
            .zone,
        Zone::Hand
    );
    assert_eq!(
        game.object(land_id).expect("sacrificed land exists").zone,
        Zone::Graveyard,
        "the bounced permanent's controller should make the enabling sacrifice"
    );
    let [copy_entry] = game.stack.as_slice() else {
        panic!(
            "accepted Chain of Vapor should leave one copy: {:#?}",
            game.stack
        );
    };
    assert_eq!(copy_entry.controller, bob);
    assert_eq!(copy_entry.targets, vec![Target::Object(next_target_id)]);
}

fn add_object_derived_player_test_library_cards(
    game: &mut GameState,
    owner: PlayerId,
    count: usize,
) {
    for index in 0..count {
        let card = CardBuilder::new(
            CardId::new(),
            format!("Object-Derived Player Library Card {index}"),
        )
        .card_types(vec![CardType::Instant])
        .build();
        game.create_object_from_card(&card, owner, Zone::Library);
    }
}

#[test]
pub(super) fn player_or_planeswalker_target_binds_fanout_to_planeswalkers_controller() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let walker = CardBuilder::new(CardId::new(), "Bob's Planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(5)
        .build();
    let walker_id = game.create_object_from_card(&walker, bob, Zone::Battlefield);
    let bob_creature = CardBuilder::new(CardId::new(), "Bob's Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let bob_creature_id = game.create_object_from_card(&bob_creature, bob, Zone::Battlefield);
    let charlie_creature = CardBuilder::new(CardId::new(), "Charlie's Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let charlie_creature_id =
        game.create_object_from_card(&charlie_creature, charlie, Zone::Battlefield);

    let source = game.new_object_id();
    let ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(walker_id)]);
    let player_filter = PlayerFilter::TargetPlayerOrControllerOfTarget;
    assert_eq!(
        crate::effects::helpers::resolve_player_filter(&game, &player_filter, &ctx)
            .expect("planeswalker controller should resolve"),
        bob
    );

    let mut controlled_creatures = crate::filter::ObjectFilter::creature();
    controlled_creatures.zone = Some(Zone::Battlefield);
    controlled_creatures.controller = Some(player_filter);
    let resolved = crate::effects::helpers::resolve_objects_from_spec(
        &game,
        &crate::target::ChooseSpec::All(controlled_creatures),
        &ctx,
    )
    .expect("controlled-creature fanout should resolve");
    assert_eq!(resolved, vec![bob_creature_id]);
    assert!(!resolved.contains(&charlie_creature_id));
}

#[test]
pub(super) fn selected_graveyard_cards_exact_owner_is_the_player_milled() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    add_object_derived_player_test_library_cards(&mut game, bob, 2);
    add_object_derived_player_test_library_cards(&mut game, charlie, 2);

    let selected = CardBuilder::new(CardId::new(), "Charlie's Graveyard Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let selected_id = game.create_object_from_card(&selected, charlie, Zone::Graveyard);
    let selected_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(selected_id).expect("selected card exists"),
        &game,
    );
    let source = game.new_object_id();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    ctx.tag_object("moved_0", selected_snapshot);
    game.move_object_by_effect(selected_id, Zone::Battlefield)
        .expect("selected card should move to the battlefield");

    let mill = Effect::mill_player(
        1,
        PlayerFilter::AliasedOwnerOf(ObjectRef::tagged("moved_0")),
    );
    crate::effects::execute_effect(&mut game, &mill, &mut ctx)
        .expect("the selected card's exact owner should mill");

    assert_eq!(
        game.player(charlie).expect("charlie exists").library.len(),
        1
    );
    assert_eq!(game.player(bob).expect("bob exists").library.len(), 2);
}

#[test]
pub(super) fn goaded_creatures_exact_controller_is_the_player_who_draws() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    add_object_derived_player_test_library_cards(&mut game, bob, 1);
    add_object_derived_player_test_library_cards(&mut game, charlie, 1);

    let borrowed = CardBuilder::new(CardId::new(), "Charlie's Borrowed Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let borrowed_id = game.create_object_from_card(&borrowed, charlie, Zone::Battlefield);
    game.set_current_controller(borrowed_id, bob);
    let borrowed_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(borrowed_id).expect("borrowed creature exists"),
        &game,
    );
    let source = game.new_object_id();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    ctx.tag_object("goaded_0", borrowed_snapshot);

    let draw = Effect::target_draws(
        1,
        PlayerFilter::AliasedControllerOf(ObjectRef::tagged("goaded_0")),
    );
    crate::effects::execute_effect(&mut game, &draw, &mut ctx)
        .expect("the goaded creature's exact controller should draw");

    assert_eq!(game.player(bob).expect("bob exists").hand.len(), 1);
    assert_eq!(game.player(charlie).expect("charlie exists").hand.len(), 0);
}

#[test]
pub(super) fn destroyed_lands_exact_controller_chooses_the_new_aura_attachment() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let destroyed_land = CardBuilder::new(CardId::new(), "Charlie's Borrowed Land")
        .card_types(vec![CardType::Land])
        .build();
    let destroyed_land_id =
        game.create_object_from_card(&destroyed_land, charlie, Zone::Battlefield);
    game.set_current_controller(destroyed_land_id, bob);
    let destroyed_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(destroyed_land_id)
            .expect("destroyed land exists"),
        &game,
    );

    for (name, owner) in [("Alice's Land", alice), ("Charlie's Land", charlie)] {
        let land = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&land, owner, Zone::Battlefield);
    }
    let aura = CardDefinitionBuilder::new(CardId::new(), "Steam Vines Runtime Aura")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .enchants(crate::filter::ObjectFilter::land())
        .build();
    let aura_id = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
    let mut ctx = crate::effects::ExecutionContext::new_default(aura_id, alice);
    ctx.tag_object("destroyed_land_0", destroyed_snapshot);
    game.move_object_by_effect(destroyed_land_id, Zone::Graveyard)
        .expect("enchanted land should be destroyed");

    let chooser = PlayerFilter::AliasedControllerOf(ObjectRef::tagged("destroyed_land_0"));
    assert_eq!(
        crate::effects::helpers::resolve_player_filter(&game, &chooser, &ctx)
            .expect("destroyed land controller should resolve from LKI"),
        bob
    );
    let mut lands = crate::filter::ObjectFilter::land();
    lands.zone = Some(Zone::Battlefield);
    let choose = Effect::choose_objects(lands, 1, chooser, "attachment_target_0");
    crate::effects::execute_effect(&mut game, &choose, &mut ctx)
        .expect("destroyed land's controller should choose a land");
    let chosen_id = ctx
        .get_tagged("attachment_target_0")
        .expect("chosen land should be tagged")
        .object_id;
    let attach = Effect::attach_objects(
        crate::target::ChooseSpec::Source,
        crate::target::ChooseSpec::Tagged(crate::tag::TagKey::from("attachment_target_0")),
    );
    crate::effects::execute_effect(&mut game, &attach, &mut ctx)
        .expect("Aura should attach to the chosen land");

    assert_eq!(
        game.object(aura_id).expect("Aura exists").attached_to,
        Some(crate::object::AttachmentTarget::Object(chosen_id))
    );
}

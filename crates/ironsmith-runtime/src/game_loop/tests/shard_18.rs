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

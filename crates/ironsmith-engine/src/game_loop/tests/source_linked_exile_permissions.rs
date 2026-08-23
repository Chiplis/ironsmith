use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
fn intrepid_paleontologist_permission_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(98_001), "Intrepid Paleontologist")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "You may cast Dinosaur creature spells from among cards you own exiled with this creature. If you cast a spell this way, that creature enters with a finality counter on it.",
        )
        .expect("Intrepid Paleontologist's source-linked permission should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
fn test_dinosaur(name: &str) -> crate::card::Card {
    CardBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dinosaur])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn intrepid_paleontologist_only_casts_owned_linked_dinosaurs_and_adds_finality() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 8);

    let paleontologist = intrepid_paleontologist_permission_definition();
    let source_id = game.create_object_from_definition(&paleontologist, alice, Zone::Battlefield);

    let linked_owned =
        game.create_object_from_card(&test_dinosaur("Linked Alice Dinosaur"), alice, Zone::Exile);
    game.add_exiled_with_source_link(source_id, linked_owned);

    let unlinked_owned = game.create_object_from_card(
        &test_dinosaur("Unlinked Alice Dinosaur"),
        alice,
        Zone::Exile,
    );

    let linked_opponent =
        game.create_object_from_card(&test_dinosaur("Linked Bob Dinosaur"), bob, Zone::Exile);
    game.add_exiled_with_source_link(source_id, linked_opponent);

    let linked_nondinosaur = CardBuilder::new(CardId::new(), "Linked Bear")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let linked_nondinosaur = game.create_object_from_card(&linked_nondinosaur, alice, Zone::Exile);
    game.add_exiled_with_source_link(source_id, linked_nondinosaur);

    let actions = compute_legal_actions(&game, alice);
    let can_cast_from_exile = |candidate| {
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::PlayFrom { .. },
                } if *spell_id == candidate
            )
        })
    };
    assert!(can_cast_from_exile(linked_owned));
    assert!(!can_cast_from_exile(unlinked_owned));
    assert!(!can_cast_from_exile(linked_opponent));
    assert!(!can_cast_from_exile(linked_nondinosaur));

    let cast_action = actions
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::PlayFrom { .. },
                } if *spell_id == linked_owned
            )
        })
        .expect("the owned linked Dinosaur should be castable from exile");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
        &mut dm,
    )
    .expect("the source-linked Dinosaur cast should complete");
    resolve_stack_entry(&mut game).expect("the linked Dinosaur should resolve");

    let entered = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Linked Alice Dinosaur")
        })
        .expect("the linked Dinosaur should enter the battlefield");
    assert_eq!(
        game.counter_count(entered, crate::object::CounterType::Finality),
        1,
        "a Dinosaur cast through the permission should enter with finality"
    );

    crate::events::processing::process_destroy(&mut game, entered, None, &mut dm);
    assert!(game.exile.iter().any(|id| {
        game.object(*id)
            .is_some_and(|object| object.name == "Linked Alice Dinosaur")
    }));
}

#[cfg(ironsmith_runtime_parser_tests)]
fn scorched_ruins_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(98_002), "Scorched Ruins")
        .card_types(vec![CardType::Land])
        .parse_text(
            "If this land would enter, sacrifice two untapped lands instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.\n{T}: Add {C}{C}{C}{C}.",
        )
        .expect("Scorched Ruins should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
fn test_land(name: &str) -> crate::card::Card {
    CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
struct SacrificeLandsDecisionMaker {
    accept: bool,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for SacrificeLandsDecisionMaker {
    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if self.accept {
            ctx.candidates
                .iter()
                .take(ctx.max.unwrap_or(ctx.candidates.len()))
                .map(|candidate| candidate.id)
                .collect()
        } else {
            Vec::new()
        }
    }

    fn decide_priority(
        &mut self,
        _game: &crate::GameState,
        _ctx: &crate::decisions::context::PriorityContext,
    ) -> crate::decision::LegalAction {
        crate::decision::LegalAction::PassPriority
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
fn put_scorched_ruins_on_stack(game: &mut crate::GameState, owner: PlayerId) {
    let ruins = scorched_ruins_definition();
    let ruins_id = game.create_object_from_definition(&ruins, owner, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(ruins_id, owner));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn scorched_ruins_sacrifices_exactly_two_controlled_untapped_lands_to_enter() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let first =
        game.create_object_from_card(&test_land("Alice Land One"), alice, Zone::Battlefield);
    let second =
        game.create_object_from_card(&test_land("Alice Land Two"), alice, Zone::Battlefield);
    let tapped =
        game.create_object_from_card(&test_land("Alice Tapped Land"), alice, Zone::Battlefield);
    game.tap(tapped);
    let opponent = game.create_object_from_card(&test_land("Bob Land"), bob, Zone::Battlefield);
    put_scorched_ruins_on_stack(&mut game, alice);

    let mut dm = SacrificeLandsDecisionMaker { accept: true };
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Scorched Ruins should resolve");

    assert!(!game.battlefield.contains(&first));
    assert!(!game.battlefield.contains(&second));
    assert!(game.battlefield.contains(&tapped));
    assert!(game.battlefield.contains(&opponent));
    assert!(game.battlefield.iter().any(|id| {
        game.object(*id)
            .is_some_and(|object| object.name == "Scorched Ruins")
    }));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn scorched_ruins_redirects_without_partially_sacrificing_when_payment_fails_or_is_declined() {
    let alice = PlayerId::from_index(0);

    for (available, accept) in [(1usize, true), (2usize, false)] {
        let mut game = setup_game();
        let lands = (0..available)
            .map(|index| {
                game.create_object_from_card(
                    &test_land(&format!("Payment Land {index}")),
                    alice,
                    Zone::Battlefield,
                )
            })
            .collect::<Vec<_>>();
        put_scorched_ruins_on_stack(&mut game, alice);

        let mut dm = SacrificeLandsDecisionMaker { accept };
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm)
            .expect("Scorched Ruins should resolve to its redirect");

        assert!(lands.iter().all(|land| game.battlefield.contains(land)));
        assert!(!game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Scorched Ruins")
        }));
        assert!(
            game.player(alice)
                .expect("Alice exists")
                .graveyard
                .iter()
                .any(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.name == "Scorched Ruins")
                })
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn scorched_ruins_static_replacement_renders_canonically() {
    let definition = scorched_ruins_definition();
    let display = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::SacrificeOrRedirectReplacement =>
            {
                Some(static_ability.display())
            }
            _ => None,
        })
        .expect("Scorched Ruins should have a sacrifice replacement");
    assert_eq!(
        display,
        "If this land would enter, sacrifice two untapped lands instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard."
    );
}

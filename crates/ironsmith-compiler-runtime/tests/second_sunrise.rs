use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{BattlefieldController, ReturnAllToBattlefieldEffect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, extract_target_requirements_from_program_with_modes,
    resolve_stack_entry,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::PlayerFilter;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::CardRegistryArtifactExt;

// Exact Oracle text and printed header from the frozen cards.json record:
// oracle_id 15cef63b-903e-4f1b-a128-e59157e07c36.
const SECOND_SUNRISE: &str = "Mana cost: {1}{W}{W}\nType: Instant\nEach player returns to the battlefield all artifact, creature, enchantment, and land cards in their graveyard that were put there from the battlefield this turn.";
const RETURN_TYPES: [CardType; 4] = [
    CardType::Artifact,
    CardType::Creature,
    CardType::Enchantment,
    CardType::Land,
];

fn definitions() -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition("Second Sunrise", SECOND_SUNRISE, false)
        .expect("the exact frozen card must strict-compile");
    let (artifact, _) = compile_to_artifact("Second Sunrise", SECOND_SUNRISE, false)
        .expect("the exact frozen card must compile to a typed artifact");
    let bytes = serde_json::to_vec(&artifact).unwrap();
    let restored: CompiledCardArtifact = serde_json::from_slice(&bytes).unwrap();
    restored.validate().unwrap();
    assert_eq!(
        restored, artifact,
        "typed payload survives transport intact"
    );
    let mut registry = ironsmith::cards::CardRegistry::new();
    registry.register_compiled_artifact(&restored).unwrap();
    [direct, registry.get("Second Sunrise").unwrap().clone()]
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = PlayerId::from_index(1);
    game.turn.priority_player = Some(PlayerId::from_index(0));
    game.player_mut(PlayerId::from_index(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::White, 3);
    game
}

fn card(game: &mut GameState, owner: PlayerId, kind: CardType, zone: Zone) -> ObjectId {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), format!("{kind:?} fixture"))
        .card_types(vec![kind]);
    if kind == CardType::Creature {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    let definition = builder.build();
    game.create_object_from_definition(&definition, owner, zone)
}

fn cast(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let alice = PlayerId::from_index(0);
    let hand = game.create_object_from_definition(definition, alice, Zone::Hand);
    let stable = game.object(hand).unwrap().stable_id;
    let action = LegalAction::CastSpell {
        spell_id: hand,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    };
    assert!(
        compute_legal_actions(game, alice)
            .unwrap()
            .contains(&action)
    );
    let mut state = PriorityLoopState::new(3);
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..30 {
        if state.pending_cast.is_none() && !game.stack_is_empty() {
            break;
        }
        if let GameProgress::NeedsDecisionCtx(ctx) = progress {
            progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm)
                .unwrap();
        } else {
            break;
        }
    }
    assert!(state.pending_cast.is_none(), "the real cast must finish");
    assert_eq!(game.stack.len(), 1);
    let stack = game.stack[0].object_id;
    assert_ne!(stack, hand, "casting creates a new source incarnation");
    assert_eq!(game.object(stack).unwrap().stable_id, stable);
    assert!(
        game.stack[0].targets.is_empty(),
        "Second Sunrise has no targets"
    );
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    stack
}

fn resolve(game: &mut GameState, stack: ObjectId) {
    let stable = game.object(stack).unwrap().stable_id;
    resolve_stack_entry(game).expect("Second Sunrise resolves through the real stack");
    assert!(game.stack_is_empty());
    let graveyard = game.find_object_by_stable_id(stable).unwrap();
    assert_ne!(
        graveyard, stack,
        "resolution creates another source incarnation"
    );
    assert_eq!(game.object(graveyard).unwrap().zone, Zone::Graveyard);
    assert!(!game.battlefield.contains(&graveyard));
}

fn find_return(effect: &ironsmith::effect::Effect) -> Option<ReturnAllToBattlefieldEffect> {
    if let Some(returned) = effect.downcast_ref::<ReturnAllToBattlefieldEffect>() {
        return Some(returned.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_return(child);
        }
    });
    found
}

#[test]
fn second_sunrise_strict_artifact_preserves_each_player_type_union_and_history() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let program = definition.spell_effect.as_ref().unwrap();
        let returned = program
            .flattened_default_effects()
            .iter()
            .find_map(find_return)
            .expect("the complete return remains executable");
        assert_eq!(returned.filter.card_types, RETURN_TYPES);
        assert_eq!(returned.filter.zone, Some(Zone::Graveyard));
        assert_eq!(returned.filter.owner, Some(PlayerFilter::IteratedPlayer));
        assert!(returned.filter.entered_graveyard_from_battlefield_this_turn);
        assert_eq!(
            returned.battlefield_controller,
            BattlefieldController::Owner
        );
        assert!(!returned.tapped);
        let mut game = game();
        let source =
            game.create_object_from_definition(&definition, PlayerId::from_index(0), Zone::Hand);
        assert!(
            extract_target_requirements_from_program_with_modes(
                &game,
                program,
                PlayerId::from_index(0),
                Some(source),
                None
            )
            .is_empty()
        );
    }
}

#[test]
fn second_sunrise_cast_returns_all_four_types_to_each_owner_from_this_turn_only() {
    for definition in definitions() {
        let mut game = game();
        let mut excluded = Vec::new();
        // Record a real earlier death, then advance its history beyond this turn.
        let old = card(
            &mut game,
            PlayerId::from_index(0),
            CardType::Creature,
            Zone::Battlefield,
        );
        let old = game.move_object_by_effect(old, Zone::Graveyard).unwrap();
        excluded.push(old);
        game.take_pending_trigger_events();
        game.turn_store.turn_history.clear_for_new_turn();
        game.turn.turn_number += 1;

        let mut qualifying = Vec::new();
        for index in 0..3 {
            let owner = PlayerId::from_index(index);
            let previous_controller = PlayerId::from_index((index + 1) % 3);
            for kind in RETURN_TYPES {
                let permanent = card(&mut game, owner, kind, Zone::Battlefield);
                game.set_current_controller(permanent, previous_controller)
                    .unwrap();
                let graveyard = game
                    .move_object_by_effect(permanent, Zone::Graveyard)
                    .unwrap();
                assert_ne!(permanent, graveyard);
                assert!(game.player(owner).unwrap().graveyard.contains(&graveyard));
                qualifying.push((
                    graveyard,
                    game.object(graveyard).unwrap().stable_id,
                    owner,
                    kind,
                ));
                for origin in [Zone::Hand, Zone::Library, Zone::Exile, Zone::Stack] {
                    let other = card(&mut game, owner, kind, origin);
                    excluded.push(game.move_object_by_effect(other, Zone::Graveyard).unwrap());
                }
                excluded.push(card(&mut game, owner, kind, Zone::Graveyard));
            }
            // The list is narrower than all permanent types.
            for kind in [CardType::Planeswalker, CardType::Battle] {
                let other = card(&mut game, owner, kind, Zone::Battlefield);
                excluded.push(game.move_object_by_effect(other, Zone::Graveyard).unwrap());
            }
        }
        let stack = cast(&mut game, &definition);
        assert!(
            qualifying
                .iter()
                .all(|(id, _, _, _)| game.object(*id).unwrap().zone == Zone::Graveyard)
        );
        resolve(&mut game, stack);
        assert_eq!(game.battlefield.len(), qualifying.len());
        for (graveyard, stable, owner, kind) in qualifying {
            let returned = game.find_object_by_stable_id(stable).unwrap();
            assert_ne!(returned, graveyard);
            let object = game.object(returned).unwrap();
            assert_eq!(object.zone, Zone::Battlefield, "{kind:?}, owner {owner:?}");
            assert_eq!(object.owner, owner);
            assert_eq!(game.current_controller(returned), Some(owner));
            assert!(!game.is_tapped(returned));
        }
        for id in excluded {
            assert_eq!(
                game.object(id).unwrap().zone,
                Zone::Graveyard,
                "excluded {id:?}"
            );
        }
    }
}

#[test]
fn second_sunrise_uses_current_graveyard_incarnation_at_resolution() {
    for definition in definitions() {
        for detour in [Zone::Hand, Zone::Library, Zone::Exile] {
            let mut game = game();
            let alice = PlayerId::from_index(0);
            let departed = card(&mut game, alice, CardType::Creature, Zone::Battlefield);
            let departed = game
                .move_object_by_effect(departed, Zone::Graveyard)
                .unwrap();
            let still_eligible = card(&mut game, alice, CardType::Artifact, Zone::Battlefield);
            let still_eligible = game
                .move_object_by_effect(still_eligible, Zone::Graveyard)
                .unwrap();
            let stable_eligible = game.object(still_eligible).unwrap().stable_id;
            let stack = cast(&mut game, &definition);
            // Eligibility is read as the spell resolves, after a response moved
            // this card out and back. CR 400.7: this graveyard object never died.
            let away = game.move_object_by_effect(departed, detour).unwrap();
            let returned_to_graveyard = game.move_object_by_effect(away, Zone::Graveyard).unwrap();
            assert_ne!(departed, returned_to_graveyard);
            let late = card(&mut game, alice, CardType::Land, Zone::Battlefield);
            let late = game.move_object_by_effect(late, Zone::Graveyard).unwrap();
            let stable_late = game.object(late).unwrap().stable_id;
            resolve(&mut game, stack);
            assert_eq!(
                game.object(returned_to_graveyard).unwrap().zone,
                Zone::Graveyard,
                "an earlier incarnation's death cannot qualify a later {detour:?} arrival"
            );
            for stable in [stable_eligible, stable_late] {
                let returned = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
            }
        }
    }
}

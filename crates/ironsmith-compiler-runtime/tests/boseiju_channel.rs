use ironsmith::ability::AbilityKind;
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, TargetsContext};
use ironsmith::events::EventKind;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::resolution::ResolutionProgram;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    Ability, CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Supertype,
    Target, Zone,
};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::CardRegistryArtifactExt;

// Exact frozen cards.json text, oracle_id bf1341dd-41a3-49f6-87ec-63170dde4324.
const BOSEIJU: &str = "Type: Legendary Land\n{T}: Add {G}.\nChannel — {1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls. That player may search their library for a land card with a basic land type, put it onto the battlefield, then shuffle. This ability costs {1} less to activate for each legendary creature you control.";

fn definitions() -> [CardDefinition; 3] {
    let direct = compile_to_runtime_definition("Boseiju, Who Endures", BOSEIJU, false).unwrap();
    let (artifact, _) = compile_to_artifact("Boseiju, Who Endures", BOSEIJU, false).unwrap();
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, artifact);
    let mut registry = ironsmith::cards::CardRegistry::new();
    registry.register_compiled_artifact(&restored).unwrap();
    let mut single_segment = direct.clone();
    let channel = single_segment
        .abilities
        .iter_mut()
        .find_map(|ability| {
            if !ability.functions_in(&Zone::Hand) {
                return None;
            }
            match &mut ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            }
        })
        .unwrap();
    assert_eq!(
        channel.effects.segments.len(),
        2,
        "preserve the exact changed segment shape"
    );
    assert!(
        channel
            .effects
            .segments
            .iter()
            .all(|segment| segment.self_replacements.is_empty())
    );
    // Reconstruct the old one-segment shape as a behavior control. Both paths
    // must obey the same rules, including when destruction does nothing.
    channel.effects =
        ResolutionProgram::from_effects(channel.effects.flattened_default_effects().to_vec());
    [
        direct,
        registry.get("Boseiju, Who Endures").unwrap().clone(),
        single_segment,
    ]
}

struct ChannelDecisions {
    target: ObjectId,
    search_player: PlayerId,
    search_card: ObjectId,
    excluded_search_card: ObjectId,
    accept_search: bool,
    may_players: Vec<PlayerId>,
    searched: bool,
}

impl DecisionMaker for ChannelDecisions {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert_eq!(ctx.requirements.len(), 1);
        assert!(
            ctx.requirements[0]
                .legal_targets
                .contains(&Target::Object(self.target))
        );
        vec![Target::Object(self.target)]
    }

    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        self.may_players.push(ctx.player);
        assert_eq!(
            ctx.player, self.search_player,
            "target controller chooses, not the caster or owner"
        );
        self.accept_search
    }

    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if ctx.candidates.iter().any(|candidate| {
            game.object(candidate.id)
                .is_some_and(|object| object.zone == Zone::Library)
        }) {
            assert_eq!(ctx.player, self.search_player);
            assert!(
                ctx.candidates
                    .iter()
                    .any(|candidate| candidate.id == self.search_card && candidate.legal),
                "a nonbasic land with a basic land type is a valid search result"
            );
            assert!(
                !ctx.candidates
                    .iter()
                    .any(|candidate| candidate.id == self.excluded_search_card && candidate.legal),
                "a land without a basic land type is not a valid search result"
            );
            self.searched = true;
            vec![self.search_card]
        } else {
            SelectFirstDecisionMaker.decide_objects(game, ctx)
        }
    }
}

fn activate(game: &mut GameState, source: ObjectId, dm: &mut ChannelDecisions) {
    let alice = PlayerId::from_index(0);
    let action = compute_legal_actions(game, alice).unwrap().into_iter().find(|action| {
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)
    }).expect("the hand-only Channel activation must be legal");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..30 {
        if state.pending_activation.is_none() && !game.stack_is_empty() {
            break;
        }
        if let GameProgress::NeedsDecisionCtx(ctx) = progress {
            progress =
                apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
        } else {
            break;
        }
    }
    assert!(
        state.pending_activation.is_none(),
        "printed activation and discard must finish"
    );
    assert_eq!(game.stack.len(), 1);
    assert!(game.stack[0].is_ability);
    assert!(
        game.object(source).is_none(),
        "discarding the source changes its object identity"
    );
}

#[derive(Clone, Copy, Debug)]
enum TargetFate {
    Destroyed,
    Indestructible,
    LeavesBeforeResolution,
}

#[test]
fn boseiju_channel_segment_shapes_preserve_destroy_and_target_controller_search() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    for definition in definitions() {
        for kind in [CardType::Artifact, CardType::Enchantment, CardType::Land] {
            for fate in [
                TargetFate::Destroyed,
                TargetFate::Indestructible,
                TargetFate::LeavesBeforeResolution,
            ] {
                for accept_search in [false, true] {
                    let mut game =
                        GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
                    game.turn.phase = Phase::FirstMain;
                    game.turn.step = None;
                    game.turn.active_player = bob;
                    game.turn.priority_player = Some(alice);
                    game.player_mut(alice)
                        .unwrap()
                        .mana_pool
                        .add(ManaSymbol::Green, 2);
                    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
                    let source_stable = game.object(source).unwrap().stable_id;
                    let mut target = CardDefinitionBuilder::new(CardId::new(), "Channel target")
                        .card_types(vec![kind]);
                    if matches!(fate, TargetFate::Indestructible) {
                        target = target
                            .with_ability(Ability::static_ability(StaticAbility::indestructible()));
                    }
                    // Charlie owns it, Bob controls it, and Alice activates.
                    let target = game.create_object_from_definition(
                        &target.build(),
                        charlie,
                        Zone::Battlefield,
                    );
                    let target_stable = game.object(target).unwrap().stable_id;
                    game.set_current_controller(target, bob).unwrap();
                    let typed_land =
                        CardDefinitionBuilder::new(CardId::new(), "Nonbasic typed land")
                            .card_types(vec![CardType::Land])
                            .subtypes(vec![Subtype::Forest, Subtype::Island])
                            .build();
                    let untyped_land = CardDefinitionBuilder::new(CardId::new(), "Untyped land")
                        .card_types(vec![CardType::Land])
                        .build();
                    let mut library_cards = Vec::new();
                    for player in [alice, bob, charlie] {
                        let typed =
                            game.create_object_from_definition(&typed_land, player, Zone::Library);
                        let untyped = game.create_object_from_definition(
                            &untyped_land,
                            player,
                            Zone::Library,
                        );
                        library_cards.push((typed, untyped));
                    }
                    let (search_card, excluded_search_card) = library_cards[1];
                    let search_stable = game.object(search_card).unwrap().stable_id;
                    let mut dm = ChannelDecisions {
                        target,
                        search_player: bob,
                        search_card,
                        excluded_search_card,
                        accept_search,
                        may_players: vec![],
                        searched: false,
                    };
                    activate(&mut game, source, &mut dm);
                    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
                    let discarded_source = game.find_object_by_stable_id(source_stable).unwrap();
                    assert_eq!(game.object(discarded_source).unwrap().zone, Zone::Graveyard);
                    if matches!(fate, TargetFate::LeavesBeforeResolution) {
                        game.move_object_by_effect(target, Zone::Exile).unwrap();
                    }
                    resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    assert!(game.stack_is_empty());
                    let target_now = game.find_object_by_stable_id(target_stable).unwrap();
                    let target_zone = match fate {
                        TargetFate::Destroyed => Zone::Graveyard,
                        TargetFate::Indestructible => Zone::Battlefield,
                        TargetFate::LeavesBeforeResolution => Zone::Exile,
                    };
                    assert_eq!(
                        game.object(target_now).unwrap().zone,
                        target_zone,
                        "{kind:?}, {fate:?}"
                    );
                    let target_legal = !matches!(fate, TargetFate::LeavesBeforeResolution);
                    assert_eq!(
                        dm.may_players,
                        if target_legal { vec![bob] } else { vec![] }
                    );
                    let searched = target_legal && accept_search;
                    assert_eq!(dm.searched, searched, "{kind:?}, {fate:?}");
                    let found = game.find_object_by_stable_id(search_stable).unwrap();
                    assert_eq!(
                        game.object(found).unwrap().zone,
                        if searched {
                            Zone::Battlefield
                        } else {
                            Zone::Library
                        }
                    );
                    if searched {
                        assert_ne!(found, search_card);
                        assert_eq!(game.current_controller(found), Some(bob));
                        assert!(!game.is_tapped(found));
                    }
                    assert_eq!(
                        game.turn_store
                            .turn_history
                            .event_kind_count(EventKind::ShuffleLibrary),
                        u32::from(searched)
                    );
                    assert_eq!(game.player(alice).unwrap().library.len(), 2);
                    assert_eq!(game.player(charlie).unwrap().library.len(), 2);
                    assert_eq!(
                        game.player(bob).unwrap().library.len(),
                        if searched { 1 } else { 2 }
                    );
                }
            }
        }
    }
}

#[test]
fn boseiju_channel_legendary_creature_reduction_preserves_green_mana_minimum() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions() {
        for legends in 0..=2 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Green, 2);
            let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
            for index in 0..legends {
                let legend = CardDefinitionBuilder::new(CardId::new(), format!("Legend {index}"))
                    .card_types(vec![CardType::Creature])
                    .supertypes(vec![Supertype::Legendary])
                    .power_toughness(PowerToughness::fixed(2, 2))
                    .build();
                game.create_object_from_definition(&legend, alice, Zone::Battlefield);
            }
            let target = CardDefinitionBuilder::new(CardId::new(), "Artifact")
                .card_types(vec![CardType::Artifact])
                .build();
            let target = game.create_object_from_definition(&target, bob, Zone::Battlefield);
            let mut dm = ChannelDecisions {
                target,
                search_player: bob,
                search_card: target,
                excluded_search_card: source,
                accept_search: false,
                may_players: vec![],
                searched: false,
            };
            activate(&mut game, source, &mut dm);
            assert_eq!(
                game.player(alice).unwrap().mana_pool.total(),
                u32::from(legends > 0),
                "generic mana is reduced, but the printed green mana still must be paid"
            );
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(dm.may_players, vec![bob]);
        }
    }
}

//! Current and past triggering-object frames. Authored, unrun source proposals.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith::effects::{EffectContext, EffectExecutor, PutCountersEffect};
use ironsmith::events::EventCause;
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::object::CounterType;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/triggered_object_predicates.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let fixture = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let (artifact, direct) = compile_to_artifact(name, fixture["text"].as_str().unwrap(), false)
        .unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(A);
    game
}
fn object(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn library(game: &mut GameState, owner: PlayerId) {
    for _ in 0..6 {
        object(game, owner, Zone::Library, "Draw resource", "Type: Land");
    }
}
struct Decisions {
    chosen: Option<ObjectId>,
}
impl DecisionMaker for Decisions {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        true
    }
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.chosen
            && ctx
                .candidates
                .iter()
                .any(|candidate| candidate.id == id && candidate.legal)
        {
            return vec![id];
        }
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .take(ctx.min)
            .map(|candidate| candidate.id)
            .collect()
    }
}
fn pending(game: &mut GameState, dm: &mut impl DecisionMaker) -> usize {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState, dm: &mut impl DecisionMaker) {
    pending(game, dm);
    for _ in 0..24 {
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry_with(game, dm).unwrap();
        pending(game, dm);
    }
    panic!("characteristic trigger did not settle");
}
fn apply(
    game: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    effect: &dyn EffectExecutor,
    dm: &mut impl DecisionMaker,
) {
    let mut context = EffectContext::new(source, player, dm);
    let outcome = effect.execute(game, &mut context).unwrap();
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}
fn kill(
    game: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    target: ObjectId,
    dm: &mut impl DecisionMaker,
) {
    apply(
        game,
        source,
        player,
        &ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(target)),
        dm,
    );
}
fn enter(game: &mut GameState, id: ObjectId) -> ObjectId {
    game.move_object(id, Zone::Battlefield, EventCause::effect())
        .unwrap()
}
fn cast_creature_from_hand(game: &mut GameState, hand: ObjectId) -> ObjectId {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let stable = game.object(hand).unwrap().stable_id;
    let action = ironsmith::decision::LegalAction::CastSpell {
        spell_id: hand,
        from_zone: Zone::Hand,
        casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
    };
    let mut state = PriorityLoopState::new(game.players.len());
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
    for _ in 0..24 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("creature cast pending without choice")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm)
            .unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    resolve_stack_entry_with(game, &mut dm).unwrap();
    game.find_object_by_stable_id(stable).unwrap()
}
fn tokens_named(game: &GameState, name: &str) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.name == name && object.kind == ironsmith::object::ObjectKind::Token
            })
        })
        .collect()
}
#[test]
fn complete_exact_fixtures_retain_every_body_in_direct_and_restored_artifacts() {
    assert_eq!(fixtures().len(), 7);
    assert_eq!(
        fixtures()
            .iter()
            .filter(|row| row["proposed_complete"] == true)
            .count(),
        7
    );
    for row in fixtures()
        .into_iter()
        .filter(|row| row["proposed_complete"] == true)
    {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert!(!definition.abilities.is_empty());
        }
    }
}
#[test]
fn sigil_captain_checks_both_stats_and_rechecks_the_current_exact_creature() {
    for definition in definitions("Sigil Captain") {
        for (stats, grows, expected) in [
            ("1/1", false, 2),
            ("1/1", true, 1),
            ("1/2", false, 0),
            ("2/1", false, 0),
        ] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let hand = object(
                &mut game,
                A,
                Zone::Hand,
                "Sigil entrant",
                &format!("Mana cost: {{0}}\nType: Creature\nPower/Toughness: {stats}"),
            );
            let entrant = cast_creature_from_hand(&mut game, hand);
            let mut dm = SelectFirstDecisionMaker;
            assert_eq!(pending(&mut game, &mut dm), usize::from(stats == "1/1"));
            if grows {
                apply(
                    &mut game,
                    source,
                    A,
                    &PutCountersEffect::new(
                        CounterType::PlusOnePlusOne,
                        1,
                        ChooseSpec::SpecificObject(entrant),
                    ),
                    &mut dm,
                );
            }
            settle(&mut game, &mut dm);
            assert_eq!(
                game.counter_count(entrant, CounterType::PlusOnePlusOne),
                expected,
                "{stats}, grows={grows}"
            );
        }
    }
}
#[test]
fn slasher_reads_absent_counters_before_death_and_retains_combat_half_life_body() {
    for definition in definitions("Unstoppable Slasher") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        let actor = object(&mut game, A, Zone::Battlefield, "Actor", "Type: Artifact");
        let mut dm = SelectFirstDecisionMaker;
        apply(
            &mut game,
            source,
            A,
            &ironsmith::effects::DealDamageEffect::new(
                3,
                ChooseSpec::Player(PlayerFilter::Specific(B)),
            )
            .with_combat(true),
            &mut dm,
        );
        settle(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().life, 8);
        kill(&mut game, actor, A, source, &mut dm);
        settle(&mut game, &mut dm);
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(returned, source);
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        assert!(game.is_tapped(returned));
        assert_eq!(game.counter_count(returned, CounterType::Stun), 2);
        kill(&mut game, actor, A, returned, &mut dm);
        assert_eq!(
            pending(&mut game, &mut dm),
            0,
            "stun counters existed before the second death"
        );
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Graveyard
        );
    }
}
#[test]
fn wilhelt_rejects_decayed_death_snapshot_and_keeps_end_step_sacrifice_draw() {
    for definition in definitions("Wilhelt, the Rotcleaver") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let zombie = object(
            &mut game,
            A,
            Zone::Battlefield,
            "Undecayed Zombie",
            "Type: Creature — Zombie\nPower/Toughness: 2/2",
        );
        library(&mut game, A);
        let mut dm = Decisions { chosen: None };
        kill(&mut game, source, A, zombie, &mut dm);
        settle(&mut game, &mut dm);
        let token = tokens_named(&game, "Zombie")[0];
        kill(&mut game, source, A, token, &mut dm);
        assert_eq!(
            pending(&mut game, &mut dm),
            0,
            "decayed in the death snapshot forbids replacement token"
        );
        let zombie = object(
            &mut game,
            A,
            Zone::Battlefield,
            "Another Zombie",
            "Type: Creature — Zombie\nPower/Toughness: 2/2",
        );
        dm.chosen = Some(zombie);
        game.queue_trigger_event(
            Default::default(),
            TriggerEvent::new_with_provenance(
                ironsmith::events::BeginningOfEndStepEvent::new(A),
                Default::default(),
            ),
        );
        settle(&mut game, &mut dm);
        assert!(game.object(zombie).is_none());
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(tokens_named(&game, "Zombie").len(), 1);
    }
}
#[test]
fn jason_draws_for_both_growth_and_shrinkage_without_using_the_new_graveyard_stats() {
    for definition in definitions("Jason Bright, Glowing Prophet") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        library(&mut game, A);
        let mut dm = SelectFirstDecisionMaker;
        for (kind, expected) in [
            (Some(CounterType::PlusOnePlusOne), 1),
            (None, 1),
            (Some(CounterType::MinusOneMinusOne), 2),
        ] {
            let zombie = object(
                &mut game,
                A,
                Zone::Battlefield,
                "Jason's subject",
                "Type: Creature — Zombie\nPower/Toughness: 3/3",
            );
            if let Some(kind) = kind {
                apply(
                    &mut game,
                    source,
                    A,
                    &PutCountersEffect::new(kind, 1, ChooseSpec::SpecificObject(zombie)),
                    &mut dm,
                );
            }
            kill(&mut game, source, A, zombie, &mut dm);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), expected);
        }
    }
}
#[test]
fn dawn_evangel_keeps_aura_controller_at_death_even_after_the_aura_leaves() {
    for definition in definitions("Dawn Evangel") {
        for aura_controller in [A, B] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let host = object(
                &mut game,
                B,
                Zone::Battlefield,
                "Enchanted subject",
                "Type: Creature\nPower/Toughness: 3/3",
            );
            let aura_owner = if aura_controller == A { B } else { A };
            let aura = object(
                &mut game,
                aura_owner,
                Zone::Battlefield,
                "Aura",
                "Type: Enchantment — Aura\nEnchant creature",
            );
            let control_source = object(
                &mut game,
                aura_controller,
                Zone::Battlefield,
                "Control actor",
                "Type: Artifact",
            );
            apply(
                &mut game,
                control_source,
                aura_controller,
                &ironsmith::effects::GainControlEffect::new(
                    ChooseSpec::SpecificObject(aura),
                    ironsmith::effect::Until::Forever,
                ),
                &mut SelectFirstDecisionMaker,
            );
            assert_ne!(
                game.object(aura).unwrap().owner,
                game.current_controller(aura).unwrap()
            );
            assert!(
                game.attach_object_to_target(
                    aura,
                    ironsmith::object::AttachmentTarget::Object(host)
                )
            );
            let target = object(
                &mut game,
                A,
                Zone::Graveyard,
                "Small return",
                "Mana cost: {2}\nType: Creature\nPower/Toughness: 2/2",
            );
            let mut dm = Decisions {
                chosen: Some(target),
            };
            kill(&mut game, source, A, host, &mut dm);
            if game.object(aura).is_some() {
                game.move_object(aura, Zone::Graveyard, EventCause::effect())
                    .unwrap();
            }
            settle(&mut game, &mut dm);
            assert_eq!(
                game.player(A).unwrap().hand.len(),
                usize::from(aura_controller == A)
            );
        }
    }
}
#[test]
fn tom_bert_and_william_return_once_as_an_artifact_using_the_old_creature_frame() {
    for definition in definitions("Tom, Bert, and William") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        let actor = object(&mut game, A, Zone::Battlefield, "Actor", "Type: Artifact");
        let mut dm = SelectFirstDecisionMaker;
        kill(&mut game, actor, A, source, &mut dm);
        settle(&mut game, &mut dm);
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(returned, source);
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        assert!(game.object_has_card_type(returned, ironsmith::CardType::Artifact));
        assert!(!game.object_has_card_type(returned, ironsmith::CardType::Creature));
        kill(&mut game, actor, A, returned, &mut dm);
        assert_eq!(pending(&mut game, &mut dm), 0);
    }
}
#[test]
fn fyndhorn_passive_blocked_history_survives_combat_but_excludes_the_blocker_and_blink() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration};
    for definition in definitions("Fyndhorn Druid") {
        for mode in 0..3 {
            let mut game = game();
            let source_player = if mode == 1 { B } else { A };
            let mut source =
                game.create_object_from_definition(&definition, source_player, Zone::Battlefield);
            let other_player = if source_player == A { B } else { A };
            let other = object(
                &mut game,
                other_player,
                Zone::Battlefield,
                "Combat partner",
                "Type: Creature\nPower/Toughness: 2/2",
            );
            let (attacker, blocker) = if mode == 1 {
                (other, source)
            } else {
                (source, other)
            };
            game.remove_summoning_sickness(attacker);
            game.turn.phase = ironsmith::Phase::Combat;
            game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
            let mut combat = CombatState::default();
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::apply_attacker_declarations(
                &mut game,
                &mut combat,
                &mut queue,
                &[AttackerDeclaration {
                    creature: attacker,
                    target: AttackTarget::Player(B),
                }],
            )
            .unwrap();
            game.combat = Some(combat.clone());
            game.turn.step = Some(ironsmith::game_state::Step::DeclareBlockers);
            ironsmith::game_loop::apply_blocker_declarations(
                &mut game,
                &mut combat,
                &mut queue,
                &[BlockerDeclaration {
                    blocker,
                    blocking: attacker,
                }],
                B,
            )
            .unwrap();
            game.combat = Some(combat);
            let mut dm = SelectFirstDecisionMaker;
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            settle(&mut game, &mut dm);
            game.combat = None;
            game.turn.phase = ironsmith::Phase::NextMain;
            game.turn.step = None;
            if mode == 2 {
                let exile = game
                    .move_object(source, Zone::Exile, EventCause::effect())
                    .unwrap();
                source = enter(&mut game, exile);
            }
            kill(&mut game, other, other_player, source, &mut dm);
            settle(&mut game, &mut dm);
            assert_eq!(
                game.player(source_player).unwrap().life,
                if mode == 0 { 24 } else { 20 },
                "history mode {mode}"
            );
        }
    }
}

fn activate(
    game: &mut GameState,
    source: ObjectId,
    ability_index: usize,
    dm: &mut impl DecisionMaker,
) {
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let action = ironsmith::decision::LegalAction::ActivateAbility {
        source,
        ability_index,
    };
    assert!(
        ironsmith::decision::compute_legal_actions(game, A)
            .unwrap()
            .contains(&action)
    );
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..32 {
        if state.pending_activation.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("activation pending without choice")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    settle(game, dm);
}
#[test]
fn jason_and_tom_keep_their_paid_secondary_sacrifice_bodies() {
    for name in ["Jason Bright, Glowing Prophet", "Tom, Bert, and William"] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let spare = object(
                &mut game,
                A,
                Zone::Battlefield,
                "Sacrificed resource",
                "Type: Creature — Zombie\nPower/Toughness: 3/3",
            );
            library(&mut game, A);
            let cost = if name.starts_with("Jason") { 2 } else { 1 };
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ironsmith::ManaSymbol::Colorless, cost);
            let index = definition
                .abilities
                .iter()
                .position(|ability| {
                    matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))
                })
                .unwrap();
            let mut dm = Decisions {
                chosen: Some(spare),
            };
            activate(&mut game, source, index, &mut dm);
            assert!(game.object(spare).is_none());
            if name.starts_with("Jason") {
                assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 1);
                assert!(game.object_has_static_ability_id(
                    source,
                    ironsmith::static_abilities::StaticAbilityId::Flying
                ));
                ironsmith::turn::execute_cleanup_step(&mut game);
                game.refresh_continuous_state().unwrap();
                assert!(!game.object_has_static_ability_id(
                    source,
                    ironsmith::static_abilities::StaticAbilityId::Flying
                ));
                assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 1);
            } else {
                assert_eq!(
                    game.player(A).unwrap().hand.len(),
                    2,
                    "three cards from actual sacrificed power, then one discarded"
                );
                assert_eq!(game.player(A).unwrap().library.len(), 3);
            }
        }
    }
}

#[test]
fn sigil_uses_completed_entry_counters_then_current_destination_recheck() {
    for definition in definitions("Sigil Captain") {
        for grows in [false, true] {
            let mut game = game();
            let captain = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let hand = object(
                &mut game,
                A,
                Zone::Hand,
                "Zero-base entrant",
                "Mana cost: {0}\nType: Creature\nPower/Toughness: 0/0\nThis creature enters with a +1/+1 counter on it.",
            );
            let entrant = cast_creature_from_hand(&mut game, hand);
            let mut dm = SelectFirstDecisionMaker;
            assert_eq!(game.object(entrant).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.calculated_power(entrant), Some(1));
            assert_eq!(
                pending(&mut game, &mut dm),
                1,
                "completed 1/1 entry must trigger despite pre-move 0/0"
            );
            if grows {
                apply(
                    &mut game,
                    captain,
                    A,
                    &PutCountersEffect::new(
                        CounterType::PlusOnePlusOne,
                        1,
                        ChooseSpec::SpecificObject(entrant),
                    ),
                    &mut dm,
                );
            }
            settle(&mut game, &mut dm);
            assert_eq!(
                game.counter_count(entrant, CounterType::PlusOnePlusOne),
                if grows { 2 } else { 3 }
            );
        }
    }
}

//! Complete frozen bodies; authored only. No compiler, test, or engine execution
//! was performed while the source-coverage gate remained closed.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::combat_state::{AttackTarget, CombatState, declare_blockers};
use ironsmith::decision::{AttackerDeclaration, DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{ManaPaymentContext, PartitionContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_attacker_declarations, apply_decision_context_with_dm, apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::targeting::can_target_object;
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::types::{Subtype, Supertype};
use ironsmith::{CardId, CardType, CounterType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/filtered_evasion_bodies.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(),
        row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, &text, false)
    });
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_artifact(name, &text, false)
    });
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    restored.validate().unwrap();
    let result = [direct, materialize_artifact(&restored).unwrap()];
    for definition in &result {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    result
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn creature(game: &mut GameState, player: PlayerId, power: i32, color: ColorSet, zone: Zone) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Evasion witness")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(power, 5))
        .color_indicator(color).build();
    game.create_object_from_card(&card, player, zone)
}
fn mana(game: &mut GameState, symbol: ManaSymbol, amount: u32) {
    game.player_mut(A).unwrap().mana_pool.add(symbol, amount);
}
#[derive(Default)]
struct Choices {
    color: Option<&'static str>,
    target: Option<Target>,
    skip_targets: bool,
    bottom: Vec<ObjectId>,
    scry_calls: usize,
    cancel: bool,
    saw_cancel: bool,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description == "Choose a color" {
            return vec![context.options.iter().find(|option| {
                option.description == self.color.expect("explicit color") && option.legal
            }).unwrap().index];
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if self.skip_targets {
            assert!(context.requirements.iter().all(|requirement| requirement.min_targets == 0));
            return vec![];
        }
        if let Some(target) = self.target {
            assert!(context.requirements.iter().all(|requirement| requirement.legal_targets.contains(&target)));
            return vec![target];
        }
        SelectFirstDecisionMaker.decide_targets(game, context)
    }
    fn decide_partition(&mut self, _: &GameState, context: &PartitionContext) -> Vec<ObjectId> {
        assert_eq!(context.player, A);
        assert_eq!(context.cards.len(), 2);
        assert_eq!(context.description, "Scry 2");
        assert!(self.bottom.iter().all(|id| context.cards.iter().any(|(card, _)| card == id)));
        self.scry_calls += 1;
        self.bottom.clone()
    }
    fn decide_mana_payment(&mut self, game: &GameState, context: &ManaPaymentContext) -> ManaPaymentResponse {
        if self.cancel {
            self.saw_cancel = true;
            ManaPaymentResponse::Cancel
        } else {
            SelectFirstDecisionMaker.decide_mana_payment(game, context)
        }
    }
}
fn stack_triggers(game: &mut GameState, choices: &mut Choices) {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), choices).unwrap();
}
fn resolve_all(game: &mut GameState, choices: &mut Choices) {
    for _ in 0..12 {
        stack_triggers(game, choices);
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, choices).unwrap();
    }
    panic!("bounded fixture did not settle");
}
fn enter(game: &mut GameState, definition: &CardDefinition, choices: &mut Choices) -> ObjectId {
    let card = game.create_object_from_definition(definition, A, Zone::Hand);
    let receipt = game.move_object_with_etb_processing_with_dm(card, Zone::Battlefield, choices).unwrap();
    assert!(!receipt.pending && receipt.programs.is_empty());
    let source = receipt.original.into_result().unwrap().new_id;
    resolve_all(game, choices);
    source
}
fn activation(game: &GameState, source: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action| {
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)
    })
}
fn announce(game: &mut GameState, action: LegalAction, choices: &mut Choices) {
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut result = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), choices);
    for _ in 0..30 {
        if !state.has_pending_action() || result.is_err() { break; }
        let GameProgress::NeedsDecisionCtx(context) = result.unwrap() else {
            panic!("pending activation requires its decision");
        };
        result = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices);
    }
    if !choices.cancel { result.unwrap(); }
    assert!(!state.has_pending_action());
}
fn activate(game: &mut GameState, source: ObjectId, choices: &mut Choices) {
    game.turn.priority_player = Some(A);
    let action = activation(game, source).expect("legal paid activation");
    announce(game, action, choices);
    assert_eq!(game.stack.len(), 1);
    resolve_all(game, choices);
}
fn can_block(game: &mut GameState, source: ObjectId, blocker: ObjectId) -> bool {
    game.refresh_continuous_state().unwrap();
    ironsmith::rules::combat::can_block(game.object(source).unwrap(), game.object(blocker).unwrap(), game)
}
fn cleanup(game: &mut GameState) {
    ironsmith::turn::execute_cleanup_step(game);
    game.refresh_continuous_state().unwrap();
}
fn begin_combat(game: &mut GameState, player: PlayerId, choices: &mut Choices) {
    game.turn.active_player = player;
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::Step::BeginCombat);
    game.queue_trigger_event(Default::default(), TriggerEvent::new_with_provenance(
        ironsmith::events::BeginningOfCombatEvent::new(player), Default::default()));
    stack_triggers(game, choices);
}

#[test]
fn five_complete_frozen_bodies_keep_both_compilation_paths_and_semantic_surfaces() {
    assert_eq!(rows().len(), 5);
    for row in rows() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            let text = ironsmith_text::compiled_text::compiled_text_lines(&definition).join("\n")
                .to_lowercase().replace("more than 1 creature", "more than one creature");
            assert!(text.contains("can't be blocked"), "{name}: {text}");
            for marker in match name {
                "Cavern Stomper" => vec!["scry 2", "power 2 or less", "this turn"],
                "Harvesttide Sentry" => vec!["beginning of combat", "different powers", "power 2 or less", "this turn"],
                "Sungold Sentinel" => vec!["enters or attacks", "exile", "graveyard", "choose a color", "hexproof", "different powers"],
                "Verdant Outrider" => vec!["power 2 or less", "this turn"],
                _ => vec!["power-up", "+1/+1 counter", "the tiger god", "legendary", "4/4", "cat god", "more than one creature"],
            } { assert!(text.contains(marker), "{name}: missing {marker}: {text}"); }
        }
    }
}

#[test]
fn paid_power_filtered_evasion_tracks_current_and_late_blockers_and_expires() {
    for (name, generic) in [("Cavern Stomper", 3), ("Verdant Outrider", 1)] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let small = creature(&mut game, B, 2, ColorSet::GREEN, Zone::Battlefield);
            let large = creature(&mut game, B, 3, ColorSet::GREEN, Zone::Battlefield);
            assert!(can_block(&mut game, source, small));
            mana(&mut game, ManaSymbol::Colorless, generic);
            assert!(activation(&game, source).is_none());
            mana(&mut game, ManaSymbol::Green, 1);
            activate(&mut game, source, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert!(!can_block(&mut game, source, small));
            assert!(can_block(&mut game, source, large));
            game.add_counters(small, CounterType::PlusOnePlusOne, 1).unwrap();
            assert!(can_block(&mut game, source, small));
            let late = creature(&mut game, B, 1, ColorSet::RED, Zone::Battlefield);
            assert!(!can_block(&mut game, source, late));
            cleanup(&mut game);
            assert!(can_block(&mut game, source, late));
        }
    }
}

#[test]
fn cavern_entry_keeps_its_scry_two_companion() {
    for definition in definitions("Cavern Stomper") {
        let mut game = game();
        for _ in 0..4 { creature(&mut game, A, 1, ColorSet::GREEN, Zone::Library); }
        let library = game.player(A).unwrap().library.clone();
        let top = *library.last().unwrap();
        let next = library[library.len() - 2];
        let mut choices = Choices { bottom: vec![top], ..Default::default() };
        enter(&mut game, &definition, &mut choices);
        assert_eq!(choices.scry_calls, 1);
        assert_eq!(game.player(A).unwrap().library.len(), 4);
        assert_eq!(game.player(A).unwrap().library.first(), Some(&top));
        assert_eq!(game.player(A).unwrap().library.last(), Some(&next));
    }
}

#[test]
fn coven_uses_distinct_current_powers_at_trigger_and_resolution_only() {
    for definition in definitions("Harvesttide Sentry") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        creature(&mut game, A, 1, ColorSet::GREEN, Zone::Battlefield);
        let duplicate = creature(&mut game, A, 1, ColorSet::GREEN, Zone::Battlefield);
        let blocker = creature(&mut game, B, 2, ColorSet::GREEN, Zone::Battlefield);
        let mut choices = Choices::default();
        begin_combat(&mut game, A, &mut choices);
        assert!(game.stack_is_empty(), "three creatures but only two powers; opponent is irrelevant");
        game.add_counters(duplicate, CounterType::PlusOnePlusOne, 1).unwrap();
        game.refresh_continuous_state().unwrap();
        begin_combat(&mut game, B, &mut choices);
        assert!(game.stack_is_empty(), "only your combat triggers");
        begin_combat(&mut game, A, &mut choices);
        assert_eq!(game.stack.len(), 1);
        game.phase_out(duplicate);
        resolve_all(&mut game, &mut choices);
        assert!(can_block(&mut game, source, blocker), "intervening condition is rechecked");
        game.phase_in(duplicate);
        begin_combat(&mut game, A, &mut choices);
        resolve_all(&mut game, &mut choices);
        assert!(!can_block(&mut game, source, blocker));
        game.phase_out(duplicate);
        assert!(!can_block(&mut game, source, blocker), "resolved evasion has its own duration");
        cleanup(&mut game);
        assert!(can_block(&mut game, source, blocker));
    }
}

#[test]
fn sungold_separate_choices_keep_both_hexproof_and_evasion_until_cleanup() {
    for definition in definitions("Sungold Sentinel") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        mana(&mut game, ManaSymbol::White, 2);
        mana(&mut game, ManaSymbol::Colorless, 2);
        assert!(activation(&game, source).is_none(), "activation needs Coven");
        creature(&mut game, A, 1, ColorSet::GREEN, Zone::Battlefield);
        let two = creature(&mut game, A, 2, ColorSet::GREEN, Zone::Battlefield);
        let red = creature(&mut game, B, 2, ColorSet::RED, Zone::Battlefield);
        let blue = creature(&mut game, B, 2, ColorSet::BLUE, Zone::Battlefield);
        let green = creature(&mut game, B, 2, ColorSet::GREEN, Zone::Battlefield);
        let own_red = creature(&mut game, A, 3, ColorSet::RED, Zone::Battlefield);
        activate(&mut game, source, &mut Choices { color: Some("Red"), ..Default::default() });
        assert!(!can_block(&mut game, source, red));
        assert!(can_block(&mut game, source, blue));
        assert!(!can_target_object(&game, source, red, B).is_legal());
        assert!(can_target_object(&game, source, own_red, A).is_legal());
        activate(&mut game, source, &mut Choices { color: Some("Blue"), ..Default::default() });
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(!can_block(&mut game, source, red));
        assert!(!can_block(&mut game, source, blue));
        assert!(can_block(&mut game, source, green));
        assert!(!can_target_object(&game, source, red, B).is_legal());
        assert!(!can_target_object(&game, source, blue, B).is_legal());
        assert!(can_target_object(&game, source, green, B).is_legal());
        game.phase_out(two);
        mana(&mut game, ManaSymbol::White, 2);
        assert!(activation(&game, source).is_none());
        assert!(!can_block(&mut game, source, red));
        cleanup(&mut game);
        assert!(can_block(&mut game, source, red));
        assert!(can_block(&mut game, source, blue));
        assert!(can_target_object(&game, source, red, B).is_legal());
        assert!(can_target_object(&game, source, blue, B).is_legal());
    }
}

fn attack(game: &mut GameState, source: ObjectId, choices: &mut Choices) -> CombatState {
    game.remove_summoning_sickness(source);
    game.turn.active_player = A;
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    game.mark_combat_phase_started();
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(game, &mut combat, &mut queue, &[AttackerDeclaration {
        creature: source, target: AttackTarget::Player(B),
    }]).unwrap();
    game.combat = Some(combat.clone());
    put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
    combat
}

#[test]
fn sungold_entry_and_attack_keep_optional_graveyard_exile() {
    for definition in definitions("Sungold Sentinel") {
        for skip in [false, true] {
            let mut game = game();
            let first = creature(&mut game, B, 1, ColorSet::GREEN, Zone::Graveyard);
            let second = creature(&mut game, A, 1, ColorSet::GREEN, Zone::Graveyard);
            let first_stable = game.object(first).unwrap().stable_id;
            let second_stable = game.object(second).unwrap().stable_id;
            let mut choices = Choices { target: Some(Target::Object(first)), skip_targets: skip, ..Default::default() };
            let source = enter(&mut game, &definition, &mut choices);
            let current = game.find_object_by_stable_id(first_stable).unwrap();
            assert_eq!(game.object(current).unwrap().zone, if skip { Zone::Graveyard } else { Zone::Exile });
            choices.target = Some(Target::Object(second));
            attack(&mut game, source, &mut choices);
            resolve_all(&mut game, &mut choices);
            let current = game.find_object_by_stable_id(second_stable).unwrap();
            assert_eq!(game.object(current).unwrap().zone, if skip { Zone::Graveyard } else { Zone::Exile });
        }
    }
}

#[test]
fn white_tiger_power_up_payment_rollback_once_limit_and_complete_token() {
    for definition in definitions("White Tiger, Ava Ayala") {
        for entered_this_turn in [true, false] {
            let mut game = game();
            let source = enter(&mut game, &definition, &mut Choices::default());
            let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated), _ => None,
            }).unwrap();
            assert_eq!(activated.keyword, Some(ironsmith_core::ActivatedAbilityKeyword::PowerUp));
            if !entered_this_turn {
                game.next_turn();
                game.turn.phase = ironsmith::Phase::FirstMain;
                game.turn.step = None;
                game.turn.priority_player = Some(A);
            }
            mana(&mut game, ManaSymbol::Colorless, if entered_this_turn { 3 } else { 5 });
            assert!(activation(&game, source).is_none(), "insufficient exact cost");
            mana(&mut game, if entered_this_turn { ManaSymbol::Colorless } else { ManaSymbol::Green }, 1);
            let before = game.player(A).unwrap().mana_pool.total();
            let mut choices = Choices { cancel: true, ..Default::default() };
            let action = activation(&game, source).unwrap();
            announce(&mut game, action, &mut choices);
            assert!(choices.saw_cancel);
            assert!(game.stack_is_empty());
            assert_eq!(game.player(A).unwrap().mana_pool.total(), before);
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
            assert_eq!(game.battlefield.len(), 1);
            assert!(activation(&game, source).is_some(), "cancellation does not spend the once limit");
            activate(&mut game, source, &mut Choices::default());
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 1);
            let tokens = game.battlefield.iter().copied().filter(|id| *id != source).collect::<Vec<_>>();
            assert_eq!(tokens.len(), 1);
            let token = tokens[0];
            let object = game.object(token).unwrap();
            assert_eq!(object.name.as_str(), "The Tiger God");
            assert_eq!(object.kind, ironsmith::object::ObjectKind::Token);
            assert!(object.supertypes.contains(&Supertype::Legendary));
            assert!(object.card_types.contains(&CardType::Creature));
            assert!(object.subtypes.contains(&Subtype::Cat) && object.subtypes.contains(&Subtype::God));
            let blocker_bounds = object.abilities.iter().filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => ability.maximum_blockers(), _ => None,
            }).collect::<Vec<_>>();
            assert_eq!(blocker_bounds, vec![1], "the token has exactly one bounded evasion ability");
            assert_eq!(game.calculated_power(token), Some(4));
            assert_eq!(game.calculated_toughness(token), Some(4));
            assert_eq!(game.current_colors(token), Some(ColorSet::GREEN));
            let first = creature(&mut game, B, 2, ColorSet::RED, Zone::Battlefield);
            let second = creature(&mut game, B, 2, ColorSet::BLUE, Zone::Battlefield);
            assert!(can_block(&mut game, token, first), "one blocker remains legal");
            let combat = attack(&mut game, token, &mut Choices::default());
            let mut valid = combat.clone();
            declare_blockers(&game, &mut valid, vec![(first, token)]).unwrap();
            let mut invalid = combat;
            assert!(declare_blockers(&game, &mut invalid, vec![(first, token), (second, token)]).is_err());
            assert!(invalid.blockers.values().all(Vec::is_empty), "failed declaration rolls back");
            cleanup(&mut game);
            game.next_turn();
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            game.turn.priority_player = Some(A);
            mana(&mut game, ManaSymbol::Green, 10);
            assert!(activation(&game, source).is_none(), "once per object, not once per turn");
            let old = game.move_object_by_effect(source, Zone::Exile).unwrap();
            let returned = game.move_object_by_effect(old, Zone::Battlefield).unwrap();
            assert_ne!(source, returned);
            assert!(activation(&game, returned).is_some(), "a new incarnation has an unused ability");
        }
    }
}

#[test]
fn source_evasion_resolves_harmlessly_after_the_activated_source_leaves() {
    for name in ["Verdant Outrider", "Sungold Sentinel"] {
        for definition in definitions(name) {
            for return_source in [false, true] {
                let mut game = game();
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                creature(&mut game, A, 1, ColorSet::GREEN, Zone::Battlefield);
                creature(&mut game, A, 2, ColorSet::GREEN, Zone::Battlefield);
                let blocker = creature(&mut game, B, 2, ColorSet::RED, Zone::Battlefield);
                mana(&mut game, ManaSymbol::Colorless, 1);
                mana(&mut game, if name == "Sungold Sentinel" { ManaSymbol::White } else { ManaSymbol::Green }, 1);
                let action = activation(&game, source).unwrap();
                let mut choices = Choices { color: Some("Red"), skip_targets: true, ..Default::default() };
                announce(&mut game, action, &mut choices);
                let departed = game.move_object_by_effect(source, Zone::Exile).unwrap();
                let recipient = if return_source {
                    game.move_object_by_effect(departed, Zone::Battlefield).unwrap()
                } else {
                    creature(&mut game, A, 4, ColorSet::WHITE, Zone::Battlefield)
                };
                resolve_all(&mut game, &mut choices);
                assert!(game.stack_is_empty());
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
                assert!(can_block(&mut game, recipient, blocker), "no new incarnation or bystander inherits evasion");
                assert!(can_target_object(&game, recipient, blocker, B).is_legal(), "no new incarnation or bystander inherits hexproof");
            }
        }
    }
}

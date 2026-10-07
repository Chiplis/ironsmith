//! Reconstructed full-body scenarios. UNRUN: builds, tests, compiler probes,
//! formatters and corpus runs remain explicitly deferred for this campaign.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, EffectExecutor, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response, apply_priority_response_with_dm, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CounterType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::{encode_runtime_definition, encode_runtime_effect,
    materialize_artifact, materialize_definition, materialize_effect};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/next_play_and_flashback.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 3] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    // These are independent public compile calls. A materialized result is
    // not used as the expected direct result.
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap_or_else(|error| panic!("{name} direct: {error}"));
    assert!(!loss.is_lossy(), "{name} direct: {}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name} artifact: {error}"));
    assert!(!loss.is_lossy(), "{name} artifact: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let artifact = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    let wire = encode_runtime_definition(direct.clone()).unwrap();
    let native = materialize_definition(serde_json::from_slice(&serde_json::to_vec(&wire).unwrap()).unwrap()).unwrap();
    [direct, materialize_artifact(&artifact).unwrap(), native]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    for player in [A, B, C] {
        for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black,
            ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
            game.player_mut(player).unwrap().mana_pool.add(symbol, 30);
        }
    }
    game
}
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    game.create_object_from_definition(&compile_to_runtime_definition(name, text, false).unwrap(), owner, zone)
}
fn creature(game: &mut GameState, zone: Zone, subtype: &str) -> ObjectId {
    card(game, A, zone, subtype, &format!("Mana cost: {{1}}{{G}}\nType: Creature — {subtype}\nPower/Toughness: 2/2"))
}
fn land_creature(game: &mut GameState, text: &str) -> ObjectId {
    card(game, A, Zone::Hand, "Creature-land candidate", &format!("Type: Land Creature — Forest Dryad\nPower/Toughness: 1/1\n{text}"))
}
fn sorcery(game: &mut GameState, zone: Zone) -> ObjectId {
    card(game, A, zone, "Sorcery candidate", "Mana cost: {1}{U}\nType: Sorcery\nDraw a card.")
}
fn library(game: &mut GameState, count: usize) {
    for index in 0..count {
        card(game, A, Zone::Library, &format!("Library {index}"), "Type: Basic Land — Island");
    }
}
fn casts(game: &GameState, player: PlayerId, id: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(game, player).unwrap().into_iter()
        .filter(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id)).collect()
}
#[derive(Default)]
struct Choices {
    target: Option<Target>, objects: Vec<ObjectId>, option: Option<&'static str>, x: u32,
    decline: bool, target_prompts: usize, pause_options: bool, waiting: bool,
    pause_boolean: bool, expected_budgets: Option<Vec<u32>>,
}
impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool { self.waiting }
    fn decide_boolean(&mut self, game: &GameState, context: &BooleanContext) -> bool {
        if let Some(expected) = &self.expected_budgets { assert_eq!(&remaining(game), expected); }
        if self.pause_boolean { self.waiting = true; return false; }
        !self.decline && context.can_accept
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if self.pause_options {
            self.waiting = true;
            return vec![];
        }
        if let Some(needle) = self.option {
            if let Some(option) = context.options.iter().find(|option| option.legal && option.description.contains(needle)) {
                return vec![option.index];
            }
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_number(&mut self, game: &GameState, context: &NumberContext) -> u32 {
        if context.is_x_value { assert!(self.x <= context.max); self.x }
        else { SelectFirstDecisionMaker.decide_number(game, context) }
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        self.target_prompts += 1;
        if let Some(target) = self.target {
            assert_eq!(context.requirements.len(), 1, "one authored target group");
            assert!(context.requirements[0].legal_targets.contains(&target), "authored target must be legal");
            vec![target]
        } else { SelectFirstDecisionMaker.decide_targets(game, context) }
    }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        if !self.objects.is_empty() {
            for id in &self.objects {
                assert!(context.candidates.iter().any(|candidate| candidate.id == *id && candidate.legal));
            }
            self.objects.clone()
        } else { SelectFirstDecisionMaker.decide_objects(game, context) }
    }
}
fn announce(game: &mut GameState, action: LegalAction, dm: &mut Choices) -> TriggerQueue {
    assert!(compute_legal_actions(game, A).unwrap().contains(&action), "action must be discoverable");
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..60 {
        if !state.has_pending_action() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending action has no decision: {progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(!state.has_pending_action(), "all announcements and costs must complete");
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    queue
}
fn cast(game: &mut GameState, id: ObjectId, dm: &mut Choices) -> ObjectId {
    game.turn.priority_player = Some(A);
    let action = casts(game, A, id).into_iter().next().expect("cast must have a legal route");
    announce(game, action, dm);
    game.stack.iter().rev().find(|entry| !entry.is_ability).unwrap().object_id
}
fn settle(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..40 {
        put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
        if game.stack_is_empty() { game.turn.priority_player = Some(A); return; }
        resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("bounded scenario did not settle");
}
fn resolve_card(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) {
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    cast(game, id, dm);
    settle(game, dm);
}
fn activate(game: &mut GameState, source: ObjectId, nth: usize, dm: &mut Choices) {
    game.turn.priority_player = Some(A);
    let index = game.current_abilities(source).unwrap().iter().enumerate()
        .filter(|(_, ability)| matches!(ability.kind, AbilityKind::Activated(_))).nth(nth).unwrap().0;
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::ActivateAbility {source: id, ability_index} | LegalAction::ActivateManaAbility {source: id, ability_index}
        if *id == source && *ability_index == index)).unwrap();
    announce(game, action, dm);
    settle(game, dm);
}
fn enter(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let receipt = game.move_object_with_etb_processing_with_dm(id, Zone::Battlefield, dm).unwrap();
    assert!(!receipt.pending);
    assert!(receipt.programs.is_empty());
    let id = receipt.original.into_result().unwrap().new_id;
    settle(game, dm);
    id
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) {
    execute_effect(game, &effect, &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)).unwrap();
}
fn remaining(game: &GameState) -> Vec<u32> {
    game.effect_store.temporary_spell_ability_grants.iter().map(|grant| grant.remaining_uses).collect()
}
fn off_main(game: &mut GameState) {
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    game.turn.priority_player = Some(A);
}

#[test]
fn all_eight_frozen_bodies_have_independent_strict_direct_artifact_and_native_programs() {
    assert_eq!(rows().len(), 8);
    for row in rows() {
        assert_eq!(row["baseline"]["category"], "parser_failure");
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        }
    }
}

#[test]
fn quicken_draws_and_all_overlapping_budgets_use_completed_matching_casts() {
    for definition in definitions("Quicken") {
        let mut game = game(); library(&mut game, 5);
        let mut dm = Choices::default();
        resolve_card(&mut game, &definition, &mut dm);
        resolve_card(&mut game, &definition, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 2, "both complete bodies draw");
        assert_eq!(remaining(&game), vec![1, 1]);
        off_main(&mut game);
        let creature = creature(&mut game, Zone::Hand, "Bear");
        assert!(casts(&game, A, creature).is_empty());
        let candidate = sorcery(&mut game, Zone::Hand);
        assert!(!casts(&game, A, candidate).is_empty());
        let instant = card(&mut game, A, Zone::Hand, "Nonmatching instant", "Mana cost: {U}\nType: Instant\nDraw a card.");
        cast(&mut game, instant, &mut dm); settle(&mut game, &mut dm);
        assert_eq!(remaining(&game), vec![1, 1]);
        cast(&mut game, candidate, &mut dm);
        assert_eq!(remaining(&game), vec![0, 0]);
        assert!(game.stack.iter().filter(|entry| !entry.is_ability).all(|entry|
            !game.object_has_static_ability_id(entry.object_id, StaticAbilityId::Flash)), "as-though timing is not a gained flash keyword");
        settle(&mut game, &mut dm);
        let second = sorcery(&mut game, Zone::Hand);
        assert!(casts(&game, A, second).is_empty());
    }
}

#[test]
fn icon_complete_entry_mana_and_chosen_type_permission_survive_source_departure() {
    for definition in definitions("Progenitor's Icon") {
        let mut game = game();
        let mut dm = Choices { option: Some("Elf"), ..Default::default() };
        let icon = enter(&mut game, &definition, &mut dm);
        assert_eq!(game.chosen_creature_type(icon), Some(ironsmith::Subtype::Elf));
        let mana = game.player(A).unwrap().mana_pool.total();
        activate(&mut game, icon, 0, &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), mana + 1);
        game.untap(icon);
        activate(&mut game, icon, 1, &mut dm);
        game.move_object_by_effect(icon, Zone::Graveyard).unwrap();
        let mut other_dm = Choices { option: Some("Human"), ..Default::default() };
        let other = enter(&mut game, &definition, &mut other_dm);
        assert_eq!(game.chosen_creature_type(other), Some(ironsmith::Subtype::Human));
        off_main(&mut game);
        let elf = creature(&mut game, Zone::Hand, "Elf");
        let human = creature(&mut game, Zone::Hand, "Human");
        assert!(!casts(&game, A, elf).is_empty());
        assert!(casts(&game, A, human).is_empty());
        cast(&mut game, elf, &mut Choices::default());
        assert_eq!(remaining(&game), vec![0]);
    }
}

#[test]
fn scout_retains_own_turn_priority_drop_and_origin_while_allowing_creature_land_timing() {
    for definition in definitions("Scout's Warning") {
        for direct in [true, false] {
            let mut game = game(); library(&mut game, 2);
            let mut dm = Choices::default();
            resolve_card(&mut game, &definition, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            let land = land_creature(&mut game, "");
            off_main(&mut game);
            let action = ironsmith::special_actions::SpecialAction::PlayLand { card_id: land };
            game.turn.active_player = B;
            assert!(ironsmith::special_actions::can_perform_check(&action, &game, A).is_err());
            game.turn.active_player = A;
            game.turn.priority_player = Some(B);
            assert!(ironsmith::special_actions::can_perform_check(&action, &game, A).is_err());
            game.turn.priority_player = Some(A);
            let grave = game.move_object_by_effect(land, Zone::Graveyard).unwrap();
            assert!(ironsmith::special_actions::can_perform_check(&ironsmith::special_actions::SpecialAction::PlayLand { card_id: grave }, &game, A).is_err());
            let land = game.move_object_by_effect(grave, Zone::Hand).unwrap();
            let action = ironsmith::special_actions::SpecialAction::PlayLand { card_id: land };
            assert!(ironsmith::special_actions::can_perform_check(&action, &game, A).is_ok());
            if direct { ironsmith::special_actions::perform(action, &mut game, A, &mut dm).unwrap(); }
            else { announce(&mut game, LegalAction::PlayLand { land_id: land }, &mut dm); }
            assert_eq!(remaining(&game), vec![0]);
            assert!(!game.player(A).unwrap().can_play_land());
            let creature = creature(&mut game, Zone::Hand, "Bear");
            assert!(casts(&game, A, creature).is_empty(), "the land play consumed Scout's one-shot permission");
        }
    }
}

#[test]
fn savage_riders_bind_the_selected_spell_before_cast_receipts_and_survive_until_entry() {
    for definition in definitions("Savage Summoning") {
        let mut game = game(); let mut dm = Choices::default();
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let source_spell = cast(&mut game, source, &mut dm);
        apply(&mut game, source_spell, Effect::counter(ChooseSpec::SpecificObject(source_spell)));
        assert!(game.object(source_spell).is_some(), "Savage itself cannot be countered");
        settle(&mut game, &mut dm);
        off_main(&mut game);
        let candidate = creature(&mut game, Zone::Hand, "Bear");
        let spell = cast(&mut game, candidate, &mut dm);
        assert_eq!(remaining(&game), vec![0, 0, 0]);
        assert!(game.object_has_static_ability_id(spell, StaticAbilityId::CantBeCountered));
        apply(&mut game, spell, Effect::counter(ChooseSpec::SpecificObject(spell)));
        assert!(game.object(spell).is_some());
        settle(&mut game, &mut dm);
        let permanent = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name.as_ref() == "Bear").unwrap();
        assert_eq!(game.object(permanent).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0), 1);
        let exiled = game.move_object_by_effect(permanent, Zone::Exile).unwrap();
        let new_id = game.move_object_by_effect(exiled, Zone::Battlefield).unwrap();
        assert_eq!(game.object(new_id).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0), 0, "new incarnation does not inherit the rider");
    }
}

#[test]
fn ride_keeps_future_target_and_mana_value_binding_after_its_source_leaves() {
    for definition in definitions("Ride the Avalanche") {
        let mut game = game();
        let recipient = creature(&mut game, Zone::Battlefield, "Bear");
        let mut dm = Choices { target: Some(Target::Object(recipient)), x: 4, ..Default::default() };
        resolve_card(&mut game, &definition, &mut dm);
        assert_eq!(dm.target_prompts, 0, "Ride has no upfront target");
        assert_eq!(game.object(recipient).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0), 0);
        off_main(&mut game);
        let candidate = card(&mut game, A, Zone::Hand, "X creature", "Mana cost: {X}{G}\nType: Creature — Elf\nPower/Toughness: 1/1");
        let spell = cast(&mut game, candidate, &mut dm);
        assert_eq!(dm.target_prompts, 1, "the future cast creates a separately targeted trigger");
        assert_eq!(game.object(spell).unwrap().x_value, Some(4));
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.object(recipient).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0), 5, "X comes from the future spell's announced mana value");
        settle(&mut game, &mut dm);
        let next = creature(&mut game, Zone::Hand, "Human");
        assert!(casts(&game, A, next).is_empty());
    }
}

fn flashback_costs(game: &GameState, id: ObjectId) -> Vec<String> {
    game.effect_store.grant_registry.granted_alternative_casts_for_card(game, id, Zone::Graveyard, A)
        .into_iter().filter(|grant| matches!(grant.method, ironsmith::alternative_cast::AlternativeCastingMethod::Flashback { .. }))
        .map(|grant| grant.method.mana_cost().expect("fixed or derived flashback mana price").to_oracle()).collect()
}
fn combat_hit(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let events = ironsmith::effects::DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(B))
        .with_combat(true).execute(game, &mut EffectContext::new_default(source, A)).unwrap().events;
    for event in events { game.queue_trigger_event(Default::default(), event); }
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
}
fn attack(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    game.remove_summoning_sickness(source);
    off_main(game);
    game.mark_combat_phase_started();
    let mut combat = ironsmith::combat_state::CombatState::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(game, &mut combat, &mut queue,
        &[ironsmith::decision::AttackerDeclaration { creature: source,
            target: ironsmith::combat_state::AttackTarget::Player(B) }]).unwrap();
    game.combat = Some(combat);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}

#[test]
fn newt_uses_resolving_saddle_state_or_exact_departure_and_registers_one_price() {
    for definition in definitions("Archmage's Newt") {
        for saddled in [false, true] {
            for departed in [false, true] {
                let mut game = game(); library(&mut game, 3);
                let newt = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let grave = sorcery(&mut game, Zone::Graveyard);
                let wrong = creature(&mut game, Zone::Graveyard, "Bear");
                let opponent_card = card(&mut game, B, Zone::Graveyard, "Other graveyard", "Mana cost: {U}\nType: Instant\nDraw a card.");
                let mut dm = Choices { target: Some(Target::Object(grave)), ..Default::default() };
                combat_hit(&mut game, newt, &mut dm);
                assert_eq!(dm.target_prompts, 1, "the two prices share one announced target");
                assert!(flashback_costs(&game, grave).is_empty());
                if saddled { game.set_saddled_until_end_of_turn(newt); }
                if departed {
                    game.move_object_by_effect(newt, Zone::Graveyard).unwrap();
                    let replacement = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                    if !saddled { game.set_saddled_until_end_of_turn(replacement); }
                }
                settle(&mut game, &mut dm);
                assert_eq!(flashback_costs(&game, grave), vec![if saddled { "{0}" } else { "{1}{U}" }]);
                assert!(flashback_costs(&game, wrong).is_empty());
                assert!(flashback_costs(&game, opponent_card).is_empty());
                dm.target = None;
                let spell = cast(&mut game, grave, &mut dm);
                settle(&mut game, &mut dm);
                assert!(game.object(spell).is_none());
                assert!(game.exile.iter().any(|id| game.object(*id).unwrap().name.as_ref() == "Sorcery candidate"), "the granted route retains flashback's exile replacement");
            }
        }
    }
}

#[test]
fn newt_full_saddle_activation_pays_other_creatures_and_remains_sorcery_timed() {
    for definition in definitions("Archmage's Newt") {
        let mut game = game();
        let newt = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let helper = card(&mut game, A, Zone::Battlefield, "Saddle contributor", "Mana cost: {2}{G}\nType: Creature — Bear\nPower/Toughness: 3/3");
        let mut dm = Choices { objects: vec![helper], ..Default::default() };
        activate(&mut game, newt, 0, &mut dm);
        assert!(game.is_saddled(newt));
        assert!(game.is_tapped(helper));
        assert!(!game.is_tapped(newt));
        game.untap(helper);
        off_main(&mut game);
        assert!(compute_legal_actions(&game, A).unwrap().iter().all(|action|
            !matches!(action, LegalAction::ActivateAbility { source, .. } if *source == newt)));
    }
}

#[test]
fn newt_missing_or_unknown_departure_evidence_errors_instead_of_choosing_a_price() {
    let mut game = game();
    let unknown = game.new_object_id();
    let context = EffectContext::new_default(unknown, A);
    assert!(matches!(ironsmith::condition_eval::evaluate_condition_resolution(&game,
        &ironsmith::ConditionExpr::SourceIsSaddled, &context),
        Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
    let source = creature(&mut game, Zone::Battlefield, "Mount");
    let mut snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    snapshot.saddled = None;
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    // A retained trigger snapshot alone cannot establish the later departure
    // designation. Exact departure history, when present, wins over this one.
    let context = EffectContext::new_default(source, A).with_source_snapshot(snapshot);
    assert!(!ironsmith::condition_eval::evaluate_condition_resolution(&game,
        &ironsmith::ConditionExpr::SourceIsSaddled, &context).unwrap());
}

#[test]
fn doctor_investigates_then_its_actual_optional_sacrifice_owns_the_later_target() {
    for definition in definitions("The Fugitive Doctor") {
        for decline in [false, true] {
            let mut game = game(); library(&mut game, 2);
            let mut dm = Choices::default();
            let doctor = enter(&mut game, &definition, &mut dm);
            let clues: Vec<_> = game.battlefield.iter().copied().filter(|id|
                game.calculated_subtypes(*id).contains(&ironsmith::Subtype::Clue)).collect();
            assert_eq!(clues.len(), 1, "the complete enter trigger investigates");
            assert_eq!(game.object(clues[0]).unwrap().name, "Clue Token");
            let grave = sorcery(&mut game, Zone::Graveyard);
            let unrelated = creature(&mut game, Zone::Graveyard, "Bear");
            dm.decline = decline;
            dm.objects = if decline { vec![] } else { clues.clone() };
            attack(&mut game, doctor, &mut dm);
            assert_eq!(dm.target_prompts, 0, "the attack trigger has no upfront target");
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(flashback_costs(&game, grave).is_empty());
            assert!(flashback_costs(&game, unrelated).is_empty());
            if decline {
                settle(&mut game, &mut dm);
                assert!(game.object(clues[0]).is_some());
                assert_eq!(dm.target_prompts, 0);
            } else {
                assert!(game.object(clues[0]).is_none(), "the Clue was actually sacrificed");
                dm.objects.clear(); dm.target = Some(Target::Object(grave));
                put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
                assert_eq!(dm.target_prompts, 1, "the sacrifice receipt creates the target-bearing reflexive trigger");
                game.move_object_by_effect(doctor, Zone::Exile).unwrap();
                settle(&mut game, &mut dm);
                assert_eq!(flashback_costs(&game, grave), vec!["{2}{R}{G}"]);
            }
        }
    }
}

#[test]
fn viral_poison_is_live_existential_opponent_only_and_its_token_has_toxic() {
    for definition in definitions("Viral Spawning") {
        let mut game = GameState::new(vec!["Alice".into(), "Ally".into(), "Bob".into(), "Charlie".into()], 20);
        let d = PlayerId(3);
        game.enable_team_vs_team(vec![vec![A, B], vec![C, d]]).unwrap();
        game.turn.active_player = A; game.turn.priority_player = Some(A); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        for symbol in [ManaSymbol::Colorless, ManaSymbol::Green] { game.player_mut(A).unwrap().mana_pool.add(symbol, 10); }
        let viral = game.create_object_from_definition(&definition, A, Zone::Graveyard);
        let other = sorcery(&mut game, Zone::Graveyard);
        assert!(casts(&game, A, viral).is_empty());
        game.add_player_counters_with_source(B, CounterType::Poison, 3, Some(viral), Some(A)).unwrap();
        assert!(casts(&game, A, viral).is_empty(), "a teammate cannot turn on corrupted");
        for opponent in [C, d] { game.add_player_counters_with_source(opponent, CounterType::Poison, 2, Some(viral), Some(A)).unwrap(); }
        assert!(casts(&game, A, viral).is_empty(), "poison is not summed across opponents");
        game.add_player_counters_with_source(C, CounterType::Poison, 1, Some(viral), Some(A)).unwrap();
        assert_eq!(flashback_costs(&game, viral), vec!["{2}{G}"]);
        assert!(casts(&game, A, other).is_empty(), "the static grant is self scoped");
        game.remove_player_counters_with_source(C, CounterType::Poison, 1, Some(viral), Some(A));
        assert!(casts(&game, A, viral).is_empty(), "the condition can turn off");
        game.add_player_counters_with_source(C, CounterType::Poison, 1, Some(viral), Some(A)).unwrap();
        cast(&mut game, viral, &mut Choices::default()); settle(&mut game, &mut Choices::default());
        let tokens: Vec<_> = game.battlefield.iter().copied().filter(|id| game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token).collect();
        assert_eq!(tokens.len(), 1);
        let token = tokens[0];
        assert_eq!(game.current_power(token), Some(3)); assert_eq!(game.current_toughness(token), Some(3));
        assert!(game.calculated_subtypes(token).contains(&ironsmith::Subtype::Phyrexian));
        assert!(game.calculated_subtypes(token).contains(&ironsmith::Subtype::Beast));
        let before = game.player(C).unwrap().poison_counters;
        let combat = ironsmith::combat_state::CombatState {
            attackers: vec![ironsmith::combat_state::AttackerInfo { creature: token, target: ironsmith::combat_state::AttackTarget::Player(C) }],
            block_declaration_complete: true, ..Default::default()
        };
        game.combat = Some(combat.clone()); game.turn.phase = ironsmith::Phase::Combat; game.turn.step = Some(ironsmith::Step::CombatDamage);
        ironsmith::game_loop::try_execute_combat_damage_step_with_dm(&mut game, &combat, false, &mut Choices::default()).unwrap();
        assert_eq!(game.player(C).unwrap().poison_counters, before + 1);
        assert!(game.exile.iter().any(|id| game.object(*id).unwrap().name.as_ref() == "Viral Spawning"));
    }
}

#[test]
fn native_next_play_payload_modes_round_trip_and_old_ability_payloads_default_narrowly() {
    use ironsmith::effects::GrantNextSpellAbilityEffect;
    use ironsmith_core::NextSpellGrantMode as Mode;
    for mode in [Mode::Ability, Mode::CastTiming, Mode::PlayTiming, Mode::IncarnationAbility] {
        let effect = GrantNextSpellAbilityEffect::new(PlayerFilter::You, ObjectFilter::creature(),
            StaticAbility::flash().into()).with_mode(mode);
        let wire = encode_runtime_effect(Effect::new(effect.clone())).unwrap();
        let restored = materialize_effect(serde_json::from_slice(&serde_json::to_vec(&wire).unwrap()).unwrap()).unwrap();
        let restored = restored.downcast_ref::<GrantNextSpellAbilityEffect>().unwrap();
        assert_eq!(restored.mode, mode);
        assert_eq!(restored.filter, effect.filter);
        assert_eq!(restored.player, effect.player);
    }
    let old: ironsmith_core::GrantNextSpellAbilityEffect<serde_json::Value> = serde_json::from_value(serde_json::json!({
        "player": PlayerFilter::You, "filter": ObjectFilter::creature(), "ability": null
    })).unwrap();
    assert_eq!(old.mode, Mode::Ability);
}

#[test]
fn matching_cast_from_an_independent_graveyard_price_consumes_timing_and_other_players_do_not() {
    for definition in definitions("Quicken") {
        let mut game = game(); library(&mut game, 4);
        let mut dm = Choices::default(); resolve_card(&mut game, &definition, &mut dm);
        off_main(&mut game);
        let candidate = card(&mut game, A, Zone::Graveyard, "Independent flashback", "Mana cost: {2}{U}\nType: Sorcery\nDraw a card.\nFlashback {0}");
        let opponent = card(&mut game, B, Zone::Hand, "Opponent sorcery", "Mana cost: {U}\nType: Sorcery\nDraw a card.");
        game.turn.priority_player = Some(B);
        assert!(casts(&game, B, opponent).is_empty());
        game.turn.priority_player = Some(A);
        let actions = casts(&game, A, candidate);
        assert!(actions.iter().any(|action| matches!(action, LegalAction::CastSpell { casting_method: CastingMethod::Alternative(0), .. })));
        cast(&mut game, candidate, &mut dm);
        assert_eq!(remaining(&game), vec![0]);
        settle(&mut game, &mut dm);
    }
}

#[test]
fn failed_pending_cast_rolls_back_all_savage_riders_and_keeps_its_timing_budget() {
    for definition in definitions("Savage Summoning") {
        let mut game = game(); resolve_card(&mut game, &definition, &mut Choices::default());
        off_main(&mut game);
        let candidate = card(&mut game, A, Zone::Hand, "Pending creature", "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/2\nKicker {1}");
        let action = casts(&game, A, candidate).into_iter().next().unwrap();
        let mut state = PriorityLoopState::new(3); let mut queue = TriggerQueue::new();
        let progress = apply_priority_response(&mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(action)).unwrap();
        assert!(matches!(progress, GameProgress::NeedsDecisionCtx(_)));
        let proposed = state.pending_cast.as_ref().expect("announcements remain pending").spell_id;
        assert_eq!(remaining(&game), vec![1, 0, 0]);
        assert!(game.object_has_static_ability_id(proposed, StaticAbilityId::CantBeCountered));
        assert!(game.turn_store.turn_history.spell_cast_order(proposed).is_none(), "proposal has no completed-cast receipt");
        let error = apply_priority_response(&mut game, &mut queue, &mut state,
            &PriorityResponse::OptionalCosts(vec![(usize::MAX, 1)])).unwrap_err();
        assert!(matches!(error, ironsmith::game_loop::GameLoopError::ActionCancelled(_)));
        assert_eq!(remaining(&game), vec![1, 1, 1]);
        assert_eq!(game.object(candidate).unwrap().zone, Zone::Hand);
        assert!(!game.object_has_static_ability_id(candidate, StaticAbilityId::CantBeCountered));
        assert!(!state.has_pending_action());
        cast(&mut game, candidate, &mut Choices::default());
        assert_eq!(remaining(&game), vec![0, 0, 0]);
    }
}

#[test]
fn pending_scout_land_entry_restores_face_budget_drop_and_original_object_on_both_routes() {
    for definition in definitions("Scout's Warning") {
        for priority in [false, true] {
            let mut game = game(); library(&mut game, 2);
            resolve_card(&mut game, &definition, &mut Choices::default());
            let land = land_creature(&mut game, "As this creature enters, choose a creature type.");
            off_main(&mut game);
            let mut paused = Choices { pause_options: true, ..Default::default() };
            if priority {
                let mut state = PriorityLoopState::new(3); let mut queue = TriggerQueue::new();
                apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                    &PriorityResponse::PriorityAction(LegalAction::PlayLand { land_id: land }), &mut paused).unwrap();
            } else {
                ironsmith::special_actions::perform(ironsmith::special_actions::SpecialAction::PlayLand {card_id: land},
                    &mut game, A, &mut paused).unwrap();
            }
            assert!(paused.waiting);
            assert_eq!(remaining(&game), vec![1]);
            assert_eq!(game.object(land).unwrap().zone, Zone::Hand);
            assert!(game.player(A).unwrap().can_play_land());
            assert!(game.chosen_creature_type(land).is_none());
            ironsmith::special_actions::perform(ironsmith::special_actions::SpecialAction::PlayLand {card_id: land},
                &mut game, A, &mut Choices { option: Some("Elf"), ..Default::default() }).unwrap();
            assert_eq!(remaining(&game), vec![0]);
        }
    }
}

#[test]
fn scout_reserves_before_entry_additions_and_pending_addition_restores_both_budgets() {
    for definition in definitions("Scout's Warning") {
        for priority in [false, true] {
            let mut game = game(); library(&mut game, 2);
            resolve_card(&mut game, &definition, &mut Choices::default());
            let land = land_creature(&mut game, "");
            off_main(&mut game);
            let source = creature(&mut game, Zone::Battlefield, "Elf");
            let addition = Effect::new(ironsmith::effects::GrantNextSpellAbilityEffect::new(
                PlayerFilter::You, ObjectFilter::creature(), StaticAbility::flash().into(),
            ).with_mode(ironsmith_core::NextSpellGrantMode::PlayTiming));
            game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
                source, A, ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(land), Some(Zone::Hand), Some(Zone::Battlefield),
                ), ironsmith::replacement::ReplacementAction::Additionally(vec![addition, Effect::may(vec![Effect::gain_life(1)])]),
            ));
            let mut dm = Choices { pause_boolean: true, expected_budgets: Some(vec![0, 1]), ..Default::default() };
            if priority {
                apply_priority_response_with_dm(&mut game, &mut TriggerQueue::new(), &mut PriorityLoopState::new(3),
                    &PriorityResponse::PriorityAction(LegalAction::PlayLand {land_id: land}), &mut dm).unwrap();
            } else {
                ironsmith::special_actions::perform(ironsmith::special_actions::SpecialAction::PlayLand {card_id: land},
                    &mut game, A, &mut dm).unwrap();
            }
            assert!(dm.waiting);
            assert_eq!(remaining(&game), vec![1], "pending restores the old permission and removes the new grant");
            assert_eq!(game.object(land).unwrap().zone, Zone::Hand);
            let mut retry = Choices { expected_budgets: Some(vec![0, 1]), ..Default::default() };
            ironsmith::special_actions::perform(ironsmith::special_actions::SpecialAction::PlayLand {card_id: land},
                &mut game, A, &mut retry).unwrap();
            assert_eq!(remaining(&game), vec![0, 1], "the original play cannot consume the addition's later grant");
        }
    }
}

#[test]
fn expired_next_play_permissions_do_not_create_new_actions() {
    for name in ["Quicken", "Scout's Warning", "Ride the Avalanche", "Savage Summoning"] {
        for definition in definitions(name) {
            let mut game = game(); library(&mut game, 2);
            resolve_card(&mut game, &definition, &mut Choices::default());
            game.turn.turn_number += 1;
            off_main(&mut game);
            let sorcery = sorcery(&mut game, Zone::Hand);
            let creature = creature(&mut game, Zone::Hand, "Bear");
            let land = land_creature(&mut game, "");
            assert!(casts(&game, A, sorcery).is_empty()); assert!(casts(&game, A, creature).is_empty());
            assert!(ironsmith::special_actions::can_perform_check(&ironsmith::special_actions::SpecialAction::PlayLand {card_id: land}, &game, A).is_err());
        }
    }
}

#[test]
fn unknown_saddle_snapshot_round_trip_remains_unknown_and_cannot_serve_as_false_evidence() {
    let mut game = game();
    let source = creature(&mut game, Zone::Battlefield, "Mount");
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    let mut legacy = serde_json::to_value(snapshot).unwrap();
    legacy.as_object_mut().unwrap().remove("saddled");
    let snapshot: ironsmith::snapshot::ObjectSnapshot = serde_json::from_value(legacy).unwrap();
    assert_eq!(snapshot.saddled, None);
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    let event = ironsmith::events::RawEvent::new(ironsmith::events::ZoneChangeEvent::with_cause(
        source, Zone::Battlefield, Zone::Graveyard,
        ironsmith::events::cause::EventCause::effect(), Some(snapshot.clone()),
    ), game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::ZoneChange));
    game.turn_store.turn_history.record_event(&event, Some(snapshot), None);
    assert!(matches!(ironsmith::condition_eval::evaluate_condition_resolution(&game,
        &ironsmith::ConditionExpr::SourceIsSaddled, &EffectContext::new_default(source, A)),
        Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
}

#[test]
fn newt_does_not_grant_a_different_incarnation_when_its_announced_card_moves() {
    for definition in definitions("Archmage's Newt") {
        let mut game = game();
        let newt = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.set_saddled_until_end_of_turn(newt);
        let grave = sorcery(&mut game, Zone::Graveyard);
        let mut dm = Choices { target: Some(Target::Object(grave)), ..Default::default() };
        combat_hit(&mut game, newt, &mut dm);
        let moved = game.move_object_by_effect(grave, Zone::Hand).unwrap();
        let returned = game.move_object_by_effect(moved, Zone::Graveyard).unwrap();
        dm.target = None;
        settle(&mut game, &mut dm);
        assert!(flashback_costs(&game, returned).is_empty());
    }
}

#[test]
fn source_lost_before_icon_activation_resolves_uses_its_exact_chosen_type_receipt() {
    for definition in definitions("Progenitor's Icon") {
        let mut game = game();
        let mut dm = Choices { option: Some("Elf"), ..Default::default() };
        let icon = enter(&mut game, &definition, &mut dm);
        let index = game.current_abilities(icon).unwrap().iter().enumerate()
            .filter(|(_, ability)| matches!(ability.kind, AbilityKind::Activated(_))).nth(1).unwrap().0;
        announce(&mut game, LegalAction::ActivateAbility {source: icon, ability_index: index}, &mut dm);
        game.move_object_by_effect(icon, Zone::Exile).unwrap();
        let replacement = enter(&mut game, &definition, &mut Choices {option: Some("Human"), ..Default::default()});
        assert_eq!(game.chosen_creature_type(replacement), Some(ironsmith::Subtype::Human));
        settle(&mut game, &mut Choices::default());
        off_main(&mut game);
        let elf = creature(&mut game, Zone::Hand, "Elf"); let human = creature(&mut game, Zone::Hand, "Human");
        assert!(!casts(&game, A, elf).is_empty()); assert!(casts(&game, A, human).is_empty());
    }
}

#[test]
fn scout_uses_the_selected_land_face_and_never_borrows_the_other_faces_creature_type() {
    use ironsmith::card::{LinkedFaceLayout, PowerToughness};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    for definition in definitions("Scout's Warning") {
        for back_is_creature in [false, true] {
            let mut game = game(); library(&mut game, 2);
            resolve_card(&mut game, &definition, &mut Choices::default());
            let front_id = ironsmith::CardId::new(); let back_id = ironsmith::CardId::new();
            let front = CardDefinitionBuilder::new(front_id, "Scout front")
                .card_types(if back_is_creature {vec![ironsmith::CardType::Land]} else {vec![ironsmith::CardType::Land, ironsmith::CardType::Creature]})
                .power_toughness(PowerToughness::fixed(1, 1))
                .other_face(back_id).other_face_name("Scout back").linked_face_layout(LinkedFaceLayout::TransformLike).build();
            let back = CardDefinitionBuilder::new(back_id, "Scout back")
                .card_types(if back_is_creature {vec![ironsmith::CardType::Land, ironsmith::CardType::Creature]} else {vec![ironsmith::CardType::Land]})
                .power_toughness(PowerToughness::fixed(1, 1))
                .other_face(front_id).other_face_name("Scout front").linked_face_layout(LinkedFaceLayout::TransformLike).build();
            game.register_linked_face_definition(&front); game.register_linked_face_definition(&back);
            let id = game.create_object_from_definition(&front, A, Zone::Hand);
            off_main(&mut game);
            let actions = compute_legal_actions(&game, A).unwrap();
            assert_eq!(actions.contains(&LegalAction::PlayLandBackFace {land_id: id}), back_is_creature);
            assert_eq!(actions.contains(&LegalAction::PlayLand {land_id: id}), !back_is_creature);
            if back_is_creature {
                announce(&mut game, LegalAction::PlayLandBackFace {land_id: id}, &mut Choices::default());
                assert_eq!(remaining(&game), vec![0]);
                assert!(game.battlefield.iter().any(|id| game.object(*id).unwrap().name.as_ref() == "Scout back"));
            }
        }
    }
}

#[test]
fn ordinary_main_phase_land_play_consumes_all_overlapping_scout_permissions() {
    for definition in definitions("Scout's Warning") {
        let mut game = game(); library(&mut game, 3);
        resolve_card(&mut game, &definition, &mut Choices::default());
        resolve_card(&mut game, &definition, &mut Choices::default());
        assert_eq!(remaining(&game), vec![1, 1]);
        let land = land_creature(&mut game, "");
        ironsmith::special_actions::perform(ironsmith::special_actions::SpecialAction::PlayLand {card_id: land},
            &mut game, A, &mut Choices::default()).unwrap();
        assert_eq!(remaining(&game), vec![0, 0]);
    }
}

#[test]
fn effect_driven_land_plays_consume_matching_scout_grants_on_the_selected_face() {
    use ironsmith::card::{LinkedFaceLayout, PowerToughness};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith_core::NextSpellGrantMode;
    for definition in definitions("Scout's Warning") {
        for as_copy in [false, true] {
            for selected_is_creature in [false, true] {
                let mut game = game(); library(&mut game, 3);
                resolve_card(&mut game, &definition, &mut Choices::default());
                resolve_card(&mut game, &definition, &mut Choices::default());
                let source = creature(&mut game, Zone::Battlefield, "Elf");
                // These otherwise matching grants must survive a land play:
                // one is cast-only and one belongs to another player.
                for (player, mode) in [(A, NextSpellGrantMode::CastTiming), (B, NextSpellGrantMode::PlayTiming)] {
                    game.add_temporary_spell_ability_grant_with_mode(player, source,
                        ObjectFilter::creature(), StaticAbility::flash().into(), 1, mode);
                }
                let types = |is_creature| if is_creature {
                    vec![ironsmith::CardType::Land, ironsmith::CardType::Creature]
                } else { vec![ironsmith::CardType::Land] };
                let front_id = ironsmith::CardId::new(); let back_id = ironsmith::CardId::new();
                let front = CardDefinitionBuilder::new(front_id, "Effect land front")
                    .card_types(types(!selected_is_creature)).power_toughness(PowerToughness::fixed(1, 1))
                    .other_face(back_id).other_face_name("Effect land back")
                    .linked_face_layout(LinkedFaceLayout::TransformLike).build();
                let back = CardDefinitionBuilder::new(back_id, "Effect land back")
                    .card_types(types(selected_is_creature)).power_toughness(PowerToughness::fixed(1, 1))
                    .other_face(front_id).other_face_name("Effect land front")
                    .linked_face_layout(LinkedFaceLayout::TransformLike).build();
                game.register_linked_face_definition(&front); game.register_linked_face_definition(&back);
                let land = game.create_object_from_definition(&front, A, Zone::Exile);
                game.object_mut(land).unwrap().apply_definition_face(&back);
                let selected = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(land).unwrap(), &game);
                let mut effect = ironsmith::effects::CastTaggedEffect::new("selected", PlayerFilter::You).allow_land();
                if as_copy { effect = effect.as_copy(); }
                off_main(&mut game);
                let mut ctx = EffectContext::new_default(source, A);
                ctx.set_tagged_objects("selected", vec![selected]);
                let outcome = effect.execute(&mut game, &mut ctx).unwrap();
                let arrival = outcome.objects().unwrap()[0];
                assert!(game.battlefield.contains(&arrival));
                assert_eq!(game.player(A).unwrap().lands_played_this_turn, 1);
                let expected = if selected_is_creature { 0 } else { 1 };
                assert_eq!(remaining(&game), vec![expected, expected, 1, 1],
                    "selected face alone determines all matching play budgets; copy={as_copy}");
                if as_copy { assert_eq!(game.object(land).unwrap().zone, Zone::Exile); }

                // Remove only the independently supplied cast permission to
                // observe whether Scout still authorizes the next creature.
                game.effect_store.temporary_spell_ability_grants.retain(|grant| grant.mode != NextSpellGrantMode::CastTiming);
                let next = creature(&mut game, Zone::Hand, "Bear");
                assert_eq!(casts(&game, A, next).is_empty(), selected_is_creature);
            }
        }
    }
}

#[test]
fn effect_driven_land_reservations_precede_additions_and_restore_on_pending_or_error() {
    #[derive(Clone, Copy, PartialEq)]
    enum Completion { Ready, Pending, Error }
    for definition in definitions("Scout's Warning") {
        for as_copy in [false, true] {
            for completion in [Completion::Ready, Completion::Pending, Completion::Error] {
                let mut game = game(); library(&mut game, 3);
                resolve_card(&mut game, &definition, &mut Choices::default());
                resolve_card(&mut game, &definition, &mut Choices::default());
                let source = creature(&mut game, Zone::Battlefield, "Elf");
                let land = card(&mut game, A, Zone::Exile, "Effect creature-land",
                    "Type: Land Creature — Forest Dryad\nPower/Toughness: 1/1");
                let selected = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(land).unwrap(), &game);
                let sentinel = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
                let addition = Effect::new(ironsmith::effects::GrantNextSpellAbilityEffect::new(
                    PlayerFilter::You, ObjectFilter::creature(), StaticAbility::flash().into(),
                ).with_mode(ironsmith_core::NextSpellGrantMode::PlayTiming));
                let mut effects = vec![addition, Effect::gain_life(3), Effect::may(vec![Effect::gain_life(1)])];
                if completion == Completion::Error { effects.push(Effect::lose_life(ironsmith::effect::Value::X)); }
                let shield = game.effect_store.replacement_effects.add_one_shot_effect(
                    ironsmith::replacement::ReplacementEffect::with_matcher(source, A,
                        ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                            ObjectFilter::default().with_type(ironsmith::CardType::Land),
                            Some(if as_copy { Zone::Command } else { Zone::Exile }), Some(Zone::Battlefield),
                        ), ironsmith::replacement::ReplacementAction::Additionally(effects)),
                );
                game.take_pending_trigger_events();
                let before_ids = game.next_object_id_counter();
                let before_objects = game.objects_in_deterministic_order().len();
                let mut effect = ironsmith::effects::CastTaggedEffect::new("selected", PlayerFilter::You).allow_land();
                if as_copy { effect = effect.as_copy(); }
                let mut dm = Choices { pause_boolean: completion == Completion::Pending,
                    expected_budgets: Some(vec![0, 0, 1]), ..Default::default() };
                let mut ctx = EffectContext::new(source, A, &mut dm);
                ctx.set_tagged_objects("selected", vec![selected.clone()]);
                ctx.set_tagged_objects("it", vec![sentinel.clone()]);
                let result = effect.execute(&mut game, &mut ctx);
                assert_eq!(ctx.get_tagged_all("selected").unwrap()[0].object_id, land);
                assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id, source);
                assert_eq!(ctx.source, source); assert_eq!(ctx.controller, A);
                if completion == Completion::Ready {
                    assert_eq!(result.unwrap().objects().unwrap().len(), 1);
                    assert_eq!(remaining(&game), vec![0, 0, 1], "later addition survives the original play");
                    assert_eq!(game.player(A).unwrap().lands_played_this_turn, 1);
                    assert_eq!(game.player(A).unwrap().life, 24);
                    assert_eq!(game.objects_in_deterministic_order().len(), before_objects + usize::from(as_copy));
                    assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
                } else {
                    if completion == Completion::Error {
                        assert!(matches!(result, Err(ironsmith::effects::ExecutionError::UnresolvableValue(_))));
                    } else {
                        assert!(result.unwrap().events.is_empty());
                        assert!(ctx.decision_maker.awaiting_choice());
                    }
                    assert_eq!(remaining(&game), vec![1, 1], "rollback restores old grants and removes the new grant");
                    assert_eq!(game.object(land).unwrap().zone, Zone::Exile);
                    assert_eq!(game.next_object_id_counter(), before_ids);
                    assert_eq!(game.objects_in_deterministic_order().len(), before_objects);
                    assert_eq!(game.player(A).unwrap().lands_played_this_turn, 0);
                    assert_eq!(game.player(A).unwrap().life, 20);
                    assert!(game.command_zone.is_empty());
                    assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
                    assert!(game.take_pending_trigger_events().is_empty());
                }
                drop(ctx);
                if completion == Completion::Pending {
                    let mut retry = Choices { expected_budgets: Some(vec![0, 0, 1]), ..Default::default() };
                    let mut ctx = EffectContext::new(source, A, &mut retry);
                    ctx.set_tagged_objects("selected", vec![selected]);
                    let outcome = effect.execute(&mut game, &mut ctx).unwrap();
                    assert_eq!(outcome.objects().unwrap().len(), 1);
                    assert_eq!(remaining(&game), vec![0, 0, 1]);
                    assert_eq!(game.player(A).unwrap().lands_played_this_turn, 1);
                    assert_eq!(game.player(A).unwrap().life, 24);
                    assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
                }
            }
        }
    }
}

#[test]
fn cleanup_ends_unused_timing_but_does_not_strip_attached_savage_incarnation_riders() {
    for definition in definitions("Scout's Warning") {
        let mut game = game(); library(&mut game, 2);
        resolve_card(&mut game, &definition, &mut Choices::default());
        game.cleanup_temporary_spell_ability_grants_end_of_turn();
        assert!(remaining(&game).is_empty(), "cleanup ends the grant before any new priority window this turn");
        off_main(&mut game);
        let land = land_creature(&mut game, "");
        assert!(ironsmith::special_actions::can_perform_check(&ironsmith::special_actions::SpecialAction::PlayLand {card_id: land}, &game, A).is_err());
    }
    for definition in definitions("Savage Summoning") {
        let mut game = game(); resolve_card(&mut game, &definition, &mut Choices::default());
        let candidate = creature(&mut game, Zone::Hand, "Bear");
        let spell = cast(&mut game, candidate, &mut Choices::default());
        game.cleanup_temporary_spell_ability_grants_end_of_turn();
        game.cleanup_temporary_object_static_ability_grants_end_of_turn();
        game.turn.turn_number += 1;
        assert!(game.object_has_static_ability_id(spell, StaticAbilityId::CantBeCountered));
        settle(&mut game, &mut Choices::default());
        let permanent = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name.as_ref() == "Bear").unwrap();
        assert_eq!(game.object(permanent).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0), 1);
    }
}

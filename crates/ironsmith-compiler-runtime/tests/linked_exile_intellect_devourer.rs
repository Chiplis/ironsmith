//! Frozen full-body scenarios, source-authored and UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{SelectObjectsContext, ViewCardsContext};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm,
    drain_pending_trigger_events_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);
fn body() -> String {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/linked_exile_static_permissions.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Intellect Devourer").unwrap();
    format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(), row["oracle_text"].as_str().unwrap())
}
fn definitions() -> [CardDefinition; 2] {
    let source = body(); let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Intellect Devourer", &source, false));
    let direct = direct.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Intellect Devourer", &source, false));
    let (artifact, _) = artifact.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text()); artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap(); assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState { let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20); main(&mut game, A); game }
fn main(game: &mut GameState, player: PlayerId) { game.turn.active_player = player; game.turn.priority_player = Some(player); game.turn.phase = Phase::FirstMain; game.turn.step = None; }
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, land: bool) -> ObjectId {
    let text = if land { "Type: Land" } else { "Mana cost: {W}\nType: Sorcery\nYou gain 1 life." };
    game.create_object_from_definition(&compile_to_runtime_definition("Hand selection card", text, false).unwrap(), owner, zone)
}
fn enter(game: &mut GameState, definition: &CardDefinition) -> (ObjectId, ironsmith::linked_exile::LinkedExileOwner) {
    let card = game.create_object_from_definition(definition, A, Zone::Hand); let stable = game.object(card).unwrap().stable_id;
    execute_effect(game, &Effect::move_to_zone(ChooseSpec::SpecificObject(card), Zone::Battlefield, false),
        &mut EffectContext::new(card, A, &mut SelectFirstDecisionMaker)).unwrap();
    let source = game.find_object_by_stable_id(stable).unwrap(); let mut queue = TriggerQueue::new();
    drain_pending_trigger_events_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.stack.len(), 1); (source, game.stack[0].linked_exile_owner.clone().unwrap())
}
#[derive(Default)]
struct Choices { calls: Vec<PlayerId>, viewed: Vec<PlayerId>, initial_exile: usize, pause: Option<PlayerId>, pending: bool }
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool { true }
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        assert!(matches!(context.player, B | C));
        assert_eq!(game.exile.len(), self.initial_exile, "every opponent chooses before any original exile");
        assert!(context.candidates.iter().all(|candidate| game.object(candidate.id).unwrap().owner == context.player));
        self.calls.push(context.player);
        if self.pause == Some(context.player) { self.pending = true; return vec![]; }
        context.candidates.iter().filter(|candidate| candidate.legal).take(context.min).map(|candidate| candidate.id).collect()
    }
    fn view_cards(&mut self, _: &GameState, player: PlayerId, _: &[ObjectId], _: &ViewCardsContext) {
        assert!(matches!(player, B | C)); self.viewed.push(player);
    }
}
fn may_play(game: &GameState, card: ObjectId, player: PlayerId) -> bool { game.effect_store.grant_registry.card_can_play_from_zone(game, card, Zone::Exile, player) }
fn actions(game: &GameState, card: ObjectId, player: PlayerId) -> Vec<LegalAction> {
    compute_legal_actions(game, player).unwrap().into_iter().filter(|action| match action {
        LegalAction::CastSpell { spell_id, .. } => *spell_id == card, LegalAction::PlayLand { land_id } => *land_id == card, _ => false,
    }).collect()
}
fn play(game: &mut GameState, member: ObjectId) {
    let action = actions(game, member, A).into_iter().next().unwrap(); let mut state = PriorityLoopState::new(3); let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker).unwrap();
    for _ in 0..32 { if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending cast has a decision"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut SelectFirstDecisionMaker).unwrap();
    }
    assert!(!state.has_pending_action());
}

#[test]
fn full_body_pairs_simultaneous_hand_choices_with_the_complete_live_mana_permission() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let producer = definition.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Triggered(trigger) => Some(trigger), _ => None }).unwrap();
        let spec = definition.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Static(ability) => ability.grant_spec(), _ => None }).unwrap();
        assert_eq!(producer.effects.linked_exile_pair, spec.linked_exile_pair); assert!(spec.linked_exile_pair.is_some());
        assert_eq!(spec.cast_mana_spend_mode, ironsmith_core::value_model::ManaSpendMode::AnyColor);
        let mut game = game(); let protected = card(&mut game, A, Zone::Hand, false);
        for player in [B, C] { card(&mut game, player, Zone::Hand, player == B); card(&mut game, player, Zone::Hand, false); }
        let (_, owner) = enter(&mut game, &definition); let mut dm = Choices::default(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.calls, vec![B, C]); assert_eq!(game.player(A).unwrap().hand, vec![protected]);
        assert_eq!(game.player(B).unwrap().hand.len(), 1); assert_eq!(game.player(C).unwrap().hand.len(), 1);
        let members = game.linked_exile_pair_members(&owner).unwrap().to_vec(); assert_eq!(members.len(), 2);
        for member in members { assert!(may_play(&game, member, A)); assert!(!may_play(&game, member, B)); assert!(!game.is_face_down(member)); }
        assert!(game.effect_store.mana_spend_effects.permissions.is_empty());
    }
}

#[test]
fn empty_hands_and_a_source_that_left_before_resolution_create_no_invented_choice_or_member() {
    for definition in definitions() { for absent in [false, true] {
        let mut game = game(); for _ in 0..2 { card(&mut game, C, Zone::Hand, false); }
        let (source, owner) = enter(&mut game, &definition);
        if absent { game.move_object_by_game_rule(source, Zone::Graveyard).unwrap(); }
        let mut dm = Choices::default(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.calls, if absent { vec![] } else { vec![C] });
        assert_eq!(game.exile.len(), usize::from(!absent));
        if !absent { assert_eq!(game.linked_exile_pair_members(&owner).unwrap().len(), 1); }
    } }
}

#[test]
fn playing_uses_exact_mana_and_the_remaining_card_returns_to_its_original_hand() {
    for definition in definitions() {
        let mut game = game(); card(&mut game, B, Zone::Hand, false); card(&mut game, C, Zone::Hand, true);
        let (source, owner) = enter(&mut game, &definition); resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        let members = game.linked_exile_pair_members(&owner).unwrap().to_vec();
        let spell = members.iter().copied().find(|id| !game.object(*id).unwrap().is_land()).unwrap();
        let land = members.iter().copied().find(|id| game.object(*id).unwrap().is_land()).unwrap();
        assert!(actions(&game, spell, A).is_empty()); game.player_mut(A).unwrap().mana_pool.red = 1;
        assert!(actions(&game, spell, A).iter().any(|action| matches!(action, LegalAction::CastSpell { casting_method: CastingMethod::ExactPermission { .. }, .. })));
        play(&mut game, spell); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0); resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        let extra = card(&mut game, B, Zone::Exile, true); game.add_exiled_with_source_link(source, extra);
        assert!(!may_play(&game, extra, A)); let stable = game.object(land).unwrap().stable_id;
        game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
        drain_pending_trigger_events_with_dm(&mut game, &mut TriggerQueue::new(), &mut Choices::default()).unwrap();
        let returned = game.find_object_by_stable_id(stable).unwrap(); assert_eq!(game.object(returned).unwrap().zone, Zone::Hand);
        assert!(game.player(C).unwrap().hand.contains(&returned)); assert_eq!(game.object(extra).unwrap().zone, Zone::Exile);
    }
}

#[test]
fn live_control_and_ability_scope_are_distinct_from_return_duration_and_card_incarnation() {
    for definition in definitions() {
        let mut game = game(); card(&mut game, B, Zone::Hand, true); let (source, owner) = enter(&mut game, &definition);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap(); let member = game.linked_exile_pair_members(&owner).unwrap()[0];
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::gain_control(source, A, source, B)); game.refresh_continuous_state().unwrap();
        assert!(!may_play(&game, member, A)); assert!(may_play(&game, member, B));
        execute_effect(&mut game, &Effect::phase_out(ChooseSpec::SpecificObject(source)), &mut EffectContext::new(source, B, &mut SelectFirstDecisionMaker)).unwrap();
        assert_eq!(game.object(member).unwrap().zone, Zone::Exile); assert!(!may_play(&game, member, B));
        execute_effect(&mut game, &Effect::phase_in(ChooseSpec::SpecificObject(source)), &mut EffectContext::new(source, B, &mut SelectFirstDecisionMaker)).unwrap();
        assert!(may_play(&game, member, B));
        let loss = game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, B, EffectTarget::Specific(source),
            Modification::RemoveAllAbilities)); game.refresh_continuous_state().unwrap();
        assert!(!may_play(&game, member, B)); game.effect_store.continuous_effects.remove_effect(loss); game.refresh_continuous_state().unwrap(); assert!(may_play(&game, member, B));
        let hand = game.move_object_by_game_rule(member, Zone::Hand).unwrap(); let returned = game.move_object_by_game_rule(hand, Zone::Exile).unwrap();
        assert!(!may_play(&game, returned, B));
    }
}

#[test]
fn all_returns_are_registered_before_a_replacement_addition_removes_the_source() {
    for definition in definitions() {
        let mut game = game(); let first = card(&mut game, B, Zone::Hand, true); card(&mut game, B, Zone::Hand, true);
        card(&mut game, C, Zone::Hand, true); card(&mut game, C, Zone::Hand, true);
        let (source, _) = enter(&mut game, &definition);
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(first), Some(Zone::Hand), Some(Zone::Exile)),
            ReplacementAction::Additionally(vec![Effect::destroy(ChooseSpec::SpecificObject(source))])));
        let mut dm = Choices::default(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        drain_pending_trigger_events_with_dm(&mut game, &mut TriggerQueue::new(), &mut Choices::default()).unwrap();
        assert_eq!(dm.calls, vec![B, C]); assert!(game.exile.is_empty()); assert!(game.object(source).is_none());
        assert_eq!(game.player(B).unwrap().hand.len(), 2); assert_eq!(game.player(C).unwrap().hand.len(), 2);
    }
}

#[test]
fn pending_choice_and_failed_replacement_restore_every_participant_before_native_recovery() {
    for definition in definitions() { for pause in [false, true] {
        let mut game = game(); let first = card(&mut game, B, Zone::Hand, true); card(&mut game, B, Zone::Hand, true);
        card(&mut game, C, Zone::Hand, true); card(&mut game, C, Zone::Hand, true);
        let (source, owner) = enter(&mut game, &definition); let next = game.next_object_id_counter();
        let replacement = (!pause).then(|| game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(first), Some(Zone::Hand), Some(Zone::Exile)),
            ReplacementAction::Additionally(vec![Effect::gain_life(3), Effect::lose_life(ironsmith::effect::Value::X)]))));
        let mut dm = Choices { pause: pause.then_some(C), ..Default::default() };
        let result = resolve_stack_entry_with(&mut game, &mut dm); if pause { assert!(result.is_ok()); assert!(dm.pending); } else { assert!(result.is_err()); }
        assert_eq!(game.player(B).unwrap().hand.len(), 2); assert_eq!(game.player(C).unwrap().hand.len(), 2);
        assert!(game.exile.is_empty()); assert_eq!(game.player(A).unwrap().life, 20); assert_eq!(game.next_object_id_counter(), next);
        assert!(game.linked_exile_pair_members(&owner).unwrap().is_empty()); assert_eq!(game.stack.len(), 1);
        game = game.clone(); if let Some(replacement) = replacement { game.effect_store.replacement_effects.remove_effect(replacement); }
        resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap(); assert_eq!(game.linked_exile_pair_members(&owner).unwrap().len(), 2);
    } }
}

#[test]
fn full_canonical_reparse_keeps_both_the_pair_and_the_mana_rider() {
    for definition in definitions() {
        let rendered = ironsmith_text::canonical_compiled_lines(&definition).join("\n");
        let text = format!("Mana cost: {{3}}{{B}}\nType: Creature — Horror\nPower/Toughness: 2/4\n{rendered}");
        let (restored, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Intellect Devourer", &text, false));
        let restored = restored.unwrap_or_else(|error| panic!("{rendered}: {error}")); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let spec = restored.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Static(ability) => ability.grant_spec(), _ => None }).unwrap();
        assert!(spec.linked_exile_pair.is_some()); assert_eq!(spec.cast_mana_spend_mode, ironsmith_core::value_model::ManaSpendMode::AnyColor);
    }
}

#[test]
fn redirected_original_and_a_copied_entry_keep_membership_and_return_receipts_exact() {
    for definition in definitions() {
        let mut game = game(); let redirected = card(&mut game, B, Zone::Hand, true); card(&mut game, B, Zone::Hand, true);
        card(&mut game, C, Zone::Hand, true); card(&mut game, C, Zone::Hand, true);
        let (source, owner) = enter(&mut game, &definition);
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(redirected), Some(Zone::Hand), Some(Zone::Exile)),
            ReplacementAction::ChangeDestination(Zone::Graveyard)));
        let target = game.stack[0].target_id();
        execute_effect(&mut game, &Effect::copy_spell(ChooseSpec::SpecificObject(target)), &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)).unwrap();
        game = game.clone(); resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.linked_exile_pair_members(&owner).unwrap().len(), 1);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap(); assert_eq!(game.linked_exile_pair_members(&owner).unwrap().len(), 3);
        assert_eq!(game.player(B).unwrap().graveyard.len(), 1); assert!(game.player(B).unwrap().hand.is_empty()); assert!(game.player(C).unwrap().hand.is_empty());
        game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
        drain_pending_trigger_events_with_dm(&mut game, &mut TriggerQueue::new(), &mut Choices::default()).unwrap();
        assert_eq!(game.player(B).unwrap().hand.len(), 1); assert_eq!(game.player(C).unwrap().hand.len(), 2);
        assert_eq!(game.player(B).unwrap().graveyard.len(), 1); assert!(game.exile.is_empty());
    }
}

#[test]
fn replacement_prefix_exiles_never_become_this_instructions_return_receipts() {
    for definition in definitions() { for replace_with_selected in [false, true] {
        let mut game = game(); let selected = card(&mut game, B, Zone::Hand, true); card(&mut game, B, Zone::Hand, true);
        card(&mut game, C, Zone::Hand, true); card(&mut game, C, Zone::Hand, true);
        let unrelated = card(&mut game, A, Zone::Graveyard, true);
        let replacement_card = if replace_with_selected { selected } else { unrelated };
        let stable = game.object(replacement_card).unwrap().stable_id;
        let (source, owner) = enter(&mut game, &definition);
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(selected), Some(Zone::Hand), Some(Zone::Exile)),
            ReplacementAction::Instead(vec![Effect::exile(ChooseSpec::SpecificObject(replacement_card))])));
        let mut dm = Choices::default(); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.calls, vec![B, C]);
        let prefix_arrival = game.find_object_by_stable_id(stable).unwrap(); assert_eq!(game.object(prefix_arrival).unwrap().zone, Zone::Exile);
        let members = game.linked_exile_pair_members(&owner).unwrap(); assert_eq!(members.len(), 1);
        assert!(!members.contains(&prefix_arrival)); assert!(!may_play(&game, prefix_arrival, A));
        game = game.clone(); game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
        drain_pending_trigger_events_with_dm(&mut game, &mut TriggerQueue::new(), &mut Choices::default()).unwrap();
        assert_eq!(game.object(prefix_arrival).unwrap().zone, Zone::Exile, "the replacement instruction supplied no until duration");
        assert_eq!(game.player(B).unwrap().hand.len(), if replace_with_selected { 1 } else { 2 });
        assert_eq!(game.player(C).unwrap().hand.len(), 2);
    } }
}

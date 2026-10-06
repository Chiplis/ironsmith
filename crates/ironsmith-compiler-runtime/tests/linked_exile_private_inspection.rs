//! Full frozen Kheru body, authored and UNRUN. Inspection entitlement is
//! durable per player/card; playing remains an active exact-acquisition grant.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
use ironsmith::decision::{AttackerDeclaration, DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{SelectObjectsContext, ViewCardsContext};
use ironsmith::effects::ExecutionError;
use ironsmith::game_loop::{apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ObjectFilter, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);
const D: PlayerId = PlayerId::from_index(3);
fn body() -> String {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/linked_exile_static_permissions.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Kheru Mind-Eater").unwrap();
    format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(),
        row["oracle_text"].as_str().unwrap())
}
fn definitions() -> [CardDefinition; 2] {
    let text = body();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Kheru Mind-Eater", &text, false));
    let direct = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Kheru Mind-Eater", &text, false));
    let (artifact, _) = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap(); let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn producer(definition: &CardDefinition) -> &Ability {
    definition.abilities.iter().find(|ability| matches!(ability.kind, AbilityKind::Triggered(_))).unwrap()
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20); main(&mut game, A); game
}
fn main(game: &mut GameState, player: PlayerId) {
    game.turn.phase = Phase::FirstMain; game.turn.step = None;
    game.turn.active_player = player; game.turn.priority_player = Some(player); game.combat = None;
}
fn hand(game: &mut GameState, player: PlayerId, land: bool) -> ObjectId {
    let text = if land { "Type: Land" } else { "Mana cost: {W}\nType: Sorcery\nYou gain 1 life." };
    let definition = compile_to_runtime_definition("Private member", text, false).unwrap();
    game.create_object_from_definition(&definition, player, Zone::Hand)
}
#[derive(Default)]
struct Choices { pick: Option<ObjectId>, chooser: Option<PlayerId>, pause: bool, pending: bool, questions: usize, views: Vec<(PlayerId, bool)> }
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool { true }
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(pick) = self.pick {
            assert!(context.candidates.iter().any(|candidate| candidate.id == pick && candidate.legal));
            assert_eq!(Some(context.player), self.chooser); self.questions += 1;
            if self.pause { self.pending = true; return vec![]; }
            vec![pick]
        } else { SelectFirstDecisionMaker.decide_objects(game, context) }
    }
    fn view_cards(&mut self, _: &GameState, viewer: PlayerId, _: &[ObjectId], context: &ViewCardsContext) {
        self.views.push((viewer, context.public));
    }
}
fn combat(game: &mut GameState, source: ObjectId) {
    game.remove_summoning_sickness(source); game.untap(source); game.turn.active_player = A;
    game.turn.phase = Phase::Combat; game.turn.step = Some(Step::DeclareAttackers);
    let mut combat = CombatState::default(); let mut queue = TriggerQueue::new();
    apply_attacker_declarations(game, &mut combat, &mut queue,
        &[AttackerDeclaration { creature: source, target: AttackTarget::Player(B) }]).unwrap();
    assert!(queue.entries.is_empty()); game.combat = Some(combat.clone()); game.turn.step = Some(Step::CombatDamage);
    let events = ironsmith::game_loop::execute_combat_damage_step_with_dm(game, &combat, false, &mut SelectFirstDecisionMaker);
    ironsmith::game_loop::queue_combat_damage_triggers(game, &events, &mut queue);
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
}
fn pending(game: &mut GameState, definition: &CardDefinition, land: bool) -> (ObjectId, ObjectId, Choices) {
    let source = game.create_object_from_definition(definition, A, Zone::Battlefield);
    let card = hand(game, B, land); combat(game, source); assert_eq!(game.stack.len(), 1);
    (source, card, Choices { pick: Some(card), chooser: Some(B), ..Default::default() })
}
fn resolve(game: &mut GameState, dm: &mut Choices) { resolve_stack_entry_with(game, dm).unwrap(); }
fn exile(game: &mut GameState, definition: &CardDefinition) -> (ObjectId, ObjectId) {
    let (source, card, mut dm) = pending(game, definition, false); let stable = game.object(card).unwrap().stable_id;
    resolve(game, &mut dm); (source, game.find_object_by_stable_id(stable).unwrap())
}
fn may_play(game: &GameState, card: ObjectId, player: PlayerId) -> bool {
    game.effect_store.grant_registry.card_can_play_from_zone(game, card, Zone::Exile, player)
}
fn look(game: &GameState, card: ObjectId, player: PlayerId) -> bool { game.can_player_look_at_face_down_exiled_card(card, player) }
fn control(game: &mut GameState, source: ObjectId, player: PlayerId) {
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::gain_control(source, A, source, player));
    game.refresh_continuous_state().unwrap();
}
fn legal(game: &GameState, member: ObjectId, player: PlayerId) -> bool {
    compute_legal_actions(game, player).unwrap().iter().any(|action| match action {
        LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } => *spell_id == member,
        LegalAction::PlayLand { land_id } => *land_id == member, _ => false,
    })
}

fn play(game: &mut GameState, member: ObjectId) {
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| match action {
        LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } => *spell_id == member,
        LegalAction::PlayLand { land_id } => *land_id == member, _ => false,
    }).unwrap();
    let mut queue = TriggerQueue::new(); let mut state = ironsmith::game_loop::PriorityLoopState::new(4);
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(game, &mut queue, &mut state,
        &ironsmith::game_loop::PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker).unwrap();
    for _ in 0..32 {
        if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending play has a decision"); };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(game, &mut queue, &mut state,
            &context, &mut SelectFirstDecisionMaker).unwrap();
    }
    assert!(!state.has_pending_action()); assert!(!game.exile.contains(&member));
}

#[test]
fn full_body_retains_menace_paired_private_choice_and_combined_static_authority() {
    for definition in definitions() {
        assert_eq!(definition.abilities.len(), 3);
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
            AbilityKind::Static(ability) if ability.id() == ironsmith::static_abilities::StaticAbilityId::Menace)));
        let AbilityKind::Triggered(trigger) = &producer(&definition).kind else { unreachable!() };
        let pair = trigger.effects.linked_exile_pair.unwrap();
        let instructions = trigger.effects.all_effects(); assert_eq!(instructions.len(), 2);
        let choose = instructions[0].downcast_ref::<ironsmith::effects::ChooseObjectsEffect>().unwrap();
        assert_eq!(choose.zone, Some(Zone::Hand)); assert!(!choose.is_search);
        let exile = instructions[1].downcast_ref::<ironsmith::effects::ExileEffect>().unwrap();
        assert!(exile.face_down && exile.exclude_prior_zone_viewers); assert!(!exile.source_controller_may_look);
        let spec = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.grant_spec(), _ => None,
        }).unwrap();
        assert!(spec.requires_linked_exile_pair && spec.may_look_at_linked_exile); assert_eq!(spec.linked_exile_pair, Some(pair));
    }
}

#[test]
fn damaged_player_chooses_but_only_active_inspector_gets_persistent_access_and_play() {
    for definition in definitions() { for land in [false, true] {
        let mut game = game(); let decoy = hand(&mut game, A, false); let other = hand(&mut game, B, false);
        let (source, victim, mut dm) = pending(&mut game, &definition, land); let stable = game.object(victim).unwrap().stable_id;
        resolve(&mut game, &mut dm); let member = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(dm.questions, 1); assert!(dm.views.iter().all(|(_, public)| !public));
        assert_eq!(game.object(decoy).unwrap().zone, Zone::Hand); assert_eq!(game.object(other).unwrap().zone, Zone::Hand);
        assert_eq!(game.player(B).unwrap().life, 19); assert!(game.is_face_down(member));
        assert!(look(&game, member, A)); for player in [B, C, D] { assert!(!look(&game, member, player)); }
        assert!(may_play(&game, member, A)); assert!(!may_play(&game, member, B));
        assert!(!legal(&game, member, A)); main(&mut game, A);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 2);
        assert_eq!(legal(&game, member, A), land, "the private permission supplies neither free nor any-color payment");
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::White, 1); assert!(legal(&game, member, A));
        let unrelated = game.move_object_by_game_rule(other, Zone::Exile).unwrap(); game.set_face_down(unrelated);
        game.add_exiled_with_source_link(source, unrelated);
        assert!(!look(&game, unrelated, A)); assert!(!may_play(&game, unrelated, A));
        play(&mut game, member);
        if land {
            let played = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(played).unwrap().zone, Zone::Battlefield); assert!(!game.is_face_down(played));
        } else {
            let cast = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(cast).unwrap().zone, Zone::Stack); assert!(!game.is_face_down(cast));
            resolve(&mut game, &mut Choices::default()); assert_eq!(game.player(A).unwrap().life, 21);
        }
    }}
}

#[test]
fn inspection_survives_loss_but_new_viewers_require_an_active_matching_reader() {
    for definition in definitions() {
        let mut game = game(); let (source, member) = exile(&mut game, &definition);
        // No view action is requested anywhere: entitlement itself persists.
        control(&mut game, source, C);
        assert!(look(&game, member, A)); assert!(look(&game, member, C)); assert!(!look(&game, member, B));
        assert!(!may_play(&game, member, A)); assert!(may_play(&game, member, C));
        let removal = game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, C,
            EffectTarget::Specific(source), Modification::RemoveAllAbilities));
        game.refresh_continuous_state().unwrap(); control(&mut game, source, D);
        assert!(!look(&game, member, D)); assert!(look(&game, member, A)); assert!(look(&game, member, C));
        assert!(!may_play(&game, member, D));
        game.effect_store.continuous_effects.remove_effect(removal); game.refresh_continuous_state().unwrap();
        assert!(look(&game, member, D)); assert!(may_play(&game, member, D));
        let hand = game.move_object_by_game_rule(source, Zone::Hand).unwrap();
        for player in [A, C, D] { assert!(look(&game, member, player)); assert!(!may_play(&game, member, player)); }
        let returned = game.move_object_by_game_rule(hand, Zone::Battlefield).unwrap(); control(&mut game, returned, B);
        assert!(!look(&game, member, B)); assert!(!may_play(&game, member, B));
        let hand = game.move_object_by_game_rule(member, Zone::Hand).unwrap();
        let reexiled = game.move_object_by_game_rule(hand, Zone::Exile).unwrap(); game.set_face_down(reexiled);
        for player in [A, B, C, D] { assert!(!look(&game, reexiled, player)); }
    }
}

#[test]
fn late_trigger_uses_current_static_controller_and_absent_reader_grants_nobody() {
    for definition in definitions() { for mode in 0..3 {
        let mut game = game(); let (source, victim, mut dm) = pending(&mut game, &definition, false);
        let stable = game.object(victim).unwrap().stable_id;
        let mut removal = None;
        match mode {
            0 => control(&mut game, source, C),
            1 => { removal = Some(game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, A,
                EffectTarget::Specific(source), Modification::RemoveAllAbilities))); game.refresh_continuous_state().unwrap(); }
            _ => { game.move_object_by_game_rule(source, Zone::Hand).unwrap(); }
        }
        assert_eq!(game.stack[0].controller, A); game = game.clone(); resolve(&mut game, &mut dm);
        let member = game.find_object_by_stable_id(stable).unwrap(); assert!(game.is_face_down(member));
        for player in [A, B, C, D] { assert_eq!(look(&game, member, player), mode == 0 && player == C); }
        if let Some(removal) = removal {
            game.effect_store.continuous_effects.remove_effect(removal); game.refresh_continuous_state().unwrap();
            assert!(look(&game, member, A)); assert!(!look(&game, member, B));
        }
    }}
}

#[test]
fn phasing_preserves_prior_inspection_but_never_creates_an_inactive_players_right() {
    for definition in definitions() {
        let mut game = game(); let (source, member) = exile(&mut game, &definition);
        game.phase_out(source); control(&mut game, source, C);
        assert!(look(&game, member, A)); assert!(!look(&game, member, C)); assert!(!may_play(&game, member, A));
        game.phase_in(source); game.refresh_continuous_state().unwrap();
        assert!(look(&game, member, C)); assert!(look(&game, member, A)); assert!(may_play(&game, member, C));
    }
}

#[test]
fn borrowed_producer_does_not_entitle_the_hosts_printed_reader_or_the_hand_chooser() {
    for definition in definitions() {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let donor = game.create_object_from_definition(&definition, A, Zone::Exile);
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, A, EffectTarget::Specific(source),
            Modification::CopyTriggeredAbilities { filter: ObjectFilter::specific(donor), exclude_source_name: false, exclude_source_id: true }));
        game.refresh_continuous_state().unwrap();
        hand(&mut game, B, false); hand(&mut game, B, false); combat(&mut game, source); assert_eq!(game.stack.len(), 2);
        let owners = game.stack.iter().map(|entry| entry.linked_exile_owner.clone().unwrap()).collect::<Vec<_>>();
        resolve(&mut game, &mut Choices::default()); resolve(&mut game, &mut Choices::default());
        for owner in owners {
            let member = game.linked_exile_pair_members(&owner).unwrap()[0];
            let printed = matches!(owner.acquisition, ironsmith::linked_exile::LinkedExileAcquisition::Printed);
            assert_eq!(look(&game, member, A), printed); assert_eq!(may_play(&game, member, A), printed);
            assert!(!look(&game, member, B));
        }
    }
}

#[test]
fn pending_selection_and_failed_added_effect_restore_entitlements_and_native_retry() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions() { for pending_choice in [false, true] {
        let mut game = game(); let (source, victim, mut dm) = pending(&mut game, &definition, false);
        let owner = game.stack[0].linked_exile_owner.clone().unwrap(); let stable = game.object(victim).unwrap().stable_id;
        let replacement = if !pending_choice { Some(game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(victim), Some(Zone::Hand), Some(Zone::Exile)),
            ReplacementAction::Additionally(vec![Effect::gain_life(3), Effect::lose_life(ironsmith::effect::Value::X)])))) } else { None };
        let next_id = game.next_object_id_counter(); dm.pause = pending_choice;
        let result = resolve_stack_entry_with(&mut game, &mut dm);
        if pending_choice { assert!(result.is_ok()); assert!(dm.pending); }
        else { assert!(matches!(result, Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ExecutionError::UnresolvableValue(_))))); }
        assert_eq!(game.object(victim).unwrap().zone, Zone::Hand); assert!(game.exile.is_empty());
        assert!(game.linked_exile_pair_members(&owner).unwrap().is_empty()); assert_eq!(game.next_object_id_counter(), next_id);
        assert_eq!(game.player(A).unwrap().life, 20); assert_eq!(game.stack.len(), 1);
        game = game.clone(); if let Some(replacement) = replacement { game.effect_store.replacement_effects.remove_effect(replacement); }
        let mut retry = Choices { pick: Some(victim), chooser: Some(B), ..Default::default() }; resolve(&mut game, &mut retry);
        let member = game.find_object_by_stable_id(stable).unwrap(); assert_eq!(game.linked_exile_pair_members(&owner).unwrap(), &[member]);
        assert!(look(&game, member, A)); assert!(!look(&game, member, B)); assert_eq!(retry.questions, 1);
    }}
}

#[test]
fn native_recovery_retains_known_viewers_without_inventing_new_imported_authority() {
    for definition in definitions() {
        let mut game = game(); let (source, member) = exile(&mut game, &definition); let saved = game.clone();
        game.replace_exiled_with_source_links(std::collections::HashMap::from([(source, vec![member])]));
        assert!(look(&game, member, A)); assert!(!look(&game, member, C));
        main(&mut game, A); assert!(matches!(compute_legal_actions(&game, A), Err(ExecutionError::IncompleteEvidence(_))));
        game = saved; control(&mut game, source, C);
        assert!(look(&game, member, A)); assert!(look(&game, member, C)); assert!(may_play(&game, member, C));
    }
}

#[test]
fn old_wire_effect_and_grant_defaults_do_not_change_the_artifact_checksum() {
    for text in ["Type: Sorcery\nExile target creature.", "Type: Enchantment\nYou may play lands from your graveyard."] {
        let (artifact, _) = compile_to_artifact("Legacy inspection defaults", text, false).unwrap();
        let bytes = artifact.to_json().unwrap(); let json = std::str::from_utf8(&bytes).unwrap();
        assert!(!json.contains("\"exclude_prior_zone_viewers\"")); assert!(!json.contains("\"may_look_at_linked_exile\""));
        let restored = CompiledCardArtifact::from_json(&bytes).unwrap(); restored.validate().unwrap();
        assert_eq!(restored.payload_checksum, artifact.payload_checksum); assert_eq!(restored.to_json().unwrap(), bytes);
    }
}


#[test]
fn empty_hand_and_replaced_destination_are_known_empty_pairs_without_inspection() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions() { for empty_hand in [false, true] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let victim = (!empty_hand).then(|| hand(&mut game, B, false));
        if let Some(victim) = victim {
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(victim), Some(Zone::Hand), Some(Zone::Exile)),
                ReplacementAction::ChangeDestination(Zone::Graveyard)));
        }
        combat(&mut game, source); let owner = game.stack[0].linked_exile_owner.clone().unwrap();
        resolve(&mut game, &mut Choices { pick: victim, chooser: Some(B), ..Default::default() });
        assert!(game.exile.is_empty()); assert!(game.linked_exile_pair_members(&owner).unwrap().is_empty());
        main(&mut game, A); compute_legal_actions(&game, A).unwrap();
    }}
}

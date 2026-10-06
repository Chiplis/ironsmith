//! Frozen full Colfenor's Plans body, authored and UNRUN. The complete source
//! includes private exile play, the skipped draw step and the global turn cap.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::ContinuousEffect;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn body() -> String {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/linked_exile_static_permissions.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Colfenor's Plans").unwrap();
    format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap())
}
fn definitions() -> [CardDefinition; 2] {
    let text = body();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Colfenor's Plans", &text, false));
    let direct = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Colfenor's Plans", &text, false));
    let (artifact, _) = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap(); let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
    game.turn.turn_number = 2; main(&mut game, A); game
}
fn main(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player; game.turn.priority_player = Some(player); game.turn.phase = Phase::FirstMain; game.turn.step = None;
}
fn mana(game: &mut GameState, player: PlayerId) {
    game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Black, 20);
    game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::White, 20);
}
fn card(game: &mut GameState, player: PlayerId, zone: Zone, land: bool) -> ObjectId {
    let text = if land { "Type: Land" } else { "Mana cost: {W}\nType: Instant\nYou gain 1 life." };
    let definition = compile_to_runtime_definition("Pool card", text, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}
fn library(game: &mut GameState, player: PlayerId, count: usize) -> Vec<ObjectId> {
    (0..count).map(|index| card(game, player, Zone::Library, index % 3 == 0)).collect()
}
fn action(game: &GameState, player: PlayerId, id: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, player).unwrap().into_iter().find(|action| match action {
        LegalAction::CastSpell { spell_id, .. } => *spell_id == id,
        LegalAction::PlayLand { land_id } => *land_id == id, _ => false,
    })
}
fn play(game: &mut GameState, player: PlayerId, id: ObjectId) {
    game.turn.priority_player = Some(player);
    let action = action(game, player, id).expect("real play/cast is legal");
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(4);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker).unwrap();
    for _ in 0..32 {
        if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending play has a decision"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut SelectFirstDecisionMaker).unwrap();
    }
    assert!(!state.has_pending_action());
}
fn resolve(game: &mut GameState) { resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap(); }
fn enter(game: &mut GameState, definition: &CardDefinition, cast: bool) -> (ObjectId, ironsmith::linked_exile::LinkedExileOwner) {
    let source = game.create_object_from_definition(definition, A, Zone::Hand); let stable = game.object(source).unwrap().stable_id;
    if cast { mana(game, A); play(game, A, source); resolve(game); }
    else {
        execute_effect(game, &Effect::move_to_zone(ChooseSpec::SpecificObject(source), Zone::Battlefield, false),
            &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)).unwrap();
    }
    let source = game.find_object_by_stable_id(stable).unwrap();
    let mut queue = TriggerQueue::new(); drain_pending_trigger_events(game, &mut queue);
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.stack.len(), 1); let owner = game.stack[0].linked_exile_owner.clone().unwrap(); (source, owner)
}
fn may_play(game: &GameState, card: ObjectId, player: PlayerId) -> bool {
    game.effect_store.grant_registry.card_can_play_from_zone(game, card, Zone::Exile, player)
}
fn look(game: &GameState, card: ObjectId, player: PlayerId) -> bool { game.can_player_look_at_face_down_exiled_card(card, player) }
fn control(game: &mut GameState, source: ObjectId, player: PlayerId) {
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::gain_control(source, A, source, player));
    game.refresh_continuous_state().unwrap();
}

#[test]
fn whole_body_retains_both_independent_restrictions_and_the_exact_private_pair() {
    for definition in definitions() {
        assert_eq!(definition.abilities.len(), 4);
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(trigger) => Some(trigger), _ => None,
        }).unwrap();
        let pair = trigger.effects.linked_exile_pair.unwrap();
        let instructions = trigger.effects.all_effects();
        let exile = instructions[0].downcast_ref::<ironsmith::effects::ExileTopOfLibraryEffect>().unwrap();
        assert!(exile.face_down); assert_eq!(exile.count, ironsmith::effect::Value::Fixed(7));
        let spec = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.grant_spec(), _ => None,
        }).unwrap();
        assert_eq!(spec.linked_exile_pair, Some(pair)); assert!(spec.may_look_at_linked_exile);
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap(); assert!(game.player_skips_draw_step(A)); assert!(!game.player_skips_draw_step(B));
        assert!(game.current_abilities(source).unwrap().iter().any(|ability| matches!(&ability.kind,
            AbilityKind::Static(ability) if ability.rule_restriction_parts().is_some())));
    }
}

#[test]
fn seven_card_receipts_use_available_top_cards_and_inspection_is_independent_of_cast_capacity() {
    for definition in definitions() { for count in [0, 3, 10] {
        let mut game = game(); let originals = library(&mut game, A, count); let other = library(&mut game, B, 2);
        let (source, owner) = enter(&mut game, &definition, true); game = game.clone(); resolve(&mut game);
        let members = game.linked_exile_pair_members(&owner).unwrap(); assert_eq!(members.len(), count.min(7));
        assert_eq!(game.player(A).unwrap().library.len(), count.saturating_sub(7));
        assert_eq!(game.player(B).unwrap().library, other);
        assert!(originals.iter().take(count.saturating_sub(7)).all(|id| game.object(*id).is_some_and(|object| object.zone == Zone::Library)));
        assert_eq!(game.turn_store.turn_history.spells_cast_by_player(A), 1);
        for member in members {
            assert!(game.is_face_down(*member)); assert!(look(&game, *member, A)); assert!(!look(&game, *member, B));
            assert!(may_play(&game, *member, A)); assert!(!may_play(&game, *member, B));
            if !game.object(*member).unwrap().is_land() { assert!(action(&game, A, *member).is_none()); }
        }
        let unrelated = card(&mut game, A, Zone::Exile, false); game.set_face_down(unrelated); game.add_exiled_with_source_link(source, unrelated);
        assert!(!look(&game, unrelated, A)); assert!(!may_play(&game, unrelated, A));
    }}
}

#[test]
fn plans_itself_counts_and_the_cap_uses_real_turn_history_across_all_origins() {
    for definition in definitions() {
        let mut game = game(); library(&mut game, A, 10);
        let (source, owner) = enter(&mut game, &definition, true); resolve(&mut game);
        let members = game.linked_exile_pair_members(&owner).unwrap().to_vec();
        let land = *members.iter().find(|id| game.object(**id).unwrap().is_land()).unwrap();
        let spells = members.iter().copied().filter(|id| !game.object(*id).unwrap().is_land()).collect::<Vec<_>>();
        assert!(action(&game, A, spells[0]).is_none()); assert!(action(&game, A, land).is_some()); play(&mut game, A, land);
        assert_eq!(game.turn_store.turn_history.spells_cast_by_player(A), 1, "a land play never consumes a spell allowance");
        for _ in 0..4 { game.next_turn(); }
        assert_eq!(game.turn.active_player, A); main(&mut game, A); mana(&mut game, A);
        assert_eq!(game.turn_store.turn_history.spells_cast_by_player(A), 0);
        play(&mut game, A, spells[0]); resolve(&mut game);
        assert_eq!(game.turn_store.turn_history.spells_cast_by_player(A), 1);
        let hand_spell = card(&mut game, A, Zone::Hand, false);
        assert!(action(&game, A, spells[1]).is_none()); assert!(action(&game, A, hand_spell).is_none());
        let opponent_spell = card(&mut game, B, Zone::Hand, false); mana(&mut game, B); game.turn.priority_player = Some(B);
        assert!(action(&game, B, opponent_spell).is_some());
        game.move_object_by_game_rule(source, Zone::Hand).unwrap(); game.refresh_continuous_state().unwrap(); game.turn.priority_player = Some(A);
        assert!(action(&game, A, hand_spell).is_some()); assert!(!may_play(&game, spells[1], A)); assert!(look(&game, spells[1], A));
    }
}

#[test]
fn noncast_entry_allows_one_spell_but_earlier_casts_still_count_when_the_restriction_enters() {
    for definition in definitions() { for earlier_cast in [false, true] {
        let mut game = game(); library(&mut game, A, 10); mana(&mut game, A);
        if earlier_cast { let first = card(&mut game, A, Zone::Hand, false); play(&mut game, A, first); resolve(&mut game); }
        let (_, owner) = enter(&mut game, &definition, false); resolve(&mut game);
        let spell = *game.linked_exile_pair_members(&owner).unwrap().iter().find(|id| !game.object(**id).unwrap().is_land()).unwrap();
        assert_eq!(action(&game, A, spell).is_some(), !earlier_cast);
        assert_eq!(game.turn_store.turn_history.spells_cast_by_player(A), u32::from(earlier_cast));
    }}
}

#[test]
fn library_actor_is_captured_but_draw_skip_play_and_new_inspection_follow_live_control() {
    for definition in definitions() {
        let mut game = game(); library(&mut game, A, 10); let b_library = library(&mut game, B, 2);
        let (source, owner) = enter(&mut game, &definition, true); control(&mut game, source, B);
        assert_eq!(game.stack[0].controller, A); resolve(&mut game);
        let members = game.linked_exile_pair_members(&owner).unwrap().to_vec(); assert_eq!(members.len(), 7);
        assert_eq!(game.player(A).unwrap().library.len(), 3); assert_eq!(game.player(B).unwrap().library, b_library);
        assert!(!game.player_skips_draw_step(A)); assert!(game.player_skips_draw_step(B));
        for member in &members { assert!(look(&game, *member, B)); assert!(!look(&game, *member, A)); assert!(may_play(&game, *member, B)); }
        game.turn.phase = Phase::Beginning; game.turn.step = Some(Step::Draw);
        let before = game.player(A).unwrap().hand.len(); ironsmith::turn::execute_draw_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), before + 1);
        game.next_turn(); assert_eq!(game.turn.active_player, B); game.turn.step = Some(Step::Draw);
        let before = game.player(B).unwrap().hand.len(); assert!(ironsmith::turn::execute_draw_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap().is_empty());
        assert_eq!(game.player(B).unwrap().hand.len(), before);
        game.move_object_by_game_rule(source, Zone::Hand).unwrap(); game.refresh_continuous_state().unwrap();
        assert!(!game.player_skips_draw_step(B)); ironsmith::turn::execute_draw_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(B).unwrap().hand.len(), before + 1);
        for member in members { assert!(look(&game, member, B)); assert!(!may_play(&game, member, B)); }
    }
}

#[test]
fn unknown_additional_scope_does_not_gain_a_pair_from_the_known_restriction_whitelist() {
    let definition = compile_to_runtime_definition("Additional scope",
        format!("{}\nWhenever this enchantment leaves the battlefield, draw a card.", body()), false).unwrap();
    for ability in definition.abilities {
        match ability.kind {
            AbilityKind::Triggered(trigger) => assert!(trigger.effects.linked_exile_pair.is_none()),
            AbilityKind::Static(ability) => if let Some(spec) = ability.grant_spec() { assert!(spec.linked_exile_pair.is_none()); },
            _ => {}
        }
    }
}


#[test]
fn failed_seven_card_addition_leaves_no_viewer_receipt_for_reused_arrival_ids() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions() {
        let mut game = game(); let library = library(&mut game, A, 10);
        let (source, owner) = enter(&mut game, &definition, true);
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(library[5]), Some(Zone::Library), Some(Zone::Exile)),
            ReplacementAction::Additionally(vec![Effect::gain_life(3), Effect::lose_life(ironsmith::effect::Value::X)])));
        let next_id = game.next_object_id_counter();
        assert!(matches!(resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker),
            Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ironsmith::effects::ExecutionError::UnresolvableValue(_)))));
        assert_eq!(game.next_object_id_counter(), next_id); assert_eq!(game.player(A).unwrap().library, library);
        assert!(game.exile.is_empty()); assert!(game.linked_exile_pair_members(&owner).unwrap().is_empty());
        assert_eq!(game.player(A).unwrap().life, 20); assert_eq!(game.stack.len(), 1);
        assert_eq!(game.turn_store.turn_history.spells_cast_by_player(A), 1);
        game = game.clone(); game.effect_store.replacement_effects.remove_effect(replacement);
        // The retry reuses the rolled-back arrival allocator with no current
        // inspector. Any leaked per-card receipt would reveal those cards.
        let removal = game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, A,
            EffectTarget::Specific(source), Modification::RemoveAllAbilities));
        game.refresh_continuous_state().unwrap(); resolve(&mut game);
        let members = game.linked_exile_pair_members(&owner).unwrap().to_vec(); assert_eq!(members.len(), 7);
        for member in &members { assert!(!look(&game, *member, A)); assert!(!may_play(&game, *member, A)); }
        game.effect_store.continuous_effects.remove_effect(removal); game.refresh_continuous_state().unwrap();
        for member in members { assert!(look(&game, member, A)); assert!(may_play(&game, member, A)); }
    }
}

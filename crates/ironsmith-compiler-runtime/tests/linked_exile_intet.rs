//! Complete frozen Intet scenarios, source-authored and UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
use ironsmith::decision::{AttackerDeclaration, DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, ViewCardsContext};
use ironsmith::effects::{EffectContext, ExecutionError, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_attacker_declarations, apply_priority_response_with_dm,
    apply_decision_context_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn body() -> String {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/linked_exile_static_permissions.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Intet, the Dreamer").unwrap();
    format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(), row["oracle_text"].as_str().unwrap())
}
fn definitions() -> [CardDefinition; 2] {
    let text = body();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Intet, the Dreamer", &text, false));
    let direct = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Intet, the Dreamer", &text, false));
    let (artifact, _) = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap(); let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap(); assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState { let mut game = GameState::new(vec!["A".into(), "B".into()], 20); main(&mut game, A); game }
fn main(game: &mut GameState, player: PlayerId) {
    game.turn.phase = Phase::FirstMain; game.turn.step = None; game.turn.active_player = player; game.turn.priority_player = Some(player); game.combat = None;
}
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, land: bool) -> ObjectId {
    let text = if land { "Type: Land" } else { "Mana cost: {5}{W}{W}\nType: Sorcery\nYou gain 1 life." };
    game.create_object_from_definition(&compile_to_runtime_definition("Permission candidate", text, false).unwrap(), owner, zone)
}
fn mana(game: &mut GameState) { game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 1); game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2); }
#[derive(Default)]
struct Choices { decline: bool, pause: bool, pending: bool, offered: usize, views: usize }
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool { true }
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        assert_eq!(ctx.player, A); self.offered += 1;
        if self.pause { self.pending = true; return false; }
        !self.decline
    }
    fn view_cards(&mut self, _: &GameState, _: PlayerId, _: &[ObjectId], _: &ViewCardsContext) { self.views += 1; }
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
fn pending(game: &mut GameState, definition: &CardDefinition, land: bool) -> (ObjectId, ObjectId) {
    let source = game.create_object_from_definition(definition, A, Zone::Battlefield); let victim = card(game, A, Zone::Library, land);
    combat(game, source); assert_eq!(game.stack.len(), 1); (source, victim)
}
fn resolve(game: &mut GameState, dm: &mut Choices) { resolve_stack_entry_with(game, dm).unwrap(); }
fn prepared(game: &mut GameState, definition: &CardDefinition, land: bool) -> (ObjectId, ObjectId) {
    let (source, victim) = pending(game, definition, land); let stable = game.object(victim).unwrap().stable_id;
    mana(game); let mut dm = Choices::default(); resolve(game, &mut dm);
    assert_eq!(dm.offered, 1); assert_eq!(dm.views, 0, "permission does not require an immediate view");
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    (source, game.find_object_by_stable_id(stable).unwrap())
}
fn look(game: &GameState, card: ObjectId, player: PlayerId) -> bool { game.can_player_look_at_face_down_exiled_card(card, player) }
fn actions(game: &GameState, card: ObjectId, player: PlayerId) -> Vec<LegalAction> {
    compute_legal_actions(game, player).unwrap().into_iter().filter(|action| match action {
        LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } => *spell_id == card,
        LegalAction::PlayLand { land_id } => *land_id == card, _ => false,
    }).collect()
}
fn play(game: &mut GameState, card: ObjectId) {
    let action = actions(game, card, A).into_iter().next().unwrap();
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker).unwrap();
    for _ in 0..32 {
        if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending play has a decision"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut SelectFirstDecisionMaker).unwrap();
    }
    assert!(!state.has_pending_action()); assert!(!game.exile.contains(&card));
}
fn instructions(definition: &CardDefinition) -> (ironsmith::effects::ExileTopOfLibraryEffect, ironsmith::effects::LookAtObjectsEffect, ironsmith::effects::GrantPlayTaggedEffect) {
    fn visit(effect: &Effect, all: &mut Vec<Effect>) { all.push(effect.clone()); effect.visit_child_effects(&mut |child| visit(child, all)); }
    let mut all = Vec::new();
    for ability in &definition.abilities { if let AbilityKind::Triggered(trigger) = &ability.kind { for effect in trigger.effects.all_effects() { visit(effect, &mut all); } } }
    (all.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::ExileTopOfLibraryEffect>().cloned()).unwrap(),
     all.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::LookAtObjectsEffect>().cloned()).unwrap(),
     all.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::GrantPlayTaggedEffect>().cloned()).unwrap())
}
#[test]
fn complete_body_keeps_flying_paid_top_exile_private_inspection_and_free_spell_or_land() {
    for definition in definitions() { for land in [false, true] {
        assert_eq!(definition.abilities.len(), 2); assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(ability) if ability.id() == ironsmith::static_abilities::StaticAbilityId::Flying)));
        let (exile, inspection, grant) = instructions(&definition);
        assert!(exile.face_down && inspection.permit_while_exiled); assert!(grant.allow_land);
        assert_eq!(grant.duration, ironsmith_core::GrantPlayTaggedDuration::ForAsLongAsSourceOnBattlefield);
        assert!(grant.alternative_cost.as_ref().unwrap().costs().is_empty());
        assert_eq!(inspection.filter.tagged_constraints[0].tag, grant.tag); assert!(exile.moved_tags.contains(&grant.tag));
        let mut game = game(); let other_top = card(&mut game, B, Zone::Library, false);
        let (source, member) = prepared(&mut game, &definition, land); assert_eq!(game.player(B).unwrap().life, 14);
        assert_eq!(game.object(other_top).unwrap().zone, Zone::Library); assert!(game.is_face_down(member));
        assert!(look(&game, member, A)); assert!(!look(&game, member, B)); assert!(actions(&game, member, A).is_empty());
        let unrelated = card(&mut game, A, Zone::Exile, land); game.set_face_down(unrelated); game.add_exiled_with_source_link(source, unrelated);
        main(&mut game, A); assert!(!actions(&game, member, A).is_empty()); assert!(actions(&game, unrelated, A).is_empty()); assert!(!look(&game, unrelated, A));
        assert!(game.effect_store.grant_registry.grants.iter().filter(|grant| grant.target_id == Some(member)).all(|grant| grant.target_stable_id.is_none()));
        if land { let mut limit = game.clone(); limit.player_mut(A).unwrap().lands_played_this_turn = 1; assert!(actions(&limit, member, A).is_empty()); }
        play(&mut game, member); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        if !land { resolve(&mut game, &mut Choices::default()); assert_eq!(game.player(A).unwrap().life, 21); }
    }}
}
#[test]
fn optional_payment_decline_or_wrong_color_never_exiles_or_grants() {
    for definition in definitions() { for decline in [false, true] {
        let mut game = game(); let (_, victim) = pending(&mut game, &definition, false);
        if decline { mana(&mut game); } else { game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 3); }
        let before = game.player(A).unwrap().mana_pool.clone(); let mut dm = Choices { decline, ..Default::default() };
        resolve(&mut game, &mut dm); assert_eq!(game.object(victim).unwrap().zone, Zone::Library);
        assert!(game.exile.is_empty()); assert!(game.effect_store.grant_registry.grants.is_empty()); assert_eq!(dm.views, 0);
        assert_eq!(game.player(A).unwrap().mana_pool, before);
    }}
}
#[test]
fn fixed_beneficiary_survives_control_and_ability_loss_but_phasing_ends_play_permanently() {
    for definition in definitions() { for phase in [false, true] {
        let mut game = game(); let (source, member) = prepared(&mut game, &definition, false);
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::gain_control(source, A, source, B));
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, A, EffectTarget::Specific(source), Modification::RemoveAllAbilities));
        game.refresh_continuous_state().unwrap(); main(&mut game, A);
        assert!(!actions(&game, member, A).is_empty()); assert!(!look(&game, member, B));
        main(&mut game, B); assert!(actions(&game, member, B).is_empty()); main(&mut game, A);
        if phase { game.phase_out(source); game.phase_in(source); }
        else { let hand = game.move_object_by_game_rule(source, Zone::Hand).unwrap(); game.move_object_by_game_rule(hand, Zone::Battlefield).unwrap(); }
        assert!(actions(&game, member, A).is_empty()); assert!(look(&game, member, A));
        game = game.clone(); assert!(actions(&game, member, A).is_empty()); assert!(look(&game, member, A));
    }}
}
#[test]
fn absent_or_phased_source_at_resolution_still_exiles_and_grants_only_inspection() {
    for definition in definitions() { for phase in [false, true] {
        let mut game = game(); let (source, victim) = pending(&mut game, &definition, false); let stable = game.object(victim).unwrap().stable_id;
        mana(&mut game);
        if phase { game.phase_out(source); } else { game.move_object_by_game_rule(source, Zone::Hand).unwrap(); }
        resolve(&mut game, &mut Choices::default()); let member = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(member).unwrap().zone, Zone::Exile); assert!(look(&game, member, A));
        if phase { game.phase_in(source); }
        main(&mut game, A); assert!(actions(&game, member, A).is_empty());
    }}
}
#[test]
fn copied_pending_trigger_keeps_old_source_and_resolution_card_identity_after_blink() {
    for definition in definitions() {
        let mut game = game(); let (source, _) = pending(&mut game, &definition, false); card(&mut game, A, Zone::Library, false);
        let target = game.stack[0].target_id();
        execute_effect(&mut game, &Effect::copy_spell(ChooseSpec::SpecificObject(target)), &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)).unwrap();
        let hand = game.move_object_by_game_rule(source, Zone::Hand).unwrap(); let new_source = game.move_object_by_game_rule(hand, Zone::Battlefield).unwrap();
        assert_ne!(source, new_source); game = game.clone();
        for _ in 0..2 { mana(&mut game); resolve(&mut game, &mut Choices::default()); }
        assert_eq!(game.exile.len(), 2); main(&mut game, A);
        for member in game.exile.clone() { assert!(look(&game, member, A)); assert!(actions(&game, member, A).is_empty()); }
    }
}
#[test]
fn exile_incarnation_does_not_reacquire_inspection_or_play_after_a_round_trip() {
    for definition in definitions() {
        let mut game = game(); let (_, member) = prepared(&mut game, &definition, false);
        let hand = game.move_object_by_game_rule(member, Zone::Hand).unwrap(); let new_member = game.move_object_by_game_rule(hand, Zone::Exile).unwrap();
        game.set_face_down(new_member); main(&mut game, A);
        assert!(!look(&game, new_member, A)); assert!(actions(&game, new_member, A).is_empty());
    }
}
#[test]
fn pending_payment_or_failed_replacement_rolls_back_and_native_recovery_retries() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions() { for pause in [false, true] {
        let mut game = game(); let (source, victim) = pending(&mut game, &definition, false); mana(&mut game);
        let before = game.player(A).unwrap().mana_pool.clone(); let next_id = game.next_object_id_counter(); let stable = game.object(victim).unwrap().stable_id;
        let replacement = (!pause).then(|| game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(victim), Some(Zone::Library), Some(Zone::Exile)),
            ReplacementAction::Additionally(vec![Effect::gain_life(3), Effect::lose_life(ironsmith::effect::Value::X)]))));
        let mut dm = Choices { pause, ..Default::default() }; let result = resolve_stack_entry_with(&mut game, &mut dm);
        if pause { assert!(result.is_ok()); assert!(dm.pending); } else { assert!(matches!(result, Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ExecutionError::UnresolvableValue(_))))); }
        assert_eq!(game.object(victim).unwrap().zone, Zone::Library); assert!(game.exile.is_empty()); assert_eq!(game.player(A).unwrap().mana_pool, before);
        assert_eq!(game.next_object_id_counter(), next_id); assert_eq!(game.player(A).unwrap().life, 20); assert_eq!(game.stack.len(), 1);
        assert!(game.effect_store.grant_registry.grants.is_empty()); game = game.clone();
        if let Some(replacement) = replacement { game.effect_store.replacement_effects.remove_effect(replacement); }
        resolve(&mut game, &mut Choices::default()); let member = game.find_object_by_stable_id(stable).unwrap();
        assert!(look(&game, member, A)); main(&mut game, A); assert!(!actions(&game, member, A).is_empty());
    }}
}
#[test]
fn executed_empty_or_replaced_exile_overwrites_an_earlier_native_tag_without_adopting_it() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions() { for replaced in [false, true] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let (exile, inspection, grant) = instructions(&definition); let foreign = card(&mut game, A, Zone::Exile, false); game.set_face_down(foreign);
        if replaced { let victim = card(&mut game, A, Zone::Library, false);
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(victim), Some(Zone::Library), Some(Zone::Exile)),
                ReplacementAction::ChangeDestination(Zone::Graveyard))); }
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(foreign).unwrap(), &game);
        let mut dm = Choices::default(); let mut ctx = EffectContext::new(source, A, &mut dm); ctx.set_tagged_objects(grant.tag.clone(), vec![snapshot]);
        execute_effect(&mut game, &Effect::new(exile), &mut ctx).unwrap(); assert!(ctx.get_tagged_all(&grant.tag).unwrap().is_empty());
        execute_effect(&mut game, &Effect::new(inspection), &mut ctx).unwrap(); execute_effect(&mut game, &Effect::new(grant), &mut ctx).unwrap();
        assert!(!look(&game, foreign, A)); main(&mut game, A); assert!(actions(&game, foreign, A).is_empty());
    }}
}
#[test]
fn missing_native_antecedent_fails_and_default_wire_shape_stays_compatible() {
    for definition in definitions() {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let (_, inspection, grant) = instructions(&definition);
        for effect in [Effect::new(inspection), Effect::new(grant)] {
            assert!(matches!(execute_effect(&mut game, &effect, &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)), Err(ExecutionError::IncompleteEvidence(_))));
        }
    }
    for text in ["Type: Sorcery\nLook at target face-down creature.", "Type: Sorcery\nExile the top card of your library. You may play that card this turn."] {
        let (artifact, _) = compile_to_artifact("Legacy inspection wire", text, false).unwrap(); let bytes = artifact.to_json().unwrap();
        let json = std::str::from_utf8(&bytes).unwrap(); assert!(!json.contains("\"permit_while_exiled\"")); assert!(!json.contains("\"battlefield_source\""));
        let restored = CompiledCardArtifact::from_json(&bytes).unwrap(); restored.validate().unwrap(); assert_eq!(restored.to_json().unwrap(), bytes);
    }
    let old = ironsmith_core::LookAtObjectsEffect::new(ObjectFilter::default(), ironsmith::target::PlayerFilter::You, ironsmith::target::PlayerFilter::You);
    let wire = serde_json::to_value(&old).unwrap(); assert!(wire.get("permit_while_exiled").is_none());
    assert!(!serde_json::from_value::<ironsmith_core::LookAtObjectsEffect>(wire).unwrap().permit_while_exiled);
}
#[test]
fn canonical_complete_body_reparses_with_both_distinct_permission_lifetimes() {
    for definition in definitions() {
        let rendered = ironsmith_text::canonical_compiled_lines(&definition).join("\n");
        let text = format!("Mana cost: {{3}}{{G}}{{U}}{{R}}\nType: Legendary Creature — Dragon\nPower/Toughness: 6/6\n{rendered}");
        let (restored, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Intet, the Dreamer", text, false));
        let restored = restored.unwrap_or_else(|error| panic!("{rendered}: {error}")); assert!(!loss.is_lossy(), "{rendered}: {}", loss.reasons_text());
        let (_, inspection, grant) = instructions(&restored); assert!(inspection.permit_while_exiled);
        assert_eq!(grant.duration, ironsmith_core::GrantPlayTaggedDuration::ForAsLongAsSourceOnBattlefield);
        let mut game = game(); let (source, member) = prepared(&mut game, &restored, false);
        game.phase_out(source); game.phase_in(source); main(&mut game, A); assert!(look(&game, member, A)); assert!(actions(&game, member, A).is_empty());
    }
}

#[test]
fn a_control_change_while_pending_keeps_the_original_payer_library_and_beneficiary() {
    for definition in definitions() {
        let mut game = game(); let (source, victim) = pending(&mut game, &definition, false); let stable = game.object(victim).unwrap().stable_id;
        let foreign_top = card(&mut game, B, Zone::Library, false);
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::gain_control(source, A, source, B));
        game.refresh_continuous_state().unwrap(); mana(&mut game); resolve(&mut game, &mut Choices::default());
        let member = game.find_object_by_stable_id(stable).unwrap(); assert_eq!(game.object(foreign_top).unwrap().zone, Zone::Library);
        main(&mut game, A); assert!(!actions(&game, member, A).is_empty()); assert!(look(&game, member, A)); assert!(!look(&game, member, B));
        main(&mut game, B); assert!(actions(&game, member, B).is_empty());
    }
}

#[test]
fn a_late_failure_restores_already_created_play_and_private_entitlements() {
    for definition in definitions() {
        let mut game = game(); let (source, victim) = pending(&mut game, &definition, false); mana(&mut game);
        let stable = game.object(victim).unwrap().stable_id; let original = game.stack[0].ability_effects.clone().unwrap();
        game.stack[0].ability_effects.as_mut().unwrap().push(Effect::lose_life(ironsmith::effect::Value::X));
        assert!(matches!(resolve_stack_entry_with(&mut game, &mut Choices::default()),
            Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ExecutionError::UnresolvableValue(_)))));
        assert_eq!(game.object(victim).unwrap().zone, Zone::Library); assert!(game.effect_store.grant_registry.grants.is_empty());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 3);
        game.stack[0].ability_effects = Some(original); let mut retry = game.clone(); resolve(&mut retry, &mut Choices::default());
        let member = retry.find_object_by_stable_id(stable).unwrap(); assert!(look(&retry, member, A));
        main(&mut retry, A); assert!(!actions(&retry, member, A).is_empty());
        // Reuse the rolled-back arrival id with only the producer. Any leaked
        // private receipt would grant inspection even though no reader ran.
        let (exile, _, _) = instructions(&definition); game.stack.clear();
        execute_effect(&mut game, &Effect::new(exile), &mut EffectContext::new(source, A, &mut Choices::default())).unwrap();
        let uninspected = game.find_object_by_stable_id(stable).unwrap(); assert_eq!(uninspected, member); assert!(!look(&game, uninspected, A));
    }
}

#[test]
fn free_casting_still_pays_the_spells_additional_cost() {
    for definition in definitions() {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let spell = compile_to_runtime_definition("Additional cost candidate", "Mana cost: {5}{W}{W}\nType: Sorcery\nAs an additional cost to cast this spell, pay 3 life.\nYou gain 1 life.", false).unwrap();
        let victim = game.create_object_from_definition(&spell, A, Zone::Library); let stable = game.object(victim).unwrap().stable_id;
        combat(&mut game, source); mana(&mut game); resolve(&mut game, &mut Choices::default());
        let member = game.find_object_by_stable_id(stable).unwrap(); main(&mut game, A); play(&mut game, member);
        assert_eq!(game.player(A).unwrap().life, 17); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve(&mut game, &mut Choices::default()); assert_eq!(game.player(A).unwrap().life, 18);
    }
}

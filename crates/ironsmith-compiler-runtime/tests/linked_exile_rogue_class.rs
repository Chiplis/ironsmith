//! Exact frozen Class scopes, source-authored and UNRUN.
//! Blind play by a new controller without inspection remains held; no full-card credit.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
use ironsmith::decision::{AttackerDeclaration, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, ExecutionError, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_attacker_declarations, apply_priority_response_with_dm,
    apply_decision_context_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{Phase, Step};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn body() -> String {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/linked_exile_static_permissions.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Rogue Class").unwrap();
    format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap())
}
fn definitions() -> [CardDefinition; 2] {
    let source = body(); let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Rogue Class", &source, false));
    let direct = direct.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Rogue Class", &source, false));
    let (artifact, _) = artifact.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text()); artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap(); assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState { let mut game = GameState::new(vec!["A".into(), "B".into()], 20); main(&mut game, A); game }
fn main(game: &mut GameState, player: PlayerId) { game.turn.phase = Phase::FirstMain; game.turn.step = None; game.turn.active_player = player; game.turn.priority_player = Some(player); game.combat = None; }
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, text: &str) -> ObjectId {
    game.create_object_from_definition(&compile_to_runtime_definition("Class witness", text, false).unwrap(), owner, zone)
}
fn creature(game: &mut GameState, owner: PlayerId) -> ObjectId { card(game, owner, Zone::Battlefield, "Type: Creature — Rogue\nPower/Toughness: 2/3") }
fn top(game: &mut GameState, land: bool) -> ObjectId { card(game, B, Zone::Library, if land { "Type: Land" } else { "Mana cost: {W}\nType: Sorcery\nYou gain 1 life." }) }
fn producer(definition: &CardDefinition) -> &Ability { definition.abilities.iter().find(|ability| matches!(ability.kind, AbilityKind::Triggered(_))).unwrap() }
fn pair(definition: &CardDefinition) -> ironsmith_core::LinkedExilePair { let AbilityKind::Triggered(trigger) = &producer(definition).kind else { unreachable!() }; trigger.effects.linked_exile_pair.unwrap() }
fn combat(game: &mut GameState, attacker: ObjectId, defender: PlayerId) {
    game.remove_summoning_sickness(attacker); game.untap(attacker); game.turn.active_player = game.current_controller(attacker).unwrap();
    game.turn.phase = Phase::Combat; game.turn.step = Some(Step::DeclareAttackers);
    let mut combat = CombatState::default(); let mut queue = TriggerQueue::new();
    apply_attacker_declarations(game, &mut combat, &mut queue, &[AttackerDeclaration { creature: attacker, target: AttackTarget::Player(defender) }]).unwrap();
    assert!(queue.entries.is_empty()); game.combat = Some(combat.clone()); game.turn.step = Some(Step::CombatDamage);
    let events = ironsmith::game_loop::execute_combat_damage_step_with_dm(game, &combat, false, &mut SelectFirstDecisionMaker);
    ironsmith::game_loop::queue_combat_damage_triggers(game, &events, &mut queue);
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
}
fn resolve(game: &mut GameState) { resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap(); }
fn prepared(game: &mut GameState, definition: &CardDefinition, land: bool) -> (ObjectId, ObjectId, ObjectId) {
    let class = game.create_object_from_definition(definition, A, Zone::Battlefield); let attacker = creature(game, A);
    let victim = top(game, land); let stable = game.object(victim).unwrap().stable_id; combat(game, attacker, B);
    assert_eq!(game.stack.len(), 1); assert_eq!(game.stack[0].linked_exile_owner.as_ref().unwrap().host, class); resolve(game);
    (class, attacker, game.find_object_by_stable_id(stable).unwrap())
}
fn actions(game: &GameState, player: PlayerId) -> Vec<LegalAction> { compute_legal_actions(game, player).unwrap() }
fn play_action(game: &GameState, member: ObjectId, player: PlayerId) -> Option<LegalAction> {
    actions(game, player).into_iter().find(|action| match action { LegalAction::CastSpell { spell_id, .. } => *spell_id == member, LegalAction::PlayLand { land_id } => *land_id == member, _ => false })
}
fn may_play(game: &GameState, member: ObjectId, player: PlayerId) -> bool { game.effect_store.grant_registry.card_can_play_from_zone(game, member, Zone::Exile, player) }
fn look(game: &GameState, member: ObjectId, player: PlayerId) -> bool { game.can_player_look_at_face_down_exiled_card(member, player) }
fn level_index(game: &GameState, class: ObjectId, level: u32) -> usize {
    game.current_abilities(class).unwrap().iter().position(|ability| matches!(&ability.kind, AbilityKind::Activated(activated)
        if activated.keyword == Some(ironsmith_core::ActivatedAbilityKeyword::ClassLevel(level)))).unwrap()
}
fn action(game: &mut GameState, action: LegalAction) {
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker).unwrap();
    for _ in 0..32 { if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending action has a decision"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut SelectFirstDecisionMaker).unwrap();
    }
    assert!(!state.has_pending_action());
}
fn level(game: &mut GameState, class: ObjectId, level: u32) {
    main(game, A); let index = level_index(game, class, level);
    game.player_mut(A).unwrap().mana_pool.blue = 1; game.player_mut(A).unwrap().mana_pool.black = 1;
    game.player_mut(A).unwrap().mana_pool.colorless = level - 1;
    let expected = LegalAction::ActivateAbility { source: class, ability_index: index };
    assert!(actions(game, A).contains(&expected)); action(game, expected); resolve(game);
    assert_eq!(game.class_level(class), level); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
}
fn complete_levels(game: &mut GameState, class: ObjectId) { level(game, class, 2); level(game, class, 3); }

#[test]
fn complete_body_retains_base_private_exile_paid_class_levels_menace_and_exact_play() {
    for definition in definitions() { for land in [false, true] {
        assert_eq!(definition.abilities.len(), 5); assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert_eq!(pair(&definition), pair(&compile_to_runtime_definition("Independent label", body(), false).unwrap()));
        let mut game = game(); let own_top = card(&mut game, A, Zone::Library, "Type: Land");
        let (class, attacker, member) = prepared(&mut game, &definition, land); let opponent = creature(&mut game, B);
        assert_eq!(game.player(B).unwrap().life, 18); assert_eq!(game.object(own_top).unwrap().zone, Zone::Library);
        assert!(game.is_face_down(member)); assert!(look(&game, member, A)); assert!(!look(&game, member, B));
        assert_eq!(game.class_level(class), 1); assert!(!may_play(&game, member, A));
        assert!(!game.object_has_ability(attacker, &ironsmith::static_abilities::StaticAbility::menace()));
        level(&mut game, class, 2); assert!(game.object_has_ability(attacker, &ironsmith::static_abilities::StaticAbility::menace()));
        assert!(!game.object_has_ability(opponent, &ironsmith::static_abilities::StaticAbility::menace())); assert!(!may_play(&game, member, A));
        level(&mut game, class, 3); assert!(may_play(&game, member, A)); assert!(!may_play(&game, member, B));
        let unrelated = card(&mut game, B, Zone::Exile, "Type: Land"); game.set_face_down(unrelated); game.add_exiled_with_source_link(class, unrelated);
        assert!(!may_play(&game, unrelated, A)); assert!(!look(&game, unrelated, A));
        if !land { assert!(play_action(&game, member, A).is_none()); game.player_mut(A).unwrap().mana_pool.red = 1; }
        let selected = play_action(&game, member, A).unwrap();
        if !land { assert!(matches!(&selected, LegalAction::CastSpell { casting_method: CastingMethod::ExactPermission { .. }, .. })); }
        action(&mut game, selected); assert!(!game.exile.contains(&member));
        if !land { assert_eq!(game.player(A).unwrap().mana_pool.total(), 0); resolve(&mut game); assert_eq!(game.player(A).unwrap().life, 21); }
    } }
}

#[test]
fn level_announcements_recheck_previous_level_sorcery_timing_and_colored_price_without_mutation() {
    for definition in definitions() { for failure in 0..4 {
        let mut game = game(); let class = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let requested = if failure == 0 { 3 } else { 2 }; let index = level_index(&game, class, requested);
        game.player_mut(A).unwrap().mana_pool.blue = if failure == 1 { 3 } else { 1 };
        game.player_mut(A).unwrap().mana_pool.black = u32::from(failure != 1);
        game.player_mut(A).unwrap().mana_pool.colorless = 2;
        if failure == 2 { game.turn.phase = Phase::Combat; game.turn.step = Some(Step::BeginCombat); }
        if failure == 3 { main(&mut game, B); }
        let forged = LegalAction::ActivateAbility { source: class, ability_index: index };
        assert!(!actions(&game, A).contains(&forged)); let mana = game.player(A).unwrap().mana_pool.clone(); let next = game.next_object_id_counter();
        assert!(apply_priority_response_with_dm(&mut game, &mut TriggerQueue::new(), &mut PriorityLoopState::new(2),
            &PriorityResponse::PriorityAction(forged), &mut SelectFirstDecisionMaker).is_err());
        assert_eq!(game.class_level(class), 1); assert_eq!(game.player(A).unwrap().mana_pool, mana); assert!(game.stack.is_empty()); assert_eq!(game.next_object_id_counter(), next);
    } }
}

#[test]
fn inspection_and_live_authority_have_distinct_controller_ability_phase_and_incarnation_lifetimes() {
    for definition in definitions() { for boundary in 0..5 {
        let mut game = game(); let (class, _, member) = prepared(&mut game, &definition, true); complete_levels(&mut game, class);
        match boundary {
            0 => { game.effect_store.continuous_effects.add_effect(ContinuousEffect::gain_control(class, A, class, B)); game.refresh_continuous_state().unwrap();
                assert!(!may_play(&game, member, A)); assert!(may_play(&game, member, B)); assert!(look(&game, member, A)); assert!(!look(&game, member, B)); }
            1 => { game.phase_out(class); assert!(!may_play(&game, member, A)); assert!(look(&game, member, A)); game.phase_in(class); assert!(may_play(&game, member, A)); }
            2 => { let loss = game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(class, A, EffectTarget::Specific(class), Modification::RemoveAllAbilities));
                game.refresh_continuous_state().unwrap(); assert!(!may_play(&game, member, A)); assert!(look(&game, member, A));
                game.effect_store.continuous_effects.remove_effect(loss); game.refresh_continuous_state().unwrap(); assert!(may_play(&game, member, A)); }
            3 => { let hand = game.move_object_by_game_rule(member, Zone::Hand).unwrap(); let returned = game.move_object_by_game_rule(hand, Zone::Exile).unwrap(); game.set_face_down(returned);
                assert!(!may_play(&game, returned, A)); assert!(!look(&game, returned, A)); }
            _ => { let hand = game.move_object_by_game_rule(class, Zone::Hand).unwrap(); let returned = game.move_object_by_game_rule(hand, Zone::Battlefield).unwrap();
                complete_levels(&mut game, returned); assert!(!may_play(&game, member, A)); assert!(look(&game, member, A)); }
        }
    } }
}

#[test]
fn pending_copies_keep_the_original_pair_and_inspector_after_source_blink_or_control_change() {
    for definition in definitions() { for blink in [false, true] {
        let mut game = game(); let class = game.create_object_from_definition(&definition, A, Zone::Battlefield); let attacker = creature(&mut game, A);
        top(&mut game, true); top(&mut game, true); combat(&mut game, attacker, B); let owner = game.stack[0].linked_exile_owner.clone().unwrap();
        let target = game.stack[0].target_id(); execute_effect(&mut game, &Effect::copy_spell(ChooseSpec::SpecificObject(target)), &mut EffectContext::new(class, A, &mut SelectFirstDecisionMaker)).unwrap();
        assert_eq!(game.stack[1].linked_exile_owner, Some(owner.clone()));
        if blink { let hand = game.move_object_by_game_rule(class, Zone::Hand).unwrap(); game.move_object_by_game_rule(hand, Zone::Battlefield).unwrap(); }
        else { game.effect_store.continuous_effects.add_effect(ContinuousEffect::gain_control(class, A, class, B)); game.refresh_continuous_state().unwrap(); }
        game = game.clone(); resolve(&mut game); resolve(&mut game); let members = game.linked_exile_pair_members(&owner).unwrap().to_vec(); assert_eq!(members.len(), 2);
        for member in members { assert!(look(&game, member, A)); assert!(!look(&game, member, B)); assert!(!may_play(&game, member, A)); }
    } }
}

#[test]
fn independent_acquisitions_do_not_cross_feed_the_level_reader_and_complete_acquisitions_do() {
    for definition in definitions() { for borrowed in [false, true] {
        let mut game = game(); let (class, attacker, first) = prepared(&mut game, &definition, true); complete_levels(&mut game, class);
        let donor = game.create_object_from_definition(&definition, A, Zone::Exile);
        let extra = if borrowed { Modification::CopyTriggeredAbilities { filter: ObjectFilter::specific(donor), exclude_source_name: false, exclude_source_id: true } }
            else { Modification::AddAbilityGeneric(producer(&definition).clone()) };
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(class, A, EffectTarget::Specific(class), extra)); game.refresh_continuous_state().unwrap();
        top(&mut game, true); top(&mut game, true); combat(&mut game, attacker, B); assert_eq!(game.stack.len(), 2);
        let owners = game.stack.iter().map(|entry| entry.linked_exile_owner.clone().unwrap()).collect::<Vec<_>>(); assert_ne!(owners[0], owners[1]); resolve(&mut game); resolve(&mut game);
        for owner in owners { for member in game.linked_exile_pair_members(&owner).unwrap() {
            assert_eq!(may_play(&game, *member, A), matches!(owner.acquisition, ironsmith::linked_exile::LinkedExileAcquisition::Printed));
        } }
        assert!(may_play(&game, first, A));
    } }
    for definition in definitions() {
        let mut game = game(); let host = card(&mut game, A, Zone::Battlefield, "Type: Enchantment — Class"); let attacker = creature(&mut game, A);
        let grantor = card(&mut game, B, Zone::Battlefield, "Type: Enchantment");
        let first = game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(grantor, B, EffectTarget::Specific(host), Modification::SetAbilities(definition.abilities.clone())));
        game.refresh_continuous_state().unwrap(); top(&mut game, true); combat(&mut game, attacker, B); let first_owner = game.stack[0].linked_exile_owner.clone().unwrap(); resolve(&mut game);
        let old = game.linked_exile_pair_members(&first_owner).unwrap()[0]; complete_levels(&mut game, host); assert!(may_play(&game, old, A));
        assert_eq!(game.class_level(grantor), 1); assert!(game.object_has_ability(attacker, &ironsmith::static_abilities::StaticAbility::menace()));
        game.effect_store.continuous_effects.remove_effect(first); game.refresh_continuous_state().unwrap(); assert!(!may_play(&game, old, A));
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(grantor, B, EffectTarget::Specific(host), Modification::SetAbilities(definition.abilities.clone()))); game.refresh_continuous_state().unwrap();
        assert!(!may_play(&game, old, A)); top(&mut game, true); combat(&mut game, attacker, B); let second_owner = game.stack[0].linked_exile_owner.clone().unwrap(); assert_ne!(first_owner, second_owner); resolve(&mut game);
        let new = game.linked_exile_pair_members(&second_owner).unwrap()[0]; assert!(may_play(&game, new, A)); assert!(!may_play(&game, old, A));
    }
}

#[test]
fn failed_exile_addition_rolls_back_members_and_private_knowledge_then_native_recovery_replays() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions() {
        let mut game = game(); let class = game.create_object_from_definition(&definition, A, Zone::Battlefield); let attacker = creature(&mut game, A); let victim = top(&mut game, true);
        combat(&mut game, attacker, B); let owner = game.stack[0].linked_exile_owner.clone().unwrap();
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(class, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(victim), Some(Zone::Library), Some(Zone::Exile)),
            ReplacementAction::Additionally(vec![Effect::gain_life(3), Effect::lose_life(ironsmith::effect::Value::X)])));
        let saved = game.clone(); let next = game.next_object_id_counter();
        assert!(matches!(resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker), Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ExecutionError::UnresolvableValue(_)))));
        assert!(game.exile.is_empty()); assert!(game.linked_exile_pair_members(&owner).unwrap().is_empty()); assert_eq!(game.player(B).unwrap().library, vec![victim]); assert_eq!(game.next_object_id_counter(), next); assert_eq!(game.player(A).unwrap().life, 20);
        game = saved; game.effect_store.replacement_effects.remove_effect(replacement); resolve(&mut game); let member = game.linked_exile_pair_members(&owner).unwrap()[0]; assert!(look(&game, member, A));
        complete_levels(&mut game, class); let recovered = game.clone(); assert!(may_play(&recovered, member, A));
        game.replace_exiled_with_source_links(std::collections::HashMap::from([(class, vec![member])])); assert!(matches!(compute_legal_actions(&game, A), Err(ExecutionError::IncompleteEvidence(_))));
    }
}

#[test]
fn a_native_reader_cannot_forge_the_class_gate_and_canonical_body_reparses_independently() {
    for definition in definitions() {
        let mut game = game(); let (class, _, _) = prepared(&mut game, &definition, true); complete_levels(&mut game, class);
        let reader = game.current_abilities(class).unwrap().into_iter().find(|ability| matches!(&ability.kind, AbilityKind::Static(reader) if reader.grant_spec().is_some())).unwrap();
        let mut forged = definition.clone(); forged.abilities = vec![producer(&definition).clone(), reader];
        let mut other = self::game(); let host = other.create_object_from_definition(&forged, A, Zone::Battlefield);
        let unrelated = card(&mut other, B, Zone::Exile, "Type: Land"); other.add_exiled_with_source_link(host, unrelated);
        assert!(matches!(compute_legal_actions(&other, A), Err(ExecutionError::IncompleteEvidence(_))));
        let rendered = ironsmith_text::canonical_compiled_lines(&definition).join("\n");
        let (restored, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Rogue Class", format!("Mana cost: {{U}}{{B}}\nType: Enchantment — Class\n{rendered}"), false));
        let restored = restored.unwrap_or_else(|error| panic!("{rendered}: {error}")); assert!(!loss.is_lossy(), "{rendered}: {}", loss.reasons_text());
        let mut replay = self::game(); let (class, _, member) = prepared(&mut replay, &restored, true); complete_levels(&mut replay, class); assert!(look(&replay, member, A)); assert!(may_play(&replay, member, A));
    }
}

#[test]
fn exact_permission_keeps_land_limits_timing_colorless_costs_and_rejects_source_only_bypass() {
    for definition in definitions() { for colorless in [false, true] {
        let mut game = game(); let class = game.create_object_from_definition(&definition, A, Zone::Battlefield); let attacker = creature(&mut game, A);
        let victim = card(&mut game, B, Zone::Library, if colorless { "Mana cost: {C}\nType: Sorcery\nYou gain 1 life." } else { "Mana cost: {W}\nType: Sorcery\nYou gain 1 life." });
        let stable = game.object(victim).unwrap().stable_id; combat(&mut game, attacker, B); resolve(&mut game); let member = game.find_object_by_stable_id(stable).unwrap(); complete_levels(&mut game, class);
        game.player_mut(A).unwrap().mana_pool.red = 1;
        if colorless { assert!(play_action(&game, member, A).is_none()); game.player_mut(A).unwrap().mana_pool.colorless = 1; }
        let selected = play_action(&game, member, A).unwrap(); let mut forged = selected.clone();
        if let LegalAction::CastSpell { casting_method, .. } = &mut forged {
            *casting_method = CastingMethod::PlayFrom { source: class, zone: Zone::Exile, use_alternative: None };
        }
        let before = game.player(A).unwrap().mana_pool.clone();
        assert!(apply_priority_response_with_dm(&mut game, &mut TriggerQueue::new(), &mut PriorityLoopState::new(2),
            &PriorityResponse::PriorityAction(forged), &mut SelectFirstDecisionMaker).is_err());
        assert_eq!(game.player(A).unwrap().mana_pool, before); assert_eq!(game.object(member).unwrap().zone, Zone::Exile);
        game.turn.phase = Phase::Combat; game.turn.step = Some(Step::BeginCombat); assert!(play_action(&game, member, A).is_none());
        main(&mut game, A); action(&mut game, selected); resolve(&mut game);
    } }
    for definition in definitions() {
        let mut game = game(); let (class, _, member) = prepared(&mut game, &definition, true); complete_levels(&mut game, class);
        game.player_mut(A).unwrap().lands_played_this_turn = 1; assert!(play_action(&game, member, A).is_none());
        game.player_mut(A).unwrap().lands_played_this_turn = 0; main(&mut game, B); assert!(play_action(&game, member, A).is_none());
    }
}

#[test]
fn opponent_damage_and_empty_or_redirected_producers_do_not_invent_members_or_inspection() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions() { for redirected in [false, true] {
        let mut game = game(); let class = game.create_object_from_definition(&definition, A, Zone::Battlefield); let enemy = creature(&mut game, B);
        card(&mut game, A, Zone::Library, "Type: Land"); combat(&mut game, enemy, A); assert!(game.stack.is_empty()); assert!(game.exile.is_empty());
        let attacker = creature(&mut game, A);
        if redirected { let victim = top(&mut game, true); game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(class, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(victim), Some(Zone::Library), Some(Zone::Exile)), ReplacementAction::ChangeDestination(Zone::Graveyard))); }
        let foreign = card(&mut game, B, Zone::Exile, "Type: Land"); game.set_face_down(foreign); game.add_exiled_with_source_link(class, foreign);
        combat(&mut game, attacker, B); let owner = game.stack[0].linked_exile_owner.clone().unwrap(); resolve(&mut game);
        assert!(game.linked_exile_pair_members(&owner).unwrap().is_empty()); assert!(!look(&game, foreign, A)); complete_levels(&mut game, class); assert!(!may_play(&game, foreign, A));
    } }
}

#[test]
fn hidden_secondary_producers_or_changed_tiers_do_not_receive_class_pair_proof() {
    for source in [format!("{}\nWhenever a creature enters, exile the top card of your library.", body()), body().replace(": Level 3", ": Level 4")] {
        let definition = compile_to_runtime_definition("Unproven Class scope", source, false).unwrap();
        assert!(definition.abilities.iter().filter_map(|ability| match &ability.kind { AbilityKind::Triggered(trigger) => Some(trigger.effects.linked_exile_pair), _ => None }).all(|pair| pair.is_none()));
    }
}

fn blind_control_change(game: &mut GameState, definition: &CardDefinition, victim_text: &str) -> (ObjectId, ObjectId) {
    let class = game.create_object_from_definition(definition, A, Zone::Battlefield); let attacker = creature(game, A);
    let victim = card(game, B, Zone::Library, victim_text); let stable = game.object(victim).unwrap().stable_id;
    combat(game, attacker, B); resolve(game); let member = game.find_object_by_stable_id(stable).unwrap(); complete_levels(game, class);
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::gain_control(class, A, class, B)); game.refresh_continuous_state().unwrap(); main(game, B);
    assert!(may_play(game, member, B)); assert!(!may_play(game, member, A)); assert!(look(game, member, A)); assert!(!look(game, member, B));
    (class, member)
}
fn blind_action(game: &GameState, member: ObjectId) -> LegalAction {
    let actions = actions(game, B).into_iter().filter(|action| ironsmith::decision::legal_action_source(action) == Some(member)).collect::<Vec<_>>();
    assert_eq!(actions.len(), 2, "two uniform intents for the exact authority");
    actions.into_iter().find(|action| matches!(action, LegalAction::OpenExiledCardForPlay { .. })).unwrap()
}

#[test]
fn new_controller_sees_identical_opaque_intents_before_land_type_price_or_method_is_opened() {
    for definition in definitions() {
        let mut expected = None;
        for victim in ["Type: Land", "Mana cost: {W}\nType: Sorcery\nYou gain 1 life.", "Mana cost: {100}\nType: Sorcery\nYou gain 1 life."] {
            let mut game = game(); let (_, member) = blind_control_change(&mut game, &definition, victim);
            game.player_mut(B).unwrap().mana_pool.red = 1; game.player_mut(B).unwrap().lands_played_this_turn = 1;
            let action = blind_action(&game, member); assert!(play_action(&game, member, B).is_none());
            if let Some(expected) = &expected { assert_eq!(&action, expected); } else { expected = Some(action); }
            assert!(game.is_face_down(member)); assert!(!look(&game, member, B));
        }
    }
}

#[test]
fn blind_opening_then_ordinary_spell_or_land_uses_the_exact_class_permission() {
    for definition in definitions() { for land in [false, true] {
        let mut game = game(); let (_, member) = blind_control_change(&mut game, &definition,
            if land { "Type: Land" } else { "Mana cost: {W}\nType: Sorcery\nYou gain 1 life." });
        game.player_mut(B).unwrap().mana_pool.red = 1; let opening = blind_action(&game, member);
        let LegalAction::OpenExiledCardForPlay { permission, .. } = &opening else { unreachable!() }; let selected = permission.identity.clone();
        action(&mut game, opening); assert!(!game.exile.contains(&member));
        if land { assert_eq!(game.player(B).unwrap().lands_played_this_turn, 1); assert_eq!(game.player(B).unwrap().mana_pool.red, 1); }
        else {
            assert_eq!(game.player(B).unwrap().mana_pool.total(), 0); let spell = game.object(game.stack[0].object_id).unwrap();
            let receipt = spell.cast_play_permission.as_deref().unwrap(); assert_eq!(receipt.identity, selected); assert_eq!(receipt.origin, member); assert_eq!(receipt.player, B);
            resolve(&mut game); assert_eq!(game.player(B).unwrap().life, 19);
        }
    } }
}

#[test]
fn an_unavailable_opened_play_consumes_nothing_and_never_makes_the_public_identity_private() {
    for definition in definitions() { for land in [false, true] {
        let mut game = game(); let (_, member) = blind_control_change(&mut game, &definition,
            if land { "Type: Land" } else { "Mana cost: {100}\nType: Sorcery\nYou gain 1 life." });
        game.player_mut(B).unwrap().mana_pool.red = 1; game.player_mut(B).unwrap().lands_played_this_turn = 1;
        let opening = blind_action(&game, member); action(&mut game, opening);
        assert_eq!(game.object(member).unwrap().zone, Zone::Exile); assert!(game.is_face_down(member)); assert!(game.stack.is_empty());
        assert!(look(&game, member, A)); assert!(look(&game, member, B)); assert_eq!(game.player(B).unwrap().mana_pool.red, 1); assert_eq!(game.player(B).unwrap().lands_played_this_turn, 1);
        assert_eq!(game.turn.priority_player, Some(B)); let restored = game.clone(); assert!(restored.is_face_down(member)); assert!(look(&restored, member, B));
    } }
}

#[test]
fn unrelated_card_or_stale_grant_cannot_use_opening_to_disclose_a_private_face() {
    for definition in definitions() { for wrong_card in [false, true] {
        let mut game = game(); let (class, member) = blind_control_change(&mut game, &definition, "Type: Land");
        let unrelated = card(&mut game, A, Zone::Exile, "Mana cost: {R}\nType: Instant\nYou gain 1 life."); game.set_face_down(unrelated); game.add_exiled_with_source_link(class, unrelated);
        let mut forged = blind_action(&game, member); if let LegalAction::OpenExiledCardForPlay { card_id, permission, .. } = &mut forged {
            if wrong_card { *card_id = unrelated; } else { permission.index += 1; }
        }
        assert!(apply_priority_response_with_dm(&mut game, &mut TriggerQueue::new(), &mut PriorityLoopState::new(2),
            &PriorityResponse::PriorityAction(forged), &mut SelectFirstDecisionMaker).is_err());
        assert!(game.is_face_down(member)); assert!(game.is_face_down(unrelated)); assert!(!look(&game, unrelated, B)); assert!(!look(&game, member, B));
    } }
}

#[test]
fn unseen_class_member_can_declare_morph_and_cast_without_opening_or_private_inspection() {
    for definition in definitions() {
        let mut game = game(); let (_, member) = blind_control_change(&mut game, &definition,
            "Mana cost: {6}\nType: Creature — Human\nPower/Toughness: 4/4\nMorph {1}");
        game.player_mut(B).unwrap().mana_pool.red = 3;
        assert!(!look(&game, member, B));
        let intent = actions(&game, B).into_iter().find(|action| matches!(action,
            LegalAction::CastExiledCardFaceDown { card_id, .. } if *card_id == member)).unwrap();
        let LegalAction::CastExiledCardFaceDown { permission, .. } = &intent else { unreachable!() };
        let selected = permission.identity.clone(); action(&mut game, intent);
        let spell = game.stack[0].object_id; assert!(game.is_face_down(spell)); assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        let receipt = game.object(spell).unwrap().cast_play_permission.as_deref().unwrap();
        assert_eq!(receipt.identity, selected); assert_eq!(receipt.origin, member); assert_eq!(receipt.player, B);
        assert!(!look(&game, member, B));
    }
}

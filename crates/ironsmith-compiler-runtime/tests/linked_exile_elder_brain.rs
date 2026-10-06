//! Frozen complete Elder Brain scenarios. Source-authored and intentionally UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, ExecutionError, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_attacker_declarations, apply_blocker_declarations,
    apply_priority_response_with_dm, apply_decision_context_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
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
    let row = rows.iter().find(|row| row["name"] == "Elder Brain").unwrap();
    format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(), row["oracle_text"].as_str().unwrap())
}
fn definitions() -> [CardDefinition; 2] {
    let text = body();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Elder Brain", &text, false));
    let direct = direct.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Elder Brain", &text, false));
    let (artifact, _) = artifact.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap(); let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap(); assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState { let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20); main(&mut game); game }
fn main(game: &mut GameState) { game.turn.active_player = A; game.turn.priority_player = Some(A); game.turn.phase = Phase::FirstMain; game.turn.step = None; game.combat = None; }
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, body: &str) -> ObjectId {
    let definition = compile_to_runtime_definition("Exile candidate", body, false).unwrap(); game.create_object_from_definition(&definition, owner, zone)
}
fn library(game: &mut GameState, owner: PlayerId, count: usize) { for _ in 0..count { card(game, owner, Zone::Library, "Type: Land"); } }
fn attack(game: &mut GameState, source: ObjectId, target: AttackTarget) {
    game.remove_summoning_sickness(source); game.untap(source); game.turn.phase = Phase::Combat; game.turn.step = Some(Step::DeclareAttackers);
    let mut combat = CombatState::default(); let mut queue = TriggerQueue::new();
    apply_attacker_declarations(game, &mut combat, &mut queue, &[AttackerDeclaration { creature: source, target }]).unwrap();
    game.combat = Some(combat); put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
}
fn resolve(game: &mut GameState) { resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap(); }
fn may_play(game: &GameState, card: ObjectId, player: PlayerId) -> bool { game.effect_store.grant_registry.card_can_play_from_zone(game, card, Zone::Exile, player) }
fn actions(game: &GameState, card: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().filter(|action| match action {
        LegalAction::CastSpell { spell_id, .. } => *spell_id == card, LegalAction::PlayLand { land_id } => *land_id == card, _ => false,
    }).collect()
}
fn play(game: &mut GameState, card: ObjectId) {
    let action = actions(game, card).into_iter().next().unwrap(); let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut SelectFirstDecisionMaker).unwrap();
    for _ in 0..32 { if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending cast has a decision"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut SelectFirstDecisionMaker).unwrap();
    }
    assert!(!state.has_pending_action());
}
fn instructions(definition: &CardDefinition) -> (ironsmith::effects::DrawCardsEffect, ironsmith::effects::GrantPlayTaggedEffect) {
    fn visit(effect: &Effect, draw: &mut Option<ironsmith::effects::DrawCardsEffect>, grant: &mut Option<ironsmith::effects::GrantPlayTaggedEffect>) {
        if let Some(value) = effect.downcast_ref::<ironsmith::effects::DrawCardsEffect>() { assert!(draw.replace(value.clone()).is_none()); }
        if let Some(value) = effect.downcast_ref::<ironsmith::effects::GrantPlayTaggedEffect>() { assert!(grant.replace(value.clone()).is_none()); }
        effect.visit_child_effects(&mut |child| visit(child, draw, grant));
    }
    let mut draw = None; let mut grant = None;
    for ability in &definition.abilities { if let AbilityKind::Triggered(trigger) = &ability.kind {
        for effect in trigger.effects.all_effects() { visit(effect, &mut draw, &mut grant); }
    } }
    (draw.unwrap(), grant.unwrap())
}

#[test]
fn full_body_retains_menace_and_binds_actual_exile_count_and_selected_permission() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let (draw, grant) = instructions(&definition); assert!(grant.permission_bound_mana);
        assert_eq!(grant.mana_spend_mode, ironsmith_core::value_model::ManaSpendMode::AnyColor);
        assert!(matches!(draw.count.unhinted(), ironsmith::effect::Value::PriorEffectMetric { query, .. }
            if query.original_destination == Some(Zone::Exile)));
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(game.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Menace));
        let blocker = card(&mut game, B, Zone::Battlefield, "Type: Creature\nPower/Toughness: 1/1");
        attack(&mut game, source, AttackTarget::Player(B)); resolve(&mut game);
        game.turn.step = Some(Step::DeclareBlockers); let mut combat = game.combat.take().unwrap();
        assert!(apply_blocker_declarations(&mut game, &mut combat, &mut TriggerQueue::new(),
            &[BlockerDeclaration { blocker, blocking: source }], B).is_err());
    }
}

#[test]
fn attacked_player_draws_actual_original_arrivals_and_only_those_cards_receive_authority() {
    for definition in definitions() { for hand_size in [0, 1, 3] { for redirect in [false, true] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield); library(&mut game, B, 8);
        let unaffected = card(&mut game, C, Zone::Hand, "Type: Land");
        let cards: Vec<_> = (0..hand_size).map(|_| card(&mut game, B, Zone::Hand, "Type: Land")).collect();
        let stable: Vec<_> = cards.iter().map(|id| game.object(*id).unwrap().stable_id).collect();
        if redirect && let Some(first) = cards.first() {
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(*first), Some(Zone::Hand), Some(Zone::Exile)),
                ReplacementAction::ChangeDestination(Zone::Graveyard)));
        }
        attack(&mut game, source, AttackTarget::Player(B)); game = game.clone(); resolve(&mut game);
        let expected = hand_size - usize::from(redirect && hand_size > 0);
        assert_eq!(game.player(B).unwrap().hand.len(), expected); assert_eq!(game.player(B).unwrap().library.len(), 8 - expected);
        assert_eq!(game.player(C).unwrap().hand, vec![unaffected]); assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(game.exile.len(), expected);
        for old in stable { let member = game.find_object_by_stable_id(old).unwrap();
            assert_eq!(may_play(&game, member, A), game.object(member).unwrap().zone == Zone::Exile);
            assert!(!may_play(&game, member, B)); assert!(!may_play(&game, member, C));
        }
        assert!(game.effect_store.mana_spend_effects.permissions.is_empty());
        main(&mut game); for member in game.exile.clone() { assert!(!actions(&game, member).is_empty()); }
    } } }
}

#[test]
fn permissions_keep_ordinary_timing_land_limits_additional_costs_and_colorless_requirements() {
    for definition in definitions() {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield); library(&mut game, B, 8);
        let spell = card(&mut game, B, Zone::Hand, "Mana cost: {W}\nType: Sorcery\nAs an additional cost to cast this spell, discard a card.\nYou gain 1 life.");
        let colorless = card(&mut game, B, Zone::Hand, "Mana cost: {C}\nType: Sorcery\nYou gain 1 life.");
        let land = card(&mut game, B, Zone::Hand, "Type: Land"); let other_land = card(&mut game, B, Zone::Hand, "Type: Land");
        let ids = [spell, colorless, land, other_land].map(|id| game.object(id).unwrap().stable_id);
        attack(&mut game, source, AttackTarget::Player(B)); resolve(&mut game); main(&mut game);
        let [spell, colorless, land, other_land] = ids.map(|id| game.find_object_by_stable_id(id).unwrap());
        assert!(actions(&game, spell).is_empty()); game.player_mut(A).unwrap().mana_pool.red = 2;
        assert!(actions(&game, spell).is_empty(), "the casting rider does not waive an additional discard");
        assert!(actions(&game, colorless).is_empty(), "any color does not mean any type");
        card(&mut game, A, Zone::Hand, "Type: Land");
        assert!(actions(&game, spell).iter().any(|action| matches!(action, LegalAction::CastSpell { casting_method: CastingMethod::ExactPermission { .. }, .. })));
        game.turn.phase = Phase::Combat; game.turn.step = Some(Step::BeginCombat); assert!(actions(&game, spell).is_empty()); main(&mut game);
        play(&mut game, land); assert!(actions(&game, other_land).is_empty());
        play(&mut game, spell); assert_eq!(game.player(A).unwrap().mana_pool.red, 1); assert!(game.player(A).unwrap().hand.is_empty()); resolve(&mut game);
        game.player_mut(A).unwrap().mana_pool.colorless = 1; assert!(!actions(&game, colorless).is_empty());
    }
}

#[test]
fn pending_copied_trigger_fixes_attacked_player_and_caster_through_source_blink() {
    for definition in definitions() {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        library(&mut game, B, 8); library(&mut game, C, 8); card(&mut game, B, Zone::Hand, "Type: Land"); card(&mut game, B, Zone::Hand, "Type: Land");
        let untouched = card(&mut game, C, Zone::Hand, "Type: Land");
        attack(&mut game, source, AttackTarget::Player(B)); let target = game.stack[0].target_id();
        execute_effect(&mut game, &Effect::copy_spell(ChooseSpec::SpecificObject(target)), &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)).unwrap();
        let hand = game.move_object_by_game_rule(source, Zone::Hand).unwrap(); let returned = game.move_object_by_game_rule(hand, Zone::Battlefield).unwrap();
        assert_ne!(source, returned); game = game.clone(); resolve(&mut game); resolve(&mut game);
        assert_eq!(game.exile.len(), 4); assert_eq!(game.player(B).unwrap().hand.len(), 2); assert_eq!(game.player(C).unwrap().hand, vec![untouched]);
        main(&mut game); for member in game.exile.clone() { assert!(may_play(&game, member, A)); assert!(!may_play(&game, member, B)); }
        let member = game.exile[0]; let hand = game.move_object_by_game_rule(member, Zone::Hand).unwrap();
        let reentered = game.move_object_by_game_rule(hand, Zone::Exile).unwrap(); assert!(!may_play(&game, reentered, A));
    }
}

#[test]
fn added_exile_is_not_an_original_arrival_and_failed_addition_rolls_back_before_recovery() {
    for definition in definitions() { for fails in [false, true] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield); library(&mut game, B, 5);
        let victim = card(&mut game, B, Zone::Hand, "Type: Land"); let stable = game.object(victim).unwrap().stable_id;
        let extra = card(&mut game, C, Zone::Hand, "Type: Land"); let extra_stable = game.object(extra).unwrap().stable_id;
        let added = if fails { vec![Effect::gain_life(3), Effect::lose_life(ironsmith::effect::Value::X)] }
            else { vec![Effect::exile(ChooseSpec::SpecificObject(extra))] };
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(victim), Some(Zone::Hand), Some(Zone::Exile)), ReplacementAction::Additionally(added)));
        attack(&mut game, source, AttackTarget::Player(B)); let next = game.next_object_id_counter();
        let result = resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker);
        if fails {
            assert!(result.is_err()); assert_eq!(game.object(victim).unwrap().zone, Zone::Hand);
            assert_eq!(game.player(B).unwrap().library.len(), 5); assert_eq!(game.player(A).unwrap().life, 20);
            assert!(game.exile.is_empty()); assert!(game.effect_store.grant_registry.grants.is_empty()); assert_eq!(game.next_object_id_counter(), next);
            game = game.clone(); game.effect_store.replacement_effects.remove_effect(replacement); resolve(&mut game);
        } else { result.unwrap(); }
        assert_eq!(game.player(B).unwrap().hand.len(), 1); assert_eq!(game.player(B).unwrap().library.len(), 4);
        assert!(may_play(&game, game.find_object_by_stable_id(stable).unwrap(), A));
        assert!(!may_play(&game, game.find_object_by_stable_id(extra_stable).unwrap(), A));
    } }
}

#[test]
fn canonical_complete_body_preserves_arrival_metric_and_permission_marker() {
    for definition in definitions() {
        let rendered = ironsmith_text::canonical_compiled_lines(&definition).join("\n");
        let text = format!("Mana cost: {{5}}{{B}}{{B}}\nType: Creature — Horror\nPower/Toughness: 6/6\n{rendered}");
        let (restored, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Elder Brain", &text, false));
        let restored = restored.unwrap_or_else(|error| panic!("{rendered}: {error}")); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let (draw, grant) = instructions(&restored); assert!(grant.permission_bound_mana);
        assert!(matches!(draw.count.unhinted(), ironsmith::effect::Value::PriorEffectMetric { query, .. } if query.original_destination == Some(Zone::Exile)));
        let mut game = game(); let source = game.create_object_from_definition(&restored, A, Zone::Battlefield);
        assert!(matches!(execute_effect(&mut game, &Effect::new(draw), &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)), Err(ExecutionError::IncompleteEvidence(_))));
        assert!(matches!(execute_effect(&mut game, &Effect::new(grant), &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)), Err(ExecutionError::IncompleteEvidence(_))));
    }
}

#[test]
fn attack_must_name_a_player_and_resolved_authority_survives_control_loss_and_phasing() {
    for definition in definitions() {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let walker = card(&mut game, B, Zone::Battlefield, "Type: Planeswalker — Witness\nLoyalty: 3");
        attack(&mut game, source, AttackTarget::Planeswalker(walker)); assert!(game.stack.is_empty());
        main(&mut game); card(&mut game, B, Zone::Hand, "Type: Land"); library(&mut game, B, 2);
        attack(&mut game, source, AttackTarget::Player(B)); resolve(&mut game); let member = game.exile[0];
        game.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::gain_control(source, A, source, B));
        game.refresh_continuous_state().unwrap(); assert!(may_play(&game, member, A)); assert!(!may_play(&game, member, B));
        execute_effect(&mut game, &Effect::phase_out(ChooseSpec::SpecificObject(source)), &mut EffectContext::new(source, B, &mut SelectFirstDecisionMaker)).unwrap();
        assert!(may_play(&game, member, A)); assert!(!may_play(&game, member, B));
        let unrelated = card(&mut game, B, Zone::Exile, "Type: Land"); game.add_exiled_with_source_link(source, unrelated);
        assert!(!may_play(&game, unrelated, A));
    }
}

#[test]
fn a_card_that_leaves_during_replacement_additions_still_counts_but_gets_no_permission() {
    for definition in definitions() {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield); library(&mut game, B, 3);
        let victim = card(&mut game, B, Zone::Hand, "Type: Land"); let stable = game.object(victim).unwrap().stable_id;
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(victim), Some(Zone::Hand), Some(Zone::Exile)),
            ReplacementAction::Additionally(vec![Effect::return_all_to_hand(ObjectFilter::default().in_zone(Zone::Exile)
                .owned_by(ironsmith::target::PlayerFilter::Specific(B)))])));
        attack(&mut game, source, AttackTarget::Player(B)); resolve(&mut game);
        assert_eq!(game.player(B).unwrap().library.len(), 2); assert_eq!(game.player(B).unwrap().hand.len(), 2);
        assert!(game.exile.is_empty()); let returned = game.find_object_by_stable_id(stable).unwrap();
        assert!(!may_play(&game, returned, A)); assert!(game.effect_store.grant_registry.grants.is_empty());
    }
}

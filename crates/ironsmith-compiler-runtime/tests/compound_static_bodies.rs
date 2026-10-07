//! Seven complete frozen bodies, through direct compilation and serialized artifacts.
//! These native scenarios are authored source evidence; execution is deferred.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, check_and_apply_sbas, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::{AttachmentTarget, CounterType, ObjectKind};
use ironsmith::static_abilities::StaticAbility;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/compound_static_bodies.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let (compiled, loss) = parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let definitions = [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()];
    for definition in &definitions {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    definitions
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 50);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 10);
    game
}
fn simple(name: &str, types: &str) -> CardDefinition {
    let size = if types.contains("Creature") { "\nPower/Toughness: 2/2" } else { "" };
    compile_to_runtime_definition(name, format!("Type: {types}{size}"), false).unwrap()
}
fn object(game: &mut GameState, owner: PlayerId, zone: Zone, types: &str) -> ObjectId {
    let id = game.create_object_from_definition(&simple("Witness", types), owner, zone);
    if zone == Zone::Battlefield { game.remove_summoning_sickness(id); }
    id
}
fn refresh(game: &mut GameState) { game.refresh_continuous_state().unwrap(); }
fn settle(game: &mut GameState) {
    let mut queue = TriggerQueue::new();
    for _ in 0..24 {
        put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        if game.stack_is_empty() { refresh(game); return; }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    }
    panic!("scenario must settle");
}
fn enter(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let hand = game.create_object_from_definition(definition, A, Zone::Hand);
    let receipt = game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut SelectFirstDecisionMaker).unwrap();
    assert!(!receipt.pending);
    assert!(receipt.programs.is_empty());
    let id = receipt.original.into_result().unwrap().new_id;
    game.remove_summoning_sickness(id);
    settle(game);
    id
}
#[derive(Default)]
struct Targets { target: Option<ObjectId>, forbidden: Option<ObjectId>, seen: bool }
impl DecisionMaker for Targets {
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        let Some(target) = self.target else { return SelectFirstDecisionMaker.decide_targets(game, context); };
        assert_eq!(context.requirements.len(), 1);
        let legal = &context.requirements[0].legal_targets;
        assert!(legal.contains(&Target::Object(target)));
        if let Some(forbidden) = self.forbidden { assert!(!legal.contains(&Target::Object(forbidden))); }
        self.seen = true;
        vec![Target::Object(target)]
    }
}
fn act(game: &mut GameState, action: LegalAction, dm: &mut Targets) {
    game.turn.priority_player = Some(A);
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() && state.pending_mana_ability.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none() && state.pending_mana_ability.is_none());
    while !game.stack_is_empty() { resolve_stack_entry_with(game, dm).unwrap(); }
    settle(game);
}
fn activate(game: &mut GameState, source: ObjectId, cost: u32, target: Option<ObjectId>, forbidden: Option<ObjectId>) {
    let ability_index = game.current_abilities(source).unwrap().iter().position(|ability| {
        matches!(&ability.kind, AbilityKind::Activated(activated)
            if activated.mana_cost.mana_cost().is_some_and(|mana| mana.mana_value() == cost))
    }).expect("full printed activated ability");
    let before = game.player(A).unwrap().mana_pool.total();
    let mut dm = Targets { target, forbidden, seen: false };
    act(game, LegalAction::ActivateAbility { source, ability_index }, &mut dm);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), before - cost);
    if target.is_some() { assert!(dm.seen); }
}
fn can_attack(game: &GameState, id: ObjectId) -> bool {
    ironsmith::rules::combat::can_attack(game.object(id).unwrap(), game)
}
fn can_block(game: &GameState, attacker: ObjectId, blocker: ObjectId) -> bool {
    ironsmith::rules::combat::can_block(game.object(attacker).unwrap(), game.object(blocker).unwrap(), game)
}

#[test]
fn seven_complete_frozen_bodies_keep_all_programs_through_both_routes() {
    assert_eq!(rows().len(), 7);
    for row in rows() { definitions(row["name"].as_str().unwrap()); }
}

#[test]
fn machinist_counts_its_controllers_artifacts_and_keeps_job_select_and_real_equip() {
    for definition in definitions("Machinist's Arsenal") {
        let mut game = game();
        let equipment = enter(&mut game, &definition);
        let Some(AttachmentTarget::Object(hero)) = game.object(equipment).unwrap().attached_to else { panic!("Job select attached Hero"); };
        assert_eq!(game.object(hero).unwrap().kind, ObjectKind::Token);
        assert!(game.current_has_subtype(hero, Subtype::Hero));
        assert!(game.current_has_subtype(hero, Subtype::Artificer));
        assert_eq!((game.current_power(hero), game.current_toughness(hero)), (Some(3), Some(3)));
        object(&mut game, B, Zone::Battlefield, "Artifact");
        let second_artifact = object(&mut game, A, Zone::Battlefield, "Artifact");
        refresh(&mut game);
        assert_eq!(game.current_power(hero), Some(5));
        let receiver = object(&mut game, A, Zone::Battlefield, "Creature — Elf");
        activate(&mut game, equipment, 4, Some(receiver), None);
        assert_eq!(game.current_power(hero), Some(1));
        assert!(!game.current_has_subtype(hero, Subtype::Artificer));
        assert_eq!(game.current_power(receiver), Some(6));
        assert!(game.current_has_subtype(receiver, Subtype::Elf));
        assert!(game.current_has_subtype(receiver, Subtype::Artificer));
        game.set_current_controller(receiver, B).unwrap();
        refresh(&mut game);
        assert_eq!(game.current_power(receiver), Some(6), "the count belongs to the Equipment's controller");
        game.move_object_by_effect(second_artifact, Zone::Graveyard).unwrap();
        refresh(&mut game);
        assert_eq!(game.current_power(receiver), Some(4));
        game.phase_out(equipment);
        refresh(&mut game);
        assert_eq!(game.current_power(receiver), Some(2));
        assert!(!game.current_has_subtype(receiver, Subtype::Artificer));
        game.phase_in(equipment);
        game.move_object_by_effect(equipment, Zone::Graveyard).unwrap();
        refresh(&mut game);
        assert_eq!(game.current_power(receiver), Some(2));
    }
}

#[test]
fn rope_keeps_its_stats_reach_blocker_limit_equip_and_sacrifice_draw() {
    for definition in definitions("Rope") {
        let mut game = game();
        let recipient = object(&mut game, A, Zone::Battlefield, "Creature — Elf");
        let equipment = enter(&mut game, &definition);
        activate(&mut game, equipment, 3, Some(recipient), None);
        assert_eq!((game.current_power(recipient), game.current_toughness(recipient)), (Some(3), Some(4)));
        assert!(game.object_has_ability(recipient, &StaticAbility::reach()));
        assert_eq!(ironsmith::rules::combat::maximum_blockers(game.object(recipient).unwrap(), &game), Some(1));
        let drawn = object(&mut game, A, Zone::Library, "Land");
        let drawn_stable = game.object(drawn).unwrap().stable_id;
        let hand_before = game.player(A).unwrap().hand.len();
        activate(&mut game, equipment, 2, None, None);
        assert!(!game.battlefield.contains(&equipment));
        assert_eq!(game.player(A).unwrap().hand.len(), hand_before + 1);
        assert!(game.player(A).unwrap().hand.iter().any(|id| game.object(*id).unwrap().stable_id == drawn_stable));
        assert_eq!((game.current_power(recipient), game.current_toughness(recipient)), (Some(2), Some(2)));
        assert!(!game.object_has_ability(recipient, &StaticAbility::reach()));
        assert_eq!(ironsmith::rules::combat::maximum_blockers(game.object(recipient).unwrap(), &game), None);
    }
}

#[test]
fn spire_metalcraft_guards_stats_and_permission_without_removing_defender() {
    for definition in definitions("Spire Serpent") {
        let mut game = game();
        let source = enter(&mut game, &definition);
        for _ in 0..3 { object(&mut game, B, Zone::Battlefield, "Artifact"); }
        for _ in 0..2 { object(&mut game, A, Zone::Battlefield, "Artifact"); }
        refresh(&mut game);
        assert_eq!((game.current_power(source), game.current_toughness(source)), (Some(3), Some(5)));
        assert!(!can_attack(&game, source));
        let third = object(&mut game, A, Zone::Battlefield, "Artifact");
        refresh(&mut game);
        assert_eq!((game.current_power(source), game.current_toughness(source)), (Some(5), Some(7)));
        assert!(can_attack(&game, source));
        assert!(game.object_has_ability(source, &StaticAbility::defender()));
        game.set_current_controller(third, B).unwrap();
        refresh(&mut game);
        assert!(!can_attack(&game, source));
        assert_eq!(game.current_power(source), Some(3));
    }
}

#[test]
fn tek_preserves_all_five_independent_land_conditions_and_recomputes_them() {
    for definition in definitions("Tek") {
        let mut game = game();
        let source = enter(&mut game, &definition);
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            object(&mut game, B, Zone::Battlefield, &format!("Basic Land — {land}"));
        }
        refresh(&mut game);
        assert_eq!((game.current_power(source), game.current_toughness(source)), (Some(2), Some(2)));
        let mut lands = Vec::new();
        for (index, land) in ["Plains", "Island", "Swamp", "Mountain", "Forest"].iter().enumerate() {
            lands.push(object(&mut game, A, Zone::Battlefield, &format!("Basic Land — {land}")));
            refresh(&mut game);
            assert_eq!(game.current_power(source), Some(if index >= 2 { 4 } else { 2 }));
            assert_eq!(game.current_toughness(source), Some(4));
            assert_eq!(game.object_has_ability(source, &StaticAbility::flying()), index >= 1);
            assert_eq!(game.object_has_ability(source, &StaticAbility::first_strike()), index >= 3);
            assert_eq!(game.object_has_ability(source, &StaticAbility::trample()), index >= 4);
        }
        game.set_current_controller(lands[1], B).unwrap();
        game.move_object_by_effect(lands[2], Zone::Graveyard).unwrap();
        refresh(&mut game);
        assert!(!game.object_has_ability(source, &StaticAbility::flying()));
        assert_eq!(game.current_power(source), Some(2));
        assert!(game.object_has_ability(source, &StaticAbility::first_strike()));
        assert!(game.object_has_ability(source, &StaticAbility::trample()));
        game.set_current_controller(source, B).unwrap();
        refresh(&mut game);
        assert_eq!((game.current_power(source), game.current_toughness(source)), (Some(4), Some(4)));
        assert!(game.object_has_ability(source, &StaticAbility::flying()));
    }
}

#[test]
fn nighthowler_counts_all_graveyards_in_creature_bestow_and_detached_forms() {
    for definition in definitions("Nighthowler") {
        let mut game = game();
        for owner in [A, B, C] { object(&mut game, owner, Zone::Graveyard, "Creature — Bear"); }
        object(&mut game, A, Zone::Graveyard, "Artifact");
        let ordinary = enter(&mut game, &definition);
        assert_eq!((game.current_power(ordinary), game.current_toughness(ordinary)), (Some(3), Some(3)));
        game.move_object_by_effect(ordinary, Zone::Exile).unwrap();
        let host = object(&mut game, A, Zone::Battlefield, "Creature — Elf");
        let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        let before = game.player(A).unwrap().mana_pool.total();
        let mut dm = Targets { target: Some(host), forbidden: None, seen: false };
        act(&mut game, LegalAction::CastSpell { spell_id: hand, from_zone: Zone::Hand,
            casting_method: CastingMethod::Alternative(0) }, &mut dm);
        assert!(dm.seen);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 4);
        let bestowed = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Nighthowler").unwrap();
        assert_eq!(game.object(bestowed).unwrap().attached_to, Some(AttachmentTarget::Object(host)));
        assert!(!game.calculated_card_types(bestowed).contains(&CardType::Creature));
        assert_eq!(game.current_power(host), Some(5));
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        check_and_apply_sbas(&mut game, &mut TriggerQueue::new()).unwrap();
        refresh(&mut game);
        assert!(game.calculated_card_types(bestowed).contains(&CardType::Creature));
        assert_eq!((game.current_power(bestowed), game.current_toughness(bestowed)), (Some(4), Some(4)));
        game.add_counters(bestowed, CounterType::PlusOnePlusOne, 1).unwrap();
        refresh(&mut game);
        assert_eq!(game.current_power(bestowed), Some(5));
        let gone = game.player(B).unwrap().graveyard[0];
        game.move_object_by_effect(gone, Zone::Exile).unwrap();
        refresh(&mut game);
        assert_eq!(game.current_power(bestowed), Some(4));
    }
}

#[test]
fn lookout_requires_one_opponents_graveyard_and_gates_both_combat_predicates() {
    for definition in definitions("Expedition Lookout") {
        let mut game = game();
        let source = enter(&mut game, &definition);
        let blocker = object(&mut game, B, Zone::Battlefield, "Creature — Bear");
        for _ in 0..8 { object(&mut game, A, Zone::Graveyard, "Artifact"); }
        for owner in [B, C] { for _ in 0..4 { object(&mut game, owner, Zone::Graveyard, "Land"); } }
        refresh(&mut game);
        assert!(!can_attack(&game, source));
        assert!(can_block(&game, source, blocker));
        for _ in 0..4 { object(&mut game, B, Zone::Graveyard, "Artifact"); }
        refresh(&mut game);
        assert!(can_attack(&game, source));
        assert!(!can_block(&game, source, blocker));
        assert!(game.object_has_ability(source, &StaticAbility::defender()));
        let removed = game.player(B).unwrap().graveyard[0];
        game.move_object_by_effect(removed, Zone::Exile).unwrap();
        refresh(&mut game);
        assert!(!can_attack(&game, source));
        assert!(can_block(&game, source, blocker));
    }
}

#[test]
fn wrecking_ball_keeps_base_layer_rule_lifetime_and_both_equip_prices() {
    for definition in definitions("Wrecking Ball Arm") {
        let mut game = game();
        let ordinary = object(&mut game, A, Zone::Battlefield, "Creature — Elf");
        let legendary = object(&mut game, A, Zone::Battlefield, "Legendary Creature — Human");
        let blocker = object(&mut game, B, Zone::Battlefield, "Creature — Bear");
        let equipment = enter(&mut game, &definition);
        game.add_counters(legendary, CounterType::PlusOnePlusOne, 1).unwrap();
        activate(&mut game, equipment, 3, Some(legendary), Some(ordinary));
        assert_eq!((game.current_power(legendary), game.current_toughness(legendary)), (Some(8), Some(8)));
        assert!(!can_block(&game, legendary, blocker));
        ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(legendary),
            ironsmith::continuous::Modification::RemoveAllAbilities, Until::EndOfTurn)
            .execute(&mut game, &mut EffectContext::new_default(equipment, A)).unwrap();
        refresh(&mut game);
        assert!(!can_block(&game, legendary, blocker), "unquoted rule survives recipient ability loss");
        game.add_counters(blocker, CounterType::PlusOnePlusOne, 1).unwrap();
        refresh(&mut game);
        assert!(can_block(&game, legendary, blocker), "current blocker power is three");
        activate(&mut game, equipment, 7, Some(ordinary), None);
        assert_eq!(game.current_power(legendary), Some(3));
        assert_eq!(game.current_power(ordinary), Some(7));
        game.phase_out(equipment);
        refresh(&mut game);
        assert_eq!(game.current_power(ordinary), Some(2));
        game.phase_in(equipment);
        refresh(&mut game);
        assert_eq!(game.current_power(ordinary), Some(7));
        game.move_object_by_effect(equipment, Zone::Graveyard).unwrap();
        refresh(&mut game);
        assert_eq!(game.current_power(ordinary), Some(2));
    }
}

#[test]
fn rope_unquoted_blocker_limit_must_survive_recipient_ability_loss() {
    // Reach is a granted ability; the unquoted blocker limit is a live rule
    // controlled by the Equipment. Ability loss affects only the former.
    for definition in definitions("Rope") {
        let mut game = game();
        let recipient = object(&mut game, A, Zone::Battlefield, "Creature — Elf");
        let equipment = enter(&mut game, &definition);
        activate(&mut game, equipment, 3, Some(recipient), None);
        ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(recipient),
            ironsmith::continuous::Modification::RemoveAllAbilities, Until::EndOfTurn)
            .execute(&mut game, &mut EffectContext::new_default(equipment, A)).unwrap();
        refresh(&mut game);
        assert!(!game.object_has_ability(recipient, &StaticAbility::reach()));
        assert_eq!(game.current_power(recipient), Some(3));
        assert_eq!(ironsmith::rules::combat::maximum_blockers(game.object(recipient).unwrap(), &game), Some(1));
        let blocker_a = object(&mut game, B, Zone::Battlefield, "Creature — Bear");
        let blocker_b = object(&mut game, B, Zone::Battlefield, "Creature — Bear");
        game.turn.phase = Phase::Combat;
        let mut combat = ironsmith::combat_state::CombatState::default();
        ironsmith::combat_state::declare_attackers(&mut game, &mut combat,
            vec![(recipient, ironsmith::combat_state::AttackTarget::Player(B))]).unwrap();
        assert!(matches!(ironsmith::combat_state::declare_blockers(&game, &mut combat,
            vec![(blocker_a, recipient), (blocker_b, recipient)]),
            Err(ironsmith::combat_state::CombatError::TooManyBlockers { maximum: 1, provided: 2, .. })));
        ironsmith::combat_state::declare_blockers(&game, &mut combat, vec![(blocker_a, recipient)]).unwrap();
        game.phase_out(equipment);
        refresh(&mut game);
        assert_eq!(ironsmith::rules::combat::maximum_blockers(game.object(recipient).unwrap(), &game), None);
        game.phase_in(equipment);
        refresh(&mut game);
        assert_eq!(ironsmith::rules::combat::maximum_blockers(game.object(recipient).unwrap(), &game), Some(1));
        ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(equipment),
            ironsmith::continuous::Modification::RemoveAllAbilities, Until::EndOfTurn)
            .execute(&mut game, &mut EffectContext::new_default(equipment, A)).unwrap();
        refresh(&mut game);
        assert_eq!(ironsmith::rules::combat::maximum_blockers(game.object(recipient).unwrap(), &game), None,
            "removing the source's actual static ability removes its rule");
    }
}

#[test]
fn rope_limit_follows_reattachment_and_two_sources_do_not_erase_each_other() {
    for definition in definitions("Rope") {
        let mut game = game();
        let first = object(&mut game, A, Zone::Battlefield, "Creature — Elf");
        let second = object(&mut game, A, Zone::Battlefield, "Creature — Elf");
        let rope_a = enter(&mut game, &definition);
        let rope_b = enter(&mut game, &definition);
        activate(&mut game, rope_a, 3, Some(first), None);
        activate(&mut game, rope_b, 3, Some(first), None);
        assert_eq!(ironsmith::rules::combat::maximum_blockers(game.object(first).unwrap(), &game), Some(1));
        game.move_object_by_effect(rope_a, Zone::Graveyard).unwrap();
        refresh(&mut game);
        assert_eq!(ironsmith::rules::combat::maximum_blockers(game.object(first).unwrap(), &game), Some(1));
        activate(&mut game, rope_b, 3, Some(second), None);
        assert_eq!(ironsmith::rules::combat::maximum_blockers(game.object(first).unwrap(), &game), None);
        assert_eq!(ironsmith::rules::combat::maximum_blockers(game.object(second).unwrap(), &game), Some(1));
        game.set_current_controller(second, B).unwrap();
        refresh(&mut game);
        assert_eq!(ironsmith::rules::combat::maximum_blockers(game.object(second).unwrap(), &game), Some(1));
        game.move_object_by_effect(rope_b, Zone::Graveyard).unwrap();
        refresh(&mut game);
        assert_eq!(ironsmith::rules::combat::maximum_blockers(game.object(second).unwrap(), &game), None);
    }
}

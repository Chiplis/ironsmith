//! Restored full frozen body and native action scenarios; all checks UNRUN.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, SelectOptionsContext};
use ironsmith::mana::ManaSymbol;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn definitions() -> [CardDefinition; 3] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/ur_dragon_body.json.fixture")).unwrap(); let row = &rows[0];
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(), row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("The Ur-Dragon", &text, false));
    let (artifact, materialized) = result.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text()); artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap(); assert_eq!(artifact, decoded);
    // compile_to_artifact returns an artifact-materialized definition. Keep
    // its route and the JSON roundtrip separate from true direct lowering.
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition("The Ur-Dragon", &text, false));
    let direct = direct.unwrap();
    assert!(!direct_loss.is_lossy(), "direct route: {}", direct_loss.reasons_text());
    [direct, materialized, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None; game.turn.active_player = A; game.turn.priority_player = Some(A); game
}
fn support(name: &str, subtype: &str) -> CardDefinition {
    compile_to_runtime_definition(name, format!("Mana cost: {{2}}{{R}}\nType: Creature — {subtype}\nPower/Toughness: 2/2"), false).unwrap()
}
fn fund(game: &mut GameState, player: PlayerId, generic: u32) {
    game.player_mut(player).unwrap().mana_pool.empty(); game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Red, 1);
    game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Colorless, generic);
}
fn action(spell: ObjectId, from_zone: Zone) -> LegalAction {
    LegalAction::CastSpell { spell_id: spell, from_zone, casting_method: CastingMethod::Normal }
}
fn can_cast(game: &mut GameState, spell: ObjectId, player: PlayerId) -> bool {
    game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None; game.turn.active_player = player; game.turn.priority_player = Some(player);
    compute_legal_actions(game, player).unwrap().contains(&action(spell, game.object(spell).unwrap().zone))
}
struct Choices { accept: bool, permanent: Option<ObjectId> }
impl Default for Choices { fn default() -> Self { Self { accept: true, permanent: None } } }
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, context: &BooleanContext) -> bool { self.accept && context.can_accept }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.permanent {
            assert!(context.candidates.iter().any(|c| c.id == id && c.legal)); vec![id]
        } else { SelectFirstDecisionMaker.decide_objects(game, context) }
    }
}
fn announce(game: &mut GameState, spell: ObjectId, player: PlayerId, dm: &mut impl DecisionMaker) -> ObjectId {
    assert!(can_cast(game, spell, player));
    let action = action(spell, game.object(spell).unwrap().zone);
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    game.stack.iter().find(|entry| !entry.is_ability).unwrap().object_id
}
fn declare(game: &mut GameState, attackers: &[(ObjectId, PlayerId)], dm: &mut Choices) {
    game.turn.phase = ironsmith::Phase::Combat; game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers); game.turn.active_player = A; game.mark_combat_phase_started();
    let declarations = attackers.iter().map(|(creature, player)| AttackerDeclaration { creature: *creature, target: AttackTarget::Player(*player) }).collect::<Vec<_>>();
    let mut combat = CombatState::default(); let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(game, &mut combat, &mut queue, &declarations).unwrap();
    game.combat = Some(combat); put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
#[test]
fn exact_full_body_retains_only_its_cost_ability_in_command() {
    for definition in definitions() {
        assert_eq!(definition.abilities.len(), 3);
        assert_eq!(definition.abilities.iter().filter(|ability| ability.functions_in(&Zone::Command)).count(), 1);
        for zone in [Zone::Command, Zone::Battlefield, Zone::Hand, Zone::Stack, Zone::Graveyard, Zone::Library, Zone::Exile] {
            let mut game = game(); game.create_object_from_definition(&definition, A, zone);
            let dragon = game.create_object_from_definition(&support("Other Dragon", "Dragon"), A, Zone::Hand);
            let bear = game.create_object_from_definition(&support("Bear", "Bear"), A, Zone::Hand);
            let foreign = game.create_object_from_definition(&support("Foreign Dragon", "Dragon"), B, Zone::Hand);
            fund(&mut game, A, 1); fund(&mut game, B, 1);
            let active = matches!(zone, Zone::Command | Zone::Battlefield);
            assert_eq!(can_cast(&mut game, dragon, A), active, "zone {zone:?}");
            assert!(!can_cast(&mut game, bear, A)); assert!(!can_cast(&mut game, foreign, B));
            if active {
                let stack = announce(&mut game, dragon, A, &mut Choices::default());
                assert_eq!(game.object(stack).unwrap().mana_spent_to_cast.total(), 2); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            }
        }
    }
}
#[test]
fn ordinary_command_sources_do_not_leak_and_actual_commander_payment_is_not_discounted() {
    for definition in definitions() {
        let mut game = game(); game.create_object_from_definition(&definition, A, Zone::Command);
        let ordinary = compile_to_runtime_definition("Ordinary cost source", "Type: Enchantment\nCreature spells you cast cost {1} less to cast.", false).unwrap();
        game.create_object_from_definition(&ordinary, A, Zone::Command);
        let dragon = game.create_object_from_definition(&support("Other Dragon", "Dragon"), A, Zone::Hand);
        fund(&mut game, A, 0); assert!(!can_cast(&mut game, dragon, A));
        let mut game = self::game(); let commander = game.create_object_from_definition(&definition, A, Zone::Command); game.set_as_commander(commander, A);
        for color in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green] { game.player_mut(A).unwrap().mana_pool.add(color, 1); }
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 3); assert!(!can_cast(&mut game, commander, A));
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
        let stack = announce(&mut game, commander, A, &mut Choices::default());
        assert_eq!(game.object(stack).unwrap().mana_spent_to_cast.total(), 9); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn live_control_phasing_and_command_ability_loss_invalidate_inventory() {
    use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
    for definition in definitions() {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = game.create_object_from_definition(&support("Own Dragon", "Dragon"), A, Zone::Hand); let foreign = game.create_object_from_definition(&support("Foreign Dragon", "Dragon"), B, Zone::Hand);
        fund(&mut game, A, 1); fund(&mut game, B, 1); assert!(can_cast(&mut game, own, A));
        game.phase_out(source); assert!(!can_cast(&mut game, own, A)); game.phase_in(source); assert!(can_cast(&mut game, own, A));
        game.set_current_controller(source, B).unwrap(); assert!(!can_cast(&mut game, own, A)); assert!(can_cast(&mut game, foreign, B));
        let command = game.move_object_by_effect(source, Zone::Command).unwrap();
        assert!(can_cast(&mut game, own, A), "moving to Command restores the owner's cost scope before ability loss");
        // Resolution effects need an explicit nonbattlefield zone, together
        // with their exact locked recipient; Specific alone is Battlefield/Stack.
        ApplyContinuousEffect::new(ironsmith::continuous::EffectTarget::Filter(
            ironsmith::ObjectFilter::specific(command).in_zone(Zone::Command)),
            ironsmith::continuous::Modification::RemoveAllAbilities, ironsmith::effect::Until::Forever)
            .lock_filter_at_resolution()
            .execute(&mut game, &mut EffectContext::new_default(command, A)).unwrap();
        assert!(!can_cast(&mut game, own, A)); assert!(!can_cast(&mut game, foreign, B));
    }
}
#[test]
fn one_attack_group_draws_the_retained_dragon_count_then_may_put_a_permanent() {
    for definition in definitions() { for accept in [true, false] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(game.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Flying));
        let first = game.create_object_from_definition(&support("First Dragon", "Dragon"), A, Zone::Battlefield);
        let second = game.create_object_from_definition(&support("Second Dragon", "Dragon"), A, Zone::Battlefield); let bear = game.create_object_from_definition(&support("Bear", "Bear"), A, Zone::Battlefield);
        let hand = game.create_object_from_definition(&support("Chosen permanent", "Elf"), A, Zone::Hand); let stable = game.object(hand).unwrap().stable_id;
        for n in 0..4 { game.create_object_from_definition(&support(&format!("Library {n}"), "Bird"), A, Zone::Library); }
        for id in [first, second, bear] { game.remove_summoning_sickness(id); }
        let mut dm = Choices { accept, permanent: Some(hand) }; declare(&mut game, &[(first, B), (second, C), (bear, B)], &mut dm);
        assert_eq!(game.stack.len(), 1); game.move_object_by_effect(first, Zone::Graveyard).unwrap();
        resolve_stack_entry_with(&mut game, &mut dm).unwrap(); assert_eq!(game.player(A).unwrap().library.len(), 2);
        assert_eq!(game.player(A).unwrap().hand.len(), if accept { 2 } else { 3 });
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, if accept { Zone::Battlefield } else { Zone::Hand });
    }}
}
#[test]
fn command_has_no_attack_trigger_and_attacking_source_counts_itself() {
    for definition in definitions() { for zone in [Zone::Command, Zone::Battlefield] {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, zone);
        let dragon = game.create_object_from_definition(&support("Dragon", "Dragon"), A, Zone::Battlefield);
        for _ in 0..3 { game.create_object_from_definition(&support("Draw resource", "Bird"), A, Zone::Library); }
        game.remove_summoning_sickness(dragon); let mut dm = Choices { accept: false, permanent: None };
        let attacks = if zone == Zone::Battlefield { game.remove_summoning_sickness(source); vec![(source, B), (dragon, B)] } else { vec![(dragon, B)] };
        declare(&mut game, &attacks, &mut dm); assert_eq!(game.stack.len(), usize::from(zone == Zone::Battlefield));
        if zone == Zone::Battlefield { resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
        assert_eq!(game.player(A).unwrap().hand.len(), if zone == Zone::Battlefield { 2 } else { 0 });
    }}
}
#[test]
fn inactive_cost_siblings_on_the_same_command_object_are_filtered_by_every_consumer() {
    struct NoLifeOptions;
    impl DecisionMaker for NoLifeOptions {
        fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
            assert!(!context.options.iter().any(|o| o.description.to_ascii_lowercase().contains("life")), "{context:?}");
            SelectFirstDecisionMaker.decide_options(game, context)
        }
    }
    for mut definition in definitions() {
        let siblings = compile_to_runtime_definition("Inactive cost siblings", "Type: Enchantment\nDragon spells you cast cost {1} less to cast.\nDragon spells you cast cost an additional 3 life to cast.\nAs an additional cost to cast red permanent spells, you may pay 2 life. Those spells cost {R} less to cast if you paid life this way. This effect reduces only the amount of red mana you pay.", false).unwrap();
        assert!(siblings.abilities.iter().all(|a| !a.functions_in(&Zone::Command))); definition.abilities.extend(siblings.abilities);
        let mut game = game(); game.create_object_from_definition(&definition, A, Zone::Command);
        let dragon = game.create_object_from_definition(&support("Dragon", "Dragon"), A, Zone::Hand);
        fund(&mut game, A, 0); assert!(!can_cast(&mut game, dragon, A), "the inactive same-source generic sibling cannot reduce the cost again");
        fund(&mut game, A, 1); game.player_mut(A).unwrap().life = 2;
        announce(&mut game, dragon, A, &mut NoLifeOptions);
        assert_eq!(game.player(A).unwrap().life, 2); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn command_inventory_discovers_current_replaced_eminence_abilities() {
    use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
    for definition in definitions() {
        let mut game = game();
        let blank = compile_to_runtime_definition("Command recipient", "Type: Enchantment", false).unwrap();
        let source = game.create_object_from_definition(&blank, A, Zone::Command);
        let dragon = game.create_object_from_definition(&support("Dragon", "Dragon"), A, Zone::Hand);
        fund(&mut game, A, 1); assert!(!can_cast(&mut game, dragon, A));
        let eminence = definition.abilities.iter().find(|ability| ability.functions_in(&Zone::Command)).unwrap().clone();
        ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Filter(
                ironsmith::ObjectFilter::specific(source).in_zone(Zone::Command)),
            ironsmith::continuous::Modification::SetAbilities(vec![eminence]),
            ironsmith::effect::Until::Forever,
        ).lock_filter_at_resolution().execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
        assert!(can_cast(&mut game, dragon, A), "current zone-qualified abilities participate even without printed cost modifiers");
        let stack = announce(&mut game, dragon, A, &mut Choices::default());
        assert_eq!(game.object(stack).unwrap().mana_spent_to_cast.total(), 2);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}

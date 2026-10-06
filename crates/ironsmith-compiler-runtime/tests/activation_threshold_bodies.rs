//! Exact frozen bodies. Source-authored and intentionally unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{AbilityOrigin, ContinuousEffect, Modification};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, EffectExecutor, ExecutionError};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/activation_threshold_bodies.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!direct_loss.is_lossy(), "direct {name}: {}", direct_loss.reasons_text());
    // This is an independent compilation; the artifact's companion definition is ignored.
    let (artifact, artifact_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, text, false));
    let (artifact, _) = artifact.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!artifact_loss.is_lossy(), "artifact {name}: {}", artifact_loss.reasons_text());
    let encoded = artifact.to_json().unwrap();
    let restored = CompiledCardArtifact::from_json(&encoded).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    assert_eq!(restored.format_version, ironsmith_compiled_artifact::FORMAT_VERSION);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["A".into(), "B".into()], 20);
    priority(&mut g, A); g
}
fn priority(g: &mut GameState, p: PlayerId) {
    g.turn.active_player = p; g.turn.priority_player = Some(p);
    g.turn.phase = Phase::FirstMain; g.turn.step = None;
}
fn origin(g: &GameState, source: ObjectId, index: usize) -> AbilityOrigin {
    g.current_characteristics(source).unwrap().abilities.origin(index).unwrap().clone()
}
fn index(g: &GameState, source: ObjectId, ordinal: usize) -> usize {
    g.current_abilities(source).unwrap().iter().enumerate()
        .filter(|(_, ability)| matches!(&ability.kind, AbilityKind::Activated(_)))
        .nth(ordinal).unwrap().0
}
fn count(g: &GameState, source: ObjectId, origin: &AbilityOrigin) -> u32 {
    let chars = g.current_characteristics(source).unwrap();
    let definition = chars.abilities.iter().enumerate().find_map(|(slot, ability)| {
        if chars.abilities.origin(slot) != Some(origin) { return None; }
        match &ability.kind { AbilityKind::Activated(ability) => ability.effects.activation_definition, _ => None }
    }).expect("authored activation definition");
    g.turn_store.turn_history.ability_activation_counts.as_ref().unwrap()
        .get(&(source, origin.clone(), Some(definition))).copied().unwrap_or(0)
}
fn add(g: &mut GameState, p: PlayerId, symbol: ManaSymbol, amount: u32) {
    g.player_mut(p).unwrap().mana_pool.add(symbol, amount);
}
fn activate(g: &mut GameState, source: ObjectId, ordinal: usize) {
    let p = g.current_controller(source).unwrap(); priority(g, p);
    let index = index(g, source, ordinal);
    let action = compute_legal_actions(g, p).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::ActivateAbility { source: id, ability_index }
        | LegalAction::ActivateManaAbility { source: id, ability_index }
        if *id == source && *ability_index == index)).unwrap();
    let mut state = PriorityLoopState::new(2); let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(g, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..40 {
        if !state.has_pending_action() { return; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending activation: {progress:?}") };
        progress = apply_decision_context_with_dm(g, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    panic!("activation did not finish");
}
fn settle(g: &mut GameState) {
    while !g.stack.is_empty() { resolve_stack_entry_with(g, &mut SelectFirstDecisionMaker).unwrap(); }
}
fn end_step(g: &mut GameState, p: PlayerId) {
    g.turn.active_player = p; g.turn.phase = Phase::Ending; g.turn.step = Some(Step::End);
    let mut queue = TriggerQueue::new();
    let event = TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfEndStepEvent::new(p), Default::default());
    for trigger in check_triggers(g, &event) { queue.add(trigger); }
    put_triggers_on_stack_with_dm(g, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    settle(g);
}
fn body(name: &str, text: &str) -> CardDefinition { compile_to_runtime_definition(name, text, false).unwrap() }
fn copy_top(g: &mut GameState, controller: PlayerId) {
    let target = g.stack.last().unwrap().target_id();
    let original = g.stack.last().unwrap().activation_origin.clone();
    let definition = g.stack.last().unwrap().activation_definition;
    let source = g.stack.last().unwrap().object_id;
    let mut ctx = EffectContext::new_default(source, controller);
    ironsmith::effects::CopySpellEffect::single(ChooseSpec::SpecificObject(target)).execute(g, &mut ctx).unwrap();
    assert_eq!(g.stack.last().unwrap().activation_origin, original);
    assert_eq!(g.stack.last().unwrap().activation_definition, definition);
}
#[test]
fn all_four_frozen_bodies_keep_every_ability() {
    for row in rows() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let abilities = g.current_abilities(source).unwrap();
            assert_eq!(abilities.iter().filter(|ability| matches!(&ability.kind, AbilityKind::Activated(_))).count(), 1);
            assert_eq!(g.current_has_static_ability_id(source, StaticAbilityId::Flying), name.ends_with("Dragon") || name == "Dragon Whelp");
            assert_eq!(g.current_has_static_ability_id(source, StaticAbilityId::Banding), name == "Nalathni Dragon");
            assert_eq!(abilities.len(), match name { "Dragon Whelp" => 2, "Nalathni Dragon" => 3, _ => 1 });
        }
    }
}
#[test]
fn three_activations_are_safe_and_four_stacked_activations_all_schedule() {
    for name in ["Dragon Whelp", "Nalathni Dragon"] { for definition in definitions(name) { for uses in [3, 4] {
        let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let key = origin(&g, source, index(&g, source, 0)); let base = g.calculated_power(source).unwrap();
        add(&mut g, A, ManaSymbol::Red, uses);
        for _ in 0..uses { activate(&mut g, source, 0); }
        assert_eq!(count(&g, source, &key), uses); assert_eq!(g.stack.len(), uses as usize);
        assert!(g.effect_store.delayed_triggers.is_empty());
        settle(&mut g);
        assert_eq!(g.calculated_power(source), Some(base + uses as i32));
        assert_eq!(g.effect_store.delayed_triggers.len(), if uses == 4 { 4 } else { 0 });
        end_step(&mut g, B);
        assert_eq!(g.object(source).is_some(), uses == 3);
    }}}
}
#[test]
fn countered_activations_count_and_copies_do_not() {
    for definition in definitions("Dragon Whelp") {
        let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let key = origin(&g, source, index(&g, source, 0)); add(&mut g, A, ManaSymbol::Red, 4);
        for _ in 0..3 { activate(&mut g, source, 0); }
        copy_top(&mut g, A); assert_eq!(count(&g, source, &key), 3);
        resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
        assert!(g.effect_store.delayed_triggers.is_empty());
        let target = g.stack.last().unwrap().target_id();
        ironsmith::effects::CounterEffect::new(ChooseSpec::SpecificObject(target))
            .execute(&mut g, &mut EffectContext::new_default(source, B)).unwrap();
        assert_eq!(count(&g, source, &key), 3);
        activate(&mut g, source, 0); copy_top(&mut g, A);
        assert_eq!(count(&g, source, &key), 4); settle(&mut g);
        assert_eq!(g.effect_store.delayed_triggers.len(), 4);
        end_step(&mut g, A); assert!(g.object(source).is_none());
    }
}
#[test]
fn mana_conversion_counts_before_its_own_resolution_for_both_actual_payment_owners() {
    for name in ["Farrelite Priest", "Initiates of the Ebon Hand"] { for definition in definitions(name) { for special in [false, true] {
        let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let slot = index(&g, source, 0); let key = origin(&g, source, slot);
        for uses in 1..=4 {
            g.player_mut(A).unwrap().mana_pool = Default::default(); add(&mut g, A, ManaSymbol::Colorless, 1);
            if special {
                ironsmith::special_actions::perform_activate_mana_ability(&mut g, A, source, slot, &mut SelectFirstDecisionMaker).unwrap();
            } else { activate(&mut g, source, 0); }
            assert!(g.stack.is_empty()); assert_eq!(count(&g, source, &key), uses);
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 1);
            let expected = if name == "Farrelite Priest" { ManaSymbol::White } else { ManaSymbol::Black };
            assert_eq!(g.player(A).unwrap().mana_pool.amount(expected), 1);
            assert_eq!(g.effect_store.delayed_triggers.len(), usize::from(uses == 4));
        }
        end_step(&mut g, B); assert!(g.object(source).is_none());
    }}}
}
#[test]
fn control_changes_preserve_history_but_sacrifice_requires_the_delayed_controller() {
    for definition in definitions("Dragon Whelp") { for regain in [false, true] {
        let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let key = origin(&g, source, index(&g, source, 0)); add(&mut g, A, ManaSymbol::Red, 4);
        for _ in 0..4 { activate(&mut g, source, 0); }
        g.set_current_controller(source, B).unwrap(); settle(&mut g);
        assert_eq!(count(&g, source, &key), 4);
        assert!(g.effect_store.delayed_triggers.iter().all(|trigger| trigger.controller == A));
        if regain { g.set_current_controller(source, A).unwrap(); }
        end_step(&mut g, B); assert_eq!(g.object(source).is_none(), regain);
    }}
}
#[test]
fn control_changes_between_mana_uses_share_only_the_same_acquisition() {
    for definition in definitions("Farrelite Priest") {
        let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let key = origin(&g, source, index(&g, source, 0)); add(&mut g, A, ManaSymbol::Colorless, 3);
        for _ in 0..3 { activate(&mut g, source, 0); }
        g.set_current_controller(source, B).unwrap(); add(&mut g, B, ManaSymbol::Colorless, 1);
        activate(&mut g, source, 0); assert_eq!(count(&g, source, &key), 4);
        assert_eq!(g.effect_store.delayed_triggers.len(), 1);
        assert_eq!(g.effect_store.delayed_triggers[0].controller, B);
        end_step(&mut g, A); assert!(g.object(source).is_none());
    }
}
#[test]
fn blink_does_not_transfer_a_delayed_sacrifice_or_activation_history() {
    for definition in definitions("Dragon Whelp") { for before_resolution in [false, true] {
        let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        add(&mut g, A, ManaSymbol::Red, 4); for _ in 0..4 { activate(&mut g, source, 0); }
        if !before_resolution { settle(&mut g); }
        let exile = g.move_object_by_effect(source, Zone::Exile).unwrap();
        let returned = g.move_object_by_effect(exile, Zone::Battlefield).unwrap(); assert_ne!(source, returned);
        if before_resolution { settle(&mut g); }
        let key = origin(&g, returned, index(&g, returned, 0)); assert_eq!(count(&g, returned, &key), 0);
        end_step(&mut g, B); assert!(g.object(returned).is_some());
    }}
}
#[test]
fn new_turn_resets_history_and_old_delays_still_resolve() {
    for definition in definitions("Farrelite Priest") {
        let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let key = origin(&g, source, index(&g, source, 0)); add(&mut g, A, ManaSymbol::Colorless, 4);
        for _ in 0..4 { activate(&mut g, source, 0); }
        let saved = g.clone(); g.next_turn();
        assert_eq!(count(&g, source, &key), 0); assert_eq!(g.effect_store.delayed_triggers.len(), 1);
        end_step(&mut g, B); assert!(g.object(source).is_none());
        g = saved; assert_eq!(count(&g, source, &key), 4); assert_eq!(g.effect_store.delayed_triggers.len(), 1);
    }
}
#[test]
fn native_clone_keeps_origin_and_unknown_history_or_origin_is_an_error() {
    for definition in definitions("Dragon Whelp") {
        let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        add(&mut g, A, ManaSymbol::Red, 4); for _ in 0..4 { activate(&mut g, source, 0); }
        let saved = g.clone(); let entry = g.stack.last().unwrap().clone();
        let condition = ironsmith::ConditionExpr::ThisAbilityActivatedThisTurnAtLeast(4);
        let mut ctx = EffectContext::new_default(source, A).with_activation_origin(entry.activation_origin.clone())
            .with_activation_definition(entry.activation_definition);
        assert!(ironsmith::condition_eval::evaluate_condition_resolution(&g, &condition, &ctx).unwrap());
        g.turn_store.turn_history.ability_activation_counts = None;
        assert!(matches!(ironsmith::condition_eval::evaluate_condition_resolution(&g, &condition, &ctx), Err(ExecutionError::IncompleteEvidence(_))));
        g = saved; ctx.activation_origin = None;
        assert!(matches!(ironsmith::condition_eval::evaluate_condition_resolution(&g, &condition, &ctx), Err(ExecutionError::IncompleteEvidence(_))));
        ctx.activation_origin = entry.activation_origin;
        assert!(ironsmith::condition_eval::evaluate_condition_resolution(&g, &condition, &ctx).unwrap());
        ctx.activation_definition = None;
        assert!(matches!(ironsmith::condition_eval::evaluate_condition_resolution(&g, &condition, &ctx), Err(ExecutionError::IncompleteEvidence(_))));
    }
}
#[test]
fn independent_grants_and_reordered_display_slots_keep_distinct_counts() {
    let threshold = "Type: Creature — Dragon\nPower/Toughness: 2/3\n{R}: This creature gets +1/+0 until end of turn. If this ability has been activated four or more times this turn, sacrifice this creature at the beginning of the next end step.";
    for definition in definitions("Dragon Whelp") {
        let grant = body("Granted threshold", threshold).abilities.into_iter()
            .find(|ability| matches!(&ability.kind, AbilityKind::Activated(_))).unwrap();
        let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let printed = origin(&g, source, index(&g, source, 0));
        g.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(source, A, vec![source], Modification::AddAbilityGeneric(grant)));
        let granted = origin(&g, source, index(&g, source, 1)); assert_ne!(printed, granted);
        add(&mut g, A, ManaSymbol::Red, 7);
        for _ in 0..3 { activate(&mut g, source, 0); activate(&mut g, source, 1); }
        settle(&mut g); assert!(g.effect_store.delayed_triggers.is_empty());
        // Removing Flying moves the displayed activation slots but retains their origins.
        g.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(source, A, vec![source],
            Modification::RemoveStaticAbilityFamily(StaticAbilityId::Flying)));
        assert_eq!(index(&g, source, 0), 0); activate(&mut g, source, 0); settle(&mut g);
        assert_eq!(count(&g, source, &printed), 4); assert_eq!(count(&g, source, &granted), 3);
        assert_eq!(g.effect_store.delayed_triggers.len(), 1);
    }
}
#[test]
fn nalathnis_flying_banding_and_damage_assignment_have_native_owners() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState, declare_blockers, set_attacking_band, combat_damage_assignment_player};
    for definition in definitions("Nalathni Dragon") {
        let mut g = game(); let dragon = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let vanilla = body("Band companion", "Type: Creature — Beast\nPower/Toughness: 2/2");
        let companion = g.create_object_from_definition(&vanilla, A, Zone::Battlefield);
        let other = g.create_object_from_definition(&vanilla, A, Zone::Battlefield);
        let ground = g.create_object_from_definition(&vanilla, B, Zone::Battlefield);
        let mut combat = CombatState { attackers: vec![AttackerInfo { creature: dragon, target: AttackTarget::Player(B) },
            AttackerInfo { creature: companion, target: AttackTarget::Player(B) },
            AttackerInfo { creature: other, target: AttackTarget::Player(B) }], ..Default::default() };
        assert!(declare_blockers(&g, &mut combat.clone(), vec![(ground, dragon)]).is_err());
        assert!(set_attacking_band(&g, &mut combat.clone(), vec![dragon, companion, other]).is_err());
        set_attacking_band(&g, &mut combat, vec![dragon, companion]).unwrap();
        declare_blockers(&g, &mut combat, vec![(ground, companion)]).unwrap();
        assert_eq!(combat.blockers.get(&dragon), Some(&vec![ground]));
        assert_eq!(combat_damage_assignment_player(&g, &combat, ground), Some(A));
        g.combat = Some(combat);
        assert!(g.set_combat_damage_assignment_for_player(B, ground, dragon, 2).is_err());
        g.set_combat_damage_assignment_for_player(A, ground, companion, 2).unwrap();
    }
}

#[test]
fn cancelled_or_failed_payments_do_not_count() {
    struct Cancel;
    impl DecisionMaker for Cancel {
        fn decide_mana_payment(&mut self, _: &GameState, _: &ironsmith::decisions::context::ManaPaymentContext)
            -> ironsmith::mana_payment::ManaPaymentResponse { ironsmith::mana_payment::ManaPaymentResponse::Cancel }
    }
    for name in ["Dragon Whelp", "Farrelite Priest", "Initiates of the Ebon Hand", "Nalathni Dragon"] {
        for definition in definitions(name) {
            let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let slot = index(&g, source, 0); let key = origin(&g, source, slot);
            add(&mut g, A, ManaSymbol::Red, 1);
            let action = compute_legal_actions(&g, A).unwrap().into_iter().find(|a| matches!(a,
                LegalAction::ActivateAbility { source: id, ability_index } | LegalAction::ActivateManaAbility { source: id, ability_index }
                if *id == source && *ability_index == slot)).unwrap();
            let mut state = PriorityLoopState::new(2); let mut queue = TriggerQueue::new(); let mut dm = Cancel;
            let mut result = apply_priority_response_with_dm(&mut g, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm);
            for _ in 0..40 {
                if !state.has_pending_action() || result.is_err() { break; }
                let GameProgress::NeedsDecisionCtx(context) = result.unwrap() else { panic!("missing cancellation choice") };
                result = apply_decision_context_with_dm(&mut g, &mut queue, &mut state, &context, &mut dm);
            }
            assert_eq!(count(&g, source, &key), 0); assert!(g.stack.is_empty());
            assert!(g.effect_store.delayed_triggers.is_empty()); assert_eq!(g.player(A).unwrap().mana_pool.total(), 1);
            if name == "Farrelite Priest" || name == "Initiates of the Ebon Hand" {
                g.player_mut(A).unwrap().mana_pool = Default::default();
                assert!(ironsmith::special_actions::perform_activate_mana_ability(&mut g, A, source, slot, &mut SelectFirstDecisionMaker).is_err());
                assert_eq!(count(&g, source, &key), 0); assert!(g.effect_store.delayed_triggers.is_empty());
            }
        }
    }
}
#[test]
fn actual_spell_payment_can_activate_the_same_converter_four_times() {
    struct Convert { source: ObjectId, slot: usize, outer: ObjectId, remaining: u32 }
    impl DecisionMaker for Convert {
        fn decide_mana_payment(&mut self, g: &GameState, c: &ironsmith::decisions::context::ManaPaymentContext)
            -> ironsmith::mana_payment::ManaPaymentResponse {
            if c.source == self.outer && self.remaining > 0 {
                self.remaining -= 1;
                ironsmith::mana_payment::ManaPaymentResponse::Activate { source: self.source, ability_index: self.slot }
            } else { SelectFirstDecisionMaker.decide_mana_payment(g, c) }
        }
    }
    for name in ["Farrelite Priest", "Initiates of the Ebon Hand"] { for definition in definitions(name) {
        let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let slot = index(&g, source, 0); let key = origin(&g, source, slot);
        let cost = if name == "Farrelite Priest" { "{W}{W}{W}{W}" } else { "{B}{B}{B}{B}" };
        let spell = body("Conversion consumer", &format!("Mana cost: {cost}\nType: Sorcery\nYou gain 2 life."));
        let spell = g.create_object_from_definition(&spell, A, Zone::Hand);
        add(&mut g, A, ManaSymbol::Colorless, 4);
        let action = compute_legal_actions(&g, A).unwrap().into_iter().find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).unwrap();
        let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
        let mut dm = Convert { source, slot, outer: spell, remaining: 4 };
        let mut progress = apply_priority_response_with_dm(&mut g, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..80 {
            if !state.has_pending_action() { break; }
            let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("nested conversion payment") };
            // A spell receives its stack incarnation before payment. Match that exact root.
            if let ironsmith::decisions::context::DecisionContext::ManaPayment(ref c) = context {
                if c.source != source { dm.outer = c.source; }
            }
            progress = apply_decision_context_with_dm(&mut g, &mut queue, &mut state, &context, &mut dm).unwrap();
        }
        assert!(!state.has_pending_action()); assert_eq!(count(&g, source, &key), 4);
        assert_eq!(g.effect_store.delayed_triggers.len(), 1); assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
        settle(&mut g); assert_eq!(g.player(A).unwrap().life, 22);
        end_step(&mut g, B); assert!(g.object(source).is_none());
    }}
}

#[test]
fn pending_mana_resolution_rolls_back_the_observed_fourth_use_and_delayed_registration() {
    struct Pause { pause: bool, pending: bool }
    impl DecisionMaker for Pause {
        fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.pending = self.pause; !self.pause
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    for mut definition in definitions("Farrelite Priest") {
        // Preserve every original instruction, then append a native suspension witness.
        let AbilityKind::Activated(ability) = &mut definition.abilities[0].kind else { panic!("mana ability") };
        ability.effects.push(Effect::may(vec![Effect::gain_life(1)]));
        let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let key = origin(&g, source, 0);
        for _ in 0..3 { add(&mut g, A, ManaSymbol::Colorless, 1);
            ironsmith::special_actions::perform_activate_mana_ability(&mut g, A, source, 0, &mut SelectFirstDecisionMaker).unwrap(); }
        g.player_mut(A).unwrap().mana_pool = Default::default(); add(&mut g, A, ManaSymbol::Colorless, 1);
        let life = g.player(A).unwrap().life; let saved = g.clone();
        let mut dm = Pause { pause: true, pending: false };
        ironsmith::special_actions::perform_activate_mana_ability(&mut g, A, source, 0, &mut dm).unwrap();
        assert!(dm.pending); assert_eq!(count(&g, source, &key), 3);
        assert_eq!(g.player(A).unwrap().mana_pool.colorless, 1); assert_eq!(g.player(A).unwrap().mana_pool.white, 0);
        assert!(g.effect_store.delayed_triggers.is_empty()); assert_eq!(g.player(A).unwrap().life, life);
        g = saved; dm.pause = false; dm.pending = false;
        ironsmith::special_actions::perform_activate_mana_ability(&mut g, A, source, 0, &mut dm).unwrap();
        assert_eq!(count(&g, source, &key), 4); assert_eq!(g.effect_store.delayed_triggers.len(), 1);
        assert_eq!(g.player(A).unwrap().mana_pool.white, 1); assert_eq!(g.player(A).unwrap().life, life + 1);
    }
}
#[test]
fn a_copied_ability_keeps_the_original_count_and_its_own_delayed_controller() {
    for definition in definitions("Dragon Whelp") {
        let mut g = game(); let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let key = origin(&g, source, index(&g, source, 0)); add(&mut g, A, ManaSymbol::Red, 4);
        for _ in 0..4 { activate(&mut g, source, 0); }
        copy_top(&mut g, B); assert_eq!(count(&g, source, &key), 4);
        settle(&mut g); assert_eq!(g.effect_store.delayed_triggers.len(), 5);
        assert_eq!(g.effect_store.delayed_triggers.iter().filter(|trigger| trigger.controller == B).count(), 1);
        g.set_current_controller(source, B).unwrap(); end_step(&mut g, B);
        assert!(g.object(source).is_none());
    }
}

#[test]
fn same_slot_face_replacement_and_borrowed_donor_faces_have_separate_definition_history() {
    use ironsmith::card::LinkedFaceLayout;
    for mut front in definitions("Dragon Whelp") { for borrowed in [false, true] {
        let mut back = body("Threshold reverse face", "Type: Creature — Dragon\nPower/Toughness: 2/3\nFlying\n{1}{R}: This creature gets +1/+0 until end of turn. If this ability has been activated four or more times this turn, sacrifice this creature at the beginning of the next end step.");
        front.card.linked_face_layout = LinkedFaceLayout::TransformLike; front.card.transforming_dfc = true;
        back.card.linked_face_layout = LinkedFaceLayout::TransformLike; back.card.transforming_dfc = true;
        front.card.other_face = Some(back.card.id); front.card.other_face_name = Some(back.card.name.clone());
        back.card.other_face = Some(front.card.id); back.card.other_face_name = Some(front.card.name.clone());
        let mut g = game(); g.register_linked_face_definition(&front); g.register_linked_face_definition(&back);
        let donor = g.create_object_from_definition(&front, A, Zone::Battlefield);
        let source = if borrowed {
            let mut host = body("Borrowing host", "Type: Creature — Ooze\nPower/Toughness: 2/3");
            host.abilities.push(ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::copy_activated_abilities(
                ironsmith::static_abilities::CopyActivatedAbilities::new(ironsmith::filter::ObjectFilter::specific(donor)))));
            g.create_object_from_definition(&host, A, Zone::Battlefield)
        } else { donor };
        let key = origin(&g, source, index(&g, source, 0));
        add(&mut g, A, ManaSymbol::Red, 6); add(&mut g, A, ManaSymbol::Colorless, 2);
        for _ in 0..3 { activate(&mut g, source, 0); }
        let front_entry = g.stack.last().unwrap().clone();
        assert!(g.transform_permanent(donor).unwrap());
        // Printed(slot) and Borrowed(donor, Printed(slot)) both still compare equal.
        assert_eq!(key, origin(&g, source, index(&g, source, 0)));
        assert_eq!(count(&g, source, &key), 0);
        activate(&mut g, source, 0); let back_entry = g.stack.last().unwrap().clone();
        assert_ne!(front_entry.activation_definition, back_entry.activation_definition);
        settle(&mut g); assert!(g.effect_store.delayed_triggers.is_empty());
        assert_eq!(count(&g, source, &key), 1);
        assert!(g.transform_permanent(donor).unwrap()); assert_eq!(count(&g, source, &key), 3);
        activate(&mut g, source, 0); copy_top(&mut g, A);
        assert!(g.transform_permanent(donor).unwrap());
        settle(&mut g); assert_eq!(g.effect_store.delayed_triggers.len(), 2);
        assert_eq!(count(&g, source, &key), 1);
    }}
}

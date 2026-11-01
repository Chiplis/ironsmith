//! Native behavior of equivalent Revolution counter reminders, using existing effects.
use ironsmith_compiler_runtime::compile_builder_to_runtime_definition;
use ironsmith::ability::AbilityKind;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, SelectOptionsContext};
use ironsmith::object::CounterType;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};

struct Choice { accept: bool, mode: usize }
impl DecisionMaker for Choice {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.accept }
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        ctx.candidates.iter().filter(|c| c.legal).take(ctx.min).map(|c| c.id).collect()
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if let Some(option) = ctx.options.iter().filter(|o| o.legal).nth(self.mode) {
            vec![option.index]
        } else { SelectFirstDecisionMaker.decide_options(game, ctx) }
    }
}
fn card(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), name)
        .with_custom_keyword("Counter lesson");
    ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
        compile_builder_to_runtime_definition(builder, text, false)
    }).unwrap()
}
fn creature(name: &str, rules: &str) -> ironsmith::cards::CardDefinition {
    card(name, &format!("Type: Creature\nPower/Toughness: 2/2\n{rules}"))
}
fn resolve_spell(game: &mut GameState, definition: &ironsmith::cards::CardDefinition, source: ObjectId, dm: &mut Choice) {
    let mut ctx = ironsmith::effects::EffectContext::new(source, PlayerId::from_index(0), dm);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        ironsmith::effects::execute_effect(game, effect, &mut ctx).unwrap();
    }
}

fn enter_and_resolve(game: &mut GameState, definition: &ironsmith::cards::CardDefinition, dm: &mut Choice) -> ObjectId {
    let a = PlayerId::from_index(0);
    let hand = game.create_object_from_definition(definition, a, Zone::Hand);
    let stable = game.object(hand).unwrap().stable_id;
    let movement = ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(
        ironsmith::target::ChooseSpec::SpecificObject(hand), Zone::Battlefield, false));
    let mut ctx = ironsmith::effects::EffectContext::new(hand, a, dm);
    let outcome = ironsmith::effects::execute_effect(game, &movement, &mut ctx).unwrap();
    for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
    let source = game.find_object_by_stable_id(stable).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, dm).unwrap();
    assert_eq!(queue.entries.len(), 1, "entry must trigger even when the keyword predicate is false");
    ironsmith::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
    source
}

#[test]
fn revolution_crystallize_optional_counter_or_treasure_preserves_controller_and_no_target() {
    let spell = card("Crystal choice", "Type: Sorcery\nYou may put a +1/+1 counter on a creature you control. If you don't, create a Treasure token.");
    for available in [false, true] { for accept in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&spell, a, Zone::Stack);
        let opponent = game.create_object_from_definition(&creature("Opponent", ""), b, Zone::Battlefield);
        // Shroud demonstrates that the chosen creature is not targeted.
        let own = available.then(|| game.create_object_from_definition(&creature("Own", "Shroud"), a, Zone::Battlefield));
        resolve_spell(&mut game, &spell, source, &mut Choice { accept, mode: 0 });
        assert_eq!(game.counter_count(opponent, CounterType::PlusOnePlusOne), 0);
        if let Some(own) = own { assert_eq!(game.counter_count(own, CounterType::PlusOnePlusOne), u32::from(accept)); }
        let treasures = game.battlefield.iter().filter(|id| game.object(**id).unwrap().subtypes.contains(&ironsmith::types::Subtype::Treasure)).count();
        assert_eq!(treasures, usize::from(!accept || !available), "available={available} accept={accept}");
    } }
}
#[test]
fn revolution_teach_checks_existing_keyword_before_counter_placement() {
    for keyword in ["flying", "lifelink", "menace", "trample", "vigilance"] {
        for already in [false, true] {
            let rules = format!("When this creature enters, if this creature has {keyword}, put a +1/+1 counter on this creature. Otherwise, put a {keyword} counter on this creature.");
            let definition = creature("Pupil", &format!("{}\n{rules}", if already { keyword } else { "" }));
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let source = enter_and_resolve(&mut game, &definition, &mut Choice { accept: true, mode: 0 });
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), u32::from(already), "{keyword} already={already}");
            let ability = match keyword {
                "flying" => ironsmith::static_abilities::StaticAbilityId::Flying,
                "lifelink" => ironsmith::static_abilities::StaticAbilityId::Lifelink,
                "menace" => ironsmith::static_abilities::StaticAbilityId::Menace,
                "trample" => ironsmith::static_abilities::StaticAbilityId::Trample,
                _ => ironsmith::static_abilities::StaticAbilityId::Vigilance,
            };
            assert!(game.current_has_static_ability_id(source, ability));
        }
    }
}
#[test]
fn revolution_propagate_either_reader_preserves_both_counter_and_spawn_branches() {
    for count in [1, 2, 3] { for mode in [0, 1] {
        let rules = format!("When this creature enters, either put {count} +1/+1 counters on this creature or create {count} 0/1 colorless Eldrazi Spawn creature tokens with \"Sacrifice this creature: Add {{C}}.\"");
        let definition = creature("Brood choice", &rules);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let source = enter_and_resolve(&mut game, &definition, &mut Choice { accept: true, mode });
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), if mode == 0 { count } else { 0 });
        let tokens = game.battlefield.iter().filter(|id| **id != source).copied().collect::<Vec<_>>();
        assert_eq!(tokens.len(), if mode == 1 { count as usize } else { 0 });
        for token in tokens {
            assert_eq!(game.calculated_power(token), Some(0));
            assert_eq!(game.calculated_toughness(token), Some(1));
            assert_eq!(game.controller_of_id(token), Some(a));
            assert!(game.object(token).unwrap().abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Activated(_))));
        }
    } }
}

#[test]
fn revolution_crystallize_full_cast_has_no_targets_and_resolves_with_shroud() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    let spell = card("Crystal casting", "Type: Sorcery\nMana cost: {0}\nYou may put a +1/+1 counter on a creature you control. If you don't, create a Treasure token.");
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let a = PlayerId::from_index(0);
    game.turn.active_player = a; game.turn.priority_player = Some(a);
    game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
    let own = game.create_object_from_definition(&creature("Protected pupil", "Shroud"), a, Zone::Battlefield);
    let hand = game.create_object_from_definition(&spell, a, Zone::Hand);
    let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)).unwrap();
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = Choice { accept: true, mode: 0 };
    let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    assert!(state.pending_cast.is_none());
    assert_eq!(game.stack.len(), 1);
    assert!(game.stack[0].targets.is_empty(), "resolution-time choice cannot be turned into a target");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(game.counter_count(own, CounterType::PlusOnePlusOne), 1);
    assert!(!game.battlefield.iter().any(|id| game.object(*id).unwrap().subtypes.contains(&ironsmith::types::Subtype::Treasure)));
}

#[test]
fn revolution_teach_target_prelude_binds_both_branches_to_one_legal_creature() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    struct Pick(ObjectId);
    impl DecisionMaker for Pick {
        fn decide_targets(&mut self, _: &GameState, _: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::Target> { vec![ironsmith::Target::Object(self.0)] }
    }
    let spell = card("Targeted lesson", "Type: Sorcery\nMana cost: {0}\nChoose target creature you control. If it has flying, put a +1/+1 counter on it. Otherwise, put a flying counter on it.");
    for already in [false, true] { for shroud in [false, true] { for own in [false, true] { for blocked_plus in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        game.turn.active_player = a; game.turn.priority_player = Some(a);
        game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        let rules = format!("{}\n{}", if already { "Flying" } else { "" }, if shroud { "Shroud" } else { "" });
        let mut pupil = creature("Pupil", &rules);
        if blocked_plus {
            pupil.abilities.push(ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::restriction(
                    ironsmith::effect::Restriction::HaveCounterTypePlaced(
                        ironsmith::target::ObjectFilter::source(), CounterType::PlusOnePlusOne,
                    ), "Plus-one counters cannot be placed on this pupil".into(),
                ),
            ));
        }
        let target = game.create_object_from_definition(&pupil, if own { a } else { b }, Zone::Battlefield);
        let hand = game.create_object_from_definition(&spell, a, Zone::Hand);
        let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand));
        assert_eq!(action.is_some(), own && !shroud, "own={own} shroud={shroud}");
        let Some(action) = action else { continue; };
        let mut state = PriorityLoopState::new(2); let mut queue = ironsmith::triggers::TriggerQueue::new(); let mut dm = Pick(target);
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..64 {
            if state.pending_cast.is_none() && state.pending_method_selection.is_none() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
        }
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets.len(), 1);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), u32::from(already && !blocked_plus));
        assert_eq!(game.counter_count(target, CounterType::Flying), u32::from(!already), "a failed true-arm counter placement must not run the false arm");
        assert!(game.current_has_static_ability_id(target, ironsmith::static_abilities::StaticAbilityId::Flying));
        assert_eq!(game.counter_count(hand, CounterType::PlusOnePlusOne), 0);
    } } } }
}

#[test]
fn revolution_teach_explicit_source_choice_preserves_the_source_identity() {
    for object in ["this creature", "it", "Explicit pupil"] { for already in [false, true] {
        let definition = creature("Explicit pupil", &format!("{}\nWhen this creature enters, choose {object}. If it has flying, put a +1/+1 counter on it. Otherwise, put a flying counter on it.", if already { "Flying" } else { "" }));
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let unrelated = game.create_object_from_definition(&creature("Unrelated", if already { "" } else { "Flying" }), a, Zone::Battlefield);
        let source = enter_and_resolve(&mut game, &definition, &mut Choice { accept: true, mode: 0 });
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), u32::from(already));
        assert_eq!(game.counter_count(source, CounterType::Flying), u32::from(!already));
        assert_eq!(game.counter_count(unrelated, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(unrelated, CounterType::Flying), 0);
    } }
}

#[test]
fn revolution_teach_zero_optional_targets_never_falls_back_to_the_source() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    struct None;
    impl DecisionMaker for None {
        fn decide_targets(&mut self, _: &GameState, _: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::Target> { Vec::new() }
    }
    let spell = card("Optional lesson", "Type: Sorcery\nMana cost: {0}\nChoose up to one target creature you control. If it has flying, put a +1/+1 counter on it. Otherwise, put a flying counter on it.");
    for own_available in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        game.turn.active_player = a; game.turn.priority_player = Some(a);
        game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = Option::None;
        let opponent = game.create_object_from_definition(&creature("Opponent", "Flying"), b, Zone::Battlefield);
        let own = own_available.then(|| game.create_object_from_definition(&creature("Available", ""), a, Zone::Battlefield));
        let hand = game.create_object_from_definition(&spell, a, Zone::Hand);
        let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)).expect("zero targets must allow casting");
        let mut state = PriorityLoopState::new(2); let mut queue = ironsmith::triggers::TriggerQueue::new(); let mut dm = None;
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..64 {
            if state.pending_cast.is_none() && state.pending_method_selection.is_none() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
        }
        assert!(game.stack[0].targets.is_empty(), "an available optional target can be declined");
        let source = game.stack[0].object_id;
        // Execute before the spell's final zone change so an erroneous source
        // counter cannot disappear with its former stack incarnation.
        let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm).with_targets(Vec::new());
        for effect in spell.spell_effect.as_ref().unwrap().all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        for object in [Some(source), Some(opponent), own].into_iter().flatten() {
            assert_eq!(game.counter_count(object, CounterType::PlusOnePlusOne), 0);
            assert_eq!(game.counter_count(object, CounterType::Flying), 0);
        }
    }
}

#[test]
fn revolution_teach_followup_reads_actual_counter_kind_and_recipient_receipt() {
    let spell = card("Placement receipt", "Type: Sorcery\nMana cost: {0}\nChoose target creature you control. If it has trample, put a +1/+1 counter on it. Otherwise, put a trample counter on it. If you put a +1/+1 counter on it this way, it can't be blocked this turn.");
    for already in [false, true] { for blocked_plus in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&spell, a, Zone::Stack);
        let mut pupil = creature("Recipient", if already { "Trample" } else { "" });
        if blocked_plus {
            pupil.abilities.push(ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::restriction(
                    ironsmith::effect::Restriction::HaveCounterTypePlaced(
                        ironsmith::target::ObjectFilter::source(), CounterType::PlusOnePlusOne,
                    ), "Plus-one counters cannot be placed on this recipient".into(),
                ),
            ));
        }
        let target = game.create_object_from_definition(&pupil, a, Zone::Battlefield);
        // Existing markers must not stand in for this instruction's receipt.
        game.object_mut(target).unwrap().counters.insert(CounterType::PlusOnePlusOne, 3);
        let blocker = game.create_object_from_definition(&creature("Blocker", ""), b, Zone::Battlefield);
        // Direct effect execution bypasses the priority loop's refresh.
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.can_have_counter_type_placed(target, CounterType::PlusOnePlusOne), !blocked_plus);
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, a)
            .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(target)]);
        for effect in spell.spell_effect.as_ref().unwrap().all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        let placed_plus = already && !blocked_plus;
        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 3 + u32::from(placed_plus));
        assert_eq!(game.counter_count(target, CounterType::Trample), u32::from(!already));
        let attacker = game.object(target).unwrap();
        let blocking_creature = game.object(blocker).unwrap();
        let can_block = ironsmith::rules::combat::can_block(attacker, blocking_creature, &game);
        assert_eq!(can_block, !placed_plus,
            "only actual +1/+1 placement on the chosen recipient grants evasion; already={already} blocked={blocked_plus}");
    } }
}

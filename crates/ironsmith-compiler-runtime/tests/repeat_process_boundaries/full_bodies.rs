//! Independent complete-body expectations. Source-authored, UNRUN.
//! These cases use real announcement/payment and trigger queues. The smaller
//! parent-module cases intentionally remain isolated process-unit scenarios.
use super::*;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{ManaPaymentContext, TargetsContext};
use ironsmith::effects::{ConditionalEffect, RepeatEffectsEffect, RepeatProcessEffect,
    RepeatProcessPromptEffect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, generate_and_queue_step_triggers,
    put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, CounterType, GameProgress, Phase, Step, Target};

fn nodes(program: &[Effect]) -> Vec<Effect> {
    fn visit(effect: &Effect, result: &mut Vec<Effect>) {
        result.push(effect.clone());
        effect.visit_child_effects(&mut |child| visit(child, result));
    }
    let mut result = Vec::new();
    for effect in program { visit(effect, &mut result); }
    result
}

#[test]
fn exact_full_body_membership_metadata_and_typed_owners_survive_all_routes() {
    let frozen = rows();
    assert_eq!(frozen.len(), 9);
    let ids = frozen.iter().map(|row| row["source"]["oracle_id"].as_str().unwrap())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 9);
    assert!(ids.contains("3bcc378c-4470-4757-a0d5-025a32c918ea"));
    assert!(!FULL_BODIES.iter().any(|(id, _)| *id == "3bcc378c-4470-4757-a0d5-025a32c918ea"));
    // Expectations are independently authored from the frozen complete bodies,
    // rather than inferred from another compiler route or debug substrings.
    for (name, kind, cost, pt, loyalty, ability_kinds) in [
        ("Another Round", CardType::Sorcery, "{X}{X}{2}{W}", None, None, (0, 0, 0)),
        ("Claim Jumper", CardType::Creature, "{2}{W}", Some((3, 3)), None, (1, 1, 0)),
        ("Countryside Crusher", CardType::Creature, "{1}{R}{R}", Some((3, 3)), None, (0, 2, 0)),
        ("Grindstone", CardType::Artifact, "{1}", None, None, (0, 0, 1)),
        ("Professor Onyx", CardType::Planeswalker, "{4}{B}{B}", None, Some(5), (0, 1, 3)),
        ("Scalpelexis", CardType::Creature, "{4}{U}", Some((1, 5)), None, (1, 1, 0)),
        ("Trade Secrets", CardType::Sorcery, "{1}{U}{U}", None, None, (0, 0, 0)),
        ("Zimone and Dina", CardType::Creature, "{B}{G}{U}", Some((3, 4)), None, (0, 1, 1)),
    ] {
        for definition in definitions(name) {
            assert_eq!(definition.card.name, name);
            assert_eq!(definition.card.card_types, vec![kind]);
            assert_eq!(definition.card.mana_cost.as_ref().unwrap().to_oracle(), cost);
            assert_eq!(definition.card.power_toughness,
                pt.map(|(p, t)| ironsmith::card::PowerToughness::fixed(p, t)));
            assert_eq!(definition.card.loyalty, loyalty);
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert!(definition.additional_cost.is_free());
            assert!(definition.alternative_casts.is_empty());
            assert!(definition.optional_costs.is_empty());
            assert_eq!(definition.spell_effect.is_some(), kind == CardType::Sorcery);
            let observed = definition.abilities.iter().fold((0, 0, 0), |mut count, ability| {
                match &ability.kind {
                    AbilityKind::Static(_) => count.0 += 1,
                    AbilityKind::Triggered(_) => count.1 += 1,
                    AbilityKind::Activated(_) => count.2 += 1,
                }
                count
            });
            assert_eq!(observed, ability_kinds, "{name}: every independent ability must survive");
            let program = nodes(&effects(&definition, name));
            match name {
                "Another Round" | "Professor Onyx" => {
                    let repeated = program.iter().filter_map(|e| e.downcast_ref::<RepeatEffectsEffect>()).collect::<Vec<_>>();
                    assert_eq!(repeated.len(), 1);
                    let expected = if name == "Another Round" { ironsmith::effect::Value::X }
                        else { ironsmith::effect::Value::Fixed(6) };
                    assert_eq!(repeated[0].count, ironsmith::effect::Value::Add(
                        Box::new(ironsmith::effect::Value::Fixed(1)), Box::new(expected)));
                    let marker = if name == "Another Round" { "repeat this process x more times" }
                        else { "repeat this process six more times" };
                    assert!(definition.canonical_text.to_ascii_lowercase().contains(marker),
                        "the entire process boundary must survive compiled presentation: {}", definition.canonical_text);
                }
                "Countryside Crusher" => {
                    let captured = program.iter().filter_map(|e| e.downcast_ref::<ConditionalEffect>())
                        .filter(|gate| gate.capture_condition_result).collect::<Vec<_>>();
                    assert_eq!(captured.len(), 1);
                    assert!(captured[0].if_false.is_empty());
                    let repeated = program.iter().find_map(|e| e.downcast_ref::<RepeatProcessEffect>()).unwrap();
                    assert_eq!(repeated.predicate, ironsmith::effect::EffectPredicate::Value(
                        ironsmith::effect::Comparison::GreaterThan(0)));
                }
                "Scalpelexis" => {
                    let repeated = program.iter().find_map(|e| e.downcast_ref::<RepeatProcessEffect>()).unwrap();
                    assert_eq!(repeated.predicate, ironsmith::effect::EffectPredicate::AffectedObjectsShare {
                        required_count: 2, characteristic: ironsmith_core::ObjectCharacteristic::Name,
                    });
                }
                "Trade Secrets" => {
                    let prompts = program.iter().filter_map(|e| e.downcast_ref::<RepeatProcessPromptEffect>()).collect::<Vec<_>>();
                    assert_eq!(prompts.len(), 1);
                    assert!(matches!(&prompts[0].decider,
                        Some(PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner))
                            if **inner == PlayerFilter::Opponent),
                        "the continuation belongs to the announced opponent, including a typed follow-up alias");
                }
                "Claim Jumper" | "Zimone and Dina" => {
                    assert!(program.iter().any(|e| e.downcast_ref::<ConditionalEffect>().is_some()));
                    assert!(!program.iter().any(|e| e.downcast_ref::<RepeatProcessEffect>().is_some()),
                        "the live gate authorizes only one additional execution");
                }
                "Grindstone" => {
                    let repeated = program.iter().find_map(|e| e.downcast_ref::<RepeatProcessEffect>()).unwrap();
                    let ironsmith::effect::EffectPredicate::PriorEffectResult(surface) = &repeated.predicate
                        else { panic!("mill continuation must name its exact typed mill receipt"); };
                    assert_eq!(surface.action, ironsmith::effect::PriorEffectAction::Milled);
                    assert_eq!(surface.required_count, Some(2));
                    assert_eq!(surface.shared_characteristic, Some(ironsmith_core::ObjectCharacteristic::Color));
                }
                _ => unreachable!(),
            }
        }
    }
}

#[derive(Default)]
struct Choices {
    answers: Vec<bool>, boolean_calls: usize, target: Option<Target>, target_calls: usize,
    numbers: Vec<(PlayerId, u32)>, number: u32, x: u32, fail_find: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        let answer = self.answers.get(self.boolean_calls).copied().unwrap_or(false);
        self.boolean_calls += 1;
        answer && ctx.can_accept
    }
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        let answer = if ctx.is_x_value { self.x } else { self.number }.clamp(ctx.min, ctx.max);
        self.numbers.push((ctx.player, answer));
        answer
    }
    fn decide_objects(&mut self, g: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if self.fail_find && (ctx.min == 0 || ctx.allow_partial_completion) { return Vec::new(); }
        SelectFirstDecisionMaker.decide_objects(g, ctx)
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.target_calls += 1;
        assert_eq!(ctx.requirements.len(), 1);
        let target = self.target.expect("the full-body scenario supplies its announced target");
        assert!(ctx.requirements[0].legal_targets.contains(&target));
        vec![target]
    }
    fn decide_mana_payment(&mut self, _: &GameState, ctx: &ManaPaymentContext)
        -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: ctx.plan.id, request_hash: ctx.plan.request_hash,
        }
    }
}
fn action_game() -> GameState {
    let mut g = game();
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g
}
fn queue(g: &mut GameState, dm: &mut Choices) {
    put_triggers_on_stack_with_dm(g, &mut TriggerQueue::new(), dm).unwrap();
}
fn settle(g: &mut GameState, dm: &mut Choices) {
    for _ in 0..40 {
        queue(g, dm);
        if g.stack_is_empty() { return; }
        resolve_stack_entry_with(g, dm).unwrap();
    }
    panic!("bounded full-body scenario failed to settle");
}
fn announce(g: &mut GameState, action: LegalAction, dm: &mut Choices) {
    g.turn.priority_player = Some(A);
    assert!(compute_legal_actions(g, A).unwrap().contains(&action));
    let mut state = PriorityLoopState::new(g.players.len());
    let mut triggers = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(g, &mut triggers, &mut state,
        &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() { break; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(g, &mut triggers, &mut state, &ctx, dm).unwrap();
    }
    assert!(!state.has_pending_action());
    put_triggers_on_stack_with_dm(g, &mut triggers, dm).unwrap();
}
fn cast(g: &mut GameState, card: ObjectId, dm: &mut Choices) {
    announce(g, LegalAction::CastSpell { spell_id: card, from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal }, dm);
}
fn activate(g: &mut GameState, source: ObjectId, nth: usize, dm: &mut Choices) {
    let index = g.current_abilities(source).unwrap().iter().enumerate()
        .filter(|(_, ability)| matches!(&ability.kind, AbilityKind::Activated(_)))
        .nth(nth).unwrap().0;
    announce(g, LegalAction::ActivateAbility { source, ability_index: index }, dm);
}
fn apply(g: &mut GameState, source: ObjectId, effect: Effect, dm: &mut Choices) {
    let outcome = ironsmith::effects::execute_effect(g, &effect,
        &mut ExecutionContext::new(source, A, dm)).unwrap();
    for event in outcome.events { g.queue_trigger_event(Default::default(), event); }
    queue(g, dm);
}

#[test]
fn printed_spell_costs_and_announced_target_survive_repetition() {
    for name in ["Another Round", "Trade Secrets"] {
        for definition in definitions(name) {
            let mut g = action_game(); library(&mut g, A, 12); library(&mut g, B, 12);
            let source = g.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choices { x: 2, number: 1, target: Some(Target::Player(B)),
                answers: vec![true, false], ..Default::default() };
            if name == "Another Round" {
                g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::White, 1);
                g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 6);
            } else {
                g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 2);
                g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
            }
            cast(&mut g, source, &mut dm);
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 0, "the complete printed price is paid once");
            assert_eq!(dm.target_calls, usize::from(name == "Trade Secrets"));
            settle(&mut g, &mut dm);
            assert_eq!(dm.target_calls, usize::from(name == "Trade Secrets"), "repetition cannot reannounce targets");
            if name == "Trade Secrets" {
                assert_eq!(g.player(A).unwrap().hand.len(), 2);
                assert_eq!(g.player(B).unwrap().hand.len(), 4);
                assert_eq!(dm.numbers, vec![(A, 1), (A, 1)], "the controller owns each up-to-four choice");
            } else {
                assert!(dm.numbers.iter().any(|(player, n)| *player == A && *n == 2));
            }
        }
    }
}

#[test]
fn grindstone_pays_three_and_taps_before_the_targeted_mill_program() {
    for definition in definitions("Grindstone") {
        let mut g = action_game(); library(&mut g, B, 4);
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut dm = Choices { target: Some(Target::Player(B)), ..Default::default() };
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
        activate(&mut g, source, 0, &mut dm);
        assert!(g.is_tapped(source));
        assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(dm.target_calls, 1);
        assert_eq!(g.player(B).unwrap().library.len(), 4, "milling waits for resolution");
        settle(&mut g, &mut dm);
        assert_eq!(g.player(B).unwrap().library.len(), 2);
        assert_eq!(g.player(B).unwrap().graveyard.len(), 2);
        assert_eq!(dm.target_calls, 1);
    }
}

#[test]
fn claim_entry_keeps_vigilance_intervening_if_and_fail_to_find_search_receipt() {
    for definition in definitions("Claim Jumper") {
        for (enemy_lands, remove_before_resolution, expected_offers) in [(1, false, 0), (3, true, 0), (3, false, 2)] {
            let mut g = action_game();
            card(&mut g, A, Zone::Battlefield, "Own land", "Type: Land");
            let enemies = (0..enemy_lands).map(|i|
                card(&mut g, B, Zone::Battlefield, &format!("Enemy {i}"), "Type: Land")).collect::<Vec<_>>();
            library(&mut g, A, 3);
            let source = g.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choices { answers: vec![true, true], fail_find: true, ..Default::default() };
            let entry = g.move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut dm).unwrap();
            assert!(!entry.pending);
            assert!(entry.programs.is_empty());
            let entered = entry.original.into_result().unwrap().new_id;
            assert!(g.current_has_static_ability_id(entered, StaticAbilityId::Vigilance));
            queue(&mut g, &mut dm);
            assert_eq!(g.stack.len(), usize::from(enemy_lands > 1));
            if remove_before_resolution {
                for land in enemies { g.move_object_by_game_rule(land, Zone::Exile).unwrap(); }
            }
            settle(&mut g, &mut dm);
            assert_eq!(dm.boolean_calls, expected_offers);
            assert_eq!(g.player(A).unwrap().library.len(), 3, "a legal failed search still completes");
        }
    }
}

#[test]
fn countryside_upkeep_and_from_anywhere_counter_trigger_both_execute() {
    for definition in definitions("Countryside Crusher") {
        let mut g = action_game();
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stop = card(&mut g, A, Zone::Library, "Stop", "Type: Sorcery");
        for i in 0..2 { card(&mut g, A, Zone::Library, &format!("Top land {i}"), "Type: Land"); }
        g.turn.phase = Phase::Beginning; g.turn.step = Some(Step::Upkeep);
        let mut triggers = TriggerQueue::new(); let mut dm = Choices::default();
        generate_and_queue_step_triggers(&mut g, &mut triggers);
        put_triggers_on_stack_with_dm(&mut g, &mut triggers, &mut dm).unwrap();
        assert_eq!(g.stack.len(), 1);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().library.as_slice(), &[stop]);
        assert_eq!(g.counter_count(source, CounterType::PlusOnePlusOne), 2);
        for (owner, zone, expected) in [(A, Zone::Hand, 3), (A, Zone::Battlefield, 4), (B, Zone::Hand, 4)] {
            let land = card(&mut g, owner, zone, "Other land", "Type: Land");
            apply(&mut g, source, Effect::move_to_zone(ChooseSpec::SpecificObject(land), Zone::Graveyard, false), &mut dm);
            settle(&mut g, &mut dm);
            assert_eq!(g.counter_count(source, CounterType::PlusOnePlusOne), expected);
        }
    }
}

#[test]
fn scalpelexis_full_trigger_uses_combat_source_and_actual_damaged_player() {
    for definition in definitions("Scalpelexis") {
        for (combat, other_source, should_exile) in [(true, false, true), (false, false, false), (true, true, false)] {
            let mut g = action_game(); library(&mut g, B, 6); library(&mut g, C, 6);
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            assert!(g.current_has_static_ability_id(source, StaticAbilityId::Flying));
            let dealer = if other_source { card(&mut g, A, Zone::Battlefield, "Other dealer", "Type: Creature\nPower/Toughness: 1/1") } else { source };
            let mut dm = Choices::default();
            apply(&mut g, dealer, Effect::new(ironsmith::effects::DealDamageEffect::new(1,
                ChooseSpec::SpecificPlayer(C)).with_combat(combat)), &mut dm);
            settle(&mut g, &mut dm);
            assert_eq!(g.player(B).unwrap().library.len(), 6);
            assert_eq!(g.player(C).unwrap().library.len(), if should_exile { 2 } else { 6 });
            assert_eq!(g.exile.len(), if should_exile { 4 } else { 0 });
        }
    }
}

#[test]
fn onyx_paid_loyalty_bodies_preserve_look_select_rest_and_each_players_greatest_power() {
    for definition in definitions("Professor Onyx") {
        for ability in [0, 1, 2] {
            let mut g = action_game(); library(&mut g, A, 4);
            let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let mut dm = Choices { answers: vec![true; 14], ..Default::default() };
            if ability == 2 {
                g.object_mut(source).unwrap().counters.insert(CounterType::Loyalty, 9);
                for player in [B, C] {
                    for i in 0..2 { card(&mut g, player, Zone::Hand, &format!("Discard {i}"), "Type: Sorcery"); }
                }
            }
            for player in [A, B, C] {
                card(&mut g, player, Zone::Battlefield, "Small", "Type: Creature\nPower/Toughness: 1/1");
                card(&mut g, player, Zone::Battlefield, "Large", "Type: Creature\nPower/Toughness: 5/5");
            }
            activate(&mut g, source, ability, &mut dm);
            assert_eq!(g.counter_count(source, CounterType::Loyalty), [6, 2, 1][ability]);
            settle(&mut g, &mut dm);
            match ability {
                0 => {
                    assert_eq!(g.player(A).unwrap().life, 29);
                    assert_eq!(g.player(A).unwrap().hand.len(), 1);
                    assert_eq!(g.player(A).unwrap().graveyard.len(), 2);
                    assert_eq!(g.player(A).unwrap().library.len(), 1);
                }
                1 => {
                    for player in [B, C] {
                        assert_eq!(g.player(player).unwrap().graveyard.len(), 1);
                        assert_eq!(g.object(g.player(player).unwrap().graveyard[0]).unwrap().name, "Large");
                    }
                    assert!(g.player(A).unwrap().graveyard.is_empty());
                }
                2 => {
                    for player in [B, C] {
                        assert!(g.player(player).unwrap().hand.is_empty());
                        assert_eq!(g.player(player).unwrap().graveyard.len(), 2);
                        assert_eq!(g.player(player).unwrap().life, 15, "two discards and five fallback rounds");
                    }
                    assert_eq!(g.player(A).unwrap().life, 30);
                }
                _ => unreachable!(),
            }
        }
    }
}

#[test]
fn onyx_magecraft_keeps_both_cast_and_copy_bodies() {
    for definition in definitions("Professor Onyx") {
        for copy in [false, true] {
            let mut g = action_game();
            let onyx = g.create_object_from_definition(&definition, A, Zone::Battlefield);
            let spell = card(&mut g, A, Zone::Hand, "Magecraft witness", "Mana cost: {0}\nType: Sorcery\nYou gain 1 life.");
            let mut dm = Choices::default();
            cast(&mut g, spell, &mut dm);
            assert_eq!(g.stack.len(), 2, "cast and magecraft remain separate stack entries");
            if copy {
                let original = g.stack.iter().find(|entry| !entry.is_ability).unwrap().object_id;
                apply(&mut g, onyx, Effect::copy_spell(ChooseSpec::SpecificObject(original)), &mut dm);
                assert_eq!(g.stack.len(), 4, "copy and its magecraft trigger are additional stack entries");
            }
            settle(&mut g, &mut dm);
            assert_eq!(g.player(A).unwrap().life, if copy { 36 } else { 33 },
                "gain two once per trigger, plus one per witness spell");
            assert_eq!(g.player(B).unwrap().life, if copy { 26 } else { 28 });
            assert_eq!(g.player(C).unwrap().life, if copy { 26 } else { 28 });
        }
    }
}

#[test]
fn zimone_pays_tap_and_another_creature_then_triggers_only_the_second_draw() {
    for definition in definitions("Zimone and Dina") {
        let mut g = action_game(); library(&mut g, A, 6);
        let source = g.create_object_from_definition(&definition, A, Zone::Battlefield);
        g.remove_summoning_sickness(source);
        for i in 0..8 { card(&mut g, A, Zone::Battlefield, &format!("Land {i}"), "Type: Land"); }
        let sacrifice = card(&mut g, A, Zone::Battlefield, "Sacrifice witness", "Type: Creature\nPower/Toughness: 1/1");
        let stable = g.object(sacrifice).unwrap().stable_id;
        let mut dm = Choices { target: Some(Target::Player(C)), ..Default::default() };
        activate(&mut g, source, 0, &mut dm);
        assert!(g.is_tapped(source));
        assert_eq!(g.object(g.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Graveyard);
        assert_eq!(g.object(source).unwrap().zone, Zone::Battlefield, "another creature is the cost");
        assert!(g.player(A).unwrap().hand.is_empty(), "paying does not execute the draw body");
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().hand.len(), 2);
        assert_eq!(g.player(A).unwrap().life, 32);
        assert_eq!(g.player(B).unwrap().life, 30);
        assert_eq!(g.player(C).unwrap().life, 28);
        assert_eq!(dm.target_calls, 1, "the second-draw trigger announces its own target once");
        apply(&mut g, source, Effect::draw(1), &mut dm);
        settle(&mut g, &mut dm);
        assert_eq!(g.player(A).unwrap().hand.len(), 3);
        assert_eq!(g.player(A).unwrap().life, 32);
        assert_eq!(dm.target_calls, 1, "the third draw does not repeat the second-draw trigger");
    }
}

//! Claude package 50 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::target::ChooseSpec;
use ironsmith::types::{CardType, Subtype};
use ironsmith::{PlayerId, Zone};

fn p50_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

/// CR 601.2b/601.2f: Explosive Singularity's optional additional cost taps
/// any number of untapped creatures you control, and each creature tapped
/// this way reduces the total cost by {1} before mana is paid — the same
/// cost-payment reduction as "for each creature sacrificed this way".
#[test]
fn explosive_singularity_tapped_creatures_reduce_the_actual_payment() {
    use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::decisions::context::{BooleanContext, DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm};
    use ironsmith::mana::ManaSymbol;
    struct Tapper { wanted: usize, mine: Vec<ironsmith::ObjectId> }
    impl DecisionMaker for Tapper {
        fn answers_player_choices(&self) -> bool { true }
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &BooleanContext) -> bool { self.wanted > 0 }
        fn decide_objects(&mut self, game: &ironsmith::GameState, ctx: &SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            let chosen: Vec<_> = ctx.candidates.iter().filter(|candidate| candidate.legal && self.mine.contains(&candidate.id))
                .take(self.wanted).map(|candidate| candidate.id).collect();
            if chosen.len() >= ctx.min { chosen } else { SelectFirstDecisionMaker.decide_objects(game, ctx) }
        }
        fn decide_options(&mut self, game: &ironsmith::GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
        fn decide_targets(&mut self, game: &ironsmith::GameState, ctx: &TargetsContext) -> Vec<ironsmith::game_state::Target> {
            SelectFirstDecisionMaker.decide_targets(game, ctx)
        }
    }
    let definition = p50_compile("Explosive Singularity",
        "Mana cost: {8}{R}{R}\nType: Sorcery\nAs an additional cost to cast this spell, you may tap any number of untapped creatures you control. This spell costs {1} less to cast for each creature tapped this way.\nExplosive Singularity deals 10 damage to any target.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("Tapped"), "the reduction reads the tap payment: {debug}");
    let alice = PlayerId::from_index(0);
    let creature = CardBuilder::new(CardId::new(), "Tapped creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 1)).build();
    for wanted in [0usize, 1, 3] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 2);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 20);
        let mine: Vec<_> = (0..4).map(|_| game.create_object_from_card(&creature, alice, Zone::Battlefield)).collect();
        // An already tapped creature can't pay the tap cost.
        let tapped = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.tap(tapped);
        let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let action = compute_legal_actions(&game, alice).unwrap().into_iter()
            .find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand))
            .expect("Explosive Singularity is castable");
        let before = game.player(alice).unwrap().mana_pool.total();
        let mut dm = Tapper { wanted, mine: mine.clone() };
        let mut state = PriorityLoopState::new(game.players.len());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..40 {
            if !game.stack.is_empty() { break; }
            progress = match progress {
                GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref ctx))
                    if ctx.description.starts_with("Choose optional costs") =>
                {
                    let chosen = if wanted == 0 { vec![] } else { vec![(0, 1)] };
                    apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                        &PriorityResponse::OptionalCosts(chosen), &mut dm).unwrap()
                }
                GameProgress::NeedsDecisionCtx(ref ctx) if !matches!(ctx, DecisionContext::Priority(_)) => {
                    apply_decision_context_with_dm(&mut game, &mut queue, &mut state, ctx, &mut dm).unwrap()
                }
                other => panic!("wanted={wanted}: the cast did not reach the stack: {other:?}"),
            };
        }
        assert!(!game.stack.is_empty(), "wanted={wanted}: the spell is on the stack");
        let paid = before - game.player(alice).unwrap().mana_pool.total();
        assert_eq!(paid as usize, 10 - wanted, "wanted={wanted}: each creature tapped this way pays {{1}}");
        assert_eq!(mine.iter().filter(|id| game.is_tapped(**id)).count(), wanted);
    }
}

/// False Orders: the removed blocker's sole-blocked attackers become
/// unblocked (CR 506.4 / 509.1h), and the caster may then have it block an
/// attacking creature of their choice that is attacking its controller.
#[test]
fn false_orders_moves_a_blocker_onto_a_chosen_attacker() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState, is_blocked};
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    struct Pick { attacker: Option<ironsmith::ObjectId>, prompts: usize }
    impl ironsmith::decision::DecisionMaker for Pick {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.attacker.is_some()
        }
        fn decide_objects(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            self.prompts += 1;
            self.attacker.filter(|id| ctx.candidates.iter().any(|candidate| candidate.id == *id)).into_iter().collect()
        }
    }
    let definition = p50_compile("False Orders",
        "Mana cost: {R}\nType: Instant\nCast this spell only during the declare blockers step.\nRemove target creature defending player controls from combat. Creatures it was blocking that had become blocked by only that creature this combat become unblocked. You may have it block an attacking creature of your choice.");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("UnblockSoleBlockedAttackersEffect") && debug.contains("BlockAttackerOfChoiceEffect"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature = |name: &str| CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    for choose in [true, false] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        let first = game.create_object_from_card(&creature("Blocked attacker"), alice, Zone::Battlefield);
        let second = game.create_object_from_card(&creature("Unblocked attacker"), alice, Zone::Battlefield);
        let blocker = game.create_object_from_card(&creature("Bob's blocker"), bob, Zone::Battlefield);
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo { creature: first, target: AttackTarget::Player(bob) });
        combat.attackers.push(AttackerInfo { creature: second, target: AttackTarget::Player(bob) });
        combat.blockers.insert(first, vec![blocker]);
        combat.blocked_attackers.insert(first);
        combat.block_declaration_complete = true;
        game.combat = Some(combat);
        let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::CreatureBlockedEvent::new(blocker, first), ironsmith::ProvNodeId::default());
        game.turn_store.turn_history.record_event(&event, None, None);
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let mut dm = Pick { attacker: choose.then_some(second), prompts: 0 };
        let mut ctx = EffectContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(blocker)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert!(dm.prompts >= 1, "the caster is offered an attacker");
        let combat = game.combat.as_ref().unwrap();
        assert!(!is_blocked(combat, first), "its only blocker left combat, so it is unblocked");
        assert!(!combat.blockers.get(&first).is_some_and(|blockers| blockers.contains(&blocker)));
        assert_eq!(is_blocked(combat, second), choose, "choose={choose}");
        assert_eq!(combat.blockers.get(&second).is_some_and(|blockers| blockers.contains(&blocker)), choose);
    }
}

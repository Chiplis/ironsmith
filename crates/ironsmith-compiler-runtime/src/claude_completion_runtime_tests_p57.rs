//! Claude package 57 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::ids::CardId;
use ironsmith::types::{CardType, Subtype};
use ironsmith::{PlayerId, Zone};

fn p57_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

/// CR 601.2b/601.2f, 614.1c: Chorus of the Conclave grants your creature
/// spells an optional "pay any amount of mana" additional cost (a repeatable
/// {1}); the creature enters with one additional +1/+1 counter per payment.
#[test]
fn chorus_of_the_conclave_any_amount_of_mana_becomes_entry_counters() {
    use ironsmith::decision::{GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::decisions::context::DecisionContext;
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::object::CounterType;
    let chorus = p57_compile("Chorus of the Conclave",
        "Mana cost: {4}{G}{G}{W}{W}\nType: Legendary Creature — Dryad\nPower/Toughness: 3/8\nForestwalk (This creature can't be blocked as long as defending player controls a Forest.)\nAs an additional cost to cast creature spells, you may pay any amount of mana. If you do, that creature enters with that many additional +1/+1 counters on it.");
    let debug = format!("{:?}", chorus.abilities);
    assert!(debug.contains("GrantSpellKeyword") && debug.contains("AnyAmountOfManaForEntryCounters"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bear = CardBuilder::new(CardId::new(), "Cast bear")
        .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Bear])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    for paid_times in [0u32, 3] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 10);
        game.create_object_from_definition(&chorus, alice, Zone::Battlefield);
        let hand = game.create_object_from_card(&bear, alice, Zone::Hand);
        let action = compute_legal_actions(&game, alice).unwrap().into_iter()
            .find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand))
            .expect("the creature spell is castable");
        let before = game.player(alice).unwrap().mana_pool.total();
        let mut dm = SelectFirstDecisionMaker;
        let mut state = PriorityLoopState::new(game.players.len());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        let mut offered = false;
        for _ in 0..40 {
            if !game.stack.is_empty() && state.pending_cast.is_none() { break; }
            progress = match progress {
                GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref ctx))
                    if ctx.description.starts_with("Choose optional costs") =>
                {
                    offered = true;
                    let chosen = if paid_times == 0 { vec![] } else { vec![(0, paid_times)] };
                    apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                        &PriorityResponse::OptionalCosts(chosen), &mut dm).unwrap()
                }
                GameProgress::NeedsDecisionCtx(ref ctx) if !matches!(ctx, DecisionContext::Priority(_)) => {
                    apply_decision_context_with_dm(&mut game, &mut queue, &mut state, ctx, &mut dm).unwrap()
                }
                other => panic!("paid={paid_times}: the cast did not reach the stack: {other:?}"),
            };
        }
        assert!(offered, "the granted cost is announced while casting");
        assert!(!game.stack.is_empty(), "paid={paid_times}: the spell is on the stack");
        let paid = before - game.player(alice).unwrap().mana_pool.total();
        assert_eq!(paid, 1 + paid_times, "each payment adds {{1}} to the total cost");
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let permanent = *game.battlefield.last().unwrap();
        assert_eq!(game.object(permanent).unwrap().name.to_string(), "Cast bear");
        assert_eq!(
            game.object(permanent).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0),
            paid_times,
            "one additional +1/+1 counter per payment"
        );
    }
}

/// CR 605.1a, 605.3a: Channel gives its caster a mana ability of their own
/// until end of turn ("pay 1 life: add {C}"). It is activatable at priority,
/// usable (repeatedly) by the payment planner, and gone after cleanup.
#[test]
fn channel_grants_a_repeatable_pay_life_mana_ability_until_end_of_turn() {
    use ironsmith::special_actions::{SpecialAction, can_perform_check, perform};
    let channel = p57_compile("Channel",
        "Mana cost: {G}{G}\nType: Sorcery\nUntil end of turn, any time you could activate a mana ability, you may pay 1 life. If you do, add {C}.");
    let program = channel.spell_effect.as_ref().expect("spell effect");
    assert!(program.all_effects().iter().any(|effect| effect
        .downcast_ref::<ironsmith::effects::GrantPayLifeManaAbilityUntilEndOfTurnEffect>()
        .is_some_and(|grant| grant.life_cost == 1
            && grant.mana == vec![ironsmith::mana::ManaSymbol::Colorless])),
        "{program:?}");
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&channel, alice, Zone::Stack);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    for effect in program.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(game.effect_store.granted_mana_abilities.len(), 1);
    let carrier = game.effect_store.granted_mana_abilities[0].carrier.expect("command-zone carrier");
    assert!(game.command_zone.contains(&carrier));
    let ability_index = game.object(carrier).unwrap().abilities_vec().iter()
        .position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .expect("carried mana ability");

    // At priority: pay 1 life, add {C}.
    let action = || SpecialAction::ActivateManaAbility { permanent_id: carrier, ability_index };
    assert!(can_perform_check(&action(), &game, alice).is_ok());
    assert!(can_perform_check(&action(), &game, PlayerId::from_index(1)).is_err(), "only its owner has it");
    perform(action(), &mut game, alice, &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 19);
    assert_eq!(game.player(alice).unwrap().mana_pool.total_wide(), 1);

    // While paying a cost: the planner activates it as often as needed.
    let cost = ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::mana::ManaSymbol::Generic(3)]]);
    let request = ironsmith::mana_payment::ManaPaymentRequest::new(
        alice, source, ironsmith::costs::PaymentReason::CastSpell, cost,
    );
    let plan = ironsmith::mana_payment::plan_first_mana_payment(&game, &request).unwrap();
    ironsmith::mana_payment::execute_mana_payment_plan(
        &mut game, &request, &plan, &mut ironsmith::decision::SelectFirstDecisionMaker,
    ).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 17, "two more activations pay the rest");
    assert_eq!(game.player(alice).unwrap().mana_pool.total_wide(), 0);

    // Cleanup ends it.
    game.turn.turn_number += 1;
    game.cleanup_granted_mana_abilities_end_of_turn();
    assert!(game.effect_store.granted_mana_abilities.is_empty());
    assert!(game.object(carrier).is_none() && !game.command_zone.contains(&carrier));
}

/// CR 706.1, 400.7: Centaur of Attention stores five d6 results on itself,
/// may reroll any subset of them, and gets +X/+X for the largest group of
/// equal stored results.
#[test]
fn centaur_of_attention_stores_rerolls_and_reads_die_results() {
    use ironsmith::decisions::context::SelectOptionsContext;
    use ironsmith::target::PlayerFilter;
    struct Pick(Vec<usize>);
    impl ironsmith::decision::DecisionMaker for Pick {
        fn decide_options(&mut self, _: &ironsmith::GameState, _: &SelectOptionsContext) -> Vec<usize> {
            self.0.clone()
        }
    }
    let centaur = p57_compile("Centaur of Attention",
        "Mana cost: {3}{G}{G}\nType: Creature — Centaur Performer\nPower/Toughness: 3/3\nWhen this creature enters, roll five six-sided dice and store those results on it.\nAt the beginning of combat on your turn, you may reroll any number of this creature's stored results.\nThis creature gets +X/+X, where X is the greatest number of stored results on it of the same value.");
    let debug = format!("{:?}", centaur.abilities);
    assert!(debug.contains("RollAndStoreDieResultsEffect") && debug.contains("count: 5") && debug.contains("sides: 6"), "{debug}");
    assert!(debug.contains("RerollStoredDieResultsEffect"), "{debug}");
    assert!(debug.contains("GreatestEqualStoredDieResultsOnSource"), "{debug}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = game.create_object_from_definition(&centaur, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(creature), Some(3), "no stored results yet");

    let roll = Effect::new(ironsmith::effects::RollAndStoreDieResultsEffect::new(PlayerFilter::You, 5, 6));
    let mut ctx = ironsmith::effects::EffectContext::new_default(creature, alice);
    ironsmith::effects::execute_effect(&mut game, &roll, &mut ctx).unwrap();
    drop(ctx);
    let (sides, stored) = game.stored_die_results(creature).expect("results are stored on it");
    assert_eq!(sides, 6);
    assert_eq!(stored.len(), 5);
    assert!(stored.iter().all(|result| (1..=6).contains(result)));

    game.set_stored_die_results(creature, 6, vec![3, 3, 3, 5, 1]);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(creature), Some(6), "three 3s give +3/+3");
    assert_eq!(game.calculated_toughness(creature), Some(6));

    // Reroll only the last two results; the three 3s stay.
    let reroll = Effect::new(ironsmith::effects::RerollStoredDieResultsEffect::new(PlayerFilter::You));
    let mut dm = Pick(vec![3, 4]);
    let mut ctx = ironsmith::effects::EffectContext::new(creature, alice, &mut dm);
    ironsmith::effects::execute_effect(&mut game, &reroll, &mut ctx).unwrap();
    drop(ctx);
    let (_, stored) = game.stored_die_results(creature).unwrap();
    assert_eq!(&stored[..3], &[3, 3, 3]);
    assert!(stored[3..].iter().all(|result| (1..=6).contains(result)));
    assert!(game.greatest_equal_stored_die_result_count(creature) >= 3);

    // Choosing none rerolls nothing.
    let before = game.stored_die_results(creature).unwrap().1.to_vec();
    let mut dm = Pick(vec![]);
    let mut ctx = ironsmith::effects::EffectContext::new(creature, alice, &mut dm);
    ironsmith::effects::execute_effect(&mut game, &reroll, &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.stored_die_results(creature).unwrap().1, before.as_slice());
}

/// CR 509.1 replaced: after Camouflage resolves, each defending player divides
/// creatures into one pile per attacker; the piles are assigned to the
/// attackers at random and each creature that can block its pile's attacker
/// does so.
#[test]
fn camouflage_turns_blocker_declaration_into_random_piles() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::decision::BlockerDeclaration;
    let camouflage = p57_compile("Camouflage",
        "Mana cost: {G}\nType: Instant\nCast this spell only during your declare attackers step.\nThis turn, instead of declaring blockers, each defending player chooses any number of creatures they control and divides them into a number of piles equal to the number of attacking creatures for whom that player is the defending player. Creatures those players control that can block additional creatures may likewise be put into additional piles. Assign each pile to a different one of those attacking creatures at random. Each creature in a pile that can block the creature that pile is assigned to does so. (Piles can be empty.)");
    let program = camouflage.spell_effect.as_ref().expect("spell effect");
    assert!(program.all_effects().iter().any(|effect| effect
        .downcast_ref::<ironsmith::effects::DeclareBlockersByRandomPilesThisTurnEffect>()
        .is_some()), "{program:?}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let ground = CardBuilder::new(CardId::new(), "Ground creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build();
    let flier = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Flying attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .flying()
        .build();
    let mut seen_ground_pair = false;
    let mut seen_single = false;
    for seed in 0..24u64 {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.set_random_seed(seed);
        game.turn.active_player = alice;
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        let walker = game.create_object_from_card(&ground, alice, Zone::Battlefield);
        let flying = game.create_object_from_definition(&flier, alice, Zone::Battlefield);
        let blockers: Vec<_> = (0..3).map(|_| game.create_object_from_card(&ground, bob, Zone::Battlefield)).collect();
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo { creature: walker, target: AttackTarget::Player(bob) });
        combat.attackers.push(AttackerInfo { creature: flying, target: AttackTarget::Player(bob) });
        game.combat = Some(combat.clone());

        assert!(!game.blockers_declared_by_random_piles_this_turn());
        let source = game.create_object_from_definition(&camouflage, alice, Zone::Stack);
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in program.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert!(game.blockers_declared_by_random_piles_this_turn());
        game.refresh_continuous_state().unwrap();

        let ironsmith::decisions::context::DecisionContext::Blockers(prompt) =
            ironsmith::game_loop::get_pile_division_blockers_decision(&game, &combat, bob)
        else {
            panic!("a pile division prompt");
        };
        assert_eq!(prompt.blocker_options.len(), 2, "one pile per attacker");
        assert!(prompt.blocker_options.iter().all(|pile| pile.valid_blockers.len() == 3 && pile.min_blockers == 0));

        // Pile one: two creatures; pile two: one creature.
        let piles = vec![
            BlockerDeclaration { blocker: blockers[0], blocking: prompt.blocker_options[0].attacker },
            BlockerDeclaration { blocker: blockers[1], blocking: prompt.blocker_options[0].attacker },
            BlockerDeclaration { blocker: blockers[2], blocking: prompt.blocker_options[1].attacker },
        ];
        let blocks = ironsmith::game_loop::assign_random_pile_blocks(&game, &combat, bob, &piles);
        assert!(blocks.iter().all(|block| block.blocking == walker), "nothing can block the flier: {blocks:?}");
        let mut blocking: Vec<_> = blocks.iter().map(|block| block.blocker).collect();
        blocking.sort();
        if blocking == vec![blockers[0], blockers[1]] {
            seen_ground_pair = true;
        } else {
            assert_eq!(blocking, vec![blockers[2]], "exactly one pile blocks the ground attacker");
            seen_single = true;
        }
    }
    assert!(seen_ground_pair && seen_single, "piles are assigned at random");
}

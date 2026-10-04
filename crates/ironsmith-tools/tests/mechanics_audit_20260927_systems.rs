//! Regressions for the September 27 audit: current identity and rule boundaries.
use ironsmith::cards::builders::CardDefinitionBuilder as B;
use ironsmith::effects::{
    ControlPlayerEffect, DoubleManaPoolEffect, EffectContext, EffectExecutor,
};
use ironsmith::events::cause::EventCause;
use ironsmith::events::{DamageEvent, DamageTarget};
use ironsmith::target::PlayerFilter;
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};

const A: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20)
}
fn permanent(game: &mut GameState, owner: PlayerId, kind: CardType) -> ObjectId {
    game.create_object_from_definition(
        &B::new(CardId::new(), "Fixture")
            .card_types(vec![kind])
            .build(),
        owner,
        Zone::Battlefield,
    )
}

#[test]
fn s1_monarch_uses_live_controller_or_departure_lki_not_a_new_incarnation() {
    for depart in [false, true] {
        let mut g = game();
        let attacker = permanent(&mut g, BOB, CardType::Creature);
        g.set_monarch(Some(A)).expect("checked designation/departure fixture");
        let event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                attacker,
                DamageTarget::Player(A),
                1,
                true,
                EventCause::from_combat_damage(attacker, BOB),
            ),
            Default::default(),
        );
        let mut queue = TriggerQueue::new();
        for entry in ironsmith::triggers::check_triggers(&g, &event) {
            queue.add(entry);
        }
        ironsmith::put_triggers_on_stack(&mut g, &mut queue).unwrap();
        assert_eq!(
            g.stack.last().unwrap().controller,
            A,
            "The monarch still controls the inherent trigger"
        );
        g.set_current_controller(attacker, C).expect("finite controller fixture must refresh successfully");
        g.refresh_continuous_state();
        if depart {
            let stable = g.object(attacker).unwrap().stable_id;
            g.move_object_by_effect(attacker, Zone::Graveyard).unwrap();
            let grave = g.find_object_by_stable_id(stable).unwrap();
            g.move_object_by_effect(grave, Zone::Battlefield).unwrap();
            let returned = g.find_object_by_stable_id(stable).unwrap();
            g.set_current_controller(returned, BOB).expect("finite controller fixture must refresh successfully");
        }
        ironsmith::resolve_stack_entry(&mut g).unwrap();
        assert_eq!(g.monarch, Some(C));
    }
}

#[test]
fn s2_whole_turn_control_survives_repeated_cleanup_and_expires_at_next_turn() {
    let mut g = game();
    let source = g.new_object_id();
    ControlPlayerEffect::during_next_turn(PlayerFilter::Specific(BOB))
        .execute(&mut g, &mut EffectContext::new_default(source, A))
        .unwrap();
    assert_eq!(g.controlling_player_for(BOB), BOB);
    g.next_turn();
    assert_eq!(g.turn.active_player, BOB);
    for _ in 0..2 {
        ironsmith::turn::execute_cleanup_step(&mut g);
        assert_eq!(g.controlling_player_for(BOB), A);
    }
    g.next_turn();
    assert_eq!(g.controlling_player_for(BOB), BOB);
}

#[test]
fn s3_storied_requires_present_source_and_three_present_qualifying_permanents() {
    for phase_source in [true, false] {
        let mut g = game();
        let source = g.create_object_from_definition(
            &B::new(CardId::new(), "Storied")
                .card_types(vec![CardType::Creature])
                .storied()
                .build(),
            A,
            Zone::Battlefield,
        );
        if phase_source {
            g.phase_out(source);
        }
        let first = permanent(&mut g, A, CardType::Artifact);
        if !phase_source {
            g.phase_out(first);
        }
        for _ in 0..2 {
            permanent(&mut g, A, CardType::Artifact);
        }
        g.refresh_continuous_state();
        assert!(!g.has_enduring_story(A));
        g.phase_in(if phase_source { source } else { first });
        g.refresh_continuous_state();
        assert!(g.has_enduring_story(A));
    }
}

#[test]
fn s7_doubled_mana_has_no_inherited_restrictions_or_bonuses() {
    use ironsmith::ability::{ManaUsageRestriction, RestrictedManaUnit};
    use ironsmith::mana::ManaSymbol;
    let mut g = game();
    let source = permanent(&mut g, A, CardType::Artifact);
    let old_source = g.new_object_id();
    let restricted = RestrictedManaUnit {
        source_controller: None,
        symbol: ManaSymbol::Red,
        source: old_source,
        source_chosen_creature_type: None,
        restrictions: vec![ManaUsageRestriction::CastSpell {
            card_types: vec![CardType::Creature],
            subtype_requirement: None,
            restrict_to_matching_spell: true,
            grant_uncounterable: true,
            enters_with_counters: vec![],
            granted_abilities: vec![],
        }],
    };
    g.player_mut(A)
        .unwrap()
        .add_restricted_mana(restricted.clone());
    let outcome = DoubleManaPoolEffect::you()
        .execute(&mut g, &mut EffectContext::new_default(source, A))
        .unwrap();
    assert_eq!(g.player(A).unwrap().mana_pool.red, 2);
    assert_eq!(g.player(A).unwrap().restricted_mana, vec![restricted]);
    let event = outcome.events[0]
        .downcast::<ironsmith::events::ManaAddedEvent>()
        .unwrap();
    assert_eq!(event.source, source);
}

fn open_attraction(g: &mut GameState) -> ObjectId {
    let def = B::new(CardId::new(), "Visit fixture")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![ironsmith::types::Subtype::Attraction])
        .attraction_lights(vec![6])
        .with_spell_effect(vec![ironsmith::Effect::gain_life(1)])
        .build();
    g.enable_attractions(vec![(
        A,
        ironsmith::game_state::AttractionDeckFormat::Limited,
        vec![def.clone(), def.clone(), def],
    )])
    .unwrap();
    let source = g.new_object_id();
    ironsmith::effects::OpenAttractionEffect::new()
        .execute(g, &mut EffectContext::new_default(source, A))
        .unwrap();
    g.face_up_attractions()[0]
}

#[test]
fn s4_phased_attractions_neither_cause_rolls_nor_receive_visits() {
    let mut g = game();
    let first = open_attraction(&mut g);
    g.phase_out(first);
    g.force_next_die_roll(6);
    let mut q = TriggerQueue::new();
    assert_eq!(
        ironsmith::game_loop::roll_to_visit_attractions(&mut g, &mut q).unwrap(),
        None
    );
    assert!(q.entries.is_empty());
    let source = g.new_object_id();
    ironsmith::effects::OpenAttractionEffect::new()
        .execute(&mut g, &mut EffectContext::new_default(source, A))
        .unwrap();
    assert_eq!(
        ironsmith::game_loop::roll_to_visit_attractions(&mut g, &mut q).unwrap(),
        Some(6)
    );
    assert_eq!(q.entries.len(), 1);
    assert_ne!(q.entries[0].source, first);
}

#[test]
fn s5_visit_is_an_ordinary_ability_that_can_be_removed_and_restored() {
    use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
    let mut g = game();
    let attraction = open_attraction(&mut g);
    let source = g.new_object_id();
    let effect = g
        .effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            source,
            A,
            EffectTarget::Specific(attraction),
            Modification::RemoveAllAbilities,
        ));
    g.refresh_continuous_state();
    let mut q = TriggerQueue::new();
    g.force_next_die_roll(6);
    assert_eq!(
        ironsmith::game_loop::roll_to_visit_attractions(&mut g, &mut q).unwrap(),
        Some(6)
    );
    assert!(q.entries.is_empty());
    g.effect_store.continuous_effects.remove_effect(effect);
    g.refresh_continuous_state();
    g.force_next_die_roll(6);
    ironsmith::game_loop::roll_to_visit_attractions(&mut g, &mut q).unwrap();
    assert_eq!(q.entries.len(), 1);
    ironsmith::put_triggers_on_stack(&mut g, &mut q).unwrap();
    ironsmith::resolve_stack_entry(&mut g).unwrap();
    assert_eq!(g.player(A).unwrap().life, 21);
}

#[test]
fn s6_attraction_roll_resumes_life_modifier_without_duplicate_roll_or_payment() {
    use ironsmith::decisions::context::DecisionContext;
    use ironsmith::turn_runner::{TurnAction, TurnRunner, TurnState};
    let mut g = game();
    open_attraction(&mut g);
    let modifier = B::new(CardId::new(), "Adjust result")
        .card_types(vec![CardType::Enchantment])
        .with_ability(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::die_roll_result_adjustment(
                PlayerFilter::You,
                1,
                1,
                true,
                "Adjust result",
            ),
        ))
        .build();
    let modifier = g.create_object_from_definition(&modifier, A, Zone::Battlefield);
    g.force_next_die_roll(5);
    let mut q = TriggerQueue::new();
    let mut runner = TurnRunner::from_state_for_sync(TurnState::FirstMain);
    let TurnAction::Decision(DecisionContext::Boolean(prompt)) =
        runner.advance(&mut g, &mut q).unwrap()
    else {
        panic!("expected a modifier choice");
    };
    assert!(prompt.description.contains("Die result 5"));
    assert_eq!(g.player(A).unwrap().life, 20);
    assert!(g.turn_store.turn_history.die_rolls_this_turn.is_empty());
    assert!(matches!(
        runner.advance(&mut g, &mut q).unwrap(),
        TurnAction::Decision(DecisionContext::Boolean(_))
    ));
    runner.respond_boolean(true);
    assert!(matches!(
        runner.advance(&mut g, &mut q).unwrap(),
        TurnAction::Decision(DecisionContext::SelectOptions(_))
    ));
    assert_eq!(g.player(A).unwrap().life, 20);
    runner.respond_options(vec![0]);
    assert!(matches!(
        runner.advance(&mut g, &mut q).unwrap(),
        TurnAction::RunPriority
    ));
    assert_eq!(g.player(A).unwrap().life, 19);
    assert_eq!(g.turn_store.turn_history.die_rolls_this_turn[&A], vec![6]);
    assert!(
        g.turn_store
            .turn_history
            .die_roll_result_adjusted_this_turn(modifier)
    );
    assert_eq!(q.entries.len(), 1);
}

#[test]
fn s6_attraction_paid_reroll_presents_mana_payment_and_commits_once() {
    use ironsmith::decisions::context::DecisionContext;
    use ironsmith::turn_runner::{TurnAction, TurnRunner, TurnState};
    let mut g = game();
    open_attraction(&mut g);
    let modifier = B::new(CardId::new(), "Paid reroll")
        .card_types(vec![CardType::Artifact])
        .with_ability(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::die_roll_reroll(
                PlayerFilter::You,
                ironsmith::mana::ManaCost::from_pips(vec![vec![
                    ironsmith::mana::ManaSymbol::Generic(1),
                ]]),
                true,
                "Pay one to reroll",
            ),
        ))
        .build();
    g.create_object_from_definition(&modifier, A, Zone::Battlefield);
    g.player_mut(A).unwrap().mana_pool.colorless = 1;
    g.force_next_die_roll(2);
    g.force_next_die_roll(6);
    let mut q = TriggerQueue::new();
    let mut runner = TurnRunner::from_state_for_sync(TurnState::FirstMain);
    let TurnAction::Decision(DecisionContext::Boolean(prompt)) =
        runner.advance(&mut g, &mut q).unwrap()
    else {
        panic!("expected a reroll choice");
    };
    assert!(prompt.description.contains("rolled 2"));
    runner.respond_boolean(true);
    let TurnAction::Decision(DecisionContext::ManaPayment(ctx)) =
        runner.advance(&mut g, &mut q).unwrap()
    else {
        panic!("must ask for payment")
    };
    assert_eq!(g.player(A).unwrap().mana_pool.colorless, 1);
    runner.respond_mana_payment(ironsmith::mana_payment::ManaPaymentResponse::Confirm {
        plan_id: ctx.plan.id,
        request_hash: ctx.plan.request_hash,
    });
    assert!(matches!(
        runner.advance(&mut g, &mut q).unwrap(),
        TurnAction::RunPriority
    ));
    assert_eq!(g.player(A).unwrap().mana_pool.colorless, 0);
    assert_eq!(g.turn_store.turn_history.die_rolls_this_turn[&A], vec![6]);
    assert_eq!(q.entries.len(), 1);
}

#[test]
fn a12_compleated_and_doubling_can_be_ordered_either_way() {
    use ironsmith::ability::Ability;
    use ironsmith::object::CounterType;
    use ironsmith::static_abilities::StaticAbility;
    use ironsmith::target::ObjectFilter;
    struct Choose {
        season_first: bool,
        choices: usize,
    }
    impl ironsmith::decision::DecisionMaker for Choose {
        fn decide_options(
            &mut self,
            _: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.choices += 1;
            let preferred = if self.season_first {
                "Doubling"
            } else {
                "Compleated"
            };
            vec![
                ctx.options
                    .iter()
                    .find(|o| o.description.contains(preferred))
                    .expect("both replacements must be offered")
                    .index,
            ]
        }
    }
    for season_first in [false, true] {
        let mut g = game();
        let season = B::new(CardId::new(), "Doubling Season fixture")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(
                StaticAbility::double_effect_counters_replacement(
                    ObjectFilter::default().you_control(),
                    None,
                    "Doubling Season".into(),
                ),
            ))
            .build();
        g.create_object_from_definition(&season, A, Zone::Battlefield);
        let walker = B::new(CardId::new(), "Compleated fixture")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(5)
            .with_ability(Ability::static_ability(StaticAbility::keyword_marker(
                "Compleated",
            )))
            .build();
        let id = g.create_object_from_definition(&walker, A, Zone::Stack);
        g.object_mut(id)
            .unwrap()
            .optional_costs_paid
            .mark_label_paid("CompleatedLifePaid");
        let mut dm = Choose {
            season_first,
            choices: 0,
        };
        let permanent = g
            .move_object_with_etb_processing_with_dm(id, Zone::Battlefield, &mut dm).map(require_plain_entry_for_test).expect("entry execution must succeed in this scenario")
            .unwrap()
            .new_id;
        assert_eq!(
            g.counter_count(permanent, CounterType::Loyalty),
            if season_first { 8 } else { 6 }
        );
        assert_eq!(dm.choices, 1);
    }
}

#[test]
fn xt1_tribute_chooser_cannot_select_a_teammate() {
    use ironsmith::ability::Ability;
    use ironsmith::static_abilities::StaticAbility;
    struct Choose {
        payers: Vec<PlayerId>,
        options: Vec<String>,
    }
    impl ironsmith::decision::DecisionMaker for Choose {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            ctx: &ironsmith::decisions::context::BooleanContext,
        ) -> bool {
            self.payers.push(ctx.player);
            true
        }
        fn decide_options(
            &mut self,
            _: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.options
                .extend(ctx.options.iter().map(|o| o.description.clone()));
            vec![0]
        }
    }
    let mut g = GameState::new(
        vec!["Alice".into(), "Ally".into(), "Bob".into(), "Buddy".into()],
        20,
    );
    g.set_teams(vec![vec![A, PlayerId(1)], vec![PlayerId(2), PlayerId(3)]])
        .unwrap();
    let tribute = B::new(CardId::new(), "Tribute")
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::static_ability(StaticAbility::tribute(2)))
        .build();
    let id = g.create_object_from_definition(&tribute, A, Zone::Hand);
    let mut dm = Choose {
        payers: vec![],
        options: vec![],
    };
    let permanent = g
        .move_object_with_etb_processing_with_dm(id, Zone::Battlefield, &mut dm).map(require_plain_entry_for_test).expect("entry execution must succeed in this scenario")
        .unwrap()
        .new_id;
    assert_eq!(dm.payers, vec![PlayerId(2)]);
    assert!(dm.options.iter().all(|option| !option.contains("Ally")));
    assert_eq!(
        g.counter_count(permanent, ironsmith::object::CounterType::PlusOnePlusOne),
        2
    );
}

#[test]
fn a12_compleated_adjusts_combined_loyalty_once_and_respects_prospective_ability_loss() {
    use ironsmith::ability::Ability;
    use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
    use ironsmith::object::CounterType;
    use ironsmith::static_abilities::StaticAbility;
    for lose_ability in [false, true] {
        let mut g = game();
        let walker = B::new(CardId::new(), "Compleated extra counters")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(5)
            .with_ability(Ability::static_ability(StaticAbility::keyword_marker(
                "Compleated",
            )))
            .build();
        let id = g.create_object_from_definition(&walker, A, Zone::Stack);
        g.object_mut(id)
            .unwrap()
            .optional_costs_paid
            .mark_label_paid("CompleatedLifePaid");
        if lose_ability {
            let source = permanent(&mut g, A, CardType::Enchantment);
            g.effect_store
                .continuous_effects
                .add_effect(ContinuousEffect::new(
                    source,
                    A,
                    EffectTarget::Filter(ironsmith::target::ObjectFilter::permanent()),
                    Modification::RemoveAllAbilities,
                ));
            g.refresh_continuous_state();
        }
        let permanent = g
            .move_object_with_etb_processing_with_initial_counters_with_dm(
                id,
                Zone::Battlefield,
                vec![(CounterType::Loyalty, 3)],
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            ).map(require_plain_entry_for_test).expect("entry execution must succeed in this scenario")
            .unwrap()
            .new_id;
        assert_eq!(
            g.counter_count(permanent, CounterType::Loyalty),
            if lose_ability { 8 } else { 6 }
        );
    }
}

// These fixtures expect a plain completed entry. Reject a continuation or
// retained added instructions rather than silently projecting them away.
fn require_plain_entry_for_test(receipt: ironsmith::game_state::EntryCommitResult)
    -> Option<ironsmith::game_state::EntersResult> {
    assert!(!receipt.pending, "fixture requires completed entry");
    assert!(receipt.programs.is_empty(), "fixture must finish retained entry replacement programs");
    receipt.original.into_result()
}

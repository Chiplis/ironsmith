//! Regression scenarios for the September 27 trigger/continuous-mechanics audit.
use ironsmith::ability::Ability;
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder as B;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::cost::TotalCost;
use ironsmith::decision::{AttackerDeclaration, DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith::effect::{EffectId, EffectPredicate, Until};
use ironsmith::effects::{
    ApplyContinuousEffect, EffectContext as ExecutionContext, EffectExecutor,
    EmitKeywordActionEffect, SacrificeEffect,
};
use ironsmith::events::{BeginningOfUpkeepEvent, KeywordActionKind};
use ironsmith::game_loop::{
    drain_pending_trigger_events, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::object::CounterType;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::triggers::{Trigger, TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{CardId, CardType, Effect, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn creature(name: &str, power: i32) -> B {
    B::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, 3))
}
fn modify(g: &mut GameState, id: ObjectId, modification: Modification) {
    ApplyContinuousEffect::new(EffectTarget::Specific(id), modification, Until::Forever)
        .execute(g, &mut ExecutionContext::new_default(id, A))
        .unwrap();
    g.refresh_continuous_state();
}
fn upkeep(g: &mut GameState) {
    g.mark_upkeep_began(A);
    let event = TriggerEvent::new(BeginningOfUpkeepEvent::new(A), Default::default());
    let mut q = TriggerQueue::new();
    for trigger in check_triggers(g, &event) {
        q.add(trigger);
    }
    put_triggers_on_stack_with_dm(g, &mut q, &mut SelectFirstDecisionMaker).unwrap();
}
#[derive(Default)]
struct Decisions {
    accept: bool,
    prompts: usize,
}
impl DecisionMaker for Decisions {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.prompts += 1;
        self.accept
    }
    fn decide_objects(&mut self, _: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        c.candidates
            .iter()
            .take(c.min.max(1))
            .map(|x| x.id)
            .collect()
    }
}
#[test]
fn tc1_lifelink_uses_live_abilities_controller_and_departure_lki() {
    for (initial, gain, depart, stolen, expected) in [
        (false, true, false, false, (21, 19)),
        (true, false, false, false, (20, 19)),
        (true, true, false, true, (20, 20)),
        (false, true, true, false, (21, 19)),
        (true, false, true, false, (20, 19)),
    ] {
        let mut g = game();
        let mut b = creature("Damage source", 2).with_ability(Ability::triggered(
            Trigger::beginning_of_upkeep(PlayerFilter::You),
            vec![Effect::deal_damage(1, ChooseSpec::SpecificPlayer(BOB))],
        ));
        if initial {
            b = b.lifelink();
        }
        let id = g.create_object_from_definition(&b.build(), A, Zone::Battlefield);
        upkeep(&mut g);
        modify(
            &mut g,
            id,
            if gain {
                Modification::AddAbility(StaticAbility::lifelink())
            } else {
                Modification::RemoveAllAbilities
            },
        );
        if stolen {
            modify(&mut g, id, Modification::ChangeController(BOB));
        }
        if depart {
            g.move_object_by_effect(id, Zone::Graveyard).unwrap();
        }
        resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(
            (g.player(A).unwrap().life, g.player(BOB).unwrap().life),
            expected,
            "initial={initial}, gain={gain}, depart={depart}, stolen={stolen}"
        );
    }
}
#[test]
fn tc1_live_infect_and_wither_changes_control_counter_damage() {
    for ability in [StaticAbility::infect(), StaticAbility::wither()] {
        for grant in [true, false] {
            let mut g = game();
            let target = g.create_object_from_definition(
                &creature("Target", 1).build(),
                BOB,
                Zone::Battlefield,
            );
            let mut b = creature("Source", 2).with_ability(Ability::triggered(
                Trigger::beginning_of_upkeep(PlayerFilter::You),
                vec![Effect::deal_damage(1, ChooseSpec::SpecificObject(target))],
            ));
            if !grant {
                b = b.with_ability(Ability::static_ability(ability.clone()));
            }
            let source = g.create_object_from_definition(&b.build(), A, Zone::Battlefield);
            upkeep(&mut g);
            modify(
                &mut g,
                source,
                if grant {
                    Modification::AddAbility(ability.clone())
                } else {
                    Modification::RemoveAllAbilities
                },
            );
            resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(
                g.counter_count(target, CounterType::MinusOneMinusOne),
                u32::from(grant)
            );
            assert_eq!(g.damage_on(target), u32::from(!grant));
        }
    }
}
#[test]
fn tc2_phased_countdowns_and_direct_counter_changes_do_nothing() {
    for fading in [false, true] {
        let mut g = game();
        let b = creature("Countdown", 2);
        let def = if fading { b.fading(2) } else { b.vanishing(2) }.build();
        let id = g.create_object_from_definition(&def, A, Zone::Battlefield);
        let counter = if fading {
            CounterType::Fade
        } else {
            CounterType::Time
        };
        g.add_counters(id, counter, 2);
        g.take_pending_trigger_events();
        upkeep(&mut g);
        g.phase_out(id);
        resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(g.counter_count(id, counter), 2);
        assert!(
            g.add_counters_with_source(id, counter, 1, None, None)
                .is_none()
        );
        assert_eq!(g.counter_count(id, counter), 2);
        g.phase_in(id);
        assert!(g.remove_counters(id, counter, 1, None, None).is_some());
        assert_eq!(g.counter_count(id, counter), 1);
    }
}
#[test]
fn tc3_cumulative_upkeep_intervening_if_stops_payment_after_departure_or_phasing() {
    for phase in [false, true] {
        let mut g = game();
        let def = creature("Upkeep", 2).cumulative_upkeep(vec![], 1).build();
        let id = g.create_object_from_definition(&def, A, Zone::Battlefield);
        g.add_counters(id, CounterType::Age, 2);
        upkeep(&mut g);
        if phase {
            g.phase_out(id);
        } else {
            g.move_object_by_effect(id, Zone::Graveyard).unwrap();
        }
        let mut dm = Decisions {
            accept: true,
            ..Default::default()
        };
        resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert_eq!(dm.prompts, 0);
        assert_eq!(g.player(A).unwrap().life, 20);
        if phase {
            assert_eq!(g.counter_count(id, CounterType::Age), 2);
        }
    }
}
#[test]
fn tc4_echo_tracks_extra_upkeeps_and_control_acquired_between_them() {
    let mut g = game();
    let def = creature("Echo", 2).echo(TotalCost::free()).build();
    let hand = g.create_object_from_definition(&def, A, Zone::Hand);
    let id = g.move_object_by_effect(hand, Zone::Battlefield).unwrap();
    upkeep(&mut g);
    assert_eq!(g.stack.len(), 1);
    resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
    upkeep(&mut g);
    assert!(
        g.stack.is_empty(),
        "no repeated echo without a control acquisition"
    );
    g.set_current_controller(id, BOB).expect("finite controller fixture must refresh successfully");
    g.set_current_controller(id, A).expect("finite controller fixture must refresh successfully");
    upkeep(&mut g);
    assert_eq!(g.stack.len(), 1, "reacquisition between upkeeps counts");
    resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
    upkeep(&mut g);
    assert!(g.stack.is_empty());
}
#[test]
fn tc5_training_completion_requires_counters_and_tc6_accepts_team_attacker() {
    for prohibited in [false, true] {
        for teammate in [false, true] {
            let mut g = GameState::new(
                vec!["Alice".into(), "Ally".into(), "Bob".into(), "Buddy".into()],
                20,
            );
            g.set_teams(vec![vec![A, BOB], vec![PlayerId(2), PlayerId(3)]])
                .unwrap();
            g.enable_shared_team_turns().unwrap();
            let mut b = creature("Training", 1)
                .training()
                .with_ability(Ability::triggered(
                    Trigger::keyword_action_from_source(
                        KeywordActionKind::Train,
                        PlayerFilter::You,
                    ),
                    vec![Effect::gain_life(1)],
                ));
            if prohibited {
                b = b.with_ability(Ability::static_ability(
                    StaticAbility::cant_have_counters_placed(),
                ));
            }
            let id = g.create_object_from_definition(&b.build(), A, Zone::Battlefield);
            let friend = g.create_object_from_definition(
                &creature("Larger", 3).build(),
                if teammate { BOB } else { A },
                Zone::Battlefield,
            );
            for id in [id, friend] {
                g.remove_summoning_sickness(id);
            }
            g.refresh_continuous_state();
            g.take_pending_trigger_events();
            g.turn.phase = ironsmith::game_state::Phase::Combat;
            g.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
            let mut combat = ironsmith::combat_state::CombatState::default();
            let mut q = TriggerQueue::new();
            let declarations = [id, friend].map(|creature| AttackerDeclaration {
                creature,
                target: ironsmith::combat_state::AttackTarget::Player(PlayerId(2)),
            });
            ironsmith::game_loop::apply_attacker_declarations(
                &mut g,
                &mut combat,
                &mut q,
                &declarations,
            )
            .unwrap();
            g.combat = Some(combat);
            assert_eq!(q.entries.len(), 1);
            put_triggers_on_stack_with_dm(&mut g, &mut q, &mut SelectFirstDecisionMaker).unwrap();
            resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
            drain_pending_trigger_events(&mut g, &mut q);
            assert_eq!(
                g.counter_count(id, CounterType::PlusOnePlusOne),
                u32::from(!prohibited)
            );
            assert_eq!(q.entries.len(), usize::from(!prohibited));
        }
    }
}
#[test]
fn tc7_suspect_grants_follow_timestamps_and_designation_survives_ability_loss() {
    for suspect_first in [false, true] {
        let mut g = game();
        let id =
            g.create_object_from_definition(&creature("Suspect", 2).build(), A, Zone::Battlefield);
        if suspect_first {
            g.set_suspected(id);
        }
        modify(&mut g, id, Modification::RemoveAllAbilities);
        if !suspect_first {
            g.set_suspected(id);
        }
        g.refresh_continuous_state();
        assert!(g.is_suspected(id));
        assert_eq!(
            g.current_has_static_ability_id(id, StaticAbilityId::Menace),
            !suspect_first
        );
        assert_eq!(
            g.current_has_static_ability_id(id, StaticAbilityId::CantBlock),
            !suspect_first
        );
        g.clear_suspected(id);
        g.refresh_continuous_state();
        assert!(!g.current_has_static_ability_id(id, StaticAbilityId::Menace));
        g.set_suspected(id);
        g.refresh_continuous_state();
        assert!(g.current_has_static_ability_id(id, StaticAbilityId::Menace));
    }
}
#[test]
fn tc8_zero_cumulative_upkeep_still_offers_payment_or_sacrifice() {
    for accept in [false, true] {
        let mut g = game();
        let def = creature("Zero age", 2)
            .with_ability(Ability::static_ability(
                StaticAbility::cant_have_counters_placed(),
            ))
            .with_ability(Ability::triggered(
                Trigger::beginning_of_upkeep(PlayerFilter::You),
                vec![
                    Effect::put_counters_on_source(CounterType::Age, 1),
                    Effect::cumulative_upkeep(
                        vec![Effect::pay_life(1)],
                        PlayerFilter::You,
                        vec![Effect::sacrifice_source()],
                    ),
                ],
            ))
            .build();
        let id = g.create_object_from_definition(&def, A, Zone::Battlefield);
        upkeep(&mut g);
        let mut dm = Decisions {
            accept,
            ..Default::default()
        };
        resolve_stack_entry_with(&mut g, &mut dm).unwrap();
        assert_eq!(dm.prompts, 1);
        assert_eq!(g.object(id).is_some(), accept);
        assert_eq!(g.player(A).unwrap().life, 20);
    }
}
#[test]
fn xt3_exploit_looks_back_only_when_source_is_sacrificed_by_that_exploit() {
    for depart_before in [false, true] {
        let mut g = game();
        let sacrifice = Effect::new(
            SacrificeEffect::you(ObjectFilter::creature(), 1)
                .with_event_object_tag(ironsmith::tag::EXPLOITED_TAG)
                .with_event_source_tag(ironsmith::tag::EXPLOITER_TAG),
        );
        let emit = Effect::new(
            EmitKeywordActionEffect::new(KeywordActionKind::Exploit, 1)
                .with_affected_object_memory_tag(EffectId(0), ironsmith::tag::EXPLOITED_TAG),
        );
        let def = creature("Exploiter", 2)
            .with_ability(Ability::triggered(
                Trigger::this_enters_battlefield(),
                vec![
                    Effect::with_id(0, Effect::may(vec![sacrifice])),
                    Effect::if_then(EffectId(0), EffectPredicate::Happened, vec![emit]),
                ],
            ))
            .with_ability(Ability::triggered(
                Trigger::keyword_action_from_source(KeywordActionKind::Exploit, PlayerFilter::You),
                vec![Effect::gain_life(1)],
            ))
            .build();
        let hand = g.create_object_from_definition(&def, A, Zone::Hand);
        let receipt = g.move_object_with_etb_processing(hand, Zone::Battlefield)
            .expect("entry execution must succeed in this scenario");
        assert!(!receipt.pending);
        assert!(receipt.programs.is_empty(), "fixture must finish added entry programs");
        let id = receipt.original.into_result().expect("the fixture must enter").new_id;
        g.create_object_from_definition(&creature("Victim", 1).build(), A, Zone::Battlefield);
        let mut q = TriggerQueue::new();
        drain_pending_trigger_events(&mut g, &mut q);
        put_triggers_on_stack_with_dm(&mut g, &mut q, &mut SelectFirstDecisionMaker).unwrap();
        if depart_before {
            g.move_object_by_effect(id, Zone::Graveyard).unwrap();
            g.take_pending_trigger_events();
        }
        resolve_stack_entry_with(
            &mut g,
            &mut Decisions {
                accept: true,
                ..Default::default()
            },
        )
        .unwrap();
        drain_pending_trigger_events(&mut g, &mut q);
        assert_eq!(q.entries.len(), usize::from(!depart_before));
    }
}
fn resolve_keyword_creature(g: &mut GameState, blitz: bool) -> ObjectId {
    let b = creature("Paid keyword", 2);
    let def = if blitz {
        b.blitz(ironsmith::mana::ManaCost::new())
    } else {
        b.dash(ironsmith::mana::ManaCost::new())
    }
    .build();
    let source = g.create_object_from_definition(&def, A, Zone::Stack);
    let mut entry = ironsmith::game_state::StackEntry::new(source, A);
    entry.casting_method = ironsmith::alternative_cast::CastingMethod::Alternative(0);
    entry
        .optional_costs_paid
        .mark_label_paid(if blitz { "Blitz" } else { "Dash" });
    g.object_mut(source).unwrap().optional_costs_paid = entry.optional_costs_paid.clone();
    g.push_to_stack(entry);
    resolve_stack_entry_with(g, &mut SelectFirstDecisionMaker).unwrap();
    g.battlefield[0]
}
#[test]
fn xt2_a10_dash_blitz_haste_persists_and_is_not_copied_or_immune_to_ability_loss() {
    for blitz in [false, true] {
        let mut g = game();
        let id = resolve_keyword_creature(&mut g, blitz);
        assert!(g.current_has_static_ability_id(id, StaticAbilityId::Haste));
        modify(&mut g, id, Modification::ChangeController(BOB));
        assert!(g.current_has_static_ability_id(id, StaticAbilityId::Haste));
        ironsmith::turn::execute_cleanup_step(&mut g);
        assert!(g.current_has_static_ability_id(id, StaticAbilityId::Haste));
        let copy = ironsmith::effects::execute_effect(
            &mut g,
            &Effect::create_token_copy(ChooseSpec::SpecificObject(id)),
            &mut ExecutionContext::new_default(id, A),
        )
        .unwrap();
        let copies = copy.objects().expect("a token copy was created");
        assert_eq!(copies.len(), 1);
        for &copy_id in copies {
            assert!(!g.current_has_static_ability_id(copy_id, StaticAbilityId::Haste));
        }
        modify(&mut g, id, Modification::RemoveAllAbilities);
        assert!(!g.current_has_static_ability_id(id, StaticAbilityId::Haste));
    }
}
#[test]
fn a11_blitz_death_draw_belongs_to_current_controller_and_can_be_removed() {
    for lose_abilities in [false, true] {
        let mut g = game();
        let id = resolve_keyword_creature(&mut g, true);
        for player in [A, BOB] {
            g.create_object_from_definition(
                &creature("Library card", 1).build(),
                player,
                Zone::Library,
            );
        }
        modify(&mut g, id, Modification::ChangeController(BOB));
        if lose_abilities {
            modify(&mut g, id, Modification::RemoveAllAbilities);
        }
        g.take_pending_trigger_events();
        g.move_object_by_effect(id, Zone::Graveyard).unwrap();
        let mut q = TriggerQueue::new();
        drain_pending_trigger_events(&mut g, &mut q);
        assert_eq!(q.entries.len(), usize::from(!lose_abilities));
        if !lose_abilities {
            assert_eq!(q.entries[0].controller, BOB);
            put_triggers_on_stack_with_dm(&mut g, &mut q, &mut SelectFirstDecisionMaker).unwrap();
            resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
        }
        assert_eq!(g.player(A).unwrap().hand.len(), 0);
        assert_eq!(
            g.player(BOB).unwrap().hand.len(),
            usize::from(!lose_abilities)
        );
    }
}

#[test]
fn tc1_deathtouch_changes_before_damage_and_phased_source_uses_lki() {
    for grant in [false, true] {
        let mut g = game();
        let target =
            g.create_object_from_definition(&creature("Target", 2).build(), BOB, Zone::Battlefield);
        let mut b = creature("Damage source", 2).with_ability(Ability::triggered(
            Trigger::beginning_of_upkeep(PlayerFilter::You),
            vec![Effect::deal_damage(1, ChooseSpec::SpecificObject(target))],
        ));
        if !grant {
            b = b.with_ability(Ability::static_ability(StaticAbility::deathtouch()));
        }
        let source = g.create_object_from_definition(&b.build(), A, Zone::Battlefield);
        upkeep(&mut g);
        modify(
            &mut g,
            source,
            if grant {
                Modification::AddAbility(StaticAbility::deathtouch())
            } else {
                Modification::RemoveAllAbilities
            },
        );
        resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(g.has_deathtouch_damage_since_sba(target), grant);
    }
    let mut g = game();
    let def = creature("Phased damage source", 2)
        .with_ability(Ability::triggered(
            Trigger::beginning_of_upkeep(PlayerFilter::You),
            vec![Effect::deal_damage(1, ChooseSpec::SpecificPlayer(BOB))],
        ))
        .build();
    let source = g.create_object_from_definition(&def, A, Zone::Battlefield);
    upkeep(&mut g);
    modify(
        &mut g,
        source,
        Modification::AddAbility(StaticAbility::lifelink()),
    );
    g.phase_out(source);
    resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(g.player(A).unwrap().life, 21);
}
#[test]
fn granted_blitz_keeps_its_battlefield_abilities_and_clears_them_on_zone_change() {
    let mut g = game();
    // A granted cost has no intrinsic battlefield keyword components in the
    // card definition. This is the same runtime representation as a grant.
    let def = creature("Granted Blitz", 2)
        .alternative_cast(
            ironsmith::alternative_cast::AlternativeCastingMethod::Blitz {
                total_cost: TotalCost::free(),
            },
        )
        .build();
    let source = g.create_object_from_definition(&def, A, Zone::Stack);
    let mut entry = ironsmith::game_state::StackEntry::new(source, A);
    entry.casting_method = ironsmith::alternative_cast::CastingMethod::Alternative(0);
    entry.optional_costs_paid.mark_label_paid("Blitz");
    g.object_mut(source).unwrap().optional_costs_paid = entry.optional_costs_paid.clone();
    g.push_to_stack(entry);
    resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
    let permanent = g.battlefield[0];
    assert!(g.current_has_static_ability_id(permanent, StaticAbilityId::Haste));
    modify(&mut g, permanent, Modification::ChangeController(BOB));
    assert!(g.current_has_static_ability_id(permanent, StaticAbilityId::Haste));
    g.take_pending_trigger_events();
    let grave = g.move_object_by_effect(permanent, Zone::Graveyard).unwrap();
    let mut q = TriggerQueue::new();
    drain_pending_trigger_events(&mut g, &mut q);
    assert_eq!(q.entries.len(), 1);
    assert_eq!(q.entries[0].controller, BOB);
    assert!(
        g.object(grave)
            .unwrap()
            .temporary_static_ability_grants
            .is_empty()
    );
}

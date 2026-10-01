//! Regressions for September 27 casting findings C01–C03 and C05–C08.
//! Specification: Comprehensive Rules effective 2026-09-25.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::cards::builders::CardDefinitionBuilder as B;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::cost::TotalCost;
use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::effect::{Effect, Until};
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::{Phase, StackEntry};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::{TriggerEvent, TriggerQueue};
use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
fn game() -> GameState {
    let mut g = GameState::new(vec!["A".into(), "B".into()], 20);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g
}
fn exhaust(mana: bool) -> Ability {
    let mut a = Ability::activated(TotalCost::mana(ManaCost::new()), vec![Effect::gain_life(1)]);
    if let AbilityKind::Activated(x) = &mut a.kind {
        x.additional_restrictions
            .push("Activate each exhaust ability only once.".into());
        if mana {
            x.mana_output = Some(vec![ManaSymbol::Green]);
            x.effects = Default::default();
        }
    }
    a
}
#[test]
fn conspire() {
    let mut g = game();
    let d = B::new(CardId::new(), "Double conspire")
        .card_types(vec![CardType::Sorcery])
        .conspire()
        .conspire()
        .build();
    let id = g.create_object_from_definition(&d, A, Zone::Stack);
    let mut e = StackEntry::new(id, A);
    e.optional_costs_paid = ironsmith::cost::OptionalCostsPaid::from_costs(&d.optional_costs);
    e.optional_costs_paid.pay(0);
    g.object_mut(id).unwrap().optional_costs_paid = e.optional_costs_paid.clone();
    g.push_to_stack(e);
    let evt = TriggerEvent::new_with_provenance(
        ironsmith::events::SpellCastEvent::new(id, A, Zone::Hand),
        Default::default(),
    );
    println!(
        "C01: conspire refs={:?}; only first paid -> {} triggers, expected 1",
        d.optional_costs
            .iter()
            .map(|c| c.cost_ref())
            .collect::<Vec<_>>(),
        ironsmith::triggers::check_triggers(&g, &evt).len()
    );
    assert_eq!(ironsmith::triggers::check_triggers(&g, &evt).len(), 1);
    let c = ironsmith_registry::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Double compiled conspire",
    )
    .card_types(vec![CardType::Sorcery])
    .parse_text("Conspire\nConspire")
    .unwrap();
    assert_ne!(
        c.optional_costs[0].cost_ref(),
        c.optional_costs[1].cost_ref()
    );
}
#[test]
fn mana_exhaust() {
    let mut g = game();
    let d = B::new(CardId::new(), "Exhaust mana fixture")
        .card_types(vec![CardType::Artifact])
        .with_ability(exhaust(true))
        .build();
    let id = g.create_object_from_definition(&d, A, Zone::Battlefield);
    let mut q = TriggerQueue::new();
    let mut s = PriorityLoopState::new(2);
    for i in 0..2 {
        let action = compute_legal_actions(&g, A).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::ActivateManaAbility{source,..} if *source==id));
        println!(
            "C02: Exhaust mana activation {} offered={} (second expected false)",
            i + 1,
            action.is_some()
        );
        assert_eq!(
            action.is_some(),
            i == 0,
            "only the first Exhaust activation is legal"
        );
        if let Some(a) = action {
            let r = ironsmith::game_loop::apply_priority_response_with_dm(
                &mut g,
                &mut q,
                &mut s,
                &PriorityResponse::PriorityAction(a),
                &mut SelectFirstDecisionMaker,
            );
            finish(&mut g, &mut q, &mut s, r.unwrap());
            println!(
                "C02: activation {} committed; mana={}, tracked={}",
                i + 1,
                g.player(A).unwrap().mana_pool.total(),
                g.exhaust_ability_activated(id, 0)
            );
        }
    }
}
#[test]
fn granted_exhaust() {
    let mut g = game();
    let d = B::new(CardId::new(), "Granted exhaust fixture")
        .card_types(vec![CardType::Artifact])
        .build();
    let id = g.create_object_from_definition(&d, A, Zone::Battlefield);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(id),
        Modification::AddAbilityGeneric(exhaust(false)),
        Until::EndOfTurn,
    )
    .execute(&mut g, &mut EffectContext::new_default(id, A))
    .unwrap();
    g.refresh_continuous_state();
    let mut q = TriggerQueue::new();
    let mut s = PriorityLoopState::new(2);
    for i in 0..2 {
        let action = compute_legal_actions(&g, A).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::ActivateAbility{source,..} if *source==id));
        println!(
            "C03: granted exhaust activation {} offered={} (second expected false)",
            i + 1,
            action.is_some()
        );
        assert_eq!(
            action.is_some(),
            i == 0,
            "only the first Exhaust activation is legal"
        );
        if let Some(a) = action {
            let r = ironsmith::game_loop::apply_priority_response_with_dm(
                &mut g,
                &mut q,
                &mut s,
                &PriorityResponse::PriorityAction(a),
                &mut SelectFirstDecisionMaker,
            );
            finish(&mut g, &mut q, &mut s, r.unwrap());
            println!(
                "C03: activation {} committed; stack={}, tracked={}",
                i + 1,
                g.stack.len(),
                g.exhaust_ability_activated(id, 0)
            );
        }
    }
}
fn finish(g: &mut GameState, q: &mut TriggerQueue, s: &mut PriorityLoopState, mut p: GameProgress) {
    for _ in 0..20 {
        if let GameProgress::NeedsDecisionCtx(c) = p {
            p = ironsmith::game_loop::apply_decision_context_with_dm(
                g,
                q,
                s,
                &c,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
        } else {
            break;
        }
        if s.pending_activation.is_none() && s.pending_mana_ability.is_none() {
            break;
        }
    }
}
#[test]
fn ninjutsu_counter() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::game_state::Step;
    let mut g = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(Step::DeclareBlockers);
    let d = B::new(CardId::new(), "Ninja")
        .card_types(vec![CardType::Creature])
        .ninjutsu(ManaCost::new())
        .build();
    let ninja = g.create_object_from_definition(&d, A, Zone::Hand);
    let d = B::new(CardId::new(), "Attacker")
        .card_types(vec![CardType::Creature])
        .build();
    let one = g.create_object_from_definition(&d, A, Zone::Battlefield);
    let two = g.create_object_from_definition(&d, A, Zone::Battlefield);
    g.combat = Some(CombatState {
        attackers: vec![
            AttackerInfo {
                creature: one,
                target: AttackTarget::Player(PlayerId(1)),
            },
            AttackerInfo {
                creature: two,
                target: AttackTarget::Player(PlayerId(2)),
            },
        ],
        ..Default::default()
    });
    for _ in 0..2 {
        ironsmith::effects::NinjutsuCostEffect::new()
            .execute(&mut g, &mut EffectContext::new_default(ninja, A))
            .unwrap();
        g.push_to_stack(StackEntry::ability(
            ninja,
            A,
            vec![Effect::new(ironsmith::effects::NinjutsuEffect::new())],
        ));
    }
    assert_eq!(
        g.stack[0].ninjutsu_attack_target,
        Some(AttackTarget::Player(PlayerId(1)))
    );
    assert_eq!(
        g.stack[1].ninjutsu_attack_target,
        Some(AttackTarget::Player(PlayerId(2)))
    );
    let top = g.stack.last().unwrap().ability_id.unwrap();
    let result =
        ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(top))
            .execute(&mut g, &mut EffectContext::new_default(ninja, A));
    println!(
        "C05: counter newest Ninjutsu activation result={:?}, surviving stack entries={}",
        result,
        g.stack.len()
    );
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(
        g.combat.as_ref().unwrap().attackers[0].target,
        AttackTarget::Player(PlayerId(1))
    );
}
#[test]
fn grouped_copy_triggers() {
    for label in ["Replicate", "Granted Conspire"] {
        let mut g = game();
        let c = ironsmith::cost::OptionalCost::custom(label, TotalCost::mana(ManaCost::new()));
        let d = B::new(CardId::new(), label)
            .card_types(vec![CardType::Sorcery])
            .optional_cost(c.clone())
            .optional_cost(c)
            .build();
        let id = g.create_object_from_definition(&d, A, Zone::Stack);
        let mut e = StackEntry::new(id, A);
        e.optional_costs_paid = ironsmith::cost::OptionalCostsPaid::from_costs(&d.optional_costs);
        e.optional_costs_paid.pay(0);
        e.optional_costs_paid.pay(1);
        g.object_mut(id).unwrap().optional_costs_paid = e.optional_costs_paid.clone();
        g.push_to_stack(e);
        let evt = TriggerEvent::new_with_provenance(
            ironsmith::events::SpellCastEvent::new(id, A, Zone::Hand),
            Default::default(),
        );
        let ts = ironsmith::triggers::check_triggers(&g, &evt);
        assert_eq!(
            ts.len(),
            2,
            "independent paid instances must create independent triggers"
        );
        assert_ne!(ts[0].trigger_identity, ts[1].trigger_identity);
        let mut queue = TriggerQueue::new();
        for trigger in ts.iter().cloned() {
            queue.add(trigger);
        }
        ironsmith::game_loop::put_triggers_on_stack(&mut g, &mut queue).unwrap();
        assert_eq!(g.stack.len(), 3);
        let countered = g.stack.last().unwrap().ability_id.unwrap();
        ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(
            countered,
        ))
        .execute(&mut g, &mut EffectContext::new_default(id, A))
        .unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert_eq!(
            g.stack.len(),
            2,
            "countering one instance preserves the other instance's one copy"
        );
        assert!(g.stack.iter().all(|entry| !entry.is_ability));
        println!(
            "C06: {label} two independently paid instances -> {} triggers (expected 2), effects={:?}",
            ts.len(),
            ts.iter().map(|t| &t.ability.effects).collect::<Vec<_>>()
        );
    }
}
#[test]
fn affinity() {
    let mut g = game();
    let d = B::new(CardId::new(), "Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    g.create_object_from_definition(&d, A, Zone::Battlefield);
    g.create_object_from_definition(&d, A, Zone::Battlefield);
    let cost = ManaCost::new().add_generic(5);
    let d = B::new(CardId::new(), "Two affinity spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(cost.clone())
        .with_ability(
            Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::affinity_for_artifacts(),
            )
            .in_zones(vec![Zone::Stack]),
        )
        .with_ability(
            Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::affinity_for_artifacts(),
            )
            .in_zones(vec![Zone::Stack]),
        )
        .build();
    let id = g.create_object_from_definition(&d, A, Zone::Stack);
    let actual =
        ironsmith::decision::calculate_effective_mana_cost(&g, A, g.object(id).unwrap(), &cost);
    assert_eq!(actual.mana_value(), 1);
}
#[test]
fn ninjutsu_unearth() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::game_state::Step;
    for sneak in [false, true] {
        let mut g = game();
        g.turn.phase = Phase::Combat;
        g.turn.step = Some(Step::DeclareBlockers);
        let d = B::new(CardId::new(), "Ninja")
            .card_types(vec![CardType::Creature])
            .ninjutsu(ManaCost::new())
            .build();
        let ninja = g.create_object_from_definition(&d, A, Zone::Hand);
        let d = B::new(CardId::new(), "Unearthed attacker")
            .card_types(vec![CardType::Creature])
            .build();
        let card = g.create_object_from_definition(&d, A, Zone::Graveyard);
        let out = ironsmith::effects::UnearthEffect::new()
            .execute(&mut g, &mut EffectContext::new_default(card, A))
            .unwrap();
        let attacker = out.first_output_object().unwrap();
        let stable = g.object(attacker).unwrap().stable_id;
        g.combat = Some(CombatState {
            attackers: vec![AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(PlayerId(1)),
            }],
            ..Default::default()
        });
        if sneak {
            ironsmith::effects::SneakCostEffect::new()
                .execute(&mut g, &mut EffectContext::new_default(ninja, A))
                .unwrap();
        } else {
            ironsmith::effects::NinjutsuCostEffect::new()
                .execute(&mut g, &mut EffectContext::new_default(ninja, A))
                .unwrap();
        }
        let id = g.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            g.object(id).unwrap().zone,
            Zone::Exile,
            "Unearth replaces the return cost"
        );
        println!(
            "C08: {} returns unearthed attacker to {:?} (expected Exile)",
            if sneak { "Sneak" } else { "Ninjutsu" },
            g.object(id).unwrap().zone
        );
    }
}

#[test]
fn conspire_each_payment_mask_and_compiled_linkage() {
    for compiled in [false, true] {
        for mask in 0..4 {
            let mut g = game();
            let b = B::new(CardId::new(), "Conspire instances").card_types(vec![CardType::Sorcery]);
            let d = if compiled {
                ironsmith_registry::cards::builders::CardDefinitionBuilder::new(
                    CardId::new(),
                    "Compiled conspire",
                )
                .card_types(vec![CardType::Sorcery])
                .parse_text("Conspire\nConspire")
                .unwrap()
            } else {
                b.conspire().conspire().build()
            };
            let spell = g.create_object_from_definition(&d, A, Zone::Stack);
            let mut entry = StackEntry::new(spell, A);
            entry.optional_costs_paid =
                ironsmith::cost::OptionalCostsPaid::from_costs(&d.optional_costs);
            for i in 0..2 {
                if mask & (1 << i) != 0 {
                    entry.optional_costs_paid.pay(i);
                }
            }
            g.object_mut(spell).unwrap().optional_costs_paid = entry.optional_costs_paid.clone();
            g.push_to_stack(entry);
            let event = TriggerEvent::new_with_provenance(
                ironsmith::events::SpellCastEvent::new(spell, A, Zone::Hand),
                Default::default(),
            );
            assert_eq!(
                ironsmith::triggers::check_triggers(&g, &event).len(),
                (mask as u32).count_ones() as usize,
                "only paid instances trigger, compiled={compiled}, mask={mask}"
            );
        }
    }
}

#[test]
fn refueler_permission_is_consumed_when_activation_begins_and_rolls_back() {
    use ironsmith::special_actions::{SpecialAction, can_perform_check};
    use ironsmith::static_abilities::StaticAbility;
    let mut g = game();
    let grant = B::new(CardId::new(), "Refueler permission")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(
            StaticAbility::exhaust_abilities_as_though_unactivated_this_turn(),
        ))
        .build();
    g.create_object_from_definition(&grant, A, Zone::Battlefield);
    let mana = B::new(CardId::new(), "Previously exhausted mana source")
        .card_types(vec![CardType::Artifact])
        .with_ability(exhaust(true))
        .build();
    let mana_id = g.create_object_from_definition(&mana, A, Zone::Battlefield);
    g.record_ability_activation(mana_id, 0);
    g.turn_store.turn_history = Default::default(); // The previous turn's use remains an object-lifetime fact.
    let mut activation = exhaust(false);
    if let AbilityKind::Activated(a) = &mut activation.kind {
        a.mana_cost = TotalCost::mana(ManaCost::new().add_generic(1));
    }
    let d = B::new(CardId::new(), "New exhaust activation")
        .card_types(vec![CardType::Artifact])
        .with_ability(activation)
        .build();
    let source = g.create_object_from_definition(&d, A, Zone::Battlefield);
    g.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    g.refresh_continuous_state();
    let old_mana_action = SpecialAction::ActivateManaAbility {
        permanent_id: mana_id,
        ability_index: 0,
    };
    assert!(
        can_perform_check(&old_mana_action, &g, A).is_ok(),
        "Refueler permits the earlier exhausted ability before any new activation"
    );
    let mut q = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let progress = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut g,
        &mut q,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
            source,
            ability_index: 0,
        }),
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    assert!(matches!(progress, GameProgress::NeedsDecisionCtx(_)));
    assert!(state.pending_activation.is_some());
    assert_eq!(g.exhaust_ability_activation_count_this_turn(A), 1);
    assert!(
        can_perform_check(&old_mana_action, &g, A).is_err(),
        "CR702.177b blocks using the exhausted mana ability to pay for the new activation"
    );
    assert!(state.rollback_action(&mut g));
    assert_eq!(g.exhaust_ability_activation_count_this_turn(A), 0);
    assert!(!g.exhaust_ability_activated(source, 0));
    assert!(can_perform_check(&old_mana_action, &g, A).is_ok());
}

#[test]
fn affinity_printed_and_multiple_grants_accumulate() {
    let mut g = game();
    let artifact = B::new(CardId::new(), "Artifact resource")
        .card_types(vec![CardType::Artifact])
        .build();
    g.create_object_from_definition(&artifact, A, Zone::Battlefield);
    let grant = ironsmith_registry::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Affinity grant",
    )
    .card_types(vec![CardType::Enchantment])
    .parse_text("Spells you cast have affinity for artifacts.")
    .unwrap();
    g.create_object_from_definition(&grant, A, Zone::Battlefield);
    g.create_object_from_definition(&grant, A, Zone::Battlefield);
    let cost = ManaCost::new().add_generic(5);
    let d = B::new(CardId::new(), "Affinity spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(cost.clone())
        .with_ability(
            Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::affinity_for_artifacts(),
            )
            .in_zones(vec![Zone::Stack]),
        )
        .build();
    for zone in [Zone::Hand, Zone::Stack] {
        let id = g.create_object_from_definition(&d, A, zone);
        g.refresh_continuous_state();
        assert_eq!(
            ironsmith::decision::calculate_effective_mana_cost(&g, A, g.object(id).unwrap(), &cost)
                .mana_value(),
            2,
            "one printed plus two granted Affinity instances each reduce by one"
        );
    }
}

#[test]
fn copied_ninjutsu_keeps_the_defender_selected_for_its_activation() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::game_state::Step;
    let mut g = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    g.turn.phase = Phase::Combat;
    g.turn.step = Some(Step::DeclareBlockers);
    let ninja = g.create_object_from_definition(
        &B::new(CardId::new(), "Ninja copy fixture")
            .card_types(vec![CardType::Creature])
            .ninjutsu(ManaCost::new())
            .build(),
        A,
        Zone::Hand,
    );
    let attacker = g.create_object_from_definition(
        &B::new(CardId::new(), "Attacker")
            .card_types(vec![CardType::Creature])
            .build(),
        A,
        Zone::Battlefield,
    );
    g.combat = Some(CombatState {
        attackers: vec![AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(PlayerId(2)),
        }],
        ..Default::default()
    });
    ironsmith::effects::NinjutsuCostEffect::new()
        .execute(&mut g, &mut EffectContext::new_default(ninja, A))
        .unwrap();
    g.push_to_stack(StackEntry::ability(
        ninja,
        A,
        vec![Effect::new(ironsmith::effects::NinjutsuEffect::new())],
    ));
    let original = g.stack[0].ability_id.unwrap();
    ironsmith::effects::CopySpellEffect::single(ironsmith::target::ChooseSpec::SpecificObject(
        original,
    ))
    .execute(&mut g, &mut EffectContext::new_default(ninja, A))
    .unwrap();
    assert_eq!(g.stack.len(), 2);
    assert_eq!(
        g.stack[1].ninjutsu_attack_target,
        Some(AttackTarget::Player(PlayerId(2)))
    );
    ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(original))
        .execute(&mut g, &mut EffectContext::new_default(ninja, A))
        .unwrap();
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(
        g.combat.as_ref().unwrap().attackers[0].target,
        AttackTarget::Player(PlayerId(2))
    );
}

#[test]
fn independent_granted_exhaust_instances_have_independent_limits() {
    let mut g = game();
    let source = g.create_object_from_definition(
        &B::new(CardId::new(), "Two grants")
            .card_types(vec![CardType::Artifact])
            .build(),
        A,
        Zone::Battlefield,
    );
    for _ in 0..2 {
        ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::AddAbilityGeneric(exhaust(false)),
            Until::EndOfTurn,
        )
        .execute(&mut g, &mut EffectContext::new_default(source, A))
        .unwrap();
    }
    g.refresh_continuous_state();
    g.record_ability_activation(source, 0);
    assert!(g.exhaust_ability_activated(source, 0));
    assert!(!g.exhaust_ability_activated(source, 1));
    let actions = compute_legal_actions(&g, A).expect("fixture has complete replacement state");
    assert!(!actions.iter().any(
        |a| matches!(a,LegalAction::ActivateAbility{source:id, ability_index:0} if *id==source)
    ));
    assert!(actions.iter().any(
        |a| matches!(a,LegalAction::ActivateAbility{source:id, ability_index:1} if *id==source)
    ));
    let first_grant = g.effect_store.continuous_effects.effects()[0].id;
    g.effect_store.continuous_effects.remove_effect(first_grant);
    g.refresh_continuous_state();
    assert!(
        !g.exhaust_ability_activated(source, 0),
        "the unused second grant moved into slot zero"
    );
    assert!(compute_legal_actions(&g, A).expect("fixture has complete replacement state").iter().any(
        |a| matches!(a,LegalAction::ActivateAbility{source:id, ability_index:0} if *id==source)
    ));
}

struct ReplacementChooser {
    replacement: &'static str,
    destination: &'static str,
    ordering_prompts: usize,
    destination_prompts: usize,
}
impl ironsmith::decision::DecisionMaker for ReplacementChooser {
    fn decide_options(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        let ordering = ctx
            .options
            .iter()
            .any(|option| option.description.contains("Madness"));
        let wanted = if ordering {
            self.ordering_prompts += 1;
            self.replacement
        } else {
            self.destination_prompts += 1;
            self.destination
        };
        vec![
            ctx.options
                .iter()
                .find(|option| option.description.contains(wanted))
                .unwrap_or_else(|| panic!("missing replacement choice {wanted}: {:?}", ctx.options))
                .index,
        ]
    }
}

#[test]
fn madness_and_graveyard_exile_are_independently_selectable_replacements() {
    for (linked, madness) in [(false, false), (false, true), (true, false), (true, true)] {
        let mut g = game();
        let card = g.create_object_from_definition(
            &B::new(CardId::new(), "Madness card")
                .card_types(vec![CardType::Instant])
                .madness(ManaCost::new())
                .build(),
            A,
            Zone::Hand,
        );
        let source = g.create_object_from_definition(
            &B::new(CardId::new(), "Graveyard replacement")
                .card_types(vec![CardType::Enchantment])
                .build(),
            A,
            Zone::Battlefield,
        );
        if linked {
            let replacement = ironsmith::replacement::ReplacementEffect::with_matcher(
                source,
                A,
                ironsmith::events::zones::matchers::WouldGoToGraveyardMatcher::new(
                    ironsmith::target::ObjectFilter::default(),
                ),
                ironsmith::replacement::ReplacementAction::ExileWithSourceLink,
            );
            ironsmith::effects::ApplyReplacementEffect::until_end_of_turn(replacement)
                .execute(&mut g, &mut EffectContext::new_default(source, A))
                .unwrap();
        } else {
            ironsmith::effects::ExileInsteadOfGraveyardEffect::you()
                .execute(&mut g, &mut EffectContext::new_default(source, A))
                .unwrap();
        }
        let mut dm = ReplacementChooser {
            replacement: if madness {
                "Madness"
            } else {
                "Graveyard replacement"
            },
            destination: "",
            ordering_prompts: 0,
            destination_prompts: 0,
        };
        let out = ironsmith::events::processing::execute_discard(
            &mut g,
            card,
            A,
            ironsmith::events::cause::EventCause::from_effect(source, A),
            false,
            Default::default(),
            &mut dm,
        ).expect("root discard should execute").expect("root discard should finish without a pending choice");
        assert_eq!(dm.ordering_prompts, 1);
        assert_eq!(out.final_zone, Zone::Exile);
        let exiled = out.new_id.unwrap();
        assert_eq!(g.is_madness_exiled(exiled), madness);
        assert_eq!(
            g.effect_store.pending_trigger_entries.len(),
            usize::from(madness)
        );
        if linked {
            assert_eq!(g.get_exiled_with_source_links(source).is_empty(), madness);
        }
    }
}

#[test]
fn madness_and_library_destination_recheck_after_each_choice() {
    for (first, destination, expected_zone, madness, destination_prompts) in [
        ("Madness", "", Zone::Exile, true, 0),
        (
            "Library replacement",
            "Top of library",
            Zone::Library,
            false,
            1,
        ),
        ("Library replacement", "Graveyard", Zone::Exile, true, 1),
    ] {
        let mut g = game();
        let card = g.create_object_from_definition(
            &B::new(CardId::new(), "Madness card")
                .card_types(vec![CardType::Instant])
                .madness(ManaCost::new())
                .build(),
            A,
            Zone::Hand,
        );
        let source = g.create_object_from_definition(&B::new(CardId::new(), "Library replacement")
            .card_types(vec![CardType::Artifact]).with_ability(Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::effect_discard_to_library_replacement())).build(), A, Zone::Battlefield);
        let mut dm = ReplacementChooser {
            replacement: first,
            destination,
            ordering_prompts: 0,
            destination_prompts: 0,
        };
        let out = ironsmith::events::processing::execute_discard(
            &mut g,
            card,
            A,
            ironsmith::events::cause::EventCause::from_effect(source, A),
            false,
            Default::default(),
            &mut dm,
        ).expect("root discard should execute").expect("root discard should finish without a pending choice");
        assert_eq!(dm.ordering_prompts, 1);
        assert_eq!(dm.destination_prompts, destination_prompts);
        assert_eq!(out.final_zone, expected_zone);
        assert_eq!(g.is_madness_exiled(out.new_id.unwrap()), madness);
        assert_eq!(
            g.effect_store.pending_trigger_entries.len(),
            usize::from(madness)
        );
    }
}

#[test]
fn discard_waits_for_the_replacement_destination_without_moving_the_card() {
    #[derive(Default)]
    struct PauseAtDestination {
        waiting: bool,
    }
    impl ironsmith::decision::DecisionMaker for PauseAtDestination {
        fn awaiting_choice(&self) -> bool {
            self.waiting
        }
        fn decide_options(
            &mut self,
            _: &GameState,
            ctx: &ironsmith::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if let Some(option) = ctx
                .options
                .iter()
                .find(|option| option.description == "Library replacement")
            {
                return vec![option.index];
            }
            self.waiting = true;
            vec![]
        }
    }
    let mut g = game();
    let source = g.create_object_from_definition(
        &B::new(CardId::new(), "Library replacement")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::effect_discard_to_library_replacement(),
            ))
            .build(),
        A,
        Zone::Battlefield,
    );
    let card = g.create_object_from_definition(
        &B::new(CardId::new(), "Madness card")
            .card_types(vec![CardType::Instant])
            .madness(ManaCost::new())
            .build(),
        A,
        Zone::Hand,
    );
    let mut dm = PauseAtDestination::default();
    let out = ironsmith::events::processing::execute_discard(
        &mut g,
        card,
        A,
        ironsmith::events::cause::EventCause::from_effect(source, A),
        false,
        Default::default(),
        &mut dm,
    ).expect("root discard should execute").expect("root discard should finish without a pending choice");
    assert!(dm.waiting);
    assert!(out.new_id.is_none());
    assert_eq!(g.object(card).unwrap().zone, Zone::Hand);
    assert!(g.effect_store.pending_trigger_entries.is_empty());
}

#[test]
fn repeated_replicate_payment_creates_one_trigger_with_several_copies() {
    let mut g = game();
    let definition = B::new(CardId::new(), "Repeated replicate")
        .card_types(vec![CardType::Sorcery])
        .optional_cost(ironsmith::cost::OptionalCost::replicate(TotalCost::mana(
            ManaCost::new(),
        )))
        .build();
    let spell = g.create_object_from_definition(&definition, A, Zone::Stack);
    let mut entry = StackEntry::new(spell, A);
    entry.optional_costs_paid =
        ironsmith::cost::OptionalCostsPaid::from_costs(&definition.optional_costs);
    entry.optional_costs_paid.pay_times(0, 3);
    g.object_mut(spell).unwrap().optional_costs_paid = entry.optional_costs_paid.clone();
    g.push_to_stack(entry);
    let event = TriggerEvent::new_with_provenance(
        ironsmith::events::SpellCastEvent::new(spell, A, Zone::Hand),
        Default::default(),
    );
    let triggers = ironsmith::triggers::check_triggers(&g, &event);
    assert_eq!(triggers.len(), 1);
    let mut queue = TriggerQueue::new();
    for trigger in triggers {
        queue.add(trigger);
    }
    ironsmith::game_loop::put_triggers_on_stack(&mut g, &mut queue).unwrap();
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(g.stack.len(), 4);
    assert!(g.stack.iter().all(|entry| !entry.is_ability));
}

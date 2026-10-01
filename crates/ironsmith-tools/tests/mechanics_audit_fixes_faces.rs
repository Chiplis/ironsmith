//! September 2026 CR audit fixes: F13, F25–29 and F32.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::card::{LinkedFaceLayout, PowerToughness};
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder as B};
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::cost::TotalCost;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effect::Until;
use ironsmith::effects::{
    ApplyContinuousEffect, ConvertEffect, EffectContext as ExecutionContext, EffectExecutor,
    ManifestTopCardOfLibraryEffect, ReconfigureEffect, ResolvedTarget, TransformEffect,
    UnearthEffect,
};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::{Phase, Target};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::AttachmentTarget;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::special_actions::{SpecialAction, TurnFaceUpMethod};
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::types::Subtype;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};

const A: PlayerId = PlayerId(0);
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.turn_number = 3;
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g.turn.phase = Phase::FirstMain;
    g
}
fn creature(name: &str) -> CardDefinition {
    B::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build()
}
fn pair(g: &mut GameState, daybound: bool) -> CardDefinition {
    let f = CardId::new();
    let b = CardId::new();
    let mut front = B::new(f, "Audit front")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .other_face(b)
        .linked_face_layout(LinkedFaceLayout::TransformLike);
    let mut back = B::new(b, "Audit back")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .other_face(f)
        .linked_face_layout(LinkedFaceLayout::TransformLike);
    if daybound {
        front = front.daybound();
        back = back.nightbound();
    }
    let front = front.build();
    let back = back.build();
    g.register_linked_face_definition(&front);
    g.register_linked_face_definition(&back);
    front
}
fn manifest(g: &mut GameState, def: &CardDefinition) -> ObjectId {
    g.create_object_from_definition(def, A, Zone::Library);
    let src = g.new_object_id();
    ManifestTopCardOfLibraryEffect::new(PlayerFilter::You)
        .execute(g, &mut ExecutionContext::new_default(src, A))
        .unwrap()
        .objects()
        .unwrap()[0]
}
fn modify(g: &mut GameState, id: ObjectId, modification: Modification, until: Until) {
    ApplyContinuousEffect::new(EffectTarget::Specific(id), modification, until)
        .execute(g, &mut ExecutionContext::new_default(id, A))
        .unwrap();
}
fn turn_up(g: &mut GameState, id: ObjectId, method: TurnFaceUpMethod) -> bool {
    ironsmith::special_actions::perform(
        SpecialAction::TurnFaceUp {
            permanent_id: id,
            method,
        },
        g,
        A,
        &mut SelectFirstDecisionMaker,
    )
    .is_ok()
}
fn morph_def(ability: StaticAbility) -> CardDefinition {
    B::new(CardId::new(), "Audit morph")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(0)]))
        .power_toughness(PowerToughness::fixed(3, 3))
        .with_ability(Ability::static_ability(ability))
        .build()
}

#[test]
fn f13_face_down_dfc_cannot_transform_or_convert_through_any_entry_point() {
    let mut g = game();
    let front = pair(&mut g, false);
    let id = manifest(&mut g, &front);
    let snapshot = g.object(id).unwrap().name.clone();
    let trans = TransformEffect::new(ChooseSpec::SpecificObject(id))
        .execute(&mut g, &mut ExecutionContext::new_default(id, A))
        .unwrap();
    let conv = ConvertEffect::new(ChooseSpec::SpecificObject(id))
        .execute(&mut g, &mut ExecutionContext::new_default(id, A))
        .unwrap();
    assert!(trans.events.is_empty() && conv.events.is_empty());
    assert!(!g.transform_permanent(id).expect("transform discovery must succeed in this scenario"));
    assert_eq!(g.transform_count(id), 0);
    assert_eq!(g.object(id).unwrap().name, snapshot);
    assert!(g.is_face_down(id));
    assert!(g.set_face_up(id).expect("fixture has complete replacement state"));
    assert_eq!(g.object(id).unwrap().name, "Audit front");
    assert!(g.transform_permanent(id).expect("transform discovery must succeed in this scenario"));
    assert_eq!(g.object(id).unwrap().name, "Audit back");
}

#[test]
fn f25_unearth_haste_survives_cleanup_when_delayed_exile_is_countered() {
    let mut g = game();
    let id = g.create_object_from_definition(&creature("Unearth"), A, Zone::Graveyard);
    let returned = UnearthEffect::new()
        .execute(&mut g, &mut ExecutionContext::new_default(id, A))
        .unwrap()
        .objects()
        .unwrap()[0];
    // Removing the one-shot delayed trigger models it having triggered and been countered.
    g.effect_store.delayed_triggers.clear();
    for _ in 0..2 {
        ironsmith::turn::execute_cleanup_step(&mut g);
        assert!(g.current_has_static_ability_id(returned, StaticAbilityId::Haste));
        g.turn.turn_number += 1;
    }
    assert_eq!(g.object(returned).unwrap().zone, Zone::Battlefield);
}

#[test]
fn f26_phased_out_permanent_cannot_turn_face_up_by_any_special_action_method() {
    for (ability, method) in [
        (
            StaticAbility::morph(TotalCost::free()),
            TurnFaceUpMethod::TurnFaceUpAbility,
        ),
        (
            StaticAbility::megamorph(TotalCost::free()),
            TurnFaceUpMethod::MegamorphAbility,
        ),
        (
            StaticAbility::disguise(TotalCost::free()),
            TurnFaceUpMethod::DisguiseAbility,
        ),
        (
            StaticAbility::morph(TotalCost::free()),
            TurnFaceUpMethod::PrintedManaCost,
        ),
    ] {
        let mut g = game();
        let id = manifest(&mut g, &morph_def(ability));
        g.phase_out(id);
        assert!(!turn_up(&mut g, id, method));
        assert!(!g.can_turn_face_up_permanent(id));
        assert!(!g.set_face_up(id).expect("fixture has complete replacement state"));
        assert!(g.is_face_down(id));
        assert!(!compute_legal_actions(&g, A).expect("fixture has complete replacement state").iter().any(
            |action| matches!(action,LegalAction::TurnFaceUp{creature_id,..} if *creature_id==id)
        ));
        g.phase_in(id);
        assert!(turn_up(&mut g, id, method));
    }
}

#[test]
fn f27_face_up_layers_can_remove_morph_megamorph_and_disguise() {
    for (ability, method) in [
        (
            StaticAbility::morph(TotalCost::free()),
            TurnFaceUpMethod::TurnFaceUpAbility,
        ),
        (
            StaticAbility::megamorph(TotalCost::free()),
            TurnFaceUpMethod::MegamorphAbility,
        ),
        (
            StaticAbility::disguise(TotalCost::free()),
            TurnFaceUpMethod::DisguiseAbility,
        ),
    ] {
        let mut g = game();
        let id = manifest(&mut g, &morph_def(ability));
        modify(
            &mut g,
            id,
            Modification::RemoveAllAbilities,
            Until::EndOfTurn,
        );
        assert!(!turn_up(&mut g, id, method));
        assert!(g.is_face_down(id));
        ironsmith::turn::execute_cleanup_step(&mut g);
        g.turn.priority_player = Some(A);
        assert!(turn_up(&mut g, id, method));
    }
}

#[test]
fn f27_manifest_and_cloak_printed_mana_permission_survives_ability_removal() {
    for cloak in [false, true] {
        let mut g = game();
        g.create_object_from_definition(
            &morph_def(StaticAbility::morph(TotalCost::free())),
            A,
            Zone::Library,
        );
        let source = g.new_object_id();
        let effect = if cloak {
            ManifestTopCardOfLibraryEffect::cloak(PlayerFilter::You)
        } else {
            ManifestTopCardOfLibraryEffect::new(PlayerFilter::You)
        };
        let id = effect
            .execute(&mut g, &mut ExecutionContext::new_default(source, A))
            .unwrap()
            .objects()
            .unwrap()[0];
        modify(&mut g, id, Modification::RemoveAllAbilities, Until::Forever);
        assert!(turn_up(&mut g, id, TurnFaceUpMethod::PrintedManaCost));
    }
}

#[test]
fn f27_morph_granted_by_continuous_effect_is_available() {
    let mut g = game();
    let id = manifest(&mut g, &creature("Granted morph"));
    modify(
        &mut g,
        id,
        Modification::AddAbility(StaticAbility::morph(TotalCost::free())),
        Until::Forever,
    );
    assert!(turn_up(&mut g, id, TurnFaceUpMethod::TurnFaceUpAbility));
}

#[test]
fn f27_ability_removal_filter_uses_hypothetical_face_up_characteristics() {
    let mut g = game();
    let def = B::new(CardId::new(), "Elven morph")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .power_toughness(PowerToughness::fixed(3, 3))
        .with_ability(Ability::static_ability(StaticAbility::morph(
            TotalCost::free(),
        )))
        .build();
    let id = manifest(&mut g, &def);
    let remover = B::new(CardId::new(), "Elves lose abilities")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(
            StaticAbility::remove_all_abilities(
                ObjectFilter::creature().with_subtype(Subtype::Elf),
            ),
        ))
        .build();
    g.create_object_from_definition(&remover, A, Zone::Battlefield);
    assert!(!g.calculated_subtypes(id).contains(&Subtype::Elf));
    assert!(!turn_up(&mut g, id, TurnFaceUpMethod::TurnFaceUpAbility));
    assert!(g.is_face_down(id));
}

fn reconfigure() -> CardDefinition {
    ironsmith_registry::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Audit Reconfigure",
    )
    .card_types(vec![CardType::Artifact, CardType::Creature])
    .subtypes(vec![Subtype::Equipment])
    .power_toughness(PowerToughness::fixed(2, 2))
    .parse_text("Reconfigure {0}")
    .unwrap()
}
fn activation_indices(g: &GameState, id: ObjectId) -> Vec<usize> {
    compute_legal_actions(g, A).expect("fixture has complete replacement state")
        .into_iter()
        .filter_map(|action| match action {
            LegalAction::ActivateAbility {
                source,
                ability_index,
            } if source == id => Some(ability_index),
            _ => None,
        })
        .collect()
}
fn branches(def: &CardDefinition) -> (usize, usize) {
    let mut attach = None;
    let mut unattach = None;
    for (i, ability) in def.abilities.iter().enumerate() {
        if let AbilityKind::Activated(activated) = &ability.kind {
            for effect in activated.effects.iter() {
                if let Some(effect) = effect.downcast_ref::<ReconfigureEffect>() {
                    if effect.target == ChooseSpec::Source {
                        unattach = Some(i)
                    } else {
                        attach = Some(i)
                    }
                }
            }
        }
    }
    (
        attach.expect("attach branch"),
        unattach.expect("unattach branch"),
    )
}

#[test]
fn f28_reconfigure_attach_and_unattach_cannot_follow_a_returned_source() {
    for unattach in [false, true] {
        let mut g = game();
        let id = g.create_object_from_definition(&reconfigure(), A, Zone::Battlefield);
        let target = g.create_object_from_definition(&creature("Bearer"), A, Zone::Battlefield);
        let snapshot = ObjectSnapshot::from_object(g.object(id).unwrap(), &g);
        let hand = g.move_object_by_effect(id, Zone::Hand).unwrap();
        let returned = g.move_object_by_effect(hand, Zone::Battlefield).unwrap();
        if unattach {
            g.attach_object_to_target(returned, AttachmentTarget::Object(target));
        }
        let mut ctx = ExecutionContext::new_default(id, A);
        ctx.source_snapshot = Some(snapshot);
        if !unattach {
            ctx.targets = vec![ResolvedTarget::Object(target)];
        }
        ReconfigureEffect::new(if unattach {
            ChooseSpec::Source
        } else {
            ChooseSpec::target_creature()
        })
        .execute(&mut g, &mut ctx)
        .unwrap();
        assert_eq!(
            g.object(returned).unwrap().attached_to,
            unattach.then_some(AttachmentTarget::Object(target))
        );
    }
}

#[test]
fn f32_reconfigure_exposes_two_distinct_restricted_activations() {
    let def = reconfigure();
    let (attach, unattach) = branches(&def);
    let text = ironsmith::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(text.contains("attached to a creature"), "{text}");

    let mut g = game();
    let id = g.create_object_from_definition(&def, A, Zone::Battlefield);
    assert!(
        activation_indices(&g, id).is_empty(),
        "cannot target itself or unattach while unattached"
    );
    let other = g.create_object_from_definition(&creature("Bearer"), A, Zone::Battlefield);
    assert_eq!(activation_indices(&g, id), vec![attach]);
    let AbilityKind::Activated(ability) = &def.abilities[attach].kind else {
        panic!()
    };
    let effect = ability.effects.iter().next().unwrap();
    let spec = effect.0.get_target_spec().unwrap();
    assert_eq!(
        ironsmith::targeting::compute_legal_targets(&g, spec, A, Some(id)),
        vec![Target::Object(other)]
    );
    assert_eq!(effect.0.get_target_count().unwrap().min, 1);
    g.attach_object_to_target(id, AttachmentTarget::Object(other));
    assert_eq!(activation_indices(&g, id), vec![attach, unattach]);
    g.turn.phase = Phase::Combat;
    assert!(activation_indices(&g, id).is_empty());
    g.turn.phase = Phase::FirstMain;
    modify(
        &mut g,
        other,
        Modification::RemoveCardTypes(vec![CardType::Creature]),
        Until::Forever,
    );
    assert!(
        activation_indices(&g, id).is_empty(),
        "unattach requires a creature attachment"
    );
}

#[derive(Default)]
struct TargetRecorder {
    prompts: Vec<Vec<Target>>,
}
impl DecisionMaker for TargetRecorder {
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let targets = ctx
            .requirements
            .iter()
            .flat_map(|r| r.legal_targets.iter().take(r.min_targets).copied())
            .collect::<Vec<_>>();
        self.prompts.push(targets.clone());
        targets
    }
}
fn activate(
    g: &mut GameState,
    id: ObjectId,
    index: usize,
    dm: &mut impl DecisionMaker,
) -> TriggerQueue {
    g.turn.priority_player = Some(A);
    let action=compute_legal_actions(g,A).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index} if *source==id&&*ability_index==index)).unwrap();
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        g,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..16 {
        if !g.stack.is_empty() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            g, &mut queue, &mut state, &ctx, dm,
        );
    }
    assert!(result.is_ok());
    assert!(!g.stack.is_empty());
    ironsmith::game_loop::drain_pending_trigger_events(g, &mut queue);
    queue
}
#[test]
fn f32_attach_targets_a_creature_and_unattach_has_no_targets_in_normal_activation() {
    let def = reconfigure();
    let (attach, unattach) = branches(&def);
    let mut g = game();
    let id = g.create_object_from_definition(&def, A, Zone::Battlefield);
    let other = g.create_object_from_definition(&creature("Bearer"), A, Zone::Battlefield);
    let mut dm = TargetRecorder::default();
    activate(&mut g, id, attach, &mut dm);
    assert_eq!(g.stack.last().unwrap().targets, vec![Target::Object(other)]);
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
    assert_eq!(
        g.object(id).unwrap().attached_to,
        Some(AttachmentTarget::Object(other))
    );
    assert!(!g.object_has_card_type(id, CardType::Creature));
    dm.prompts.clear();
    activate(&mut g, id, unattach, &mut dm);
    assert!(g.stack.last().unwrap().targets.is_empty());
    assert!(dm.prompts.is_empty());
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
    assert_eq!(g.object(id).unwrap().attached_to, None);
    assert!(g.object_has_card_type(id, CardType::Creature));
}

#[test]
fn f29_day_and_night_skip_phased_out_permanents_then_catch_up_on_phase_in() {
    for night_to_day in [false, true] {
        let mut g = game();
        let front = pair(&mut g, true);
        g.set_daytime(true);
        let id = g.create_object_from_definition(&front, A, Zone::Battlefield);
        if night_to_day {
            g.set_daytime(false);
        }
        let before = g.object(id).unwrap().name.clone();
        let count = g.transform_count(id);
        g.phase_out(id);
        g.set_daytime(night_to_day);
        assert_eq!(g.object(id).unwrap().name, before);
        assert_eq!(g.transform_count(id), count);
        g.phase_in(id);
        g.refresh_continuous_state();
        assert_eq!(g.transform_count(id), count + 1);
    }
}
#[test]
fn f29_day_and_night_use_current_abilities_and_catch_up_when_removal_expires() {
    for night_to_day in [false, true] {
        let mut g = game();
        let front = pair(&mut g, true);
        g.set_daytime(true);
        let id = g.create_object_from_definition(&front, A, Zone::Battlefield);
        if night_to_day {
            g.set_daytime(false);
        }
        modify(
            &mut g,
            id,
            Modification::RemoveAllAbilities,
            Until::EndOfTurn,
        );
        let before = g.object(id).unwrap().name.clone();
        let count = g.transform_count(id);
        g.set_daytime(night_to_day);
        assert_eq!(g.object(id).unwrap().name, before);
        assert_eq!(g.transform_count(id), count);
        ironsmith::turn::execute_cleanup_step(&mut g);
        g.refresh_continuous_state();
        assert_eq!(g.transform_count(id), count + 1);
    }
}
#[test]
fn f29_initial_designation_ignores_phased_out_or_removed_keywords() {
    for (keyword, daytime) in [
        (StaticAbility::daybound(), true),
        (StaticAbility::nightbound(), false),
    ] {
        for phase in [false, true] {
            let mut g = game();
            let id = g.create_object_from_definition(
                &creature("Initially ordinary"),
                A,
                Zone::Battlefield,
            );
            if phase {
                g.phase_out(id);
            } else {
                modify(
                    &mut g,
                    id,
                    Modification::RemoveAllAbilities,
                    Until::EndOfTurn,
                );
            }
            std::sync::Arc::make_mut(&mut g.object_mut(id).unwrap().abilities)
                .push(Ability::static_ability(keyword.clone()));
            g.refresh_continuous_state();
            assert!(!g.has_day_night());
            if phase {
                g.phase_in(id);
            } else {
                ironsmith::turn::execute_cleanup_step(&mut g);
            }
            g.refresh_continuous_state();
            assert!(g.has_day_night());
            assert_eq!(g.is_daytime(), daytime);
        }
    }
}

#[test]
fn f29_daybound_transform_restriction_is_removed_with_the_ability_for_both_actions() {
    for convert in [false, true] {
        let mut g = game();
        let front = pair(&mut g, true);
        g.set_daytime(true);
        let id = g.create_object_from_definition(&front, A, Zone::Battlefield);
        let mut ctx = ExecutionContext::new_default(id, A);
        if convert {
            ConvertEffect::new(ChooseSpec::Source)
                .execute(&mut g, &mut ctx)
                .unwrap();
        } else {
            TransformEffect::new(ChooseSpec::Source)
                .execute(&mut g, &mut ctx)
                .unwrap();
        }
        assert_eq!(g.transform_count(id), 0);
        modify(&mut g, id, Modification::RemoveAllAbilities, Until::Forever);
        if convert {
            ConvertEffect::new(ChooseSpec::Source)
                .execute(&mut g, &mut ctx)
                .unwrap();
        } else {
            TransformEffect::new(ChooseSpec::Source)
                .execute(&mut g, &mut ctx)
                .unwrap();
        }
        assert_eq!(g.transform_count(id), 1);
    }
}

#[test]
fn f29_day_night_transitions_emit_once_and_initial_designation_is_not_a_transition() {
    let mut g = game();
    let id = g.create_object_from_definition(&creature("Initially ordinary"), A, Zone::Battlefield);
    g.phase_out(id);
    std::sync::Arc::make_mut(&mut g.object_mut(id).unwrap().abilities)
        .push(Ability::static_ability(StaticAbility::daybound()));
    g.refresh_continuous_state();
    g.take_pending_trigger_events();
    g.phase_in(id);
    g.refresh_continuous_state();
    assert!(g.is_daytime());
    let count = |events: Vec<ironsmith::triggers::TriggerEvent>| {
        events
            .iter()
            .filter(|event| {
                event
                    .downcast::<ironsmith::events::DayNightChangedEvent>()
                    .is_some()
            })
            .count()
    };
    assert_eq!(count(g.take_pending_trigger_events()), 0);
    g.set_daytime(false);
    g.set_daytime(false);
    g.refresh_continuous_state();
    assert_eq!(count(g.take_pending_trigger_events()), 1);
    g.set_daytime(true);
    g.set_daytime(true);
    g.refresh_continuous_state();
    assert_eq!(count(g.take_pending_trigger_events()), 1);
}

#[test]
fn f32_only_attach_activation_fires_becoming_target_triggers() {
    let def = reconfigure();
    let (attach, unattach) = branches(&def);
    let mut g = game();
    let id = g.create_object_from_definition(&def, A, Zone::Battlefield);
    let watcher = ironsmith_registry::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Target watcher",
    )
    .card_types(vec![CardType::Creature])
    .power_toughness(PowerToughness::fixed(3, 3))
    .parse_text("Whenever this creature becomes the target of a spell or ability, you gain 1 life.")
    .unwrap();
    let bearer = g.create_object_from_definition(&watcher, A, Zone::Battlefield);
    let mut dm = TargetRecorder::default();
    let queue = activate(&mut g, id, attach, &mut dm);
    let queued = queue
        .entries
        .iter()
        .filter(|entry| entry.source == bearer)
        .count();
    let stacked = g
        .stack
        .iter()
        .filter(|entry| entry.source_name.as_deref() == Some("Target watcher"))
        .count();
    assert_eq!(queued + stacked, 1);
    while !g.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
    }
    let queue = activate(&mut g, id, unattach, &mut dm);
    assert!(queue.entries.is_empty());
    assert_eq!(g.stack.len(), 1);
    assert!(
        g.stack
            .iter()
            .all(|entry| entry.source_name.as_deref() != Some("Target watcher"))
    );
}

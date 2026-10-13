//! Regression contracts for the remaining executor decomposition.
use super::{captured_selection::*, scoped_choice::*};
use crate::decisions::context::{SelectObjectsContext, SelectOptionsContext, SelectableOption};
use crate::{
    effects::{EffectExecutor, ExecutionContext, ExecutionError},
    game_state::GameState,
    ids::{ObjectId, PlayerId},
};

#[derive(Default)]
struct Answer {
    options: Vec<usize>,
    objects: Vec<ObjectId>,
    pending: bool,
    menus: Vec<SelectOptionsContext>,
    object_menus: Vec<SelectObjectsContext>,
}
impl crate::decision::DecisionMaker for Answer {
    fn decide_options(&mut self, _: &GameState, menu: &SelectOptionsContext) -> Vec<usize> {
        self.menus.push(menu.clone());
        self.options.clone()
    }
    fn decide_objects(&mut self, _: &GameState, menu: &SelectObjectsContext) -> Vec<ObjectId> {
        self.object_menus.push(menu.clone());
        self.objects.clone()
    }
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}
fn game() -> GameState {
    crate::tests::test_helpers::setup_two_player_game()
}
fn card(game: &mut GameState, zone: crate::Zone) -> ObjectId {
    game.create_object_from_card(
        &crate::card::CardBuilder::new(crate::CardId::new(), "Subject")
            .card_types(vec![crate::CardType::Creature])
            .build(),
        PlayerId(0),
        zone,
    )
}

#[test]
fn typed_menu_keeps_object_metadata_and_never_defaults_a_pending_answer() {
    let game = game();
    for (answer, pending, expected) in [
        (vec![1], false, Some(20)),
        (vec![99, 1], false, Some(10)),
        (vec![], false, Some(10)),
        (vec![1], true, None),
    ] {
        let mut dm = Answer {
            options: answer,
            pending,
            ..Default::default()
        };
        let chosen = select_typed_menu(
            &game,
            &mut dm,
            PlayerId(1),
            ObjectId(7),
            "History",
            &[10, 20],
            SingleChoicePolicy::FirstOrDefault,
            |i, value| {
                SelectableOption::new(i, format!("Cast {value}")).with_object(ObjectId(*value))
            },
        )
        .unwrap();
        assert_eq!(chosen, expected);
        assert_eq!(dm.menus[0].player, PlayerId(1));
        assert_eq!(dm.menus[0].source, Some(ObjectId(7)));
        assert_eq!(dm.menus[0].options[1].object_id, Some(ObjectId(20)));
    }
}
#[test]
fn strict_prevention_choice_preserves_arity_and_invalid_index_errors() {
    let game = game();
    for (answer, expected) in [
        (vec![], "arity"),
        (vec![0, 1], "arity"),
        (vec![99], "index"),
    ] {
        let mut dm = Answer {
            options: answer,
            ..Default::default()
        };
        let mut ctx = ExecutionContext::new(ObjectId(7), PlayerId(0), &mut dm);
        let error = select_typed_option(
            &game,
            &mut ctx,
            PlayerId(0),
            "Color",
            &[(10, "White".into())],
            SingleChoicePolicy::StrictMessages {
                arity: "arity",
                index: "index",
            },
        )
        .unwrap_err();
        assert!(matches!(error, ExecutionError::UnresolvableValue(message) if message == expected));
    }
}
#[test]
fn captured_object_answer_policies_do_not_conflate_first_valid_and_last() {
    let candidates = [ObjectId(1), ObjectId(2)];
    for (answers, first, valid, last) in [
        (
            vec![ObjectId(99), ObjectId(2)],
            None,
            Some(ObjectId(2)),
            Some(ObjectId(2)),
        ),
        (
            vec![ObjectId(2), ObjectId(99)],
            Some(ObjectId(2)),
            Some(ObjectId(2)),
            Some(ObjectId(1)),
        ),
        (vec![], None, Some(ObjectId(1)), Some(ObjectId(1))),
    ] {
        for (policy, expected) in [
            (ObjectAnswerPolicy::First, first),
            (ObjectAnswerPolicy::FirstValidOrDefault, valid),
            (ObjectAnswerPolicy::LastOrDefault, last),
        ] {
            assert_eq!(
                select_captured_answer(&candidates, &answers, policy).unwrap(),
                expected
            );
        }
        assert!(select_captured_answer(&candidates, &answers, ObjectAnswerPolicy::Strict).is_err());
    }
}
#[test]
fn explicit_singleton_choice_is_distinct_from_an_automatic_singleton() {
    let mut game = game();
    let id = card(&mut game, crate::Zone::Battlefield);
    for skip in [false, true] {
        let mut dm = Answer::default();
        let mut ctx = ExecutionContext::new(id, PlayerId(0), &mut dm);
        let selected = choose_captured_object(
            &game,
            &mut ctx,
            PlayerId(1),
            &[id],
            "Attach",
            CapturedChoicePolicy {
                skip_singleton: skip,
                ..CapturedChoicePolicy::EXPLICIT_FIRST
            },
        )
        .unwrap();
        assert_eq!(selected, skip.then_some(id));
        drop(ctx);
        assert_eq!(dm.object_menus.len(), usize::from(!skip));
        if !skip {
            assert!(dm.object_menus[0].require_explicit_choice);
            assert_eq!(dm.object_menus[0].player, PlayerId(1));
        }
    }
}
#[test]
fn later_suspended_choice_does_not_publish_a_partial_plan() {
    struct Pause {
        calls: usize,
    }
    impl crate::decision::DecisionMaker for Pause {
        fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> {
            self.calls += 1;
            vec![0]
        }
        fn awaiting_choice(&self) -> bool {
            self.calls == 2
        }
    }
    let mut game = game();
    let mut dm = Pause { calls: 0 };
    let mut ctx = ExecutionContext::new(ObjectId(99), PlayerId(0), &mut dm);
    let mut visits = vec![];
    let plan = collect_choices_before_commit(
        &mut game,
        &mut ctx,
        [PlayerId(0), PlayerId(1), PlayerId(2)],
        |game, ctx, player| {
            visits.push(player);
            select_typed_option(
                game,
                ctx,
                player,
                "Choose",
                &[(1, "One".into())],
                SingleChoicePolicy::First,
            )
        },
    )
    .unwrap();
    assert!(plan.is_none());
    assert_eq!(visits, vec![PlayerId(0), PlayerId(1)]);
}
#[test]
fn nested_source_actor_scopes_restore_on_error_and_keep_unrelated_outputs() {
    let mut game = game();
    let source = card(&mut game, crate::Zone::Battlefield);
    let snapshot = crate::snapshot::ObjectSnapshot::from_object_id(&game, source);
    let mut ctx = ExecutionContext::new_default(ObjectId(99), PlayerId(0));
    let original_cause = format!("{:?}", ctx.cause);
    let result: Result<(), ExecutionError> =
        super::execute_with_source::with_source_binding(&mut ctx, &(source, snapshot), |ctx| {
            super::execute_with_source::with_actor_binding(ctx, PlayerId(1), None, |ctx| {
                assert_eq!(ctx.source, source);
                assert_eq!(ctx.controller, PlayerId(1));
                assert_eq!(format!("{:?}", ctx.cause), original_cause);
                ctx.set_tagged_players("output", vec![PlayerId(1)]);
                super::execute_with_source::with_actor_binding(
                    ctx,
                    PlayerId(0),
                    Some(crate::events::cause::EventCause::from_effect(
                        source,
                        PlayerId(0),
                    )),
                    |_| Err(ExecutionError::InvalidTarget),
                )
            })
        });
    assert!(result.is_err());
    assert_eq!(ctx.source, ObjectId(99));
    assert!(ctx.source_snapshot.is_none());
    assert_eq!(ctx.controller, PlayerId(0));
    assert_eq!(format!("{:?}", ctx.cause), original_cause);
    assert_eq!(ctx.tagged_players.get("output"), Some(&vec![PlayerId(1)]));
}
#[test]
fn filter_traversal_preserves_field_and_wrapper_boundaries() {
    use crate::effects::player_reference_binding::*;
    use crate::target::{ObjectFilter, PlayerFilter};
    let base = ObjectFilter {
        controller: Some(PlayerFilter::IteratedPlayer),
        owner: Some(PlayerFilter::OpponentOf(Box::new(
            PlayerFilter::IteratedPlayer,
        ))),
        cast_by: Some(PlayerFilter::IteratedPlayer),
        attached_to_player: Some(PlayerFilter::IteratedPlayer),
        attached_to_object: Some(Box::new(ObjectFilter {
            owner: Some(PlayerFilter::IteratedPlayer),
            ..Default::default()
        })),
        any_of: vec![ObjectFilter {
            owner: Some(PlayerFilter::IteratedPlayer),
            ..Default::default()
        }],
        ..Default::default()
    };
    for fields in [
        PlayerReferenceFields::ControllerOwner,
        PlayerReferenceFields::SpellFacets,
        PlayerReferenceFields::AllAuthored,
    ] {
        let mut filter = base.clone();
        walk_filter_references(
            &mut filter,
            fields,
            &mut |reference| {
                if matches!(reference, PlayerFilter::IteratedPlayer) {
                    *reference = PlayerFilter::Specific(PlayerId(1));
                }
                Ok::<_, ExecutionError>(())
            },
            &mut |_| Ok(()),
        )
        .unwrap();
        assert_eq!(filter.controller, Some(PlayerFilter::Specific(PlayerId(1))));
        assert_eq!(
            filter.any_of[0].owner,
            Some(PlayerFilter::Specific(PlayerId(1)))
        );
        assert_eq!(filter.owner, base.owner, "wrapper binding is opt-in");
        let wide = matches!(fields, PlayerReferenceFields::AllAuthored);
        assert_eq!(
            filter.attached_to_player == Some(PlayerFilter::Specific(PlayerId(1))),
            wide
        );
        assert_eq!(
            filter.attached_to_object.as_ref().unwrap().owner
                == Some(PlayerFilter::Specific(PlayerId(1))),
            wide
        );
        assert_eq!(
            filter.cast_by == Some(PlayerFilter::Specific(PlayerId(1))),
            !matches!(fields, PlayerReferenceFields::ControllerOwner)
        );
    }
    let mut nested = base.owner.unwrap();
    walk_player_reference_wrappers(&mut nested, &mut |reference| {
        if matches!(reference, PlayerFilter::IteratedPlayer) {
            *reference = PlayerFilter::Specific(PlayerId(1));
        }
    });
    assert_eq!(
        nested,
        PlayerFilter::OpponentOf(Box::new(PlayerFilter::Specific(PlayerId(1))))
    );
}
#[test]
fn player_order_policies_keep_custom_order_missing_start_and_unknown_participants() {
    use super::player_order::*;
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    let [a, b, c] = [PlayerId(0), PlayerId(1), PlayerId(2)];
    game.turn_store.turn_order = vec![c, b, a];
    game.turn.active_player = b;
    assert_eq!(
        active_players_from(&game, b, PlayerOrderBasis::Seating),
        vec![b, c, a]
    );
    assert_eq!(
        active_players_from(&game, b, PlayerOrderBasis::TurnWithSeatingFallback),
        vec![b, a, c]
    );
    assert_eq!(apnap_position(&game, a), 1);
    assert_eq!(apnap_position(&game, PlayerId(9)), usize::MAX);
    assert_eq!(
        selected_players_from(&game, vec![PlayerId(9), c, a], a),
        vec![a, c, PlayerId(9)]
    );
    game.turn_store.turn_order = vec![c, a];
    assert_eq!(
        active_players_from(&game, b, PlayerOrderBasis::TurnWithSeatingFallback),
        vec![b, c, a]
    );
    assert_eq!(apnap_position(&game, c), 2);
    game.turn_store.turn_order.clear();
    assert_eq!(selected_players_from(&game, vec![c, c, a], b), vec![c, a]);
}
#[test]
fn extrema_keeps_all_ties_excludes_absent_and_can_choose_minimum() {
    let values = [(0, None), (1, Some(4)), (2, Some(4)), (3, Some(2))];
    assert_eq!(
        retain_extrema_by_key(values, |(_, value)| *value, true),
        vec![values[1], values[2]]
    );
    assert_eq!(
        retain_extrema_by_key(values, |(_, value)| *value, false),
        vec![values[3]]
    );
    assert!(retain_extrema_by_key([None::<i32>, None], |value| *value, true).is_empty());
}
#[test]
fn delayed_exile_exposes_its_body_only_to_structural_inspection() {
    let effect = crate::effects::ExileTaggedWhenSourceLeavesEffect::new(
        "subjects",
        crate::target::PlayerFilter::You,
    );
    let mut children = vec![];
    effect.visit_child_effects(&mut |child| children.push(child.clone()));
    assert_eq!(children.len(), 1);
    assert!(
        children[0]
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some()
    );
    effect.visit_resolution_child_effects(&mut |_| {
        panic!("delayed body must not execute on scheduling")
    });
}
#[test]
fn outcome_projection_preserves_instruction_result_and_facts() {
    let mut aggregate = crate::effect::EffectOutcome::count(99);
    aggregate.instruction_result = Some(Box::new(crate::effect::EffectOutcome::count(1)));
    let untouched = super::original_observations::project_outcome_summary(aggregate, |_| None);
    assert_eq!(untouched.as_count(), Some(99));
    assert_eq!(untouched.instruction_result().as_count(), Some(1));

    let outcome = crate::effect::EffectOutcome::with_objects(vec![ObjectId(123)])
        .with_execution_fact(crate::effect::ExecutionFact::ChosenNumber(7));
    let prior_result = format!("{:?}", outcome.instruction_result);
    let projected = super::original_observations::project_outcome_summary(outcome, |_| {
        Some(crate::effect::EffectOutcome::target_invalid())
    });
    assert_eq!(format!("{:?}", projected.instruction_result), prior_result);
    assert_eq!(projected.execution_facts.len(), 1);
    assert_eq!(
        projected.status,
        crate::effect::OutcomeStatus::TargetInvalid
    );
}

#[test]
fn empty_recipient_prompt_policy_is_explicit() {
    let game = game();
    for prompt_empty in [false, true] {
        let mut dm = Answer::default();
        let mut ctx = ExecutionContext::new(ObjectId(99), PlayerId(0), &mut dm);
        assert_eq!(
            choose_captured_object(
                &game,
                &mut ctx,
                PlayerId(0),
                &[],
                "Recipient",
                CapturedChoicePolicy {
                    prompt_empty,
                    ..CapturedChoicePolicy::EXPLICIT_FIRST
                }
            )
            .unwrap(),
            None
        );
        drop(ctx);
        assert_eq!(dm.object_menus.len(), usize::from(prompt_empty));
    }
}

#[test]
fn umbra_completion_clears_damage_when_aura_is_protected_or_missing() {
    for protected in [false, true] {
        let mut game = game();
        let permanent = card(&mut game, crate::Zone::Battlefield);
        game.mark_damage(permanent, 2);
        let mut definition = crate::cards::CardDefinition::new(
            crate::card::CardBuilder::new(crate::CardId::new(), "Aura")
                .card_types(vec![crate::CardType::Enchantment])
                .build(),
        );
        if protected {
            definition
                .abilities
                .push(crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::indestructible(),
                ));
        }
        let aura =
            game.create_object_from_definition(&definition, PlayerId(0), crate::Zone::Battlefield);
        game.object_mut(aura).unwrap().attached_to =
            Some(crate::object::AttachmentTarget::Object(permanent));
        let mut ctx = ExecutionContext::new_default(aura, PlayerId(0));
        crate::effects::UmbraArmorEffect::new(aura)
            .execute_with_outputs(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.damage_on(permanent), 0);
        assert_eq!(
            game.object(aura)
                .is_some_and(|o| o.zone == crate::Zone::Battlefield),
            protected
        );
    }
    let mut game = game();
    let mut ctx = ExecutionContext::new_default(ObjectId(99), PlayerId(0));
    let output = crate::effects::UmbraArmorEffect::new(ObjectId(98))
        .execute_with_outputs(&mut game, &mut ctx)
        .unwrap();
    // The destruction owner returns a receipt even for this absent Aura;
    // the replacement payload preserves its resolved summary.
    assert_eq!(
        output.outcome.status,
        crate::effect::OutcomeStatus::Succeeded
    );
    assert!(output.outcome.events.is_empty());
}

#[test]
fn retained_player_extrema_uses_snapshot_owner_and_maximum_per_player() {
    let mut game = game();
    let mut snapshots = Vec::new();
    for (owner, cost) in [(PlayerId(0), 1), (PlayerId(0), 7), (PlayerId(1), 4)] {
        let definition = crate::card::CardBuilder::new(crate::CardId::new(), "Retained")
            .mana_cost(crate::mana::ManaCost::new().add_generic(cost))
            .build();
        let id = game.create_object_from_card(&definition, owner, crate::Zone::Exile);
        snapshots.push(crate::snapshot::ObjectSnapshot::from_object_id(&game, id).unwrap());
        game.remove_object(id);
    }
    let mut ctx = ExecutionContext::new_default(ObjectId(99), PlayerId(0));
    ctx.set_tagged_players("players", vec![PlayerId(0), PlayerId(1), PlayerId(2)]);
    ctx.set_tagged_objects("round", snapshots);
    let effect = crate::effects::KeepGreatestManaValuePlayersEffect::new("players", "round");
    assert_eq!(
        effect.execute(&mut game, &mut ctx).unwrap().as_count(),
        Some(1)
    );
    assert_eq!(ctx.tagged_players.get("players"), Some(&vec![PlayerId(0)]));
    assert!(ctx.get_tagged_all("round").is_none());
    assert_eq!(
        effect.execute(&mut game, &mut ctx).unwrap().as_count(),
        Some(0)
    );
    assert!(ctx.tagged_players.get("players").unwrap().is_empty());
}

#[test]
fn correlated_tag_aliases_preserve_export_and_lexical_restoration() {
    #[derive(Clone, Debug)]
    struct Produce;
    impl EffectExecutor for Produce {
        fn execute(
            &self,
            _: &mut GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<crate::effect::EffectOutcome, ExecutionError> {
            let snapshots = ctx.get_tagged_all("__it__").cloned().unwrap();
            ctx.set_tagged_objects("result", snapshots);
            Ok(crate::effect::EffectOutcome::count(1))
        }
    }
    for binding in ["source", "__it__", "result"] {
        let mut game = game();
        let id = card(&mut game, crate::Zone::Battlefield);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_id(&game, id).unwrap();
        let mut ctx = ExecutionContext::new_default(id, PlayerId(0));
        ctx.set_tagged_objects("__it__", vec![snapshot.clone()]);
        ctx.set_tagged_objects("source", vec![snapshot.clone()]);
        let effect = crate::effects::ForEachObjectCorrelatedResultEffect::new(
            crate::target::ObjectFilter::creature(),
            vec![crate::effect::Effect::new(Produce)],
            "result",
            binding,
            "consumer",
            vec![],
        );
        effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(ctx.get_tagged_all("result").unwrap()[0].object_id, id);
        assert!(ctx.get_tagged_all("consumer").is_none());
        // Duplicate lexical removals historically restore in removal order.
        if binding == "__it__" {
            assert!(ctx.get_tagged_all("__it__").is_none());
        } else {
            assert_eq!(ctx.get_tagged_all("__it__").unwrap()[0].object_id, id);
        }
    }
}

#[test]
fn delayed_exile_keeps_one_exact_subject_body_per_source_watcher() {
    let mut game = game();
    let source = card(&mut game, crate::Zone::Battlefield);
    let subjects = [
        card(&mut game, crate::Zone::Battlefield),
        card(&mut game, crate::Zone::Battlefield),
    ];
    let mut ctx = ExecutionContext::new_default(source, PlayerId(0));
    ctx.set_tagged_objects(
        "subjects",
        subjects
            .iter()
            .map(|id| crate::snapshot::ObjectSnapshot::from_object_id(&game, *id).unwrap())
            .collect(),
    );
    let effect = crate::effects::ExileTaggedWhenSourceLeavesEffect::new(
        "subjects",
        crate::target::PlayerFilter::Specific(PlayerId(1)),
    );
    assert_eq!(
        effect.execute(&mut game, &mut ctx).unwrap().as_count(),
        Some(2)
    );
    assert_eq!(game.effect_store.delayed_triggers.len(), 2);
    for (trigger, subject) in game.effect_store.delayed_triggers.iter().zip(subjects) {
        assert_eq!(trigger.target_objects, vec![source]);
        assert_eq!(trigger.controller, PlayerId(1));
        assert!(trigger.one_shot);
        let effects = trigger.effects.all_effects();
        assert_eq!(effects.len(), 1);
        let movement = effects[0]
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .unwrap();
        assert_eq!(
            movement.target,
            crate::target::ChooseSpec::SpecificObject(subject)
        );
        assert_eq!(movement.zone, crate::Zone::Exile);
    }
}

use super::*;
use crate::card::CardBuilder;
use crate::decision::AutoPassDecisionMaker;
use crate::ids::CardId;

#[test]
pub(super) fn creature_type_announcement_precedes_targets_and_survives_spell_copy() {
    for x in [0, 1, 2, 3] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        game.turn.active_player = alice;
        game.turn.phase = crate::game_state::Phase::NextMain;
        game.turn.step = None;
        let mut objects = Vec::new();
        for subtype in [
            crate::types::Subtype::Goblin,
            crate::types::Subtype::Goblin,
            crate::types::Subtype::Elf,
        ] {
            let card = CardBuilder::new(CardId::new(), "Subtype fixture")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![subtype])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
            objects.push(game.create_object_from_card(&card, bob, Zone::Battlefield));
        }
        let card = CardBuilder::new(CardId::new(), "Announced type fixture")
            .card_types(vec![CardType::Sorcery])
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Stack);
        let program = crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
            crate::effects::ReturnToHandEffect::targets(
                ChooseSpec::Object(ObjectFilter::creature().of_chosen_creature_type()),
                crate::effect::ChoiceCount::dynamic_x(),
            ),
        )]);
        game.object_mut(source).unwrap().spell_effect = Some(program.clone().into());
        game.object_mut(source).unwrap().x_value = Some(x);
        let mut diagnostic = game.clone();
        diagnostic.set_chosen_subtype(source, crate::types::Subtype::Goblin);
        assert_eq!(
            spell_program_has_legal_targets_with_modes(&game, &program, alice, Some(source), None),
            x <= 2,
            "x={x}, goblin requirements={:#?}",
            extract_target_requirements_from_program_with_modes(
                &diagnostic,
                &program,
                alice,
                Some(source),
                None
            )
        );
        assert!(
            game.chosen_subtype(source).is_none(),
            "preflight must not commit a choice"
        );
        if x == 3 {
            continue;
        }
        let requirements = extract_target_requirements_from_program_with_modes(
            &game,
            &program,
            alice,
            Some(source),
            None,
        );
        let mut pending = PendingCast::new(
            source,
            Zone::Hand,
            alice,
            crate::provenance::ProvNodeId::default(),
            CastStage::ChoosingTargets,
            None,
            requirements,
            CastingMethod::Normal,
            crate::cost::OptionalCostsPaid::default(),
            None,
            source,
        );
        pending.x_value = Some(x);
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut queue = TriggerQueue::new();
        let mut dm = AutoPassDecisionMaker;
        let progress = continue_to_targets_or_mana_payment(
            &mut game, &mut queue, &mut state, pending, &mut dm,
        )
        .unwrap();
        let GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(options),
        ) = progress
        else {
            panic!("type choice must precede targets: {progress:?}");
        };
        assert_eq!(options.player, alice);
        let goblin = options
            .options
            .iter()
            .find(|o| o.description.eq_ignore_ascii_case("goblin"))
            .unwrap()
            .index;
        assert_eq!(
            options
                .options
                .iter()
                .any(|o| o.description.eq_ignore_ascii_case("elf")),
            x < 2
        );
        assert_eq!(
            options
                .options
                .iter()
                .any(|o| o.description.eq_ignore_ascii_case("dragon")),
            x == 0
        );
        assert!(
            apply_creature_type_announcement_response(
                &mut game,
                &mut queue,
                &mut state,
                usize::MAX,
                &mut dm
            )
            .is_err()
        );
        assert!(game.chosen_subtype(source).is_none());
        let progress = apply_creature_type_announcement_response(
            &mut game, &mut queue, &mut state, goblin, &mut dm,
        )
        .unwrap();
        assert_eq!(
            game.chosen_subtype(source),
            Some(crate::types::Subtype::Goblin)
        );
        let GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(
            targets,
        )) = progress
        else {
            panic!("target choice required: {progress:?}");
        };
        assert_eq!(targets.requirements[0].min_targets, x as usize);
        assert_eq!(targets.requirements[0].max_targets, Some(x as usize));
        assert_eq!(
            targets.requirements[0].legal_targets,
            vec![Target::Object(objects[0]), Target::Object(objects[1])]
        );
        let chosen = objects[..x as usize]
            .iter()
            .copied()
            .map(Target::Object)
            .collect::<Vec<_>>();
        apply_targets_response(&mut game, &mut queue, &mut state, &chosen, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, chosen);
        let copy_effect = Effect::new(crate::effects::CopySpellEffect::single(ChooseSpec::spell()));
        let mut ctx = crate::effects::ExecutionContext::new_default(source, bob)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(source)]);
        crate::effects::execute_effect(&mut game, &copy_effect, &mut ctx).unwrap();
        let copy = game.stack.last().unwrap().object_id;
        assert_ne!(copy, source);
        assert_eq!(
            game.chosen_subtype(copy),
            Some(crate::types::Subtype::Goblin)
        );
        assert_eq!(game.stack.last().unwrap().controller, bob);
        let copy_requirements = extract_target_requirements_from_program_with_modes(
            &game,
            &program,
            bob,
            Some(copy),
            None,
        );
        assert!(
            !copy_requirements[0]
                .legal_targets
                .contains(&Target::Object(objects[2]))
        );
        if x == 2 {
            game.object_mut(objects[0]).unwrap().subtypes = vec![crate::types::Subtype::Elf].into();
            game.mark_continuous_state_dirty();
        }
        resolve_stack_entry(&mut game).unwrap();
        if x == 2 {
            assert_eq!(
                game.object(objects[0]).unwrap().zone,
                Zone::Battlefield,
                "a changed type invalidates that target"
            );
            assert!(
                game.object(objects[1]).is_none(),
                "the remaining legal target still returns"
            );
        } else if x == 1 {
            assert!(game.object(objects[0]).is_none());
        }
        assert_eq!(game.object(objects[2]).unwrap().zone, Zone::Battlefield);
        resolve_stack_entry(&mut game).unwrap();
        assert!(game.stack.is_empty());
        assert_eq!(game.player(bob).unwrap().hand.len(), usize::from(x > 0));
    }
}

struct AnnouncedTypeCastDecisions {
    targets: Vec<Target>,
    order: Vec<&'static str>,
}
impl DecisionMaker for AnnouncedTypeCastDecisions {
    fn decide_number(
        &mut self,
        _: &GameState,
        _: &crate::decisions::context::NumberContext,
    ) -> u32 {
        self.order.push("X");
        2
    }
    fn decide_options(
        &mut self,
        _: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if let Some(option) = ctx
            .options
            .iter()
            .find(|o| o.description.eq_ignore_ascii_case("Goblin"))
        {
            self.order.push("type");
            vec![option.index]
        } else {
            ctx.options
                .first()
                .map(|o| vec![o.index])
                .unwrap_or_default()
        }
    }
    fn decide_targets(
        &mut self,
        _: &GameState,
        _: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        self.order.push("targets");
        self.targets.clone()
    }
}

#[test]
fn creature_type_announcement_real_cast_pays_x_and_can_roll_back() {
    for cancel in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        game.turn.active_player = alice;
        game.turn.phase = crate::game_state::Phase::NextMain;
        game.turn.step = None;
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Blue, 1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Colorless, 2);
        let card = CardBuilder::new(CardId::new(), "Cast type fixture")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![
                vec![crate::mana::ManaSymbol::X],
                vec![crate::mana::ManaSymbol::Blue],
            ]))
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Hand);
        let program = crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
            crate::effects::ReturnToHandEffect::targets(
                ChooseSpec::Object(ObjectFilter::creature().of_chosen_creature_type()),
                crate::effect::ChoiceCount::dynamic_x(),
            ),
        )]);
        game.object_mut(source).unwrap().spell_effect = Some(program.into());
        let mut targets = Vec::new();
        for _ in 0..2 {
            let goblin = CardBuilder::new(CardId::new(), "Goblin fixture")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Goblin])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
            targets.push(Target::Object(game.create_object_from_card(
                &goblin,
                bob,
                Zone::Battlefield,
            )));
        }
        let action = compute_legal_actions(&game, alice).into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source))
            .expect("the spell must be offered before the subtype is known");
        let mut dm = AnnouncedTypeCastDecisions {
            targets,
            order: Vec::new(),
        };
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut queue = TriggerQueue::new();
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..12 {
            if cancel
                && state
                    .pending_cast
                    .as_ref()
                    .is_some_and(|p| p.stage == CastStage::ChoosingTargets)
            {
                let stack_source = state.pending_cast.as_ref().unwrap().spell_id;
                assert_eq!(game.chosen_subtype(stack_source), Some(Subtype::Goblin));
                assert!(state.rollback_action(&mut game));
                assert!(game.chosen_subtype(stack_source).is_none());
                assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
                assert_eq!(game.player(alice).unwrap().mana_pool.total(), 3);
                break;
            }
            if !state.has_pending_action() {
                break;
            }
            let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("unhandled cast progress: {progress:?}");
            };
            progress =
                apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm)
                    .unwrap();
        }
        if cancel {
            assert!(game.stack.is_empty());
        } else {
            assert_eq!(dm.order, vec!["X", "type", "targets"]);
            assert_eq!(game.stack.len(), 1);
            assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
            resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(bob).unwrap().hand.len(), 2);
        }
    }
}

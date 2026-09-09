use super::*;
const TEXT: &str = "{4}, {T}: An opponent chooses target creature they control. Create a token that's a copy of that creature. That token gains haste until end of turn. Exile the token at the beginning of the next end step. Activate only as a sorcery.";
#[test]
fn opponent_target_copy_exposes_target_before_resolution() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Echo Chamber")
        .card_types(vec![CardType::Artifact])
        .parse_text(TEXT)
        .unwrap();
    let ability = definition
        .abilities
        .iter()
        .find_map(|a| {
            if let AbilityKind::Activated(a) = &a.kind {
                Some(a)
            } else {
                None
            }
        })
        .unwrap();
    fn target(effect: &Effect) -> Option<&crate::effects::TargetOnlyEffect> {
        if let Some(tag) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return target(&tag.effect);
        }
        if let Some(id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return target(&id.effect);
        }
        effect.downcast_ref()
    }
    let declaration = ability
        .effects
        .segments
        .iter()
        .flat_map(|s| &s.default_effects)
        .find_map(target)
        .expect("explicit target declaration");
    assert_eq!(declaration.chooser, Some(PlayerFilter::Opponent));
    let ChooseSpec::Object(filter) = declaration.target.base() else {
        panic!("object target")
    };
    assert_eq!(filter.controller, Some(PlayerFilter::IteratedPlayer));
}

struct CopyChoice {
    alice: crate::ids::PlayerId,
    carol: crate::ids::PlayerId,
    target: crate::ids::ObjectId,
    other: crate::ids::ObjectId,
    protected: crate::ids::ObjectId,
    chose_player: bool,
    chose_target: bool,
}
impl crate::decision::DecisionMaker for CopyChoice {
    fn decide_options(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if let Some(option) = ctx.options.iter().find(|o| o.description == "Carol") {
            assert_eq!(ctx.player, self.alice);
            self.chose_player = true;
            return vec![option.index];
        }
        ctx.options
            .iter()
            .filter(|o| o.legal)
            .take(ctx.min)
            .map(|o| o.index)
            .collect()
    }
    fn decide_targets(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<crate::game_state::Target> {
        assert_eq!(ctx.player, self.carol);
        self.chose_target = true;
        assert_eq!(ctx.requirements.len(), 1);
        assert!(
            ctx.requirements[0]
                .legal_targets
                .contains(&crate::game_state::Target::Object(self.target))
        );
        assert!(
            !ctx.requirements[0]
                .legal_targets
                .contains(&crate::game_state::Target::Object(self.other))
        );
        assert!(
            !ctx.requirements[0]
                .legal_targets
                .contains(&crate::game_state::Target::Object(self.protected))
        );
        vec![crate::game_state::Target::Object(self.target)]
    }
}
#[test]
fn opponent_target_copy_activates_with_delegated_target_and_exiles_only_its_token() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Echo Chamber")
        .card_types(vec![CardType::Artifact])
        .parse_text(TEXT)
        .unwrap();
    for change in ["keep", "leave", "control"] {
        let remove_target = change != "keep";
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let carol = game.players[2].id;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = crate::game_state::Phase::FirstMain;
        game.turn.step = None;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Original")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(3, 4))
            .build();
        let target = game.create_object_from_definition(&creature, carol, Zone::Battlefield);
        let other = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        let protected_definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Protected Target")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(3, 4))
                .parse_text("Hexproof")
                .unwrap();
        let protected =
            game.create_object_from_definition(&protected_definition, carol, Zone::Battlefield);

        let unrelated_definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Unrelated Token")
                .card_types(vec![CardType::Artifact])
                .build();
        {
            let mut ctx = crate::effects::EffectContext::new_default(source, alice);
            crate::effects::execute_effect(
                &mut game,
                &Effect::create_tokens(unrelated_definition, 1),
                &mut ctx,
            )
            .unwrap();
        }
        let unrelated = *game.battlefield.last().unwrap();
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Colorless, 4);
        let action=crate::decision::compute_legal_actions(&game,alice).into_iter().find(|action| matches!(action,crate::decision::LegalAction::ActivateAbility {source:id,..} if *id==source)).expect("legal activation");
        let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
        let mut queue = crate::triggers::TriggerQueue::new();
        let mut dm = CopyChoice {
            alice,
            carol,
            target,
            other,
            protected,
            chose_player: false,
            chose_target: false,
        };
        let mut progress = crate::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &crate::game_loop::PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..12 {
            if !game.stack.is_empty() {
                break;
            }
            let crate::decision::GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("activation stalled: {progress:?}")
            };
            progress = crate::game_loop::apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &ctx, &mut dm,
            )
            .unwrap();
        }
        assert!(dm.chose_player && dm.chose_target);
        assert_eq!(game.stack.len(), 1);
        assert!(game.is_tapped(source));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert!(
            game.stack[0]
                .targets
                .contains(&crate::game_state::Target::Object(target))
        );
        if change == "leave" {
            game.move_object_by_effect(target, Zone::Graveyard);
        }
        if change == "control" {
            game.set_current_controller(target, bob);
        }
        let before = game.battlefield.clone();
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        let created = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| !before.contains(id))
            .collect::<Vec<_>>();
        assert_eq!(created.len(), usize::from(!remove_target));
        if remove_target {
            continue;
        }
        let token = created[0];
        assert_eq!(game.controller_of_id(token), Some(alice));
        assert_eq!(game.current_power(token), Some(3));
        assert_eq!(game.current_toughness(token), Some(4));
        assert!(
            game.current_has_static_ability_id(
                token,
                crate::static_abilities::StaticAbilityId::Haste
            )
        );
        assert!(!game.current_has_static_ability_id(
            target,
            crate::static_abilities::StaticAbilityId::Haste
        ));
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::BeginningOfEndStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let triggers = crate::triggers::check_delayed_triggers(&mut game, &event);
        assert_eq!(triggers.len(), 1);
        let mut queue = crate::triggers::TriggerQueue::new();
        for trigger in triggers {
            queue.add(trigger);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(!game.battlefield.contains(&token));
        assert!(game.battlefield.contains(&target));
        assert!(game.battlefield.contains(&other));
        assert!(game.battlefield.contains(&unrelated));
    }
}
#[test]
fn opponent_target_copy_preserves_target_and_token_references() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Echo Chamber")
        .card_types(vec![CardType::Artifact])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

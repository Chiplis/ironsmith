#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::*;

#[test]
pub(super) fn test_gift_optional_cost_choice_refreshes_pending_target_prompt() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell = CardBuilder::new(CardId::from_raw(9110), "Gift Target Pending")
        .card_types(vec![CardType::Instant])
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);

    let artifact = CardBuilder::new(CardId::from_raw(9111), "Opponent Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact_id = game.create_object_from_card(&artifact, bob, Zone::Battlefield);

    let default_target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
        crate::target::ObjectFilter::creature().opponent_controls(),
    ));
    let gift_target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
        crate::target::ObjectFilter::nonland_permanent().opponent_controls(),
    ));
    let program =
        crate::resolution::ResolutionProgram::new(vec![crate::resolution::ResolutionSegment {
            default_effects: vec![Effect::move_to_zone(default_target, Zone::Hand, false)],
            self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                crate::effect::Condition::ThisSpellPaidLabel("Gift".into()),
                vec![Effect::move_to_zone(gift_target, Zone::Hand, false)],
            )],
            starts_new_source_line: false,
        }]);

    if let Some(obj) = game.object_mut(spell_id) {
        obj.spell_effect = Some(program.into());
        obj.optional_costs = vec![crate::cost::OptionalCost::custom(
            "Gift a tapped Fish",
            crate::cost::TotalCost::free(),
        )]
        .into();
        obj.optional_costs_paid = crate::cost::OptionalCostsPaid::from_costs(&obj.optional_costs);
    }

    let pre_gift_requirements = game
        .object(spell_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .map(|program| {
            extract_target_requirements_from_program_with_modes(
                &game,
                program,
                alice,
                Some(spell_id),
                None,
            )
        })
        .expect("test spell should have a resolution program");
    assert!(
        pre_gift_requirements.is_empty(),
        "pre-gift branch should have no legal targets in this setup"
    );

    let optional_costs_paid = game
        .object(spell_id)
        .map(|obj| crate::cost::OptionalCostsPaid::from_costs(&obj.optional_costs))
        .unwrap_or_default();
    state.pending_cast = Some(PendingCast::new(
        spell_id,
        Zone::Stack,
        alice,
        crate::provenance::ProvNodeId::default(),
        CastStage::ChoosingOptionalCosts,
        None,
        pre_gift_requirements,
        CastingMethod::Normal,
        optional_costs_paid,
        None,
        spell_id,
    ));

    let mut dm = AutoPassDecisionMaker;
    let progress = apply_optional_costs_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &[(0, 1)],
        &mut dm,
    )
    .expect("promising the Gift should update the target prompt");

    match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(
            ctx,
        )) => {
            assert_eq!(
                ctx.requirements.len(),
                1,
                "expected one target prompt after Gift"
            );
            assert_eq!(
                ctx.requirements[0].legal_targets,
                vec![Target::Object(artifact_id)],
                "expected the target prompt to refresh to the gifted nonland permanent branch"
            );
        }
        other => panic!("expected Targets context after Gift choice, got {other:?}"),
    }

    let pending = state
        .pending_cast
        .as_ref()
        .expect("pending cast should continue to target selection");
    assert_eq!(pending.stage, CastStage::ChoosingTargets);
    assert_eq!(pending.remaining_requirements.len(), 1);
    assert_eq!(
        pending.remaining_requirements[0].legal_targets,
        vec![Target::Object(artifact_id)],
        "expected cached pending requirements to be refreshed after choosing Gift"
    );
}

#[test]
pub(super) fn gift_player_choice_cost_waits_for_choice_before_being_paid() {
    #[derive(Default)]
    struct PromptOnlyDecisionMaker {
        prompted: bool,
    }

    impl DecisionMaker for PromptOnlyDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.prompted
        }

        fn decide_options(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.prompted = true;
            Vec::new()
        }
    }

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let alice = PlayerId::from_index(0);

    let spell = CardBuilder::new(CardId::from_raw(9120), "Gift Choice Pending")
        .card_types(vec![CardType::Instant])
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);
    let gift_player_cost = crate::costs::Cost::effect(
        crate::effects::ChoosePlayerEffect::new(
            PlayerFilter::You,
            PlayerFilter::Opponent,
            "gifted_player",
        )
        .remember_as_chosen_player(),
    );

    let mut pending = PendingCast::new(
        spell_id,
        Zone::Hand,
        alice,
        crate::provenance::ProvNodeId::default(),
        CastStage::ProcessingCosts,
        None,
        Vec::new(),
        CastingMethod::Normal,
        crate::cost::OptionalCostsPaid::default(),
        None,
        spell_id,
    );
    pending.remaining_cost_steps = vec![super::priority_state::ActivationCostStep::Cost(
        gift_player_cost,
    )];

    let mut dm = PromptOnlyDecisionMaker::default();
    let progress = super::priority_cast::continue_spell_cost_payment(
        &mut game,
        &mut trigger_queue,
        &mut state,
        pending,
        &mut dm,
    )
    .expect("prompting for Gift's opponent should pause cost payment");

    assert!(matches!(progress, GameProgress::Continue));
    assert!(
        dm.prompted,
        "Gift cost should ask for the promised opponent"
    );
    let pending = state
        .pending_cast
        .as_ref()
        .expect("cast should remain pending while opponent choice is unresolved");
    assert_eq!(
        pending.remaining_cost_steps.len(),
        1,
        "Gift opponent choice cost must not be removed before the choice is answered"
    );
    assert_eq!(
        game.chosen_player(spell_id),
        None,
        "prompt fallback must not commit a promised opponent"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_overload_cast_swaps_in_rewritten_effects_and_hits_all_matches() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let spell_def = CardDefinitionBuilder::new(CardId::new(), "Overload Runtime Probe")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![
            vec![crate::mana::ManaSymbol::Generic(1)],
            vec![crate::mana::ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Return target nonland permanent you don't control to its owner's hand.\nOverload {0} (You may cast this spell for its overload cost. If you do, change \"target\" in its text to \"each.\")",
        )
        .expect("overload runtime probe should parse");

    let spell_id = game.create_object_from_definition(&spell_def, alice, Zone::Hand);
    let _bounced_one = create_creature(&mut game, "Opposing One", bob, 2, 2);
    let _bounced_two = create_creature(&mut game, "Opposing Two", bob, 3, 3);
    let survivor = create_creature(&mut game, "Friendly Survivor", alice, 1, 1);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: found,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(0),
            } if *found == spell_id
        )),
        "overload cast should be legal from hand"
    );

    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Alternative(0),
    )
    .expect("overload cast should move to stack");
    let stack_obj = game
        .object(stack_id)
        .expect("overloaded spell should exist");
    let requirements = super::targeting::extract_target_requirements(
        &game,
        stack_obj
            .spell_effect
            .as_deref()
            .map_or(&[][..], |program| program.flattened_default_effects()),
        alice,
        Some(stack_id),
    );
    assert!(
        requirements.is_empty(),
        "overloaded spell should not require target selection after rewrite"
    );

    game.stack
        .push(StackEntry::new(stack_id, alice).with_casting_method(CastingMethod::Alternative(0)));
    resolve_stack_entry(&mut game).expect("overloaded spell should resolve");

    assert!(
        game.player(bob)
            .expect("bob exists")
            .hand
            .iter()
            .filter_map(|&id| game.object(id))
            .any(|obj| obj.name == "Opposing One"),
        "first opposing permanent should return to hand"
    );
    assert!(
        game.player(bob)
            .expect("bob exists")
            .hand
            .iter()
            .filter_map(|&id| game.object(id))
            .any(|obj| obj.name == "Opposing Two"),
        "second opposing permanent should return to hand"
    );
    assert!(
        game.object(survivor)
            .is_some_and(|obj| obj.zone == Zone::Battlefield),
        "friendly permanent should not be affected by overloaded spell"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cleave_swaps_text_before_target_requirements_are_derived() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let spell_def = CardDefinitionBuilder::new(CardId::new(), "Cleave Runtime Probe")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Blue,
        ]]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Cleave {0}\nReturn target nonland permanent [you control] to its owner's hand.",
        )
        .expect("Cleave runtime probe should parse");

    let spell_id = game.create_object_from_definition(&spell_def, alice, Zone::Hand);
    let spell_stable_id = game.object(spell_id).expect("spell exists").stable_id;
    let friendly = create_creature(&mut game, "Friendly Permanent", alice, 2, 2);
    let opposing = create_creature(&mut game, "Opposing Permanent", bob, 3, 3);
    let remaining_opposing = create_creature(&mut game, "Remaining Opposing", bob, 4, 4);

    let normal_requirements = super::targeting::extract_target_requirements(
        &game,
        game.object(spell_id)
            .and_then(|spell| spell.spell_effect.as_deref())
            .map_or(&[][..], |program| program.flattened_default_effects()),
        alice,
        Some(spell_id),
    );
    assert_eq!(normal_requirements.len(), 1);
    assert!(
        normal_requirements[0]
            .legal_targets
            .contains(&Target::Object(friendly))
    );
    assert!(
        !normal_requirements[0]
            .legal_targets
            .contains(&Target::Object(opposing)),
        "the bracketed controller restriction applies to a normal cast"
    );

    assert!(
        crate::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: found,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Alternative(0),
                } if *found == spell_id
            )),
        "the Cleave alternative cost should be available from hand"
    );

    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Alternative(0),
    )
    .expect("Cleave cast should move to the stack");
    let cleave_requirements = super::targeting::extract_target_requirements(
        &game,
        game.object(stack_id)
            .and_then(|spell| spell.spell_effect.as_deref())
            .map_or(&[][..], |program| program.flattened_default_effects()),
        alice,
        Some(stack_id),
    );
    assert_eq!(cleave_requirements.len(), 1);
    assert!(
        cleave_requirements[0]
            .legal_targets
            .contains(&Target::Object(opposing)),
        "Cleave must remove the controller restriction before targets are chosen"
    );

    let mut entry =
        StackEntry::new(stack_id, alice).with_casting_method(CastingMethod::Alternative(0));
    entry.targets = vec![Target::Object(opposing)];
    game.stack.push(entry);
    resolve_stack_entry(&mut game).expect("cleaved spell should resolve");
    assert!(
        game.player(bob)
            .expect("bob exists")
            .hand
            .iter()
            .any(|&id| {
                game.object(id)
                    .is_some_and(|object| object.name == "Opposing Permanent")
            })
    );

    let graveyard_spell = game
        .find_object_by_stable_id(spell_stable_id)
        .expect("the resolved instant should remain tracked");
    assert_eq!(
        game.object(graveyard_spell)
            .expect("resolved spell exists")
            .zone,
        Zone::Graveyard
    );
    let restored_requirements = super::targeting::extract_target_requirements(
        &game,
        game.object(graveyard_spell)
            .and_then(|spell| spell.spell_effect.as_deref())
            .map_or(&[][..], |program| program.flattened_default_effects()),
        alice,
        Some(graveyard_spell),
    );
    assert_eq!(restored_requirements.len(), 1);
    assert!(
        !restored_requirements[0]
            .legal_targets
            .contains(&Target::Object(remaining_opposing)),
        "the bracketed normal-cast restriction must return after the spell leaves the stack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn spree_requires_a_payable_mode_and_charges_every_selected_mode() {
    struct ChooseBothModes;

    impl DecisionMaker for ChooseBothModes {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx.description.starts_with("Choose mode for") {
                return vec![0, 1];
            }
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let definition = CardDefinitionBuilder::new(CardId::new(), "Spree Runtime Probe")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Red]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Spree (Choose one or more additional costs.)\n\
             + {1} — You gain 1 life.\n\
             + {2} — You gain 2 life.",
        )
        .expect("Spree runtime probe should parse");
    let spell_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: found,
                    casting_method: CastingMethod::Normal,
                    ..
                } if *found == spell_id
            )),
        "the printed mana cost alone cannot begin a Spree cast"
    );

    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 3);
    assert!(
        crate::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: found,
                    casting_method: CastingMethod::Normal,
                    ..
                } if *found == spell_id
            )),
        "a payable Spree mode should expose the cast action"
    );

    let life_before = game.player(alice).expect("Alice exists").life;
    let mut decision_maker = ChooseBothModes;
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut decision_maker,
    )
    .expect("Spree proposal should run")
    .expect("paying the base and both mode costs should commit");

    let entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == stack_id)
        .expect("Spree spell should be on the stack");
    assert_eq!(entry.chosen_modes.as_deref(), Some(&[0, 1][..]));
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0,
        "the cast should pay {{R}} + {{1}} + {{2}}"
    );

    resolve_stack_entry(&mut game).expect("Spree spell should resolve");
    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        life_before + 3
    );
}

#[test]
pub(super) fn assist_gives_the_chosen_player_the_first_mana_window_and_only_pays_generic() {
    struct AssistDecisionMaker {
        bob: PlayerId,
        alice: PlayerId,
        mana_window_players: Vec<PlayerId>,
    }

    impl DecisionMaker for AssistDecisionMaker {
        fn decide_number(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::NumberContext,
        ) -> u32 {
            2
        }

        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            ctx.requirements
                .iter()
                .filter_map(|requirement| requirement.legal_targets.first().copied())
                .collect()
        }

        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx
                .description
                .starts_with("Choose another player to assist")
            {
                return vec![1];
            }
            if ctx
                .description
                .starts_with("Activate mana abilities before paying costs")
            {
                self.mana_window_players.push(ctx.player);
                if ctx.player == self.bob || ctx.player == self.alice {
                    return vec![0];
                }
            }
            if ctx.description.starts_with("Choose how much generic mana") {
                return vec![2];
            }
            ctx.options
                .iter()
                .find(|option| option.legal)
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }
    }

    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let assist_spell = CardDefinitionBuilder::new(CardId::new(), "Assist Runtime Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("Assist\nDestroy target creature with power X or less.")
        .expect("Assist X target probe should parse");
    let spell_id = game.create_object_from_definition(&assist_spell, alice, Zone::Hand);
    let _target = create_creature(&mut game, "Power-Two Target", charlie, 2, 2);

    let alice_land = CardDefinitionBuilder::new(CardId::new(), "Alice Blue Source")
        .card_types(vec![CardType::Land])
        .with_ability(Ability::mana(
            crate::cost::TotalCost::from_costs(vec![crate::costs::Cost::tap()]),
            vec![ManaSymbol::Blue],
        ))
        .build();
    let alice_land = game.create_object_from_definition(&alice_land, alice, Zone::Battlefield);
    let bob_land = CardDefinitionBuilder::new(CardId::new(), "Bob Double Source")
        .card_types(vec![CardType::Land])
        .with_ability(Ability::mana(
            crate::cost::TotalCost::from_costs(vec![crate::costs::Cost::tap()]),
            vec![ManaSymbol::Colorless, ManaSymbol::Colorless],
        ))
        .build();
    let bob_land = game.create_object_from_definition(&bob_land, bob, Zone::Battlefield);
    game.player_mut(charlie)
        .expect("Charlie exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    assert!(
        crate::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: found,
                    casting_method: CastingMethod::Normal,
                    ..
                } if *found == spell_id
            )),
        "Assist should make the X=2 targetable cast legal when one other player can cover X"
    );

    let mut decision_maker = AssistDecisionMaker {
        bob,
        alice,
        mana_window_players: Vec::new(),
    };
    let stack_id = super::cast_spell_from_resolving_effect(
        &mut game,
        spell_id,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut decision_maker,
    )
    .expect("Assist proposal should run")
    .expect("the chosen player and caster should jointly pay the spell");

    assert!(game.stack.iter().any(|entry| entry.object_id == stack_id));
    assert!(game.is_tapped(bob_land), "Bob should activate mana first");
    assert!(
        game.is_tapped(alice_land),
        "Alice should receive the caster mana window after Bob"
    );
    assert_eq!(decision_maker.mana_window_players, vec![bob, alice]);
    assert_eq!(
        game.player(bob).expect("Bob exists").mana_pool.total(),
        0,
        "Bob should spend exactly the two generic mana he announced"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0,
        "Alice should pay the blue component"
    );
    assert_eq!(
        game.player(charlie)
            .expect("Charlie exists")
            .mana_pool
            .total(),
        2,
        "an unchosen player cannot contribute mana"
    );
    let spent = &game
        .object(stack_id)
        .expect("Assist spell remains on the stack")
        .mana_spent_to_cast;
    assert_eq!(spent.blue, 1);
    assert_eq!(spent.colorless, 2);
    let turn_spending = &game
        .turn_store
        .turn_history
        .mana_spent_to_cast_spells_this_turn;
    assert_eq!(turn_spending.get(&alice), Some(&1));
    assert_eq!(turn_spending.get(&bob), Some(&2));
    assert_eq!(turn_spending.get(&charlie), None);
}

#[test]
pub(super) fn assist_cannot_cover_colored_mana_and_the_chosen_player_may_pay_zero() {
    struct DeclineAssist;

    impl DecisionMaker for DeclineAssist {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx
                .description
                .starts_with("Choose another player to assist")
            {
                return vec![1];
            }
            if ctx.description.starts_with("Choose how much generic mana") {
                return vec![0];
            }
            ctx.options
                .iter()
                .find(|option| option.legal)
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }
    }

    let mut colored_game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    colored_game.turn.phase = Phase::FirstMain;
    colored_game.turn.step = None;
    colored_game.turn.active_player = alice;
    colored_game.turn.priority_player = Some(alice);
    let colored_spell = CardDefinitionBuilder::new(CardId::new(), "Colored Assist Probe")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
        .card_types(vec![CardType::Sorcery])
        .with_ability(Ability::static_ability(StaticAbility::assist()))
        .build();
    let colored_spell =
        colored_game.create_object_from_definition(&colored_spell, alice, Zone::Hand);
    colored_game
        .player_mut(bob)
        .expect("Bob exists")
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    assert!(
        !crate::decision::compute_legal_actions(&colored_game, alice)
            .iter()
            .any(|action| matches!(
                action,
                LegalAction::CastSpell { spell_id, .. } if *spell_id == colored_spell
            )),
        "Assist cannot let another player pay a colored mana symbol"
    );

    let mut decline_game = setup_three_player_game();
    decline_game.turn.phase = Phase::FirstMain;
    decline_game.turn.step = None;
    decline_game.turn.active_player = alice;
    decline_game.turn.priority_player = Some(alice);
    let generic_spell = CardDefinitionBuilder::new(CardId::new(), "Declined Assist Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .with_ability(Ability::static_ability(StaticAbility::assist()))
        .build();
    let generic_spell =
        decline_game.create_object_from_definition(&generic_spell, alice, Zone::Hand);
    decline_game
        .player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    decline_game
        .player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    decline_game
        .player_mut(bob)
        .expect("Bob exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    let mut decision_maker = DeclineAssist;
    super::cast_spell_from_resolving_effect(
        &mut decline_game,
        generic_spell,
        Zone::Hand,
        alice,
        &CastingMethod::Normal,
        false,
        None,
        crate::provenance::ProvNodeId::default(),
        &mut decision_maker,
    )
    .expect("declined Assist proposal should run")
    .expect("the caster should pay after the chosen player contributes zero");
    assert_eq!(
        decline_game
            .player(bob)
            .expect("Bob exists")
            .mana_pool
            .total(),
        2,
        "the chosen player may decline to contribute mana"
    );
    assert_eq!(
        decline_game
            .player(alice)
            .expect("Alice exists")
            .mana_pool
            .total(),
        0
    );
}

pub(super) fn test_adventure_pair_definitions()
-> (crate::cards::CardDefinition, crate::cards::CardDefinition) {
    let front_id = CardId::from_raw(88_100);
    let adventure_id = CardId::from_raw(88_101);

    let front = CardDefinitionBuilder::new(front_id, "Curious Pair")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Peasant])
        .power_toughness(PowerToughness::fixed(1, 3))
        .other_face(adventure_id)
        .other_face_name("Treats to Share")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    let adventure = CardDefinitionBuilder::new(adventure_id, "Treats to Share")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![Subtype::Adventure])
        .other_face(front_id)
        .other_face_name("Curious Pair")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();

    (front, adventure)
}

pub(super) fn test_land_adventure_pair_definitions()
-> (crate::cards::CardDefinition, crate::cards::CardDefinition) {
    let front_id = CardId::from_raw(9_888_110);
    let adventure_id = CardId::from_raw(9_888_111);

    let front = CardDefinitionBuilder::new(front_id, "Test Adventure Land")
        .card_types(vec![CardType::Land])
        .other_face(adventure_id)
        .other_face_name("Test Land Adventure")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    let adventure = CardDefinitionBuilder::new(adventure_id, "Test Land Adventure")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![Subtype::Adventure])
        .other_face(front_id)
        .other_face_name("Test Adventure Land")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();

    (front, adventure)
}

pub(super) fn test_spell_land_pair_definitions()
-> (crate::cards::CardDefinition, crate::cards::CardDefinition) {
    let spell_id = CardId::from_raw(9_888_210);
    let land_id = CardId::from_raw(9_888_211);

    let spell = CardDefinitionBuilder::new(spell_id, "Test Front Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Sorcery])
        .other_face(land_id)
        .other_face_name("Test Back Land")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    let land = CardDefinitionBuilder::new(land_id, "Test Back Land")
        .card_types(vec![CardType::Land])
        .other_face(spell_id)
        .other_face_name("Test Front Spell")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();

    (spell, land)
}

pub(super) fn register_test_adventure_pair(game: &mut GameState) -> crate::cards::CardDefinition {
    let (front, adventure) = test_adventure_pair_definitions();
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&adventure);
    front
}

pub(super) fn register_test_land_adventure_pair(
    game: &mut GameState,
) -> crate::cards::CardDefinition {
    let (front, adventure) = test_land_adventure_pair_definitions();
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&adventure);
    front
}

pub(super) fn register_test_spell_land_pair(game: &mut GameState) -> crate::cards::CardDefinition {
    let (spell, land) = test_spell_land_pair_definitions();
    game.register_linked_face_definition(&spell);
    game.register_linked_face_definition(&land);
    spell
}

pub(super) fn register_costed_test_adventure_pair(
    game: &mut GameState,
    front_raw_id: u32,
    adventure_raw_id: u32,
    front_name: &str,
    adventure_name: &str,
    front_mana_value: u8,
    adventure_mana_value: u8,
) -> crate::cards::CardDefinition {
    let front_id = CardId::from_raw(front_raw_id);
    let adventure_id = CardId::from_raw(adventure_raw_id);

    let front = CardDefinitionBuilder::new(front_id, front_name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            front_mana_value,
        )]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human])
        .power_toughness(PowerToughness::fixed(2, 2))
        .other_face(adventure_id)
        .other_face_name(adventure_name)
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    let adventure = CardDefinitionBuilder::new(adventure_id, adventure_name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            adventure_mana_value,
        )]]))
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![Subtype::Adventure])
        .other_face(front_id)
        .other_face_name(front_name)
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();

    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&adventure);
    front
}

pub(super) fn add_test_play_from_grant_source(
    game: &mut GameState,
    player: PlayerId,
    filter: crate::filter::ObjectFilter,
    zone: Zone,
) -> ObjectId {
    let source = CardBuilder::new(CardId::new(), "Play-From Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source_id = game.create_object_from_card(&source, player, Zone::Battlefield);
    let grant = crate::grant::GrantSpec::new(crate::grant::Grantable::play_from(), filter, zone);
    game.object_mut(source_id)
        .expect("grant source should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::grants(grant)));
    source_id
}

#[test]
pub(super) fn test_spell_land_linked_card_offers_cast_and_land_play_actions() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let front = register_test_spell_land_pair(&mut game);
    game.create_object_from_definition(&crate::cards::basic_forest(), alice, Zone::Battlefield);
    let card_id = game.create_object_from_definition(&front, alice, Zone::Hand);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == card_id
            )
        }),
        "front-face spell should be castable from hand; got {actions:?}"
    );
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::PlayLand { land_id } if *land_id == card_id
            )
        }),
        "linked land face should be playable from hand; got {actions:?}"
    );
    assert_eq!(
        crate::decision::format_action_short(&game, &LegalAction::PlayLand { land_id: card_id },),
        "Play Test Back Land"
    );
}

#[test]
pub(super) fn test_spell_land_linked_card_enters_as_land_face_when_played() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let front = register_test_spell_land_pair(&mut game);
    let card_id = game.create_object_from_definition(&front, alice, Zone::Hand);
    let mut dm = SelectFirstDecisionMaker;
    crate::special_actions::perform(
        crate::special_actions::SpecialAction::PlayLand { card_id },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("linked land face should be playable from hand");

    let battlefield_id = game
        .battlefield
        .iter()
        .copied()
        .find(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Test Back Land")
        })
        .expect("linked land face should enter the battlefield");
    let land = game
        .object(battlefield_id)
        .expect("land object should exist");
    assert!(land.card_types.contains(&CardType::Land));
    assert_eq!(
        game.player(alice)
            .expect("player should exist")
            .lands_played_this_turn,
        1
    );
}

#[test]
pub(super) fn test_land_adventure_half_is_legal_hand_cast_action() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let front = register_test_land_adventure_pair(&mut game);
    game.create_object_from_definition(&crate::cards::basic_forest(), alice, Zone::Battlefield);
    let card_id = game.create_object_from_definition(&front, alice, Zone::Hand);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::PlayLand { land_id } if *land_id == card_id
            )
        }),
        "front-face land should still be playable as a land; got {actions:?}"
    );
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::SplitOtherHalf,
                } if *spell_id == card_id
            )
        }),
        "front-face land should offer its Adventure half from hand; got {actions:?}"
    );
}

#[test]
pub(super) fn test_linked_front_land_stays_front_face_when_played() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let front = register_test_land_adventure_pair(&mut game);
    let card_id = game.create_object_from_definition(&front, alice, Zone::Hand);
    let mut dm = SelectFirstDecisionMaker;
    crate::special_actions::perform(
        crate::special_actions::SpecialAction::PlayLand { card_id },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("front-face Adventure land should be playable");

    let battlefield_id = game
        .battlefield
        .iter()
        .copied()
        .find(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Test Adventure Land")
        })
        .expect("front-face land should remain on the battlefield");
    let land = game
        .object(battlefield_id)
        .expect("land object should exist");
    assert!(land.card_types.contains(&CardType::Land));
    assert!(
        !land.subtypes.contains(&Subtype::Adventure),
        "land play should not rewrite the permanent into the Adventure face"
    );
}

#[test]
pub(super) fn test_adventure_exiled_land_can_be_played_from_exile() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let front = register_test_land_adventure_pair(&mut game);
    let exiled_id = game.create_object_from_definition(&front, alice, Zone::Exile);
    game.set_adventure_exiled(exiled_id);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::PlayLand { land_id } if *land_id == exiled_id
            )
        }),
        "Adventure-exiled front-face land should be playable from exile; got {actions:?}"
    );

    let mut dm = SelectFirstDecisionMaker;
    crate::special_actions::perform(
        crate::special_actions::SpecialAction::PlayLand { card_id: exiled_id },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("Adventure-exiled land should be playable from exile");

    let battlefield_id = game
        .battlefield
        .iter()
        .copied()
        .find(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Test Adventure Land")
        })
        .expect("exiled Adventure land should enter as the front face");
    assert!(
        !game.is_adventure_exiled(battlefield_id),
        "Adventure exile marker should not persist onto the new battlefield object"
    );
}

#[test]
pub(super) fn test_adventure_half_is_legal_hand_cast_action() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let front = register_test_adventure_pair(&mut game);
    game.create_object_from_definition(&crate::cards::basic_forest(), alice, Zone::Battlefield);
    let card_id = game.create_object_from_definition(&front, alice, Zone::Hand);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::SplitOtherHalf,
                } if *spell_id == card_id
            )
        }),
        "Adventure card should offer its Adventure half from hand; got {actions:?}"
    );
}

#[test]
pub(super) fn test_adventure_stack_spell_uses_adventure_face_mana_cost() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let front = register_test_adventure_pair(&mut game);
    let card_id = game.create_object_from_definition(&front, alice, Zone::Hand);
    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        card_id,
        Zone::Hand,
        alice,
        &CastingMethod::SplitOtherHalf,
    )
    .expect("Adventure half should move to the stack");
    let stack_obj = game
        .object(stack_id)
        .expect("adventure spell should exist on stack");

    let cost = crate::decision::spell_mana_cost_for_cast(
        &game,
        alice,
        stack_obj,
        &CastingMethod::SplitOtherHalf,
        Zone::Hand,
    )
    .expect("Adventure stack spell should have a mana cost");
    assert_eq!(
        cost,
        ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
        "stack-time Adventure cost should use the Adventure face, not the creature face"
    );
}

pub(super) fn hand_free_cast_mana_value_three_or_less_effect() -> Effect {
    Effect::may_cast_matching_spell_without_paying_mana_cost(
        PlayerFilter::You,
        crate::target::ObjectFilter::nonland()
            .with_mana_value(crate::filter::Comparison::LessThanOrEqual(3)),
        Zone::Hand,
    )
}

pub(super) fn exiled_free_cast_mana_value_less_than_four_effect() -> Effect {
    Effect::may_cast_matching_spell_without_paying_mana_cost(
        PlayerFilter::You,
        crate::target::ObjectFilter::nonland()
            .with_mana_value(crate::filter::Comparison::LessThan(4)),
        Zone::Exile,
    )
}

pub(super) struct ChooseFreeCastOptionByLabel {
    pub(super) needle: &'static str,
}

impl DecisionMaker for ChooseFreeCastOptionByLabel {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        ctx.options
            .iter()
            .find(|option| option.legal && option.description.contains(self.needle))
            .map(|option| vec![option.index])
            .unwrap_or_else(|| {
                ctx.options
                    .iter()
                    .filter(|option| option.legal)
                    .map(|option| option.index)
                    .take(ctx.min)
                    .collect()
            })
    }
}

#[test]
pub(super) fn test_effect_driven_free_cast_can_choose_adventure_half_from_hand() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source = CardBuilder::new(CardId::from_raw(88_120), "Free-Cast Source")
        .card_types(vec![CardType::Sorcery])
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Stack);
    let front = register_test_adventure_pair(&mut game);
    let card_id = game.create_object_from_definition(&front, alice, Zone::Hand);

    let effect = hand_free_cast_mana_value_three_or_less_effect();
    let mut dm = ChooseFreeCastOptionByLabel {
        needle: "Treats to Share",
    };
    let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
    execute_effect(&mut game, &effect, &mut ctx).expect("effect-driven free cast should resolve");

    let stack_entry = game
        .stack
        .last()
        .expect("free-cast spell should be stacked");
    assert_eq!(stack_entry.casting_method, CastingMethod::SplitOtherHalf);
    let stack_obj = game
        .object(stack_entry.object_id)
        .expect("Adventure half should exist on the stack");
    assert_eq!(stack_obj.name, "Treats to Share");
    assert!(
        game.object(card_id)
            .is_none_or(|object| object.zone != Zone::Hand),
        "physical Adventure card should leave hand while its Adventure half is on the stack"
    );
}

#[test]
pub(super) fn test_effect_driven_free_cast_can_choose_adventure_half_from_exile() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source = CardBuilder::new(CardId::from_raw(88_125), "Free-Cast Source")
        .card_types(vec![CardType::Sorcery])
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Stack);
    let front = register_test_adventure_pair(&mut game);
    let card_id = game.create_object_from_definition(&front, alice, Zone::Exile);

    let effect = exiled_free_cast_mana_value_less_than_four_effect();
    let mut dm = ChooseFreeCastOptionByLabel {
        needle: "Treats to Share",
    };
    let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
    execute_effect(&mut game, &effect, &mut ctx)
        .expect("effect-driven exiled free cast should resolve");

    let stack_entry = game
        .stack
        .last()
        .expect("free-cast spell should be stacked");
    assert_eq!(stack_entry.casting_method, CastingMethod::SplitOtherHalf);
    let stack_obj = game
        .object(stack_entry.object_id)
        .expect("Adventure half should exist on the stack");
    assert_eq!(stack_obj.name, "Treats to Share");
    assert!(
        game.object(card_id)
            .is_none_or(|object| object.zone != Zone::Exile),
        "physical Adventure card should leave exile while its Adventure half is on the stack"
    );
}

#[cfg(feature = "generated-registry")]
#[test]
pub(super) fn test_generated_adventure_free_cast_can_choose_adventure_half_from_exile() {
    let mut registry = crate::cards::CardRegistry::new();
    registry.ensure_cards_loaded(["Curious Pair"]);
    let front = registry
        .get("Curious Pair")
        .expect("generated Curious Pair should load");

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let source = CardBuilder::new(CardId::from_raw(88_126), "Free-Cast Source")
        .card_types(vec![CardType::Sorcery])
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Stack);
    let card_id = game.create_object_from_catalog_definition(front, &registry, alice, Zone::Exile);

    let effect = exiled_free_cast_mana_value_less_than_four_effect();
    let mut dm = ChooseFreeCastOptionByLabel {
        needle: "Treats to Share",
    };
    let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
    execute_effect(&mut game, &effect, &mut ctx)
        .expect("effect-driven generated exiled free cast should resolve");

    let stack_entry = game
        .stack
        .last()
        .expect("free-cast spell should be stacked");
    assert_eq!(stack_entry.casting_method, CastingMethod::SplitOtherHalf);
    let stack_obj = game
        .object(stack_entry.object_id)
        .expect("Adventure half should exist on the stack");
    assert_eq!(stack_obj.name, "Treats to Share");
    assert!(
        game.object(card_id)
            .is_none_or(|object| object.zone != Zone::Exile),
        "physical generated Adventure card should leave exile while its Adventure half is on the stack"
    );
}

#[cfg(feature = "generated-registry")]
#[test]
pub(super) fn test_generated_cascade_can_choose_adventure_half() {
    let mut registry = crate::cards::CardRegistry::new();
    registry.ensure_cards_loaded(["Curious Pair"]);
    let front = registry
        .get("Curious Pair")
        .expect("generated Curious Pair should load");

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let source = CardBuilder::new(CardId::from_raw(88_127), "Bloodbraid Elf")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Stack);
    game.create_object_from_catalog_definition(front, &registry, alice, Zone::Library);

    let effect = Effect::new(crate::effects::CascadeEffect::new());
    let mut dm = ChooseFreeCastOptionByLabel {
        needle: "Treats to Share",
    };
    let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
    execute_effect(&mut game, &effect, &mut ctx).expect("cascade effect should resolve");

    let stack_entry = game
        .stack
        .last()
        .expect("cascade-cast spell should be stacked");
    assert_eq!(stack_entry.casting_method, CastingMethod::SplitOtherHalf);
    let stack_obj = game
        .object(stack_entry.object_id)
        .expect("Adventure half should exist on the stack");
    assert_eq!(stack_obj.name, "Treats to Share");
}

#[test]
pub(super) fn test_cascade_land_drop_puts_exiled_land_onto_battlefield_before_cleanup() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let averna = CardDefinitionBuilder::new(CardId::from_raw(88_128), "Averna Probe")
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::static_ability(StaticAbility::cascade_land_drop()))
        .build();
    game.create_object_from_definition(&averna, alice, Zone::Battlefield);

    let source = CardBuilder::new(CardId::from_raw(88_129), "Cascade Source")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Stack);

    let hit = CardBuilder::new(CardId::from_raw(88_130), "Cascade Hit")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Sorcery])
        .build();
    game.create_object_from_card(&hit, alice, Zone::Library);

    let land = CardBuilder::new(CardId::from_raw(88_131), "Cascade Land")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land, alice, Zone::Library);
    let land_stable_id = game
        .object(land_id)
        .expect("cascade land should exist")
        .stable_id;

    let effect = Effect::new(crate::effects::CascadeEffect::new());
    let mut dm = ChooseFreeCastOptionByLabel {
        needle: "Cascade Hit",
    };
    let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
    execute_effect(&mut game, &effect, &mut ctx).expect("cascade effect should resolve");

    let current_land_id = game
        .find_object_by_stable_id(land_stable_id)
        .expect("cascade land should still be tracked");
    let current_land = game
        .object(current_land_id)
        .expect("cascade land should exist after cascade");
    assert_eq!(current_land.zone, Zone::Battlefield);
    assert_eq!(game.controller_of(current_land), alice);
    assert!(game.is_tapped(current_land_id));

    let stack_entry = game
        .stack
        .last()
        .expect("cascade-cast spell should be stacked");
    let stack_obj = game
        .object(stack_entry.object_id)
        .expect("cascade hit should exist on the stack");
    assert_eq!(stack_obj.name, "Cascade Hit");
}

#[cfg(feature = "generated-registry")]
#[test]
pub(super) fn test_generated_parent_name_adventure_card_resolves_adventure_half() {
    let mut registry = crate::cards::CardRegistry::new();
    registry.ensure_cards_loaded(["Curious Pair // Treats to Share"]);
    assert!(
        registry.get("Treats to Share").is_some(),
        "parent-name loading should also prime the Adventure face"
    );
    let front = registry
        .get("Curious Pair // Treats to Share")
        .or_else(|| registry.get("Curious Pair"))
        .expect("generated Curious Pair parent name should load");

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let card_id =
        game.create_object_from_catalog_definition(front, &registry, alice, Zone::Library);
    let card = game
        .object(card_id)
        .expect("generated Adventure card should exist");
    let adventure_view = crate::decision::spell_view_for_split_other_half_cast(&game, card)
        .expect("parent-loaded Adventure card should have a linked spell half");

    assert_eq!(adventure_view.name, "Treats to Share");
    assert!(adventure_view.subtypes.contains(&Subtype::Adventure));
}

#[test]
pub(super) fn test_effect_driven_free_cast_matches_adventure_half_when_front_face_is_too_expensive()
{
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source = CardBuilder::new(CardId::from_raw(88_130), "Free-Cast Source")
        .card_types(vec![CardType::Sorcery])
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Stack);
    let front = register_costed_test_adventure_pair(
        &mut game,
        88_131,
        88_132,
        "Tall Tale Knight",
        "Small Errand",
        5,
        3,
    );
    game.create_object_from_definition(&front, alice, Zone::Hand);

    let effect = hand_free_cast_mana_value_three_or_less_effect();
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
    execute_effect(&mut game, &effect, &mut ctx).expect("effect-driven free cast should resolve");

    let stack_entry = game
        .stack
        .last()
        .expect("free-cast spell should be stacked");
    assert_eq!(stack_entry.casting_method, CastingMethod::SplitOtherHalf);
    let stack_obj = game
        .object(stack_entry.object_id)
        .expect("Adventure half should exist on the stack");
    assert_eq!(stack_obj.name, "Small Errand");
}

#[test]
pub(super) fn test_effect_driven_free_cast_uses_front_face_when_adventure_half_is_too_expensive() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source = CardBuilder::new(CardId::from_raw(88_140), "Free-Cast Source")
        .card_types(vec![CardType::Sorcery])
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Stack);
    let front = register_costed_test_adventure_pair(
        &mut game,
        88_141,
        88_142,
        "Small Front",
        "Large Quest",
        3,
        7,
    );
    game.create_object_from_definition(&front, alice, Zone::Hand);

    let effect = hand_free_cast_mana_value_three_or_less_effect();
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
    execute_effect(&mut game, &effect, &mut ctx).expect("effect-driven free cast should resolve");

    let stack_entry = game
        .stack
        .last()
        .expect("free-cast spell should be stacked");
    assert_eq!(stack_entry.casting_method, CastingMethod::Normal);
    let stack_obj = game
        .object(stack_entry.object_id)
        .expect("front-face spell should exist on the stack");
    assert_eq!(stack_obj.name, "Small Front");
}

#[test]
pub(super) fn test_resolved_adventure_exiles_front_face_and_allows_creature_cast() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let front = register_test_adventure_pair(&mut game);
    for _ in 0..2 {
        game.create_object_from_definition(&crate::cards::basic_forest(), alice, Zone::Battlefield);
    }
    let card_id = game.create_object_from_definition(&front, alice, Zone::Hand);
    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        card_id,
        Zone::Hand,
        alice,
        &CastingMethod::SplitOtherHalf,
    )
    .expect("Adventure half should move to the stack");

    let stack_obj = game
        .object(stack_id)
        .expect("adventure spell should exist on stack");
    assert_eq!(stack_obj.name, "Treats to Share");
    assert!(stack_obj.subtypes.contains(&Subtype::Adventure));

    let mut dm = SelectFirstDecisionMaker;
    game.stack
        .push(StackEntry::new(stack_id, alice).with_casting_method(CastingMethod::SplitOtherHalf));
    resolve_stack_entry_with(&mut game, &mut dm).expect("Adventure spell should resolve");

    let exile_id = game
        .exile
        .iter()
        .copied()
        .find(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Curious Pair")
        })
        .expect("resolved Adventure should exile the front-face card");
    let exile_obj = game
        .object(exile_id)
        .expect("exiled adventure card should exist");
    assert!(game.is_adventure_exiled(exile_id));
    assert!(exile_obj.card_types.contains(&CardType::Creature));
    assert!(!exile_obj.subtypes.contains(&Subtype::Adventure));

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == exile_id
            )
        }),
        "Adventure-exiled card should offer the normal creature cast from exile; got {actions:?}"
    );
}

#[test]
pub(super) fn test_stable_exile_play_from_grant_survives_adventure_resolution() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let front = register_test_adventure_pair(&mut game);
    let source_id =
        game.create_object_from_definition(&crate::cards::basic_forest(), alice, Zone::Battlefield);
    for _ in 0..2 {
        game.create_object_from_definition(&crate::cards::basic_forest(), alice, Zone::Battlefield);
    }
    let exiled_id = game.create_object_from_definition(&front, bob, Zone::Exile);
    let stable_id = game
        .object(exiled_id)
        .expect("exiled Adventure card should exist")
        .stable_id;
    game.effect_store.grant_registry.grant_to_stable_card(
        exiled_id,
        stable_id,
        Zone::Exile,
        alice,
        crate::grant::Grantable::PlayFrom,
        crate::grant_registry::GrantSource::Effect {
            source_id,
            expires_end_of_turn: u32::MAX,
        },
    );

    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        exiled_id,
        Zone::Exile,
        alice,
        &CastingMethod::SplitOtherHalf,
    )
    .expect("granted exiled Adventure half should move to the stack");
    game.stack
        .push(StackEntry::new(stack_id, alice).with_casting_method(CastingMethod::SplitOtherHalf));

    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm).expect("Adventure spell should resolve");

    let new_exile_id = game
        .find_object_by_stable_id(stable_id)
        .expect("physical card should still be tracked by stable id");
    assert!(
        game.object(new_exile_id)
            .is_some_and(|object| object.zone == Zone::Exile && object.name == "Curious Pair"),
        "resolved Adventure should return the front face to exile"
    );

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    ..
                } if *spell_id == new_exile_id
            )
        }),
        "stable exiled-card grant should still allow casting the creature after Adventure resolution; got {actions:?}"
    );
}

#[test]
pub(super) fn test_countered_adventure_goes_to_graveyard_as_front_face() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let front = register_test_adventure_pair(&mut game);
    let card_id = game.create_object_from_definition(&front, alice, Zone::Hand);
    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        card_id,
        Zone::Hand,
        alice,
        &CastingMethod::SplitOtherHalf,
    )
    .expect("Adventure half should move to the stack");

    let graveyard_id = game
        .move_object_by_effect(stack_id, Zone::Graveyard)
        .expect("countered Adventure spell should move to graveyard");
    let graveyard_obj = game
        .object(graveyard_id)
        .expect("graveyard object should exist");

    assert_eq!(graveyard_obj.name, "Curious Pair");
    assert!(graveyard_obj.card_types.contains(&CardType::Creature));
    assert!(!graveyard_obj.subtypes.contains(&Subtype::Adventure));
}

#[test]
pub(super) fn test_library_play_from_grant_offers_top_creature_card() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let source_id = add_test_play_from_grant_source(
        &mut game,
        alice,
        crate::filter::ObjectFilter::default().with_type(CardType::Creature),
        Zone::Library,
    );
    let front = register_test_adventure_pair(&mut game);
    for _ in 0..2 {
        game.create_object_from_definition(&crate::cards::basic_forest(), alice, Zone::Battlefield);
    }
    let library_id = game.create_object_from_definition(&front, alice, Zone::Library);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Library,
                    casting_method: CastingMethod::PlayFrom {
                        source,
                        zone: Zone::Library,
                        use_alternative: None,
                    },
                } if *spell_id == library_id && *source == source_id
            )
        }),
        "top-library PlayFrom grant should offer the creature half; got {actions:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn thundermane_dragon_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Thundermane Dragon")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dragon])
        .power_toughness(PowerToughness::fixed(5, 5))
        .parse_text(
            "Flying\nYou may cast creature spells with power 4 or greater from the top of your library. If you cast a creature spell this way, it gains haste until end of turn.",
        )
        .expect("Thundermane Dragon should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn thundermane_dragon_casts_top_power_four_creature_and_grants_haste_until_eot() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let thundermane = thundermane_dragon_definition();
    let source_id = game.create_object_from_definition(&thundermane, alice, Zone::Battlefield);
    let top_creature = CardBuilder::new(CardId::new(), "Top Power Four")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let top_id = game.create_object_from_card(&top_creature, alice, Zone::Library);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    let casting_method = actions
        .iter()
        .find_map(|action| match action {
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Library,
                casting_method:
                    method @ CastingMethod::PlayFrom {
                        source,
                        zone: Zone::Library,
                        ..
                    },
            } if *spell_id == top_id && *source == source_id => Some(method.clone()),
            _ => None,
        })
        .expect("Thundermane Dragon should let Alice cast the top power-4 creature");

    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        top_id,
        Zone::Library,
        alice,
        &casting_method,
    )
    .expect("top creature should move to the stack");
    let battlefield_id = game
        .move_object_by_effect(stack_id, Zone::Battlefield)
        .expect("creature spell should resolve to the battlefield");
    game.refresh_continuous_state();

    assert!(
        game.object_has_static_ability_id(
            battlefield_id,
            crate::static_abilities::StaticAbilityId::Haste
        ),
        "creature cast from the top with Thundermane Dragon should gain haste"
    );

    crate::turn::execute_cleanup_step(&mut game);
    game.refresh_continuous_state();
    assert!(
        !game.object_has_static_ability_id(
            battlefield_id,
            crate::static_abilities::StaticAbilityId::Haste
        ),
        "Thundermane Dragon's haste grant should expire at end of turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn thundermane_dragon_does_not_cast_top_creature_below_power_four() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let thundermane = thundermane_dragon_definition();
    game.create_object_from_definition(&thundermane, alice, Zone::Battlefield);
    let small_creature = CardBuilder::new(CardId::new(), "Top Power Three")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let small_id = game.create_object_from_card(&small_creature, alice, Zone::Library);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Library,
                casting_method: CastingMethod::PlayFrom { .. },
            } if *spell_id == small_id
        )),
        "Thundermane Dragon should not let Alice cast a top creature with power below 4"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn cemetery_illuminator_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Cemetery Illuminator")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spirit])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(
            "Flying\nWhenever this creature enters or attacks, exile a card from a graveyard.\nYou may look at the top card of your library any time.\nOnce each turn, you may cast a spell from the top of your library if it shares a card type with a card exiled with this creature.",
        )
        .expect("Cemetery Illuminator should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_cemetery_illuminator_in_cast_window(
    game: &mut GameState,
    active_player: PlayerId,
    priority_player: PlayerId,
) {
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = active_player;
    game.turn.priority_player = Some(priority_player);
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn cemetery_illuminator_play_from_action(
    game: &GameState,
    player: PlayerId,
    spell_id: ObjectId,
    source_id: ObjectId,
) -> Option<CastingMethod> {
    crate::decision::compute_legal_actions(game, player)
        .into_iter()
        .find_map(|action| match action {
            LegalAction::CastSpell {
                spell_id: action_spell_id,
                from_zone: Zone::Library,
                casting_method:
                    method @ CastingMethod::PlayFrom {
                        source,
                        zone: Zone::Library,
                        ..
                    },
            } if action_spell_id == spell_id && source == source_id => Some(method),
            _ => None,
        })
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_zero_cost_card(
    game: &mut GameState,
    player: PlayerId,
    name: &str,
    card_types: Vec<CardType>,
    zone: Zone,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::new())
        .card_types(card_types)
        .build();
    game.create_object_from_card(&card, player, zone)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cemetery_illuminator_casts_top_spell_sharing_type_with_source_exiled_card() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    put_cemetery_illuminator_in_cast_window(&mut game, alice, alice);

    let source = cemetery_illuminator_definition();
    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);
    let exiled_instant = create_zero_cost_card(
        &mut game,
        alice,
        "Exiled Instant",
        vec![CardType::Instant],
        Zone::Exile,
    );
    game.add_exiled_with_source_link(source_id, exiled_instant);
    let top_instant = create_zero_cost_card(
        &mut game,
        alice,
        "Top Instant",
        vec![CardType::Instant],
        Zone::Library,
    );

    assert!(
        cemetery_illuminator_play_from_action(&game, alice, top_instant, source_id).is_some(),
        "Cemetery Illuminator should allow casting a top-library spell sharing a card type with a card exiled with it"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cemetery_illuminator_does_not_cast_top_spell_without_source_exiled_card() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    put_cemetery_illuminator_in_cast_window(&mut game, alice, alice);

    let source = cemetery_illuminator_definition();
    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);
    let top_instant = create_zero_cost_card(
        &mut game,
        alice,
        "Top Instant",
        vec![CardType::Instant],
        Zone::Library,
    );

    assert!(
        cemetery_illuminator_play_from_action(&game, alice, top_instant, source_id).is_none(),
        "Cemetery Illuminator should not allow top-library casting before a card is exiled with it"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cemetery_illuminator_does_not_cast_top_spell_with_nonmatching_source_exiled_type() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    put_cemetery_illuminator_in_cast_window(&mut game, alice, alice);

    let source = cemetery_illuminator_definition();
    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);
    let exiled_creature = create_zero_cost_card(
        &mut game,
        alice,
        "Exiled Creature",
        vec![CardType::Creature],
        Zone::Exile,
    );
    game.add_exiled_with_source_link(source_id, exiled_creature);
    let top_instant = create_zero_cost_card(
        &mut game,
        alice,
        "Top Instant",
        vec![CardType::Instant],
        Zone::Library,
    );

    assert!(
        cemetery_illuminator_play_from_action(&game, alice, top_instant, source_id).is_none(),
        "Cemetery Illuminator should require the top spell to share a card type with a source-exiled card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cemetery_illuminator_can_cast_matching_top_instant_on_opponents_turn() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    put_cemetery_illuminator_in_cast_window(&mut game, bob, alice);

    let source = cemetery_illuminator_definition();
    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);
    let exiled_instant = create_zero_cost_card(
        &mut game,
        alice,
        "Exiled Instant",
        vec![CardType::Instant],
        Zone::Exile,
    );
    game.add_exiled_with_source_link(source_id, exiled_instant);
    let top_instant = create_zero_cost_card(
        &mut game,
        alice,
        "Top Instant",
        vec![CardType::Instant],
        Zone::Library,
    );

    assert!(
        cemetery_illuminator_play_from_action(&game, alice, top_instant, source_id).is_some(),
        "'Once each turn' should allow a matching top instant on an opponent's turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cemetery_illuminator_top_library_cast_is_limited_to_once_each_turn() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    put_cemetery_illuminator_in_cast_window(&mut game, alice, alice);

    let source = cemetery_illuminator_definition();
    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);
    let exiled_instant = create_zero_cost_card(
        &mut game,
        alice,
        "Exiled Instant",
        vec![CardType::Instant],
        Zone::Exile,
    );
    game.add_exiled_with_source_link(source_id, exiled_instant);
    let first_top = create_zero_cost_card(
        &mut game,
        alice,
        "First Top Instant",
        vec![CardType::Instant],
        Zone::Library,
    );

    assert!(
        cemetery_illuminator_play_from_action(&game, alice, first_top, source_id).is_some(),
        "first matching top spell should be castable"
    );
    game.turn_store
        .grant_cast_uses_this_turn
        .insert((alice, source_id));

    let second_top = create_zero_cost_card(
        &mut game,
        alice,
        "Second Top Instant",
        vec![CardType::Instant],
        Zone::Library,
    );
    assert!(
        cemetery_illuminator_play_from_action(&game, alice, second_top, source_id).is_none(),
        "Cemetery Illuminator should not allow a second top-library cast from the same source in one turn"
    );
}

#[test]
pub(super) fn test_library_play_from_grant_offers_adventure_half_when_linked_face_matches() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mut spell_filter = crate::filter::ObjectFilter::default();
    spell_filter.any_of = vec![
        crate::filter::ObjectFilter::default().with_type(CardType::Instant),
        crate::filter::ObjectFilter::default().with_type(CardType::Sorcery),
    ];
    add_test_play_from_grant_source(&mut game, alice, spell_filter.clone(), Zone::Library);
    let front = register_test_adventure_pair(&mut game);
    game.create_object_from_definition(&crate::cards::basic_forest(), alice, Zone::Battlefield);
    let library_id = game.create_object_from_definition(&front, alice, Zone::Library);

    let view = crate::derived_view::DerivedGameView::new(&game);
    let card = game
        .object(library_id)
        .expect("library Adventure card should exist");
    let adventure_view = crate::decision::spell_view_for_split_other_half_cast(&game, card)
        .expect("Adventure card should have a linked spell half");
    assert!(
        crate::filter::ObjectFilter::default()
            .with_type(CardType::Sorcery)
            .matches_non_recursive(
                &adventure_view,
                &game.filter_context_for(alice, None),
                &game
            ),
        "direct sorcery filter should match the linked Adventure half"
    );
    assert!(
        spell_filter.matches_non_recursive(
            &adventure_view,
            &game.filter_context_for(alice, None),
            &game
        ),
        "instant/sorcery grant filter should match the linked Adventure half; view card types: {:?}, subtypes: {:?}",
        adventure_view.card_types,
        adventure_view.subtypes,
    );
    assert!(
        !view
            .granted_play_from_for_card_view(library_id, &adventure_view, Zone::Library, alice)
            .is_empty(),
        "top-library grant should apply to the linked Adventure half"
    );
    assert!(
        crate::decision::can_cast_spell_with_view(
            &game,
            alice,
            card,
            &CastingMethod::SplitOtherHalf,
            &view,
        ),
        "linked Adventure half should be castable from top library once a grant applies"
    );

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Library,
                    casting_method: CastingMethod::SplitOtherHalf,
                } if *spell_id == library_id
            )
        }),
        "top-library PlayFrom grant should offer the linked Adventure half; got {actions:?}"
    );
}

#[test]
pub(super) fn test_exile_play_from_grant_offers_adventure_half() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    add_test_play_from_grant_source(
        &mut game,
        alice,
        crate::filter::ObjectFilter::default(),
        Zone::Exile,
    );
    let front = register_test_adventure_pair(&mut game);
    game.create_object_from_definition(&crate::cards::basic_forest(), alice, Zone::Battlefield);
    let exiled_id = game.create_object_from_definition(&front, alice, Zone::Exile);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::SplitOtherHalf,
                } if *spell_id == exiled_id
            )
        }),
        "exile PlayFrom grant should offer the linked Adventure half; got {actions:?}"
    );
}

#[test]
pub(super) fn test_granted_enter_with_counters_applies_to_adventure_creature_entering() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let mut grant_filter = crate::filter::ObjectFilter::creature();
    grant_filter.controller = Some(PlayerFilter::You);
    let adventure_filter = crate::filter::ObjectFilter::default().with_subtype(Subtype::Adventure);
    let source = CardBuilder::new(CardId::new(), "Adventure Counter Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
    let granted = StaticAbility::enters_with_counters_for_filter(
        adventure_filter,
        crate::object::CounterType::PlusOnePlusOne,
        1,
    );
    game.object_mut(source_id)
        .expect("grant source should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::grant_ability(
            grant_filter,
            granted,
        )));

    let front = register_test_adventure_pair(&mut game);
    let card_id = game.create_object_from_definition(&front, alice, Zone::Hand);
    let entered = game
        .move_object_with_etb_processing(card_id, Zone::Battlefield)
        .expect("Adventure creature should enter")
        .new_id;

    assert_eq!(
        game.counter_count(entered, crate::object::CounterType::PlusOnePlusOne),
        1,
        "granted ETB replacement should see that the creature has an Adventure"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_split_card_cast_prompt_offers_front_back_and_fuse_methods() {
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 4);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Black, 2);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    let registry =
        crate::cards::CardRegistry::with_builtin_cards_for_names(["Breaking", "Grizzly Bears"]);
    let breaking = registry
        .get("Breaking")
        .expect("Breaking should be available in test registry");
    let grizzly = registry
        .get("Grizzly Bears")
        .expect("Grizzly Bears should exist in test registry");
    game.create_object_from_definition(grizzly, bob, Zone::Graveyard);
    let split_id = game.create_object_from_definition(breaking, alice, Zone::Hand);

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = AutoPassDecisionMaker;
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: split_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &cast_response,
        &mut dm,
    )
    .expect("split cast should start successfully");

    let ctx = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!(
            "expected casting-method selection prompt for split card, got {:?}",
            other
        ),
    };

    let descriptions: Vec<String> = ctx
        .options
        .iter()
        .map(|opt| opt.description.clone())
        .collect();
    assert_eq!(
        descriptions.len(),
        3,
        "split card should offer three cast methods"
    );
    assert!(
        descriptions
            .iter()
            .any(|desc| desc.contains("Breaking: {U}{B}")),
        "front half should be available, got {descriptions:?}"
    );
    assert!(
        descriptions
            .iter()
            .any(|desc| desc.contains("Entering: {4}{B}{R}")),
        "back half should be available, got {descriptions:?}"
    );
    assert!(
        descriptions
            .iter()
            .any(|desc| desc.contains("Fuse: {4}{U}{B}{B}{R}"))
            || descriptions
                .iter()
                .any(|desc| desc.contains("Fuse: {U}{B}{4}{B}{R}")),
        "fuse option should be available, got {descriptions:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_split_other_half_cast_uses_back_face_characteristics_on_stack() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let registry =
        crate::cards::CardRegistry::with_builtin_cards_for_names(["Breaking", "Grizzly Bears"]);
    let breaking = registry
        .get("Breaking")
        .expect("Breaking should be available in test registry");
    let grizzly = registry
        .get("Grizzly Bears")
        .expect("Grizzly Bears should exist in test registry");
    let reanimate_target = game.create_object_from_definition(grizzly, bob, Zone::Graveyard);

    let split_id = game.create_object_from_definition(breaking, alice, Zone::Hand);
    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        split_id,
        Zone::Hand,
        alice,
        &CastingMethod::SplitOtherHalf,
    )
    .expect("back-half split cast should move to stack");

    let stack_obj = game
        .object(stack_id)
        .expect("stack split spell should exist");
    assert_eq!(stack_obj.name, "Entering");
    assert!(stack_obj.card_types.contains(&CardType::Sorcery));

    let mut dm = SelectFirstDecisionMaker;
    game.stack.push(
        StackEntry::new(stack_id, alice)
            .with_targets(vec![Target::Object(reanimate_target)])
            .with_casting_method(CastingMethod::SplitOtherHalf),
    );
    resolve_stack_entry_with(&mut game, &mut dm).expect("back-half split spell should resolve");
    let reanimated = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .find(|obj| obj.name == "Grizzly Bears" && game.controller_of(obj) == alice);
    assert!(
        reanimated.is_some(),
        "Entering should return a creature card from a graveyard under the caster's control"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_fused_split_cast_combines_effects_and_resolves_in_order() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let registry =
        crate::cards::CardRegistry::with_builtin_cards_for_names(["Breaking", "Grizzly Bears"]);
    let breaking = registry
        .get("Breaking")
        .expect("Breaking should be available in test registry");
    let grizzly = registry
        .get("Grizzly Bears")
        .expect("Grizzly Bears should exist in test registry");
    let reanimate_target = game.create_object_from_definition(grizzly, bob, Zone::Graveyard);

    for idx in 0..8 {
        let card = CardDefinitionBuilder::new(CardId::new(), format!("Bob Library Card {idx}"))
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_definition(&card, bob, Zone::Library);
    }

    let split_id = game.create_object_from_definition(breaking, alice, Zone::Hand);
    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        split_id,
        Zone::Hand,
        alice,
        &CastingMethod::Fuse,
    )
    .expect("fused split cast should move to stack");

    let stack_obj = game
        .object(stack_id)
        .expect("fused split spell should exist");
    assert_eq!(stack_obj.name, "Breaking // Entering");
    assert_eq!(
        stack_obj
            .mana_cost
            .as_ref()
            .map(|cost| cost.to_oracle())
            .as_deref(),
        Some("{U}{B}{4}{B}{R}"),
        "fused split spell should use the combined mana cost on stack"
    );

    let mut dm = SelectFirstDecisionMaker;
    game.stack.push(
        StackEntry::new(stack_id, alice)
            .with_targets(vec![Target::Player(bob), Target::Object(reanimate_target)])
            .with_casting_method(CastingMethod::Fuse),
    );
    let library_before = game.player(bob).expect("bob exists").library.len();
    resolve_stack_entry_with(&mut game, &mut dm).expect("fused split spell should resolve");
    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        library_before.saturating_sub(8),
        "front-half fused effect should mill eight cards"
    );
    let reanimated = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .find(|obj| obj.name == "Grizzly Bears" && game.controller_of(obj) == alice);
    assert!(
        reanimated.is_some(),
        "Entering half of fused spell should return a creature under the caster's control"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_cipher_resolution_encodes_and_combat_damage_casts_a_copy() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell_def = CardDefinitionBuilder::new(CardId::new(), "Cipher Runtime Probe")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Blue,
        ]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("You gain 3 life.\nCipher")
        .expect("cipher runtime probe should parse");

    let encoded_creature = create_creature(&mut game, "Encoded Creature", alice, 2, 2);
    let stack_id = game.create_object_from_definition(&spell_def, alice, Zone::Stack);
    game.stack.push(StackEntry::new(stack_id, alice));

    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm).expect("cipher spell should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").life,
        23,
        "original cipher spell should resolve before encoding"
    );

    let encoded_card = game
        .get_imprinted_cards(encoded_creature)
        .first()
        .copied()
        .expect("cipher spell should be encoded on the chosen creature");
    assert_eq!(
        game.object(encoded_card).map(|obj| obj.zone),
        Some(Zone::Exile),
        "encoded card should remain in exile"
    );
    assert!(
        game.object(encoded_creature)
            .expect("encoded creature exists")
            .abilities
            .iter()
            .any(
                |ability| crate::ability::ability_surface_text_for_tests(ability)
                    .is_some_and(|text| text.contains("encoded card"))
            ),
        "encoded creature should gain the cipher combat-damage trigger"
    );

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            encoded_creature,
            crate::events::DamageTarget::Player(bob),
            2,
            true,
            crate::events::cause::EventCause::combat_damage(encoded_creature),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    for trigger in check_triggers(&game, &damage_event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "cipher should trigger on combat damage"
    );

    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("cipher trigger should go on stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("cipher trigger should resolve");

    let copied_spell_id = game
        .stack
        .last()
        .map(|entry| entry.object_id)
        .expect("cipher trigger should cast a copy onto the stack");
    assert_eq!(
        game.object(copied_spell_id).map(|obj| obj.kind),
        Some(ObjectKind::Token),
        "cipher should create a stack copy rather than moving the encoded card"
    );

    resolve_stack_entry_with(&mut game, &mut dm).expect("cipher copy should resolve");
    assert_eq!(
        game.player(alice).expect("alice exists").life,
        26,
        "casting the cipher copy should resolve the copied spell"
    );
    assert_eq!(
        game.object(encoded_card).map(|obj| obj.zone),
        Some(Zone::Exile),
        "casting a cipher copy should leave the encoded card exiled"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_cipher_copy_cast_prompts_for_targets_before_resolving() {
    struct CipherCopyDecisionMaker {
        encode_creature: ObjectId,
        copied_spell_target: ObjectId,
    }

    impl DecisionMaker for CipherCopyDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if ctx
                .description
                .to_ascii_lowercase()
                .contains("creature you control to encode")
            {
                return vec![self.encode_creature];
            }
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(ctx.min)
                .collect()
        }

        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            let target = Target::Object(self.copied_spell_target);
            if ctx
                .requirements
                .iter()
                .any(|requirement| requirement.legal_targets.contains(&target))
            {
                return vec![target];
            }
            crate::targeting::normalize_targets_for_requirements(&ctx.requirements, Vec::new())
                .unwrap_or_default()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell_def = CardDefinitionBuilder::new(CardId::new(), "Targeted Cipher Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("Create a token that's a copy of target creature.\nCipher")
        .expect("targeted cipher probe should parse");

    let encoded_creature = create_creature(&mut game, "Encoded Creature", alice, 2, 2);
    let copy_target = create_creature(&mut game, "Copy Target", bob, 3, 3);
    let stack_id = game.create_object_from_definition(&spell_def, alice, Zone::Stack);
    game.stack
        .push(StackEntry::new(stack_id, alice).with_targets(vec![Target::Object(copy_target)]));

    let mut dm = CipherCopyDecisionMaker {
        encode_creature: encoded_creature,
        copied_spell_target: copy_target,
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("targeted cipher spell should resolve");
    assert_eq!(
        game.battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|obj| obj.name == "Copy Target" && game.controller_of(obj) == alice)
            .count(),
        1,
        "original targeted cipher spell should create one token copy"
    );

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            encoded_creature,
            crate::events::DamageTarget::Player(bob),
            2,
            true,
            crate::events::cause::EventCause::combat_damage(encoded_creature),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    for trigger in check_triggers(&game, &damage_event) {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("cipher trigger should go on stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("cipher trigger should resolve");

    let copied_spell_entry = game
        .stack
        .last()
        .expect("cipher trigger should cast copied targeted spell");
    assert_eq!(
        copied_spell_entry.targets,
        vec![Target::Object(copy_target)],
        "casting the encoded copy should ask for and store new targets"
    );

    resolve_stack_entry_with(&mut game, &mut dm).expect("targeted cipher copy should resolve");
    assert_eq!(
        game.battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|obj| obj.name == "Copy Target" && game.controller_of(obj) == alice)
            .count(),
        2,
        "targeted cipher copy should resolve using its chosen target"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn prototype_portal_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(90_401), "Prototype Portal")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Imprint — When this artifact enters, you may exile an artifact card from your hand.\n\
             {X}, {T}: Create a token that's a copy of the exiled card. X is the mana value of that card.",
        )
        .expect("Prototype Portal should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_prototype_portal_imprint(
    game: &mut GameState,
    portal_id: ObjectId,
    controller: PlayerId,
) {
    let triggered_effects = game
        .object(portal_id)
        .expect("Prototype Portal should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.effects.clone()),
            _ => None,
        })
        .expect("Prototype Portal should have an imprint trigger");
    game.stack.push(
        StackEntry::ability(portal_id, controller, triggered_effects).with_triggering_event(
            TriggerEvent::new_with_provenance(
                EnterBattlefieldEvent::new(portal_id, Zone::Hand),
                crate::provenance::ProvNodeId::default(),
            ),
        ),
    );
    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(game, &mut dm)
        .expect("Prototype Portal imprint trigger should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn prototype_portal_imprints_artifact_and_copies_it_for_exact_dynamic_x_cost() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let portal = prototype_portal_definition();
    let fuel_card = CardBuilder::new(CardId::from_raw(90_402), "Portal Fuel")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let fuel_id = game.create_object_from_card(&fuel_card, alice, Zone::Hand);
    let portal_id = game.create_object_from_definition(&portal, alice, Zone::Battlefield);

    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. }
                    if source == portal_id
            )),
        "Prototype Portal should not be activatable before a card is imprinted"
    );

    resolve_prototype_portal_imprint(&mut game, portal_id, alice);

    assert!(
        game.object(fuel_id).is_none(),
        "the imprinted card should receive a fresh object id after moving to exile"
    );
    let imprinted = game.get_imprinted_cards(portal_id);
    assert_eq!(
        imprinted.len(),
        1,
        "Prototype Portal should source-link the exiled artifact card"
    );
    let imprinted_id = imprinted[0];
    let imprinted_object = game
        .object(imprinted_id)
        .expect("imprinted card should exist in exile");
    assert_eq!(imprinted_object.zone, Zone::Exile);
    assert_eq!(imprinted_object.name, "Portal Fuel");

    let activated = game
        .object(portal_id)
        .expect("Prototype Portal should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.clone()),
            _ => None,
        })
        .expect("Prototype Portal should have an activated ability");

    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 2);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(portal_id, alice, &mut dm);
    assert!(
        crate::special_actions::pay_total_cost_with_choice_in_context(
            &mut game,
            alice,
            portal_id,
            &activated.mana_cost,
            crate::costs::PaymentReason::ActivateAbility,
            &mut ctx,
        )
        .is_err(),
        "Prototype Portal activation should not accept less mana than the imprinted card's mana value"
    );
    assert!(
        !game.is_tapped(portal_id),
        "failed dynamic mana payment should not pay the tap cost"
    );

    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);
    crate::special_actions::pay_total_cost_with_choice_in_context(
        &mut game,
        alice,
        portal_id,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut ctx,
    )
    .expect("Prototype Portal activation should accept mana equal to imprinted card's mana value");
    assert!(
        game.is_tapped(portal_id),
        "Prototype Portal should tap as a cost"
    );
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .mana_pool
            .total(),
        0,
        "Prototype Portal should spend exactly three mana for the imprinted mana value"
    );

    execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        portal_id,
        &activated.effects,
        None,
        &[],
    )
    .expect("Prototype Portal activation should resolve");

    let copied_tokens = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .filter(|object| object.name == "Portal Fuel" && object.kind == ObjectKind::Token)
        .count();
    assert_eq!(
        copied_tokens, 1,
        "Prototype Portal should create one token copy of the imprinted artifact card"
    );
}

#[test]
pub(super) fn test_rebound_exiles_on_resolution_and_schedules_next_upkeep_cast() {
    use crate::ability::Ability;
    use crate::cards::CardDefinition;
    use crate::effect::Effect;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;
    use crate::triggers::TriggerQueue;
    use crate::types::CardType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);

    let card = crate::card::CardBuilder::new(CardId::new(), "Rebound Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .build();
    let mut definition = CardDefinition::new(card);
    definition.spell_effect = Some(crate::resolution::ResolutionProgram::from_effects(vec![
        Effect::gain_life(1),
    ]));
    definition
        .abilities
        .push(Ability::static_ability(StaticAbility::rebound()).in_zones(vec![Zone::Stack]));

    let rebound_id = game.create_object_from_definition(&definition, alice, Zone::Hand);

    let mut state = PriorityLoopState::new(2);
    let mut trigger_queue = TriggerQueue::new();

    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: rebound_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });
    let result = apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
    assert!(result.is_ok(), "normal cast with rebound should succeed");

    assert_eq!(game.stack.len(), 1, "spell should be on stack");
    resolve_stack_entry(&mut game).expect("rebound spell should resolve");

    let in_exile = game.exile.iter().any(|&id| {
        game.object(id)
            .map(|o| o.name == "Rebound Probe")
            .unwrap_or(false)
    });
    assert!(in_exile, "rebound spell should be exiled on resolution");

    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        1,
        "rebound should schedule exactly one next-upkeep cast trigger"
    );
    let delayed_debug = format!("{:?}", game.effect_store.delayed_triggers[0].effects);
    assert!(
        delayed_debug.contains("CastSourceEffect"),
        "rebound delayed trigger should cast the exiled source, got {delayed_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_flashback_pays_alternative_cost() {
    use crate::cards::definitions::think_twice;
    use crate::mana::ManaSymbol;
    use crate::triggers::TriggerQueue;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Add exactly 3 blue mana (flashback cost is {2}{U})
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 3);

    // Create Think Twice in graveyard
    let think_twice_def = think_twice();
    let think_twice_id =
        game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);

    // Cast with flashback
    let mut state = PriorityLoopState::new(2);
    let mut trigger_queue = TriggerQueue::new();

    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: think_twice_id,
        from_zone: Zone::Graveyard,
        casting_method: CastingMethod::Alternative(0),
    });

    let result = apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
    assert!(result.is_ok(), "Casting with flashback should succeed");

    // Verify mana was spent (flashback costs {2}{U} = 3 total, we had 3 blue)
    let mana_pool = &game.player(alice).unwrap().mana_pool;
    assert_eq!(mana_pool.blue, 0, "Should have spent all mana on flashback");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_normal_cast_goes_to_graveyard() {
    use crate::cards::definitions::think_twice;
    use crate::mana::ManaSymbol;
    use crate::triggers::TriggerQueue;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Add 2 blue mana (normal cost is {1}{U})
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);

    // Create Think Twice in HAND
    let think_twice_def = think_twice();
    let think_twice_id = game.create_object_from_definition(&think_twice_def, alice, Zone::Hand);

    // Cast normally
    let mut state = PriorityLoopState::new(2);
    let mut trigger_queue = TriggerQueue::new();

    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: think_twice_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });

    let result = apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response);
    assert!(result.is_ok(), "Normal casting should succeed");

    // Resolve the spell
    resolve_stack_entry(&mut game).expect("Resolution should succeed");

    // Verify spell is in graveyard (not exile) after normal cast
    let player = game.player(alice).unwrap();
    let in_graveyard = player.graveyard.iter().any(|&id| {
        game.object(id)
            .map(|o| o.name == "Think Twice")
            .unwrap_or(false)
    });
    assert!(
        in_graveyard,
        "Think Twice SHOULD be in graveyard after normal cast"
    );

    let in_exile = game.exile.iter().any(|&id| {
        game.object(id)
            .map(|o| o.name == "Think Twice")
            .unwrap_or(false)
    });
    assert!(
        !in_exile,
        "Think Twice should NOT be in exile after normal cast"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_flashback_requires_enough_mana() {
    use crate::cards::definitions::think_twice;
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Add only 2 mana (flashback costs {2}{U} = 3 total)
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);

    // Create Think Twice in graveyard
    let think_twice_def = think_twice();
    let think_twice_id =
        game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should NOT find flashback action (not enough mana)
    let flashback_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                casting_method: CastingMethod::Alternative(_),
                ..
            } if *spell_id == think_twice_id
        )
    });

    assert!(
        flashback_action.is_none(),
        "Should NOT be able to cast with flashback without enough mana"
    );
}

// =========================================================================
// Everflowing Chalice / Multikicker Tests
// =========================================================================

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn strength_of_the_tajuru_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(76_300), "Strength of the Tajuru")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Multikicker {1} (You may pay an additional {1} any number of times as you cast this spell.)\n\
             Choose target creature, then choose another target creature for each time this spell was kicked. Put X +1/+1 counters on each of them.",
        )
        .expect("Strength of the Tajuru should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn spell_contortion_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(76_301), "Spell Contortion")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Multikicker {1}{U} (You may pay an additional {1}{U} any number of times as you cast this spell.)\n\
             Counter target spell unless its controller pays {2}. Draw a card for each time Spell Contortion was kicked.",
        )
        .expect("Spell Contortion should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn frightful_delusion_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(76_303), "Frightful Delusion")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target spell unless its controller pays {1}. That player discards a card.",
        )
        .expect("Frightful Delusion should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn thassas_intervention_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(76_304), "Thassa's Intervention")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose one —\n\
             • Look at the top X cards of your library. Put up to two of them into your hand and the rest on the bottom of your library in a random order.\n\
             • Counter target spell unless its controller pays twice {X}.",
        )
        .expect("Thassa's Intervention should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn stack_spell_probe_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(76_302), "Stack Spell Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Draw a card.")
        .expect("stack spell probe should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_spell_contortion_on_stack(
    game: &mut GameState,
    controller: PlayerId,
    target_spell: ObjectId,
    kicks: u32,
) -> ObjectId {
    let def = spell_contortion_definition();
    let source = game.create_object_from_definition(&def, controller, Zone::Stack);
    let mut paid = crate::cost::OptionalCostsPaid::from_costs(&def.optional_costs);
    paid.pay_times(0, kicks);
    game.push_to_stack(
        StackEntry::new(source, controller)
            .with_targets(vec![Target::Object(target_spell)])
            .with_optional_costs_paid(paid),
    );
    source
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_frightful_delusion_on_stack(
    game: &mut GameState,
    controller: PlayerId,
    target_spell: ObjectId,
) -> ObjectId {
    let def = frightful_delusion_definition();
    let source = game.create_object_from_definition(&def, controller, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(source, controller).with_targets(vec![Target::Object(target_spell)]),
    );
    source
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_thassas_intervention_counter_mode_on_stack(
    game: &mut GameState,
    controller: PlayerId,
    target_spell: ObjectId,
    x_value: u32,
) -> ObjectId {
    let def = thassas_intervention_definition();
    let source = game.create_object_from_definition(&def, controller, Zone::Stack);
    if let Some(spell) = game.object_mut(source) {
        spell.x_value = Some(x_value);
    }
    game.push_to_stack(
        StackEntry::new(source, controller)
            .with_x(x_value)
            .with_chosen_modes(Some(vec![1]))
            .with_targets(vec![Target::Object(target_spell)]),
    );
    source
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ChooseSpecificDiscardDecisionMaker {
    pub(super) card_to_discard: ObjectId,
    pub(super) accept_boolean: bool,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChooseSpecificDiscardDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.accept_boolean
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if ctx
            .candidates
            .iter()
            .any(|candidate| candidate.id == self.card_to_discard && candidate.legal)
        {
            vec![self.card_to_discard]
        } else {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(ctx.min)
                .collect()
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_hand_card(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
    raw_id: u32,
) -> ObjectId {
    let card = CardBuilder::new(CardId::from_raw(raw_id), name)
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&card, owner, Zone::Hand)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_strength_of_the_tajuru_no_kicks_requires_one_creature_target() {
    let def = strength_of_the_tajuru_definition();
    let effects = def.spell_effect.clone().expect("expected spell effects");
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    game.object_mut(source).expect("source").optional_costs_paid =
        crate::cost::OptionalCostsPaid::from_costs(&def.optional_costs);

    let creature_a = create_creature(&mut game, "Tajuru Target A", bob, 2, 2);
    let creature_b = create_creature(&mut game, "Tajuru Target B", bob, 3, 3);

    let requirements = extract_target_requirements_from_program_with_modes(
        &game,
        &effects,
        alice,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 1, "expected one target requirement");
    assert_eq!(requirements[0].min_targets, 1);
    assert_eq!(requirements[0].max_targets, Some(1));
    assert!(
        requirements[0]
            .legal_targets
            .contains(&Target::Object(creature_a))
            && requirements[0]
                .legal_targets
                .contains(&Target::Object(creature_b)),
        "both creatures should be legal no-kick targets, got {:?}",
        requirements[0].legal_targets
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_strength_of_the_tajuru_two_kicks_requires_three_creature_targets() {
    let def = strength_of_the_tajuru_definition();
    let effects = def.spell_effect.clone().expect("expected spell effects");
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut paid = crate::cost::OptionalCostsPaid::from_costs(&def.optional_costs);
    paid.pay_times(0, 2);
    game.object_mut(source).expect("source").optional_costs_paid = paid;

    let creature_a = create_creature(&mut game, "Tajuru Target A", bob, 2, 2);
    let creature_b = create_creature(&mut game, "Tajuru Target B", bob, 3, 3);
    let creature_c = create_creature(&mut game, "Tajuru Target C", bob, 4, 4);

    let requirements = extract_target_requirements_from_program_with_modes(
        &game,
        &effects,
        alice,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 1, "expected one target requirement");
    assert_eq!(requirements[0].min_targets, 3);
    assert_eq!(requirements[0].max_targets, Some(3));
    assert!(
        [creature_a, creature_b, creature_c]
            .into_iter()
            .all(|id| requirements[0].legal_targets.contains(&Target::Object(id))),
        "all three creatures should be legal kicked targets, got {:?}",
        requirements[0].legal_targets
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_strength_of_the_tajuru_two_kicks_is_illegal_with_only_two_targets() {
    let def = strength_of_the_tajuru_definition();
    let effects = def.spell_effect.clone().expect("expected spell effects");
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut paid = crate::cost::OptionalCostsPaid::from_costs(&def.optional_costs);
    paid.pay_times(0, 2);
    game.object_mut(source).expect("source").optional_costs_paid = paid;

    create_creature(&mut game, "Tajuru Target A", bob, 2, 2);
    create_creature(&mut game, "Tajuru Target B", bob, 3, 3);

    let requirements = extract_target_requirements_from_program_with_modes(
        &game,
        &effects,
        alice,
        Some(source),
        None,
    );
    assert!(
        requirements.is_empty(),
        "two kicks require three creature targets, got {requirements:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_strength_of_the_tajuru_puts_x_counters_on_each_kicked_target() {
    let def = strength_of_the_tajuru_definition();
    let effects = def.spell_effect.clone().expect("expected spell effects");
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut paid = crate::cost::OptionalCostsPaid::from_costs(&def.optional_costs);
    paid.pay_times(0, 2);
    game.object_mut(source).expect("source").optional_costs_paid = paid.clone();

    let creature_a = create_creature(&mut game, "Tajuru Target A", bob, 2, 2);
    let creature_b = create_creature(&mut game, "Tajuru Target B", bob, 3, 3);
    let creature_c = create_creature(&mut game, "Tajuru Target C", bob, 4, 4);
    let untargeted = create_creature(&mut game, "Untargeted Creature", bob, 5, 5);

    let entry = StackEntry::ability(source, alice, effects)
        .with_targets(vec![
            Target::Object(creature_a),
            Target::Object(creature_b),
            Target::Object(creature_c),
        ])
        .with_optional_costs_paid(paid)
        .with_x(4);
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("Strength of the Tajuru should resolve");

    for target in [creature_a, creature_b, creature_c] {
        let counters = game
            .object(target)
            .expect("target creature")
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0);
        assert_eq!(counters, 4, "target {target:?} should get X counters");
    }
    let untargeted_counters = game
        .object(untargeted)
        .expect("untargeted creature")
        .counters
        .get(&crate::object::CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        untargeted_counters, 0,
        "Strength of the Tajuru should affect only its chosen targets"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn frightful_delusion_unpaid_counter_discards_target_spell_controller_card() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let target_def = stack_spell_probe_definition();
    let target_spell = game.create_object_from_definition(&target_def, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(target_spell, bob));
    let bob_discard = create_hand_card(&mut game, bob, "Bob Discards", 76_304);
    let alice_keeps = create_hand_card(&mut game, alice, "Alice Keeps", 76_305);
    let frightful_delusion = put_frightful_delusion_on_stack(&mut game, alice, target_spell);
    let mut dm = ChooseSpecificDiscardDecisionMaker {
        card_to_discard: bob_discard,
        accept_boolean: false,
    };

    resolve_stack_entry_with(&mut game, &mut dm).expect("Frightful Delusion should resolve");

    let countered_target = game
        .current_object_id_after_zone_change(target_spell)
        .expect("target spell should still be tracked after zone change");
    assert_eq!(
        game.object(countered_target)
            .expect("target spell exists")
            .zone,
        Zone::Graveyard,
        "the target spell should be countered when its controller cannot pay {{1}}"
    );
    assert!(
        game.player(bob)
            .is_some_and(|player| player.graveyard.iter().any(|&id| game
                .object(id)
                .is_some_and(|obj| obj.name == "Bob Discards"))),
        "the target spell's controller should discard a card"
    );
    assert!(
        game.object(alice_keeps)
            .is_some_and(|obj| obj.zone == Zone::Hand),
        "Frightful Delusion should not make its controller discard"
    );
    assert!(
        !game
            .stack
            .iter()
            .any(|entry| entry.object_id == frightful_delusion),
        "Frightful Delusion should leave the stack after resolving"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn frightful_delusion_paid_target_survives_but_controller_still_discards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.player_mut(bob)
        .expect("bob exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    let target_def = stack_spell_probe_definition();
    let target_spell = game.create_object_from_definition(&target_def, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(target_spell, bob));
    let bob_discard = create_hand_card(&mut game, bob, "Bob Pays And Discards", 76_306);
    let frightful_delusion = put_frightful_delusion_on_stack(&mut game, alice, target_spell);
    let mut dm = ChooseSpecificDiscardDecisionMaker {
        card_to_discard: bob_discard,
        accept_boolean: true,
    };

    resolve_stack_entry_with(&mut game, &mut dm).expect("Frightful Delusion should resolve");

    assert_eq!(
        game.object(target_spell).expect("target spell exists").zone,
        Zone::Stack,
        "the target spell should remain on the stack when its controller pays {{1}}"
    );
    assert!(
        game.stack
            .iter()
            .any(|entry| entry.object_id == target_spell),
        "the paid-for target spell should still have a stack entry"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").mana_pool.total(),
        0,
        "the target spell's controller should spend {{1}} to prevent the counter effect"
    );
    assert!(
        game.player(bob)
            .is_some_and(|player| player.graveyard.iter().any(|&id| game
                .object(id)
                .is_some_and(|obj| obj.name == "Bob Pays And Discards"))),
        "the target spell's controller should discard even after paying for Frightful Delusion"
    );
    assert!(
        !game
            .stack
            .iter()
            .any(|entry| entry.object_id == frightful_delusion),
        "Frightful Delusion should leave the stack after resolving"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn spell_contortion_no_kicks_counters_unpaid_target_and_draws_no_cards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    put_test_cards_in_zone(&mut game, alice, Zone::Library, 3);

    let target_def = stack_spell_probe_definition();
    let target_spell = game.create_object_from_definition(&target_def, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(target_spell, bob));
    let spell_contortion = put_spell_contortion_on_stack(&mut game, alice, target_spell, 0);

    let alice_hand_before = game.player(alice).expect("alice exists").hand.len();
    resolve_stack_entry(&mut game).expect("Spell Contortion should resolve");

    let countered_target = game
        .current_object_id_after_zone_change(target_spell)
        .expect("target spell should still be tracked after zone change");
    assert_eq!(
        game.object(countered_target)
            .expect("target spell exists")
            .zone,
        Zone::Graveyard,
        "the target spell should be countered when its controller cannot pay {{2}}"
    );
    assert!(
        !game
            .stack
            .iter()
            .any(|entry| entry.object_id == spell_contortion)
            && game.objects_in_zone(Zone::Graveyard).iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.name == "Spell Contortion")
            }),
        "Spell Contortion should leave the stack and go to its owner's graveyard after resolving"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        alice_hand_before,
        "Spell Contortion with zero kicks should draw no cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn spell_contortion_two_kicks_counters_unpaid_target_and_still_draws_two_cards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    put_test_cards_in_zone(&mut game, alice, Zone::Library, 3);

    let target_def = stack_spell_probe_definition();
    let target_spell = game.create_object_from_definition(&target_def, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(target_spell, bob));
    let spell_contortion = put_spell_contortion_on_stack(&mut game, alice, target_spell, 2);

    let alice_hand_before = game.player(alice).expect("alice exists").hand.len();
    resolve_stack_entry(&mut game).expect("Spell Contortion should resolve");

    let countered_target = game
        .current_object_id_after_zone_change(target_spell)
        .expect("target spell should still be tracked after zone change");
    assert_eq!(
        game.object(countered_target)
            .expect("target spell exists")
            .zone,
        Zone::Graveyard,
        "the target spell should be countered when its controller cannot pay {{2}}"
    );
    assert!(
        !game
            .stack
            .iter()
            .any(|entry| entry.object_id == spell_contortion)
            && game.objects_in_zone(Zone::Graveyard).iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.name == "Spell Contortion")
            }),
        "Spell Contortion should leave the stack and go to its owner's graveyard after resolving"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        alice_hand_before + 2,
        "Spell Contortion kicked twice should draw two cards even when the target spell is countered"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn spell_contortion_two_kicks_draws_two_cards_when_target_controller_pays() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    put_test_cards_in_zone(&mut game, alice, Zone::Library, 3);
    game.player_mut(bob)
        .expect("bob exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);

    let target_def = stack_spell_probe_definition();
    let target_spell = game.create_object_from_definition(&target_def, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(target_spell, bob));
    let spell_contortion = put_spell_contortion_on_stack(&mut game, alice, target_spell, 2);

    let alice_hand_before = game.player(alice).expect("alice exists").hand.len();
    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm).expect("Spell Contortion should resolve");

    assert_eq!(
        game.object(target_spell).expect("target spell exists").zone,
        Zone::Stack,
        "the target spell should remain on the stack when its controller pays {{2}}"
    );
    assert!(
        game.stack
            .iter()
            .any(|entry| entry.object_id == target_spell),
        "the paid-for target spell should still have a stack entry"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").mana_pool.total(),
        0,
        "the target spell's controller should spend {{2}} to prevent the counter effect"
    );
    assert!(
        !game
            .stack
            .iter()
            .any(|entry| entry.object_id == spell_contortion)
            && game.objects_in_zone(Zone::Graveyard).iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.name == "Spell Contortion")
            }),
        "Spell Contortion should leave the stack and go to its owner's graveyard after resolving"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        alice_hand_before + 2,
        "Spell Contortion kicked twice should draw two cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn thassas_intervention_counter_mode_counters_when_target_controller_cannot_pay_twice_x()
{
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let target_def = stack_spell_probe_definition();
    let target_spell = game.create_object_from_definition(&target_def, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(target_spell, bob));
    let thassas_intervention =
        put_thassas_intervention_counter_mode_on_stack(&mut game, alice, target_spell, 2);

    resolve_stack_entry(&mut game).expect("Thassa's Intervention should resolve");

    let countered_target = game
        .current_object_id_after_zone_change(target_spell)
        .expect("target spell should still be tracked after zone change");
    assert_eq!(
        game.object(countered_target)
            .expect("target spell exists")
            .zone,
        Zone::Graveyard,
        "the target spell should be countered when its controller cannot pay twice X"
    );
    assert!(
        !game
            .stack
            .iter()
            .any(|entry| entry.object_id == thassas_intervention)
            && game.objects_in_zone(Zone::Graveyard).iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.name == "Thassa's Intervention")
            }),
        "Thassa's Intervention should leave the stack and go to its owner's graveyard after resolving"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn thassas_intervention_counter_mode_does_not_counter_when_target_controller_pays_twice_x()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.player_mut(bob)
        .expect("bob exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 4);

    let target_def = stack_spell_probe_definition();
    let target_spell = game.create_object_from_definition(&target_def, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(target_spell, bob));
    let thassas_intervention =
        put_thassas_intervention_counter_mode_on_stack(&mut game, alice, target_spell, 2);

    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm).expect("Thassa's Intervention should resolve");

    assert_eq!(
        game.object(target_spell).expect("target spell exists").zone,
        Zone::Stack,
        "the target spell should remain on the stack when its controller pays twice X"
    );
    assert!(
        game.stack
            .iter()
            .any(|entry| entry.object_id == target_spell),
        "the paid-for target spell should still have a stack entry"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").mana_pool.total(),
        0,
        "the target spell's controller should spend four mana when X is 2"
    );
    assert!(
        !game
            .stack
            .iter()
            .any(|entry| entry.object_id == thassas_intervention)
            && game.objects_in_zone(Zone::Graveyard).iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.name == "Thassa's Intervention")
            }),
        "Thassa's Intervention should leave the stack and go to its owner's graveyard after resolving"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_everflowing_chalice_no_kicks() {
    use crate::cards::definitions::everflowing_chalice;
    use crate::cost::OptionalCostsPaid;
    use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::object::CounterType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Everflowing Chalice directly on battlefield with 0 kicks
    let chalice_def = everflowing_chalice();
    let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Battlefield);

    // Simulate that it entered with 0 kicks by running the ETB effect
    // with an ExecutionContext that has 0 kicks
    let paid = OptionalCostsPaid::from_costs(&chalice_def.optional_costs);
    let mut ctx = ExecutionContext::new_default(chalice_id, alice)
        .with_optional_costs_paid(paid)
        .with_targets(vec![ResolvedTarget::Object(chalice_id)]);

    // Execute the ETB effect (put charge counters equal to kick count)
    let etb_effect = Effect::put_counters_on_source(CounterType::Charge, Value::KickCount);
    execute_effect(&mut game, &etb_effect, &mut ctx).unwrap();

    // Should have 0 charge counters
    let chalice = game.object(chalice_id).unwrap();
    let charge_counters = chalice
        .counters
        .get(&CounterType::Charge)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        charge_counters, 0,
        "Should have 0 charge counters with 0 kicks"
    );

    // Tap for mana - should produce 0 colorless
    let mana_effect = Effect::add_colorless_mana(Value::CountersOnSource(CounterType::Charge));
    let mut mana_ctx = ExecutionContext::new_default(chalice_id, alice);
    execute_effect(&mut game, &mana_effect, &mut mana_ctx).unwrap();

    assert_eq!(
        game.player(alice).unwrap().mana_pool.colorless,
        0,
        "Should produce 0 colorless mana with 0 counters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_everflowing_chalice_one_kick() {
    use crate::cards::definitions::everflowing_chalice;
    use crate::cost::OptionalCostsPaid;
    use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::object::CounterType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create Everflowing Chalice directly on battlefield
    let chalice_def = everflowing_chalice();
    let chalice_id = game.create_object_from_definition(&chalice_def, alice, Zone::Battlefield);

    // Simulate that it entered with 1 kick
    let mut paid = OptionalCostsPaid::from_costs(&chalice_def.optional_costs);
    paid.pay(0); // Pay multikicker once
    let mut ctx = ExecutionContext::new_default(chalice_id, alice)
        .with_optional_costs_paid(paid)
        .with_targets(vec![ResolvedTarget::Object(chalice_id)]);

    // Execute the ETB effect
    let etb_effect = Effect::put_counters_on_source(CounterType::Charge, Value::KickCount);
    execute_effect(&mut game, &etb_effect, &mut ctx).unwrap();

    // Should have 1 charge counter
    let chalice = game.object(chalice_id).unwrap();
    assert_eq!(
        chalice.counters.get(&CounterType::Charge),
        Some(&1),
        "Should have 1 charge counter with 1 kick"
    );

    // Tap for mana - should produce 1 colorless
    let mana_effect = Effect::add_colorless_mana(Value::CountersOnSource(CounterType::Charge));
    let mut mana_ctx = ExecutionContext::new_default(chalice_id, alice);
    execute_effect(&mut game, &mana_effect, &mut mana_ctx).unwrap();

    assert_eq!(
        game.player(alice).unwrap().mana_pool.colorless,
        1,
        "Should produce 1 colorless mana with 1 counter"
    );
}

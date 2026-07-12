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
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_word_of_blasting_destroys_wall_and_deals_mana_value_damage_to_controller() {
    use crate::decision::LegalAction;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let word_def = CardDefinitionBuilder::new(CardId::new(), "Word of Blasting")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target Wall. It can't be regenerated. Word of Blasting deals damage equal to that Wall's mana value to the Wall's controller.",
        )
        .expect("Word of Blasting should parse");
    let spell_id = game.create_object_from_definition(&word_def, alice, Zone::Hand);

    game.player_mut(alice)
        .expect("alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 2);

    let wall = CardBuilder::new(CardId::new(), "Runed Wall")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Wall])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .power_toughness(PowerToughness::fixed(0, 4))
        .build();
    let wall_id = game.create_object_from_card(&wall, bob, Zone::Battlefield);

    let cast_action = LegalAction::CastSpell {
        spell_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    };
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());

    let cast_response = PriorityResponse::PriorityAction(cast_action);
    let progress =
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
            .expect("Word of Blasting cast should start");
    assert!(
        matches!(
            progress,
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(_)
            )
        ),
        "Word of Blasting cast should request a wall target"
    );

    let choose_target = PriorityResponse::Targets(vec![Target::Object(wall_id)]);
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &choose_target)
        .expect("should accept wall target");
    assert_eq!(game.stack.len(), 1, "Word of Blasting should be on stack");

    resolve_stack_entry(&mut game).expect("Word of Blasting should resolve");

    assert!(
        !game.battlefield.contains(&wall_id),
        "Word of Blasting should destroy the targeted Wall"
    );
    assert_eq!(
        game.player(bob).expect("bob should exist").life,
        16,
        "Word of Blasting should deal damage equal to the destroyed Wall's mana value"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_word_of_blasting_has_no_cast_action_without_a_wall_target() {
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let word_def = CardDefinitionBuilder::new(CardId::new(), "Word of Blasting")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target Wall. It can't be regenerated. Word of Blasting deals damage equal to that Wall's mana value to the Wall's controller.",
        )
        .expect("Word of Blasting should parse");
    let spell_id = game.create_object_from_definition(&word_def, alice, Zone::Hand);

    game.player_mut(alice)
        .expect("alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 2);

    let non_wall = CardBuilder::new(CardId::new(), "Vanilla Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_card(&non_wall, bob, Zone::Battlefield);

    let actions = compute_legal_actions(&game, alice);
    let can_cast_word = actions.iter().any(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id: candidate,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *candidate == spell_id
        )
    });
    assert!(
        !can_cast_word,
        "Word of Blasting should not be castable when no Wall is a legal target"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_brutal_suppression_adds_a_land_sacrifice_activation_cost() {
    use crate::PriorityResponse;
    use crate::cost::TotalCost;
    use crate::decision::LegalAction;
    use crate::types::Subtype;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let brutal_suppression = CardDefinitionBuilder::new(CardId::new(), "Brutal Suppression")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Activated abilities of nontoken Rebels cost an additional \"Sacrifice a land\" to activate.",
        )
        .expect("Brutal Suppression should parse");
    game.create_object_from_definition(&brutal_suppression, alice, Zone::Battlefield);

    let rebel = CardBuilder::new(CardId::new(), "Rebel Initiate")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Rebel])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let rebel_id = game.create_object_from_card(&rebel, alice, Zone::Battlefield);

    game.object_mut(rebel_id)
        .expect("rebel exists")
        .abilities_mut()
        .push(Ability::activated(
            TotalCost::free(),
            crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
        ));

    let actions_without_land = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        !actions_without_land.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility { source, .. } if *source == rebel_id
        )),
        "rebel ability should not be activatable without a land to sacrifice"
    );

    let land = CardBuilder::new(CardId::new(), "Test Land")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
    let land_stable_id = game.object(land_id).expect("land exists").stable_id;

    let actions_with_land = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions_with_land.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility { source, .. } if *source == rebel_id
        )),
        "rebel ability should become activatable once a land is available"
    );

    let ability_index = game
        .object(rebel_id)
        .expect("rebel should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("rebel should have an activated ability");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;

    let activate = PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
        source: rebel_id,
        ability_index,
    });
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &activate,
        &mut dm,
    )
    .expect("activation should start");

    let progress = match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(cost_ctx),
        ) => {
            let sacrifice_cost_index = cost_ctx
                .options
                .iter()
                .find(|option| {
                    option
                        .description
                        .to_ascii_lowercase()
                        .contains("sacrifice")
                })
                .map(|option| option.index)
                .expect("expected a sacrifice cost option");

            apply_priority_response_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::NextCostChoice(sacrifice_cost_index),
                &mut dm,
            )
            .expect("should choose the sacrifice cost")
        }
        direct => direct,
    };

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(_),
        ) => {}
        other => panic!(
            "expected sacrifice target chooser for Brutal Suppression, got {:?}",
            other
        ),
    }

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::SacrificeTarget(land_id),
        &mut dm,
    )
    .expect("should sacrifice the land as the activation cost");

    let current_land_id = game
        .find_object_by_stable_id(land_stable_id)
        .expect("sacrificed land should still be tracked by stable id");
    assert_eq!(
        game.object(current_land_id)
            .expect("sacrificed land should still exist")
            .zone,
        Zone::Graveyard,
        "Brutal Suppression should sacrifice the chosen land as part of activation"
    );
    assert_eq!(
        game.stack.len(),
        1,
        "the Rebel ability should reach the stack after paying the extra activation cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_yawgmoth_sacrifice_activation_targets_before_paying_costs() {
    use crate::decision::{DecisionMaker, LegalAction};

    #[derive(Debug)]
    struct YawgmothOrderingDecisionMaker {
        alice: PlayerId,
        sacrifice: ObjectId,
        decision_order: Vec<&'static str>,
        life_when_object_cost_chosen: Option<i32>,
    }

    impl DecisionMaker for YawgmothOrderingDecisionMaker {
        fn decide_objects(
            &mut self,
            game: &GameState,
            _ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.decision_order.push("objects");
            self.life_when_object_cost_chosen = game.player(self.alice).map(|player| player.life);
            vec![self.sacrifice]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let registry = crate::cards::CardRegistry::with_builtin_cards_for_names([
        "Yawgmoth, Thran Physician",
        "Black Lotus",
    ]);
    let yawgmoth_def = registry
        .get("Yawgmoth, Thran Physician")
        .expect("Yawgmoth, Thran Physician should be present in registry");
    let yawgmoth_id = game.create_object_from_definition(yawgmoth_def, alice, Zone::Battlefield);

    let fodder = CardBuilder::new(CardId::new(), "Fodder")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let fodder_id = game.create_object_from_card(&fodder, alice, Zone::Battlefield);

    let target_creature = CardBuilder::new(CardId::new(), "Target Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target_id = game.create_object_from_card(&target_creature, bob, Zone::Battlefield);

    let sacrifice_ability_index = game
        .object(yawgmoth_id)
        .expect("Yawgmoth should exist")
        .abilities
        .iter()
        .position(|ability| {
            if let AbilityKind::Activated(activated) = &ability.kind {
                activated.life_cost_amount() == Some(1)
            } else {
                false
            }
        })
        .expect("Yawgmoth should have sacrifice ability");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = YawgmothOrderingDecisionMaker {
        alice,
        sacrifice: fodder_id,
        decision_order: Vec::new(),
        life_when_object_cost_chosen: None,
    };

    let activate = PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
        source: yawgmoth_id,
        ability_index: sacrifice_ability_index,
    });
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &activate,
        &mut dm,
    )
    .expect("Yawgmoth sacrifice ability should activate");

    let targets_ctx = match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(ctx),
        ) => ctx,
        other => panic!(
            "expected target prompt before paying Yawgmoth's costs, got {:?}",
            other
        ),
    };

    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        20,
        "life should not be paid before the target decision"
    );
    assert_eq!(
        targets_ctx.requirements.len(),
        1,
        "Yawgmoth's first ability should prompt for its creature target before costs"
    );

    let choose_target = PriorityResponse::Targets(vec![Target::Object(target_id)]);
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &choose_target,
        &mut dm,
    )
    .expect("Yawgmoth target choice should continue activation");

    let next_cost_ctx = match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!(
            "expected next-cost chooser after Yawgmoth target selection, got {:?}",
            other
        ),
    };
    let life_cost_index = next_cost_ctx
        .options
        .iter()
        .find(|opt| opt.description.to_ascii_lowercase().contains("life"))
        .map(|opt| opt.index)
        .expect("expected a life-payment option");
    let choose_life_first = PriorityResponse::NextCostChoice(life_cost_index);
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &choose_life_first,
        &mut dm,
    )
    .expect("Yawgmoth should accept paying life first");

    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(_),
        ) => {}
        other => panic!(
            "expected sacrifice selection prompt after Yawgmoth life payment, got {:?}",
            other
        ),
    }

    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        19,
        "Yawgmoth activation should pay 1 life"
    );
    assert!(game.battlefield.contains(&fodder_id));

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::SacrificeTarget(fodder_id),
        &mut dm,
    )
    .expect("Yawgmoth should accept the chosen sacrifice");

    assert!(!game.battlefield.contains(&fodder_id));
    assert!(
        game.player(alice)
            .expect("Alice exists")
            .graveyard
            .iter()
            .filter_map(|&id| game.object(id))
            .any(|obj| obj.name == "Fodder"),
        "chosen creature should appear in Alice's graveyard after being sacrificed"
    );
    assert_eq!(
        game.stack.len(),
        1,
        "Yawgmoth ability should be on the stack"
    );
    let yawgmoth_entry = game
        .stack
        .last()
        .expect("Yawgmoth ability should be on the stack");
    let sacrificed = yawgmoth_entry
        .tagged_objects
        .get(&crate::tag::TagKey::from("sacrifice_cost_0"))
        .expect("Yawgmoth stack entry should keep the sacrificed-creature tag");
    assert_eq!(sacrificed.len(), 1);
    assert_eq!(sacrificed[0].name, "Fodder");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn phyrexian_tower_direct_mana_activation_pays_selected_sacrifice_cost() {
    use crate::PriorityResponse;
    use crate::cards::definitions::{grizzly_bears, ornithopter, phyrexian_tower};
    use crate::decision::LegalAction;
    use crate::game_loop::{PriorityLoopState, apply_priority_response_with_dm};

    struct ChooseTowerSacrificeDecisionMaker {
        desired: ObjectId,
        seen_candidates: Vec<ObjectId>,
    }

    impl DecisionMaker for ChooseTowerSacrificeDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.seen_candidates = ctx
                .candidates
                .iter()
                .map(|candidate| candidate.id)
                .collect();
            vec![self.desired]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let tower_id = game.create_object_from_definition(&phyrexian_tower(), alice, Zone::Battlefield);
    let bear_id = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
    let thopter_id = game.create_object_from_definition(&ornithopter(), alice, Zone::Battlefield);
    let bear_stable_id = game
        .object(bear_id)
        .expect("Grizzly Bears should exist")
        .stable_id;

    let ability_index = game
        .object(tower_id)
        .expect("Phyrexian Tower should exist")
        .abilities
        .iter()
        .position(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Activated(activated)
                    if activated.mana_output.as_ref().is_some_and(|mana| {
                        mana.iter().filter(|symbol| **symbol == ManaSymbol::Black).count() == 2
                    })
            )
        })
        .expect("Phyrexian Tower should have its sacrifice mana ability");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = ChooseTowerSacrificeDecisionMaker {
        desired: bear_id,
        seen_candidates: Vec::new(),
    };
    let response = PriorityResponse::PriorityAction(LegalAction::ActivateManaAbility {
        source: tower_id,
        ability_index,
    });

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &response,
        &mut dm,
    )
    .expect("Phyrexian Tower sacrifice mana ability should resolve");

    assert!(
        dm.seen_candidates.contains(&bear_id) && dm.seen_candidates.contains(&thopter_id),
        "Tower sacrifice prompt should offer Alice's creatures: {:?}",
        dm.seen_candidates
    );
    assert!(
        !game.battlefield.contains(&bear_id),
        "selected creature should be sacrificed"
    );
    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .graveyard
            .contains(
                &game
                    .find_object_by_stable_id(bear_stable_id)
                    .expect("sacrificed creature should still be tracked by stable id")
            ),
        "selected creature should move to Alice's graveyard"
    );
    assert!(
        game.is_tapped(tower_id),
        "Phyrexian Tower should tap as part of the activation cost"
    );
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .mana_pool
            .black,
        2,
        "Phyrexian Tower should add two black mana"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_yawgmoth_proliferate_activation_prompts_discard_choice() {
    use crate::decision::{GameProgress, LegalAction};
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let registry = crate::cards::CardRegistry::with_builtin_cards_for_names([
        "Yawgmoth, Thran Physician",
        "Black Lotus",
    ]);
    let yawgmoth_def = registry
        .get("Yawgmoth, Thran Physician")
        .expect("Yawgmoth, Thran Physician should be present in registry");
    let yawgmoth_id = game.create_object_from_definition(yawgmoth_def, alice, Zone::Battlefield);

    let discard_one = CardBuilder::new(CardId::new(), "Discard One")
        .card_types(vec![CardType::Instant])
        .build();
    let discard_two = CardBuilder::new(CardId::new(), "Discard Two")
        .card_types(vec![CardType::Sorcery])
        .build();
    let hand_card_one = game.create_object_from_card(&discard_one, alice, Zone::Hand);
    let hand_card_two = game.create_object_from_card(&discard_two, alice, Zone::Hand);

    if let Some(player) = game.player_mut(alice) {
        player.mana_pool.add(ManaSymbol::Black, 2);
    }

    let proliferate_ability_index = game
        .object(yawgmoth_id)
        .expect("Yawgmoth should exist")
        .abilities
        .iter()
        .position(|ability| {
            if let AbilityKind::Activated(activated) = &ability.kind {
                activated.mana_cost.mana_cost().is_some()
                    && activated
                        .mana_cost
                        .costs()
                        .iter()
                        .any(|cost| cost.is_discard())
            } else {
                false
            }
        })
        .expect("Yawgmoth should have proliferate ability with discard cost");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;

    let activate = PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
        source: yawgmoth_id,
        ability_index: proliferate_ability_index,
    });
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &activate,
        &mut dm,
    )
    .expect("activation should start");

    let next_cost_ctx = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!(
            "expected next-cost chooser for proliferate activation, got {:?}",
            other
        ),
    };

    assert!(
        next_cost_ctx
            .description
            .to_lowercase()
            .contains("choose the next cost to pay"),
        "expected next-cost prompt, got description: {}",
        next_cost_ctx.description
    );

    let choose_discard_cost = PriorityResponse::NextCostChoice(1);
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &choose_discard_cost,
        &mut dm,
    )
    .expect("discard cost should be selectable first");

    let objects_ctx = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(ctx),
        ) => ctx,
        other => panic!(
            "expected SelectObjects discard decision after choosing discard cost, got {:?}",
            other
        ),
    };

    assert!(
        objects_ctx.description.to_lowercase().contains("discard"),
        "discard cost activation should prompt discard selection, got description: {}",
        objects_ctx.description
    );
    assert_eq!(objects_ctx.min, 1);
    assert_eq!(objects_ctx.max, Some(1));
    let candidate_ids: Vec<ObjectId> = objects_ctx.candidates.iter().map(|c| c.id).collect();
    assert!(
        candidate_ids.contains(&hand_card_one),
        "first hand card should be selectable for discard cost"
    );
    assert!(
        candidate_ids.contains(&hand_card_two),
        "second hand card should be selectable for discard cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_yawgmoth_proliferate_activation_is_legal_with_black_lotus_and_discard_card() {
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let registry =
        crate::cards::CardRegistry::with_builtin_cards_for_names(["Yawgmoth, Thran Physician"]);
    let yawgmoth_def = registry
        .get("Yawgmoth, Thran Physician")
        .expect("Yawgmoth, Thran Physician should be present in registry");
    let yawgmoth_id = game.create_object_from_definition(yawgmoth_def, alice, Zone::Battlefield);

    let discard_card = CardBuilder::new(CardId::new(), "Discard Me")
        .card_types(vec![CardType::Instant])
        .build();
    game.create_object_from_card(&discard_card, alice, Zone::Hand);

    let lotus_def = CardDefinitionBuilder::new(CardId::new(), "Black Lotus")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}, Sacrifice this artifact: Add three mana of any one color.")
        .expect("Black Lotus text should parse");
    game.create_object_from_definition(&lotus_def, alice, Zone::Battlefield);

    let proliferate_ability_index = game
        .object(yawgmoth_id)
        .expect("Yawgmoth should exist")
        .abilities
        .iter()
        .position(|ability| {
            if let AbilityKind::Activated(activated) = &ability.kind {
                activated.mana_cost.mana_cost().is_some()
                    && activated
                        .mana_cost
                        .costs()
                        .iter()
                        .any(|cost| cost.is_discard())
            } else {
                false
            }
        })
        .expect("Yawgmoth should have proliferate ability with discard cost");

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index }
                    if *source == yawgmoth_id && *ability_index == proliferate_ability_index
            )
        }),
        "Yawgmoth's proliferate ability should be legal with Black Lotus on the battlefield and a discardable card in hand"
    );
}

#[test]
pub(super) fn test_cleanup_discard_no_decision_when_under_limit() {
    use crate::turn::get_cleanup_discard_spec;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;

    // Add only 5 cards to hand (under max hand size of 7)
    for i in 0..5 {
        let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_card(&card, alice, Zone::Hand);
    }

    // Get the discard spec - should be None
    let spec = get_cleanup_discard_spec(&game);
    assert!(
        spec.is_none(),
        "Should not require discard when under hand limit"
    );
}

#[test]
pub(super) fn test_legend_rule_no_decision_when_different_names() {
    use crate::rules::state_based::get_legend_rule_specs;
    use crate::types::Supertype;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create two legendary creatures with DIFFERENT names
    let legend1_card = CardBuilder::new(CardId::from_raw(1), "Isamaru")
        .supertypes(vec![crate::types::Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();

    let legend2_card = CardBuilder::new(CardId::from_raw(2), "Ragavan")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 1))
        .build();

    game.create_object_from_card(&legend1_card, alice, Zone::Battlefield);
    game.create_object_from_card(&legend2_card, alice, Zone::Battlefield);

    // Get legend rule specs - should be empty (different names)
    let specs = get_legend_rule_specs(&game);
    assert!(
        specs.is_empty(),
        "Should not have legend rule specs for different legendary names"
    );
}

// ============================================================================
// Game Loop Integration Tests for Legend Rule and Cleanup Discard
// ============================================================================

/// Custom decision maker for testing legend rule choices
pub(super) struct LegendRuleDecisionMaker {
    /// Which legend to keep (index into the legends list)
    pub(super) keep_index: usize,
    /// Record of decisions made
    pub(super) decisions_made: Vec<String>,
}

impl LegendRuleDecisionMaker {
    pub(super) fn new(keep_index: usize) -> Self {
        Self {
            keep_index,
            decisions_made: Vec::new(),
        }
    }
}

impl crate::decision::DecisionMaker for LegendRuleDecisionMaker {
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        // Record that a legend rule decision was made
        self.decisions_made.push(format!(
            "Legend rule for '{}' with {} options",
            ctx.description,
            ctx.candidates.len()
        ));
        // Return the legend to keep based on index
        let legal_candidates: Vec<ObjectId> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .collect();
        let keep_id = legal_candidates
            .get(
                self.keep_index
                    .min(legal_candidates.len().saturating_sub(1)),
            )
            .copied()
            .unwrap_or_else(|| ctx.candidates[0].id);
        vec![keep_id]
    }
}

#[test]
pub(super) fn test_legend_rule_via_game_loop() {
    use crate::triggers::TriggerQueue;
    use crate::types::Supertype;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create two legendary creatures with the same name
    let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();

    let legend1_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
    let legend2_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);

    // Verify both are on battlefield
    assert_eq!(game.battlefield.len(), 2);

    // Create a decision maker that chooses the SECOND legend to keep
    let mut dm = LegendRuleDecisionMaker::new(1);
    let mut trigger_queue = TriggerQueue::new();

    // Run SBAs through the game loop - this should prompt for legend rule choice
    let result = check_and_apply_sbas_with(&mut game, &mut trigger_queue, &mut dm);
    assert!(result.is_ok());

    // Verify the decision was made
    assert_eq!(dm.decisions_made.len(), 1);
    assert!(dm.decisions_made[0].contains("Isamaru"));

    // Verify only one legend remains on battlefield
    assert_eq!(
        game.battlefield.len(),
        1,
        "Should have one legend remaining"
    );

    // The SECOND legend should be the one kept (since we chose index 1)
    assert!(
        game.battlefield.contains(&legend2_id),
        "Second legend should be kept"
    );
    assert!(
        !game.battlefield.contains(&legend1_id),
        "First legend should be gone"
    );

    // First legend should be in graveyard
    assert_eq!(
        game.player(alice).unwrap().graveyard.len(),
        1,
        "One legend should be in graveyard"
    );
}

#[test]
pub(super) fn test_legend_rule_keeps_first_legend() {
    use crate::triggers::TriggerQueue;
    use crate::types::Supertype;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create three legendary creatures with the same name
    let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();

    let legend1_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
    let _legend2_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
    let _legend3_id = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);

    // Verify all three are on battlefield
    assert_eq!(game.battlefield.len(), 3);

    // Create a decision maker that chooses the FIRST legend to keep
    let mut dm = LegendRuleDecisionMaker::new(0);
    let mut trigger_queue = TriggerQueue::new();

    // Run SBAs through the game loop
    let result = check_and_apply_sbas_with(&mut game, &mut trigger_queue, &mut dm);
    assert!(result.is_ok());

    // Verify only one legend remains on battlefield
    assert_eq!(
        game.battlefield.len(),
        1,
        "Should have one legend remaining"
    );

    // The FIRST legend should be the one kept
    assert!(
        game.battlefield.contains(&legend1_id),
        "First legend should be kept"
    );

    // Two legends should be in graveyard
    assert_eq!(
        game.player(alice).unwrap().graveyard.len(),
        2,
        "Two legends should be in graveyard"
    );
}

/// Custom decision maker for testing cleanup discard choices
pub(super) struct CleanupDiscardDecisionMaker {
    /// Which card indices to discard (from the hand list)
    pub(super) discard_indices: Vec<usize>,
    /// Record of decisions made
    pub(super) decisions_made: Vec<String>,
}

impl CleanupDiscardDecisionMaker {
    pub(super) fn new(discard_indices: Vec<usize>) -> Self {
        Self {
            discard_indices,
            decisions_made: Vec::new(),
        }
    }
}

impl crate::decision::DecisionMaker for CleanupDiscardDecisionMaker {
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.decisions_made.push(format!(
            "Discard {} cards from hand of {}",
            ctx.min,
            ctx.candidates.len()
        ));
        // Select cards at the specified indices
        self.discard_indices
            .iter()
            .filter_map(|&idx| ctx.candidates.get(idx).map(|c| c.id))
            .take(ctx.min)
            .collect()
    }

    fn decide_priority(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        LegalAction::PassPriority
    }
}

#[test]
pub(super) fn test_cleanup_discard_via_game_loop() {
    use crate::decisions::make_decision;
    use crate::turn::get_cleanup_discard_spec;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;

    // Add 10 cards to hand (3 over max hand size of 7)
    let mut card_ids = Vec::new();
    for i in 0..10 {
        let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
            .card_types(vec![CardType::Sorcery])
            .build();
        let obj_id = game.create_object_from_card(&card, alice, Zone::Hand);
        card_ids.push(obj_id);
    }

    assert_eq!(game.player(alice).unwrap().hand.len(), 10);

    // Create a decision maker that discards the first 3 cards
    let mut dm = CleanupDiscardDecisionMaker::new(vec![0, 1, 2]);

    // Manually run cleanup discard decision flow
    if let Some((player, spec)) = get_cleanup_discard_spec(&game) {
        let cards: Vec<ObjectId> = make_decision(&game, &mut dm, player, None, spec);
        let mut auto_dm = crate::decision::AutoPassDecisionMaker;
        crate::turn::apply_cleanup_discard(&mut game, &cards, &mut auto_dm);
    }

    // Verify the decision was made
    assert_eq!(dm.decisions_made.len(), 1);
    assert!(dm.decisions_made[0].contains("Discard 3 cards"));

    // Verify hand size is now 7
    assert_eq!(
        game.player(alice).unwrap().hand.len(),
        7,
        "Hand should have 7 cards after discard"
    );

    // Verify graveyard has 3 cards
    assert_eq!(
        game.player(alice).unwrap().graveyard.len(),
        3,
        "Graveyard should have 3 discarded cards"
    );

    // Verify the specific cards that were discarded (first 3)
    let graveyard = &game.player(alice).unwrap().graveyard;
    // The cards get new IDs when moving zones, so we check by name
    let discarded_names: Vec<String> = graveyard
        .iter()
        .filter_map(|id| game.object(*id).map(|o| o.name.to_string()))
        .collect();

    // Cards 0, 1, 2 should be in graveyard
    assert!(
        discarded_names.contains(&"Card 0".to_string()),
        "Card 0 should be in graveyard"
    );
    assert!(
        discarded_names.contains(&"Card 1".to_string()),
        "Card 1 should be in graveyard"
    );
    assert!(
        discarded_names.contains(&"Card 2".to_string()),
        "Card 2 should be in graveyard"
    );
    let _ = card_ids; // Suppress unused warning
}

#[test]
pub(super) fn test_cleanup_discard_specific_card_choice() {
    use crate::decisions::make_decision;
    use crate::turn::get_cleanup_discard_spec;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;

    // Add 9 cards to hand (2 over max hand size of 7)
    for i in 0..9 {
        let card = CardBuilder::new(CardId::new(), &format!("Card {}", i))
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_card(&card, alice, Zone::Hand);
    }

    let initial_hand = game.player(alice).unwrap().hand.clone();
    assert_eq!(initial_hand.len(), 9);

    // Get the names of cards at indices 3 and 7 (the ones we'll discard)
    let card_3_name = game.object(initial_hand[3]).unwrap().name.to_string();
    let card_7_name = game.object(initial_hand[7]).unwrap().name.to_string();

    // Create a decision maker that discards cards at indices 3 and 7
    let mut dm = CleanupDiscardDecisionMaker::new(vec![3, 7]);

    // Run cleanup discard decision flow
    if let Some((player, spec)) = get_cleanup_discard_spec(&game) {
        let cards: Vec<ObjectId> = make_decision(&game, &mut dm, player, None, spec);
        let mut auto_dm = crate::decision::AutoPassDecisionMaker;
        crate::turn::apply_cleanup_discard(&mut game, &cards, &mut auto_dm);
    }

    // Verify hand size is now 7
    assert_eq!(game.player(alice).unwrap().hand.len(), 7);

    // Verify the correct cards were discarded by checking names in graveyard
    let graveyard_names: Vec<String> = game
        .player(alice)
        .unwrap()
        .graveyard
        .iter()
        .filter_map(|id| game.object(*id).map(|o| o.name.to_string()))
        .collect();

    assert!(
        graveyard_names.contains(&card_3_name),
        "Card at index 3 ({}) should be in graveyard",
        card_3_name
    );
    assert!(
        graveyard_names.contains(&card_7_name),
        "Card at index 7 ({}) should be in graveyard",
        card_7_name
    );

    // Verify those cards are NOT in hand anymore
    let hand_names: Vec<String> = game
        .player(alice)
        .unwrap()
        .hand
        .iter()
        .filter_map(|id| game.object(*id).map(|o| o.name.to_string()))
        .collect();

    assert!(
        !hand_names.contains(&card_3_name),
        "Card at index 3 should NOT be in hand"
    );
    assert!(
        !hand_names.contains(&card_7_name),
        "Card at index 7 should NOT be in hand"
    );
}

#[test]
pub(super) fn test_legend_rule_with_different_controllers() {
    use crate::triggers::TriggerQueue;
    use crate::types::Supertype;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    // Create the same legendary creature for two different players
    let legend_card = CardBuilder::new(CardId::from_raw(1), "Isamaru, Hound of Konda")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();

    let alice_legend = game.create_object_from_card(&legend_card, alice, Zone::Battlefield);
    let bob_legend = game.create_object_from_card(&legend_card, bob, Zone::Battlefield);

    // Verify both are on battlefield
    assert_eq!(game.battlefield.len(), 2);

    // Create a decision maker
    let mut dm = LegendRuleDecisionMaker::new(0);
    let mut trigger_queue = TriggerQueue::new();

    // Run SBAs - legend rule should NOT apply because they have different controllers
    let result = check_and_apply_sbas_with(&mut game, &mut trigger_queue, &mut dm);
    assert!(result.is_ok());

    // No legend rule decisions should have been made
    assert_eq!(
        dm.decisions_made.len(),
        0,
        "No legend rule decisions for different controllers"
    );

    // Both legends should still be on battlefield
    assert_eq!(game.battlefield.len(), 2);
    assert!(game.battlefield.contains(&alice_legend));
    assert!(game.battlefield.contains(&bob_legend));
}

// ============================================================================
// Flashback Tests
// ============================================================================

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_marang_river_prowler_not_castable_from_graveyard_without_black_or_green_permanent()
 {
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let prowler = CardDefinitionBuilder::new(CardId::from_raw(72_610), "Marang River Prowler")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Fish])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text(
            "Skulk (This creature can't be blocked by creatures with greater power.)\nYou may cast this card from your graveyard as long as you control a black or green permanent.",
        )
        .expect("Marang River Prowler should parse");
    let prowler_id = game.create_object_from_definition(&prowler, alice, Zone::Graveyard);

    let actions = compute_legal_actions(&game, alice);
    let graveyard_cast = actions.iter().find(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                ..
            } if *spell_id == prowler_id
        )
    });
    assert!(
        graveyard_cast.is_none(),
        "Marang River Prowler should not be castable from graveyard without a black or green permanent"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_marang_river_prowler_castable_from_graveyard_with_black_permanent() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let prowler = CardDefinitionBuilder::new(CardId::from_raw(72_611), "Marang River Prowler")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Fish])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text(
            "Skulk (This creature can't be blocked by creatures with greater power.)\nYou may cast this card from your graveyard as long as you control a black or green permanent.",
        )
        .expect("Marang River Prowler should parse");
    let prowler_id = game.create_object_from_definition(&prowler, alice, Zone::Graveyard);

    let black_permanent = CardBuilder::new(CardId::from_raw(72_612), "Black Permanent Probe")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Black]))
        .color_indicator(crate::color::ColorSet::BLACK)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&black_permanent, alice, Zone::Battlefield);

    let can_play_from_graveyard = game.effect_store.grant_registry.card_can_play_from_zone(
        &game,
        prowler_id,
        Zone::Graveyard,
        alice,
    );
    assert!(
        can_play_from_graveyard,
        "Marang River Prowler should grant play-from-graveyard when you control a black permanent"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_marang_river_prowler_castable_from_graveyard_with_green_permanent() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let prowler = CardDefinitionBuilder::new(CardId::from_raw(72_613), "Marang River Prowler")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Fish])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text(
            "Skulk (This creature can't be blocked by creatures with greater power.)\nYou may cast this card from your graveyard as long as you control a black or green permanent.",
        )
        .expect("Marang River Prowler should parse");
    let prowler_id = game.create_object_from_definition(&prowler, alice, Zone::Graveyard);

    let green_permanent = CardBuilder::new(CardId::from_raw(72_614), "Green Permanent Probe")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Green]))
        .color_indicator(crate::color::ColorSet::GREEN)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&green_permanent, alice, Zone::Battlefield);

    let can_play_from_graveyard = game.effect_store.grant_registry.card_can_play_from_zone(
        &game,
        prowler_id,
        Zone::Graveyard,
        alice,
    );
    assert!(
        can_play_from_graveyard,
        "Marang River Prowler should grant play-from-graveyard when you control a green permanent"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn squee_the_immortal_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_616), "Squee, the Immortal")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .supertypes(vec![Supertype::Legendary])
        .subtypes(vec![Subtype::Goblin])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text("You may cast this card from your graveyard or from exile.")
        .expect("Squee, the Immortal should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_squee_the_immortal_castable_from_graveyard() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 3);

    let squee = squee_the_immortal_definition();
    let squee_id = game.create_object_from_definition(&squee, alice, Zone::Graveyard);

    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            squee_id,
            Zone::Graveyard,
            alice,
        ),
        "Squee should grant permission to cast itself from graveyard"
    );
    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::PlayFrom { use_alternative: None, .. },
            } if *spell_id == squee_id
        )),
        "Squee should expose a normal-cost cast action from graveyard; got {actions:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_squee_the_immortal_castable_from_exile() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 3);

    let squee = squee_the_immortal_definition();
    let squee_id = game.create_object_from_definition(&squee, alice, Zone::Exile);

    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            squee_id,
            Zone::Exile,
            alice,
        ),
        "Squee should grant permission to cast itself from exile"
    );
    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Exile,
                casting_method: CastingMethod::PlayFrom { use_alternative: None, .. },
            } if *spell_id == squee_id
        )),
        "Squee should expose a normal-cost cast action from exile; got {actions:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_squee_the_immortal_not_castable_from_unlisted_zone() {
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 3);

    let squee = squee_the_immortal_definition();
    let squee_id = game.create_object_from_definition(&squee, alice, Zone::Library);

    assert!(
        !game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            squee_id,
            Zone::Library,
            alice,
        ),
        "Squee should not grant permission from library"
    );
    let actions = compute_legal_actions(&game, alice);
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell { spell_id, .. } if *spell_id == squee_id
        )),
        "Squee should not be castable from library; got {actions:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn hundred_battle_veteran_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_617), "Hundred-Battle Veteran")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(4, 2))
        .parse_text(
            "As long as there are three or more different kinds of counters among creatures you control, this creature gets +2/+4.\nYou may cast this card from your graveyard. If you do, it enters with a finality counter on it.",
        )
        .expect("Hundred-Battle Veteran should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn hundred_battle_veteran_counter_kind_threshold_buffs_only_at_three_kinds() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let veteran = hundred_battle_veteran_definition();
    let veteran_id = game.create_object_from_definition(&veteran, alice, Zone::Battlefield);
    let ally = create_creature(&mut game, "Counter Ally", alice, 1, 1);
    let other_ally = create_creature(&mut game, "Other Counter Ally", alice, 1, 1);

    game.add_counters(veteran_id, crate::object::CounterType::Finality, 1);
    game.add_counters(ally, crate::object::CounterType::PlusOnePlusOne, 1);
    let below = game
        .calculated_characteristics(veteran_id)
        .expect("Veteran characteristics below threshold");
    assert_eq!(below.power, Some(4));
    assert_eq!(below.toughness, Some(2));

    game.add_counters(other_ally, crate::object::CounterType::Stun, 1);
    let active = game
        .calculated_characteristics(veteran_id)
        .expect("Veteran characteristics at threshold");
    assert_eq!(active.power, Some(6));
    assert_eq!(active.toughness, Some(6));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn hundred_battle_veteran_graveyard_cast_enters_with_finality_counter_and_exiles_on_death()
 {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 3);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Black, 1);

    let veteran = hundred_battle_veteran_definition();
    let graveyard_id = game.create_object_from_definition(&veteran, alice, Zone::Graveyard);
    let cast_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::PlayFrom { use_alternative: None, .. },
                } if *spell_id == graveyard_id
            )
        })
        .expect("Hundred-Battle Veteran should be castable from graveyard");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
        &mut dm,
    )
    .expect("Hundred-Battle Veteran graveyard cast should complete");
    resolve_stack_entry(&mut game).expect("Hundred-Battle Veteran should resolve");

    let entered = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Hundred-Battle Veteran")
        })
        .expect("graveyard-cast Hundred-Battle Veteran should enter the battlefield");
    assert_eq!(
        game.counter_count(entered, crate::object::CounterType::Finality),
        1,
        "graveyard-cast Veteran should enter with one finality counter"
    );

    crate::events::processing::process_destroy(&mut game, entered, None, &mut dm);
    assert!(
        game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Hundred-Battle Veteran")
        }),
        "a Veteran with a finality counter should be exiled instead of dying"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn hundred_battle_veteran_normal_cast_does_not_get_finality_counter() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 3);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Black, 1);

    let veteran = hundred_battle_veteran_definition();
    let hand_id = game.create_object_from_definition(&veteran, alice, Zone::Hand);
    let cast_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == hand_id
            )
        })
        .expect("Hundred-Battle Veteran should be normally castable from hand");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
        &mut dm,
    )
    .expect("Hundred-Battle Veteran normal cast should complete");
    resolve_stack_entry(&mut game).expect("Hundred-Battle Veteran should resolve");

    let entered = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Hundred-Battle Veteran")
        })
        .expect("normal-cast Hundred-Battle Veteran should enter the battlefield");
    assert_eq!(
        game.counter_count(entered, crate::object::CounterType::Finality),
        0,
        "normal-cast Veteran should not enter with a finality counter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn eelectrocute_definition() -> crate::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(72_615), "Eelectrocute")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Eelectrocute deals 2 damage to any target.\nYou may cast this card from your graveyard as long as you've rolled a 6 this turn. If you cast it this way and it would be put into your graveyard, exile it instead.",
        )
        .expect("Eelectrocute should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_eelectrocute_not_castable_from_graveyard_without_rolled_six() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 2);
    game.turn_store.turn_history.record_die_roll(alice, 5);

    let eelectrocute = eelectrocute_definition();
    let eelectrocute_id = game.create_object_from_definition(&eelectrocute, alice, Zone::Graveyard);

    let actions = compute_legal_actions(&game, alice);
    let graveyard_cast = actions.iter().find(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::PlayFrom { use_alternative: Some(_), .. },
            } if *spell_id == eelectrocute_id
        )
    });
    assert!(
        graveyard_cast.is_none(),
        "Eelectrocute should not be castable from graveyard unless you rolled a 6 this turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_eelectrocute_cast_from_graveyard_after_rolled_six_exiles_after_resolution() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 2);
    game.turn_store.turn_history.record_die_roll(alice, 6);

    let eelectrocute = eelectrocute_definition();
    let eelectrocute_id = game.create_object_from_definition(&eelectrocute, alice, Zone::Graveyard);

    let cast_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Graveyard,
                    casting_method: CastingMethod::PlayFrom { use_alternative: Some(_), .. },
                } if *spell_id == eelectrocute_id
            )
        })
        .expect("Eelectrocute should be castable from graveyard after rolling a 6");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
        &mut dm,
    )
    .expect("Eelectrocute graveyard cast should start");
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Player(bob)]),
        &mut dm,
    )
    .expect("choosing player target should complete Eelectrocute cast");

    resolve_stack_entry(&mut game).expect("Eelectrocute should resolve");

    assert_eq!(
        game.player(bob).expect("Bob should exist").life,
        18,
        "Eelectrocute should deal 2 damage to the chosen target"
    );
    assert!(
        game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Eelectrocute")
        }),
        "Eelectrocute cast from the graveyard this way should be exiled after resolution"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_eelectrocute_normal_cast_goes_to_graveyard_not_exile() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 2);

    let eelectrocute = eelectrocute_definition();
    let eelectrocute_id = game.create_object_from_definition(&eelectrocute, alice, Zone::Hand);

    let cast_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *spell_id == eelectrocute_id
            )
        })
        .expect("Eelectrocute should be normally castable from hand");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(cast_action),
        &mut dm,
    )
    .expect("Eelectrocute hand cast should start");
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Player(bob)]),
        &mut dm,
    )
    .expect("choosing player target should complete normal Eelectrocute cast");

    resolve_stack_entry(&mut game).expect("Eelectrocute should resolve");

    let player = game.player(alice).expect("Alice should exist");
    assert!(
        player.graveyard.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Eelectrocute")
        }),
        "normally cast Eelectrocute should go to graveyard"
    );
    assert!(
        !game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Eelectrocute")
        }),
        "normally cast Eelectrocute should not be exiled"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_flashback_appears_in_legal_actions_from_graveyard() {
    use crate::cards::definitions::think_twice;
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase for sorcery-timing spells
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Add 3 blue mana directly (for flashback cost {2}{U})
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 3);

    // Create Think Twice IN GRAVEYARD
    let think_twice_def = think_twice();
    let think_twice_id =
        game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should find a CastSpell action for Think Twice with Alternative casting method
    let flashback_action = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::Alternative(0),
            } if *spell_id == think_twice_id
        )
    });

    assert!(
        flashback_action.is_some(),
        "Should be able to cast Think Twice with flashback from graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_flashback_not_available_from_hand() {
    use crate::cards::definitions::think_twice;
    use crate::decision::compute_legal_actions;
    use crate::mana::ManaSymbol;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Add 3 blue mana directly
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 3);

    // Create Think Twice IN HAND
    let think_twice_def = think_twice();
    let think_twice_id = game.create_object_from_definition(&think_twice_def, alice, Zone::Hand);

    // Compute legal actions
    let actions = compute_legal_actions(&game, alice);

    // Should find a CastSpell action for Think Twice from hand with Normal casting
    let normal_cast = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *spell_id == think_twice_id
        )
    });

    assert!(
        normal_cast.is_some(),
        "Should be able to cast Think Twice normally from hand"
    );

    // Should NOT find flashback from hand
    let flashback_from_hand = actions.iter().find(|a| {
        matches!(
            a,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(_),
            } if *spell_id == think_twice_id
        )
    });

    assert!(
        flashback_from_hand.is_none(),
        "Should NOT be able to use flashback from hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_flashback_exiles_after_resolution() {
    use crate::cards::definitions::think_twice;
    use crate::mana::ManaSymbol;
    use crate::triggers::TriggerQueue;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Set up main phase
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    // Add 3 blue mana directly
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 3);

    // Add a card to alice's library so draw can succeed
    use crate::cards::definitions::basic_island;
    let island_def = basic_island();
    let _library_card = game.create_object_from_definition(&island_def, alice, Zone::Library);

    // Create Think Twice in graveyard
    let think_twice_def = think_twice();
    let think_twice_id =
        game.create_object_from_definition(&think_twice_def, alice, Zone::Graveyard);

    // Record initial hand size
    let initial_hand_size = game.player(alice).unwrap().hand.len();

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

    // Spell should be on stack now
    assert_eq!(game.stack.len(), 1, "Spell should be on stack");
    let stack_entry = &game.stack[0];
    assert_eq!(
        stack_entry.casting_method,
        CastingMethod::Alternative(0),
        "Stack entry should record flashback casting method"
    );

    // Resolve the spell
    resolve_stack_entry(&mut game).expect("Resolution should succeed");

    // Verify draw happened
    let final_hand_size = game.player(alice).unwrap().hand.len();
    assert_eq!(
        final_hand_size,
        initial_hand_size + 1,
        "Should have drawn 1 card"
    );

    // Verify spell is in exile (not graveyard)
    let player = game.player(alice).unwrap();
    let in_graveyard = player.graveyard.iter().any(|&id| {
        game.object(id)
            .map(|o| o.name == "Think Twice")
            .unwrap_or(false)
    });
    assert!(
        !in_graveyard,
        "Think Twice should NOT be in graveyard after flashback"
    );

    let in_exile = game.exile.iter().any(|&id| {
        game.object(id)
            .map(|o| o.name == "Think Twice")
            .unwrap_or(false)
    });
    assert!(in_exile, "Think Twice SHOULD be in exile after flashback");
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn increasing_confusion_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(262_860), "Increasing Confusion")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target player mills X cards. If this spell was cast from a graveyard, that player mills twice that many cards instead.\n\
             Flashback {X}{U}",
        )
        .expect("Increasing Confusion should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn add_filler_cards_to_library(game: &mut GameState, player: PlayerId, count: usize) {
    let filler = crate::cards::definitions::basic_island();
    for _ in 0..count {
        game.create_object_from_definition(&filler, player, Zone::Library);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_increasing_confusion_on_stack(
    game: &mut GameState,
    controller: PlayerId,
    target: PlayerId,
    x_value: u32,
    casting_method: CastingMethod,
) -> ObjectId {
    let def = increasing_confusion_definition();
    let (flashback_index, flashback) = def
        .alternative_casts
        .iter()
        .enumerate()
        .find(|method| {
            matches!(
                method.1,
                crate::alternative_cast::AlternativeCastingMethod::Flashback { .. }
            )
        })
        .expect("Increasing Confusion should expose flashback");
    assert_eq!(
        flashback_index, 0,
        "flashback should be first because this test uses Alternative(0)"
    );
    assert_eq!(
        flashback.cast_from_zone(),
        Zone::Graveyard,
        "Increasing Confusion flashback should cast from the graveyard"
    );
    assert!(
        flashback.mana_cost().is_some(),
        "Increasing Confusion flashback should have a mana cost"
    );
    let flashback_debug = format!("{flashback:?}");
    assert!(
        flashback_debug.contains("X") && flashback_debug.contains("Blue"),
        "Increasing Confusion flashback should preserve {{X}}{{U}}, got {flashback_debug}"
    );

    let spell_id = game.create_object_from_definition(&def, controller, Zone::Stack);
    game.object_mut(spell_id)
        .expect("Increasing Confusion on stack")
        .x_value = Some(x_value);
    game.push_to_stack(
        StackEntry::new(spell_id, controller)
            .with_x(x_value)
            .with_targets(vec![Target::Player(target)])
            .with_casting_method(casting_method),
    );
    spell_id
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn increasing_confusion_normal_cast_mills_only_x_cards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_filler_cards_to_library(&mut game, bob, 5);

    put_increasing_confusion_on_stack(&mut game, alice, bob, 3, CastingMethod::Normal);
    resolve_stack_entry(&mut game).expect("Increasing Confusion should resolve normally");

    assert_eq!(
        game.player(bob).expect("bob exists").graveyard.len(),
        3,
        "normal cast should mill exactly X cards"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        2,
        "normal cast should leave the un-milled cards in the target player's library"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn increasing_confusion_flashback_mills_twice_x_and_exiles_spell() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_filler_cards_to_library(&mut game, bob, 8);

    let spell_id =
        put_increasing_confusion_on_stack(&mut game, alice, bob, 3, CastingMethod::Alternative(0));
    let spell_stable = game
        .object(spell_id)
        .expect("Increasing Confusion on stack")
        .stable_id;
    resolve_stack_entry(&mut game).expect("Increasing Confusion should resolve with flashback");

    assert_eq!(
        game.player(bob).expect("bob exists").graveyard.len(),
        6,
        "flashback cast should mill twice X cards"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").library.len(),
        2,
        "flashback cast should leave only the cards not milled by twice X"
    );
    let resolved_spell = game
        .find_object_by_stable_id(spell_stable)
        .expect("resolved Increasing Confusion should still be tracked");
    assert!(
        game.exile.contains(&resolved_spell),
        "flashback cast should exile Increasing Confusion after resolution"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn the_sixth_doctor_copies_historic_spells_without_legendary_and_only_once_each_turn() {
    use crate::PriorityResponse;
    use crate::decision::LegalAction;
    use crate::game_loop::PriorityLoopState;
    use crate::zone::Zone;

    let mut game = setup_game();
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let alice = PlayerId::from_index(0);

    let doctor = CardDefinitionBuilder::new(CardId::from_raw(81_610), "The Sixth Doctor")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .subtypes(vec![crate::types::Subtype::Doctor])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Time Lord's Prerogative — Whenever you cast a historic spell, copy it, except the copy isn't legendary. This ability triggers only once each turn.",
        )
        .expect("The Sixth Doctor should parse");
    game.create_object_from_definition(&doctor, alice, Zone::Battlefield);

    let first_relic = CardBuilder::new(CardId::from_raw(81_611), "Legendary Relic")
        .card_types(vec![CardType::Artifact])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .build();
    let second_relic = CardBuilder::new(CardId::from_raw(81_612), "Second Relic")
        .card_types(vec![CardType::Artifact])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .build();
    let first_id = game.create_object_from_card(&first_relic, alice, Zone::Hand);
    let second_id = game.create_object_from_card(&second_relic, alice, Zone::Hand);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);

    let cast_first = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: first_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_first)
        .expect("first historic spell should cast");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("The Sixth Doctor trigger should stack");
    assert_eq!(
        game.stack.len(),
        2,
        "trigger should sit above the original spell"
    );

    resolve_stack_entry(&mut game).expect("The Sixth Doctor trigger should resolve");
    assert_eq!(
        game.stack.len(),
        2,
        "the copy should be added on top of the original spell"
    );
    let copy_id = game
        .stack
        .last()
        .expect("copy should be on stack")
        .object_id;
    let copy_obj = game.object(copy_id).expect("copy object should exist");
    assert!(
        !copy_obj
            .supertypes
            .contains(&crate::types::Supertype::Legendary),
        "the copied spell should lose legendary"
    );
    let original_id = game
        .stack
        .first()
        .expect("original spell should still be on stack")
        .object_id;
    assert!(
        game.object(original_id)
            .expect("original spell should still exist")
            .supertypes
            .contains(&crate::types::Supertype::Legendary),
        "the original spell should remain legendary"
    );

    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("copy and original should resolve");
    }
    let battlefield_relics: Vec<_> = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|obj| obj.name == "Legendary Relic")
        })
        .collect();
    assert_eq!(
        battlefield_relics.len(),
        2,
        "the original spell and its copy should both have resolved"
    );
    assert!(
        game.battlefield.iter().any(|&id| {
            game.object(id).is_some_and(|obj| {
                obj.name == "Legendary Relic"
                    && obj.supertypes.contains(&crate::types::Supertype::Legendary)
            })
        }),
        "the original legendary permanent should be on the battlefield"
    );
    assert!(
        game.battlefield.iter().any(|&id| {
            game.object(id).is_some_and(|obj| {
                obj.name == "Legendary Relic"
                    && !obj.supertypes.contains(&crate::types::Supertype::Legendary)
            })
        }),
        "the copied permanent should be nonlegendary on the battlefield"
    );

    let cast_second = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: second_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_second)
        .expect("second historic spell should cast");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("second cast should not create another trigger");
    assert_eq!(
        game.stack.len(),
        1,
        "The Sixth Doctor should only trigger once each turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_creeping_renaissance_returns_chosen_permanent_type_from_graveyard() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::cards::definitions::{basic_forest, grizzly_bears};
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::mana::{ManaCost, ManaSymbol};

    struct ChooseLandDecisionMaker;
    impl DecisionMaker for ChooseLandDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(
                ctx.description, "Choose a permanent type",
                "Creeping Renaissance should prompt for a permanent type at runtime"
            );
            ctx.options
                .iter()
                .find(|option| option.description.eq_ignore_ascii_case("land"))
                .map(|option| vec![option.index])
                .unwrap_or_else(|| vec![0])
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let def = CardDefinitionBuilder::new(CardId::new(), "Creeping Renaissance")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose a permanent type. Return all cards of the chosen type from your graveyard to your hand.\nFlashback {5}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)",
        )
        .expect("Creeping Renaissance should parse");
    let source_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    game.create_object_from_definition(&basic_forest(), alice, Zone::Graveyard);
    game.create_object_from_definition(&basic_forest(), alice, Zone::Graveyard);
    let bears_id = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Graveyard);

    let mut dm = ChooseLandDecisionMaker;
    let mut ctx = ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut dm);

    for effect in def.spell_effect.as_ref().expect("spell effects") {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Creeping Renaissance effect should resolve");
    }

    assert_eq!(
        game.chosen_card_type(source_id),
        Some(CardType::Land),
        "the spell should store the chosen permanent type on the source"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .filter(|&&id| game.object(id).is_some_and(|obj| obj.name == "Forest"))
            .count()
            == 2,
        "both Forests should return to hand when land is chosen"
    );
    assert!(
        game.player(alice)
            .expect("alice exists")
            .graveyard
            .iter()
            .any(|&id| {
                id == bears_id
                    && game
                        .object(id)
                        .is_some_and(|obj| obj.name == "Grizzly Bears")
            }),
        "non-land cards should stay in the graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_make_an_example_sacrifices_the_chosen_pile() {
    use crate::effects::ExecutionContext;

    struct ChooseFirstObjectDecisionMaker;

    impl DecisionMaker for ChooseFirstObjectDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            panic!("Make an Example should not use a boolean pile-choice prompt: {ctx:?}");
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(1)
                .collect()
        }

        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(ctx.description, "Choose a mode");
            assert!(
                ctx.options
                    .iter()
                    .any(|option| option.description == "Choose the separated pile"),
                "expected a named separated-pile choice, got {ctx:?}"
            );
            assert!(
                ctx.options.iter().any(|option| {
                    option.description == "Choose the separated pile"
                        && option
                            .related_object_ids
                            .as_ref()
                            .is_some_and(|object_ids| !object_ids.is_empty())
                }),
                "expected pile options to expose their related objects, got {ctx:?}"
            );
            vec![0]
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let make_an_example = CardDefinitionBuilder::new(CardId::new(), "Make an Example")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Each opponent separates the creatures they control into two piles. For each opponent, you choose one of their piles. Each opponent sacrifices the creatures in their chosen pile. (Piles can be empty.)",
        )
        .expect("Make an Example should parse");

    let source_id = game.create_object_from_definition(&make_an_example, alice, Zone::Hand);
    let pile_bear = CardBuilder::new(CardId::new(), "Pile Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let pile_bear_id = game.create_object_from_card(&pile_bear, bob, Zone::Battlefield);

    let spell_effects = make_an_example
        .spell_effect
        .as_ref()
        .expect("Make an Example should have spell effects");
    let mut dm = ChooseFirstObjectDecisionMaker;
    let mut ctx = ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut dm);

    for effect in spell_effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Make an Example effect should resolve");
    }

    assert!(
        game.player(bob)
            .expect("bob exists")
            .graveyard
            .iter()
            .any(|&id| { game.object(id).is_some_and(|obj| obj.name == "Pile Bear") }),
        "the chosen pile should be sacrificed"
    );
    assert!(
        !game.battlefield.contains(&pile_bear_id),
        "the chosen creature should leave the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_make_an_example_multiplayer_splits_all_piles_before_caster_choices() {
    use crate::effects::ExecutionContext;

    #[derive(Debug, PartialEq, Eq)]
    enum MakeAnExampleDecision {
        Split(PlayerId),
        Choose(PlayerId),
    }

    struct RecordingPileDecisionMaker {
        calls: Vec<MakeAnExampleDecision>,
    }

    impl DecisionMaker for RecordingPileDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            panic!("Make an Example should not use a boolean pile-choice prompt: {ctx:?}");
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.calls.push(MakeAnExampleDecision::Split(ctx.player));
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(1)
                .collect()
        }

        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.calls.push(MakeAnExampleDecision::Choose(ctx.player));
            assert_eq!(ctx.description, "Choose a mode");
            assert!(
                ctx.options
                    .iter()
                    .any(|option| option.description == "Choose the separated pile"),
                "expected a named separated-pile choice, got {ctx:?}"
            );
            assert!(
                ctx.options.iter().any(|option| {
                    option.description == "Choose the separated pile"
                        && option
                            .related_object_ids
                            .as_ref()
                            .is_some_and(|object_ids| !object_ids.is_empty())
                }),
                "expected pile options to expose their related objects, got {ctx:?}"
            );
            vec![0]
        }
    }

    let mut game = GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let make_an_example = CardDefinitionBuilder::new(CardId::new(), "Make an Example")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Each opponent separates the creatures they control into two piles. For each opponent, you choose one of their piles. Each opponent sacrifices the creatures in their chosen pile. (Piles can be empty.)",
        )
        .expect("Make an Example should parse");

    let source_id = game.create_object_from_definition(&make_an_example, alice, Zone::Hand);
    for (controller, name) in [
        (bob, "Bob's First Creature"),
        (bob, "Bob's Second Creature"),
        (charlie, "Charlie's First Creature"),
        (charlie, "Charlie's Second Creature"),
    ] {
        let creature = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&creature, controller, Zone::Battlefield);
    }

    let spell_effects = make_an_example
        .spell_effect
        .as_ref()
        .expect("Make an Example should have spell effects");
    let mut dm = RecordingPileDecisionMaker { calls: Vec::new() };
    let mut ctx = ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut dm);

    for effect in spell_effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Make an Example effect should resolve");
    }

    assert_eq!(
        dm.calls,
        vec![
            MakeAnExampleDecision::Split(bob),
            MakeAnExampleDecision::Split(charlie),
            MakeAnExampleDecision::Choose(alice),
            MakeAnExampleDecision::Choose(alice),
        ],
        "all opponents should separate their piles before the caster chooses any pile"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_split_the_spoils_opponent_can_take_the_split_pile_into_hand() {
    use crate::decision::DecisionMaker;
    use crate::effects::{ExecutionContext, execute_effect};

    struct SplitTheSpoilsDecisionMaker {
        caster: PlayerId,
        opponent: PlayerId,
        split_names: &'static [&'static str],
        choose_split_pile: bool,
    }

    impl DecisionMaker for SplitTheSpoilsDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.choose_split_pile
        }

        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let legal = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .collect::<Vec<_>>();

            if ctx.player == self.caster {
                return self
                    .split_names
                    .iter()
                    .map(|wanted_name| {
                        legal
                            .iter()
                            .find(|candidate| {
                                game.object(candidate.id)
                                    .is_some_and(|object| object.name == *wanted_name)
                            })
                            .map(|candidate| candidate.id)
                            .unwrap_or_else(|| {
                                panic!("expected to find {wanted_name} in the split")
                            })
                    })
                    .collect();
            }

            assert_eq!(
                ctx.player, self.opponent,
                "only the opponent should make the final pile-selection decision"
            );
            Vec::new()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let split_the_spoils = CardDefinitionBuilder::new(CardId::from_raw(91_100), "Split the Spoils")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile up to five target permanent cards from your graveyard and separate them into two piles. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard.",
        )
        .expect("Split the Spoils should parse");

    let source_id = game.create_object_from_definition(&split_the_spoils, alice, Zone::Stack);
    let alpha = CardBuilder::new(CardId::from_raw(91_110), "Spoils Alpha")
        .card_types(vec![CardType::Artifact])
        .build();
    let beta = CardBuilder::new(CardId::from_raw(91_111), "Spoils Beta")
        .card_types(vec![CardType::Enchantment])
        .build();
    let gamma = CardBuilder::new(CardId::from_raw(91_112), "Spoils Gamma")
        .card_types(vec![CardType::Land])
        .build();
    let alpha_id = game.create_object_from_card(&alpha, alice, Zone::Graveyard);
    let beta_id = game.create_object_from_card(&beta, alice, Zone::Graveyard);
    let gamma_id = game.create_object_from_card(&gamma, alice, Zone::Graveyard);

    let spell_effects = split_the_spoils
        .spell_effect
        .as_ref()
        .expect("Split the Spoils should have spell effects");
    let mut dm = SplitTheSpoilsDecisionMaker {
        caster: alice,
        opponent: bob,
        split_names: &["Spoils Alpha", "Spoils Beta"],
        choose_split_pile: true,
    };
    let mut ctx = ExecutionContext::new_default(source_id, alice)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(alpha_id),
            crate::effects::ResolvedTarget::Object(beta_id),
            crate::effects::ResolvedTarget::Object(gamma_id),
        ])
        .with_decision_maker(&mut dm);

    for effect in spell_effects {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Split the Spoils effect should resolve");
    }

    let alice_hand = game.player(alice).expect("alice exists").hand.clone();
    let alice_graveyard = game.player(alice).expect("alice exists").graveyard.clone();
    assert!(
        alice_hand.iter().any(|&id| game
            .object(id)
            .is_some_and(|obj| obj.name == "Spoils Alpha"))
            && alice_hand
                .iter()
                .any(|&id| game.object(id).is_some_and(|obj| obj.name == "Spoils Beta")),
        "the split pile should move into hand when the opponent chooses it"
    );
    assert!(
        alice_graveyard.iter().any(|&id| game
            .object(id)
            .is_some_and(|obj| obj.name == "Spoils Gamma")),
        "the complement pile should return to the graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_split_the_spoils_opponent_can_take_the_other_pile_into_hand() {
    use crate::decision::DecisionMaker;
    use crate::effects::{ExecutionContext, execute_effect};

    struct SplitTheSpoilsDecisionMaker {
        caster: PlayerId,
        opponent: PlayerId,
        split_names: &'static [&'static str],
        choose_split_pile: bool,
    }

    impl DecisionMaker for SplitTheSpoilsDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.choose_split_pile
        }

        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let legal = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .collect::<Vec<_>>();

            if ctx.player == self.caster {
                return self
                    .split_names
                    .iter()
                    .map(|wanted_name| {
                        legal
                            .iter()
                            .find(|candidate| {
                                game.object(candidate.id)
                                    .is_some_and(|object| object.name == *wanted_name)
                            })
                            .map(|candidate| candidate.id)
                            .unwrap_or_else(|| {
                                panic!("expected to find {wanted_name} in the split")
                            })
                    })
                    .collect();
            }

            assert_eq!(
                ctx.player, self.opponent,
                "only the opponent should make the final pile-selection decision"
            );
            Vec::new()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let split_the_spoils = CardDefinitionBuilder::new(CardId::from_raw(91_101), "Split the Spoils")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile up to five target permanent cards from your graveyard and separate them into two piles. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard.",
        )
        .expect("Split the Spoils should parse");

    let source_id = game.create_object_from_definition(&split_the_spoils, alice, Zone::Stack);
    let alpha = CardBuilder::new(CardId::from_raw(91_120), "Spoils Delta")
        .card_types(vec![CardType::Artifact])
        .build();
    let beta = CardBuilder::new(CardId::from_raw(91_121), "Spoils Epsilon")
        .card_types(vec![CardType::Enchantment])
        .build();
    let gamma = CardBuilder::new(CardId::from_raw(91_122), "Spoils Zeta")
        .card_types(vec![CardType::Land])
        .build();
    let alpha_id = game.create_object_from_card(&alpha, alice, Zone::Graveyard);
    let beta_id = game.create_object_from_card(&beta, alice, Zone::Graveyard);
    let gamma_id = game.create_object_from_card(&gamma, alice, Zone::Graveyard);

    let spell_effects = split_the_spoils
        .spell_effect
        .as_ref()
        .expect("Split the Spoils should have spell effects");
    let mut dm = SplitTheSpoilsDecisionMaker {
        caster: alice,
        opponent: bob,
        split_names: &["Spoils Delta", "Spoils Epsilon"],
        choose_split_pile: false,
    };
    let mut ctx = ExecutionContext::new_default(source_id, alice)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(alpha_id),
            crate::effects::ResolvedTarget::Object(beta_id),
            crate::effects::ResolvedTarget::Object(gamma_id),
        ])
        .with_decision_maker(&mut dm);

    for effect in spell_effects {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Split the Spoils effect should resolve");
    }

    let alice_hand = game.player(alice).expect("alice exists").hand.clone();
    let alice_graveyard = game.player(alice).expect("alice exists").graveyard.clone();
    assert!(
        alice_hand
            .iter()
            .any(|&id| game.object(id).is_some_and(|obj| obj.name == "Spoils Zeta")),
        "the complement pile should move into hand when the opponent declines the split pile"
    );
    assert!(
        alice_graveyard.iter().any(|&id| game
            .object(id)
            .is_some_and(|obj| obj.name == "Spoils Delta"))
            && alice_graveyard.iter().any(|&id| game
                .object(id)
                .is_some_and(|obj| obj.name == "Spoils Epsilon")),
        "the original split pile should go back to the graveyard when the other pile is chosen"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn unesh_criosphinx_sovereign_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(91_130), "Unesh, Criosphinx Sovereign")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Sphinx])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Flying\n\
             Sphinx spells you cast cost {2} less to cast.\n\
             Whenever Unesh or another Sphinx you control enters, reveal the top four cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard.",
        )
        .expect("Unesh, Criosphinx Sovereign should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn unesh_library_card(id: u32, name: &str) -> crate::card::Card {
    CardBuilder::new(CardId::from_raw(id), name)
        .card_types(vec![CardType::Sorcery])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct UneshPileDecisionMaker {
    pub(super) opponent: PlayerId,
    pub(super) split_names: &'static [&'static str],
    pub(super) choose_split_pile: bool,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for UneshPileDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.choose_split_pile
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        assert_eq!(
            ctx.player, self.opponent,
            "only the opponent should separate Unesh's revealed cards into a pile"
        );
        let legal = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .collect::<Vec<_>>();
        self.split_names
            .iter()
            .map(|wanted_name| {
                legal
                    .iter()
                    .find(|candidate| {
                        game.object(candidate.id)
                            .is_some_and(|object| object.name == *wanted_name)
                    })
                    .map(|candidate| candidate.id)
                    .unwrap_or_else(|| panic!("expected to find {wanted_name} in Unesh's pile"))
            })
            .collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn execute_unesh_trigger_with_pile_choice(
    choose_split_pile: bool,
) -> (GameState, Vec<&'static str>, Vec<&'static str>) {
    use crate::effects::{ExecutionContext, execute_effect};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let unesh = unesh_criosphinx_sovereign_definition();
    let source_id = game.create_object_from_definition(&unesh, alice, Zone::Battlefield);
    let triggered = unesh
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Unesh should have an enters trigger");

    game.create_object_from_card(
        &unesh_library_card(91_131, "Unesh Bottom Card"),
        alice,
        Zone::Library,
    );
    for (id, name) in [
        (91_132, "Unesh Gamma"),
        (91_133, "Unesh Delta"),
        (91_134, "Unesh Beta"),
        (91_135, "Unesh Alpha"),
    ] {
        game.create_object_from_card(&unesh_library_card(id, name), alice, Zone::Library);
    }

    let mut dm = UneshPileDecisionMaker {
        opponent: bob,
        split_names: &["Unesh Alpha", "Unesh Beta"],
        choose_split_pile,
    };
    let entering_event = TriggerEvent::new_with_provenance(
        EnterBattlefieldEvent::new(source_id, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    let mut ctx =
        ExecutionContext::new(source_id, alice, &mut dm).with_triggering_event(entering_event);
    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Unesh trigger effect should resolve");
    }

    if choose_split_pile {
        (
            game,
            vec!["Unesh Gamma", "Unesh Delta"],
            vec!["Unesh Alpha", "Unesh Beta"],
        )
    } else {
        (
            game,
            vec!["Unesh Alpha", "Unesh Beta"],
            vec!["Unesh Gamma", "Unesh Delta"],
        )
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn unesh_criosphinx_sovereign_reduces_only_sphinx_spell_costs() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let unesh = unesh_criosphinx_sovereign_definition();
    game.create_object_from_definition(&unesh, alice, Zone::Battlefield);

    let sphinx_spell = CardDefinitionBuilder::new(CardId::from_raw(91_136), "Runtime Sphinx")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Sphinx])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let bear_spell = CardDefinitionBuilder::new(CardId::from_raw(91_137), "Runtime Bear")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let sphinx_id = game.create_object_from_definition(&sphinx_spell, alice, Zone::Hand);
    let bear_id = game.create_object_from_definition(&bear_spell, alice, Zone::Hand);

    let sphinx = game.object(sphinx_id).expect("sphinx spell exists");
    let sphinx_cost = crate::decision::calculate_effective_mana_cost(
        &game,
        alice,
        sphinx,
        sphinx.mana_cost.as_ref().expect("sphinx mana cost"),
    );
    assert_eq!(sphinx_cost.to_oracle(), "{2}");

    let bear = game.object(bear_id).expect("bear spell exists");
    let bear_cost = crate::decision::calculate_effective_mana_cost(
        &game,
        alice,
        bear,
        bear.mana_cost.as_ref().expect("bear mana cost"),
    );
    assert_eq!(bear_cost.to_oracle(), "{4}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn unesh_criosphinx_sovereign_trigger_matches_self_and_other_sphinx_only() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let unesh = unesh_criosphinx_sovereign_definition();
    let unesh_id = game.create_object_from_definition(&unesh, alice, Zone::Battlefield);
    let other_sphinx = CardDefinitionBuilder::new(CardId::from_raw(91_138), "Other Runtime Sphinx")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Sphinx])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let other_sphinx_id =
        game.create_object_from_definition(&other_sphinx, alice, Zone::Battlefield);
    let bear = CardDefinitionBuilder::new(CardId::from_raw(91_139), "Other Runtime Bear")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let bear_id = game.create_object_from_definition(&bear, alice, Zone::Battlefield);

    let entering_event = |object_id| {
        crate::events::RawEvent::new(
            crate::events::ZoneChangeEvent::with_cause(
                object_id,
                Zone::Stack,
                Zone::Battlefield,
                crate::events::cause::EventCause::from_game_rule(),
                None,
            ),
            crate::provenance::ProvNodeId::default(),
        )
    };

    let self_triggers = crate::triggers::check_triggers(&game, &entering_event(unesh_id));
    assert!(
        self_triggers
            .iter()
            .any(|trigger| trigger.source == unesh_id),
        "Unesh should trigger when it enters"
    );
    let sphinx_triggers = crate::triggers::check_triggers(&game, &entering_event(other_sphinx_id));
    assert!(
        sphinx_triggers
            .iter()
            .any(|trigger| trigger.source == unesh_id),
        "Unesh should trigger when another Sphinx you control enters"
    );
    let bear_triggers = crate::triggers::check_triggers(&game, &entering_event(bear_id));
    assert!(
        !bear_triggers
            .iter()
            .any(|trigger| trigger.source == unesh_id),
        "Unesh should not trigger for a non-Sphinx entering"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn unesh_criosphinx_sovereign_puts_opponent_split_pile_into_hand() {
    let (game, expected_hand, expected_graveyard) = execute_unesh_trigger_with_pile_choice(false);
    let alice = PlayerId::from_index(0);
    let hand = game.player(alice).expect("alice exists").hand.clone();
    let graveyard = game.player(alice).expect("alice exists").graveyard.clone();

    for name in expected_hand {
        assert!(
            hand.iter()
                .any(|&id| game.object(id).is_some_and(|object| object.name == name)),
            "{name} should be in hand when the caster chooses the opponent-created pile"
        );
    }
    for name in expected_graveyard {
        assert!(
            graveyard
                .iter()
                .any(|&id| game.object(id).is_some_and(|object| object.name == name)),
            "{name} should be in graveyard as part of the other pile"
        );
    }
    assert!(
        game.player(alice)
            .expect("alice exists")
            .library
            .iter()
            .any(|&id| game
                .object(id)
                .is_some_and(|object| object.name == "Unesh Bottom Card")),
        "Unesh should reveal only the top four cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn unesh_criosphinx_sovereign_can_choose_the_other_pile_for_hand() {
    let (game, expected_hand, expected_graveyard) = execute_unesh_trigger_with_pile_choice(true);
    let alice = PlayerId::from_index(0);
    let hand = game.player(alice).expect("alice exists").hand.clone();
    let graveyard = game.player(alice).expect("alice exists").graveyard.clone();

    for name in expected_hand {
        assert!(
            hand.iter()
                .any(|&id| game.object(id).is_some_and(|object| object.name == name)),
            "{name} should be in hand when the caster chooses the other pile"
        );
    }
    for name in expected_graveyard {
        assert!(
            graveyard
                .iter()
                .any(|&id| game.object(id).is_some_and(|object| object.name == name)),
            "{name} should be in graveyard when the caster declines the opponent-created pile"
        );
    }
}

#[test]
pub(super) fn test_dash_grants_haste_and_returns_to_hand_at_next_end_step() {
    use crate::cards::CardDefinitionBuilder;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::triggers::TriggerQueue;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 2);

    let dash_def = CardDefinitionBuilder::new(CardId::new(), "Dash Runtime Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 1))
        .dash(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .build();
    let dash_id = game.create_object_from_definition(&dash_def, alice, Zone::Hand);

    let mut state = PriorityLoopState::new(2);
    let mut trigger_queue = TriggerQueue::new();
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: dash_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Alternative(0),
    });
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
        .expect("dash cast should succeed");
    resolve_stack_entry(&mut game).expect("dash spell should resolve");

    let dashed_id = *game
        .battlefield
        .iter()
        .find(|&&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Dash Runtime Probe")
        })
        .expect("dashed creature should be on battlefield");
    let dashed_obj = game
        .object(dashed_id)
        .expect("dashed creature should exist");
    assert!(
        crate::rules::combat::can_attack(dashed_obj, &game),
        "dashed creature should be able to attack immediately"
    );
    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        1,
        "dash should schedule a next end-step return trigger"
    );

    let end_step_event = TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(game.turn.active_player),
        crate::provenance::ProvNodeId::default(),
    );
    for trigger in crate::triggers::check_delayed_triggers(&mut game, &end_step_event) {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("put dash delayed trigger on stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("resolve dash delayed trigger");
    }

    assert!(
        game.player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .any(|&id| game
                .object(id)
                .is_some_and(|obj| obj.name == "Dash Runtime Probe")),
        "dashed creature should return to hand at the next end step"
    );
}

#[test]
pub(super) fn test_copied_blitz_creature_spell_schedules_blitz_delayed_triggers() {
    use crate::cards::CardDefinitionBuilder;
    use crate::effects::{CopySpellEffect, EffectExecutor, ExecutionContext};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::triggers::TriggerQueue;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Green, 4);

    let blitz_def = CardDefinitionBuilder::new(CardId::new(), "Blitz Runtime Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 1))
        .blitz(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
        ]))
        .build();
    let blitz_id = game.create_object_from_definition(&blitz_def, alice, Zone::Hand);

    let mut state = PriorityLoopState::new(2);
    let mut trigger_queue = TriggerQueue::new();
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: blitz_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Alternative(0),
    });
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
        .expect("blitz cast should succeed");

    let original_entry = game
        .stack
        .last()
        .expect("blitz spell should be on stack")
        .clone();
    CopySpellEffect::new(
        crate::target::ChooseSpec::SpecificObject(original_entry.object_id),
        1,
    )
    .execute(
        &mut game,
        &mut ExecutionContext::new_default(original_entry.object_id, alice)
            .with_casting_method(CastingMethod::Alternative(0)),
    )
    .expect("copying blitz spell should succeed");

    resolve_stack_entry(&mut game).expect("copied blitz spell should resolve");
    resolve_stack_entry(&mut game).expect("original blitz spell should resolve");

    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        4,
        "copy and original should each schedule dies-draw plus end-step sacrifice triggers"
    );

    let end_step_event = TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(game.turn.active_player),
        crate::provenance::ProvNodeId::default(),
    );
    for trigger in crate::triggers::check_delayed_triggers(&mut game, &end_step_event) {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("put blitz sacrifice triggers on stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("resolve blitz sacrifice trigger");
    }

    assert!(
        game.battlefield.iter().all(|&id| game
            .object(id)
            .is_none_or(|obj| obj.name != "Blitz Runtime Probe")),
        "both blitzed permanents should be sacrificed at the beginning of the end step"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn howl_of_the_horde_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(435_001), "Howl of the Horde")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "When you next cast an instant or sorcery spell this turn, copy that spell. You may choose new targets for the copy.\nRaid — If you attacked this turn, when you next cast an instant or sorcery spell this turn, copy that spell an additional time. You may choose new targets for the copy.",
        )
        .expect("Howl of the Horde should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn execute_howl_of_the_horde_spell_effect(game: &mut GameState, attacked: bool) {
    let alice = PlayerId::from_index(0);
    let howl = howl_of_the_horde_definition();
    let howl_id = game.create_object_from_definition(&howl, alice, Zone::Stack);
    if attacked {
        game.turn_store
            .turn_history
            .players_attacked_this_turn
            .insert(alice);
    }
    let mut ctx = crate::effects::ExecutionContext::new_default(howl_id, alice);
    execute_resolution_program(
        game,
        &mut ctx,
        alice,
        howl_id,
        howl.spell_effect.as_ref().expect("Howl spell effects"),
        None,
        &[],
    )
    .expect("Howl spell effect should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_howl_target_spell_on_stack(game: &mut GameState) -> ObjectId {
    let alice = PlayerId::from_index(0);
    let bolt = CardBuilder::new(CardId::from_raw(435_002), "Runtime Bolt")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Instant])
        .build();
    let spell_id = game.create_object_from_card(&bolt, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));
    spell_id
}

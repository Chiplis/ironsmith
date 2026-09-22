fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Flash of Insight",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{NumberContext, OrderContext, ViewCardsContext};
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

#[derive(Default)]
struct Choices {
    x: u32,
    looked: Vec<(PlayerId, Vec<String>)>,
    ordered: Vec<String>,
}
impl DecisionMaker for Choices {
    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value { self.x } else { ctx.min }
    }
    fn view_cards(
        &mut self,
        game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        _ctx: &ViewCardsContext,
    ) {
        self.looked.push((
            viewer,
            cards
                .iter()
                .map(|id| game.object(*id).unwrap().name.to_string())
                .collect(),
        ));
    }
    fn decide_order(&mut self, game: &GameState, ctx: &OrderContext) -> Vec<ObjectId> {
        let ids: Vec<_> = ctx.items.iter().rev().map(|(id, _)| *id).collect();
        self.ordered = ids
            .iter()
            .map(|id| game.object(*id).unwrap().name.to_string())
            .collect();
        ids
    }
}
fn probe(name: &str, color: ManaSymbol) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![color]]))
        .build()
}

#[test]
fn actual_cast_pays_selected_x_and_flashback_exiles_only_own_blue_cards() {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for zone in [Zone::Hand, Zone::Graveyard] {
        for x in [0, 1, 3] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = bob;
            game.turn.priority_player = Some(alice);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Blue, 10);
            let spell = game.create_object_from_definition(&def, alice, zone);
            let spell_stable = game.object(spell).unwrap().stable_id;
            let mut blue = Vec::new();
            for i in 0..4 {
                let id = game.create_object_from_definition(
                    &probe(&format!("Blue {i}"), ManaSymbol::Blue),
                    alice,
                    Zone::Graveyard,
                );
                blue.push(game.object(id).unwrap().stable_id);
            }
            let red = game.create_object_from_definition(
                &probe("Red", ManaSymbol::Red),
                alice,
                Zone::Graveyard,
            );
            let opponent = game.create_object_from_definition(
                &probe("Opponent blue", ManaSymbol::Blue),
                bob,
                Zone::Graveyard,
            );
            for i in 0..5 {
                game.create_object_from_definition(
                    &probe(&format!("Library {i}"), ManaSymbol::Red),
                    alice,
                    Zone::Library,
                );
            }
            let before: Vec<_> = game
                .player(alice)
                .unwrap()
                .library
                .iter()
                .map(|id| game.object(*id).unwrap().name.to_string())
                .collect();
            let action = compute_legal_actions(&game, alice)
                .into_iter()
                .find(
                    |a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell),
                )
                .expect("instant is castable on opponent's turn");
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut state = PriorityLoopState::new(game.players_in_game());
            let mut dm = Choices {
                x,
                ..Default::default()
            };
            let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &PriorityResponse::PriorityAction(action),
                &mut dm,
            );
            for _ in 0..32 {
                if !game.stack.is_empty() || result.is_err() {
                    break;
                }
                let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
                    break;
                };
                result = if matches!(&ctx, ironsmith::decisions::context::DecisionContext::Number(n) if n.is_x_value)
                {
                    ironsmith::game_loop::apply_priority_response_with_dm(
                        &mut game,
                        &mut queue,
                        &mut state,
                        &PriorityResponse::XValue(x),
                        &mut dm,
                    )
                } else {
                    ironsmith::game_loop::apply_decision_context_with_dm(
                        &mut game, &mut queue, &mut state, &ctx, &mut dm,
                    )
                };
            }
            assert_eq!(game.stack.len(), 1, "zone={zone:?},x={x},result={result:?}");
            assert_eq!(game.stack[0].x_value, Some(x), "zone={zone:?}");
            assert_eq!(
                game.player(alice).unwrap().mana_pool.total(),
                10 - if zone == Zone::Hand { x + 2 } else { 2 }
            );
            assert_eq!(
                blue.iter()
                    .filter(|stable| game
                        .object(game.find_object_by_stable_id(**stable).unwrap())
                        .unwrap()
                        .zone
                        == Zone::Exile)
                    .count(),
                if zone == Zone::Graveyard {
                    x as usize
                } else {
                    0
                }
            );
            assert_eq!(game.object(red).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.object(opponent).unwrap().zone, Zone::Graveyard);
            assert_eq!(
                game.object(game.find_object_by_stable_id(spell_stable).unwrap())
                    .unwrap()
                    .zone,
                Zone::Stack
            );
            let mut countered = game.clone();
            let stack_id = countered.stack[0].object_id;
            let mut counter_ctx = ironsmith::effects::EffectContext::new_default(stack_id, bob);
            ironsmith::effects::execute_effect(
                &mut countered,
                &ironsmith::Effect::counter(ironsmith::target::ChooseSpec::SpecificObject(
                    stack_id,
                )),
                &mut counter_ctx,
            )
            .unwrap();
            assert!(countered.stack.is_empty());
            assert!(countered.player(alice).unwrap().hand.is_empty());
            assert_eq!(countered.player(alice).unwrap().library.len(), 5);
            assert_eq!(
                countered.player(alice).unwrap().mana_pool.total(),
                game.player(alice).unwrap().mana_pool.total()
            );
            assert_eq!(
                countered
                    .object(countered.find_object_by_stable_id(spell_stable).unwrap())
                    .unwrap()
                    .zone,
                if zone == Zone::Graveyard {
                    Zone::Exile
                } else {
                    Zone::Graveyard
                }
            );
            for stable in &blue {
                assert_eq!(
                    countered
                        .object(countered.find_object_by_stable_id(*stable).unwrap())
                        .unwrap()
                        .zone,
                    game.object(game.find_object_by_stable_id(*stable).unwrap())
                        .unwrap()
                        .zone
                );
            }
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            let expected_zone = if zone == Zone::Graveyard {
                Zone::Exile
            } else {
                Zone::Graveyard
            };
            assert_eq!(
                game.object(game.find_object_by_stable_id(spell_stable).unwrap())
                    .unwrap()
                    .zone,
                expected_zone
            );
            let hand = &game.player(alice).unwrap().hand;
            assert_eq!(hand.len(), usize::from(x > 0));
            assert_eq!(
                game.player(alice).unwrap().library.len(),
                5 - usize::from(x > 0)
            );
            if x > 0 {
                let selected = game.object(hand[0]).unwrap().name.to_string();
                assert!(before[5 - x as usize..].contains(&selected));
                assert!(dm.looked.iter().all(|(viewer, _)| *viewer == alice));
                assert!(dm.looked.iter().any(|(_, cards)| cards.len() == x as usize));
                let after: Vec<_> = game
                    .player(alice)
                    .unwrap()
                    .library
                    .iter()
                    .map(|id| game.object(*id).unwrap().name.to_string())
                    .collect();
                assert_eq!(&after[x as usize - 1..], &before[..5 - x as usize]);
                if x > 2 {
                    assert_eq!(&after[..x as usize - 1], dm.ordered.as_slice());
                }
            }
        }
    }
}

#[test]
fn resolving_with_fewer_than_x_cards_uses_only_available_cards() {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    for size in [0_usize, 1, 2] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        for i in 0..size {
            game.create_object_from_definition(
                &probe(&format!("Short library {i}"), ManaSymbol::Red),
                alice,
                Zone::Library,
            );
        }
        let spell = game.create_object_from_definition(&def, alice, Zone::Stack);
        let mut entry = ironsmith::game_state::StackEntry::new(spell, alice);
        entry.x_value = Some(5);
        game.push_to_stack(entry);
        let mut dm = Choices::default();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            usize::from(size > 0)
        );
        assert_eq!(
            game.player(alice).unwrap().library.len(),
            size.saturating_sub(1)
        );
        assert!(
            dm.looked
                .iter()
                .all(|(viewer, cards)| *viewer == alice && cards.len() == size)
        );
        if size > 0 {
            assert_eq!(dm.looked.len(), 1);
        }
    }
}

#[test]
fn flashback_x_cannot_be_paid_with_itself_or_ineligible_cards() {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for available in [0, 1] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.priority_player = Some(alice);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 10);
        let spell = game.create_object_from_definition(&def, alice, Zone::Graveyard);
        let stable = game.object(spell).unwrap().stable_id;
        for i in 0..available {
            game.create_object_from_definition(
                &probe(&format!("Eligible {i}"), ManaSymbol::Blue),
                alice,
                Zone::Graveyard,
            );
        }
        game.create_object_from_definition(&probe("Red", ManaSymbol::Red), alice, Zone::Graveyard);
        game.create_object_from_definition(
            &probe("Other blue", ManaSymbol::Blue),
            bob,
            Zone::Graveyard,
        );
        let action = compute_legal_actions(&game, alice)
            .into_iter()
            .find(|a| matches!(a, LegalAction::CastSpell {spell_id, ..} if *spell_id == spell))
            .expect("zero X remains legal");
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut dm = Choices {
            x: available + 1,
            ..Default::default()
        };
        let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        );
        for _ in 0..32 {
            if !game.stack.is_empty() || result.is_err() {
                break;
            }
            let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
                break;
            };
            result = if matches!(&ctx, ironsmith::decisions::context::DecisionContext::Number(n) if n.is_x_value)
            {
                ironsmith::game_loop::apply_priority_response_with_dm(
                    &mut game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::XValue(available + 1),
                    &mut dm,
                )
            } else {
                ironsmith::game_loop::apply_decision_context_with_dm(
                    &mut game, &mut queue, &mut state, &ctx, &mut dm,
                )
            };
        }
        assert!(
            game.stack.is_empty(),
            "cannot cast X={} with only {available} eligible other cards",
            available + 1
        );
        assert!(
            result.is_err(),
            "unpayable proposal must be rejected: {result:?}"
        );
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Graveyard
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 10);
    }
}

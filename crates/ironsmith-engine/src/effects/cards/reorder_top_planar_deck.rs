//! Look at and reorder the top of a planar deck.

#[cfg(test)]
use crate::decisions::context::ViewCardsContext;
use crate::decisions::context::{SelectObjectsContext, SelectableObject};
use crate::effect::EffectOutcome;
use crate::effects::helpers::{resolve_player_filter, resolve_player_filter_as_chooser};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::zone::Zone;
pub use ironsmith_core::ReorderTopPlanarDeckEffect;

fn normalize_single_selection(
    selected: Vec<ObjectId>,
    candidates_top_to_bottom: &[ObjectId],
) -> Option<ObjectId> {
    selected
        .into_iter()
        .find(|id| candidates_top_to_bottom.contains(id))
        .or_else(|| candidates_top_to_bottom.first().copied())
}

impl EffectExecutor for ReorderTopPlanarDeckEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player = resolve_player_filter(game, &self.player, ctx)?;
                let chooser = resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
                let count = self.count as usize;
                if count == 0 {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let top_to_bottom = game
                    .planar_deck(player)
                    .ok_or_else(|| {
                        ExecutionError::Impossible("Planechase is not enabled".to_string())
                    })?
                    .iter()
                    .rev()
                    .take(count)
                    .copied()
                    .collect::<Vec<_>>();
                if top_to_bottom.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let observation = super::look_at_cards_with_outputs(
                    game,
                    ctx,
                    chooser,
                    player,
                    Zone::Command,
                    &top_to_bottom,
                    "Look at cards from the top of a planar deck",
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let candidates = top_to_bottom
                    .iter()
                    .filter_map(|id| {
                        game.object(*id)
                            .map(|object| SelectableObject::new(*id, object.name.to_string()))
                    })
                    .collect::<Vec<_>>();
                if candidates.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::from_children(
                        [observation],
                        |_| EffectOutcome::count(0),
                    ));
                }
                let selection_context = SelectObjectsContext::new(
                    chooser,
                    Some(ctx.source),
                    "Choose a card to put on the bottom of your planar deck",
                    candidates,
                    1,
                    Some(1),
                );
                let selected = ctx.decision_maker.decide_objects(game, &selection_context);
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let Some(chosen) = normalize_single_selection(selected, &top_to_bottom) else {
                    return Ok(crate::effects::CompletedEffectOutputs::from_children(
                        [observation],
                        |_| EffectOutcome::count(0),
                    ));
                };

                game.move_planar_deck_card_to_bottom(player, chosen)
                    .map_err(ExecutionError::Impossible)?;

                Ok(crate::effects::CompletedEffectOutputs::from_children(
                    [observation],
                    |children| {
                        EffectOutcome::aggregate_with_primary_result(
                            EffectOutcome::resolved().with_affected_objects(vec![chosen]),
                            children,
                        )
                    },
                ))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::decision::DecisionMaker;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::{PlanarCardKind, PlayerFilter};

    struct ChooseNamed {
        name: String,
        viewed: Vec<ObjectId>,
    }

    impl DecisionMaker for ChooseNamed {
        fn view_cards(
            &mut self,
            _game: &GameState,
            _viewer: PlayerId,
            cards: &[ObjectId],
            _ctx: &ViewCardsContext,
        ) {
            self.viewed = cards.to_vec();
        }

        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter_map(|candidate| {
                    game.object(candidate.id)
                        .filter(|object| object.name == self.name)
                        .map(|_| candidate.id)
                })
                .collect()
        }
    }

    fn planar_card(id: u32, name: &str) -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Plane])
            .build()
    }

    #[test]
    fn looks_at_top_cards_and_moves_the_chosen_one_to_planar_bottom() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice_deck = (0..10)
            .map(|index| {
                (
                    planar_card(80_000 + index, &format!("Alice Plane {index}")),
                    PlanarCardKind::Plane,
                )
            })
            .collect();
        let bob_deck = (0..10)
            .map(|index| {
                (
                    planar_card(81_000 + index, &format!("Bob Plane {index}")),
                    PlanarCardKind::Plane,
                )
            })
            .collect();
        game.enable_planechase(vec![(alice, alice_deck), (bob, bob_deck)])
            .expect("valid planar decks");

        let top = game
            .planar_deck(alice)
            .expect("alice deck")
            .last()
            .copied()
            .expect("top card");
        let second = game.planar_deck(alice).expect("alice deck")
            [game.planar_deck(alice).expect("alice deck").len() - 2];
        let second_name = game.object(second).expect("second card").name.to_string();
        let source = game.new_object_id();
        let mut dm = ChooseNamed {
            name: second_name,
            viewed: Vec::new(),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        ReorderTopPlanarDeckEffect::new(PlayerFilter::You, PlayerFilter::You, 2)
            .execute(&mut game, &mut ctx)
            .expect("planar reorder resolves");

        assert_eq!(dm.viewed, vec![top, second]);
        let deck = game.planar_deck(alice).expect("alice deck");
        assert_eq!(deck.first().copied(), Some(second));
        assert_eq!(deck.last().copied(), Some(top));
    }

    #[test]
    fn reorders_the_shared_deck_in_communal_planechase() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let communal = (0..20)
            .map(|index| {
                (
                    planar_card(82_000 + index, &format!("Communal Plane {index}")),
                    PlanarCardKind::Plane,
                )
            })
            .collect();
        game.enable_planechase_communal(communal)
            .expect("valid communal planar deck");

        let deck = game.planar_deck(alice).expect("communal deck");
        let top = deck[deck.len() - 1];
        let second = deck[deck.len() - 2];
        let second_name = game.object(second).expect("second card").name.to_string();
        let source = game.new_object_id();
        let mut dm = ChooseNamed {
            name: second_name,
            viewed: Vec::new(),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        ReorderTopPlanarDeckEffect::you(2)
            .execute(&mut game, &mut ctx)
            .expect("communal planar reorder resolves");

        assert_eq!(dm.viewed, vec![top, second]);
        let deck = game.planar_deck(alice).expect("communal deck");
        assert_eq!(deck.first().copied(), Some(second));
        assert_eq!(deck.last().copied(), Some(top));
    }
}

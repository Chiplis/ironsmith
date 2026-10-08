//! Look at hand effect implementation.

#[cfg(test)]
use crate::decisions::context::ViewCardsContext;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_players_from_spec, view_hidden_candidate_objects};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub type LookAtHandEffect = ironsmith_core::LookAtHandEffect;

impl EffectExecutor for LookAtHandEffect {
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        self.reveal
            .then_some(crate::effect::PriorEffectAction::Revealed)
    }
    /// "Reveal your hand" can be paid as a cost (Land Grant's alternative
    /// cost); an empty hand can still be revealed.
    fn as_cost_executable(&self) -> Option<&dyn crate::effects::CostExecutableEffect> {
        (self.reveal
            && matches!(
                self.target,
                ChooseSpec::Player(crate::target::PlayerFilter::You)
            ))
        .then_some(self as &dyn crate::effects::CostExecutableEffect)
    }

    fn cost_description(&self) -> Option<String> {
        (self.reveal
            && matches!(
                self.target,
                ChooseSpec::Player(crate::target::PlayerFilter::You)
            ))
        .then(|| "Reveal your hand".to_string())
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(PreparedHandDisclosure {
            effect: self.clone(),
            iterated_player: ctx.iteration.iterated_player,
            selected: None,
        }))
    }

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
                let Some(selected) = select_hand_disclosure(self, game, ctx)? else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                selected.disclose(game, ctx)
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        if self.reveal {
            "player whose hand is revealed"
        } else {
            "player to look at"
        }
    }
}

/// Identities and the private viewer belong to selection, before any sibling
/// original commits. Authentication and disclosure remain with their child owners.
#[derive(Debug)]
struct SelectedHand {
    player: crate::ids::PlayerId,
    cards: Vec<crate::ids::ObjectId>,
    snapshots: Vec<crate::snapshot::ObjectSnapshot>,
}

#[derive(Debug)]
enum SelectedHandDisclosure {
    Finished(EffectOutcome),
    Hands {
        reveal: bool,
        viewer: crate::ids::PlayerId,
        hands: Vec<SelectedHand>,
    },
}

fn select_hand_disclosure(
    effect: &LookAtHandEffect,
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> Result<Option<SelectedHandDisclosure>, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let players = resolve_players_from_spec(game, &effect.target, ctx)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    if players.is_empty() {
        return Ok(Some(SelectedHandDisclosure::Finished(
            if effect.target.is_target() {
                EffectOutcome::target_invalid()
            } else {
                EffectOutcome::count(0)
            },
        )));
    }
    let hands = players
        .into_iter()
        .map(|player| {
            let cards = game
                .player(player)
                .map(|p| p.hand.to_vec())
                .unwrap_or_default();
            let snapshots = cards
                .iter()
                .filter_map(|id| crate::snapshot::ObjectSnapshot::from_object_id(game, *id))
                .collect();
            SelectedHand {
                player,
                cards,
                snapshots,
            }
        })
        .collect();
    Ok(Some(SelectedHandDisclosure::Hands {
        reveal: effect.reveal,
        viewer: ctx.controller,
        hands,
    }))
}

impl SelectedHandDisclosure {
    fn disclose(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Self::Hands {
            reveal,
            viewer,
            hands,
        } = self
        else {
            let Self::Finished(outcome) = self else {
                unreachable!()
            };
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                outcome,
            ));
        };
        let mut total_cards = 0;
        let mut outcome = EffectOutcome::count(0);
        let mut all_snapshots = Vec::new();
        let mut children = Vec::new();
        for hand in hands {
            total_cards += hand.cards.len() as i32;
            all_snapshots.extend(hand.snapshots.iter().cloned());
            let child = if reveal {
                let child = super::reveal_objects_with_outputs(
                    game,
                    ctx,
                    hand.snapshots,
                    Some(hand.player),
                    "Reveal that player's hand",
                    None,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                for snapshot in child
                    .outcome
                    .chosen_object_memory()
                    .unwrap_or_default()
                    .iter()
                    .cloned()
                {
                    ctx.tag_object(crate::effects::REVEALED_THIS_WAY_TAG, snapshot);
                }
                child
            } else {
                // Private tags retain the selected identities, before observation.
                for snapshot in hand.snapshots {
                    ctx.tag_object(crate::tag::LOOKED_AT_HAND_TAG, snapshot);
                }
                super::look_at_cards_with_outputs(
                    game,
                    ctx,
                    viewer,
                    hand.player,
                    crate::zone::Zone::Hand,
                    &hand.cards,
                    "Look at that player's hand",
                )?
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            outcome = EffectOutcome::aggregate([outcome, child.outcome.clone()]);
            children.push(child);
        }
        outcome.set_value(crate::effect::OutcomeValue::Count(i64::from(total_cards)));
        if !reveal {
            outcome = outcome
                .with_chosen_object_memory(all_snapshots.clone())
                .with_affected_object_memory(all_snapshots);
        }
        Ok(crate::effects::CompletedEffectOutputs::from_children(
            children,
            |_| outcome,
        ))
    }
}

#[derive(Debug)]
struct PreparedHandDisclosure {
    effect: LookAtHandEffect,
    iterated_player: Option<crate::ids::PlayerId>,
    selected: Option<SelectedHandDisclosure>,
}

impl crate::effects::SimultaneousEffectProposal for PreparedHandDisclosure {
    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.selected.is_some() || ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        self.selected = ctx.with_temp_iterated_player(self.iterated_player, |ctx| {
            select_hand_disclosure(&self.effect, game, ctx)
        })?;
        Ok(())
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.prepare_selection(game, ctx)
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let selected = self.selected.ok_or_else(|| {
            ExecutionError::InternalError(
                "hand disclosure committed before selection completed".into(),
            )
        })?;
        ctx.with_temp_iterated_player(self.iterated_player, |ctx| {
            crate::effects::composition::execute_transaction(
                game,
                ctx,
                || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                |game, ctx| selected.disclose(game, ctx),
            )
            .map(crate::effects::SimultaneousEffectCommit::finished)
        })
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(|receipt| receipt.outcome.into_outcome())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, CardBuilder};
    use crate::decision::DecisionMaker;
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[derive(Debug)]
    struct ViewCall {
        viewer: PlayerId,
        subject: PlayerId,
        zone: Zone,
        cards: Vec<ObjectId>,
    }

    #[derive(Debug, Default)]
    struct CaptureViewDm {
        calls: Vec<ViewCall>,
    }

    impl DecisionMaker for CaptureViewDm {
        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.calls.push(ViewCall {
                viewer,
                subject: ctx.subject,
                zone: ctx.zone,
                cards: cards.to_vec(),
            });
        }
    }

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_spell_card(card_id: u32, name: &str) -> Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Instant])
            .build()
    }

    fn add_card_to_hand(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_spell_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Hand);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_look_at_target_players_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let card1 = add_card_to_hand(&mut game, "Card 1", bob);
        let card2 = add_card_to_hand(&mut game, "Card 2", bob);

        let source = game.new_object_id();
        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Player(bob)]);

        let effect = LookAtHandEffect::new(ChooseSpec::target_player());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(dm.calls.len(), 1);

        let call = &dm.calls[0];
        assert_eq!(call.viewer, alice);
        assert_eq!(call.subject, bob);
        assert_eq!(call.zone, Zone::Hand);
        assert_eq!(call.cards, vec![card1, card2]);
    }

    #[test]
    fn test_reveal_target_players_hand_to_all_players() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let card1 = add_card_to_hand(&mut game, "Card 1", bob);
        let card2 = add_card_to_hand(&mut game, "Card 2", bob);

        let source = game.new_object_id();
        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Player(bob)]);

        let effect = LookAtHandEffect::reveal(ChooseSpec::target_player());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(dm.calls.len(), 2, "both players should see revealed hand");
        assert!(dm.calls.iter().all(|call| call.subject == bob));
        assert!(dm.calls.iter().all(|call| call.zone == Zone::Hand));
        assert!(dm.calls.iter().all(|call| call.cards == vec![card1, card2]));
    }
}

impl crate::effects::CostExecutableEffect for LookAtHandEffect {
    fn can_execute_as_cost(
        &self,
        _game: &GameState,
        _source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::executor_trait::CostValidationError> {
        Ok(())
    }
}

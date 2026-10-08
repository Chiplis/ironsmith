//! Reveal tagged cards effect implementation.
//!
//! Reveals currently update player-facing visibility and carry that visibility
//! through tagged contexts when later stack objects still need it.

use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

#[cfg(test)]
use crate::tag::TagKey;
#[cfg(test)]
use crate::decisions::context::ViewCardsContext;
pub type RevealTaggedEffect = ironsmith_core::RevealTaggedEffect;

#[derive(Debug)]
struct RevealTaggedProposal {
    effect: RevealTaggedEffect,
    selected: Vec<crate::snapshot::ObjectSnapshot>,
}
impl crate::effects::SimultaneousEffectProposal for RevealTaggedProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(|receipt| receipt.outcome.into_outcome())
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        ctx.set_tagged_objects(self.effect.tag.clone(), self.selected);
        let result = self.effect.execute_with_outputs(game, ctx);
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            checkpoint.restore(ctx);
        }
        result.map(crate::effects::SimultaneousEffectCommit::finished)
    }
}

impl EffectExecutor for RevealTaggedEffect {
    fn cost_choice_bindings(&self) -> crate::effects::CostChoiceBindings {
        crate::effects::CostChoiceBindings::requiring(self.tag.clone())
    }

    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Revealed)
    }
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(RevealTaggedProposal {
            effect: self.clone(),
            selected: ctx.get_tagged_all(&self.tag).cloned().unwrap_or_default(),
        }))
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
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
        let tagged = ctx
            .get_tagged_all(self.tag.clone())
            .cloned()
            .unwrap_or_default();
        if let Some(admission) = super::reveal::prospective_reveal_admission(ctx, tagged.len()) {
            return Ok(admission);
        }
        let outcome =
            super::reveal_objects_with_outputs(game, ctx, tagged, None, "Reveal cards", None)?;
        if !ctx.decision_maker.awaiting_choice() {
            ctx.set_tagged_objects(
                self.tag.clone(),
                outcome
                    .outcome
                    .chosen_object_memory()
                    .unwrap_or_default()
                    .to_vec(),
            );
        }
        Ok(outcome)
    }
}

impl CostExecutableEffect for RevealTaggedEffect {
    fn can_execute_as_cost(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[derive(Debug, Default)]
    struct CaptureViewDm {
        calls: Vec<(PlayerId, PlayerId, Zone, bool, Vec<crate::ids::ObjectId>)>,
    }

    impl DecisionMaker for CaptureViewDm {
        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[crate::ids::ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.calls
                .push((viewer, ctx.subject, ctx.zone, ctx.public, cards.to_vec()));
        }
    }

    #[test]
    fn reveal_tagged_emits_public_view_for_tagged_cards() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(201), "Tagged Card")
            .card_types(vec![CardType::Instant])
            .build();
        let object_id = game.create_object_from_card(&card, bob, Zone::Library);

        let snapshot = {
            let obj = game.object(object_id).expect("tagged object");
            ObjectSnapshot::from_object(obj, &game)
        };
        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.set_tagged_objects(TagKey::from("revealed"), vec![snapshot]);

        RevealTaggedEffect::new("revealed")
            .execute(&mut game, &mut ctx)
            .expect("reveal tagged");

        assert_eq!(dm.calls.len(), 2);
        assert!(dm.calls.iter().all(|(_, subject, zone, public, cards)| {
            *subject == bob && *zone == Zone::Library && *public && cards == &vec![object_id]
        }));
    }
}

#[cfg(test)]
mod selected_hidden_opening_tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::{SelectObjectsContext, SelectionRevealPolicy};
    use crate::snapshot::ObjectSnapshot;
    struct Opening { pause: bool, pending: bool, answer: Vec<ObjectId>, views: usize, requests: Vec<Vec<ObjectId>> }
    impl DecisionMaker for Opening {
        fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
            assert_eq!(context.reveal_policy, SelectionRevealPolicy::Public);
            assert_eq!(context.min, context.max.unwrap());
            self.requests.push(context.candidates.iter().map(|candidate| candidate.id).collect());
            self.pending = self.pause;
            self.answer.clone()
        }
        fn awaiting_choice(&self) -> bool { self.pending }
        fn view_cards(&mut self, game: &GameState, _: PlayerId, cards: &[ObjectId], _: &ViewCardsContext) {
            assert!(cards.iter().all(|id| !game.is_hidden_card_placeholder(*id)));
            self.views += 1;
        }
    }
    fn definition() -> crate::cards::CardDefinition {
        crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Opened name")
            .card_types(vec![crate::types::CardType::Land]).build()
    }
    fn run(game: &mut GameState, source: ObjectId, snapshot: &ObjectSnapshot, dm: &mut Opening) -> Result<EffectOutcome, ExecutionError> {
        let mut ctx = ExecutionContext::new(source, PlayerId(0), dm);
        ctx.set_tagged_objects("chosen", vec![snapshot.clone()]);
        let outcome = RevealTaggedEffect::new("chosen").execute(game, &mut ctx)?;
        if !ctx.decision_maker.awaiting_choice() {
            assert_eq!(ctx.get_tagged_all("chosen").unwrap()[0].name, "Opened name");
        }
        Ok(outcome)
    }
    #[test]
    fn owner_and_peer_open_only_the_selected_set_and_suspend_before_identity_use() {
        for peer in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.new_object_id();
            let definition = definition();
            let selected = game.create_hidden_card_placeholder(PlayerId(1), crate::zone::Zone::Hand, 0, "selected".into());
            let unselected = game.create_hidden_card_placeholder(PlayerId(1), crate::zone::Zone::Hand, 1, "unselected".into());
            if !peer { game.reveal_hidden_card_with_definition(selected, &definition).unwrap(); }
            let original = ObjectSnapshot::from_object(game.object(selected).unwrap(), &game);
            let mut dm = Opening { pause: true, pending: false, answer: vec![selected], views: 0, requests: Vec::new() };
            run(&mut game, source, &original, &mut dm).unwrap();
            assert!(dm.pending);
            assert_eq!(dm.requests, vec![vec![selected]]);
            assert_eq!(dm.views, 0);
            assert!(!game.is_publicly_revealed_hidden_card(selected));
            assert!(!game.is_publicly_revealed_hidden_card(unselected));
            if peer { game.reveal_hidden_card_with_definition(selected, &definition).unwrap(); }
            dm.pause = false;
            dm.pending = false;
            let outcome = run(&mut game, source, &original, &mut dm).unwrap();
            assert_eq!(outcome.as_count(), Some(1));
            assert_eq!(dm.views, 2);
            assert!(game.is_publicly_revealed_hidden_card(selected));
            assert!(!game.is_publicly_revealed_hidden_card(unselected));
            assert!(game.is_hidden_card_placeholder(unselected));
            assert!(dm.requests.iter().all(|ids| ids == &[selected]));
        }
    }
    #[test]
    fn a_replayed_answer_without_the_required_peer_opening_is_incomplete() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.new_object_id();
        let selected = game.create_hidden_card_placeholder(PlayerId(1), crate::zone::Zone::Hand, 0, "selected".into());
        let original = ObjectSnapshot::from_object(game.object(selected).unwrap(), &game);
        let mut dm = Opening { pause: false, pending: false, answer: vec![selected], views: 0, requests: Vec::new() };
        assert!(matches!(run(&mut game, source, &original, &mut dm), Err(ExecutionError::IncompleteEvidence(_))));
        assert_eq!(dm.views, 0);
        assert!(!game.is_publicly_revealed_hidden_card(selected));
    }
}

#[cfg(test)]
mod multi_reveal_receipts {
    use super::*;
    #[test]
    fn revealed_cards_have_distinct_occurrences_and_disclosure_is_not_read_only_preparation() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Revealed")
            .card_types(vec![crate::types::CardType::Land]).build();
        let source = game.new_object_id();
        let cards: Vec<_> = (0..2).map(|_| game.create_object_from_definition(&definition, PlayerId(1), crate::Zone::Hand)).collect();
        let snapshots = cards.iter().map(|id| crate::snapshot::ObjectSnapshot::from_object(game.object(*id).unwrap(), &game)).collect();
        let mut ctx = ExecutionContext::new_default(source, PlayerId(0));
        ctx.set_tagged_objects("revealed", snapshots);
        let effect = RevealTaggedEffect::new("revealed");
        assert!(!effect.is_read_only_simultaneous_player_action());
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(outcome.events.len(), 2);
        assert_ne!(outcome.events[0].occurrence_key(), outcome.events[1].occurrence_key());
        assert_eq!(ctx.get_tagged_all(crate::effects::PUBLIC_REVEALED_TAG).unwrap().len(), 2);
    }
}

#[cfg(test)]
mod simultaneous_selected_reveal_tests {
    use super::*;
    #[derive(Default)]
    struct Observe { revealed: Vec<ObjectId> }
    impl crate::decision::DecisionMaker for Observe {
        fn view_cards(&mut self, _: &GameState, _: PlayerId, cards: &[ObjectId], _: &ViewCardsContext) {
            self.revealed.extend_from_slice(cards);
        }
    }
    #[test]
    fn simultaneous_preparation_freezes_each_set_without_publishing_it() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.new_object_id();
        let definition = crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Selected")
            .card_types(vec![crate::types::CardType::Land]).build();
        let first = game.create_object_from_definition(&definition, PlayerId(0), crate::Zone::Hand);
        let second = game.create_object_from_definition(&definition, PlayerId(1), crate::Zone::Hand);
        let first_snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(first).unwrap(), &game);
        let second_snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(second).unwrap(), &game);
        let effect = RevealTaggedEffect::new("selected");
        assert!(effect.supports_simultaneous_player_action());
        assert!(!effect.is_read_only_simultaneous_player_action());
        let before_provenance = game.provenance_graph().node_count();
        let mut dm = Observe::default();
        let (first_proposal, second_proposal) = {
            let mut ctx = ExecutionContext::new(source, PlayerId(0), &mut dm);
            ctx.set_tagged_objects("selected", vec![first_snapshot]);
            let first = effect.prepare_simultaneous_player_action(&game, &mut ctx).unwrap();
            ctx.set_tagged_objects("selected", vec![second_snapshot]);
            let second = effect.prepare_simultaneous_player_action(&game, &mut ctx).unwrap();
            assert!(ctx.get_tagged_all(crate::effects::PUBLIC_REVEALED_TAG).is_none());
            (first, second)
        };
        assert!(dm.revealed.is_empty());
        assert_eq!(game.provenance_graph().node_count(), before_provenance);
        let mut ctx = ExecutionContext::new(source, PlayerId(0), &mut dm);
        let first_result = first_proposal.commit(&mut game, &mut ctx).unwrap();
        let second_result = second_proposal.commit(&mut game, &mut ctx).unwrap();
        assert_eq!(first_result.chosen_object_memory().unwrap()[0].object_id, first);
        assert_eq!(second_result.chosen_object_memory().unwrap()[0].object_id, second);
        assert_eq!(ctx.get_tagged_all(crate::effects::PUBLIC_REVEALED_TAG).unwrap().len(), 2);
        drop(ctx);
        assert_eq!(dm.revealed, vec![first, first, second, second]);
    }
}

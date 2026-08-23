//! Exile top cards of library effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{
    resolve_player_filter, resolve_value, view_hidden_candidate_objects,
};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Effect that exiles cards from the top of a player's library.
#[derive(Debug, Clone, PartialEq)]
pub struct ExileTopOfLibraryEffect {
    /// How many cards to exile.
    pub count: Value,
    /// Which player's library to exile from.
    pub player: PlayerFilter,
    /// Authored actor placement; gameplay semantics remain in `player`.
    pub surface: Option<ironsmith_core::ExileTopLibrarySurface>,
    /// Optional tags to record the cards moved this way.
    pub moved_tags: Vec<TagKey>,
    /// Optional tags that accumulate all cards moved across repeated executions.
    pub accumulated_tags: Vec<TagKey>,
    /// Whether the cards are exiled face down without being revealed.
    pub face_down: bool,
}

impl ExileTopOfLibraryEffect {
    /// Create a new exile-top effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
            surface: None,
            moved_tags: Vec::new(),
            accumulated_tags: Vec::new(),
            face_down: false,
        }
    }

    pub fn tag_moved(mut self, tag: impl Into<TagKey>) -> Self {
        self.moved_tags.push(tag.into());
        self
    }

    pub fn with_surface(mut self, surface: ironsmith_core::ExileTopLibrarySurface) -> Self {
        self.surface = Some(surface);
        self
    }

    pub fn append_tagged(mut self, tag: impl Into<TagKey>) -> Self {
        self.accumulated_tags.push(tag.into());
        self
    }

    pub fn face_down(mut self) -> Self {
        self.face_down = true;
        self
    }
}

impl EffectExecutor for ExileTopOfLibraryEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        for tag in &self.moved_tags {
            ctx.clear_object_tag(tag.as_str());
        }

        let top_cards = game
            .player(player_id)
            .map(|p| {
                let lib_len = p.library.len();
                let exile_count = count.min(lib_len);
                p.library[lib_len.saturating_sub(exile_count)..].to_vec()
            })
            .unwrap_or_default();

        let mut moved_ids = Vec::new();
        for card_id in top_cards {
            if let Some(exiled_id) = game.move_object_by_effect(card_id, Zone::Exile) {
                game.add_exiled_with_source_link(ctx.source, exiled_id);
                if self.face_down {
                    game.set_face_down(exiled_id);
                }
                if (!self.moved_tags.is_empty() || !self.accumulated_tags.is_empty())
                    && let Some(obj) = game.object(exiled_id)
                {
                    let snapshot = ObjectSnapshot::from_object(obj, game);
                    for tag in &self.moved_tags {
                        ctx.tag_object(tag.clone(), snapshot.clone());
                    }
                    for tag in &self.accumulated_tags {
                        ctx.tag_object(tag.clone(), snapshot.clone());
                    }
                }
                moved_ids.push(exiled_id);
            }
        }

        if !self.face_down {
            view_hidden_candidate_objects(
                game,
                ctx,
                player_id,
                &moved_ids,
                "Reveal exiled library cards",
                true,
            );
        }

        Ok(EffectOutcome::with_objects(moved_ids.clone())
            .with_affected_objects_from_game(game, moved_ids))
    }
}

impl CostExecutableEffect for ExileTopOfLibraryEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        let player_id = match self.player {
            PlayerFilter::You => controller,
            PlayerFilter::Specific(id) => id,
            _ => controller,
        };
        let count = match &self.count {
            Value::Fixed(count) => (*count).max(0) as usize,
            Value::X => {
                return Err(CostValidationError::Other(
                    "dynamic X exile-top costs are not supported".to_string(),
                ));
            }
            _ => {
                let ctx = ExecutionContext::new_default(source, controller);
                resolve_value(game, &self.count, &ctx)
                    .map_err(|err| CostValidationError::Other(format!("{err:?}")))?
                    .max(0) as usize
            }
        };
        let available = game.player(player_id).map_or(0, |p| p.library.len());
        if available >= count {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "not enough cards in library to pay exile-top cost".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::ViewCardsContext;

    #[derive(Default)]
    struct CaptureViewsDecisionMaker {
        views: Vec<(PlayerId, Vec<ObjectId>, ViewCardsContext)>,
    }

    impl DecisionMaker for CaptureViewsDecisionMaker {
        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &ViewCardsContext,
        ) {
            self.views.push((viewer, cards.to_vec(), ctx.clone()));
        }
    }

    #[test]
    fn exiling_hidden_library_cards_opens_a_public_view_before_followup_choices() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let hidden = game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            0,
            "alice-slot-0".to_string(),
        );
        let source = ObjectId::from_raw(9001);
        let mut dm = CaptureViewsDecisionMaker::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = ExileTopOfLibraryEffect::new(Value::Fixed(1), PlayerFilter::You);

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("exile-top should resolve");
        let exiled = outcome
            .affected_objects()
            .and_then(|ids| ids.first().copied())
            .expect("hidden card should be exiled");

        assert_ne!(
            exiled, hidden,
            "zone move should reseat the hidden object id"
        );
        assert!(game.hidden_card_info(exiled).is_some());
        assert_eq!(
            game.get_exiled_with_source_links(source),
            &[exiled],
            "exile-top must retain the source link used by source-relative permissions"
        );
        assert_eq!(game.exiled_with_source_revision(source), 1);
        assert!(
            dm.views.iter().any(|(viewer, cards, view_ctx)| {
                *viewer == alice
                    && cards == &[exiled]
                    && view_ctx.public
                    && view_ctx.zone == Zone::Exile
            }),
            "exiling a hidden library card face up should create a public view before later prompts"
        );
    }

    #[test]
    fn exiling_top_cards_face_down_keeps_them_hidden_and_appends_the_collection_tag() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first = game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            0,
            "alice-slot-0".to_string(),
        );
        let second = game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            1,
            "alice-slot-1".to_string(),
        );
        let source = ObjectId::from_raw(9002);
        let mut dm = CaptureViewsDecisionMaker::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let preexisting = ObjectSnapshot::from_object(
            game.object(first).expect("first library card should exist"),
            &game,
        );
        ctx.tag_object("pile", preexisting);

        let outcome = ExileTopOfLibraryEffect::new(Value::Fixed(2), PlayerFilter::You)
            .append_tagged("pile")
            .face_down()
            .execute(&mut game, &mut ctx)
            .expect("face-down exile-top should resolve");
        let exiled = outcome
            .affected_objects()
            .expect("both hidden cards should be exiled");

        assert_eq!(exiled.len(), 2);
        assert!(exiled.iter().all(|id| game.is_face_down(*id)));
        assert!(exiled.iter().all(|id| game.hidden_card_info(*id).is_some()));
        assert!(exiled.iter().all(|id| {
            !game.can_player_look_at_face_down_exiled_card(*id, alice)
                && !game.can_player_look_at_face_down_exiled_card(*id, bob)
        }));
        let pile_len = ctx.get_tagged_all("pile").map(Vec::len);
        drop(ctx);
        assert!(
            dm.views.is_empty(),
            "face-down exile must not reveal the pile"
        );
        assert_eq!(
            pile_len,
            Some(3),
            "append_tagged must retain the existing collection and append both moved cards"
        );
        assert!(exiled.iter().all(|id| *id != first && *id != second));
    }

    #[test]
    fn dynamic_opponent_count_exiles_exactly_that_many_cards_from_the_top_face_down() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bottom = game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            0,
            "alice-bottom".to_string(),
        );
        game.create_hidden_card_placeholder(alice, Zone::Library, 1, "alice-middle".to_string());
        game.create_hidden_card_placeholder(alice, Zone::Library, 2, "alice-top".to_string());
        let source = ObjectId::from_raw(9003);
        let mut dm = CaptureViewsDecisionMaker::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        let outcome = ExileTopOfLibraryEffect::new(
            Value::CountPlayers(PlayerFilter::Opponent),
            PlayerFilter::You,
        )
        .face_down()
        .execute(&mut game, &mut ctx)
        .expect("opponent-count exile-top should resolve");
        let exiled = outcome
            .affected_objects()
            .expect("two top cards should be exiled");

        assert_eq!(exiled.len(), 2);
        assert!(exiled.iter().all(|id| game.is_face_down(*id)));
        assert_eq!(
            game.player(alice).expect("alice").library,
            vec![bottom],
            "the bottom card must remain when the two top cards are exiled"
        );
        drop(ctx);
        assert!(dm.views.is_empty(), "face-down cards must not be revealed");
    }
}

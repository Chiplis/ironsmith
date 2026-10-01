//! Exile top cards of library effect implementation.

use crate::effects::zones::apply_zone_change_with_context_and_additional_effects;
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
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        for tag in &self.moved_tags {
            ctx.clear_object_tag(tag.as_str());
        }

        let top_cards = game
            .player(player_id)
            .map(|p| {
                let mut cards = p.library.iter().rev()
                    .filter(|id| !ctx.replacement.entry_reserved_objects.contains(id))
                    .take(count).copied().collect::<Vec<_>>();
                // Preserve the existing bottom-to-top processing order within
                // the selected top group while skipping simultaneous entrants.
                cards.reverse();
                cards
            })
            .unwrap_or_default();

        let mut moved_ids = Vec::new();
        let mut receipts = Vec::new();
        for card_id in top_cards {
            // CR 614.1: exiling from the library is an ordinary zone change,
            // so replacement effects apply to it (as for mill and surveil).
            let additional_effects = ctx.additional_replacement_effects_snapshot();
            let receipt = crate::effects::zones::apply_zone_change_with_context_and_additional_effects(
    game,
    card_id,
    Zone::Library,
    Zone::Exile,
    ctx.cause.clone(),
    ctx,
    &additional_effects
)?;
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            let exiled_id = match &receipt.original {
                crate::events::processing::EventOutcome::Proceed(change) if change.final_zone == Zone::Exile => {
                    change.new_object_id
                }
                _ => None,
            };
            if let Some(exiled_id) = exiled_id {
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
            receipts.push((card_id, receipt));
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

        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let original_outcome = EffectOutcome::with_objects(moved_ids.clone())
            .with_affected_objects_from_game(game, moved_ids);
        // Face-down state, links, tags and public reveal belong to the original
        // instruction. The additions observe those finished results.
        crate::effects::zones::finish_zone_change_receipts(game, ctx, original_outcome, receipts)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if ctx.decision_maker.awaiting_choice() { return result.map(|_| EffectOutcome::count(0)); }
        result
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


#[cfg(test)]
mod additional_owner_contract_tests {
    use super::*;
    use crate::effect::{Effect, OutcomeValue};
    use crate::ids::CardId;
    use crate::effects::{CounterEffect, SurveilEffect};
    use crate::effects::cards::ImprintFromHandEffect;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::{ChooseSpec, ObjectFilter};
    struct ObserveOriginal { owner: u8, alice: PlayerId, source: ObjectId, pause: bool, pending: bool, questions: usize }
    impl crate::decision::DecisionMaker for ObserveOriginal {
        fn decide_objects(&mut self, _: &GameState, ctx: &crate::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            ctx.candidates.iter().filter(|c| c.legal).map(|c| c.id).take(ctx.max.unwrap_or(1).min(1)).collect()
        }
        fn decide_partition(&mut self, _: &GameState, ctx: &crate::decisions::context::PartitionContext) -> Vec<ObjectId> {
            ctx.cards.iter().map(|(id,_)| *id).collect()
        }
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.questions += 1;
            match self.owner {
                0 => {
                    let linked = game.get_exiled_with_source_links(self.source);
                    assert_eq!(linked.len(), 2, "all original exile links precede additions");
                    assert!(linked.iter().all(|id| game.is_face_down(*id)));
                    assert!(game.player(self.alice).unwrap().library.is_empty());
                }
                1 => { assert_eq!(game.player(self.alice).unwrap().graveyard.len(), 2); assert!(game.player(self.alice).unwrap().library.is_empty()); }
                2 => { assert_eq!(game.get_imprinted_cards(self.source).len(), 1); assert_eq!(game.get_exiled_with_source_links(self.source).len(), 1); }
                3 => { assert!(game.stack.is_empty(), "the whole original counter instruction precedes additions"); assert_eq!(game.player(self.alice).unwrap().graveyard.len(), 2); }
                _ => unreachable!(),
            }
            self.pending = self.pause;
            !self.pause
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn run_owner(owner: u8, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        match owner {
            0 => ExileTopOfLibraryEffect::new(2, PlayerFilter::You).face_down().tag_moved("original_exiles").append_tagged("accumulated").execute(game, ctx),
            1 => SurveilEffect::you(2).execute(game, ctx),
            2 => ImprintFromHandEffect::new(ObjectFilter::default()).execute(game, ctx),
            3 => CounterEffect::new(ChooseSpec::all(ObjectFilter::default().in_zone(Zone::Stack))).execute(game, ctx),
            _ => unreachable!(),
        }
    }
    fn assert_original(owner: u8, outcome: &EffectOutcome) {
        match owner {
            0 | 2 => { let OutcomeValue::Objects(ids) = &outcome.value else { panic!("original object summary"); }; assert_eq!(ids.len(), if owner == 0 { 2 } else { 1 }); }
            1 => {
                assert_eq!(outcome.count_or_zero(), 2);
                assert_eq!(outcome.events.iter().filter_map(|e| e.downcast::<crate::events::KeywordActionEvent>()).filter(|e| e.action == crate::events::KeywordActionKind::Surveil).count(), 1);
            }
            3 => { assert!(matches!(outcome.value, OutcomeValue::None)); assert_eq!(outcome.events.iter().filter_map(|e| e.downcast::<crate::events::SpellCounteredEvent>()).count(), 2); }
            _ => unreachable!(),
        }
    }
    fn check_owner(owner: u8, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let card = crate::card::CardBuilder::new(CardId::new(), "Original zone owner fixture")
            .card_types(vec![crate::types::CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2,2)).build();
        let from = match owner { 0 | 1 => Zone::Library, 2 => Zone::Hand, 3 => Zone::Stack, _ => unreachable!() };
        let to = if owner == 0 || owner == 2 { Zone::Exile } else { Zone::Graveyard };
        let mut originals = Vec::new();
        for _ in 0..if owner == 2 { 1 } else { 2 } {
            let id = game.create_object_from_card(&card, alice, from);
            if owner == 3 { game.stack.push(crate::game_state::StackEntry::new(id, alice)); }
            originals.push(id);
        }
        // The fixture's only added operation changes counters, not zones.
        // Track the original card explicitly for inspection after consumers
        // have taken their arrival receipts; do not infer a missing receipt
        // means the original move failed.
        let tracked_stable = game.object(originals[0]).unwrap().stable_id;
        let source = game.create_object_from_card(&card, bob, Zone::Battlefield);
        let effects = if mode == 3 { vec![Effect::new(crate::effects::PutCountersEffect::new(crate::object::CounterType::PlusOnePlusOne, 1, ChooseSpec::tagged("it")))] }
            else if mode == 1 { vec![Effect::gain_life(3), Effect::lose_life(Value::X)] }
            else { vec![Effect::gain_life(3), Effect::may(vec![Effect::gain_life(4)])] };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(originals[0]), Some(from), Some(to)),
            ReplacementAction::Additionally(effects)));
        game.take_pending_trigger_events();
        let before_id = game.next_object_id_counter();
        let library = game.player(alice).unwrap().library.clone(); let hand = game.player(alice).unwrap().hand.clone();
        let stack = game.stack.iter().map(|e| e.object_id).collect::<Vec<_>>();
        let parent_tag = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let mut dm = ObserveOriginal { owner, alice, source, pause: mode == 2, pending: false, questions: 0 };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        for name in ["it", "original_exiles", "accumulated"] { ctx.set_tagged_objects(name, vec![parent_tag.clone()]); }
        let result = run_owner(owner, &mut game, &mut ctx);
        if mode == 1 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
        else {
            let outcome = result.unwrap();
            if mode == 2 { assert!(ctx.decision_maker.awaiting_choice()); assert!(outcome.events.is_empty()); }
            else {
                assert_original(owner, &outcome);
                if mode == 3 {
                    let arrived = game.find_object_by_stable_id(tracked_stable).expect("the original moved card still exists");
                    assert_eq!(game.object(arrived).unwrap().counters.get(&crate::object::CounterType::PlusOnePlusOne), Some(&1));
                    assert!(!game.object(source).unwrap().counters.contains_key(&crate::object::CounterType::PlusOnePlusOne));
                    let occurrences = outcome.execution_facts.iter().filter_map(|fact| match fact { crate::effect::ExecutionFact::AffectedObjectMemory(memory) => Some(memory.as_slice()), _ => None }).flatten().filter(|memory| memory.object_id == arrived && memory.zone == to).count();
                    assert!(occurrences >= if owner == 0 { 2 } else { 1 }, "added action memory reaches the original owner");
                    assert_eq!(outcome.affected_object_memory().unwrap_or(&[]).iter().filter(|memory| memory.object_id == arrived && memory.zone == to).count(), usize::from(owner == 0), "original memory excludes auxiliary duplicates and retains its own exile result");
                } else {
                    assert_eq!(game.player(bob).unwrap().life, 27);
                    let gains = outcome.events.iter().filter_map(|e| e.downcast::<crate::events::LifeGainEvent>()).collect::<Vec<_>>();
                    assert_eq!(gains.iter().map(|e| e.amount).collect::<Vec<_>>(), vec![3,4]);
                    assert!(gains.iter().all(|e| e.player == bob));
                }
                if owner == 0 { assert_eq!(ctx.get_tagged_all("original_exiles").unwrap().len(), 2); assert_eq!(ctx.get_tagged_all("accumulated").unwrap().len(), 3); }
            }
        }
        assert_eq!(ctx.source, source); assert_eq!(ctx.controller, alice);
        assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id, source);
        assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
        if mode == 1 || mode == 2 {
            assert_eq!(ctx.get_tagged_all("original_exiles").unwrap()[0].object_id, source);
            assert_eq!(ctx.get_tagged_all("accumulated").unwrap().len(), 1);
        }
        drop(ctx);
        assert_eq!(game.player(alice).unwrap().life, 20);
        if mode == 1 || mode == 2 {
            assert_eq!(game.player(alice).unwrap().library, library); assert_eq!(game.player(alice).unwrap().hand, hand);
            assert_eq!(game.stack.iter().map(|e| e.object_id).collect::<Vec<_>>(), stack);
            assert!(game.player(alice).unwrap().graveyard.is_empty()); assert!(game.exile.is_empty());
            assert_eq!(game.player(bob).unwrap().life, 20); assert_eq!(game.next_object_id_counter(), before_id);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
            assert!(game.get_exiled_with_source_links(source).is_empty()); assert!(game.get_imprinted_cards(source).is_empty());
            assert!(game.take_pending_trigger_events().is_empty());
            assert!(originals.iter().all(|id| game.object(*id).unwrap().zone == from));
        } else { assert!(game.effect_store.replacement_effects.get_effect(shield).is_none()); }
        if mode == 2 {
            assert_eq!(dm.questions, 1);
            let mut dm = ObserveOriginal { owner, alice, source, pause: false, pending: false, questions: 0 };
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let outcome = run_owner(owner, &mut game, &mut ctx).unwrap(); assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx);
            assert_original(owner, &outcome); assert_eq!(dm.questions, 1); assert_eq!(game.player(bob).unwrap().life, 27);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            assert_eq!(outcome.events.iter().filter_map(|e| e.downcast::<crate::events::LifeGainEvent>()).map(|e| e.amount).collect::<Vec<_>>(), vec![3,4]);
        }
    }
    #[test] fn additional_exile_top_retains_original_then_payload() { check_owner(0, 0); }
    #[test] fn additional_exile_top_error_restores_original_and_prefix() { check_owner(0, 1); }
    #[test] fn additional_exile_top_pending_restores_then_replays_once() { check_owner(0, 2); }
    #[test] fn additional_exile_top_binds_arrival_and_retains_added_facts() { check_owner(0, 3); }
    #[test] fn additional_surveil_retains_original_then_payload() { check_owner(1, 0); }
    #[test] fn additional_surveil_error_restores_original_and_prefix() { check_owner(1, 1); }
    #[test] fn additional_surveil_pending_restores_then_replays_once() { check_owner(1, 2); }
    #[test] fn additional_surveil_binds_arrival_and_retains_added_facts() { check_owner(1, 3); }
    #[test] fn additional_imprint_retains_original_then_payload() { check_owner(2, 0); }
    #[test] fn additional_imprint_error_restores_original_and_prefix() { check_owner(2, 1); }
    #[test] fn additional_imprint_pending_restores_then_replays_once() { check_owner(2, 2); }
    #[test] fn additional_imprint_binds_arrival_and_retains_added_facts() { check_owner(2, 3); }
    #[test] fn additional_counter_retains_original_then_payload() { check_owner(3, 0); }
    #[test] fn additional_counter_error_restores_original_and_prefix() { check_owner(3, 1); }
    #[test] fn additional_counter_pending_restores_then_replays_once() { check_owner(3, 2); }
    #[test] fn additional_counter_binds_arrival_and_retains_added_facts() { check_owner(3, 3); }
}

//! Lose the game effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::LoseTheGameEffect;

/// Effect that causes a player to lose the game.
///
/// Checks for effects that prevent losing (e.g., Platinum Angel).
///
/// # Fields
///
/// * `player` - The player who loses the game
///
/// # Example
///
/// ```ignore
/// // Target player loses the game
/// let effect = LoseTheGameEffect::new(PlayerFilter::Opponent);
///
/// // You lose the game (alternate win condition trigger)
/// let effect = LoseTheGameEffect::you();
/// ```
impl EffectExecutor for LoseTheGameEffect {
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

        let Some((_, outcome)) = crate::events::processing::process_player_loss_with_context(
            game, player_id, ctx, &std::collections::HashMap::new(),
        )? else { return Ok(EffectOutcome::count(0)); };
        Ok(outcome)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            *game = checkpoint; context_checkpoint.restore(ctx);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::events::other::WouldLoseGameMatcher;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    struct ChooseReplacement(usize);

    impl DecisionMaker for ChooseReplacement {
        fn decide_options(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            vec![self.0]
        }
    }

    fn setup_game() -> (GameState, PlayerId, PlayerId) {
        (
            GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20),
            PlayerId::from_index(0),
            PlayerId::from_index(1),
        )
    }

    fn source_permanent(game: &mut GameState, controller: PlayerId, name: &str) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn register_loss_replacement(
        game: &mut GameState,
        source: ObjectId,
        controller: PlayerId,
        effects: Vec<Effect>,
    ) -> ReplacementEffect {
        let replacement = ReplacementEffect::with_matcher(
            source,
            controller,
            WouldLoseGameMatcher,
            ReplacementAction::Instead(effects),
        );
        game.effect_store
            .replacement_effects
            .add_resolution_effect(replacement.clone());
        replacement
    }

    #[test]
    fn ordinary_lose_game_effect_commits_loss_and_emits_event() {
        let (mut game, alice, _) = setup_game();
        let source = source_permanent(&mut game, alice, "Loss Source");
        let mut dm = ChooseReplacement(0);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        LoseTheGameEffect::new(PlayerFilter::You)
            .execute(&mut game, &mut ctx)
            .expect("loss resolves");

        assert!(!game.player(alice).expect("alice").is_in_game());
        assert_eq!(
            game.take_pending_trigger_events()
                .iter()
                .filter(|event| event
                    .downcast::<crate::events::PlayerLosesGameEvent>()
                    .is_some())
                .count(),
            1
        );
    }

    #[test]
    fn lose_game_replacement_executes_instead_of_committing_loss() {
        let (mut game, alice, _) = setup_game();
        let source = source_permanent(&mut game, alice, "Mirror");
        register_loss_replacement(&mut game, source, alice, vec![Effect::set_life_total(7)]);
        game.player_mut(alice).expect("alice").life = 0;

        let mut dm = ChooseReplacement(0);
        assert_eq!(
            crate::events::processing::process_player_loss(&mut game, alice, &mut dm).expect("replacement operation must finish without execution error").expect("synchronous loss verdict must be committed"),
            crate::events::processing::PlayerLossOutcome::Replaced
        );
        assert!(game.player(alice).expect("alice").is_in_game());
        assert_eq!(game.player(alice).expect("alice").life, 7);
    }

    #[test]
    fn affected_player_chooses_between_loss_replacements() {
        struct ChooseSecondFor(PlayerId);

        impl DecisionMaker for ChooseSecondFor {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                assert_eq!(ctx.player, self.0);
                vec![1]
            }
        }

        let (mut game, alice, _) = setup_game();
        let first = source_permanent(&mut game, alice, "First Mirror");
        let second = source_permanent(&mut game, alice, "Second Mirror");
        register_loss_replacement(&mut game, first, alice, vec![Effect::set_life_total(3)]);
        register_loss_replacement(&mut game, second, alice, vec![Effect::set_life_total(9)]);
        game.player_mut(alice).expect("alice").life = 0;

        let mut dm = ChooseSecondFor(alice);
        crate::events::processing::process_player_loss(&mut game, alice, &mut dm).expect("replacement operation must finish without execution error").expect("synchronous loss verdict must be committed");

        assert_eq!(game.player(alice).expect("alice").life, 9);
        assert!(game.player(alice).expect("alice").is_in_game());
    }

    #[test]
    fn declining_optional_loss_replacement_allows_original_loss() {
        let (mut game, alice, _) = setup_game();
        let source = source_permanent(&mut game, alice, "Optional Mirror");
        let replacement = ReplacementEffect::with_matcher(
            source,
            alice,
            WouldLoseGameMatcher,
            ReplacementAction::Instead(vec![Effect::set_life_total(5)]),
        )
        .optional();
        let registered = game.effect_store
            .replacement_effects
            .add_resolution_effect(replacement);
        let decline = game.effect_store.replacement_effects
            .get_effect(registered).unwrap()
            .optional_decline_effect()
            .expect("optional replacement has decline choice");
        game.effect_store
            .replacement_effects
            .add_resolution_effect(decline);

        let mut dm = ChooseReplacement(1);
        assert_eq!(
            crate::events::processing::process_player_loss(&mut game, alice, &mut dm).expect("replacement operation must finish without execution error").expect("synchronous loss verdict must be committed"),
            crate::events::processing::PlayerLossOutcome::Lost
        );
        assert!(!game.player(alice).expect("alice").is_in_game());
    }

    #[test]
    fn empty_replacement_sequence_removes_loss_event() {
        let (mut game, alice, _) = setup_game();
        let source = source_permanent(&mut game, alice, "Null Mirror");
        register_loss_replacement(&mut game, source, alice, Vec::new());

        let mut dm = ChooseReplacement(0);
        assert_eq!(
            crate::events::processing::process_player_loss(&mut game, alice, &mut dm).expect("replacement operation must finish without execution error").expect("synchronous loss verdict must be committed"),
            crate::events::processing::PlayerLossOutcome::Replaced
        );
        assert!(game.player(alice).expect("alice").is_in_game());
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn replacement_source_lki_survives_source_exile() {
        let (mut game, alice, _) = setup_game();
        let card = CardBuilder::new(CardId::new(), "LKI Angel")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        register_loss_replacement(
            &mut game,
            source,
            alice,
            vec![
                Effect::exile(ChooseSpec::Source),
                Effect::gain_life(Value::PowerOf(Box::new(ChooseSpec::Source))),
            ],
        );
        game.player_mut(alice).expect("alice").life = 0;

        let mut dm = ChooseReplacement(0);
        crate::events::processing::process_player_loss(&mut game, alice, &mut dm).expect("replacement operation must finish without execution error").expect("synchronous loss verdict must be committed");

        assert_eq!(game.player(alice).expect("alice").life, 4);
        assert!(game.objects_in_zone(Zone::Exile).iter().any(|object_id| {
            game.object(*object_id)
                .is_some_and(|object| object.name == "LKI Angel")
        }));
        assert!(game.player(alice).expect("alice").is_in_game());
    }
}

#[cfg(test)]
mod replacement_loss_owner_contract_tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::types::CardType;
    use crate::zone::Zone;
    #[derive(Debug, Clone)]
    struct LossOf(PlayerId);
    impl crate::events::ReplacementMatcher for LossOf {
        fn matches_event(&self, event: &dyn crate::events::GameEventType, _: &crate::events::EventContext) -> bool {
            event.as_any().downcast_ref::<crate::events::PlayerLosesGameEvent>().is_some_and(|event| event.player == self.0)
        }
        fn display(&self) -> String { "Fixture loss".into() }
    }
    struct Answers { alice: PlayerId, source: ObjectId, pause: bool, pending: bool, calls: usize, instead: bool }
    impl DecisionMaker for Answers {
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.calls += 1;
            assert_eq!(game.player(self.alice).unwrap().is_in_game(), self.instead);
            assert_eq!(game.object(self.source).is_some(), self.instead);
            self.pending = self.pause; !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn perform(game: &mut GameState, queue: &mut crate::triggers::TriggerQueue, alice: PlayerId, parent: ObjectId, sba: bool, dm: &mut Answers) -> Result<(), String> {
        if sba { crate::game_loop::check_and_apply_sbas_with(game, queue, dm).map_err(|error| format!("{error:?}")) }
        else {
            let mut ctx = ExecutionContext::new(parent, alice, dm);
            LoseTheGameEffect::you().execute(game, &mut ctx).map(|_| ()).map_err(|error| format!("{error:?}"))
        }
    }
    fn check(sba: bool, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game(); let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let parent = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Parent").card_types(vec![CardType::Artifact]).build(), bob, Zone::Battlefield);
        let source = game.create_object_from_card(&CardBuilder::new(CardId::new(), "Loss replacement").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 3)).build(), alice, Zone::Battlefield);
        game.player_mut(alice).unwrap().life = 0;
        let effects = match mode {
            1 | 4 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![Effect::gain_life(Value::SourcePower), Effect::may(vec![Effect::gain_life(0)])],
            _ => vec![Effect::gain_life(3), Effect::may(vec![Effect::gain_life(4)])],
        };
        let action = if mode >= 4 { ReplacementAction::Instead(effects) } else { ReplacementAction::Additionally(effects) };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, bob, LossOf(alice), action));
        game.take_pending_trigger_events(); let ids = game.next_object_id_counter(); let objects = game.objects_in_deterministic_order().len();
        let mut queue = crate::triggers::TriggerQueue::new();
        let mut dm = Answers { alice, source, pause: mode == 2 || mode == 5, pending: false, calls: 0, instead: mode >= 4 };
        let result = perform(&mut game, &mut queue, alice, parent, sba, &mut dm);
        if mode == 1 || mode == 4 { assert!(result.is_err(), "surface loss replacement error"); assert!(result.unwrap_err().contains("UnresolvableValue")); }
        else if mode == 2 || mode == 5 { assert!(dm.awaiting_choice()); assert!(result.is_ok()); }
        else {
            assert!(result.is_ok()); assert!(!game.player(alice).unwrap().is_in_game()); assert!(game.object(source).is_none());
            assert_eq!(game.player(bob).unwrap().life, if mode == 3 { 22 } else { 27 });
            assert_eq!(dm.calls, 1); assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            if !sba { let events = game.take_pending_trigger_events(); assert_eq!(events.iter().filter(|event| event.kind() == crate::events::EventKind::PlayerLosesGame).count(), 1); }
        }
        if mode == 1 || mode == 2 || mode == 4 || mode == 5 {
            assert!(game.player(alice).unwrap().is_in_game()); assert_eq!(game.player(alice).unwrap().life, 0);
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield); assert_eq!(game.player(bob).unwrap().life, 20);
            assert_eq!(game.next_object_id_counter(), ids); assert_eq!(game.objects_in_deterministic_order().len(), objects);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some()); assert!(game.take_pending_trigger_events().is_empty()); assert!(queue.entries.is_empty());
        }
        if mode == 2 { dm.pause = false; dm.pending = false;
            assert!(perform(&mut game, &mut queue, alice, parent, sba, &mut dm).is_ok());
            assert!(!game.player(alice).unwrap().is_in_game()); assert!(game.object(source).is_none()); assert_eq!(game.player(bob).unwrap().life, 27); assert_eq!(dm.calls, 2); assert!(!dm.awaiting_choice());
        }
    }
    #[test] fn effect_additions_follow_loss_commit() { check(false, 0); }
    #[test] fn effect_added_error_restores_loss() { check(false, 1); }
    #[test] fn effect_added_pending_replays_once() { check(false, 2); }
    #[test] fn effect_addition_retains_departed_source_snapshot() { check(false, 3); }
    #[test] fn effect_instead_error_restores_loss() { check(false, 4); }
    #[test] fn effect_instead_pending_restores_loss() { check(false, 5); }
    #[test] fn sba_additions_follow_loss_commit() { check(true, 0); }
    #[test] fn sba_added_error_restores_loss() { check(true, 1); }
    #[test] fn sba_added_pending_replays_once() { check(true, 2); }
    #[test] fn sba_addition_retains_departed_source_snapshot() { check(true, 3); }
    #[test] fn sba_instead_error_restores_loss() { check(true, 4); }
    #[test] fn sba_instead_pending_restores_loss() { check(true, 5); }
}

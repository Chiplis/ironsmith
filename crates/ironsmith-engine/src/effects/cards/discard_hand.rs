//! Discard hand effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::PlayerFilter;
pub use ironsmith_core::DiscardHandEffect;

#[derive(Debug)]
struct DiscardHandProposal {
    player: PlayerId,
    cards: Vec<ObjectId>,
    revealed: Vec<ObjectId>,
}

impl crate::effects::SimultaneousEffectProposal for DiscardHandProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        game.mark_hidden_cards_publicly_revealed(&self.revealed);
        discard_hand_cards(game, ctx, self.player, self.cards)
    }
}

/// Effect that causes a player to discard their entire hand.
///
/// # Fields
///
/// * `player` - The player who discards their hand
///
/// # Example
///
/// ```ignore
/// // Discard your hand
/// let effect = DiscardHandEffect::you();
///
/// // Target player discards their hand
/// let effect = DiscardHandEffect::new(PlayerFilter::Any);
/// ```
impl EffectExecutor for DiscardHandEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        use crate::decisions::context::SelectionRevealPolicy;
        use crate::decisions::{make_decision, specs::ChooseObjectsSpec};
        let player = resolve_player_filter(game, &self.player, ctx)?;
        let cards = game
            .player(player)
            .map(|p| p.hand.to_vec())
            .unwrap_or_default();
        let private: Vec<_> = cards
            .iter()
            .copied()
            .filter(|id| *id != ctx.source && game.hidden_identity_is_private(*id))
            .collect();
        // Prepare every owner's required public opening before any hand moves.
        // The proposal freezes the original hand, so discard replacements that
        // draw cards cannot add those new cards to this instruction's discard.
        let revealed = if private.is_empty() {
            Vec::new()
        } else {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Reveal the cards you discard".to_string(),
                private.clone(),
                private.len(),
                Some(private.len()),
            )
            .require_explicit_choice()
            .with_selection_reveal_policy(SelectionRevealPolicy::Public);
            let chosen: Vec<ObjectId> =
                make_decision(game, ctx.decision_maker, player, Some(ctx.source), spec);
            chosen
                .into_iter()
                .filter(|id| private.contains(id))
                .collect()
        };
        Ok(Box::new(DiscardHandProposal {
            player,
            cards,
            revealed,
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
        // CR 603.2c: discarding a hand is one event for "one or more" triggers.
        let opened_batch = game.open_simultaneous_action();
        let outcome = execute_discard_hand(self, game, ctx);
        game.close_simultaneous_action(opened_batch);
        outcome
    }

    fn cost_description(&self) -> Option<String> {
        Some("Discard your hand".to_string())
    }
}

fn execute_discard_hand(
    this: &DiscardHandEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    let player_id = resolve_player_filter(game, &this.player, ctx)?;

    let hand_cards: Vec<_> = game
        .player(player_id)
        .map(|p| p.hand.to_vec())
        .unwrap_or_default();

    // Hidden-information matches: the owner reveals the hand publicly
    // before any card moves, so every peer applies Madness (CR 702.35a)
    // and discard triggers to the same identities (see
    // `game_state::hidden_hand_choices`). Never prompts otherwise.
    if hand_cards
        .iter()
        .any(|id| game.hidden_identity_is_private(*id))
    {
        let to_reveal: Vec<_> = hand_cards
            .iter()
            .copied()
            .filter(|id| *id != ctx.source)
            .collect();
        if game
            .reveal_private_hidden_cards_publicly(
                &mut *ctx.decision_maker,
                player_id,
                ctx.source,
                &to_reveal,
                "Reveal the cards you discard",
                false,
            )
            .is_none()
        {
            return Ok(EffectOutcome::count(0));
        }
    }

    discard_hand_cards(game, ctx, player_id, hand_cards)
}

pub(crate) fn discard_hand_cards(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    cards: Vec<ObjectId>,
) -> Result<EffectOutcome, ExecutionError> {
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = discard_hand_cards_inner(game, ctx, player, cards);
    if result.is_err() || ctx.decision_maker.awaiting_choice() {
        *game = checkpoint;
        context_checkpoint.restore(ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
    }
    result
}

fn discard_hand_cards_inner(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    cards: Vec<ObjectId>,
) -> Result<EffectOutcome, ExecutionError> {
    let mut count = 0;
    let mut successful_discards = Vec::new();
    let mut receipts = Vec::new();
    for card in cards {
        if !game.player(player).is_some_and(|p| p.hand.contains(&card)) {
            continue;
        }
        let receipt = crate::events::processing::execute_discard_with_scope(
            game,
            card,
            player,
            ctx.cause.clone(),
            false,
            ctx.provenance,
            &mut *ctx.decision_maker,
            &ctx.replacement,
            ctx.source_snapshot.as_ref(),
        )?;
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        if !receipt.result.prevented && receipt.result.new_id.is_some() {
            count += 1;
            let event = receipt.resolved_event.as_ref()
                .ok_or_else(|| ExecutionError::InternalError("completed discard has no resolved event".into()))?;
            // Discard currently does not support target/player redirection.
            // Refuse an incompatible result rather than publishing an authored
            // player/cause that differs from the actual committed event.
            if event.player != player || event.cause != ctx.cause {
                return Err(ExecutionError::InternalError("discard-hand receipt changed its batch player or cause".into()));
            }
            successful_discards.push((event.card, receipt.discarded_snapshot.clone(), receipt.result.final_zone, receipt.result.new_id));
        }
        receipts.push(receipt);
    }
    let events = super::discard::completed_discard_events(
        game, player, ctx.cause.clone(), ctx.provenance, successful_discards,
    );
    super::discard::finish_discard_receipts(game, ctx, EffectOutcome::count(count).with_events(events), receipts)
}

impl CostExecutableEffect for DiscardHandEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        _source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        let player = match self.player {
            PlayerFilter::You => controller,
            PlayerFilter::Specific(id) => id,
            _ => {
                return Err(crate::effects::CostValidationError::Other(
                    "discard-hand cost supports only 'you' or a specific player".to_string(),
                ));
            }
        };

        if game.player(player).is_some() {
            Ok(())
        } else {
            Err(crate::effects::CostValidationError::Other(
                "player not found".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, CardBuilder};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

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
        game.add_object(obj); // add_object automatically updates player.hand for Zone::Hand
        id
    }

    #[test]
    fn test_discard_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Add 3 cards to hand
        add_card_to_hand(&mut game, "Card 1", alice);
        add_card_to_hand(&mut game, "Card 2", alice);
        add_card_to_hand(&mut game, "Card 3", alice);

        assert_eq!(game.player(alice).unwrap().hand.len(), 3);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DiscardHandEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
        assert_eq!(game.player(alice).unwrap().hand.len(), 0);
    }

    #[test]
    fn test_discard_hand_empty() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        assert!(game.player(alice).unwrap().hand.is_empty());

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DiscardHandEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_discard_hand_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        // Add cards to both players' hands
        add_card_to_hand(&mut game, "Alice Card", alice);
        add_card_to_hand(&mut game, "Bob Card 1", bob);
        add_card_to_hand(&mut game, "Bob Card 2", bob);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DiscardHandEffect::new(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 1); // Alice's hand unchanged
        assert_eq!(game.player(bob).unwrap().hand.len(), 0);
    }

    #[test]
    fn simultaneous_discard_hand_freezes_the_prepared_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        add_card_to_hand(&mut game, "First", alice);
        add_card_to_hand(&mut game, "Second", alice);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let proposal = DiscardHandEffect::you()
            .prepare_simultaneous_player_action(&game, &mut ctx)
            .unwrap();
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            2,
            "preparation cannot discard"
        );
        let later_card = add_card_to_hand(&mut game, "Arrived after preparation", alice);
        let outcome = proposal.commit(&mut game, &mut ctx).unwrap();
        assert_eq!(outcome.count_or_zero(), 2);
        assert_eq!(game.player(alice).unwrap().hand.as_slice(), &[later_card]);
    }

    #[test]
    fn discard_hand_counts_only_unprevented_discards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        add_card_to_hand(&mut game, "Retained", alice);
        game.effect_store.replacement_effects.add_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::WouldDiscardMatcher::you(),
                crate::replacement::ReplacementAction::Prevent,
            ),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = DiscardHandEffect::you()
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    }

    #[test]
    fn test_discard_hand_clone_box() {
        let effect = DiscardHandEffect::you();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("DiscardHandEffect"));
    }
}

#[cfg(test)]
mod additional_contract_tests {
    use super::*;
    use crate::effect::{Effect,Value};
    use crate::ids::CardId;
    use crate::replacement::{ReplacementAction,ReplacementEffect};
    struct PauseAdded { pause:bool,pending:bool,questions:usize }
    impl crate::decision::DecisionMaker for PauseAdded {
        fn decide_boolean(&mut self,_game:&GameState,_ctx:&crate::decisions::context::BooleanContext)->bool {
            self.questions+=1;if self.pause { self.pending=true;false }else{true}
        }
        fn awaiting_choice(&self)->bool {self.pending}
    }
    fn check_additional_discard_hand(mode:u8) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();
        let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
        let card=crate::card::CardBuilder::new(CardId::new(),"Discard addition probe")
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(4,4)).build();
        let first=game.create_object_from_card(&card,alice,crate::zone::Zone::Hand);
        let second=game.create_object_from_card(&card,alice,crate::zone::Zone::Hand);
        let source=game.create_object_from_card(&card,bob,crate::zone::Zone::Battlefield);
        let effects=if mode==3 {vec![Effect::new(crate::effects::PutCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,1,crate::target::ChooseSpec::tagged("it")))]}
        else {
            let mut effects=vec![Effect::gain_life(2)];
            if mode==1 {effects.push(Effect::lose_life(Value::X));}
            if mode==2 {effects.push(Effect::may(vec![Effect::gain_life(4)]));}
            effects.push(Effect::gain_life(8));effects
        };
        let shield=game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source,bob,
            crate::events::cards::matchers::WouldDiscardMatcher::any_player()
                .with_card_filter(crate::target::ObjectFilter::specific(first)),ReplacementAction::Additionally(effects)));
        let hand=game.player(alice).unwrap().hand.clone();
        let parent_tag=crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(),&game);
        game.take_pending_trigger_events();let before_id=game.next_object_id_counter();
        let mut dm=PauseAdded {pause:mode==2,pending:false,questions:0};
        let mut ctx=ExecutionContext::new_default(source,alice).with_decision_maker(&mut dm);
        ctx.set_tagged_objects("it",vec![parent_tag.clone()]);
        let result=DiscardHandEffect::you().execute(&mut game,&mut ctx);
        if mode==1 {assert!(matches!(result,Err(ExecutionError::UnresolvableValue(_))));}
        else {
            let outcome=result.unwrap();
            if mode==2 {assert!(ctx.decision_maker.awaiting_choice());assert!(outcome.events.is_empty());}
            else {
                assert_eq!(outcome.count_or_zero(),2);
                assert!(game.player(alice).unwrap().hand.is_empty());
                if mode==3 {
                    let arrived=game.current_object_id_after_zone_change(first).unwrap();
                    assert_eq!(game.object(arrived).unwrap().counters.get(&crate::object::CounterType::PlusOnePlusOne),Some(&1));
                    assert!(!game.object(source).unwrap().counters.contains_key(&crate::object::CounterType::PlusOnePlusOne));
                }else{assert_eq!(game.player(bob).unwrap().life,30);}
                let mut events=game.take_pending_trigger_events();events.extend(outcome.events);
                let discards=events.iter().filter_map(|event|event.downcast::<crate::events::CardDiscardedEvent>()).collect::<Vec<_>>();
                assert_eq!(discards.len(),2);
                assert!(discards.iter().all(|discard|discard.player==alice));
            }
        }
        assert_eq!(ctx.source,source);assert_eq!(ctx.controller,alice);
        assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id,parent_tag.object_id);
        assert!(ctx.replacement.suppressed_replacement_effects.is_empty());drop(ctx);
        assert_eq!(game.player(alice).unwrap().life,20);
        assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(),mode==1||mode==2);
        if mode==1||mode==2 {
            assert_eq!(game.player(alice).unwrap().hand,hand);assert_eq!(game.next_object_id_counter(),before_id);
            assert!(game.object(first).is_some());assert!(game.object(second).is_some());
            assert_eq!(game.player(bob).unwrap().life,20);assert!(game.take_pending_trigger_events().is_empty());
        }
        if mode==2 {
            assert_eq!(dm.questions,1);let mut replay=PauseAdded {pause:false,pending:false,questions:0};
            let mut ctx=ExecutionContext::new_default(source,alice).with_decision_maker(&mut replay);
            ctx.set_tagged_objects("it",vec![parent_tag]);let outcome=DiscardHandEffect::you().execute(&mut game,&mut ctx).unwrap();drop(ctx);
            assert_eq!(replay.questions,1);assert_eq!(outcome.count_or_zero(),2);assert!(game.player(alice).unwrap().hand.is_empty());
            assert_eq!(game.player(bob).unwrap().life,34);assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            let mut events=game.take_pending_trigger_events();events.extend(outcome.events);
            assert_eq!(events.iter().filter_map(|event|event.downcast::<crate::events::LifeGainEvent>()).map(|gain|gain.amount).collect::<Vec<_>>(),vec![2,4,8]);
            assert_eq!(events.iter().filter_map(|event|event.downcast::<crate::events::CardDiscardedEvent>()).count(),2);
        }
    }
    #[test] fn additional_discard_hand_preserves_primary_count_and_payload() {check_additional_discard_hand(0);}
    #[test] fn additional_discard_hand_error_restores_batch_and_prefix() {check_additional_discard_hand(1);}
    #[test] fn additional_discard_hand_pending_restores_then_replays_once() {check_additional_discard_hand(2);}
    #[test] fn additional_discard_hand_binds_arriving_card_without_changing_parent_tag() {check_additional_discard_hand(3);}
}

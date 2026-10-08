//! Behold mechanic effect.
//!
//! This is a custom mechanic used in some card sets in this repository.
//!
//! Reminder text example:
//! "To behold an Elemental, choose an Elemental you control or reveal an Elemental card from your hand."

use crate::effect::EffectOutcome;
use crate::effects::helpers::normalize_object_selection;
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::{FilterContext, ObjectFilter};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::PlayerFilter;
use crate::types::Subtype;
use crate::zone::Zone;

/// Effect that "beholds" one or more objects of a given subtype.
///
/// For each behold, the player chooses a matching object they control on the battlefield
/// or reveals a matching card from their hand.
///
/// Revealing a hand card publishes it to every player. In peer matches, where
/// other peers hold placeholders for hidden hand cards, the choice follows
/// `game_state::hidden_hand_choices`.
#[derive(Debug, Clone, PartialEq)]
pub struct BeholdEffect {
    pub subtype: Subtype,
    pub count: u32,
    pub chooser: PlayerFilter,
}

impl BeholdEffect {
    pub fn new(subtype: Subtype, count: u32, chooser: PlayerFilter) -> Self {
        Self {
            subtype,
            count,
            chooser,
        }
    }

    pub fn you(subtype: Subtype, count: u32) -> Self {
        Self::new(subtype, count, PlayerFilter::You)
    }
}

/// The hand part of a behold choice: a card of `subtype` in the chooser's hand.
fn hand_filter(chooser: PlayerId, subtype: Subtype) -> ObjectFilter {
    ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::Specific(chooser))
        .with_subtype(subtype)
}

/// Objects the chooser may behold, and whether the choice depends on hidden
/// hand identities (peer matches; symmetric across peers, see
/// `game_state::hidden_hand_choices`). When it does, this peer's hidden-card
/// placeholders stay choosable: only the owner knows whether they match.
fn candidates(
    game: &GameState,
    chooser: PlayerId,
    source: ObjectId,
    subtype: Subtype,
) -> (Vec<ObjectId>, bool) {
    let mut out = Vec::new();

    out.extend(
        game.battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(id, _)| {
                !game.is_phased_out(*id)
                    && game.current_controller(*id) == Some(chooser)
                    && game.current_has_subtype(*id, subtype)
            })
            .map(|(id, _)| id),
    );

    let mut hidden_hand_choice = false;
    if let Some(player) = game.player(chooser) {
        let hand: Vec<ObjectId> = player
            .hand
            .iter()
            .copied()
            .filter(|id| *id != source)
            .collect();
        let filter = hand_filter(chooser, subtype);
        hidden_hand_choice =
            game.hand_choice_depends_on_hidden_identity(&filter, hand.iter().copied());
        let placeholders = if hidden_hand_choice {
            let filter_ctx = FilterContext::new(chooser).with_source(source);
            game.hidden_hand_placeholder_candidates(&filter, &filter_ctx, hand.iter().copied())
        } else {
            Vec::new()
        };
        out.extend(
            hand.into_iter()
                .filter(|id| placeholders.contains(id) || game.current_has_subtype(*id, subtype)),
        );
    }

    (out, hidden_hand_choice)
}

impl EffectExecutor for BeholdEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
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
        super::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| self.execute_behold_with_outputs(game, ctx),
        )
    }

    fn cost_description(&self) -> Option<String> {
        let subtype_name = self.subtype.to_string();
        if self.count == 1 {
            return Some(format!("Behold a {}", subtype_name));
        }
        Some(format!("Behold {} {}s", self.count, subtype_name))
    }
}

impl BeholdEffect {
    fn execute_behold_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        use crate::decisions::context::SelectionRevealPolicy;
        use crate::decisions::make_decision;
        use crate::decisions::specs::ChooseObjectsSpec;

        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let required = self.count as usize;
        if required == 0 {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::resolved(),
            ));
        }

        let (pool, hidden_hand_choice) = candidates(game, chooser, ctx.source, self.subtype);
        // With hidden hand cards involved, the local pool differs between the
        // owner and the peers holding placeholders, so neither a shortfall
        // nor a lone candidate may decide anything: every peer asks the same
        // question and replays the owner's answer.
        if pool.len() < required && !hidden_hand_choice {
            if ctx.optional_action {
                // "You may behold a Dragon. If you do, ...": beholding nothing
                // is simply not doing it.
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::impossible(),
                ));
            }
            return Err(ExecutionError::Impossible(format!(
                "Not enough objects to behold ({} needed, {} available)",
                required,
                pool.len()
            )));
        }

        let chosen = if pool.len() == required && !hidden_hand_choice {
            pool.clone()
        } else {
            let subtype_name = self.subtype.to_string().to_ascii_lowercase();
            let mut spec = ChooseObjectsSpec::new(
                ctx.source,
                format!("Choose {} {} to behold", required, subtype_name),
                pool.clone(),
                required,
                Some(required),
            );
            if hidden_hand_choice {
                // A chosen hand card is revealed: the owner opens it on every
                // peer before the answer is replayed.
                spec = spec
                    .require_explicit_choice()
                    .with_selection_reveal_policy(SelectionRevealPolicy::Public);
                if ctx.optional_action {
                    // The owner may hold fewer matching cards than a peer's
                    // placeholder count suggests.
                    spec = spec.allow_partial_completion();
                }
            }
            make_decision(game, ctx.decision_maker, chooser, Some(ctx.source), spec)
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let chosen = if hidden_hand_choice {
            // No fill-up: it would pick different cards on peers holding
            // placeholders.
            let mut normalized = Vec::new();
            for id in chosen {
                if normalized.len() < required && pool.contains(&id) && !normalized.contains(&id) {
                    normalized.push(id);
                }
            }
            normalized
        } else {
            normalize_object_selection(chosen, &pool, required)
        };

        let revealed_from_hand: Vec<_> = chosen
            .iter()
            .copied()
            .filter(|id| {
                game.player(chooser)
                    .is_some_and(|player| player.hand.contains(id))
            })
            .collect();
        if hidden_hand_choice {
            let filter = hand_filter(chooser, self.subtype);
            let filter_ctx = FilterContext::new(chooser).with_source(ctx.source);
            let description = format!("behold a {}", self.subtype);
            let hidden_chosen: Vec<ObjectId> = revealed_from_hand
                .iter()
                .copied()
                .filter(|id| game.is_hidden_tracked_hand_card(*id))
                .collect();
            game.record_hidden_identity_obligations(
                &hidden_chosen,
                &filter,
                &filter_ctx,
                &description,
            );
            // Every peer opened the chosen hand cards before this replay.
            game.mark_hidden_cards_publicly_revealed(&revealed_from_hand);
        }
        if chosen.len() < required {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::impossible(),
            ));
        }

        let chosen_memory = chosen
            .iter()
            .filter_map(|id| crate::snapshot::ObjectSnapshot::from_object_id(game, *id))
            .collect::<Vec<_>>();
        let revealed = chosen_memory
            .iter()
            .filter(|snapshot| revealed_from_hand.contains(&snapshot.object_id))
            .cloned()
            .collect();
        let reveal = crate::effects::cards::reveal_objects_with_outputs(
            game,
            ctx,
            revealed,
            Some(chooser),
            "Reveal cards from hand",
            None,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        Ok(crate::effects::CompletedEffectOutputs::from_children(
            [reveal],
            |children| {
                EffectOutcome::aggregate_with_primary_result(
                    EffectOutcome::with_objects(chosen).with_chosen_object_memory(chosen_memory),
                    children,
                )
            },
        ))
    }
}

impl CostExecutableEffect for BeholdEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        use crate::effects::CostValidationError;

        let chooser = match self.chooser {
            PlayerFilter::You => controller,
            PlayerFilter::Specific(id) => id,
            _ => controller,
        };

        let available = candidates(game, chooser, source, self.subtype).0.len() as u32;
        if available < self.count {
            return Err(CostValidationError::Other(format!(
                "Not enough {}s to behold ({} needed, {} available)",
                self.subtype.to_string().to_ascii_lowercase(),
                self.count,
                available
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn simple_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        subtype: Subtype,
        zone: Zone,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![subtype])
            .build();
        game.create_object_from_card(&card, controller, zone)
    }

    #[derive(Debug, Default)]
    struct CaptureViewDm {
        calls: Vec<(PlayerId, PlayerId, Zone, bool, Vec<ObjectId>)>,
    }

    impl DecisionMaker for CaptureViewDm {
        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.calls
                .push((viewer, ctx.subject, ctx.zone, ctx.public, cards.to_vec()));
        }
    }

    #[test]
    fn test_behold_validates_candidates_across_battlefield_and_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // One Elemental on battlefield and one in hand.
        let _bf = simple_creature(
            &mut game,
            "BF Elemental",
            alice,
            Subtype::Elemental,
            Zone::Battlefield,
        );
        let _hand = simple_creature(
            &mut game,
            "Hand Elemental",
            alice,
            Subtype::Elemental,
            Zone::Hand,
        );

        let effect = BeholdEffect::you(Subtype::Elemental, 2);
        assert!(
            crate::effects::EffectExecutor::can_execute_as_cost(&effect, &game, source, alice)
                .is_ok()
        );
    }

    #[test]
    fn test_behold_errors_when_insufficient_candidates() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let _hand = simple_creature(
            &mut game,
            "Hand Elemental",
            alice,
            Subtype::Elemental,
            Zone::Hand,
        );

        let effect = BeholdEffect::you(Subtype::Elemental, 2);
        assert!(
            crate::effects::EffectExecutor::can_execute_as_cost(&effect, &game, source, alice)
                .is_err()
        );
    }

    #[test]
    fn test_behold_reveals_hand_cards_publicly_when_chosen() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let hand = simple_creature(&mut game, "Hand Dragon", alice, Subtype::Dragon, Zone::Hand);

        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![crate::effects::ResolvedTarget::Object(hand)]);

        BeholdEffect::you(Subtype::Dragon, 1)
            .execute(&mut game, &mut ctx)
            .expect("behold from hand should execute");

        assert_eq!(
            dm.calls.len(),
            2,
            "all players should see the revealed hand card"
        );
        assert!(dm.calls.iter().all(|(_, subject, zone, public, cards)| {
            *subject == alice && *zone == Zone::Hand && *public && cards == &vec![hand]
        }));
    }

    /// Answers every object choice with its first `take` candidates and
    /// records the contexts it was asked.
    #[derive(Debug, Default)]
    struct RecordingChooseDm {
        take: usize,
        asked: Vec<crate::decisions::context::SelectObjectsContext>,
    }

    impl DecisionMaker for RecordingChooseDm {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.asked.push(ctx.clone());
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(self.take)
                .collect()
        }
    }

    fn track_as_hidden_hand_card(game: &mut GameState, id: ObjectId, owner: PlayerId, slot: u16) {
        game.set_hidden_card_info(
            id,
            crate::game_state::HiddenCardInfo {
                incarnation: Some(0),
                owner,
                zone: Zone::Hand,
                slot,
                commitment: format!("slot-{slot}"),
                origin_slot: Some(slot),
                origin_commitment: Some(format!("slot-{slot}")),
                public_slot: None,
                public_commitment: None,
            },
        );
    }

    #[test]
    fn test_behold_offers_hidden_hand_placeholder_on_peer() {
        // The guest's engine holds a placeholder for the host's hand Dragon.
        // It must replay the host's reveal instead of failing with "0
        // available" (Sarkhan, Dragon Ascendant desync).
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let placeholder =
            game.create_hidden_card_placeholder(alice, Zone::Hand, 3, "slot-3".to_string());

        let mut dm = RecordingChooseDm {
            take: 1,
            ..Default::default()
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.optional_action = true;
        let outcome = BeholdEffect::you(Subtype::Dragon, 1)
            .execute(&mut game, &mut ctx)
            .expect("behold of a placeholder should replay");

        assert_eq!(
            outcome.value,
            crate::effect::OutcomeValue::Objects(vec![placeholder])
        );
        assert_eq!(dm.asked.len(), 1);
        let asked = &dm.asked[0];
        assert!(asked.require_explicit_choice && asked.allow_partial_completion);
        assert_eq!(
            asked.reveal_policy,
            crate::decisions::context::SelectionRevealPolicy::Public,
            "the chosen hand card must be opened on every peer"
        );
        assert!(game.is_publicly_revealed_hidden_card(placeholder));
    }

    #[test]
    fn test_behold_never_auto_picks_lone_hidden_hand_candidate_on_owner() {
        // The owner knows its lone Dragon; auto-picking it would skip the
        // decision (and its public opening) that peers must replay.
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let dragon = simple_creature(&mut game, "Hand Dragon", alice, Subtype::Dragon, Zone::Hand);
        track_as_hidden_hand_card(&mut game, dragon, alice, 0);

        let mut dm = RecordingChooseDm {
            take: 1,
            ..Default::default()
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.optional_action = true;
        let outcome = BeholdEffect::you(Subtype::Dragon, 1)
            .execute(&mut game, &mut ctx)
            .expect("owner behold should execute");

        assert_eq!(
            outcome.value,
            crate::effect::OutcomeValue::Objects(vec![dragon])
        );
        assert_eq!(
            dm.asked.len(),
            1,
            "a lone hidden hand candidate is still asked"
        );
        assert_eq!(
            dm.asked[0].reveal_policy,
            crate::decisions::context::SelectionRevealPolicy::Public
        );
    }

    #[test]
    fn test_optional_behold_without_candidates_fails_instead_of_erroring() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut dm = RecordingChooseDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.optional_action = true;
        let outcome = BeholdEffect::you(Subtype::Dragon, 1)
            .execute(&mut game, &mut ctx)
            .expect("accepting 'you may behold' with nothing to behold must not abort resolution");
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Impossible);
    }
}

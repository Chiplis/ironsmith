//! Numbered-card draw triggers such as "your second card each turn" and
//! "your first or second card each turn".

use crate::effects::ExecutionError;
use crate::events::EventKind;
use crate::events::other::CardsDrawnEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_subject};

/// Trigger for "Whenever [player] draws their Nth card each turn".
///
/// This fires once when the draw event includes the configured draw number.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDrawsNthCardEachTurnTrigger {
    pub player: PlayerFilter,
    pub card_number: u32,
}

/// Trigger for any of a reusable set of numbered draws each turn.
///
/// Unlike an `OrTrigger` of single-number matchers, this matcher preserves
/// multiplicity when one batched draw event crosses more than one configured
/// ordinal.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDrawsNumberedCardsEachTurnTrigger {
    pub player: PlayerFilter,
    pub card_numbers: Vec<u32>,
}

impl PlayerDrawsNthCardEachTurnTrigger {
    pub fn new(player: PlayerFilter, card_number: u32) -> Self {
        Self {
            player,
            card_number,
        }
    }
}

impl PlayerDrawsNumberedCardsEachTurnTrigger {
    pub fn new(player: PlayerFilter, card_numbers: impl IntoIterator<Item = u32>) -> Self {
        let mut card_numbers = card_numbers
            .into_iter()
            .filter(|number| *number > 0)
            .collect::<Vec<_>>();
        card_numbers.sort_unstable();
        card_numbers.dedup();
        Self {
            player,
            card_numbers,
        }
    }

    fn matching_card_numbers(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        let result = (|| {
            let Some((total_before, total_after)) = draw_number_window(&self.player, event, ctx)? else {
                return Ok(0);
            };
            let count = self.card_numbers
                .iter()
                .filter(|number| total_before < i64::from(**number) && i64::from(**number) <= total_after)
                .count();
            u32::try_from(count).map_err(|_| ExecutionError::ResourceLimitExceeded {
                resource: "numbered draw trigger count",
                requested: count as u128,
                maximum: u32::MAX as u128,
            })
        })();
        match result {
            Ok(count) => count,
            Err(error) => { ctx.game.record_token_resource_failure(&error); 0 }
        }
    }
}

/// Accumulate only the prefix ending at this exact draw occurrence. Later
/// staged events belong to later windows, even when collected in one batch.
fn checked_draw_number_window(
    draws: impl IntoIterator<Item = (usize, bool)>,
) -> Result<(i64, i64), ExecutionError> {
    let mut before = 0i64;
    for (count, is_current) in draws {
        let requested = before as u128 + count as u128;
        let after = i64::try_from(requested).map_err(|_| ExecutionError::ResourceLimitExceeded {
            resource: "draw-history ordinal",
            requested,
            maximum: i64::MAX as u128,
        })?;
        if is_current {
            return Ok((before, after));
        }
        before = after;
    }
    Err(ExecutionError::IncompleteEvidence(
        "numbered draw trigger lacks its exact retained draw occurrence; native recovery or replay required".into(),
    ))
}

fn draw_number_window(
    player: &PlayerFilter,
    event: &TriggerEvent,
    ctx: &TriggerContext,
) -> Result<Option<(i64, i64)>, ExecutionError> {
    if event.kind() != EventKind::CardsDrawn {
        return Ok(None);
    }
    let e = event.downcast::<CardsDrawnEvent>().ok_or_else(|| ExecutionError::IncompleteEvidence(
        "numbered draw trigger lacks its draw event payload".into(),
    ))?;
    if !crate::filter::player_filter_matches_game(player, e.player, ctx.game, &ctx.filter_ctx) {
        return Ok(None);
    }
    if ctx.game.player(e.player).is_none() {
        return Err(ExecutionError::IncompleteEvidence(
            "numbered draw trigger lacks its drawing player".into(),
        ));
    }
    let records = ctx.game.turn_store.turn_history.ordered_draw_occurrences()?;
    let retained = records.iter()
        .find(|record| record.event.ptr_eq(event))
        .and_then(|record| record.event.downcast::<CardsDrawnEvent>())
        .ok_or_else(|| ExecutionError::IncompleteEvidence(
            "numbered draw trigger lacks its exact retained draw occurrence; native recovery or replay required".into(),
        ))?;
    if retained.player != e.player || retained.cards != e.cards {
        return Err(ExecutionError::IncompleteEvidence(
            "numbered draw trigger payload differs from its retained draw occurrence".into(),
        ));
    }
    checked_draw_number_window(records.iter().filter_map(|record| {
        let draw = record.event.downcast::<CardsDrawnEvent>()?;
        (draw.player == e.player).then_some((draw.cards.len(), record.event.ptr_eq(event)))
    })).map(Some)
}

fn numbered_draw_display(player: &PlayerFilter, card_numbers: &[u32]) -> String {
    let ordinals = card_numbers
        .iter()
        .map(|number| {
            ironsmith_core::ordinal_word(*number).unwrap_or_else(|| format!("{number}th"))
        })
        .collect::<Vec<_>>();
    let ordinal_text = match ordinals.as_slice() {
        [] => "numbered".to_string(),
        [ordinal] => ordinal.clone(),
        [first, second] => format!("{first} or {second}"),
        many => format!(
            "{}, or {}",
            many[..many.len() - 1].join(", "),
            many.last().expect("numbered draw list is nonempty")
        ),
    };
    match player {
        PlayerFilter::You => format!("Whenever you draw your {ordinal_text} card each turn"),
        PlayerFilter::Any => {
            format!("Whenever a player draws their {ordinal_text} card each turn")
        }
        PlayerFilter::Opponent => {
            format!("Whenever an opponent draws their {ordinal_text} card each turn")
        }
        PlayerFilter::Active => {
            format!("Whenever a player draws their {ordinal_text} card during their turn")
        }
        PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => {
            format!("Whenever that player draws their {ordinal_text} card each turn")
        }
        _ => format!(
            "Whenever {} draws their {ordinal_text} card each turn",
            describe_player_filter_subject(player)
        ),
    }
}

impl TriggerMatcher for PlayerDrawsNthCardEachTurnTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if self.card_number == 0 {
            return false;
        }
        match draw_number_window(&self.player, event, ctx) {
            Ok(Some((total_before, total_after))) => {
                total_before < i64::from(self.card_number) && i64::from(self.card_number) <= total_after
            }
            Ok(None) => false,
            // Existing checked trigger collection owns the failure scope and
            // returns this error instead of interpreting a skipped match as zero.
            Err(error) => { ctx.game.record_token_resource_failure(&error); false }
        }
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CardsDrawn])
    }

    fn display(&self) -> String {
        let ordinal =
            ironsmith_core::ordinal_word(self.card_number).unwrap_or_else(|| "nth".to_string());
        match &self.player {
            PlayerFilter::You => format!("Whenever you draw your {ordinal} card each turn"),
            PlayerFilter::Any => format!("Whenever a player draws their {ordinal} card each turn"),
            // Only the active player's own draws: "during their turn".
            PlayerFilter::Active => {
                format!("Whenever a player draws their {ordinal} card during their turn")
            }
            PlayerFilter::Opponent => {
                format!("Whenever an opponent draws their {ordinal} card each turn")
            }
            PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => {
                format!("Whenever that player draws their {ordinal} card each turn")
            }
            _ => format!(
                "Whenever {} draws their {ordinal} card each turn",
                describe_player_filter_subject(&self.player)
            ),
        }
    }
}

impl TriggerMatcher for PlayerDrawsNumberedCardsEachTurnTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.matching_card_numbers(event, ctx) > 0
    }

    fn trigger_count_with_context(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        self.matching_card_numbers(event, ctx)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CardsDrawn])
    }

    fn display(&self) -> String {
        numbered_draw_display(&self.player, &self.card_numbers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    fn observer(game: &mut GameState, player: PlayerId, numbered: bool) -> ObjectId {
        let trigger = if numbered {
            crate::triggers::Trigger::new(PlayerDrawsNumberedCardsEachTurnTrigger::new(
                PlayerFilter::You, [1, 2],
            ))
        } else {
            crate::triggers::Trigger::player_draws_nth_card_each_turn(PlayerFilter::You, 2)
        };
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Draw observer")
            .card_types(vec![crate::types::CardType::Enchantment])
            .with_ability(crate::Ability::triggered(trigger, vec![crate::Effect::gain_life(1)]))
            .build();
        game.create_object_from_definition(&definition, player, crate::zone::Zone::Battlefield)
    }

    #[test]
    fn ordinal_window_stays_wide_and_ends_at_the_exact_occurrence() {
        let before = usize::try_from(u32::MAX).unwrap();
        assert_eq!(checked_draw_number_window([
            (before, false), (1, true), (usize::MAX, false),
        ]).unwrap(), (i64::from(u32::MAX), i64::from(u32::MAX) + 1));
        assert!(matches!(checked_draw_number_window([(1, false)]),
            Err(ExecutionError::IncompleteEvidence(_))));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn ordinal_window_reports_wide_overflow_instead_of_saturating() {
        assert!(matches!(checked_draw_number_window([
            (usize::try_from(i64::MAX).unwrap(), false), (1, true),
        ]), Err(ExecutionError::ResourceLimitExceeded {
            resource: "draw-history ordinal", requested, maximum,
        }) if requested == i64::MAX as u128 + 1 && maximum == i64::MAX as u128));
    }

    #[test]
    fn collected_draw_batch_preserves_each_players_exact_event_windows() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        observer(&mut game, alice, false);
        let events = [
            CardsDrawnEvent::single(alice, ObjectId::from_raw(10), true),
            CardsDrawnEvent::new(bob, vec![ObjectId::from_raw(11), ObjectId::from_raw(12)], true),
            CardsDrawnEvent::single(alice, ObjectId::from_raw(13), false),
            CardsDrawnEvent::single(alice, ObjectId::from_raw(14), false),
        ].map(|draw| TriggerEvent::new_with_provenance(draw, Default::default()));
        for event in &events { game.stage_turn_history_event(event); }
        let (root, meter) = game.begin_token_resource_scope();
        let counts: Vec<_> = crate::triggers::check_triggers_batch(&game, &events)
            .iter().map(Vec::len).collect();
        assert_eq!(counts, vec![0, 0, 1, 0]);
        assert!(game.token_resource_failure().is_none());
        game.end_token_resource_scope(root, &meter);
    }

    #[test]
    fn native_draw_windows_survive_later_receipt_publication_before_earlier_staging() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = observer(&mut game, alice, false);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Draw witness")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        for _ in 0..2 { game.create_object_from_card(&card, alice, crate::zone::Zone::Library); }
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = crate::effects::EffectContext::new(source, alice, &mut dm);
        let first = crate::effects::execute_effect(&mut game, &crate::Effect::draw(1), &mut ctx).unwrap();
        let mut second = crate::effects::execute_effect(&mut game, &crate::Effect::draw(1), &mut ctx).unwrap();
        // A replacement original can publish its receipt while an enclosing
        // earlier draw still waits in the staged observation journal.
        crate::effects::capture_triggers_before_added_program(
            &mut game, &ctx, None, second.events.iter_mut(),
        ).unwrap();
        assert_eq!(game.effect_store.pending_trigger_entries.len(), 1);
        let first_draw = first.events.iter().find(|event| event.kind() == EventKind::CardsDrawn).unwrap();
        assert!(crate::triggers::check_triggers_checked(&game, first_draw).unwrap().is_empty());
        let second_draw = second.events.iter().find(|event| event.kind() == EventKind::CardsDrawn).unwrap();
        assert!(game.effect_store.pending_trigger_entries[0].triggering_event.ptr_eq(second_draw));
    }

    #[test]
    fn checked_collection_rejects_missing_or_reconstructed_draw_occurrences() {
        for numbered in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            observer(&mut game, alice, numbered);
            let draw = CardsDrawnEvent::new(alice,
                vec![ObjectId::from_raw(10), ObjectId::from_raw(11)], true);
            let event = TriggerEvent::new_with_provenance(draw.clone(), Default::default());
            assert!(matches!(crate::triggers::check_triggers_checked(&game, &event),
                Err(ExecutionError::IncompleteEvidence(_))));
            game.stage_turn_history_event(&event);
            let alias = event.clone();
            assert_eq!(crate::triggers::check_triggers_checked(&game, &alias).unwrap().len(),
                if numbered { 2 } else { 1 });
            let reconstructed = TriggerEvent::new_with_provenance(draw, event.provenance());
            assert!(matches!(crate::triggers::check_triggers_checked(&game, &reconstructed),
                Err(ExecutionError::IncompleteEvidence(_))));
            let altered = event.with_inner_event(CardsDrawnEvent::single(alice,
                ObjectId::from_raw(10), true));
            assert!(matches!(crate::triggers::check_triggers_checked(&game, &altered),
                Err(ExecutionError::IncompleteEvidence(_))));
        }
    }

    #[test]
    fn public_pending_draw_collection_returns_incomplete_evidence_and_restores_queue() {
        for simultaneous in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            observer(&mut game, alice, false);
            let event = TriggerEvent::new_with_provenance(CardsDrawnEvent::new(alice,
                vec![ObjectId::from_raw(10), ObjectId::from_raw(11)], true), Default::default());
            game.record_turn_history_event(&event);
            let altered = event.with_inner_event(CardsDrawnEvent::single(alice,
                ObjectId::from_raw(10), true));
            let altered = if simultaneous {
                altered.with_simultaneous_batch(Default::default())
            } else { altered };
            game.effect_store.pending_trigger_events.push(altered);
            let mut queue = crate::triggers::TriggerQueue::new();
            assert!(matches!(crate::game_loop::put_triggers_on_stack_with_dm(
                &mut game, &mut queue, &mut crate::decision::SelectFirstDecisionMaker,
            ), Err(crate::game_loop::GameLoopError::ExecutionFailed(
                ExecutionError::IncompleteEvidence(_),
            ))));
            assert!(game.stack_is_empty());
            assert!(queue.entries.is_empty());
            assert_eq!(game.effect_store.pending_trigger_events.len(), 1);
            assert_eq!(game.turn_store.turn_history.event_records.len(), 1);
            assert_eq!(game.turn_store.turn_history.event_records[0].event
                .downcast::<CardsDrawnEvent>().unwrap().cards.len(), 2);
        }
    }

    #[test]
    fn numbered_set_ignores_later_draws_after_its_retained_batch() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        observer(&mut game, alice, true);
        let batch = TriggerEvent::new_with_provenance(CardsDrawnEvent::new(alice,
            vec![ObjectId::from_raw(10), ObjectId::from_raw(11)], true), Default::default());
        let later = TriggerEvent::new_with_provenance(CardsDrawnEvent::single(alice,
            ObjectId::from_raw(12), false), Default::default());
        game.stage_turn_history_event(&batch);
        game.stage_turn_history_event(&later);
        assert_eq!(crate::triggers::check_triggers_checked(&game, &batch).unwrap().len(), 2);
        assert!(crate::triggers::check_triggers_checked(&game, &later).unwrap().is_empty());
    }

    #[test]
    fn draw_alias_enrichment_and_promotion_preserve_first_observation_order() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = observer(&mut game, alice, false);
        let first = TriggerEvent::new_with_provenance(CardsDrawnEvent::single(alice,
            ObjectId::from_raw(10), true), Default::default());
        let second = TriggerEvent::new_with_provenance(CardsDrawnEvent::single(alice,
            ObjectId::from_raw(11), false), Default::default());
        game.stage_turn_history_event(&first);
        game.stage_turn_history_event(&second);
        let enriched = first.clone().with_player_tags(std::collections::HashMap::from([
            (crate::tag::TagKey::from("drawer"), vec![alice]),
        ]));
        game.stage_turn_history_event(&enriched);
        game.record_turn_history_event(&second);
        let mut completed = vec![enriched];
        crate::effects::observe_lifecycle_completions(&mut game, &mut completed).unwrap();
        game.record_turn_history_event(&completed[0]);
        let enriched = completed[0].clone().with_player_tags(std::collections::HashMap::from([
            (crate::tag::TagKey::from("completed drawer"), vec![alice]),
        ]));
        // Exercise GameState's already-completed refresh-and-return path.
        game.stage_turn_history_event(&enriched);
        let records = game.turn_store.turn_history.ordered_draw_occurrences().unwrap();
        assert_eq!(records.len(), 2);
        assert!(records[0].event.ptr_eq(&enriched));
        assert_eq!(records[0].event.player_tags().get(&crate::tag::TagKey::from("completed drawer")), Some(&vec![alice]));
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert_eq!(draw_number_window(&PlayerFilter::You, &enriched, &ctx).unwrap(), Some((0, 1)));
        assert_eq!(draw_number_window(&PlayerFilter::You, &second, &ctx).unwrap(), Some((1, 2)));
    }

    #[test]
    fn missing_native_chronology_and_conflicting_aliases_do_not_become_zero() {
        for missing_owner in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            observer(&mut game, alice, false);
            let first = TriggerEvent::new_with_provenance(CardsDrawnEvent::single(alice,
                ObjectId::from_raw(10), true), Default::default());
            if missing_owner {
                game.turn_store.turn_history.draw_occurrences = None;
            } else {
                // Reconstructed aggregate records do not prove native order.
                game.turn_store.turn_history.event_records.push(crate::turn_history::TurnEventRecord {
                    event: first.clone(), object_snapshot: None, source_snapshot: None,
                });
            }
            let second = TriggerEvent::new_with_provenance(CardsDrawnEvent::single(alice,
                ObjectId::from_raw(11), false), Default::default());
            game.stage_turn_history_event(&second);
            assert!(matches!(crate::triggers::check_triggers_checked(&game, &second),
                Err(ExecutionError::IncompleteEvidence(_))));
        }
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        observer(&mut game, alice, false);
        let event = TriggerEvent::new_with_provenance(CardsDrawnEvent::single(alice,
            ObjectId::from_raw(10), true), Default::default());
        game.stage_turn_history_event(&event);
        let changed = event.with_inner_event(CardsDrawnEvent::new(alice,
            vec![ObjectId::from_raw(10), ObjectId::from_raw(11)], true));
        game.stage_turn_history_event(&changed);
        assert!(game.turn_store.turn_history.draw_occurrences.is_none());
        assert!(matches!(crate::triggers::check_triggers_checked(&game, &changed),
            Err(ExecutionError::IncompleteEvidence(_))));
    }

    #[test]
    fn native_checkpoint_restores_root_and_inactive_lane_draw_chronology() {
        let mut game = GameState::new((0..8).map(|index| format!("P{index}")).collect(), 20);
        game.restore_grand_melee((0..8).map(PlayerId::from_index).collect()).unwrap();
        let markers = game.grand_melee_marker_views();
        let root = markers[0].number;
        let other = markers[1].number;
        let root_player = markers[0].holder;
        let other_player = markers[1].holder;
        let root_event = TriggerEvent::new_with_provenance(CardsDrawnEvent::new(root_player,
            vec![ObjectId::from_raw(10), ObjectId::from_raw(11)], true), Default::default());
        game.stage_turn_history_event(&root_event);
        game.select_grand_melee_turn_marker(other).unwrap();
        let other_event = TriggerEvent::new_with_provenance(CardsDrawnEvent::new(other_player,
            vec![ObjectId::from_raw(12), ObjectId::from_raw(13), ObjectId::from_raw(14)], true), Default::default());
        game.stage_turn_history_event(&other_event);
        game.select_grand_melee_turn_marker(root).unwrap();
        let checkpoint = game.clone();
        game.turn_store.turn_history.draw_occurrences = None;
        game.select_grand_melee_turn_marker(other).unwrap();
        game.turn_store.turn_history.draw_occurrences = None;
        game.restore_execution_checkpoint(checkpoint.clone(), false);
        let ctx = TriggerContext::for_source(ObjectId::from_raw(99), root_player, &game);
        assert_eq!(draw_number_window(&PlayerFilter::You, &root_event, &ctx).unwrap(), Some((0, 2)));
        game.select_grand_melee_turn_marker(other).unwrap();
        let ctx = TriggerContext::for_source(ObjectId::from_raw(99), other_player, &game);
        assert_eq!(draw_number_window(&PlayerFilter::You, &other_event, &ctx).unwrap(), Some((0, 3)));
        let mut sibling = checkpoint;
        sibling.select_grand_melee_turn_marker(other).unwrap();
        sibling.turn_store.turn_history.draw_occurrences = None;
        assert_eq!(game.turn_store.turn_history.ordered_draw_occurrences().unwrap().len(), 1);
    }

    #[test]
    fn only_real_new_turn_boundaries_reestablish_complete_empty_draw_history() {
        assert!(crate::turn_history::TurnHistory::default().ordered_draw_occurrences().is_err());
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = observer(&mut game, alice, false);
        let old = TriggerEvent::new_with_provenance(CardsDrawnEvent::new(alice,
            vec![ObjectId::from_raw(10), ObjectId::from_raw(11)], true), Default::default());
        game.stage_turn_history_event(&old);
        game.next_turn();
        assert!(game.turn_store.turn_history.ordered_draw_occurrences().unwrap().is_empty());
        let fresh = TriggerEvent::new_with_provenance(CardsDrawnEvent::single(alice,
            ObjectId::from_raw(12), true), Default::default());
        game.stage_turn_history_event(&fresh);
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert_eq!(draw_number_window(&PlayerFilter::You, &fresh, &ctx).unwrap(), Some((0, 1)));
        assert!(matches!(draw_number_window(&PlayerFilter::You, &old, &ctx),
            Err(ExecutionError::IncompleteEvidence(_))));
        game.turn_store.turn_history.draw_occurrences = None;
        game.turn_store.turn_history.clear_for_new_turn();
        assert!(game.turn_store.turn_history.ordered_draw_occurrences().unwrap().is_empty());
    }

    #[test]
    fn test_display() {
        let trigger = PlayerDrawsNthCardEachTurnTrigger::new(PlayerFilter::You, 2);
        assert!(trigger.display().contains("second card each turn"));
    }

    #[test]
    fn numbered_set_display_preserves_all_ordinals() {
        let trigger = PlayerDrawsNumberedCardsEachTurnTrigger::new(PlayerFilter::You, [1, 2]);
        assert_eq!(
            trigger.display(),
            "Whenever you draw your first or second card each turn"
        );
    }

    #[test]
    fn numbered_set_matches_first_and_second_separate_draws() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let trigger = PlayerDrawsNumberedCardsEachTurnTrigger::new(PlayerFilter::You, [1, 2]);

        let first = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(2), false),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&first);
        let first_ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&first, &first_ctx));
        assert_eq!(trigger.trigger_count_with_context(&first, &first_ctx), 1);

        let second = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(3), false),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&second);
        let second_ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&second, &second_ctx));
        assert_eq!(trigger.trigger_count_with_context(&second, &second_ctx), 1);
    }

    #[test]
    fn numbered_set_counts_each_ordinal_crossed_by_one_batched_draw() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::new(
                alice,
                vec![ObjectId::from_raw(2), ObjectId::from_raw(3)],
                true,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let trigger = PlayerDrawsNumberedCardsEachTurnTrigger::new(PlayerFilter::You, [1, 2]);

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(trigger.trigger_count_with_context(&event, &ctx), 2);
    }

    #[test]
    fn test_matches_second_draw() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let prior_event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(10), true),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&prior_event);
        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(2), false),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let trigger = PlayerDrawsNthCardEachTurnTrigger::new(PlayerFilter::You, 2);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_matches_second_draw_in_two_card_draw_event() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::new(
                alice,
                vec![ObjectId::from_raw(2), ObjectId::from_raw(3)],
                true,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let trigger = PlayerDrawsNthCardEachTurnTrigger::new(PlayerFilter::You, 2);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_wrong_number() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let prior_event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::new(
                alice,
                vec![ObjectId::from_raw(10), ObjectId::from_raw(11)],
                true,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&prior_event);
        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(2), false),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let trigger = PlayerDrawsNthCardEachTurnTrigger::new(PlayerFilter::You, 2);
        assert!(!trigger.matches(&event, &ctx));
    }
}

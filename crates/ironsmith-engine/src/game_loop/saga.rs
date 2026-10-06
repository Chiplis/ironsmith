use super::*;

// ============================================================================
// Saga Support
// ============================================================================

#[derive(Debug, Clone, Copy)]
pub(crate) struct SagaProfile {
    pub controller: PlayerId,
    pub final_chapter: u32,
    pub has_read_ahead: bool,
}

pub(crate) fn final_chapter_number_from_abilities(
    abilities: &[crate::ability::Ability],
) -> Option<u32> {
    abilities
        .iter()
        .filter_map(|ability| {
            if let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind {
                triggered
                    .trigger
                    .saga_chapters()
                    .and_then(|chapters| chapters.iter().copied().max())
            } else {
                None
            }
        })
        .max()
}

pub(crate) fn final_chapter_number_with_view(
    view: &crate::derived_view::DerivedGameView<'_>,
    object_id: ObjectId,
) -> Option<u32> {
    let abilities = view.abilities_rc(object_id)?;
    final_chapter_number_from_abilities(abilities.as_ref())
}

fn saga_profile_and_chapters_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    object_id: ObjectId,
) -> Option<(SagaProfile, Option<u32>)> {
    if !view.calculated_subtypes(object_id).contains(&Subtype::Saga) {
        return None;
    }
    let final_chapter = final_chapter_number_with_view(view, object_id);
    let controller = view
        .calculated_characteristics(object_id)
        .map(|chars| chars.controller)
        .or_else(|| game.object(object_id).map(|obj| game.controller_of(obj)))?;
    let has_read_ahead = view.object_has_static_ability_id(
        object_id,
        crate::static_abilities::StaticAbilityId::ReadAhead,
    );
    Some((
        SagaProfile {
            controller,
            final_chapter: final_chapter.unwrap_or(0),
            has_read_ahead,
        },
        final_chapter,
    ))
}

/// Precombat lore applies only to Sagas that have chapter abilities.
pub(crate) fn saga_profile_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    object_id: ObjectId,
) -> Option<SagaProfile> {
    let (profile, chapter_number) = saga_profile_and_chapters_with_view(game, view, object_id)?;
    chapter_number?;
    Some(profile)
}

/// Entry lore applies to every calculated Saga, including one without chapters.
fn saga_entry_profile_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    object_id: ObjectId,
) -> Option<SagaProfile> {
    saga_profile_and_chapters_with_view(game, view, object_id).map(|(profile, _)| profile)
}

pub(crate) fn source_has_read_ahead(game: &GameState, source_id: ObjectId) -> bool {
    game.current_has_static_ability_id(
        source_id,
        crate::static_abilities::StaticAbilityId::ReadAhead,
    )
}

pub(crate) fn source_entered_battlefield_this_turn(game: &GameState, source_id: ObjectId) -> bool {
    game.object(source_id)
        .and_then(|obj| {
            game.turn_store
                .turn_history
                .object_entered_battlefield_controller_this_turn(obj.stable_id)
        })
        .is_some()
}

/// Add lore counters to Sagas at the start of the precombat main phase.
///
/// Per CR 714.3b, this applies only to Sagas the active player controls that
/// currently have one or more chapter abilities.
pub fn add_saga_lore_counters(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) -> Result<(), crate::effects::ExecutionError> {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    add_saga_lore_counters_with_dm(game, trigger_queue, &mut dm)
}

/// The caller must replay the turn-based action if a replacement choice is pending.
pub fn add_saga_lore_counters_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), crate::effects::ExecutionError> {
    let checkpoint = game.clone();
    let trigger_checkpoint = trigger_queue.clone();
    let result = (|| {
        let active_player = game.turn.active_player;
        let sagas: Vec<ObjectId> = {
            let view = crate::derived_view::DerivedGameView::new(game);
            game.battlefield
                .iter()
                .copied()
                .filter(|&id| {
                    saga_profile_with_view(game, &view, id)
                        .is_some_and(|profile| profile.controller == active_player)
                })
                .collect()
        };

        for saga_id in sagas {
            put_lore_counters_and_check_chapters_with_cause(
                game,
                saga_id,
                1,
                saga_entry_lore_cause(game, saga_id),
                trigger_queue,
                decision_maker,
            )?;
            if decision_maker.awaiting_choice() {
                return Ok(());
            }
        }
        Ok(())
    })();
    if result.is_err() || decision_maker.awaiting_choice() {
        *game = checkpoint;
        *trigger_queue = trigger_checkpoint;
    }
    result
}

/// Lore counters a Saga gets as it enters the battlefield (CR 714.3a), or
/// the chosen chapter for read ahead (CR 702.155b). None unless a calculated Saga.
fn entry_lore_counter_amount(
    game: &mut GameState,
    saga_id: ObjectId,
    decision_maker: &mut dyn DecisionMaker,
) -> Option<u32> {
    let profile = {
        let view = crate::derived_view::DerivedGameView::new(game);
        saga_entry_profile_with_view(game, &view, saga_id)
    }?;
    Some(if profile.has_read_ahead {
        choose_read_ahead_chapter(
            game,
            saga_id,
            profile.controller,
            profile.final_chapter,
            decision_maker,
        )
    } else {
        1
    })
}

/// CR 714.3a: "As a Saga without read ahead enters the battlefield, its
/// controller puts a lore counter on it." This applies to every entry
/// (cast, played, put onto the battlefield, token copies), so the central
/// battlefield-entry path calls it. The counter-placed event drives chapter
/// triggers once pending events are drained.
pub(crate) fn add_entry_lore_counters(
    game: &mut GameState,
    saga_id: ObjectId,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), crate::effects::ExecutionError> {
    if game.has_processed_saga_entry_lore(saga_id) {
        return Ok(());
    }
    let Some(amount) = entry_lore_counter_amount(game, saga_id, decision_maker) else {
        return Ok(());
    };
    if decision_maker.awaiting_choice() {
        return Ok(());
    }
    if amount == 0 {
        game.mark_saga_entry_lore_processed(saga_id);
        return Ok(());
    }
    // CR 122.6 / 614.1: the lore counters a Saga enters with are "put" on it,
    // so counter replacements and "can't have counters" effects apply.
    let amount = crate::events::processing::process_put_counters_with_event_with_dm(
        game,
        saga_id,
        CounterType::Lore,
        amount,
        saga_entry_lore_cause(game, saga_id),
        decision_maker,
    )?;
    if decision_maker.awaiting_choice() {
        return Ok(());
    }
    if let Some(event) = game.add_counters(saga_id, CounterType::Lore, amount) {
        game.queue_trigger_event(event.provenance(), event);
    }
    game.mark_saga_entry_lore_processed(saga_id);
    Ok(())
}

/// The entry lore counter (CR 714.3a) is put on by a game rule, not by an
/// effect or an intrinsic ability (contrast planeswalker loyalty, CR 306.5b,
/// and battle defense, CR 310.4b). So "if an effect would put counters"
/// doublers (Doubling Season) don't apply, while "if you would put counters"
/// ones (Vorinclex, Innkeeper's Talent) do: the Saga's controller puts it.
fn saga_entry_lore_cause(game: &GameState, saga_id: ObjectId) -> crate::events::cause::EventCause {
    let mut cause = crate::events::cause::EventCause::from_game_rule();
    if let Some(controller) = game
        .object(saga_id)
        .map(|object| game.controller_of(object))
    {
        cause.source = Some(saga_id);
        cause.source_controller = Some(controller);
    }
    cause
}

/// Legacy entry hook for callers that put a Saga onto the battlefield
/// without the central entry path. A Saga that already got its entry lore
/// counters there is left alone.
pub fn handle_saga_enters_battlefield(
    game: &mut GameState,
    saga_id: ObjectId,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), crate::effects::ExecutionError> {
    if game.has_processed_saga_entry_lore(saga_id) {
        return Ok(());
    }
    let checkpoint = game.clone();
    let trigger_checkpoint = trigger_queue.clone();
    let result = (|| {
        let Some(amount) = entry_lore_counter_amount(game, saga_id, decision_maker) else {
            return Ok(());
        };
        if decision_maker.awaiting_choice() {
            return Ok(());
        }
        let cause = saga_entry_lore_cause(game, saga_id);
        put_lore_counters_and_check_chapters_with_cause(
            game,
            saga_id,
            amount,
            cause,
            trigger_queue,
            decision_maker,
        )?;
        if !decision_maker.awaiting_choice() {
            game.mark_saga_entry_lore_processed(saga_id);
        }
        Ok(())
    })();
    if result.is_err() || decision_maker.awaiting_choice() {
        *game = checkpoint;
        *trigger_queue = trigger_checkpoint;
    }
    result
}

fn choose_read_ahead_chapter(
    game: &mut GameState,
    saga_id: ObjectId,
    controller: PlayerId,
    final_chapter: u32,
    decision_maker: &mut dyn DecisionMaker,
) -> u32 {
    if final_chapter == 0 {
        return 0;
    }
    let display_options = (1..=final_chapter)
        .enumerate()
        .map(|(idx, chapter)| {
            let label = chapter_number_to_roman(chapter)
                .map(|roman| format!("Chapter {roman}"))
                .unwrap_or_else(|| format!("Chapter {chapter}"));
            crate::decisions::spec::DisplayOption::new(idx, label)
        })
        .collect::<Vec<_>>();
    let choice_spec = crate::decisions::specs::ChoiceSpec::single(saga_id, display_options);
    let mut chosen = crate::decisions::make_decision(
        game,
        decision_maker,
        controller,
        Some(saga_id),
        choice_spec,
    );
    if decision_maker.awaiting_choice() {
        // Unwind without committing a fallback chapter; callers treat 0 as
        // "place no lore counters".
        return 0;
    }
    chosen
        .pop()
        .and_then(|idx| u32::try_from(idx + 1).ok())
        .filter(|chapter| (1..=final_chapter).contains(chapter))
        .unwrap_or(1)
}

fn chapter_number_to_roman(chapter: u32) -> Option<&'static str> {
    match chapter {
        1 => Some("I"),
        2 => Some("II"),
        3 => Some("III"),
        4 => Some("IV"),
        5 => Some("V"),
        6 => Some("VI"),
        7 => Some("VII"),
        8 => Some("VIII"),
        9 => Some("IX"),
        10 => Some("X"),
        _ => None,
    }
}

/// Add one lore counter to a Saga and check for chapter triggers.
pub fn add_lore_counter_and_check_chapters(
    game: &mut GameState,
    saga_id: ObjectId,
    trigger_queue: &mut TriggerQueue,
) -> Result<(), crate::effects::ExecutionError> {
    add_lore_counters_and_check_chapters(game, saga_id, 1, trigger_queue)
}

/// Add lore counters to a Saga and check for chapter triggers.
///
/// This uses the normal trigger system: adding lore counters emits a
/// CounterPlaced event, and chapter abilities match threshold crossings.
pub fn add_lore_counters_and_check_chapters(
    game: &mut GameState,
    saga_id: ObjectId,
    amount: u32,
    trigger_queue: &mut TriggerQueue,
) -> Result<(), crate::effects::ExecutionError> {
    // CR 714.3b: the precombat-main lore counter is a turn-based action, not
    // an effect ("if an effect would put counters" replacements don't apply).
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    put_lore_counters_and_check_chapters_with_cause(
        game,
        saga_id,
        amount,
        crate::events::cause::EventCause::from_game_rule(),
        trigger_queue,
        &mut dm,
    )
}

/// Apply a game-rule placement using the complete replacement outcome.
fn put_lore_counters_and_check_chapters_with_cause(
    game: &mut GameState,
    saga_id: ObjectId,
    amount: u32,
    cause: crate::events::cause::EventCause,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), crate::effects::ExecutionError> {
    let checkpoint = game.clone();
    let trigger_checkpoint = trigger_queue.clone();
    let result = (|| {
        let controller = game
            .object(saga_id)
            .map(|object| game.controller_of(object))
            .ok_or(crate::effects::ExecutionError::InvalidTarget)?;
        let mut ctx = crate::effects::ExecutionContext::new(saga_id, controller, decision_maker);
        ctx.cause = cause;
        let event = crate::events::Event::put_counters(
            saga_id,
            CounterType::Lore,
            amount,
            ctx.cause.clone(),
        )
        .with_provenance(ctx.provenance);
        let outcome =
            crate::effects::counters::execute_object_counter_placement(game, &mut ctx, event)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        try_queue_triggers_from_reported_events(game, trigger_queue, outcome.events, false)?;
        Ok(())
    })();
    if result.is_err() || decision_maker.awaiting_choice() {
        *game = checkpoint;
        *trigger_queue = trigger_checkpoint;
    }
    result
}

#[cfg(test)]
mod replacement_application_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::{Effect, Value};
    use crate::ids::CardId;
    use crate::replacement::{RedirectTarget, RedirectWhich, ReplacementAction};

    struct Answers {
        pause: bool,
        pending: bool,
        calls: usize,
    }
    impl DecisionMaker for Answers {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            assert!(!self.pending, "a suspended lore action must stop asking");
            self.calls += 1;
            self.pending = self.pause;
            !self.pause
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    fn saga(game: &mut GameState, player: PlayerId, name: &str) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Saga])
            .build();
        let object = game.create_object_from_card(&card, player, Zone::Battlefield);
        game.object_mut(object)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability::triggered(
                crate::triggers::Trigger::saga_chapter(vec![1]),
                vec![Effect::gain_life(1)],
            ));
        object
    }

    fn check_lore_application(mode: usize) {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        // The first Saga completes before the second Saga's payload suspends/errors.
        let first = saga(&mut game, alice, "Earlier Saga");
        let second = saga(&mut game, alice, "Replaced Saga");
        let recipient = saga(&mut game, bob, "Actual recipient");
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Replacement source").build(),
            alice,
            Zone::Battlefield,
        );
        let mut replacement = crate::static_abilities::StaticAbility::double_counters_replacement(
            crate::target::ObjectFilter::specific(second),
            Some(CounterType::Lore),
            "Replace lore placement".into(),
        )
        .generate_replacement_effect(source, alice)
        .unwrap();
        replacement.replacement = match mode {
            0 => ReplacementAction::Redirect {
                target: RedirectTarget::ToObject(recipient),
                which: RedirectWhich::First,
            },
            1 => ReplacementAction::Instead(vec![Effect::gain_life(2)]),
            2 => {
                ReplacementAction::Instead(vec![Effect::gain_life(2), Effect::lose_life(Value::X)])
            }
            _ => ReplacementAction::Instead(vec![
                Effect::gain_life(2),
                Effect::may(vec![Effect::gain_life(1)]),
                Effect::may(vec![Effect::gain_life(3)]),
            ]),
        };
        let one_shot = game
            .effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        game.take_pending_trigger_events();
        let mut queue = TriggerQueue::new();
        let mut dm = Answers {
            pause: mode == 3,
            pending: false,
            calls: 0,
        };
        let result = add_saga_lore_counters_with_dm(&mut game, &mut queue, &mut dm);
        assert_eq!(result.is_err(), mode == 2);
        assert_eq!(game.counter_count(second, CounterType::Lore), 0);
        if mode >= 2 {
            assert_eq!(game.counter_count(first, CounterType::Lore), 0);
            assert_eq!(game.counter_count(recipient, CounterType::Lore), 0);
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert!(queue.entries.is_empty());
            assert!(game.take_pending_trigger_events().is_empty());
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(one_shot)
                    .is_some()
            );
            if mode == 2 {
                return;
            }
            assert!(dm.pending);
            assert_eq!(dm.calls, 1);
            let mut replay = Answers {
                pause: false,
                pending: false,
                calls: 0,
            };
            add_saga_lore_counters_with_dm(&mut game, &mut queue, &mut replay).unwrap();
            assert_eq!(replay.calls, 2);
        }
        assert_eq!(game.counter_count(first, CounterType::Lore), 1);
        assert_eq!(game.counter_count(second, CounterType::Lore), 0);
        assert_eq!(
            game.counter_count(recipient, CounterType::Lore),
            u32::from(mode == 0)
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            match mode {
                0 => 20,
                1 => 22,
                _ => 26,
            }
        );
        assert_eq!(queue.entries.len(), if mode == 0 { 2 } else { 1 });
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_none()
        );
        assert!(game.take_pending_trigger_events().is_empty());
    }

    fn check_entry_application(mode: usize) {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let entrant = saga(&mut game, alice, "Legacy entrant");
        let recipient = saga(&mut game, bob, "Redirected recipient");
        let mut replacement = crate::static_abilities::StaticAbility::double_counters_replacement(
            crate::target::ObjectFilter::specific(entrant),
            Some(CounterType::Lore),
            "Replace entry lore".into(),
        )
        .generate_replacement_effect(entrant, alice)
        .unwrap();
        replacement.replacement = match mode {
            0 => ReplacementAction::Redirect {
                target: RedirectTarget::ToObject(recipient),
                which: RedirectWhich::First,
            },
            1 => ReplacementAction::Instead(vec![Effect::gain_life(2)]),
            2 => {
                ReplacementAction::Instead(vec![Effect::gain_life(2), Effect::lose_life(Value::X)])
            }
            _ => ReplacementAction::Instead(vec![
                Effect::gain_life(2),
                Effect::may(vec![Effect::gain_life(1)]),
                Effect::may(vec![Effect::gain_life(3)]),
            ]),
        };
        let one_shot = game
            .effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        game.take_pending_trigger_events();
        let mut queue = TriggerQueue::new();
        let mut dm = Answers {
            pause: mode == 3,
            pending: false,
            calls: 0,
        };
        let result = handle_saga_enters_battlefield(&mut game, entrant, &mut queue, &mut dm);
        assert_eq!(result.is_err(), mode == 2);
        assert_eq!(game.counter_count(entrant, CounterType::Lore), 0);
        if mode >= 2 {
            if mode == 2 {
                assert!(matches!(
                    result,
                    Err(crate::effects::ExecutionError::UnresolvableValue(_))
                ));
            }
            assert!(!game.has_processed_saga_entry_lore(entrant));
            assert_eq!(game.counter_count(recipient, CounterType::Lore), 0);
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert!(queue.entries.is_empty());
            assert!(game.take_pending_trigger_events().is_empty());
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(one_shot)
                    .is_some()
            );
            if mode == 2 {
                return;
            }
            assert!(dm.pending);
            assert_eq!(dm.calls, 1);
            let mut replay = Answers {
                pause: false,
                pending: false,
                calls: 0,
            };
            handle_saga_enters_battlefield(&mut game, entrant, &mut queue, &mut replay).unwrap();
            assert_eq!(replay.calls, 2);
        }
        assert!(game.has_processed_saga_entry_lore(entrant));
        assert!(!game.has_processed_saga_entry_lore(recipient));
        assert_eq!(game.counter_count(entrant, CounterType::Lore), 0);
        assert_eq!(
            game.counter_count(recipient, CounterType::Lore),
            u32::from(mode == 0)
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            match mode {
                0 => 20,
                1 => 22,
                _ => 26,
            }
        );
        assert_eq!(queue.entries.len(), usize::from(mode == 0));
        if mode == 0 {
            assert_eq!(queue.entries[0].source, recipient);
        }
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_none()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        // Even a redirected or Instead entry completes exactly once.
        let chapters = queue.entries.len();
        handle_saga_enters_battlefield(&mut game, entrant, &mut queue, &mut dm).unwrap();
        assert_eq!(game.counter_count(entrant, CounterType::Lore), 0);
        assert_eq!(
            game.player(alice).unwrap().life,
            match mode {
                0 => 20,
                1 => 22,
                _ => 26,
            }
        );
        assert_eq!(queue.entries.len(), chapters);
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn legacy_entry_lore_commits_redirected_recipient_and_actual_chapter() {
        check_entry_application(0);
    }
    #[test]
    fn legacy_entry_lore_executes_instead_and_completes_once() {
        check_entry_application(1);
    }
    #[test]
    fn legacy_entry_lore_payload_error_restores_completion_and_one_shot() {
        check_entry_application(2);
    }
    #[test]
    fn legacy_entry_lore_pause_restores_completion_and_replays_once() {
        check_entry_application(3);
    }

    #[test]
    fn precombat_lore_commits_redirected_recipient_and_actual_chapter() {
        check_lore_application(0);
    }
    #[test]
    fn precombat_lore_executes_instead_without_original_chapter() {
        check_lore_application(1);
    }
    #[test]
    fn precombat_lore_payload_error_restores_all_sagas_and_chapters() {
        check_lore_application(2);
    }
    #[test]
    fn precombat_lore_pause_restores_all_sagas_and_replays_once() {
        check_lore_application(3);
    }
}

#[cfg(test)]
mod entry_completion_tests {
    use super::*;

    fn definition() -> crate::cards::CardDefinition {
        crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Entry Saga")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Saga])
            .with_ability(crate::ability::Ability::triggered(
                crate::triggers::Trigger::saga_chapter(vec![1, 2, 3]),
                vec![crate::effect::Effect::gain_life(1)],
            ))
            .build()
    }

    #[test]
    fn prevented_entry_lore_completes_once_and_resets_on_reentry() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_card(
            &crate::card::CardBuilder::new(crate::ids::CardId::new(), "Counter replacement").build(),
            alice,
            Zone::Battlefield,
        );
        let prevent = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::counters::matchers::WouldPutCountersMatcher::new(
                    crate::target::ObjectFilter::default().in_zone(Zone::Battlefield),
                    Some(CounterType::Lore),
                ),
                crate::replacement::ReplacementAction::Prevent,
            ),
        );
        let card = game.create_object_from_definition(&definition(), alice, Zone::Hand);
        let entered = game
            .move_object_with_etb_processing(card, Zone::Battlefield).expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions().unwrap()
            .new_id;
        assert_eq!(game.counter_count(entered, CounterType::Lore), 0);
        assert!(game.has_processed_saga_entry_lore(entered));
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(prevent)
                .is_none()
        );
        game.take_pending_trigger_events();
        let mut queue = TriggerQueue::new();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        handle_saga_enters_battlefield(&mut game, entered, &mut queue, &mut dm).unwrap();
        assert_eq!(game.counter_count(entered, CounterType::Lore), 0);
        assert!(queue.is_empty());
        assert!(game.take_pending_trigger_events().is_empty());
        let checkpoint = game.clone();
        let graveyard = game
            .move_object(
                entered,
                Zone::Graveyard,
                crate::events::cause::EventCause::effect(),
            )
            .unwrap();
        assert!(!game.has_processed_saga_entry_lore(entered));
        assert!(!game.has_processed_saga_entry_lore(graveyard));
        assert!(checkpoint.has_processed_saga_entry_lore(entered));
        assert_eq!(checkpoint.counter_count(entered, CounterType::Lore), 0);
        let returned = game
            .move_object_with_etb_processing(graveyard, Zone::Battlefield).expect("replacement operation must execute successfully in this scenario")
            .assert_completed_without_additions().unwrap()
            .new_id;
        assert!(game.has_processed_saga_entry_lore(returned));
        assert_eq!(game.counter_count(returned, CounterType::Lore), 1);
        handle_saga_enters_battlefield(&mut game, returned, &mut queue, &mut dm).unwrap();
        assert_eq!(game.counter_count(returned, CounterType::Lore), 1);
        let notifications = game.take_pending_trigger_events();
        let placements = notifications
            .iter()
            .filter_map(|event| {
                if let Some(counter) = event.downcast::<crate::events::CounterPlacedEvent>() {
                    Some((counter.permanent, counter.counter_type, counter.amount))
                } else if let Some(marker) = event.downcast::<crate::events::MarkersChangedEvent>() {
                    if let (
                        crate::marker::MarkerLocation::Object(object),
                        crate::marker::Marker::Counter(kind),
                    ) = (&marker.location, &marker.marker)
                    {
                        marker.is_added().then_some((*object, *kind, marker.amount))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(placements, vec![(returned, CounterType::Lore, 1)]);
        game.remove_object(returned);
        assert!(!game.has_processed_saga_entry_lore(returned));
    }

    #[test]
    fn legacy_entry_completion_is_independent_of_existing_or_removed_counters() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let saga = game.create_object_from_definition(&definition(), alice, Zone::Battlefield);
        let _ = game.add_counters(saga, CounterType::Lore, 2);
        assert!(!game.has_processed_saga_entry_lore(saga));
        let mut queue = TriggerQueue::new();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        handle_saga_enters_battlefield(&mut game, saga, &mut queue, &mut dm).unwrap();
        assert_eq!(game.counter_count(saga, CounterType::Lore), 3);
        assert!(game.has_processed_saga_entry_lore(saga));
        let initial_triggers = queue.entries.len();
        assert_eq!(initial_triggers, 1);
        let _ = game.remove_counters(saga, CounterType::Lore, 3, None, None);
        handle_saga_enters_battlefield(&mut game, saga, &mut queue, &mut dm).unwrap();
        assert_eq!(game.counter_count(saga, CounterType::Lore), 0);
        assert_eq!(queue.entries.len(), initial_triggers);
    }
}

#[cfg(test)]
mod entry_eligibility_tests {
    use super::*;

    #[test]
    fn chapterless_saga_gets_entry_lore_but_no_precombat_lore() {
        for central_entry in [false, true] {
            let alice = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            let definition = crate::cards::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                "Chapterless Saga",
            )
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Saga])
            .build();
            let created = game.create_object_from_definition(
                &definition,
                alice,
                if central_entry {
                    Zone::Hand
                } else {
                    Zone::Battlefield
                },
            );
            let mut queue = TriggerQueue::new();
            let mut dm = crate::decision::SelectFirstDecisionMaker;
            let saga = if central_entry {
                game.move_object_with_etb_processing(created, Zone::Battlefield).expect("replacement operation must execute successfully in this scenario")
                    .assert_completed_without_additions().unwrap()
                    .new_id
            } else {
                handle_saga_enters_battlefield(&mut game, created, &mut queue, &mut dm).unwrap();
                created
            };
            assert_eq!(game.counter_count(saga, CounterType::Lore), 1);
            assert!(game.has_processed_saga_entry_lore(saga));
            assert!(queue.is_empty());
            game.take_pending_trigger_events();
            add_saga_lore_counters(&mut game, &mut queue).unwrap();
            assert_eq!(game.counter_count(saga, CounterType::Lore), 1);
            handle_saga_enters_battlefield(&mut game, saga, &mut queue, &mut dm).unwrap();
            assert_eq!(game.counter_count(saga, CounterType::Lore), 1);
            assert!(queue.is_empty());
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }

    #[test]
    fn saga_entry_and_precombat_use_calculated_subtypes() {
        for printed_saga in [false, true] {
            let alice = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            let source = game.create_object_from_card(
                &crate::card::CardBuilder::new(crate::ids::CardId::new(), "Subtype source")
                    .card_types(vec![CardType::Enchantment])
                    .build(),
                alice,
                Zone::Battlefield,
            );
            let definition = crate::cards::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                "Calculated Saga",
            )
            .card_types(vec![CardType::Enchantment, CardType::Creature])
            .subtypes(if printed_saga {
                vec![Subtype::Saga]
            } else {
                Vec::new()
            })
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .with_ability(crate::ability::Ability::triggered(
                crate::triggers::Trigger::saga_chapter(vec![1, 2, 3]),
                vec![crate::effect::Effect::gain_life(1)],
            ))
            .build();
            let card = game.create_object_from_definition(&definition, alice, Zone::Hand);
            game.effect_store.continuous_effects.add_effect(
                crate::continuous::ContinuousEffect::new(
                    source,
                    alice,
                    crate::continuous::EffectTarget::AllCreatures,
                    if printed_saga {
                        crate::continuous::Modification::RemoveSubtypes(vec![Subtype::Saga])
                    } else {
                        crate::continuous::Modification::AddSubtypes(vec![Subtype::Saga])
                    },
                ),
            );
            game.refresh_continuous_state();
            let entrant = game
                .move_object_with_etb_processing(card, Zone::Battlefield).expect("replacement operation must execute successfully in this scenario")
                .assert_completed_without_additions().unwrap()
                .new_id;
            let expected_entry = u32::from(!printed_saga);
            assert_eq!(
                game.object(entrant)
                    .unwrap()
                    .subtypes
                    .contains(&Subtype::Saga),
                printed_saga
            );
            assert_eq!(
                game.counter_count(entrant, CounterType::Lore),
                expected_entry
            );
            assert_eq!(game.has_processed_saga_entry_lore(entrant), !printed_saga);
            assert_eq!(
                crate::derived_view::DerivedGameView::new(&game)
                    .calculated_subtypes(entrant)
                    .contains(&Subtype::Saga),
                !printed_saga
            );
            let mut queue = TriggerQueue::new();
            add_saga_lore_counters(&mut game, &mut queue).unwrap();
            assert_eq!(
                game.counter_count(entrant, CounterType::Lore),
                expected_entry * 2
            );
            let mut dm = crate::decision::SelectFirstDecisionMaker;
            handle_saga_enters_battlefield(&mut game, entrant, &mut queue, &mut dm).unwrap();
            assert_eq!(
                game.counter_count(entrant, CounterType::Lore),
                expected_entry * 2
            );
        }
    }
}

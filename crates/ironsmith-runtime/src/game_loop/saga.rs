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

pub(crate) fn saga_profile_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    object_id: ObjectId,
) -> Option<SagaProfile> {
    if !view.calculated_subtypes(object_id).contains(&Subtype::Saga) {
        return None;
    }
    let final_chapter = final_chapter_number_with_view(view, object_id)?;
    let controller = view
        .calculated_characteristics(object_id)
        .map(|chars| chars.controller)
        .or_else(|| game.object(object_id).map(|obj| game.controller_of(obj)))?;
    let has_read_ahead = view.object_has_static_ability_id(
        object_id,
        crate::static_abilities::StaticAbilityId::ReadAhead,
    );
    Some(SagaProfile {
        controller,
        final_chapter,
        has_read_ahead,
    })
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
/// Per CR 714.3c, this applies only to Sagas the active player controls that
/// currently have one or more chapter abilities.
pub fn add_saga_lore_counters(game: &mut GameState, trigger_queue: &mut TriggerQueue) {
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
        add_lore_counter_and_check_chapters(game, saga_id, trigger_queue);
    }
}

pub fn handle_saga_enters_battlefield(
    game: &mut GameState,
    saga_id: ObjectId,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) {
    let Some(profile) = ({
        let view = crate::derived_view::DerivedGameView::new(game);
        saga_profile_with_view(game, &view, saga_id)
    }) else {
        return;
    };

    let amount = if profile.has_read_ahead {
        choose_read_ahead_chapter(
            game,
            saga_id,
            profile.controller,
            profile.final_chapter,
            decision_maker,
        )
    } else {
        1
    };

    add_lore_counters_and_check_chapters(game, saga_id, amount, trigger_queue);
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
) {
    add_lore_counters_and_check_chapters(game, saga_id, 1, trigger_queue);
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
) {
    if amount == 0 {
        return;
    }
    let Some(event) = game.add_counters(saga_id, CounterType::Lore, amount) else {
        return;
    };
    queue_triggers_from_event(game, trigger_queue, event, false);
}

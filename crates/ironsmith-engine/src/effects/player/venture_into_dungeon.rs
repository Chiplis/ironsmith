//! Venture into a dungeon by starting or advancing dungeon progress.

use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::dungeon::{
    ActiveDungeonProgress, first_room_name, next_room_names, normal_venture_dungeon_names,
    undercity_name,
};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;

#[derive(Debug, Clone, PartialEq)]
pub struct VentureIntoDungeonEffect {
    pub player: PlayerFilter,
    pub undercity_if_no_active: bool,
}

impl VentureIntoDungeonEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            undercity_if_no_active: false,
        }
    }

    pub fn via_initiative(player: PlayerFilter) -> Self {
        Self {
            player,
            undercity_if_no_active: true,
        }
    }
}

fn choose_named_option(
    ctx: &mut ExecutionContext,
    game: &GameState,
    chooser: PlayerId,
    prompt: &str,
    options: &[String],
) -> Result<Option<String>, ExecutionError> {
    let selectable = options
        .iter()
        .enumerate()
        .map(|(idx, option)| SelectableOption::new(idx, option.clone()))
        .collect::<Vec<_>>();
    let choice_ctx = SelectOptionsContext::new(chooser, Some(ctx.source), prompt, selectable, 1, 1);
    let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    Ok(selected
        .into_iter()
        .next()
        .filter(|idx| *idx < options.len())
        .map(|idx| options[idx].clone()))
}

/// Synthetic source id for the room abilities of the dungeon `player` owns.
/// Room abilities are controlled by the dungeon's owner (CR 309.4c); the
/// dungeon card itself is not a game object here.
pub(crate) fn dungeon_room_source_id(player: PlayerId) -> ObjectId {
    ObjectId::from_raw(u64::MAX - 0x1_0000 - u64::from(player.index() as u32))
}

/// Remove a completed dungeon from the game (CR 309.7) and emit the
/// completion event.
fn complete_player_dungeon(
    game: &mut GameState,
    player_id: PlayerId,
    source: ObjectId,
    provenance: crate::provenance::ProvNodeId,
) -> Option<TriggerEvent> {
    let progress = game.active_dungeon(player_id).cloned()?;
    game.clear_active_dungeon(player_id);
    game.record_completed_dungeon(player_id, progress.dungeon_name);
    Some(TriggerEvent::new_with_provenance(
        KeywordActionEvent::new(KeywordActionKind::CompleteDungeon, player_id, source, 1),
        provenance,
    ))
}

/// CR 704.5t / 309.6: a dungeon whose venture marker is on its bottommost
/// room is removed from the game once no room ability from it is waiting to
/// be put on the stack or still on the stack.
pub(crate) fn complete_finished_dungeons(
    game: &mut GameState,
    trigger_queue: &crate::triggers::TriggerQueue,
) -> bool {
    let finished = game
        .players
        .iter()
        .map(|player| player.id)
        .filter(|&player| {
            game.active_dungeon(player).is_some_and(|progress| {
                next_room_names(&progress.dungeon_name, &progress.room_name)
                    .is_some_and(|next| next.is_empty())
            })
        })
        .collect::<Vec<_>>();
    let mut completed = false;
    for player in finished {
        let source = dungeon_room_source_id(player);
        let room_ability_pending = trigger_queue
            .entries
            .iter()
            .chain(game.effect_store.pending_trigger_entries.iter())
            .any(|entry| {
                entry.source == source
                    && entry.source_kind == crate::triggers::TriggeredAbilitySourceKind::DungeonRoom
            })
            || game.stack.iter().any(|entry| entry.object_id == source);
        if room_ability_pending {
            continue;
        }
        let provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::KeywordAction);
        if let Some(event) = complete_player_dungeon(game, player, source, provenance) {
            game.queue_trigger_event(provenance, event);
            completed = true;
        }
    }
    completed
}

/// Queue the room ability of the room the venture marker just moved into
/// (CR 309.4c).
fn queue_room_ability(
    game: &mut GameState,
    player_id: PlayerId,
    dungeon_name: &str,
    room_name: &str,
    triggering_event: TriggerEvent,
) {
    let Some(room) = crate::dungeon::room_ability(dungeon_name, room_name) else {
        return;
    };
    let ability = crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::keyword_action(
            KeywordActionKind::VentureIntoDungeon,
            PlayerFilter::You,
        ),
        effects: crate::resolution::ResolutionProgram::from_effects(room.effects),
        choices: room.choices,
        intervening_if: None,
        presentation_label: None,
    };
    let trigger_identity = crate::triggers::compute_trigger_identity(&ability);
    let source = dungeon_room_source_id(player_id);
    game.defer_trigger_entries([crate::triggers::TriggeredAbilityEntry {
        source,
        controller: player_id,
        x_value: None,
        event_value_amount: None,
        ability,
        triggering_event,
        source_stable_id: crate::ids::StableId::from(source),
        source_name: room_name.to_string(),
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: crate::triggers::TriggeredAbilitySourceKind::DungeonRoom,
        trigger_identity,
    }]);
}

fn choose_dungeon_to_start(
    game: &GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    undercity_if_no_active: bool,
) -> Result<Option<(String, String)>, ExecutionError> {
    let dungeon_options = if undercity_if_no_active {
        vec![undercity_name().to_string()]
    } else {
        normal_venture_dungeon_names()
    };
    let dungeon_name = if dungeon_options.len() == 1 {
        dungeon_options[0].clone()
    } else {
        let Some(dungeon_name) =
            choose_named_option(ctx, game, player_id, "Choose a dungeon", &dungeon_options)?
        else {
            return Ok(None);
        };
        dungeon_name
    };
    let room_name = first_room_name(&dungeon_name)
        .ok_or_else(|| ExecutionError::Impossible(format!("unknown dungeon {dungeon_name}")))?
        .to_string();
    Ok(Some((dungeon_name, room_name)))
}

pub(crate) fn advance_player_dungeon(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    undercity_if_no_active: bool,
) -> Result<EffectOutcome, ExecutionError> {
    let mut outcome = EffectOutcome::resolved();
    let (dungeon_name, room_name) = if let Some(progress) = game.active_dungeon(player_id).cloned()
    {
        let next_rooms =
            next_room_names(&progress.dungeon_name, &progress.room_name).ok_or_else(|| {
                ExecutionError::Impossible(format!(
                    "missing next room data for {} -> {}",
                    progress.dungeon_name, progress.room_name
                ))
            })?;
        if next_rooms.is_empty() {
            // CR 701.49c: venturing from the bottommost room (its room ability
            // is still on the stack) completes the dungeon and starts another.
            if let Some(event) =
                complete_player_dungeon(game, player_id, ctx.source, ctx.provenance)
            {
                outcome = outcome.with_event(event);
            }
            let Some(start) =
                choose_dungeon_to_start(game, ctx, player_id, undercity_if_no_active)?
            else {
                return Ok(EffectOutcome::count(0));
            };
            start
        } else {
            let next_room = if next_rooms.len() == 1 {
                next_rooms[0].clone()
            } else {
                let Some(next_room) = choose_named_option(
                    ctx,
                    game,
                    player_id,
                    "Choose the next dungeon room",
                    &next_rooms,
                )?
                else {
                    return Ok(EffectOutcome::count(0));
                };
                next_room
            };
            (progress.dungeon_name, next_room)
        }
    } else {
        let Some(start) = choose_dungeon_to_start(game, ctx, player_id, undercity_if_no_active)?
        else {
            return Ok(EffectOutcome::count(0));
        };
        start
    };

    game.set_active_dungeon(
        player_id,
        ActiveDungeonProgress::new(dungeon_name.clone(), room_name.clone()),
    );
    let venture_event = TriggerEvent::new_with_provenance(
        KeywordActionEvent::new(KeywordActionKind::VentureIntoDungeon, player_id, ctx.source, 1),
        ctx.provenance,
    );
    // CR 309.4c: moving the venture marker into a room triggers its room
    // ability. Completing the dungeon waits for that ability (CR 704.5t).
    queue_room_ability(game, player_id, &dungeon_name, &room_name, venture_event.clone());

    Ok(outcome.with_event(venture_event))
}

/// Mad Wizard's Lair: "Draw three cards and reveal them. You may cast one of
/// them without paying its mana cost."
#[derive(Debug, Clone, PartialEq)]
pub struct MadWizardsLairEffect;

impl EffectExecutor for MadWizardsLairEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = ctx.controller;
        let hand_before = game
            .player(player)
            .map(|player| player.hand.to_vec())
            .unwrap_or_default();
        let draw_outcome =
            crate::effects::execute_effect(game, &crate::effect::Effect::draw(3), ctx)?;
        let drawn = game
            .player(player)
            .map(|player| player.hand.to_vec())
            .unwrap_or_default()
            .into_iter()
            .filter(|id| !hand_before.contains(id))
            .collect::<Vec<_>>();
        if drawn.is_empty() {
            return Ok(draw_outcome);
        }
        for viewer_idx in 0..game.players.len() {
            let viewer = PlayerId::from_index(viewer_idx as u8);
            let view_ctx = crate::decisions::context::ViewCardsContext::new(
                viewer,
                player,
                Some(ctx.source),
                crate::zone::Zone::Hand,
                "Cards drawn and revealed",
            )
            .with_public(true);
            ctx.decision_maker.view_cards(game, viewer, &drawn, &view_ctx);
        }
        let castable = drawn
            .iter()
            .copied()
            .filter(|id| game.object(*id).is_some_and(|object| !object.is_land()))
            .collect::<Vec<_>>();
        if castable.is_empty() {
            return Ok(draw_outcome);
        }
        let spec = crate::decisions::specs::ChooseObjectsSpec::new(
            ctx.source,
            "You may cast one of them without paying its mana cost",
            castable.clone(),
            0,
            Some(1),
        );
        let chosen: Vec<ObjectId> = crate::decisions::make_decision(
            game,
            ctx.decision_maker,
            player,
            Some(ctx.source),
            spec,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(card) = chosen.into_iter().find(|id| castable.contains(id)) else {
            return Ok(draw_outcome);
        };
        let Some(snapshot) = game
            .object(card)
            .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
        else {
            return Ok(draw_outcome);
        };
        let tag = "__mad_wizards_lair_cast";
        ctx.tag_object(tag, snapshot);
        let mut cast = crate::effects::CastTaggedEffect::new(tag, PlayerFilter::You);
        cast.without_paying_mana_cost = true;
        let cast_outcome = crate::effects::execute_effect(
            game,
            &crate::effect::Effect::new(cast),
            ctx,
        )?;
        Ok(EffectOutcome::aggregate(vec![draw_outcome, cast_outcome]))
    }
}

/// Throne of the Dead Three: "Reveal the top ten cards of your library. Put
/// a creature card from among them onto the battlefield with three +1/+1
/// counters on it. It gains hexproof until your next turn. Then shuffle."
#[derive(Debug, Clone, PartialEq)]
pub struct ThroneOfTheDeadThreeEffect;

impl EffectExecutor for ThroneOfTheDeadThreeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = ctx.controller;
        let revealed = game
            .player(player)
            .map(|player| player.library.iter().rev().take(10).copied().collect::<Vec<_>>())
            .unwrap_or_default();
        for viewer_idx in 0..game.players.len() {
            let viewer = PlayerId::from_index(viewer_idx as u8);
            let view_ctx = crate::decisions::context::ViewCardsContext::new(
                viewer,
                player,
                Some(ctx.source),
                crate::zone::Zone::Library,
                "Revealed from the top of the library",
            )
            .with_public(true);
            ctx.decision_maker
                .view_cards(game, viewer, &revealed, &view_ctx);
        }
        let creatures = revealed
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id)
                    .is_some_and(|object| object.has_card_type(crate::types::CardType::Creature))
            })
            .collect::<Vec<_>>();
        let mut outcomes = Vec::new();
        if !creatures.is_empty() {
            let spec = crate::decisions::specs::ChooseObjectsSpec::new(
                ctx.source,
                "Choose a creature card to put onto the battlefield",
                creatures.clone(),
                1,
                Some(1),
            );
            let chosen: Vec<ObjectId> = crate::decisions::make_decision(
                game,
                ctx.decision_maker,
                player,
                Some(ctx.source),
                spec,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let card = chosen
                .into_iter()
                .find(|id| creatures.contains(id))
                .unwrap_or(creatures[0]);
            if let Some(entered) = game
                .move_object_with_etb_processing_with_initial_counters_with_dm(
                    card,
                    crate::zone::Zone::Battlefield,
                    vec![(crate::object::CounterType::PlusOnePlusOne, 3)],
                    &mut ctx.decision_maker,
                )
                && game
                    .object(entered.new_id)
                    .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
            {
                let hexproof = crate::effects::ApplyContinuousEffect::with_spec(
                    crate::target::ChooseSpec::SpecificObject(entered.new_id),
                    crate::continuous::Modification::AddAbilityGeneric(
                        crate::ability::Ability::static_ability(
                            crate::static_abilities::StaticAbility::hexproof(),
                        ),
                    ),
                    crate::effect::Until::YourNextTurn,
                );
                outcomes.push(hexproof.execute(game, ctx)?);
                outcomes.push(EffectOutcome::with_objects(vec![entered.new_id]));
            }
        }
        outcomes.push(crate::effects::execute_effect(
            game,
            &crate::effect::Effect::shuffle_library(),
            ctx,
        )?);
        Ok(EffectOutcome::aggregate(outcomes))
    }
}

impl EffectExecutor for VentureIntoDungeonEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        advance_player_dungeon(game, ctx, player_id, self.undercity_if_no_active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectOptionsContext;
    use crate::ids::{ObjectId, PlayerId};

    struct ChooseFirstOptionDecisionMaker;

    impl DecisionMaker for ChooseFirstOptionDecisionMaker {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            ctx.options
                .first()
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }
    }

    #[test]
    fn venture_starts_lost_mine_by_default() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(700);
        let mut dm = ChooseFirstOptionDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        VentureIntoDungeonEffect::new(PlayerFilter::Specific(alice))
            .execute(&mut game, &mut ctx)
            .expect("venture should resolve");

        let progress = game
            .active_dungeon(alice)
            .expect("venture should start a dungeon");
        assert_eq!(progress.dungeon_name, "Lost Mine of Phandelver");
        assert_eq!(progress.room_name, "Cave Entrance");
        assert!(!game.has_completed_dungeon(alice));
    }

    #[test]
    fn venture_can_complete_a_dungeon_and_emit_completion_event() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(701);
        let mut dm = ChooseFirstOptionDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = VentureIntoDungeonEffect::new(PlayerFilter::Specific(alice));

        for _ in 0..3 {
            effect
                .execute(&mut game, &mut ctx)
                .expect("venture progress should resolve");
        }
        let final_outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("final venture should resolve");

        assert!(game.active_dungeon(alice).is_none());
        assert!(game.has_completed_named_dungeon(alice, "Lost Mine of Phandelver"));
        let completion = final_outcome.events[0]
            .downcast::<KeywordActionEvent>()
            .expect("expected dungeon completion event");
        assert_eq!(completion.action, KeywordActionKind::CompleteDungeon);
        assert_eq!(completion.player, alice);
    }
}

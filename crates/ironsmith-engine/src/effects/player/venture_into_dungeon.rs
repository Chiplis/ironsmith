//! Venture into a dungeon by starting or advancing dungeon progress.

use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::dungeon::{
    ActiveDungeonProgress, first_room_name, next_room_names, venture_dungeon_names,
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

/// One owner for dungeon removal and completion history, whether reached by
/// a venture instruction or the rule-driven finished-dungeon check.
#[derive(Debug, Clone)]
struct CompletePlayerDungeon(PlayerId);

impl EffectExecutor for CompletePlayerDungeon {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(progress) = game.active_dungeon(self.0).cloned() else {
            return Ok(EffectOutcome::count(0));
        };
        game.clear_active_dungeon(self.0);
        game.record_completed_dungeon(self.0, progress.dungeon_name);
        crate::effects::composition::complete_keyword_action(
            game,
            ctx,
            KeywordActionEvent::new(KeywordActionKind::CompleteDungeon, self.0, ctx.source, 1),
        )
    }
}

fn complete_player_dungeon(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
) -> Result<EffectOutcome, ExecutionError> {
    crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
        CompletePlayerDungeon(player).execute_child(game, ctx)
    })
}

/// CR 704.5t / 309.6: a dungeon whose venture marker is on its bottommost
/// room is removed from the game once no room ability from it is waiting to
/// be put on the stack or still on the stack.
pub(crate) fn complete_finished_dungeons(
    game: &mut GameState,
    trigger_queue: &crate::triggers::TriggerQueue,
) -> Result<bool, ExecutionError> {
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
        let mut ctx = ExecutionContext::new_default(source, player);
        ctx.provenance = provenance;
        ctx.cause = crate::events::EventCause::from_sba();
        let outcome = complete_player_dungeon(game, &mut ctx, player)?;
        for event in outcome.events {
            let provenance = event.provenance();
            game.queue_trigger_event(provenance, event);
            completed = true;
        }
    }
    Ok(completed)
}

/// Queue the room ability of the room the venture marker just moved into
/// (CR 309.4c). The ability is the dungeon's compiled room ability.
fn queue_room_ability(
    game: &mut GameState,
    player_id: PlayerId,
    dungeon_name: &str,
    room_name: &str,
    triggering_event: TriggerEvent,
) {
    let Some(ability) = crate::dungeon::room_ability(dungeon_name, room_name) else {
        return;
    };
    let trigger_identity = crate::triggers::compute_trigger_identity(&ability);
    let source = dungeon_room_source_id(player_id);
    let entry = crate::triggers::TriggeredAbilityEntry {
        linked_exile_owner: None,
        source_number_owner: None,
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
    };
    // CR 603.2d: "Room abilities of dungeons you own trigger an additional
    // time" (Hama Pashar, Dungeon Delver) applies to this deferred entry too.
    let entries = crate::triggers::check::with_additional_trigger_copies(game, vec![entry]);
    game.defer_trigger_entries(entries);
}

/// The quality "venture into Undercity" restricts the choice to (CR 701.49d).
const UNDERCITY_QUALITY: &str = "Undercity";

fn choose_dungeon_to_start(
    game: &GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    undercity_if_no_active: bool,
) -> Result<Option<(String, String)>, ExecutionError> {
    let quality = undercity_if_no_active.then_some(UNDERCITY_QUALITY);
    let dungeon_options = venture_dungeon_names(quality);
    let dungeon_name = match dungeon_options.as_slice() {
        // No compiled dungeon card is available to this session (the host
        // registers them alongside card artifacts), so none can be chosen.
        [] => return Ok(None),
        [only] => only.clone(),
        _ => {
            let Some(dungeon_name) =
                choose_named_option(ctx, game, player_id, "Choose a dungeon", &dungeon_options)?
            else {
                return Ok(None);
            };
            dungeon_name
        }
    };
    let room_name = first_room_name(&dungeon_name)
        .ok_or_else(|| ExecutionError::Impossible(format!("unknown dungeon {dungeon_name}")))?;
    Ok(Some((dungeon_name, room_name)))
}

pub(crate) fn advance_player_dungeon(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    undercity_if_no_active: bool,
) -> Result<EffectOutcome, ExecutionError> {
    crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
        let mut outcome = EffectOutcome::resolved();
        let (dungeon_name, room_name) =
            if let Some(progress) = game.active_dungeon(player_id).cloned() {
                let next_rooms = next_room_names(&progress.dungeon_name, &progress.room_name)
                    .ok_or_else(|| {
                        ExecutionError::Impossible(format!(
                            "missing next room data for {} -> {}",
                            progress.dungeon_name, progress.room_name
                        ))
                    })?;
                if next_rooms.is_empty() {
                    // CR 701.49c: venturing from the bottommost room (its room ability
                    // is still on the stack) completes the dungeon and starts another.
                    let completion = complete_player_dungeon(game, ctx, player_id)?;
                    outcome = EffectOutcome::aggregate_with_primary_result(outcome, [completion]);
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
                let Some(start) =
                    choose_dungeon_to_start(game, ctx, player_id, undercity_if_no_active)?
                else {
                    return Ok(EffectOutcome::count(0));
                };
                start
            };

        game.set_active_dungeon(
            player_id,
            ActiveDungeonProgress::new(dungeon_name.clone(), room_name.clone()),
        );
        let completion = crate::effects::composition::complete_keyword_action(
            game,
            ctx,
            KeywordActionEvent::new(
                KeywordActionKind::VentureIntoDungeon,
                player_id,
                ctx.source,
                1,
            ),
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let venture_event = completion.events.first().cloned().ok_or_else(|| {
            ExecutionError::InternalError("venture completed without its action observation".into())
        })?;
        // CR 309.4c: moving the venture marker into a room triggers its room
        // ability. Completing the dungeon waits for that ability (CR 704.5t).
        queue_room_ability(
            game,
            player_id,
            &dungeon_name,
            &room_name,
            venture_event.clone(),
        );

        Ok(EffectOutcome::aggregate_with_primary_result(
            outcome,
            [completion],
        ))
    })
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

    /// A two-branch, three-level dungeon standing in for a compiled dungeon
    /// card (engine tests cannot link the compiler).
    fn register_test_dungeon() {
        use crate::triggers::Trigger;
        let mut definition = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Test Dungeon of Venturing",
        )
        .card_types(vec![crate::types::CardType::Dungeon])
        .build();
        for (room, leads_to) in [
            ("Entry", vec!["Left", "Right"]),
            ("Left", vec!["Vault"]),
            ("Right", vec!["Vault"]),
            ("Vault", vec![]),
        ] {
            definition
                .abilities
                .push(crate::ability::Ability::triggered(
                    Trigger::dungeon_room(room, leads_to.into_iter().map(String::from).collect()),
                    vec![crate::effect::Effect::gain_life(1)],
                ));
        }
        crate::dungeon::register_dungeon_definition(&definition)
            .expect("test dungeon should be valid");
    }

    #[test]
    fn venture_starts_the_only_dungeon() {
        register_test_dungeon();
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
        assert_eq!(progress.room_name, "Entry");
        assert!(!game.has_completed_dungeon(alice));
    }

    #[test]
    fn venturing_from_the_bottommost_room_completes_the_dungeon() {
        register_test_dungeon();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(701);
        let mut dm = ChooseFirstOptionDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = VentureIntoDungeonEffect::new(PlayerFilter::Specific(alice));

        game.set_active_dungeon(
            alice,
            ActiveDungeonProgress::new("Test Dungeon of Venturing", "Vault"),
        );
        let final_outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("final venture should resolve");

        // CR 701.49c: the completed dungeon leaves and a new one starts.
        assert!(game.has_completed_named_dungeon(alice, "Test Dungeon of Venturing"));
        let completion = final_outcome.events[0]
            .downcast::<KeywordActionEvent>()
            .expect("expected dungeon completion event");
        assert_eq!(completion.action, KeywordActionKind::CompleteDungeon);
        assert_eq!(completion.player, alice);
    }
}

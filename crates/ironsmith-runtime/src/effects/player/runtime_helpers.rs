use crate::effect::EffectOutcome;
use crate::events::other::LandPlayedEvent;
use crate::events::spells::SpellCastEvent;
use crate::executor::ExecutionContext;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::triggers::TriggerEvent;
use crate::types::Subtype;
use crate::zone::Zone;

pub(super) fn register_effect_driven_spell_cast(
    game: &mut GameState,
    new_id: ObjectId,
    caster: PlayerId,
    from_zone: Zone,
    provenance: crate::provenance::ProvNodeId,
) -> TriggerEvent {
    if from_zone == Zone::Command {
        game.record_commander_cast_from_command_zone(new_id);
    }
    let event = if let Some(obj) = game.object(new_id) {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(obj, game);
        SpellCastEvent::new_with_snapshot(new_id, caster, from_zone, snapshot)
    } else {
        SpellCastEvent::new(new_id, caster, from_zone)
    };
    TriggerEvent::new_with_provenance(event, provenance)
}

pub(super) fn queue_effect_driven_land_play(
    game: &mut GameState,
    ctx: &ExecutionContext,
    land_id: ObjectId,
    player: PlayerId,
    from_zone: Zone,
) {
    game.queue_trigger_event(
        ctx.provenance,
        TriggerEvent::new_with_provenance(
            LandPlayedEvent::new(land_id, player, from_zone),
            ctx.provenance,
        ),
    );

    if game
        .object(land_id)
        .is_some_and(|obj| obj.subtypes.contains(&Subtype::Saga))
        && let Some(event) = game.add_counters(land_id, CounterType::Lore, 1)
    {
        game.queue_trigger_event(ctx.provenance, event);
    }

    if let Some(player_data) = game.player_mut(player) {
        player_data.record_land_play();
    }
}

pub(super) fn with_spell_cast_event(
    outcome: EffectOutcome,
    game: &mut GameState,
    new_id: ObjectId,
    caster: PlayerId,
    from_zone: Zone,
    provenance: crate::provenance::ProvNodeId,
) -> EffectOutcome {
    let event = register_effect_driven_spell_cast(game, new_id, caster, from_zone, provenance);
    outcome.with_event(event)
}

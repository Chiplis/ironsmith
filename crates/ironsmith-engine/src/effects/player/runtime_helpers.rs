use crate::alternative_cast::CastingMethod;
use crate::effect::EffectOutcome;
use crate::effects::ExecutionContext;
use crate::effects::ExecutionError;
use crate::events::other::LandPlayedEvent;
use crate::events::spells::SpellCastEvent;
use crate::filter::AlternativeCastKind;
use crate::filter::ObjectFilterExt as _;
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
    // `cast_spell_from_resolving_effect` completes the same CR 601 cast
    // transaction as a priority cast and records command-zone commander casts
    // when that transaction is committed.  This helper only publishes the
    // resulting SpellCastEvent; recording here would count effect-driven
    // command-zone casts twice.
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

    // A Saga land gets its lore counter as it enters (CR 714.3a), on the
    // central battlefield-entry path; only add one if that didn't happen.
    if game.object(land_id).is_some_and(|obj| {
        obj.subtypes.contains(&Subtype::Saga)
            && obj.counters.get(&CounterType::Lore).copied().unwrap_or(0) == 0
    }) && let Some(event) = game.add_counters(land_id, CounterType::Lore, 1)
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

#[derive(Debug, Clone)]
pub(super) struct EffectDrivenCastOption {
    pub object_id: ObjectId,
    pub from_zone: Zone,
    pub casting_method: CastingMethod,
    pub label: String,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum EffectDrivenCastPayment {
    WithoutPayingManaCost,
    AlternativeCost(AlternativeCastKind),
}

#[derive(Debug, Clone, Copy)]
pub(super) struct EffectDrivenCastResult {
    pub new_id: ObjectId,
    pub from_zone: Zone,
}

fn cast_filter_matches(
    game: &GameState,
    caster: PlayerId,
    source: ObjectId,
    view: &crate::object::Object,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let filter_ctx = game
        .filter_context_for(caster, Some(source))
        .with_caster(Some(caster));
    filter.matches(view, &filter_ctx, game)
}

/// As [`cast_filter_matches`], against a caller-supplied filter context.
///
/// A permission granted mid-resolution can name its cards by a tag bound
/// earlier in the same resolution ("a spell from among them"). Those bindings
/// live on the execution context, not on the game, so a context built from the
/// game alone can never match them.
fn cast_filter_matches_in_context(
    game: &GameState,
    filter_ctx: &crate::target::FilterContext,
    view: &crate::object::Object,
    filter: &crate::target::ObjectFilter,
) -> bool {
    filter.matches(view, filter_ctx, game)
}

pub(super) fn effect_driven_cast_options_for_card(
    game: &GameState,
    caster: PlayerId,
    source: ObjectId,
    object_id: ObjectId,
    from_zone: Zone,
    filter: &crate::target::ObjectFilter,
) -> Vec<EffectDrivenCastOption> {
    effect_driven_cast_options_for_card_with_payment(
        game,
        caster,
        source,
        object_id,
        from_zone,
        filter,
        EffectDrivenCastPayment::WithoutPayingManaCost,
    )
}

pub(super) fn effect_driven_cast_options_for_card_in_context(
    game: &GameState,
    caster: PlayerId,
    source: ObjectId,
    object_id: ObjectId,
    from_zone: Zone,
    filter: &crate::target::ObjectFilter,
    payment: EffectDrivenCastPayment,
    filter_ctx: &crate::target::FilterContext,
) -> Vec<EffectDrivenCastOption> {
    effect_driven_cast_options_inner(
        game,
        caster,
        source,
        object_id,
        from_zone,
        filter,
        payment,
        Some(filter_ctx),
    )
}

pub(super) fn effect_driven_cast_options_for_card_with_payment(
    game: &GameState,
    caster: PlayerId,
    source: ObjectId,
    object_id: ObjectId,
    from_zone: Zone,
    filter: &crate::target::ObjectFilter,
    payment: EffectDrivenCastPayment,
) -> Vec<EffectDrivenCastOption> {
    effect_driven_cast_options_inner(
        game, caster, source, object_id, from_zone, filter, payment, None,
    )
}

#[allow(clippy::too_many_arguments)]
fn effect_driven_cast_options_inner(
    game: &GameState,
    caster: PlayerId,
    source: ObjectId,
    object_id: ObjectId,
    from_zone: Zone,
    filter: &crate::target::ObjectFilter,
    payment: EffectDrivenCastPayment,
    filter_ctx: Option<&crate::target::FilterContext>,
) -> Vec<EffectDrivenCastOption> {
    let matches_filter = |object: &crate::object::Object| match filter_ctx {
        Some(filter_ctx) => cast_filter_matches_in_context(game, filter_ctx, object, filter),
        None => cast_filter_matches(game, caster, source, object, filter),
    };
    let Some(object) = game.object(object_id) else {
        return Vec::new();
    };
    if object.is_land() || object.zone != from_zone {
        return Vec::new();
    }
    // CR 709.3a: a split card is cast as one half, and only that half is
    // evaluated against the cast permission's filter.
    let chosen_half_view = object.split_combined.is_some().then(|| {
        let mut view = object.clone();
        view.split_combined = None;
        view
    });
    let chosen_half = chosen_half_view.as_ref().unwrap_or(object);

    let mut options = Vec::new();
    if let EffectDrivenCastPayment::AlternativeCost(kind) = payment {
        if !matches_filter(chosen_half) {
            return Vec::new();
        }
        for (idx, method) in object.alternative_casts.iter().enumerate() {
            if alternative_cast_matches_kind(method, kind) {
                options.push(EffectDrivenCastOption {
                    object_id,
                    from_zone,
                    casting_method: CastingMethod::Alternative(idx),
                    label: format!("Cast {} for its {} cost", object.name, method.name()),
                });
            }
        }
        return options;
    }

    if matches_filter(chosen_half) {
        let casting_method = if from_zone == Zone::Hand && object.owner == caster {
            CastingMethod::Normal
        } else {
            CastingMethod::PlayFrom {
                source,
                zone: from_zone,
                use_alternative: None,
            }
        };
        options.push(EffectDrivenCastOption {
            object_id,
            from_zone,
            casting_method,
            label: format!("Cast {}", object.name),
        });
    }

    if let Some(other_half) = crate::decision::spell_view_for_split_other_half_cast(game, object)
        && cast_filter_matches(game, caster, source, &other_half, filter)
    {
        options.push(EffectDrivenCastOption {
            object_id,
            from_zone,
            casting_method: CastingMethod::SplitOtherHalf,
            label: format!("Cast {}", other_half.name),
        });
    }

    options
}

fn alternative_cast_matches_kind(
    method: &crate::alternative_cast::AlternativeCastingMethod,
    kind: AlternativeCastKind,
) -> bool {
    use crate::alternative_cast::AlternativeCastingMethod;
    matches!(
        (kind, method),
        (
            AlternativeCastKind::Blitz,
            AlternativeCastingMethod::Blitz { .. }
        ) | (
            AlternativeCastKind::Dash,
            AlternativeCastingMethod::Dash { .. }
        ) | (
            AlternativeCastKind::Flashback,
            AlternativeCastingMethod::Flashback { .. }
        ) | (
            AlternativeCastKind::JumpStart,
            AlternativeCastingMethod::JumpStart { .. }
        ) | (
            AlternativeCastKind::Escape,
            AlternativeCastingMethod::Escape { .. }
        ) | (
            AlternativeCastKind::Madness,
            AlternativeCastingMethod::Madness { .. }
        ) | (
            AlternativeCastKind::Miracle,
            AlternativeCastingMethod::Miracle { .. }
        ) | (
            AlternativeCastKind::Suspend,
            AlternativeCastingMethod::Suspend { .. }
        )
    )
}

pub(super) fn cast_effect_driven_spell_without_paying(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    caster: PlayerId,
    option: &EffectDrivenCastOption,
) -> Result<Option<EffectDrivenCastResult>, crate::effects::ExecutionError> {
    cast_effect_driven_spell_with_payment(
        game,
        ctx,
        caster,
        option,
        EffectDrivenCastPayment::WithoutPayingManaCost,
    )
}

pub(super) fn cast_effect_driven_spell_with_payment(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    caster: PlayerId,
    option: &EffectDrivenCastOption,
    payment: EffectDrivenCastPayment,
) -> Result<Option<EffectDrivenCastResult>, ExecutionError> {
    let result = crate::game_loop::cast_spell_from_resolving_effect(
        game,
        option.object_id,
        option.from_zone,
        caster,
        &option.casting_method,
        matches!(payment, EffectDrivenCastPayment::WithoutPayingManaCost),
        None,
        ctx.provenance,
        &mut ctx.decision_maker,
    )
    .map_err(|err| crate::effects::ExecutionError::Impossible(err.to_string()))?;
    Ok(result.map(|new_id| EffectDrivenCastResult {
        new_id,
        from_zone: option.from_zone,
    }))
}

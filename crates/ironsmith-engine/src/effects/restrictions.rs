//! Effects that apply rule restrictions ("can't" effects).

use std::collections::HashSet;

use crate::effect::{EffectOutcome, Restriction, RestrictionStart, Until};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::ObjectFilter;
pub use ironsmith_core::CantEffect;

/// The object "this" names for a restriction this resolution creates.
///
/// CR 400.7, 611.2c: the source is the object it still is, or the object a
/// zone-change trigger recorded it becoming. Any other zone change makes it
/// a new object the restriction doesn't affect — including for activated
/// abilities and spells, which carry no triggering event to follow.
fn restriction_source_object(game: &GameState, ctx: &ExecutionContext) -> Option<ObjectId> {
    if game.object(ctx.source).is_some() {
        return Some(ctx.source);
    }
    if ctx
        .triggering_event
        .as_ref()
        .is_some_and(|event| event.downcast::<crate::events::ZoneChangeEvent>().is_some())
    {
        return crate::effects::helpers::resolve_source_object_id(game, ctx);
    }
    None
}

fn collapse_tagged_filter_to_specific_objects(
    filter: &ObjectFilter,
    ctx: &ExecutionContext,
    game: &GameState,
) -> ObjectFilter {
    if filter.source {
        // CR 400.7: follow the source only to the object it still is (or a
        // zone-change trigger's recorded destination), never to the same card
        // after an unrelated zone change. A source that is gone binds the
        // restriction to its old object, which nothing is anymore.
        return ObjectFilter::specific(restriction_source_object(game, ctx).unwrap_or(ctx.source));
    }
    if filter.tagged_constraints.is_empty() {
        return filter.clone();
    }

    if !filter.tagged_constraints.iter().all(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) {
        return filter.clone();
    }

    let mut seen = HashSet::new();
    let mut object_ids = filter
        .tagged_constraints
        .iter()
        .filter_map(|constraint| ctx.get_tagged_all(&constraint.tag))
        .flat_map(|snapshots| snapshots.iter())
        .filter_map(|snapshot| {
            seen.insert(snapshot.object_id)
                .then_some(snapshot.object_id)
        })
        .collect::<Vec<_>>();

    if object_ids.is_empty()
        && let Some(source) = restriction_source_object(game, ctx).filter(|source| {
            game.object(*source)
                .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
        })
    {
        object_ids.push(source);
    }

    if object_ids.is_empty() {
        let mut fallback_seen = HashSet::new();
        object_ids = ctx
            .tagged_objects
            .values()
            .flat_map(|snapshots| snapshots.iter())
            .filter_map(|snapshot| {
                fallback_seen
                    .insert(snapshot.object_id)
                    .then_some(snapshot.object_id)
            })
            .collect();
        // Tag-map iteration order is per-instance; keep the stored filter stable.
        object_ids.sort_unstable();
    }

    match object_ids.as_slice() {
        [] => filter.clone(),
        [object_id] => ObjectFilter::specific(*object_id),
        _ => ObjectFilter {
            any_of: object_ids.into_iter().map(ObjectFilter::specific).collect(),
            ..Default::default()
        },
    }
}

fn bind_restriction_target_players(
    filter: &ObjectFilter,
    ctx: &ExecutionContext,
    game: &GameState,
) -> ObjectFilter {
    let mut resolved = filter.clone();
    for player in [&mut resolved.controller, &mut resolved.owner] {
        if let Some(
            reference @ (crate::target::PlayerFilter::Target(_)
            | crate::target::PlayerFilter::AliasedTarget(_)),
        ) = player
            && let Ok(id) = crate::effects::helpers::resolve_player_filter(game, reference, ctx)
        {
            *reference = crate::target::PlayerFilter::Specific(id);
        }
    }
    // CR 608.2h: a number a resolving instruction reads ("power less than or
    // or equal to that number", Rumbling Ruin) is determined once, as the
    // instruction resolves; the affected objects' own power stays live.
    for comparison in [
        &mut resolved.power,
        &mut resolved.toughness,
        &mut resolved.mana_value,
    ] {
        freeze_resolution_comparison(comparison, ctx, game);
    }
    resolved.any_of = resolved
        .any_of
        .iter()
        .map(|branch| bind_restriction_target_players(branch, ctx, game))
        .collect();
    resolved
}

/// Replace a game-wide quantity in a comparison with its value now. Only
/// quantities that never depend on the compared object are frozen.
fn freeze_resolution_comparison(
    comparison: &mut Option<crate::filter::Comparison>,
    ctx: &ExecutionContext,
    game: &GameState,
) {
    use crate::effect::Value;
    use crate::filter::Comparison;
    let Some(current) = comparison.as_ref() else {
        return;
    };
    let (value, rebuild): (&Value, fn(i32) -> Comparison) = match current {
        Comparison::EqualExpr(value) => (value, Comparison::Equal),
        Comparison::NotEqualExpr(value) => (value, Comparison::NotEqual),
        Comparison::LessThanExpr(value) => (value, Comparison::LessThan),
        Comparison::LessThanOrEqualExpr(value) => (value, Comparison::LessThanOrEqual),
        Comparison::GreaterThanExpr(value) => (value, Comparison::GreaterThan),
        Comparison::GreaterThanOrEqualExpr(value) => (value, Comparison::GreaterThanOrEqual),
        _ => return,
    };
    if !matches!(
        value.unhinted(),
        Value::Count(_)
            | Value::CountersOn(..)
            | Value::TotalPower(_)
            | Value::TotalToughness(_)
            | Value::GreatestPower(_)
            | Value::GreatestToughness(_)
            | Value::GreatestManaValue(_)
            | Value::EffectValue(_)
            | Value::X
    ) {
        return;
    }
    if let Ok(amount) = crate::effects::helpers::resolve_value(game, value, ctx) {
        *comparison = Some(rebuild(amount));
    }
}

fn collapse_filter_to_current_matching_objects(
    filter: &ObjectFilter,
    ctx: &ExecutionContext,
    game: &GameState,
) -> ObjectFilter {
    let resolved = bind_restriction_target_players(filter, ctx, game);
    let filter = &resolved;
    if filter.tagged_constraints.is_empty() {
        return filter.clone();
    }

    let filter_ctx = ctx.filter_context(game);
    let object_ids = game
        .battlefield
        .iter()
        .copied()
        .filter(|object_id| {
            game.object(*object_id).is_some_and(|object| {
                if game.is_phased_out(*object_id) {
                    let snapshot = crate::snapshot::ObjectSnapshot::from_object(object, game);
                    filter.matches_snapshot(&snapshot, &filter_ctx, game)
                } else {
                    filter.matches(object, &filter_ctx, game)
                }
            })
        })
        .collect::<Vec<_>>();

    match object_ids.as_slice() {
        [] => ObjectFilter::specific(crate::ids::ObjectId::from_raw(0)),
        [object_id] => ObjectFilter::specific(*object_id),
        _ => ObjectFilter {
            any_of: object_ids.into_iter().map(ObjectFilter::specific).collect(),
            ..Default::default()
        },
    }
}

/// Freeze a resolving effect's affected object set while its full execution
/// context is still available.
///
/// Unlike a battlefield static restriction, a temporary restriction created
/// by a resolving spell or ability does not keep reevaluating an authored X
/// or a transient result tag. Materialize those matches as concrete object
/// identities before registering the duration.
fn lock_filter_to_current_matching_objects(
    filter: &ObjectFilter,
    ctx: &ExecutionContext,
    game: &GameState,
) -> ObjectFilter {
    let filter_ctx = ctx.filter_context(game);
    let object_ids = game
        .battlefield
        .iter()
        .copied()
        .filter(|object_id| {
            game.object(*object_id).is_some_and(|object| {
                // Phased-out permanents are intentionally absent from normal
                // battlefield queries, but a resolving "they can't phase in"
                // restriction must freeze the set just phased out.  Snapshot
                // matching preserves that identity without making phased-out
                // objects generally visible to other filters.
                if game.is_phased_out(*object_id) {
                    let snapshot = crate::snapshot::ObjectSnapshot::from_object(object, game);
                    filter.matches_snapshot(&snapshot, &filter_ctx, game)
                } else {
                    filter.matches(object, &filter_ctx, game)
                }
            })
        })
        .collect::<Vec<_>>();

    match object_ids.as_slice() {
        [] => ObjectFilter::specific(crate::ids::ObjectId::from_raw(0)),
        [object_id] => ObjectFilter::specific(*object_id),
        _ => ObjectFilter {
            any_of: object_ids.into_iter().map(ObjectFilter::specific).collect(),
            ..Default::default()
        },
    }
}

fn filter_has_not_tagged_constraint(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
    })
}

fn normalize_restriction_for_resolution(
    restriction: &Restriction,
    ctx: &ExecutionContext,
    game: &GameState,
) -> Restriction {
    match restriction {
        Restriction::BlockSpecificAttacker { blockers, attacker } => {
            // A resolving ability's choice belongs to this resolution. A
            // later activation may choose another color on the same source,
            // but must not rewrite this restriction. Keep ordinary blocker
            // characteristics live: creatures that enter later, or change
            // power/color, must still be checked when blocks are declared.
            let blockers = crate::static_abilities::bind_chosen_filter_qualities(
                blockers,
                game,
                ctx.source,
            )
            .unwrap_or_else(|| blockers.clone());
            Restriction::block_specific_attacker(
                blockers,
                collapse_tagged_filter_to_specific_objects(attacker, ctx, game),
            )
        }
        Restriction::BeBlocked(filter) => Restriction::be_blocked(
            collapse_tagged_filter_to_specific_objects(filter, ctx, game),
        ),
        Restriction::BeCountered(filter) => Restriction::be_countered(
            collapse_tagged_filter_to_specific_objects(filter, ctx, game),
        ),
        Restriction::MaximumBlockers { filter, maximum } => Restriction::MaximumBlockers {
            filter: collapse_tagged_filter_to_specific_objects(filter, ctx, game),
            maximum: *maximum,
        },
        Restriction::MustBlock(filter) => Restriction::must_block(
            collapse_tagged_filter_to_specific_objects(filter, ctx, game),
        ),
        Restriction::MustAttack(filter) => Restriction::must_attack(
            // Plain creature/controller filters stay live. Exact anaphoric
            // object references remain the identities the instruction named.
            // A targeted controller ("each creature that player controls",
            // Rowan Kenrith) keeps the announced player after the target
            // slots are gone.
            collapse_tagged_filter_to_specific_objects(
                &bind_restriction_target_players(filter, ctx, game),
                ctx,
                game,
            ),
        ),
        // "This creature attacks that player this combat if able": the
        // creature and the player are the ones this resolution named
        // (CR 608.2c), so a later choice cannot redirect the requirement.
        Restriction::MustAttackPlayer { attackers, player } => Restriction::must_attack_player(
            // A targeted controller ("creatures that player controls attack
            // you", Taunt) keeps the announced player once target slots end.
            collapse_tagged_filter_to_specific_objects(
                &bind_restriction_target_players(attackers, ctx, game),
                ctx,
                game,
            ),
            // A group ("a player": any opponent) stays a group; a single
            // named player is bound now.
            if matches!(
                player,
                crate::target::PlayerFilter::Any | crate::target::PlayerFilter::Opponent
            ) {
                player.clone()
            } else {
                crate::effects::helpers::resolve_player_filter(game, player, ctx)
                    .map(crate::target::PlayerFilter::Specific)
                    .unwrap_or_else(|_| player.clone())
            },
        ),
        Restriction::MustBeBlocked(filter) => Restriction::must_be_blocked(
            collapse_filter_to_current_matching_objects(filter, ctx, game),
        ),
        Restriction::Attack(filter) if filter_has_not_tagged_constraint(filter) => {
            Restriction::attack(bind_restriction_target_players(filter, ctx, game))
        }
        Restriction::Attack(filter) => Restriction::attack(
            collapse_filter_to_current_matching_objects(filter, ctx, game),
        ),
        Restriction::AttackPlayerOrPlaneswalkersControlledBy { attackers, player } => {
            Restriction::attack_player_or_planeswalkers_controlled_by(
                collapse_filter_to_current_matching_objects(attackers, ctx, game),
                player.clone(),
            )
        }
        Restriction::Block(filter) if filter_has_not_tagged_constraint(filter) => {
            Restriction::block(bind_restriction_target_players(filter, ctx, game))
        }
        Restriction::Block(filter) => Restriction::block(
            collapse_filter_to_current_matching_objects(filter, ctx, game),
        ),
        Restriction::Untap(filter) => Restriction::untap(
            bind_restriction_target_players(filter, ctx, game),
        ),
        Restriction::BecomeUntapped(filter) => Restriction::become_untapped(
            bind_restriction_target_players(filter, ctx, game),
        ),
        Restriction::AttackOrBlock(filter) => {
            Restriction::attack_or_block(lock_filter_to_current_matching_objects(filter, ctx, game))
        }
        Restriction::PhaseIn(filter) => {
            Restriction::phase_in(lock_filter_to_current_matching_objects(filter, ctx, game))
        }
        // "You gain protection from that player": the targeting sources are
        // the ones the player named by this resolution controls (CR 702.16k).
        Restriction::BeTargetedPlayerFrom(player, source_filter) => {
            Restriction::be_targeted_player_from(
                player.clone(),
                crate::effects::player_reference_binding::bind_filter_player_references(
                    source_filter,
                    game,
                    ctx,
                ),
            )
        }
        _ => restriction.clone(),
    }
}

#[derive(Debug)]
struct CantEffectProposal {
    effect: CantEffect,
    iterated_player: Option<crate::ids::PlayerId>,
    tagged_objects:
        std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
}

impl crate::effects::SimultaneousEffectProposal for CantEffectProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Self {
            effect,
            iterated_player,
            tagged_objects,
        } = *self;
        ctx.with_temp_iterated_player(iterated_player, |ctx| {
            // A preceding read-only choice can define the matching set for a
            // restriction ("only creatures in the chosen pile can block").
            // ForPlayers restores the shared pre-action tags before committing
            // proposals, so carry this player's frozen choice into the commit.
            let previous_tags = std::mem::replace(&mut ctx.tagged_objects, tagged_objects);
            let outcome = effect.execute(game, ctx);
            ctx.tagged_objects = previous_tags;
            outcome
        })
    }
}

/// A followup names the exact objects of its earlier instruction, not later
/// incarnations found through the generic stable-card membership relation.
fn exact_untap_subject(filter: &ObjectFilter) -> ObjectFilter {
    let mut filter = filter.clone();
    for constraint in &mut filter.tagged_constraints {
        if constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject {
            constraint.relation = crate::filter::TaggedOpbjectRelation::SameObjectId;
        }
    }
    filter.any_of = filter.any_of.iter().map(exact_untap_subject).collect();
    filter
}

/// Effect that applies a restriction for a duration.
/// Split an object-subject restriction whose `ForAsLongAs` duration names
/// the affected object into one restriction per currently matching object,
/// each lasting while that object's condition holds.
fn execute_cant_per_affected_object(
    effect: &CantEffect,
    predicate: &ironsmith_core::ContinuousDurationPredicate,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> EffectOutcome {
    {
        let restriction = normalize_restriction_for_resolution(&effect.restriction, ctx, game);
        let Some(subjects) = restriction_object_subject(&restriction) else {
            return EffectOutcome::count(0);
        };
        let filter_ctx = ctx.filter_context(game);
        let objects: Vec<_> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id)
                    .is_some_and(|object| subjects.matches(object, &filter_ctx, game))
            })
            .collect();
        let mut added = 0;
        for object_id in objects {
            let Some(object_predicate) = crate::effects::continuous::materialize_duration_predicate(
                predicate,
                &crate::continuous::EffectTarget::Specific(object_id),
                &None,
                game,
                ctx,
            ) else {
                continue;
            };
            if !crate::continuous::continuous_duration_predicate_matches(&object_predicate, game) {
                continue;
            }
            let Some(object_restriction) = restriction_with_object_subject(
                &restriction,
                crate::target::ObjectFilter::specific(object_id),
            ) else {
                continue;
            };
            game.add_restriction_effect_with_start_and_tagged_objects(
                object_restriction,
                Until::ForAsLongAs(object_predicate),
                ctx.source,
                ctx.controller,
                ctx.iteration.iterated_player,
                None,
                ctx.tagged_objects.clone(),
            );
            added += 1;
        }
        if added > 0 {
            game.update_cant_effects();
        }
        EffectOutcome::count(added)
    }
}

/// The object filter a restriction applies to, for the restrictions whose
/// subject is a set of objects.
fn restriction_object_subject(restriction: &Restriction) -> Option<&crate::target::ObjectFilter> {
    match restriction {
        Restriction::Attack(filter)
        | Restriction::Untap(filter)
        | Restriction::Block(filter)
        | Restriction::AttackOrBlock(filter)
        | Restriction::AttackPlayerOrPlaneswalkersControlledBy {
            attackers: filter, ..
        }
        | Restriction::AttackPlayer {
            attackers: filter, ..
        } => Some(filter),
        _ => None,
    }
}

fn restriction_with_object_subject(
    restriction: &Restriction,
    subject: crate::target::ObjectFilter,
) -> Option<Restriction> {
    Some(match restriction {
        Restriction::Attack(_) => Restriction::attack(subject),
        Restriction::Untap(_) => Restriction::untap(subject),
        Restriction::Block(_) => Restriction::block(subject),
        Restriction::AttackOrBlock(_) => Restriction::attack_or_block(subject),
        Restriction::AttackPlayerOrPlaneswalkersControlledBy { player, .. } => {
            Restriction::attack_player_or_planeswalkers_controlled_by(subject, player.clone())
        }
        Restriction::AttackPlayer { player, .. } => {
            Restriction::attack_player(subject, player.clone())
        }
        _ => return None,
    })
}

impl EffectExecutor for CantEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(CantEffectProposal {
            effect: self.clone(),
            iterated_player: ctx.iteration.iterated_player,
            tagged_objects: ctx.tagged_objects.clone(),
        }))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_atomically(game, ctx, |game, ctx| {
        game.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;
        let duration = if let Until::ForAsLongAs(predicate) = &self.duration {
            let Some(predicate) = crate::effects::continuous::materialize_duration_predicate(
                predicate,
                &crate::continuous::EffectTarget::Source,
                &None,
                game,
                ctx,
            ) else {
                // "Each of those creatures can't attack you ... for as long
                // as it has a vow counter on it" (Promise of Loyalty): the
                // duration names each restricted object, so each object gets
                // its own restriction lasting while its own condition holds.
                return Ok(execute_cant_per_affected_object(self, predicate, game, ctx));
            };
            if !crate::continuous::continuous_duration_predicate_matches(&predicate, game) {
                return Ok(EffectOutcome::count(0));
            }
            Until::ForAsLongAs(predicate)
        } else if let Until::PlayersNextUntapStep { player } = &self.duration {
            Until::PlayersNextUntapStep {
                player: crate::target::PlayerFilter::Specific(
                    crate::effects::helpers::resolve_player_filter(game, player, ctx)?,
                ),
            }
        } else {
            self.duration.clone()
        };
        let restriction = normalize_restriction_for_resolution(&self.restriction, ctx, game);
        // A resolved player prohibition must retain its announced player after
        // this context and target slots disappear. The land set remains a rule
        // about future plays, not the currently visible lands.
        let restriction = if let Restriction::PlayerHexproofFrom(player, filter) = restriction {
            // A temporary player grant must keep its resolved recipient after
            // target slots and other execution-only references are gone.
            let player = match player {
                crate::target::PlayerFilter::Target(_)
                | crate::target::PlayerFilter::AliasedTarget(_)
                | crate::target::PlayerFilter::TargetPlayerOrControllerOfTarget
                | crate::target::PlayerFilter::IteratedPlayer
                | crate::target::PlayerFilter::TaggedPlayer(_)
                | crate::target::PlayerFilter::ChosenPlayer => {
                    crate::target::PlayerFilter::Specific(
                        crate::effects::helpers::resolve_player_filter(game, &player, ctx)?,
                    )
                }
                player => player,
            };
            let filter = crate::static_abilities::bind_chosen_filter_qualities(
                &filter, game, ctx.source,
            ).unwrap_or(filter);
            Restriction::PlayerHexproofFrom(player, filter)
        } else if let Restriction::PlayLandsMatching(player, filter) = restriction {
            let player = match player {
                crate::target::PlayerFilter::Target(_)
                | crate::target::PlayerFilter::AliasedTarget(_)
                | crate::target::PlayerFilter::TargetPlayerOrControllerOfTarget
                | crate::target::PlayerFilter::IteratedPlayer
                | crate::target::PlayerFilter::TaggedPlayer(_)
                | crate::target::PlayerFilter::ChosenPlayer => {
                    crate::target::PlayerFilter::Specific(
                        crate::effects::helpers::resolve_player_filter(game, &player, ctx)?,
                    )
                }
                player => player,
            };
            Restriction::PlayLandsMatching(player, filter)
        } else {
            restriction
        };
        if self.start == RestrictionStart::LastAddedCombatPhase {
            // A missing phase cannot turn a phase-bound restriction into an
            // immediate restriction on an unrelated combat.
            if let Some(order) = ctx.combat.last_added_combat_order {
                game.add_restriction_effect_with_start_and_tagged_objects(
                    restriction,
                    duration.clone(),
                    ctx.source,
                    ctx.controller,
                    ctx.iteration.iterated_player,
                    None,
                    ctx.tagged_objects.clone(),
                );
                game.effect_store
                    .restriction_effects
                    .last_mut()
                    .unwrap()
                    .starts_in_added_combat = Some(order);
                game.update_cant_effects();
            }
            return Ok(EffectOutcome::resolved());
        }
        let starts_next_turn_of = match &self.start {
            RestrictionStart::Immediate | RestrictionStart::LastAddedCombatPhase => None,
            RestrictionStart::NextTurn(player) => Some(
                crate::effects::helpers::resolve_player_filter(game, player, ctx)?,
            ),
        };
        if matches!(duration, Until::ControllersNextUntapStep | Until::YourNextUntapStep)
            && let Restriction::Untap(filter) = &restriction
        {
            let filter = exact_untap_subject(filter);
            let filter_ctx = ctx.filter_context(game);
            let targets: Vec<_> = game
                .battlefield
                .iter()
                .filter_map(|object_id| {
                    let obj = game.object(*object_id)?;
                    if game.is_phased_out(*object_id) { return None; }
                    if filter.matches(obj, &filter_ctx, game) {
                        Some((*object_id, game.controller_of(obj)))
                    } else {
                        None
                    }
                })
                .collect();

            for (object_id, _) in targets {
                game.add_restriction_effect_with_start_and_tagged_objects(
                    Restriction::untap(crate::target::ObjectFilter::specific(object_id)),
                    duration.clone(),
                    ctx.source,
                    ctx.controller,
                    ctx.iteration.iterated_player,
                    starts_next_turn_of,
                    Default::default(),
                );
                if matches!(duration, Until::ControllersNextUntapStep) {
                    game.effect_store.restriction_effects.last_mut().unwrap().untap_step_object = Some(object_id);
                }
                game.effect_store
                    .restriction_effects
                    .last_mut()
                    .unwrap()
                    .additional_untap_steps = self.duration_surface.additional_untap_steps();
            }
            // A known empty affected set cannot become a future broad rule.
        } else {
            game.add_restriction_effect_with_start_and_tagged_objects(
                restriction,
                duration.clone(),
                ctx.source,
                ctx.controller,
                ctx.iteration.iterated_player,
                starts_next_turn_of,
                ctx.tagged_objects.clone(),
            );
        }
        game.update_cant_effects();
        Ok(EffectOutcome::resolved())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PowerToughness;
    use crate::card::CardBuilder;
    use crate::effects::ExecutionContext;
    use crate::effects::RegenerateEffect;
    use crate::game_state::{GameState, Phase, Step};
    use crate::ids::CardId;
    use crate::ids::PlayerId;
    use crate::snapshot::ObjectSnapshot;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn resolved_evasion_freezes_choice_but_keeps_blocker_characteristics_live() {
        use crate::color::{Color, ColorSet};
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature = |game: &mut GameState, owner, color| {
            let card = CardBuilder::new(CardId::new(), "Evasion participant")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .color_indicator(color).build();
            game.create_object_from_card(&card, owner, Zone::Battlefield)
        };
        let source = creature(&mut game, alice, ColorSet::WHITE);
        let red = creature(&mut game, bob, ColorSet::RED);
        let blue = creature(&mut game, bob, ColorSet::BLUE);
        let green = creature(&mut game, bob, ColorSet::GREEN);
        let can_block = |game: &GameState, attacker, blocker| {
            crate::rules::combat::can_block(
                game.object(attacker).unwrap(), game.object(blocker).unwrap(), game,
            )
        };
        let effect = CantEffect::until_end_of_turn(Restriction::block_specific_attacker(
            ObjectFilter::creature().of_chosen_color(), ObjectFilter::source(),
        ));
        let mut ctx = ExecutionContext::new_default(source, alice);
        game.set_chosen_color(source, Color::Red);
        effect.execute(&mut game, &mut ctx).unwrap();
        game.set_chosen_color(source, Color::Blue);
        effect.execute(&mut game, &mut ctx).unwrap();
        assert!(!can_block(&game, source, red));
        assert!(!can_block(&game, source, blue));
        assert!(can_block(&game, source, green));
        let late_red = creature(&mut game, bob, ColorSet::RED);
        game.refresh_continuous_state().unwrap();
        assert!(!can_block(&game, source, late_red));
        game.object_mut(green).unwrap().color_override = Some(ColorSet::RED);
        game.refresh_continuous_state().unwrap();
        assert!(!can_block(&game, source, green));
        game.object_mut(red).unwrap().color_override = Some(ColorSet::GREEN);
        game.refresh_continuous_state().unwrap();
        assert!(can_block(&game, source, red));
        game.cleanup_restrictions_end_of_turn();
        game.refresh_continuous_state().unwrap();
        assert!(can_block(&game, source, blue));
        assert!(can_block(&game, source, green));
    }

    #[test]
    fn absent_color_cannot_be_supplied_by_a_later_choice() {
        use crate::color::{Color, ColorSet};
        for blocker_colors in [ColorSet::RED, ColorSet::RED.with(Color::Blue)] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let card = CardBuilder::new(CardId::new(), "Color witness")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .color_indicator(blocker_colors).build();
            let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
            let blocker = game.create_object_from_card(&card, bob, Zone::Battlefield);
            let effect = CantEffect::until_end_of_turn(Restriction::block_specific_attacker(
                ObjectFilter::creature().with_colors(ColorSet::RED).of_chosen_color(),
                ObjectFilter::source(),
            ));
            effect.execute(&mut game, &mut ExecutionContext::new_default(source, alice)).unwrap();
            game.set_chosen_color(source, Color::Red);
            game.refresh_continuous_state().unwrap();
            assert!(crate::rules::combat::can_block(
                game.object(source).unwrap(), game.object(blocker).unwrap(), &game,
            ));
        }
    }

    #[test]
    fn fixed_and_chosen_colors_remain_conjunctive_after_resolution() {
        use crate::color::{Color, ColorSet};
        for require_green in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let creature = |game: &mut GameState, owner, colors| {
                let card = CardBuilder::new(CardId::new(), "Conjunctive color witness")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(2, 2))
                    .color_indicator(colors).build();
                game.create_object_from_card(&card, owner, Zone::Battlefield)
            };
            let source = creature(&mut game, alice, ColorSet::WHITE);
            let red = creature(&mut game, bob, ColorSet::RED);
            let blue = creature(&mut game, bob, ColorSet::BLUE);
            let red_blue = creature(&mut game, bob, ColorSet::RED.with(Color::Blue));
            let red_blue_green = creature(&mut game, bob, ColorSet::RED.with(Color::Blue).with(Color::Green));
            let mut blockers = ObjectFilter::creature().with_colors(ColorSet::RED).of_chosen_color();
            if require_green { blockers.required_colors = Some(ColorSet::GREEN); }
            game.set_chosen_color(source, Color::Blue);
            CantEffect::until_end_of_turn(Restriction::block_specific_attacker(
                blockers, ObjectFilter::source(),
            )).execute(&mut game, &mut ExecutionContext::new_default(source, alice)).unwrap();
            for next_choice in [Color::Blue, Color::Red, Color::Green] {
                game.set_chosen_color(source, next_choice);
                game.refresh_continuous_state().unwrap();
                let can_block = |blocker| crate::rules::combat::can_block(
                    game.object(source).unwrap(), game.object(blocker).unwrap(), &game,
                );
                assert!(can_block(red));
                assert!(can_block(blue));
                assert_eq!(can_block(red_blue), require_green);
                assert!(!can_block(red_blue_green));
            }
        }
    }

    #[test]
    fn added_combat_restriction_waits_through_other_combats_and_expires() {
        for expire_pending in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
            let restriction = CantEffect::starting(
                Restriction::gain_life(PlayerFilter::Any),
                Until::EndOfCombat,
                RestrictionStart::LastAddedCombatPhase,
            );
            // No preceding phase must not accidentally register a global rule.
            restriction.execute(&mut game, &mut ctx).unwrap();
            assert!(game.effect_store.restriction_effects.is_empty());
            crate::effects::AdditionalPhasesEffect {
                phases: vec![crate::effects::AdditionalPhase::Combat],
                after_main_phase: false,
            }
            .execute(&mut game, &mut ctx)
            .unwrap();
            restriction.execute(&mut game, &mut ctx).unwrap();
            assert!(game.can_gain_life(alice));
            game.add_additional_phase_group([Phase::Combat]);
            game.pop_additional_phase();
            assert!(game.can_gain_life(alice));
            game.cleanup_restrictions_end_of_combat();
            assert_eq!(game.effect_store.restriction_effects.len(), 1);
            if expire_pending {
                game.cleanup_restrictions_end_of_turn();
                assert!(game.effect_store.restriction_effects.is_empty());
            } else {
                game.pop_additional_phase();
                assert!(!game.can_gain_life(alice));
                game.cleanup_restrictions_end_of_combat();
                assert!(game.can_gain_life(alice));
                assert!(game.effect_store.restriction_effects.is_empty());
            }
        }
    }

    #[test]
    fn cant_effect_blocks_life_gain() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CantEffect::until_end_of_turn(Restriction::gain_life(PlayerFilter::Any));
        effect.execute(&mut game, &mut ctx).expect("execute cant");

        game.update_cant_effects();

        assert!(!game.can_gain_life(PlayerId::from_index(0)));
        assert!(!game.can_gain_life(PlayerId::from_index(1)));
    }

    #[test]
    fn forever_player_rule_removes_only_that_players_maximum_hand_size() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        CantEffect::new(
            Restriction::no_maximum_hand_size(PlayerFilter::You),
            Until::Forever,
        )
        .execute(&mut game, &mut ctx)
        .expect("register the lasting player rule");

        assert_eq!(game.player(alice).unwrap().max_hand_size, i32::MAX);
        assert_eq!(game.player(bob).unwrap().max_hand_size, 7);
        assert_eq!(game.effect_store.restriction_effects.len(), 1);

        // Rebuilding derived rule state must retain a resolving effect whose
        // duration is the rest of the game.
        game.update_cant_effects();
        assert_eq!(game.player(alice).unwrap().max_hand_size, i32::MAX);
        assert_eq!(game.player(bob).unwrap().max_hand_size, 7);
    }

    #[test]
    fn cant_effect_can_start_at_the_affected_players_next_turn_boundary() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let spell_filter = ObjectFilter::default().with_type(CardType::Instant);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.iteration.iterated_player = Some(bob);
        CantEffect::during_next_turn(
            Restriction::cast_spells_matching(PlayerFilter::IteratedPlayer, spell_filter.clone()),
            PlayerFilter::IteratedPlayer,
        )
        .execute(&mut game, &mut ctx)
        .expect("schedule next-turn cast restriction");

        assert!(game.effect_store.restriction_effects[0].is_pending());
        assert!(
            game.effect_store
                .cant_effects
                .cast_filters_for_player(bob)
                .is_none(),
            "the restriction must not apply during the current turn"
        );

        game.next_turn();
        assert_eq!(game.turn.active_player, bob);
        assert!(
            game.effect_store
                .cant_effects
                .cast_filters_for_player(bob)
                .is_some_and(|filters| filters.iter().any(|entry| entry.filter == spell_filter)),
            "the restriction must be active before priority on the affected player's next turn"
        );

        game.next_turn();
        assert_eq!(game.turn.active_player, alice);
        assert!(
            game.effect_store
                .cant_effects
                .cast_filters_for_player(bob)
                .is_none(),
            "the restriction must expire after that turn"
        );
    }

    #[test]
    fn next_turn_restriction_waits_if_the_affected_players_turn_is_skipped() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        ctx.iteration.iterated_player = Some(bob);

        CantEffect::during_next_turn(
            Restriction::gain_life(PlayerFilter::IteratedPlayer),
            PlayerFilter::IteratedPlayer,
        )
        .execute(&mut game, &mut ctx)
        .expect("schedule next-turn restriction");
        game.turn_store.skip_next_turn.insert(bob);

        game.next_turn();
        assert_eq!(game.turn.active_player, charlie);
        assert!(game.can_gain_life(bob));
        assert!(game.effect_store.restriction_effects[0].is_pending());

        game.next_turn();
        assert_eq!(game.turn.active_player, alice);
        assert!(game.can_gain_life(bob));

        game.next_turn();
        assert_eq!(game.turn.active_player, bob);
        assert!(!game.can_gain_life(bob));
    }

    #[test]
    fn cant_phase_out_restriction_expires_at_controller_next_upkeep() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let permanent_card = CardBuilder::new(CardId::from_raw(11), "Anchored Relic")
            .card_types(vec![CardType::Artifact])
            .build();
        let permanent_id = game.create_object_from_card(&permanent_card, alice, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(permanent_id, alice);
        CantEffect::new(
            Restriction::phase_out(ObjectFilter::specific(permanent_id)),
            Until::YourNextUpkeep,
        )
        .execute(&mut game, &mut ctx)
        .expect("apply phase-out restriction");

        assert!(!game.can_phase_out(permanent_id));

        game.next_turn();
        game.update_cant_effects();
        assert!(
            !game.can_phase_out(permanent_id),
            "restriction should remain active through another player's turn"
        );

        game.next_turn();
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Untap);
        game.update_cant_effects();
        assert!(
            !game.can_phase_out(permanent_id),
            "restriction should remain active through the controller's untap step"
        );

        game.turn.step = Some(Step::Upkeep);
        game.update_cant_effects();
        assert!(
            game.can_phase_out(permanent_id),
            "restriction should expire as the controller's next upkeep begins"
        );
    }

    #[test]
    fn cant_be_regenerated_clears_existing_regeneration_shields() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_card = CardBuilder::new(CardId::from_raw(1), "Shielded Bear")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

        let mut regen_ctx = ExecutionContext::new_default(creature_id, alice);
        RegenerateEffect::source(Until::EndOfTurn)
            .execute(&mut game, &mut regen_ctx)
            .expect("apply regeneration shield");
        assert!(
            game.effect_store
                .replacement_effects
                .count_one_shot_effects_from_source(creature_id)
                > 0
        );

        let source = game.new_object_id();
        let mut cant_ctx = ExecutionContext::new_default(source, alice);
        CantEffect::until_end_of_turn(Restriction::be_regenerated(ObjectFilter::specific(
            creature_id,
        )))
        .execute(&mut game, &mut cant_ctx)
        .expect("apply cant be regenerated");

        assert!(!game.can_be_regenerated(creature_id));
        assert_eq!(
            game.effect_store
                .replacement_effects
                .count_one_shot_effects_from_source(creature_id),
            0
        );
    }

    #[test]
    fn cant_effect_normalizes_source_tagged_be_blocked_filter() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_card = CardBuilder::new(CardId::from_raw(1), "Tagged Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

        let source_snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("source creature exists"),
            &game,
        );
        let mut ctx = ExecutionContext::new_default(creature_id, alice);
        ctx.tag_object("carry", source_snapshot);

        CantEffect::until_end_of_turn(Restriction::be_blocked(ObjectFilter::tagged("carry")))
            .execute(&mut game, &mut ctx)
            .expect("execute be blocked cant effect");

        assert!(
            !game.can_be_blocked(creature_id),
            "tagged source be-blocked restriction should normalize to the source object"
        );
    }

    #[test]
    fn cant_effect_normalizes_tagged_be_blocked_filter_even_when_source_is_stack_object() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_card = CardBuilder::new(CardId::from_raw(1), "Tagged Octopus")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 1))
            .build();
        let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
        let stack_object_id = game.new_object_id();

        let creature_snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("tagged creature exists"),
            &game,
        );
        let mut ctx = ExecutionContext::new_default(stack_object_id, alice);
        ctx.tag_object("carry", creature_snapshot);

        CantEffect::until_end_of_turn(Restriction::be_blocked(ObjectFilter::tagged("carry")))
            .execute(&mut game, &mut ctx)
            .expect("execute be blocked cant effect from stack source");

        assert!(
            !game.can_be_blocked(creature_id),
            "tagged be-blocked restriction should stay attached to the resolved creature, not the stack object source"
        );
    }

    #[test]
    fn cant_effect_applies_block_restrictions_with_iterated_player() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let chosen_card = CardBuilder::new(CardId::from_raw(1), "Chosen Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let chosen_id = game.create_object_from_card(&chosen_card, bob, Zone::Battlefield);
        let other_card = CardBuilder::new(CardId::from_raw(2), "Other Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let other_id = game.create_object_from_card(&other_card, bob, Zone::Battlefield);

        let chosen_snapshot = ObjectSnapshot::from_object(
            game.object(chosen_id).expect("chosen creature exists"),
            &game,
        );
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        ctx.iteration.iterated_player = Some(bob);
        ctx.tag_object("divvy_chosen", chosen_snapshot);

        CantEffect::until_end_of_turn(Restriction::block(
            ObjectFilter::creature()
                .controlled_by(PlayerFilter::IteratedPlayer)
                .not_tagged("divvy_chosen"),
        ))
        .execute(&mut game, &mut ctx)
        .expect("execute block cant effect");

        assert!(
            game.can_block(chosen_id),
            "the chosen pile should remain able to block"
        );
        assert!(
            !game.can_block(other_id),
            "the unchosen pile should be unable to block"
        );
    }

    #[test]
    fn simultaneous_cant_effect_keeps_the_players_frozen_tagged_choice() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let chosen_card = CardBuilder::new(CardId::from_raw(11), "Chosen Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let chosen_id = game.create_object_from_card(&chosen_card, bob, Zone::Battlefield);
        let other_card = CardBuilder::new(CardId::from_raw(12), "Other Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let other_id = game.create_object_from_card(&other_card, bob, Zone::Battlefield);

        let chosen_snapshot = ObjectSnapshot::from_object(
            game.object(chosen_id).expect("chosen creature exists"),
            &game,
        );
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        ctx.iteration.iterated_player = Some(bob);
        ctx.tag_object("divvy_chosen", chosen_snapshot);

        let effect = CantEffect::until_end_of_turn(Restriction::block(
            ObjectFilter::creature()
                .controlled_by(PlayerFilter::IteratedPlayer)
                .not_tagged("divvy_chosen"),
        ));
        let proposal = effect
            .prepare_simultaneous_player_action(&game, &mut ctx)
            .expect("prepare block restriction");

        // The each-player coordinator restores the pre-action context before
        // committing every frozen proposal.
        ctx.tagged_objects.clear();
        proposal
            .commit(&mut game, &mut ctx)
            .expect("commit block restriction");

        assert!(
            game.can_block(chosen_id),
            "the chosen pile should remain able to block"
        );
        assert!(
            !game.can_block(other_id),
            "the unchosen pile should be unable to block"
        );
    }

    #[test]
    fn cant_effect_applies_fight_or_flight_attack_restrictions_with_iterated_player() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let chosen_card = CardBuilder::new(CardId::from_raw(3), "Chosen Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let chosen_id = game.create_object_from_card(&chosen_card, bob, Zone::Battlefield);
        let other_card = CardBuilder::new(CardId::from_raw(4), "Other Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let other_id = game.create_object_from_card(&other_card, bob, Zone::Battlefield);

        let chosen_snapshot = ObjectSnapshot::from_object(
            game.object(chosen_id).expect("chosen creature exists"),
            &game,
        );
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        ctx.iteration.iterated_player = Some(bob);
        ctx.tag_object("divvy_chosen", chosen_snapshot);

        CantEffect::until_end_of_turn(Restriction::attack(
            ObjectFilter::creature()
                .controlled_by(PlayerFilter::IteratedPlayer)
                .not_tagged("divvy_chosen"),
        ))
        .execute(&mut game, &mut ctx)
        .expect("execute attack cant effect");

        assert!(
            game.can_attack(chosen_id),
            "the Fight or Flight chosen pile should remain able to attack"
        );
        assert!(
            !game.can_attack(other_id),
            "creatures outside the Fight or Flight chosen pile should be unable to attack"
        );
    }

    #[test]
    fn cant_effect_normalizes_tagged_be_blocked_filter_when_runtime_tag_aliases_drift() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_card = CardBuilder::new(CardId::from_raw(1), "Alias Drift Octopus")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 1))
            .build();
        let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

        let creature_snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("tagged creature exists"),
            &game,
        );
        let mut ctx = ExecutionContext::new_default(creature_id, alice);
        ctx.tag_object("granted_0", creature_snapshot.clone());
        ctx.tag_object("__it__", creature_snapshot);

        CantEffect::until_end_of_turn(Restriction::be_blocked(ObjectFilter::tagged("targeted_0")))
            .execute(&mut game, &mut ctx)
            .expect("execute be blocked cant effect with drifted runtime tag alias");

        assert!(
            !game.can_be_blocked(creature_id),
            "tagged be-blocked restriction should still resolve when the runtime context only retains equivalent aliases for the same object"
        );
    }
}

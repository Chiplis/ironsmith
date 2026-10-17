//! Register a one-shot spell-cost reduction for the next matching spell this turn.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter_to_list, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::PlayerId;

pub type GrantNextSpellCostReductionEffect = ironsmith_core::GrantNextSpellCostReductionEffect;

impl EffectExecutor for GrantNextSpellCostReductionEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let players =
            resolve_player_filter_to_list(game, &self.player, &ctx.filter_context(game), ctx)?;
        if self.without_paying_mana_cost {
            for player in players {
                let mut filter = self.filter.clone();
                lock_target_player_filters_for_player(&mut filter, player);
                if !lock_source_type_choice(&mut filter, game, ctx) { continue; }
                // The permission is offered on the card before it is cast;
                // the spell-only facets are checked when the cast consumes it.
                filter.zone = None;
                filter.stack_kind = None;
                filter.cast_by = None;
                game.effect_store
                    .grant_registry
                    .grant_alternative_cast_to_next_matching_spell(
                        filter,
                        crate::zone::Zone::Hand,
                        player,
                        if self.pay_life_equal_to_mana_value {
                            // The spell being cast is the payment's source,
                            // so its mana value is the life paid.
                            crate::alternative_cast::AlternativeCastingMethod::alternative_cost(
                                "Pay life equal to its mana value",
                                None,
                                vec![crate::costs::Cost::effect(crate::effects::PayLifeEffect::you(
                                    crate::effect::Value::ManaValueOf(Box::new(
                                        crate::target::ChooseSpec::Source,
                                    )),
                                ))],
                            )
                        } else {
                            crate::alternative_cast::AlternativeCastingMethod::alternative_cost(
                                "Without paying its mana cost",
                                None,
                                vec![],
                            )
                        },
                        crate::grant_registry::GrantSource::Effect {
                            source_id: ctx.source,
                            expires_end_of_turn: game.turn.turn_number,
                        },
                    );
            }
            return Ok(EffectOutcome::resolved());
        }
        if self.generic_reduction.is_some() {
            let amount = self
                .generic_reduction
                .as_ref()
                .map(|value| resolve_value(game, value, ctx))
                .transpose()?
                .unwrap_or(0)
                .max(0);
            let remaining_uses = if self.applies_to_all_matching_this_turn {
                u32::MAX
            } else {
                1
            };
            for player in players {
                let mut filter = self.filter.clone();
                lock_target_player_filters_for_player(&mut filter, player);
                if !lock_source_type_choice(&mut filter, game, ctx) { continue; }
                // The registered rule outlives this resolution, so a relative
                // "that player" in the spell filter must name the player it
                // meant now, not whoever is iterated when a spell is cast.
                lock_iterated_player_filters(&mut filter, ctx.iteration.iterated_player);
                if self.increases_cost {
                    game.add_temporary_generic_spell_cost_increase_until(
                        player,
                        ctx.source,
                        ctx.controller,
                        filter,
                        crate::effect::Value::Fixed(amount),
                        remaining_uses,
                        self.applies_to_all_matching_this_turn,
                        self.duration.clone(),
                    );
                    continue;
                }
                game.add_temporary_generic_spell_cost_reduction_until(
                    player,
                    ctx.source,
                    ctx.controller,
                    filter,
                    crate::effect::Value::Fixed(amount),
                    remaining_uses,
                    self.applies_to_all_matching_this_turn,
                    self.duration.clone(),
                );
            }
        } else {
            for player in players {
                let mut filter = self.filter.clone();
                lock_target_player_filters_for_player(&mut filter, player);
                lock_iterated_player_filters(&mut filter, ctx.iteration.iterated_player);
                game.add_temporary_spell_cost_reduction_until(
                    player,
                    ctx.source,
                    ctx.controller,
                    filter,
                    self.reduction.clone(),
                    1,
                    self.duration.clone(),
                );
            }
        }
        Ok(EffectOutcome::resolved())
    }
}

/// A temporary rule retains the choice read by the resolving ability, even
/// after its source leaves or is given a different choice later.
fn lock_source_type_choice(
    filter: &mut crate::target::ObjectFilter,
    game: &GameState,
    ctx: &ExecutionContext,
) -> bool {
    let snapshot = game.source_departure_snapshot(ctx.source).or_else(|| {
        ctx.source_snapshot.as_ref().filter(|snapshot| snapshot.object_id == ctx.source)
    });
    let card_type = game.chosen_card_type(ctx.source).or_else(|| snapshot.and_then(|snapshot| snapshot.chosen_card_type));
    if filter.chosen_creature_type && !filter.has_chosen_type_this_way_surface() {
        let subtype = game.chosen_subtype(ctx.source).or_else(|| snapshot.and_then(|snapshot| snapshot.chosen_subtype));
        if let Some(subtype) = subtype {
            filter.all_subtypes.push(subtype);
        } else if let Some(card_type) = card_type {
            filter.all_card_types.push(card_type);
        } else {
            return false;
        }
        filter.chosen_creature_type = false;
    }
    if filter.chosen_card_type {
        let Some(card_type) = card_type else { return false; };
        filter.all_card_types.push(card_type);
        filter.chosen_card_type = false;
    }
    true
}

fn lock_target_player_filters_for_player(
    filter: &mut crate::target::ObjectFilter,
    player: PlayerId,
) {
    use crate::effects::player_reference_binding::{
        PlayerReferenceFields, walk_filter_references, walk_player_reference_wrappers,
    };
    walk_filter_references(
        filter,
        PlayerReferenceFields::AllAuthored,
        &mut |reference| {
            walk_player_reference_wrappers(reference, &mut |reference| {
                if matches!(
                    reference,
                    crate::target::PlayerFilter::Target(_)
                        | crate::target::PlayerFilter::AliasedTarget(_)
                ) {
                    *reference = crate::target::PlayerFilter::Specific(player);
                }
            });
            Ok::<_, std::convert::Infallible>(())
        },
        &mut |_| Ok(()),
    )
    .unwrap();
}

/// Replace each `IteratedPlayer` reference in the filter's player facets with
/// the concrete player iterated at resolution time.
fn lock_iterated_player_filters(
    filter: &mut crate::target::ObjectFilter,
    iterated: Option<PlayerId>,
) {
    let Some(iterated) = iterated else {
        return;
    };
    use crate::effects::player_reference_binding::{PlayerReferenceFields, walk_filter_references};
    walk_filter_references(
        filter,
        PlayerReferenceFields::SpellFacets,
        &mut |reference| {
            if matches!(reference, crate::target::PlayerFilter::IteratedPlayer) {
                *reference = crate::target::PlayerFilter::Specific(iterated);
            }
            Ok::<_, std::convert::Infallible>(())
        },
        &mut |_| Ok(()),
    )
    .unwrap();
}

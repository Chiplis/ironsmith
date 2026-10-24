//! Collapse compatible unless branches into one selection without changing action owners.
//!
//! A leading, nonrandom object choice can select a branch when its accepted sets
//! are disjoint. Ambiguous selections keep the explicit branch decision.
//! Searches and aggregate/group constraints
//! keep their native flow until they can supply an equivalent selection contract.
use super::unless_action::{SelectedUnlessBranch, UnlessActionEffect};
use crate::decisions::context::{ObjectSelectionAlternative, SelectionRevealPolicy};
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::Effect;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError, ProgramActionScope};
use crate::filter::{ObjectFilter, ObjectFilterExt};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::zone::Zone;

#[derive(PartialEq)]
enum Family {
    Discard,
    Sacrifice,
    Choose,
}
struct Choice {
    family: Family,
    player: PlayerId,
    rule: ObjectSelectionAlternative,
    filter: ObjectFilter,
    filter_context: crate::filter::FilterContext,
    reveal: SelectionRevealPolicy,
}

fn choice(
    effects: &[Effect],
    game: &GameState,
    ctx: &mut ExecutionContext,
    payment: bool,
) -> Result<Option<Choice>, ExecutionError> {
    let Some(first) = effects.first() else {
        return Ok(None);
    };
    let (family, player, filter, zones, min, max, reveal) =
        if let Some(discard) = first.downcast_ref::<crate::effects::DiscardEffect>() {
            if discard.random || discard.any_number || effects.len() != 1 {
                return Ok(None);
            }
            let player = resolve_player_filter(game, &discard.player, ctx)?;
            let count = resolve_value(game, &discard.count, ctx)?.max(0) as usize;
            let filter = discard.card_filter.clone().unwrap_or_default();
            (
                Family::Discard,
                player,
                filter,
                vec![Zone::Hand],
                count,
                count,
                SelectionRevealPolicy::Public,
            )
        } else if let Some(sacrifice) = first.downcast_ref::<crate::effects::SacrificeEffect>() {
            if effects.len() != 1 {
                return Ok(None);
            }
            let count = resolve_value(game, &sacrifice.count, ctx)?.max(0) as usize;
            (
                Family::Sacrifice,
                resolve_player_filter(game, &sacrifice.player, ctx)?,
                sacrifice.filter.clone(),
                vec![Zone::Battlefield],
                count,
                count,
                SelectionRevealPolicy::Public,
            )
        } else if let Some(choose) = first.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
            if choose.is_search
                || choose.count.is_random()
                || choose.top_only
                || choose.bottom_only
                || choose.aggregate_constraint.is_some()
                || choose.optional_search_zones
                || choose.search_participant_tag.is_some()
                || super::selection_relations::has_relations(&choose.filter)
            {
                return Ok(None);
            }
            let zones = super::choose_objects::search_zones(choose)?;
            // Hidden searches and private choices have their own view/disclosure owners.
            if zones.iter().any(|zone| zone.is_hidden()) {
                return Ok(None);
            }
            let (min, max) = if let Some(value) = &choose.count_value {
                let n = resolve_value(game, value, ctx)?.max(0) as usize;
                (if choose.count.up_to_x { 0 } else { n }, n)
            } else if choose.count.dynamic_x {
                let Some(n) = ctx.x_value else {
                    return Ok(None);
                };
                (
                    if choose.count.up_to_x { 0 } else { n as usize },
                    n as usize,
                )
            } else {
                (choose.count.min, choose.count.max.unwrap_or(usize::MAX))
            };
            (
                Family::Choose,
                resolve_player_filter(game, &choose.chooser, ctx)?,
                choose.filter.clone(),
                zones,
                min,
                max,
                SelectionRevealPolicy::None,
            )
        } else {
            return Ok(None);
        };
    if super::selection_relations::has_relations(&filter) {
        return Ok(None);
    }
    let filter_ctx = ctx.filter_context(game);
    let mut pool: Vec<_> =
        if let Some(choose) = first.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
            super::choose_objects_runtime::collect_candidates(choose, game, ctx, player)?
        } else {
            zones
                .iter()
                .flat_map(|zone| game.objects_in_zone(*zone))
                .collect()
        };
    pool.retain(|id| {
        game.object(*id).is_some_and(|object| {
            !game.is_phased_out(*id)
                && !ctx.replacement.entry_reserved_objects.contains(id)
                && match family {
                    Family::Discard => {
                        object.owner == player
                            && (!payment || object.kind == crate::object::ObjectKind::Card)
                    }
                    Family::Sacrifice => {
                        game.controller_of(object) == player
                            && game.can_be_sacrificed_with_cause(*id, &ctx.cause)
                    }
                    Family::Choose => true,
                }
        })
    });
    // A filtered mandatory hand choice can require a hidden "do as much as
    // possible" proof. Keep that native protocol rather than inventing bounds
    // from identities which differ between peers.
    if !payment
        && family == Family::Discard
        && game.hand_choice_depends_on_hidden_identity(&filter, pool.iter().copied())
    {
        return Ok(None);
    }
    let placeholders = if family == Family::Discard {
        game.hidden_hand_placeholder_candidates(&filter, &filter_ctx, pool.iter().copied())
    } else {
        Vec::new()
    };
    pool.retain(|id| {
        family == Family::Choose
            || placeholders.contains(id)
            || game
                .object(*id)
                .is_some_and(|object| filter.matches(object, &filter_ctx, game))
    });
    if payment && let Some(choose) = first.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        pool.retain(|id| {
            effects[1..].iter().all(|effect| {
                effect.0.as_cost_executable().and_then(|cost| {
                    cost.cost_choice_candidate_is_eligible(
                        game,
                        ctx,
                        crate::costs::PaymentReason::Effect,
                        &choose.tag,
                        *id,
                    )
                }) != Some(false)
            })
        });
    }
    pool.sort();
    pool.dedup();
    let effective_min = if payment { min } else { min.min(pool.len()) };
    let effective_max = max.min(pool.len());
    let verb = match family {
        Family::Discard => "Discard",
        Family::Sacrifice => "Sacrifice",
        Family::Choose => "Choose",
    };
    let singular = min == 1 && max == 1;
    let noun = if filter == ObjectFilter::default() {
        match family {
            Family::Discard => {
                if singular {
                    "card"
                } else {
                    "cards"
                }
            }
            Family::Sacrifice => {
                if singular {
                    "permanent"
                } else {
                    "permanents"
                }
            }
            Family::Choose => {
                if singular {
                    "object"
                } else {
                    "objects"
                }
            }
        }
        .to_string()
    } else {
        let description = filter.description();
        if family == Family::Discard && !description.contains("card") {
            format!("{description} {}", if singular { "card" } else { "cards" })
        } else {
            description
        }
    };
    let amount = if min == max {
        min.to_string()
    } else {
        format!("{min}–{}", max.min(pool.len()))
    };
    Ok(Some(Choice {
        family,
        player,
        filter,
        filter_context: filter_ctx,
        reveal,
        rule: ObjectSelectionAlternative {
            description: format!("{verb} {amount} {noun}"),
            candidates: pool,
            min: effective_min,
            max: effective_max,
        },
    }))
}

pub(super) struct CombinedChoice {
    main: Choice,
    alternative: Choice,
}

pub(super) fn combine(
    effect: &UnlessActionEffect,
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> Result<Option<CombinedChoice>, ExecutionError> {
    if effect.player == crate::target::PlayerFilter::Any {
        return Ok(None);
    }
    let main_is_payment = ctx.targets_are_cost_choices;
    let Some(main) = choice(&effect.effects, game, ctx, main_is_payment)? else {
        return Ok(None);
    };
    let payer = resolve_player_filter(game, &effect.player, ctx)?;
    let payment = crate::costs::PaymentScope::new(ctx, payer, crate::costs::PaymentReason::Effect);
    let Some(mut alternative) =
        payment.run_value(ctx, |ctx| choice(&effect.alternative, game, ctx, true))?
    else {
        return Ok(None);
    };
    if main.player != alternative.player
        || main.player != resolve_player_filter(game, &effect.player, ctx)?
    {
        return Ok(None);
    }
    // Whether to ask a combined question must be identical on every peer.
    // Treat privately known hand identities as potentially matching here, while
    // the actual rules shown to the chooser retain their exact candidates.
    let potential_candidates = |choice: &Choice| {
        let mut ids = choice.rule.candidates.clone();
        if choice.family == Family::Discard
            && let Some(player) = game.player(choice.player)
        {
            for id in &player.hand {
                if game.is_hidden_tracked_hand_card(*id) && !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        ids
    };
    let main_possible = potential_candidates(&main);
    let alternative_possible = potential_candidates(&alternative);
    let overlap_min = main.rule.min.max(alternative.rule.min);
    let overlap_max = main
        .rule
        .max
        .min(alternative.rule.max.max(alternative.rule.min));
    let shared_candidates = main_possible
        .iter()
        .filter(|id| alternative_possible.contains(id))
        .count();
    let overlap = overlap_min <= overlap_max && shared_candidates >= overlap_min;
    if overlap {
        // Even the same physical objects can have different payment, incomplete-
        // instruction, or follow-up semantics. Never guess the player's branch.
        return Ok(None);
    }
    if payment.run_value(ctx, |ctx| {
        effect.alternative_is_infeasible(game, ctx, payer)
    }) {
        alternative.rule.min = 1;
        alternative.rule.max = 0;
    }
    Ok(Some(CombinedChoice { main, alternative }))
}

impl CombinedChoice {
    pub(super) fn select(
        self,
        effect: &UnlessActionEffect,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<SelectedUnlessBranch>, ExecutionError> {
        let rules = vec![self.alternative.rule.clone(), self.main.rule.clone()];
        let mut candidates = rules
            .iter()
            .flat_map(|r| r.candidates.iter().copied())
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.dedup();
        let mut spec = ChooseObjectsSpec::new(
            ctx.source,
            format!("{} or {}", rules[0].description, rules[1].description),
            candidates,
            rules.iter().map(|r| r.min).min().unwrap_or(0),
            rules.iter().map(|r| r.max).max(),
        )
        .require_explicit_choice()
        .with_selection_reveal_policy(self.main.reveal.max(self.alternative.reveal));
        spec.alternatives = rules;
        let selected = crate::decisions::make_decision(
            game,
            ctx.decision_maker,
            self.main.player,
            Some(ctx.source),
            spec,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let accepts = |choice: &Choice| {
            choice.rule.accepts(&selected)
                && selected.iter().all(|id| {
                    game.object(*id).is_some_and(|o| {
                        if choice.family != Family::Discard {
                            true
                        } else if game.is_hidden_card_placeholder(*id) {
                            ctx.prospective_cost_payment
                        } else {
                            choice.filter.matches(o, &choice.filter_context, game)
                        }
                    })
                })
        };
        let alternative = accepts(&self.alternative);
        if !alternative && !accepts(&self.main) {
            return Err(ExecutionError::Impossible(
                "selection does not satisfy either alternative".into(),
            ));
        }
        Ok(Some(SelectedUnlessBranch {
            effects: if alternative {
                effect.alternative.clone()
            } else {
                effect.effects.clone()
            },
            identity: if alternative { vec![1, 0] } else { vec![0] },
            selected: Some(selected),
            targets: None,
            scope: ProgramActionScope {
                payment: alternative.then(|| {
                    crate::costs::PaymentScope::new(
                        ctx,
                        self.main.player,
                        crate::costs::PaymentReason::Effect,
                    )
                }),
                iterated_player: Some(ctx.iteration.iterated_player.or(Some(self.main.player))),
                ..Default::default()
            },
        }))
    }
}

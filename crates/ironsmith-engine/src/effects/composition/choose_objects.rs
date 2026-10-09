//! ChooseObjects effect implementation.
//!
//! This effect allows a player to choose objects matching a filter and tag them
//! for reference by subsequent effects in the same spell/ability.

use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::Comparison;
use crate::filter::ObjectFilterExt as _;
use crate::filter::PlayerFilterExt;
use crate::game_state::GameState;
use crate::zone::Zone;

/// Effect that prompts a player to choose objects matching a filter and tags them.
///
/// This enables patterns like sacrifice costs and interactive selections:
/// - "Sacrifice a creature" → ChooseObjectsEffect + SacrificeEffect
/// - "Choose a creature you control" → ChooseObjectsEffect (for later reference)
///
/// # Fields
///
/// * `filter` - Filter for which objects can be chosen
/// * `count` - Number of objects to choose
/// * `chooser` - Which player makes the choice
/// * `zone` - Optional fallback zone to search when the filter itself is zone-less
/// * `tag` - Tag name to store chosen objects under
/// * `description` - Human-readable description for the UI
/// * `reveal` - Whether chosen cards are revealed before moving them
///
/// # Result
///
/// Returns `crate::effect::OutcomeValue::Objects(chosen_ids)` with the chosen object IDs.
/// If no valid objects exist, returns `crate::effect::OutcomeValue::Count(0)`.
///
/// # Example
///
/// ```ignore
/// // "Sacrifice a creature" as composed effects:
/// vec![
///     Effect::choose_objects(
///         ObjectFilter::creature().you_control(),
///         1,
///         PlayerFilter::You,
///         "sacrificed",
///     ),
///     Effect::sacrifice(ChooseSpec::tagged("sacrificed")),
/// ]
/// ```
pub type ChooseObjectsEffect = ironsmith_core::ChooseObjectsEffect;

pub(crate) fn top_only_selection_limit(
    effect: &ChooseObjectsEffect,
    x_value: Option<u32>,
) -> usize {
    if !effect.top_only && !effect.bottom_only {
        return usize::MAX;
    }
    if effect.count.dynamic_x {
        return x_value
            .and_then(|x| usize::try_from(x).ok())
            .filter(|x| *x > 0)
            .unwrap_or(1);
    }
    effect.count.max.unwrap_or(effect.count.min).max(1)
}

pub(crate) fn search_zones(effect: &ChooseObjectsEffect) -> Result<Vec<Zone>, ExecutionError> {
    let mut zones = Vec::new();
    if let Some(primary_zone) = effect.filter.zone.or(effect.zone) {
        zones.push(primary_zone);
    } else if effect.filter.match_captured_public_destination {
        zones.extend(crate::object_query::PUBLIC_REFERENCE_ZONES);
    } else {
        // A union filter ("a creature or a creature card in your graveyard")
        // carries its zones on the branches; search every branch zone.
        let mut branch_zones = Vec::new();
        let mut zone_less_branch = false;
        for branch in &effect.filter.any_of {
            match branch.zone {
                Some(zone) if !branch_zones.contains(&zone) => branch_zones.push(zone),
                Some(_) => {}
                None => zone_less_branch = true,
            }
        }
        // CR 109.2: an object description that names no zone means a
        // permanent on the battlefield (sacrifice is always a permanent, CR
        // 701.21a). Zone-less union branches fall under the same rule.
        if branch_zones.is_empty() || zone_less_branch {
            zones.push(Zone::Battlefield);
        }
        for zone in branch_zones {
            if !zones.contains(&zone) {
                zones.push(zone);
            }
        }
    }
    for zone in &effect.additional_zones {
        if !zones.contains(zone) {
            zones.push(*zone);
        }
    }
    Ok(zones)
}

fn mana_value_equals_x(filter: &crate::filter::ObjectFilter) -> bool {
    matches!(&filter.mana_value, Some(Comparison::EqualExpr(value))
        if matches!(value.unhinted(), crate::effect::Value::X))
}

fn comparison_references_unbound_x(comparison: &Option<Comparison>) -> bool {
    matches!(
        comparison,
        Some(
            Comparison::EqualExpr(value)
                | Comparison::NotEqualExpr(value)
                | Comparison::LessThanExpr(value)
                | Comparison::LessThanOrEqualExpr(value)
                | Comparison::GreaterThanExpr(value)
                | Comparison::GreaterThanOrEqualExpr(value)
        ) if matches!(value.unhinted(), crate::effect::Value::X)
    )
}

fn relax_unbound_x_comparisons(filter: &mut crate::filter::ObjectFilter) -> bool {
    let mut changed = false;
    for comparison in [
        &mut filter.mana_value,
        &mut filter.power,
        &mut filter.toughness,
    ] {
        if comparison_references_unbound_x(comparison) {
            *comparison = None;
            changed = true;
        }
    }
    for branch in &mut filter.any_of {
        changed |= relax_unbound_x_comparisons(branch);
    }
    changed
}

/// A cost choice such as "exile a red card with mana value X" or "sacrifice
/// a creature with mana value X" is checked for payability before X is
/// announced (CR 601.2b/601.2f). Legality only needs *some* announceable X, so
/// an X-relative characteristic cannot be the reason a choice is unpayable
/// while X is unbound; the real X constrains the selection at payment time.
pub(crate) fn with_unbound_x_relaxed(effect: &ChooseObjectsEffect) -> Option<ChooseObjectsEffect> {
    let mut relaxed = effect.clone();
    relax_unbound_x_comparisons(&mut relaxed.filter).then_some(relaxed)
}

fn cost_candidate_count(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    x_value: Option<u32>,
) -> Result<usize, CostValidationError> {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, controller, &mut dm);
    ctx.x_value = x_value;
    cost_candidate_count_with_context(effect, game, &ctx)
}

fn cost_candidate_count_with_context(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<usize, CostValidationError> {
    if ctx.x_value.is_none()
        && let Some(relaxed) = with_unbound_x_relaxed(effect)
    {
        return cost_candidate_count_with_context(&relaxed, game, ctx);
    }
    let source = ctx.source;
    let controller = ctx.controller;
    let filter_ctx = ctx.filter_context(game);
    let chooser_id =
        match crate::effects::helpers::resolve_player_filter_as_chooser(game, &effect.chooser, ctx)
        {
            Ok(player) => player,
            Err(_) => controller,
        };
    let search_zones =
        search_zones(effect).map_err(|err| CostValidationError::Other(format!("{err:?}")))?;
    let top_only_limit = top_only_selection_limit(effect, ctx.x_value);

    let matches_filter = |obj: &crate::object::Object| {
        if effect.filter.other && obj.id == source {
            return false;
        }
        effect.filter.matches(obj, &filter_ctx, game)
    };

    let hidden_zone_owner_ids =
        |filter: &crate::filter::ObjectFilter| -> Vec<crate::ids::PlayerId> {
            filter.owner.as_ref().map_or_else(
                || vec![chooser_id],
                |owner_filter| {
                    game.players
                        .iter()
                        .map(|player| player.id)
                        .filter(|player_id| owner_filter.matches_player(*player_id, &filter_ctx))
                        .collect()
                },
            )
        };

    let mut total = 0usize;
    for search_zone in search_zones {
        match search_zone {
            Zone::Battlefield => {
                total += game
                    .battlefield
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| matches_filter(obj))
                    .count();
            }
            Zone::Hand => {
                let mut hidden_zone_filter = effect.filter.clone();
                hidden_zone_filter.owner = None;
                let matches_hidden_filter = |obj: &crate::object::Object| {
                    if effect.filter.other && obj.id == source {
                        return false;
                    }
                    hidden_zone_filter.matches(obj, &filter_ctx, game)
                };
                let hand_ids: Vec<crate::ids::ObjectId> = hidden_zone_owner_ids(&effect.filter)
                    .into_iter()
                    .filter_map(|owner_id| game.player(owner_id))
                    .flat_map(|player| player.hand.iter().copied())
                    .collect();
                // Peers holding hidden-card placeholders cannot evaluate the
                // filter; count them as payable so the owner's real choice
                // replays on every peer (see `game_state::hidden_hand_choices`).
                let placeholders = if game.hand_choice_depends_on_hidden_identity(
                    &hidden_zone_filter,
                    hand_ids.iter().copied(),
                ) {
                    game.hidden_hand_placeholder_candidates(
                        &hidden_zone_filter,
                        &filter_ctx,
                        hand_ids.iter().copied(),
                    )
                } else {
                    Vec::new()
                };
                total += hand_ids
                    .iter()
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| {
                        (placeholders.contains(&obj.id)
                            && !(effect.filter.other && obj.id == source))
                            || matches_hidden_filter(obj)
                    })
                    .count();
            }
            Zone::Graveyard => {
                let mut hidden_zone_filter = effect.filter.clone();
                hidden_zone_filter.owner = None;
                let matches_hidden_filter = |obj: &crate::object::Object| {
                    if effect.filter.other && obj.id == source {
                        return false;
                    }
                    hidden_zone_filter.matches(obj, &filter_ctx, game)
                };
                if effect.filter.single_graveyard && effect.filter.owner.is_none() {
                    // "From a single graveyard" may use any player's
                    // graveyard, but the required number must all come from
                    // the same one. Cost preflight therefore needs the
                    // largest eligible owner-group, not either the payer's
                    // graveyard alone or the total across all graveyards.
                    let maximum_in_one_graveyard = game
                        .players
                        .iter()
                        .map(|player| {
                            let matches = player
                                .graveyard
                                .iter()
                                .rev()
                                .filter_map(|&id| game.object(id))
                                .filter(|obj| matches_hidden_filter(obj));
                            if effect.top_only {
                                matches.take(top_only_limit).count()
                            } else {
                                matches.count()
                            }
                        })
                        .max()
                        .unwrap_or(0);
                    total += maximum_in_one_graveyard;
                    continue;
                }
                if effect.top_only {
                    let mut zone_total = 0usize;
                    'owners: for owner_id in hidden_zone_owner_ids(&effect.filter) {
                        let Some(player) = game.player(owner_id) else {
                            continue;
                        };
                        for obj in player
                            .graveyard
                            .iter()
                            .rev()
                            .filter_map(|&id| game.object(id))
                        {
                            if matches_hidden_filter(obj) {
                                zone_total += 1;
                                if zone_total >= top_only_limit {
                                    break 'owners;
                                }
                            }
                        }
                    }
                    total += zone_total;
                } else {
                    total += hidden_zone_owner_ids(&effect.filter)
                        .into_iter()
                        .filter_map(|owner_id| game.player(owner_id))
                        .flat_map(|player| player.graveyard.iter())
                        .filter_map(|&id| game.object(id))
                        .filter(|obj| matches_hidden_filter(obj))
                        .count();
                }
            }
            Zone::Library => {
                let mut hidden_zone_filter = effect.filter.clone();
                hidden_zone_filter.owner = None;
                let matches_hidden_filter = |obj: &crate::object::Object| {
                    if effect.filter.other && obj.id == source {
                        return false;
                    }
                    hidden_zone_filter.matches(obj, &filter_ctx, game)
                };
                if effect.top_only {
                    let mut zone_total = 0usize;
                    'owners: for owner_id in hidden_zone_owner_ids(&effect.filter) {
                        let Some(player) = game.player(owner_id) else {
                            continue;
                        };
                        for obj in player
                            .library
                            .iter()
                            .rev()
                            .filter_map(|&id| game.object(id))
                        {
                            if matches_hidden_filter(obj) {
                                zone_total += 1;
                                if zone_total >= top_only_limit {
                                    break 'owners;
                                }
                            }
                        }
                    }
                    total += zone_total;
                } else {
                    total += hidden_zone_owner_ids(&effect.filter)
                        .into_iter()
                        .filter_map(|owner_id| game.player(owner_id))
                        .flat_map(|player| player.library.iter())
                        .filter_map(|&id| game.object(id))
                        .filter(|obj| matches_hidden_filter(obj))
                        .count();
                }
            }
            Zone::OutsideGame => {
                let mut hidden_zone_filter = effect.filter.clone();
                hidden_zone_filter.owner = None;
                let matches_hidden_filter = |obj: &crate::object::Object| {
                    if effect.filter.other && obj.id == source {
                        return false;
                    }
                    hidden_zone_filter.matches(obj, &filter_ctx, game)
                };
                total += hidden_zone_owner_ids(&effect.filter)
                    .into_iter()
                    .filter_map(|owner_id| game.player(owner_id))
                    .flat_map(|player| player.sideboard.iter())
                    .filter_map(|&id| game.object(id))
                    .filter(|obj| matches_hidden_filter(obj))
                    .count();
            }
            _ => {
                total += game
                    .objects_in_zone(search_zone)
                    .into_iter()
                    .filter_map(|id| game.object(id))
                    .filter(|obj| matches_filter(obj))
                    .count();
            }
        }
    }

    Ok(total)
}

pub(crate) fn check_relation_cost_with_context(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<(), CostValidationError> {
    if !super::selection_relations::has_relations(&effect.filter) {
        return Ok(());
    }
    let chooser =
        crate::effects::helpers::resolve_player_filter_as_chooser(game, &effect.chooser, ctx)
            .map_err(|_| CostValidationError::Other("group chooser is unresolved".into()))?;
    let filter_ctx = ctx.filter_context(game);
    let mut candidates = Vec::new();
    for zone in
        search_zones(effect).map_err(|error| CostValidationError::Other(format!("{error:?}")))?
    {
        let ids: Vec<_> = game
            .objects_in_zone(zone)
            .into_iter()
            .filter(|id| {
                !ctx.replacement.entry_reserved_objects.contains(id)
                    && game.object(*id).is_some_and(|object| {
                        !matches!(zone, Zone::Hand | Zone::Library | Zone::Graveyard)
                            || effect.filter.owner.is_some()
                            || object.owner == chooser
                    })
            })
            .collect();
        let placeholders =
            game.hidden_hand_payable_placeholders(&effect.filter, &filter_ctx, ids.iter().copied());
        for id in ids {
            if !candidates.contains(&id)
                && (placeholders.contains(&id)
                    || game
                        .object(id)
                        .is_some_and(|object| effect.filter.matches(object, &filter_ctx, game)))
            {
                candidates.push(id);
            }
        }
    }
    let required = if effect.count.dynamic_x {
        ctx.x_value.unwrap_or(0) as usize
    } else if let Some(value) = &effect.count_value {
        crate::effects::helpers::resolve_value(game, value, ctx)
            .map_err(|_| CostValidationError::Other("group count is unresolved".into()))?
            .max(0) as usize
    } else {
        effect.count.min
    };
    if effect.count.max.is_some_and(|max| max < required) {
        return Err(CostValidationError::NotEnoughCards);
    }
    super::selection_relations::find_group(game, &effect.filter, &candidates, required, true)
        .map(|_| ())
        .ok_or(CostValidationError::NotEnoughCards)
}

impl EffectExecutor for ChooseObjectsEffect {
    fn cost_choice_bindings(&self) -> crate::effects::CostChoiceBindings {
        crate::effects::CostChoiceBindings {
            required: Vec::new(),
            published: vec![self.tag.clone()],
        }
    }

    fn is_object_selection_prelude(&self) -> bool {
        true
    }

    fn visit_prepared_selection_bindings(
        &self,
        visitor: &mut dyn FnMut(crate::effects::PreparedSelectionBinding),
    ) {
        visitor(crate::effects::PreparedSelectionBinding::ObjectTag(
            self.tag.clone(),
        ));
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn references_cost_x(&self) -> bool {
        self.count.dynamic_x
            || mana_value_equals_x(&self.filter)
            || self
                .aggregate_constraint
                .as_ref()
                .is_some_and(|constraint| {
                    constraint
                        .minimum
                        .as_ref()
                        .is_some_and(|value| matches!(value.unhinted(), crate::effect::Value::X))
                })
    }

    fn max_cost_x(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Option<u32> {
        if !self.references_cost_x() {
            return None;
        }
        if mana_value_equals_x(&self.filter) {
            // Equality gives a finite set of possible X values. Recheck each
            // against the complete cost filter, including hidden-zone ownership.
            let mut values: Vec<_> = search_zones(self)
                .ok()?
                .into_iter()
                .flat_map(|zone| game.objects_in_zone(zone))
                .map(|id| crate::filter::object_current_mana_value(game, id))
                .filter_map(|value| u32::try_from(value).ok())
                .collect();
            values.sort_unstable();
            values.dedup();
            return Some(
                values
                    .into_iter()
                    .rev()
                    .find(|x| {
                        let required = if self.count.dynamic_x {
                            self.count.min.max(*x as usize)
                        } else {
                            self.count.min
                        };
                        cost_candidate_count(self, game, source, controller, Some(*x))
                            .is_ok_and(|count| count >= required)
                    })
                    .unwrap_or(0),
            );
        }
        if self.aggregate_constraint.is_some() {
            return aggregate_cost_capacity(self, game, source, controller)
                .ok()
                .map(|amount| amount.max(0) as u32);
        }
        cost_candidate_count(self, game, source, controller, None)
            .ok()
            .and_then(|count| u32::try_from(count).ok())
    }

    fn supports_prepared_action_program(&self) -> bool {
        !self.is_search && self.reveal && self.reveal_is_presentation_only == Some(false)
    }

    fn select_prepared_action_program(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        super::choose_objects_runtime::prepare_choose_objects_program(self, game, ctx)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::choose_objects_runtime::run_choose_objects(self, game, ctx)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::choose_objects_runtime::run_choose_objects_with_outputs(self, game, ctx)
    }

    /// Pure selection may run choice-by-choice in APNAP order. An authored
    /// reveal also commits public visibility and observations through its child
    /// owner, so it must not advertise the selection-only scheduling shortcut.
    fn is_read_only_simultaneous_player_action(&self) -> bool {
        !self.is_search && (!self.reveal || self.reveal_is_presentation_only == Some(true))
    }

    fn cost_description(&self) -> Option<String> {
        use crate::color::Color;

        let count_str = match (self.count.min, self.count.max) {
            (0, Some(1)) => "up to one".to_string(),
            (0, Some(n)) => format!("up to {}", n),
            (min, Some(max)) if min == max => match min {
                1 => "a".to_string(),
                n => format!("{}", n),
            },
            (min, Some(max)) => format!("{} to {}", min, max),
            (1, None) => "one or more".to_string(),
            (min, None) => format!("{} or more", min),
        };

        let color_desc = if let Some(colors) = &self.filter.colors {
            if colors.count() == 1 {
                let color_name = Color::ALL
                    .iter()
                    .find(|&&c| colors.contains(c))
                    .map(|c| c.name().to_string())
                    .unwrap_or_default();
                if !color_name.is_empty() {
                    format!("{} ", color_name)
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let type_desc = if !self.filter.card_types.is_empty() {
            self.filter
                .card_types
                .iter()
                .map(|t| t.name().to_string())
                .collect::<Vec<_>>()
                .join(" or ")
        } else if !self.filter.subtypes.is_empty() {
            self.filter
                .subtypes
                .iter()
                .map(|s| s.to_string().to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join(" or ")
        } else {
            "card".to_string()
        };

        let zone_desc = match self.filter.zone.or(self.zone) {
            Some(Zone::Hand) => "from your hand",
            Some(Zone::Graveyard)
                if self.filter.single_graveyard && self.filter.owner.is_none() =>
            {
                "from a single graveyard"
            }
            Some(Zone::Graveyard) => "from your graveyard",
            Some(Zone::OutsideGame) => "from outside the game",
            Some(Zone::Battlefield) | None => "",
            _ => "",
        };

        let mana_value_desc = match &self.filter.mana_value {
            Some(Comparison::Equal(value)) => format!(" with mana value {}", value),
            Some(Comparison::LessThan(value)) => format!(" with mana value less than {}", value),
            Some(Comparison::LessThanOrEqual(value)) => {
                format!(" with mana value {} or less", value)
            }
            Some(Comparison::GreaterThan(value)) => {
                format!(" with mana value greater than {}", value)
            }
            Some(Comparison::GreaterThanOrEqual(value)) => {
                format!(" with mana value {} or greater", value)
            }
            Some(Comparison::NotEqual(value)) => {
                format!(" with mana value not equal to {}", value)
            }
            Some(Comparison::OneOf(values)) => {
                let joined = values
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(" with mana value {}", joined)
            }
            Some(Comparison::EqualExpr(_))
            | Some(Comparison::NotEqualExpr(_))
            | Some(Comparison::LessThanExpr(_))
            | Some(Comparison::LessThanOrEqualExpr(_))
            | Some(Comparison::GreaterThanExpr(_))
            | Some(Comparison::GreaterThanOrEqualExpr(_)) => {
                " with a constrained mana value".to_string()
            }
            None => String::new(),
        };

        Some(format!(
            "Exile {} {}{}{}{}",
            count_str,
            color_desc,
            type_desc,
            mana_value_desc,
            if zone_desc.is_empty() {
                String::new()
            } else {
                format!(" {}", zone_desc)
            }
        ))
    }
}

/// Cost preflight uses the same captured value and filter inputs as live
/// selection, including participant tags and the source's last known state.
fn check_selection_cost_with_context(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<(), CostValidationError> {
    if let Some(constraint) = &effect.aggregate_constraint
        && let Some(crate::effect::Value::Fixed(minimum)) =
            constraint.minimum.as_ref().map(|value| value.unhinted())
        && aggregate_cost_capacity_with_context(effect, game, ctx)? < *minimum
    {
        return Err(CostValidationError::NotEnoughCards);
    }
    if effect.count.min == 0 {
        return Ok(());
    }

    check_relation_cost_with_context(effect, game, ctx)?;
    let candidate_count = cost_candidate_count_with_context(effect, game, ctx)?;

    if candidate_count < effect.count.min {
        return Err(CostValidationError::Other(format!(
            "Not enough objects to choose ({} needed, {} available)",
            effect.count.min, candidate_count
        )));
    }

    Ok(())
}

impl CostExecutableEffect for ChooseObjectsEffect {
    fn finalize_payment_bindings(
        &self,
        game: &GameState,
        _outcome: &EffectOutcome,
        execution: &mut ExecutionContext,
        payment_x: Option<u32>,
    ) -> Result<(), crate::cost::CostPaymentError> {
        // A resolving instruction may do as much as possible. A cost selection
        // must supply its entire required input before another component pays it.
        let required = if self.count.up_to_x
            || (self.is_search && self.search_mode == crate::effect::SearchSelectionMode::Optional)
        {
            0
        } else if let Some(value) = self.count_value.as_ref() {
            crate::effects::helpers::resolve_value(game, value, execution)
                .map_err(crate::cost::CostPaymentError::ExecutionFailed)?
                .max(0) as usize
        } else if self.count.dynamic_x {
            payment_x.ok_or_else(|| {
                crate::cost::CostPaymentError::Other("X value not set for cost".into())
            })? as usize
        } else {
            self.count.min
        };
        let selected = execution.tagged_objects.get(&self.tag).map_or(0, Vec::len);
        if selected < required {
            return Err(crate::cost::CostPaymentError::Other(format!(
                "Not enough objects selected to pay cost ({required} needed, {selected} selected)"
            )));
        }
        // Completed zero differs from a selection that never ran. Preserve the
        // binding so subsequent components and replay observe that distinction.
        execution
            .tagged_objects
            .entry(self.tag.clone())
            .or_default();
        Ok(())
    }

    fn can_execute_as_cost_with_context(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
        _reason: crate::costs::PaymentReason,
    ) -> Result<(), CostValidationError> {
        check_selection_cost_with_context(self, game, ctx)
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        let mut ctx = ExecutionContext::new_default(source, controller);
        ctx.x_value = game.object(source).and_then(|object| object.x_value);
        check_selection_cost_with_context(self, game, &ctx)
    }
}

fn aggregate_cost_capacity(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
) -> Result<i32, CostValidationError> {
    let ctx = ExecutionContext::new_default(source, controller);
    aggregate_cost_capacity_with_context(effect, game, &ctx)
}

fn aggregate_cost_capacity_with_context(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<i32, CostValidationError> {
    if ctx.x_value.is_none()
        && let Some(relaxed) = with_unbound_x_relaxed(effect)
    {
        return aggregate_cost_capacity_with_context(&relaxed, game, ctx);
    }
    let constraint = effect
        .aggregate_constraint
        .as_ref()
        .expect("aggregate cost");
    let context = ctx.filter_context(game);
    let mut contributions: Vec<_> = search_zones(effect)
        .map_err(|error| CostValidationError::Other(format!("{error:?}")))?
        .into_iter()
        .flat_map(|zone| game.objects_in_zone(zone))
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| effect.filter.matches(object, &context, game))
        })
        .map(|id| crate::targeting::aggregate_object_value(game, id, constraint.metric))
        .collect();
    if constraint.metric == crate::effect::ChoiceAggregateMetric::DistinctCardTypes {
        let mut states = std::collections::HashMap::from([(0i32, 0usize)]);
        for value in contributions {
            for (mask, count) in states.clone() {
                if count >= effect.count.max.unwrap_or(usize::MAX) {
                    continue;
                }
                let entry = states.entry(mask | value).or_insert(usize::MAX);
                *entry = (*entry).min(count + 1);
            }
        }
        return Ok(states
            .keys()
            .map(|mask| mask.count_ones() as i32)
            .max()
            .unwrap_or(0));
    }
    contributions.sort_unstable_by(|left, right| right.cmp(left));
    Ok(contributions
        .into_iter()
        .take(effect.count.max.unwrap_or(usize::MAX))
        .filter(|amount| *amount > 0)
        .fold(0i32, i32::saturating_add))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::PlayerFilter;
    use crate::test_prelude::*;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_graveyard_creature(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let object = Object::from_card(id, &card, owner, Zone::Graveyard);
        game.add_object(object);
        id
    }

    #[test]
    fn single_graveyard_cost_requires_the_full_count_in_one_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let effect = ChooseObjectsEffect::new(
            crate::filter::ObjectFilter::creature()
                .in_zone(Zone::Graveyard)
                .single_graveyard(),
            2,
            crate::target::PlayerFilter::You,
            "chosen",
        )
        .in_zone(Zone::Graveyard);

        create_graveyard_creature(&mut game, "Alice Creature", alice);
        create_graveyard_creature(&mut game, "Bob Creature A", bob);
        assert!(
            crate::effects::CostExecutableEffect::can_execute_as_cost(
                &effect, &game, source, alice,
            )
            .is_err(),
            "one matching card in each of two graveyards must not satisfy a single-graveyard cost"
        );

        create_graveyard_creature(&mut game, "Bob Creature B", bob);
        crate::effects::CostExecutableEffect::can_execute_as_cost(&effect, &game, source, alice)
            .expect("two matching cards in Bob's graveyard should make the cost payable");
    }

    #[test]
    fn test_choose_objects_no_candidates() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        // No creatures on battlefield
        let effect =
            ChooseObjectsEffect::new(ObjectFilter::creature(), 1, PlayerFilter::You, "selected");
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert!(ctx.get_tagged("selected").is_none());
    }

    #[test]
    fn test_choose_objects_single() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature1 = create_creature(&mut game, "Bear 1", alice);
        let _creature2 = create_creature(&mut game, "Bear 2", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect =
            ChooseObjectsEffect::new(ObjectFilter::creature(), 1, PlayerFilter::You, "selected");
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should have chosen one creature (SelectFirstDecisionMaker picks first)
        if let crate::effect::OutcomeValue::Objects(chosen) = result.value {
            assert_eq!(chosen.len(), 1);
            assert_eq!(chosen[0], creature1);
        } else {
            panic!("Expected Objects result");
        }

        // Should be tagged
        let tagged = ctx.get_tagged("selected");
        assert!(tagged.is_some());
        assert_eq!(tagged.unwrap().name, "Bear 1");
    }

    #[test]
    fn test_choose_objects_filtered() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create creatures for both players
        let _alice_creature = create_creature(&mut game, "Alice Bear", alice);
        let bob_creature = create_creature(&mut game, "Bob Bear", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        // Choose creature you don't control (opponent's)
        let effect = ChooseObjectsEffect::new(
            ObjectFilter::creature().opponent_controls(),
            1,
            PlayerFilter::You,
            "target",
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(chosen) = result.value {
            assert_eq!(chosen.len(), 1);
            assert_eq!(chosen[0], bob_creature);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_choose_objects_zero_count() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let _creature = create_creature(&mut game, "Bear", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect =
            ChooseObjectsEffect::new(ObjectFilter::creature(), 0, PlayerFilter::You, "selected");
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_choose_objects_clone_box() {
        let effect =
            ChooseObjectsEffect::new(ObjectFilter::creature(), 1, PlayerFilter::You, "target");
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ChooseObjectsEffect"));
    }

    #[test]
    fn test_choose_objects_with_zone() {
        let effect =
            ChooseObjectsEffect::new(ObjectFilter::creature(), 1, PlayerFilter::You, "target")
                .in_zone(Zone::Graveyard);

        assert_eq!(effect.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn test_typed_zone_less_choice_uses_battlefield_only() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let permanent = create_creature(&mut game, "Battlefield choice", alice);
        let graveyard_card = create_graveyard_creature(&mut game, "Graveyard exclusion", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ChooseObjectsEffect::new(ObjectFilter::creature(), 1, PlayerFilter::You, "selected");
        let outcome = effect.execute(&mut game, &mut ctx).expect("typed zone-less description chooses a permanent");
        assert_eq!(outcome.objects().unwrap(), &[permanent]);
        assert_eq!(game.object(graveyard_card).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
    }


}

//! Discard effect implementation.

use crate::effect::{EffectOutcome, ExecutionFact, Value};
use crate::effects::helpers::{normalize_object_selection, resolve_player_filter, resolve_value};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::cards::DiscardEvent;
use crate::events::other::CardDiscardedEvent;
use crate::filter::ObjectFilter;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::types::CardType;
use crate::zone::Zone;

/// Effect that causes a player to discard cards.
///
/// Can optionally discard at random.
///
/// # Fields
///
/// * `count` - Number of cards to discard
/// * `player` - The player who discards
/// * `random` - Whether to discard at random
///
/// # Example
///
/// ```ignore
/// // Discard a card
/// let effect = DiscardEffect::you(1);
///
/// // Discard two cards at random
/// let effect = DiscardEffect::random(2, PlayerFilter::You);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DiscardEffect {
    /// Number of cards to discard.
    pub count: Value,
    /// The player who discards.
    pub player: PlayerFilter,
    /// Whether to discard at random.
    pub random: bool,
    /// Whether the player may discard any number of matching cards.
    pub any_number: bool,
    /// Optional hand-card restriction for cards that can be discarded.
    pub card_filter: Option<ObjectFilter>,
    /// Optional tag used to track discarded cards for later clauses such as
    /// "didn't discard a creature card this way".
    pub tag: Option<TagKey>,
}

fn validate_discard_payment_selection(
    game: &GameState,
    selected: &[ObjectId],
    candidates: &[ObjectId],
    min: usize,
    max: usize,
    prospective: bool,
) -> Result<(), ExecutionError> {
    if selected.len() < min
        || selected.len() > max
        || selected
            .iter()
            .enumerate()
            .any(|(index, id)| !candidates.contains(id) || selected[..index].contains(id))
    {
        return Err(ExecutionError::Impossible(
            "discard payment needs the exact legal selection".into(),
        ));
    }
    if !prospective
        && selected
            .iter()
            .any(|id| game.is_hidden_card_placeholder(*id))
    {
        return Err(ExecutionError::IncompleteEvidence(
            "discard payment is awaiting its selected public identity opening".into(),
        ));
    }
    Ok(())
}

impl DiscardEffect {
    /// Create a new discard effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter, random: bool) -> Self {
        Self::new_with_filter(count, player, random, None)
    }

    /// Create a new discard effect with an optional card filter.
    pub fn new_with_filter(
        count: impl Into<Value>,
        player: PlayerFilter,
        random: bool,
        card_filter: Option<ObjectFilter>,
    ) -> Self {
        Self {
            count: count.into(),
            player,
            random,
            any_number: false,
            card_filter,
            tag: None,
        }
    }

    /// Allow the player to choose any number of eligible cards.
    pub fn with_any_number(mut self, any_number: bool) -> Self {
        self.any_number = any_number;
        self
    }

    /// Tag discarded cards for later reference in the same effect sequence.
    pub fn with_tag(mut self, tag: impl Into<TagKey>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// The controller discards N cards (player chooses).
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You, false)
    }

    /// The controller discards N cards at random.
    pub fn you_random(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You, true)
    }

    /// Target player discards N cards at random.
    pub fn random(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self::new(count, player, true)
    }

    /// Target opponent discards N cards.
    pub fn opponent(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::Opponent, false)
    }

    fn mana_value_is_cost_x(&self) -> bool {
        matches!(self.card_filter.as_ref().and_then(|filter| filter.mana_value.as_ref()),
            Some(crate::filter::Comparison::EqualExpr(value)) if matches!(value.unhinted(), Value::X))
    }

    /// Inspect the actual payer's current hand. Only the explicitly announced
    /// mana-value equality may be relaxed before announcement; unknown tags and
    /// every other filter predicate remain binding.
    fn cost_candidates(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
        reason: crate::costs::PaymentReason,
        relax_mana_x: bool,
    ) -> Result<Vec<crate::ids::ObjectId>, crate::effects::CostValidationError> {
        use crate::effects::CostValidationError;
        let player = match self.player {
            PlayerFilter::You => ctx.controller,
            PlayerFilter::Specific(player) => player,
            _ => {
                return Err(CostValidationError::Other(
                    "discard cost needs an explicit payer".into(),
                ));
            }
        };
        let hand = &game
            .player(player)
            .ok_or_else(|| CostValidationError::Other("discard payer is absent".into()))?
            .hand;
        let mut filter = self.card_filter.clone().unwrap_or_default();
        if relax_mana_x && self.mana_value_is_cost_x() {
            filter.mana_value = None;
        }
        let filter_ctx = ctx.filter_context(game);
        let eligible = hand.iter().copied().filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == crate::object::ObjectKind::Card
                    && object.zone == crate::Zone::Hand
                    && object.owner == player
            }) && !ctx.replacement.entry_reserved_objects.contains(id)
                && !(reason == crate::costs::PaymentReason::CastSpell && *id == ctx.source)
        });
        let candidates: Vec<_> = eligible
            .filter(|id| {
                filter.tagged_constraints.iter().all(|constraint| {
                    constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        || ctx
                            .tagged_objects
                            .get(&constraint.tag)
                            .is_some_and(|snapshots| {
                                snapshots.iter().any(|snapshot| snapshot.object_id == *id)
                            })
                })
            })
            .collect();
        let placeholders =
            game.hidden_hand_payable_placeholders(&filter, &filter_ctx, candidates.iter().copied());
        Ok(candidates
            .into_iter()
            .filter(|id| {
                placeholders.contains(id)
                    || game.object(*id).is_some_and(|object| {
                        object.zone == Zone::Hand
                            && object.owner == player
                            && filter.matches(object, &filter_ctx, game)
                    })
            })
            .collect())
    }

    pub(crate) fn check_cost_with_context(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
        reason: crate::costs::PaymentReason,
        allow_unannounced_x: bool,
    ) -> Result<(), crate::effects::CostValidationError> {
        use crate::effects::CostValidationError;
        let unannounced = ctx.x_value.is_none() && self.references_cost_x();
        if unannounced && !allow_unannounced_x {
            return Err(CostValidationError::Other(
                "X was not announced for discard cost".into(),
            ));
        }
        let required = if unannounced && matches!(self.count.unhinted(), Value::X) {
            0
        } else {
            resolve_value(game, &self.count, ctx)
                .map_err(|_| CostValidationError::Other("discard count is unresolved".into()))?
                .max(0) as usize
        };
        let candidates = self.cost_candidates(game, ctx, reason, unannounced)?;
        if candidates.len() < required {
            return Err(CostValidationError::NotEnoughCards);
        }
        if unannounced && self.mana_value_is_cost_x() && required > 1 {
            let mut counts = std::collections::HashMap::new();
            for id in candidates {
                if let Some(object) = game.object(id) {
                    *counts
                        .entry(crate::filter::object_mana_value_for_filter(object))
                        .or_insert(0usize) += 1;
                }
            }
            if !counts.values().any(|count| *count >= required) {
                return Err(CostValidationError::NotEnoughCards);
            }
        }
        Ok(())
    }

    fn discards_source_as_cost(&self) -> bool {
        self.card_filter
            .as_ref()
            .is_some_and(|filter| filter.source && filter.zone == Some(Zone::Hand))
    }
}

fn card_type_name(card_type: CardType) -> &'static str {
    card_type.name()
}

fn format_discard_card_type_phrase(card_types: &[CardType]) -> String {
    if card_types.is_empty() {
        return "card".to_string();
    }
    if card_types.len() == 1 {
        return format!("{} card", card_type_name(card_types[0]));
    }

    let mut parts: Vec<&str> = card_types.iter().map(|ct| card_type_name(*ct)).collect();
    let last = parts.pop().expect("len checked");
    format!("{} or {} card", parts.join(", "), last)
}

fn collect_selected_object_tags(filter: &ObjectFilter, tags: &mut Vec<TagKey>) {
    for constraint in &filter.tagged_constraints {
        if crate::effects::helpers::tagged_relation_names_members(constraint.relation)
            && !tags.contains(&constraint.tag)
        {
            tags.push(constraint.tag.clone());
        }
    }
    for branch in &filter.any_of {
        collect_selected_object_tags(branch, tags);
    }
}

fn selected_object_tags(filter: &ObjectFilter) -> Vec<TagKey> {
    let mut tags = Vec::new();
    collect_selected_object_tags(filter, &mut tags);
    tags.sort();
    tags
}

fn count_filter(value: &Value) -> Option<&ObjectFilter> {
    match value {
        Value::SurfaceHinted { value, .. } => count_filter(value),
        Value::Count(filter) => Some(filter),
        _ => None,
    }
}

fn tracks_same_selected_objects(count: &Value, card_filter: Option<&ObjectFilter>) -> bool {
    let Some(count_filter) = count_filter(count) else {
        return false;
    };
    let Some(card_filter) = card_filter else {
        return false;
    };
    let count_tags = selected_object_tags(count_filter);
    let card_tags = selected_object_tags(card_filter);
    !count_tags.is_empty() && count_tags == card_tags
}

#[derive(Clone)]
struct SelectedDiscardCards {
    player_id: crate::ids::PlayerId,
    cards: Vec<ObjectId>,
}

/// A selected discard instruction retains admission and originals in separate phases.
struct DiscardProposal {
    effect: DiscardEffect,
    selected: Option<Vec<ObjectId>>,
    revealed_by_choice: Vec<ObjectId>,
    selection_ready: bool,
    selection: Option<SelectedDiscardCards>,
    original_ready: bool,
    prepared: Option<PreparedDiscardBatch>,
}

impl DiscardProposal {
    /// Mutable execution selects through the ordinary owner inside its transaction.
    fn native(effect: DiscardEffect) -> Self {
        Self {
            effect,
            selected: None,
            revealed_by_choice: Vec::new(),
            selection_ready: false,
            selection: None,
            original_ready: false,
            prepared: None,
        }
    }
}

impl std::fmt::Debug for DiscardProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscardProposal")
            .field("effect", &self.effect)
            .field("selected", &self.selected)
            .field("selection_ready", &self.selection_ready)
            .field("original_ready", &self.original_ready)
            .finish_non_exhaustive()
    }
}

impl crate::effects::SimultaneousEffectProposal for DiscardProposal {
    fn has_simultaneous_originals(&self) -> bool {
        true
    }

    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.selection_ready || ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        self.selection = if let Some(selected) = &self.selected {
            game.mark_hidden_cards_publicly_revealed(&self.revealed_by_choice);
            let mut effect = self.effect.clone();
            effect.count = Value::Fixed(selected.len() as i32);
            let mut filter = ObjectFilter::default();
            filter.any_of = selected
                .iter()
                .copied()
                .map(ObjectFilter::specific)
                .collect();
            effect.card_filter = Some(filter);
            let previous_targets = std::mem::replace(
                &mut ctx.targets,
                selected
                    .iter()
                    .copied()
                    .map(crate::effects::ResolvedTarget::Object)
                    .collect(),
            );
            let result = effect.select_discard_cards(game, ctx);
            ctx.targets = previous_targets;
            result?
        } else {
            self.effect.select_discard_cards(game, ctx)?
        };
        self.selection_ready = !ctx.decision_maker.awaiting_choice();
        Ok(())
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.original_ready || ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        self.prepare_selection(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        if let Some(selected) = &self.selection {
            self.prepared = prepare_selected_discard_batch_with_retention(
                game,
                ctx,
                selected.player_id,
                selected.cards.clone(),
                self.effect.tag.as_ref(),
                false,
                true,
            )?;
        }
        self.original_ready = !ctx.decision_maker.awaiting_choice();
        Ok(())
    }

    fn commit_original_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.prepare_original(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() || self.prepared.is_none() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let prepared = self.prepared.take().ok_or_else(|| {
            ExecutionError::InternalError("discard original was not retained".into())
        })?;
        commit_selected_discard_original_stream(game, ctx, prepared)
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::complete_prepared_original_with_outputs(self, game, ctx, true)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
}

impl DiscardEffect {
    fn execute_in_simultaneous_batch(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_checkpoint_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| self.execute_in_simultaneous_batch_inner(game, ctx),
        )
    }

    fn execute_in_simultaneous_batch_inner(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::complete_prepared_original_with_outputs(
            Box::new(DiscardProposal::native(self.clone())),
            game,
            ctx,
            true,
        )
    }

    fn select_discard_cards(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<SelectedDiscardCards>, ExecutionError> {
        use crate::decisions::context::SelectionRevealPolicy;
        use crate::decisions::make_decision;
        use crate::decisions::specs::ChooseObjectsSpec;
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let resolved_count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        let one_or_more = self
            .count
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::OneOrMoreChoice);
        let count = if self.any_number && resolved_count == 0 {
            usize::MAX
        } else {
            resolved_count
        };

        let mut hand_cards: Vec<_> = game
            .player(player_id)
            .map(|p| {
                p.hand
                    .iter()
                    .copied()
                    .filter(|id| !ctx.replacement.entry_reserved_objects.contains(id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if ctx.targets_are_cost_choices {
            hand_cards.retain(|id| {
                game.object(*id).is_some_and(|object| {
                    object.kind == crate::object::ObjectKind::Card
                        && object.zone == Zone::Hand
                        && object.owner == player_id
                })
            });
        }
        // A filtered discard from a hand holding hidden cards depends on
        // identities only the owner knows. Peers keep their placeholders
        // choosable and every peer asks the same (never skipped, never
        // auto-filled) question; see `game_state::hidden_hand_choices`.
        let mut hidden_hand_choice = false;
        let mut hidden_filter_ctx = None;
        if let Some(filter) = &self.card_filter {
            let filter_ctx = ctx.filter_context(game);
            hidden_hand_choice = !self.random
                && game.hand_choice_depends_on_hidden_identity(filter, hand_cards.iter().copied());
            let placeholders = if hidden_hand_choice {
                game.hidden_hand_placeholder_candidates(
                    filter,
                    &filter_ctx,
                    hand_cards.iter().copied(),
                )
            } else {
                Vec::new()
            };
            let full_hand = hand_cards.clone();
            hand_cards.retain(|card_id| {
                placeholders.contains(card_id)
                    || game
                        .object(*card_id)
                        .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
            });
            // "Discard a creature card at random": every peer must shuffle the
            // same qualifying cards, so the owner reveals them publicly first.
            if self.random
                && !game.settle_hidden_hand_random_pool(
                    &mut *ctx.decision_maker,
                    ctx.source,
                    filter,
                    &filter_ctx,
                    &full_hand,
                    &mut hand_cards,
                )
            {
                return Ok(None);
            }
            hidden_filter_ctx = Some(filter_ctx);
        }

        // Discarded hidden cards become public, and Madness / discard triggers
        // read their identity as they move. When the player chooses, the choice
        // itself reveals the chosen cards publicly (the peer front end opens
        // them before replaying the answer); otherwise the owner answers a
        // forced reveal below. See `game_state::hidden_hand_choices`.
        let reveal_chosen_publicly = hand_cards
            .iter()
            .any(|id| game.hidden_identity_is_private(*id));

        let required = count.min(hand_cards.len());
        if ctx.targets_are_cost_choices && !self.any_number && required != count {
            return Err(ExecutionError::Impossible(
                "not enough cards for the full discard payment".into(),
            ));
        }
        if required == 0 && !self.any_number && !hidden_hand_choice {
            return Ok(None);
        }

        // Only object targets that are actually cards in this hand select the
        // discard. Unrelated object targets kept in scope from an earlier
        // effect (e.g. Recoil's bounced permanent) must not take away the
        // discarding player's choice (CR 701.9b).
        let explicit_cards: Vec<_> = ctx
            .targets
            .iter()
            .filter_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => Some(*id),
                crate::effects::ResolvedTarget::Player(_) => None,
            })
            .filter(|id| ctx.targets_are_cost_choices || hand_cards.contains(id))
            .collect();

        let cards_to_discard = if !self.random
            && !self.any_number
            && required == hand_cards.len()
            && tracks_same_selected_objects(&self.count, self.card_filter.as_ref())
        {
            // "Discard those cards" consumes the prior tagged selection. It
            // is not a second opportunity to choose from the affected hand,
            // and unrelated object targets in the execution context must not
            // replace the selected set.
            hand_cards.clone()
        } else if !explicit_cards.is_empty() {
            if ctx.targets_are_cost_choices {
                validate_discard_payment_selection(
                    game,
                    &explicit_cards,
                    &hand_cards,
                    required,
                    required,
                    ctx.prospective_cost_payment,
                )?;
                explicit_cards
            } else {
                normalize_object_selection(explicit_cards, &hand_cards, required)
            }
        } else if self.discards_source_as_cost() && hand_cards.contains(&ctx.source) {
            vec![ctx.source]
        } else if self.random {
            game.shuffle_slice(&mut hand_cards);
            hand_cards.into_iter().take(required).collect::<Vec<_>>()
        } else if self.any_number {
            // A positive count paired with `any_number` is an "up to N"
            // choice. A zero count retains the unbounded "any number" shape.
            // Both are optional choices, so neither requires the player to
            // select the maximum number of eligible cards.
            if one_or_more && hand_cards.is_empty() && !hidden_hand_choice {
                return Ok(None);
            }
            let min_required = usize::from(one_or_more && !hidden_hand_choice);
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                if one_or_more {
                    "Choose one or more cards to discard".to_string()
                } else {
                    "Choose any number of cards to discard".to_string()
                },
                hand_cards.clone(),
                min_required,
                Some(required),
            );
            let spec = if hidden_hand_choice {
                if ctx.targets_are_cost_choices {
                    spec.require_explicit_choice()
                } else {
                    spec.allow_partial_completion().require_explicit_choice()
                }
            } else {
                spec
            };
            let spec = if reveal_chosen_publicly {
                spec.with_selection_reveal_policy(SelectionRevealPolicy::Public)
            } else {
                spec
            };
            let spec = if ctx.targets_are_cost_choices {
                spec.with_cost_payment(ctx.source, ctx.controller)
            } else {
                spec
            };
            let chosen: Vec<_> =
                make_decision(game, ctx.decision_maker, player_id, Some(ctx.source), spec);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
            if ctx.targets_are_cost_choices {
                validate_discard_payment_selection(
                    game,
                    &chosen,
                    &hand_cards,
                    min_required,
                    required,
                    ctx.prospective_cost_payment,
                )?;
            }
            if reveal_chosen_publicly {
                // Only offered candidates are opened by the peer front end.
                let opened: Vec<_> = chosen
                    .iter()
                    .copied()
                    .filter(|id| hand_cards.contains(id))
                    .collect();
                game.mark_hidden_cards_publicly_revealed(&opened);
            }
            if min_required > 0 {
                normalize_object_selection(chosen, &hand_cards, min_required)
            } else {
                chosen
                    .into_iter()
                    .filter(|id| hand_cards.contains(id))
                    .fold(Vec::new(), |mut chosen, id| {
                        if !chosen.contains(&id) {
                            chosen.push(id);
                        }
                        chosen
                    })
            }
        } else {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                format!(
                    "Choose {} card{} to discard",
                    required,
                    if required == 1 { "" } else { "s" }
                ),
                hand_cards.clone(),
                required,
                Some(required),
            );
            let spec = if hidden_hand_choice {
                if ctx.targets_are_cost_choices {
                    spec.require_explicit_choice()
                } else {
                    spec.allow_partial_completion().require_explicit_choice()
                }
            } else {
                spec
            };
            let spec = if reveal_chosen_publicly {
                spec.with_selection_reveal_policy(SelectionRevealPolicy::Public)
            } else {
                spec
            };
            let spec = if ctx.targets_are_cost_choices {
                spec.with_cost_payment(ctx.source, ctx.controller)
            } else {
                spec
            };
            let chosen: Vec<_> =
                make_decision(game, ctx.decision_maker, player_id, Some(ctx.source), spec);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
            if ctx.targets_are_cost_choices {
                validate_discard_payment_selection(
                    game,
                    &chosen,
                    &hand_cards,
                    required,
                    required,
                    ctx.prospective_cost_payment,
                )?;
            }
            if reveal_chosen_publicly {
                // Only offered candidates are opened by the peer front end.
                let opened: Vec<_> = chosen
                    .iter()
                    .copied()
                    .filter(|id| hand_cards.contains(id))
                    .collect();
                game.mark_hidden_cards_publicly_revealed(&opened);
            }
            if hidden_hand_choice {
                // No fill-up: it would pick different cards on peers that
                // hold placeholders.
                let mut normalized = Vec::new();
                for id in chosen {
                    if normalized.len() < required
                        && hand_cards.contains(&id)
                        && !normalized.contains(&id)
                    {
                        normalized.push(id);
                    }
                }
                normalized
            } else {
                normalize_object_selection(chosen, &hand_cards, required)
            }
        };
        if hidden_hand_choice
            && let (Some(filter), Some(filter_ctx)) =
                (self.card_filter.as_ref(), hidden_filter_ctx.as_ref())
        {
            game.record_hidden_identity_obligations(
                &cards_to_discard,
                filter,
                filter_ctx,
                "discard a card matching the filter",
            );
        }

        // Random, "those cards" and explicitly selected discards: the owner
        // reveals the still-private cards publicly before any of them moves,
        // so every peer applies Madness (CR 702.35a) and discard triggers to
        // the same identities. A hand card that is the effect's own source was
        // named by the command that activated it and is already public.
        if reveal_chosen_publicly {
            let to_reveal: Vec<_> = cards_to_discard
                .iter()
                .copied()
                .filter(|id| *id != ctx.source)
                .collect();
            let opened = if ctx.targets_are_cost_choices {
                game.reveal_private_hidden_cards_publicly_as_cost(
                    &mut *ctx.decision_maker,
                    player_id,
                    ctx.source,
                    &to_reveal,
                    "Reveal the cards you discard",
                    ctx.prospective_cost_payment,
                )
            } else {
                game.reveal_private_hidden_cards_publicly(
                    &mut *ctx.decision_maker,
                    player_id,
                    ctx.source,
                    &to_reveal,
                    "Reveal the cards you discard",
                    false,
                )
            };
            if opened.is_none() {
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                return Err(ExecutionError::IncompleteEvidence(
                    "discard payment needs the exact opened selection".into(),
                ));
            }
        }

        Ok(Some(SelectedDiscardCards {
            player_id,
            cards: cards_to_discard,
        }))
    }
}

/// Shared commit for selected discards, including whole-hand and prepared
/// simultaneous selections. Selection/reveal validation stays with its owner.
pub(crate) fn discard_selected_cards(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: crate::ids::PlayerId,
    cards_to_discard: Vec<crate::ids::ObjectId>,
    tag: Option<&TagKey>,
    require_arrival: bool,
) -> Result<EffectOutcome, ExecutionError> {
    discard_selected_cards_with_outputs(
        game,
        ctx,
        player_id,
        cards_to_discard,
        tag,
        require_arrival,
    )
    .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// Selected discard inputs and replacement proposals captured before originals commit.
/// Selection, disclosure and payment validation remain with the instruction owner.
pub(crate) struct PreparedDiscardBatch {
    player_id: crate::ids::PlayerId,
    cause: crate::events::EventCause,
    chosen_cards: Vec<ObjectId>,
    chosen_memory: Vec<ObjectSnapshot>,
    tag: Option<TagKey>,
    require_arrival: bool,
    prepared_discards: Vec<(
        ObjectId,
        Option<ObjectSnapshot>,
        Option<ObjectSnapshot>,
        crate::events::processing::PreparedDiscard,
    )>,
}

pub(crate) fn prepare_selected_discard_batch(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: crate::ids::PlayerId,
    cards_to_discard: Vec<ObjectId>,
    tag: Option<&TagKey>,
    require_arrival: bool,
) -> Result<Option<PreparedDiscardBatch>, ExecutionError> {
    prepare_selected_discard_batch_with_retention(
        game,
        ctx,
        player_id,
        cards_to_discard,
        tag,
        require_arrival,
        false,
    )
}

fn prepare_selected_discard_batch_with_retention(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: crate::ids::PlayerId,
    cards_to_discard: Vec<ObjectId>,
    tag: Option<&TagKey>,
    require_arrival: bool,
    retain_original: bool,
) -> Result<Option<PreparedDiscardBatch>, ExecutionError> {
    // Commit the frozen selection using the discard action owner. The cause is inherited from
    // the execution context so discard-as-cost stays cost-caused.
    let cause = ctx.cause.clone();
    let chosen_cards = cards_to_discard.clone();
    let chosen_memory: Vec<_> = chosen_cards
        .iter()
        .filter_map(|id| ObjectSnapshot::from_object_id(game, *id))
        .collect();
    let mut prepared_discards = Vec::new();
    for card_id in cards_to_discard {
        let pre_memory = ObjectSnapshot::from_object_id(game, card_id);
        let pre_discard_snapshot = game
            .object(card_id)
            .map(|obj| ObjectSnapshot::from_object(obj, game));
        let Some(prepared) = crate::events::processing::prepare_discard_with_retention_scope(
            game,
            card_id,
            player_id,
            cause.clone(),
            ctx.provenance,
            &mut *ctx.decision_maker,
            &ctx.replacement,
            ctx.source_snapshot.as_ref(),
            retain_original,
        )?
        else {
            return Ok(None);
        };
        prepared_discards.push((card_id, pre_memory, pre_discard_snapshot, prepared));
    }
    Ok(Some(PreparedDiscardBatch {
        player_id,
        cause,
        chosen_cards,
        chosen_memory,
        tag: tag.cloned(),
        require_arrival,
        prepared_discards,
    }))
}

/// One original discard batch, with its real packets and commit-time Madness arrivals.
pub(crate) struct CommittedDiscardBatch {
    outcome: EffectOutcome,
    receipts: Vec<crate::events::processing::CommittedDiscardOriginal>,
    madness_cards: Vec<ObjectId>,
    results: Vec<(ObjectId, crate::events::processing::DiscardResult)>,
}
impl CommittedDiscardBatch {
    /// Retain originals at the shared freeze/observation boundary. Exact result
    /// readers run here, before any added program can change those arrivals.
    pub(crate) fn prepare_completion_with_outputs(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<ObservedDiscardOriginals>, ExecutionError> {
        let Some(receipt) =
            crate::effects::composition::prepare_standalone_completion_with_outputs(
                game,
                ctx,
                retain_discard_completion(self.outcome, self.receipts),
            )?
        else {
            return Ok(None);
        };
        Ok(Some(ObservedDiscardOriginals {
            receipt,
            madness_cards: self.madness_cards,
            results: self.results,
        }))
    }

    pub(crate) fn complete_with_outputs(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Some(originals) = self.prepare_completion_with_outputs(game, ctx)? else {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        };
        originals.complete_added_programs_with_outputs(game, ctx)
    }
}

/// Real observed original results and their still-unexecuted additions.
/// This phase record owns existing receipts, without reconstructing outputs.
pub(crate) struct ObservedDiscardOriginals {
    receipt: crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    pub(crate) madness_cards: Vec<ObjectId>,
    results: Vec<(ObjectId, crate::events::processing::DiscardResult)>,
}
impl ObservedDiscardOriginals {
    /// Exact requested-card result, independent of counts and added actions.
    pub(crate) fn result_for(
        &self,
        card: ObjectId,
    ) -> Option<&crate::events::processing::DiscardResult> {
        self.results
            .iter()
            .find_map(|(requested, result)| (*requested == card).then_some(result))
    }

    pub(crate) fn complete_added_programs_with_outputs(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::complete_committed_original_with_outputs(
            game,
            ctx,
            self.receipt,
        )
    }
}

/// Project actual completed discard receipts at their own arrival boundary.
/// Physical commitment supplies actual receipts here. Deferred original owners
/// can use this same boundary to capture arrival snapshots and Madness.
struct DiscardBatchProjection {
    player_id: crate::ids::PlayerId,
    cause: crate::events::cause::EventCause,
    chosen_cards: Vec<ObjectId>,
    chosen_memory: Vec<ObjectSnapshot>,
    tag: Option<TagKey>,
    require_arrival: bool,
    discarded: i32,
    discarded_cards: Vec<ObjectId>,
    discarded_snapshots: Vec<ObjectSnapshot>,
    successful_discards: Vec<(ObjectId, Option<ObjectSnapshot>, Zone, Option<ObjectId>)>,
    affected_memory: Vec<ObjectSnapshot>,
    receipts: Vec<crate::events::processing::CommittedDiscardOriginal>,
    madness_cards: Vec<ObjectId>,
    results: Vec<(ObjectId, crate::events::processing::DiscardResult)>,
}
impl DiscardBatchProjection {
    fn retain_completed_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        card_id: ObjectId,
        pre_memory: Option<ObjectSnapshot>,
        pre_discard_snapshot: Option<ObjectSnapshot>,
        original: crate::events::processing::CommittedDiscardOriginal,
    ) -> Result<(), ExecutionError> {
        let crate::events::processing::CommittedDiscardOriginal {
            receipt,
            mut arrival_snapshot,
        } = original;
        self.results.push((card_id, receipt.result.clone()));
        let result = &receipt.result;
        if !result.prevented && (!self.require_arrival || result.new_id.is_some()) {
            if card_id == ctx.source
                && let Some(x) = ctx.x_value
                && let Some(new_id) = result.new_id
                && let Some(obj) = game.object_mut(new_id)
            {
                // Preserve the chosen X on "discard this card" costs so
                // "when you cycle this card" triggers in the graveyard can
                // still evaluate references like "mana value equal to X".
                obj.x_value = Some(x);
                if let Some(snapshot) = arrival_snapshot.as_mut()
                    && snapshot.object_id == new_id
                {
                    snapshot.x_value = Some(x);
                }
            }
            if let Some(event) = &receipt.resolved_event {
                if event.player != self.player_id
                    || event.card != card_id
                    || event.cause != self.cause
                {
                    return Err(ExecutionError::InternalError(
                        "discard receipt changed an unsupported batch identity".into(),
                    ));
                }
            } else {
                return Err(ExecutionError::InternalError(
                    "completed discard has no resolved event".into(),
                ));
            }
            self.discarded = self
                .discarded
                .checked_add(1)
                .ok_or_else(|| ExecutionError::InternalError("discard count overflow".into()))?;
            self.discarded_cards.push(card_id);
            if let Some(memory) = pre_memory {
                self.affected_memory.push(memory);
            }
            self.successful_discards.push((
                card_id,
                pre_discard_snapshot,
                result.final_zone,
                result.new_id,
            ));
            let snapshot = arrival_snapshot.clone().or_else(|| {
                game.object(result.new_id.unwrap_or(card_id))
                    .map(|object| ObjectSnapshot::from_object(object, game))
            });
            if let Some(snapshot) = snapshot {
                self.discarded_snapshots.push(snapshot);
            }
        }
        if let Some(id) = receipt.result.new_id
            && game.is_madness_exiled(id)
        {
            self.madness_cards.push(id);
        }
        self.receipts
            .push(crate::events::processing::CommittedDiscardOriginal {
                receipt,
                arrival_snapshot,
            });
        Ok(())
    }

    fn finish(self, game: &mut GameState, ctx: &mut ExecutionContext) -> CommittedDiscardBatch {
        let Self {
            player_id,
            cause,
            chosen_cards,
            chosen_memory,
            tag,
            require_arrival: _,
            discarded,
            discarded_cards,
            discarded_snapshots,
            successful_discards,
            affected_memory,
            receipts,
            madness_cards,
            results,
        } = self;
        let discard_events =
            completed_discard_events(game, player_id, cause, ctx.provenance, successful_discards);

        if let Some(tag) = tag.as_ref()
            && !discarded_snapshots.is_empty()
        {
            ctx.tag_objects(tag.clone(), discarded_snapshots);
        }

        let mut outcome = EffectOutcome::count(discarded)
            .with_events(discard_events)
            .with_execution_fact(ExecutionFact::ChosenObjects(chosen_cards))
            .with_chosen_object_memory(chosen_memory);
        if !discarded_cards.is_empty() {
            outcome = outcome.with_execution_fact(ExecutionFact::AffectedObjects(discarded_cards));
            outcome = outcome.with_affected_object_memory(affected_memory);
        }

        CommittedDiscardBatch {
            outcome,
            receipts,
            madness_cards,
            results,
        }
    }
}

type SelectedDiscardOriginal = (
    ObjectId,
    Option<ObjectSnapshot>,
    Option<ObjectSnapshot>,
    crate::events::processing::PreparedDiscard,
);

struct PendingDiscardOriginal {
    card_id: ObjectId,
    pre_memory: Option<ObjectSnapshot>,
    pre_discard_snapshot: Option<ObjectSnapshot>,
    original: crate::events::processing::CommittedDiscardOriginal,
    program: crate::events::processing::RetainedDiscardOriginal,
}

/// The native authored discard stream, not a cohort of internal replacements.
/// Stop at the selected subtree's draw before committing later physical siblings.
struct DiscardOriginalStream {
    projection: DiscardBatchProjection,
    remaining: std::vec::IntoIter<SelectedDiscardOriginal>,
    pending: Option<PendingDiscardOriginal>,
    pending_frozen: bool,
    original_frame: Option<crate::game_state::CapturedSimultaneousOriginalFrame>,
}

enum DiscardOriginalStreamCommit {
    Finished(CommittedDiscardBatch),
    Retained(DiscardOriginalStream),
}

impl DiscardOriginalStream {
    fn new(prepared: PreparedDiscardBatch) -> Self {
        let PreparedDiscardBatch {
            player_id,
            cause,
            chosen_cards,
            chosen_memory,
            tag,
            require_arrival,
            prepared_discards,
        } = prepared;
        let projection = DiscardBatchProjection {
            player_id,
            cause,
            chosen_cards,
            chosen_memory,
            tag,
            require_arrival,
            discarded: 0,
            discarded_cards: Vec::new(),
            discarded_snapshots: Vec::new(),
            successful_discards: Vec::new(),
            affected_memory: Vec::new(),
            receipts: Vec::new(),
            madness_cards: Vec::new(),
            results: Vec::new(),
        };
        Self {
            projection,
            remaining: prepared_discards.into_iter(),
            pending: None,
            pending_frozen: false,
            original_frame: None,
        }
    }

    fn commit_remaining(
        mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        retain_original: bool,
    ) -> Result<DiscardOriginalStreamCommit, ExecutionError> {
        if self.pending.is_some() {
            return Err(ExecutionError::InternalError(
                "discard stream advanced past its retained original".into(),
            ));
        }
        while let Some((card_id, pre_memory, pre_discard_snapshot, prepared)) =
            self.remaining.next()
        {
            let committed = if retain_original {
                crate::events::processing::commit_prepared_discard_original(
                    game,
                    prepared,
                    &mut *ctx.decision_maker,
                )?
            } else {
                crate::events::processing::DiscardOriginalCommit {
                    original: crate::events::processing::commit_prepared_discard(
                        game,
                        prepared,
                        &mut *ctx.decision_maker,
                    )?,
                    program: None,
                }
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok(DiscardOriginalStreamCommit::Finished(
                    CommittedDiscardBatch {
                        outcome: EffectOutcome::count(0),
                        receipts: Vec::new(),
                        madness_cards: Vec::new(),
                        results: Vec::new(),
                    },
                ));
            }
            if let Some(program) = committed.program {
                self.pending = Some(PendingDiscardOriginal {
                    card_id,
                    pre_memory,
                    pre_discard_snapshot,
                    original: committed.original,
                    program,
                });
                self.pending_frozen = false;
                if self.original_frame.is_none() {
                    self.original_frame = Some(game.capture_simultaneous_original_frame());
                }
                return Ok(DiscardOriginalStreamCommit::Retained(self));
            }
            // Source X, exact arrival and Madness are projected immediately,
            // before any later sibling or selected program can change the object.
            self.projection.retain_completed_original(
                game,
                ctx,
                card_id,
                pre_memory,
                pre_discard_snapshot,
                committed.original,
            )?;
        }
        Ok(DiscardOriginalStreamCommit::Finished(
            self.projection.finish(game, ctx),
        ))
    }

    /// Prefix views do not allocate completed discard identities or publish tags.
    /// Those belong to the one final batch projection after the stream finishes.
    fn prefix_outputs(&self) -> crate::effects::CompletedEffectOutputs {
        let primary = crate::effect::OutcomeValue::Count(i64::from(self.projection.discarded));
        let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(self.projection.discarded)
                .with_execution_fact(ExecutionFact::ChosenObjects(
                    self.projection.chosen_cards.clone(),
                ))
                .with_chosen_object_memory(self.projection.chosen_memory.clone()),
        );
        outputs = outputs.append_replacement_outputs(
            self.projection
                .receipts
                .iter()
                .filter_map(|original| {
                    original
                        .receipt
                        .payload_outcome
                        .as_ref()
                        .map(crate::effects::CompletedEffectOutputs::clone_projection)
                })
                .chain(
                    self.pending
                        .iter()
                        .map(|pending| pending.program.outcome().clone_projection()),
                ),
        );
        outputs.outcome.value = primary;
        outputs
    }

    fn inherit_observations(&mut self, original: &EffectOutcome) {
        for payload in self
            .projection
            .receipts
            .iter_mut()
            .filter_map(|original| original.receipt.payload_outcome.as_mut())
            .chain(
                self.pending
                    .iter_mut()
                    .map(|pending| pending.program.parts().0),
            )
        {
            crate::effects::composition::inherit_original_observations(
                &mut payload.outcome,
                &original.events,
            );
            payload.synchronize_observations();
        }
    }
}

/// Finished compatibility callers compose the same native stream atomically.
pub(crate) fn commit_selected_discard_batch(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    prepared: PreparedDiscardBatch,
) -> Result<CommittedDiscardBatch, ExecutionError> {
    match DiscardOriginalStream::new(prepared).commit_remaining(game, ctx, false)? {
        DiscardOriginalStreamCommit::Finished(batch) => Ok(batch),
        DiscardOriginalStreamCommit::Retained(_) => Err(ExecutionError::InternalError(
            "atomic discard batch retained an original stream".into(),
        )),
    }
}

/// Prepared instructions retain the actual original stream at its first draw.
fn commit_selected_discard_original_stream(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    prepared: PreparedDiscardBatch,
) -> Result<
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    ExecutionError,
> {
    discard_original_stream_receipt(
        DiscardOriginalStream::new(prepared).commit_remaining(game, ctx, true)?,
    )
}

fn discard_original_stream_receipt(
    committed: DiscardOriginalStreamCommit,
) -> Result<
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    ExecutionError,
> {
    Ok(match committed {
        DiscardOriginalStreamCommit::Finished(batch) => {
            retain_discard_completion(batch.outcome, batch.receipts)
        }
        DiscardOriginalStreamCommit::Retained(stream) => crate::effects::SimultaneousEffectCommit {
            outcome: stream.prefix_outputs(),
            completion: Some(Box::new(stream)),
        },
    })
}

impl crate::effects::SimultaneousEffectCompletion for DiscardOriginalStream {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        // This cursor owns selected original instructions. Enclosing additions
        // remain on exact discard records until the batch projection finishes.
        crate::effects::OriginalPhaseStatus::Retained
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        if !self.pending_frozen {
            let pending = self.pending.as_mut().ok_or_else(|| {
                ExecutionError::InternalError(
                    "retained discard stream lost its selected original".into(),
                )
            })?;
            let (outputs, completion) = pending.program.parts();
            game.freeze_completed_entry_events(outputs.outcome.events.iter_mut())?;
            if let Some(completion) = completion {
                completion.freeze(game)?;
            }
            self.pending_frozen = true;
        }
        Ok(())
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        self.inherit_observations(original);
        if let Some(pending) = &mut self.pending {
            let (outputs, completion) = pending.program.parts();
            if let Some(completion) = completion {
                crate::effects::composition::observe_original_completion(
                    game,
                    ctx,
                    completion,
                    &mut outputs.outcome,
                )?;
            }
            crate::effects::composition::inherit_original_observations(
                original,
                &outputs.outcome.events,
            );
            outputs.synchronize_observations();
        }
        Ok(())
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.complete_original_phase_from_outputs(
            game,
            ctx,
            crate::effects::CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn complete_original_phase_from_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.inherit_observations(&original.outcome);
        let frame = self.original_frame.clone().ok_or_else(|| {
            ExecutionError::InternalError(
                "retained discard stream lost its original grouping frame".into(),
            )
        })?;
        let (mut receipt, observations) = crate::effects::with_action_observations(game, |game| {
            game.with_simultaneous_original_frame(&frame, |game| {
                // Complete each selected subtree in native authored order before
                // later siblings. New pauses are frozen/observed at their own world.
                loop {
                    let mut pending = self.pending.take().ok_or_else(|| {
                        ExecutionError::InternalError(
                            "discard original completion lost its selected program".into(),
                        )
                    })?;
                    if !self.pending_frozen && !pending.program.prepare(game, ctx)? {
                        return Ok(crate::effects::SimultaneousEffectCommit::finished(
                            crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ),
                        ));
                    }
                    let Some(original) = pending.program.complete(game, ctx, pending.original)?
                    else {
                        return Ok(crate::effects::SimultaneousEffectCommit::finished(
                            crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ),
                        ));
                    };
                    self.projection.retain_completed_original(
                        game,
                        ctx,
                        pending.card_id,
                        pending.pre_memory,
                        pending.pre_discard_snapshot,
                        original,
                    )?;
                    match (*self).commit_remaining(game, ctx, true)? {
                        DiscardOriginalStreamCommit::Finished(batch) => {
                            return Ok(retain_discard_completion(batch.outcome, batch.receipts));
                        }
                        DiscardOriginalStreamCommit::Retained(stream) => self = Box::new(stream),
                    }
                }
            })
        })?;
        crate::effects::composition::original_observations::retain_original_observations(
            std::iter::once(&mut receipt),
            observations,
        );
        if !ctx.decision_maker.awaiting_choice() {
            receipt.outcome.retain_owned_child(original);
        }
        Ok(receipt)
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.prepare_draw_boundary_from_outputs(
            game,
            ctx,
            crate::effects::CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        // Producer already paused at the first selected subtree's actual draw.
        Ok(crate::effects::SimultaneousEffectCommit {
            outcome: original,
            completion: Some(self),
        })
    }

    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.complete_from_original_outputs(
            game,
            ctx,
            crate::effects::CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let receipt = self.complete_original_phase_from_outputs(game, ctx, original)?;
        crate::effects::composition::complete_standalone_original_with_outputs(game, ctx, receipt)
    }

    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
}

pub(crate) fn discard_selected_cards_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: crate::ids::PlayerId,
    cards_to_discard: Vec<ObjectId>,
    tag: Option<&TagKey>,
    require_arrival: bool,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let Some(prepared) = prepare_selected_discard_batch(
        game,
        ctx,
        player_id,
        cards_to_discard,
        tag,
        require_arrival,
    )?
    else {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    };
    let committed = commit_selected_discard_batch(game, ctx, prepared)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    committed.complete_with_outputs(game, ctx)
}

impl EffectExecutor for DiscardEffect {
    fn supports_replacement_draw_continuation(&self) -> bool {
        true
    }

    fn prepare_replacement_draw_continuation_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        crate::effects::replacement::prepare_native_proposal_draw_continuation_with_outputs(
            self.result_action(),
            game,
            ctx,
            |_game, _ctx| Ok(Box::new(DiscardProposal::native(self.clone()))),
        )
    }

    fn cost_choice_bindings(&self) -> crate::effects::CostChoiceBindings {
        self.card_filter
            .as_ref()
            .map(crate::effects::CostChoiceBindings::from_filter)
            .unwrap_or_default()
    }

    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Discarded)
    }
    fn supports_simultaneous_player_action(&self) -> bool {
        !self.random && !self.any_number && self.card_filter.is_none()
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        use crate::decisions::{make_decision, specs::ChooseObjectsSpec};
        if !self.supports_simultaneous_player_action() {
            return Err(ExecutionError::Impossible(
                "discard shape lacks simultaneous preparation".into(),
            ));
        }
        let player = resolve_player_filter(game, &self.player, ctx)?;
        let hand = game
            .player(player)
            .map(|player| player.hand.to_vec())
            .unwrap_or_default();
        let count = (resolve_value(game, &self.count, ctx)?.max(0) as usize).min(hand.len());
        let explicit = ctx
            .targets
            .iter()
            .filter_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => Some(*id),
                _ => None,
            })
            .collect::<Vec<_>>();
        let reveal_chosen_publicly = hand.iter().any(|id| game.hidden_identity_is_private(*id));
        let mut revealed_by_choice = Vec::new();
        let selected = if count == 0 {
            Vec::new()
        } else if !explicit.is_empty() {
            normalize_object_selection(explicit, &hand, count)
        } else {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                format!(
                    "Choose {} card{} to discard",
                    count,
                    if count == 1 { "" } else { "s" }
                ),
                hand.clone(),
                count,
                Some(count),
            );
            // Discarded hidden cards are opened publicly before the answer is
            // replayed (Madness, discard triggers); see `hidden_hand_choices`.
            let spec = if reveal_chosen_publicly {
                spec.with_selection_reveal_policy(
                    crate::decisions::context::SelectionRevealPolicy::Public,
                )
            } else {
                spec
            };
            let chosen = make_decision(game, ctx.decision_maker, player, Some(ctx.source), spec);
            if reveal_chosen_publicly {
                revealed_by_choice = chosen
                    .iter()
                    .copied()
                    .filter(|id| hand.contains(id))
                    .collect();
            }
            if ctx.decision_maker.awaiting_choice() {
                Vec::new()
            } else {
                normalize_object_selection(chosen, &hand, count)
            }
        };
        let mut effect = self.clone();
        effect.player = PlayerFilter::Specific(player);
        Ok(Box::new(DiscardProposal {
            effect,
            selected: Some(selected),
            revealed_by_choice,
            selection_ready: false,
            selection: None,
            original_ready: false,
            prepared: None,
        }))
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        // CR 701.9a / 603.2c: the cards one instruction discards are discarded
        // at the same time, as one event.
        let opened_batch = game.open_simultaneous_action();
        let outcome = self.execute_in_simultaneous_batch(game, ctx);
        game.close_simultaneous_action(opened_batch);
        outcome
    }

    fn references_cost_x(&self) -> bool {
        matches!(self.count.unhinted(), Value::X) || self.mana_value_is_cost_x()
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
        let ctx = ExecutionContext::new_default(source, controller);
        let candidates = self
            .cost_candidates(
                game,
                &ctx,
                crate::costs::PaymentReason::ActivateAbility,
                true,
            )
            .ok()?;
        if self.mana_value_is_cost_x() {
            let mut counts = std::collections::HashMap::new();
            for id in candidates {
                let amount =
                    crate::filter::object_mana_value_for_filter(game.object(id)?).max(0) as u32;
                *counts.entry(amount).or_insert(0usize) += 1;
            }
            let count_is_x = matches!(self.count.unhinted(), Value::X);
            let fixed = if count_is_x {
                0
            } else {
                resolve_value(game, &self.count, &ctx).ok()?.max(0) as usize
            };
            return Some(
                counts
                    .into_iter()
                    .filter(|(amount, count)| {
                        *count >= if count_is_x { *amount as usize } else { fixed }
                    })
                    .map(|(amount, _)| amount)
                    .max()
                    .unwrap_or(0),
            );
        }
        u32::try_from(candidates.len()).ok()
    }

    fn cost_description(&self) -> Option<String> {
        if self.discards_source_as_cost() {
            return Some("Discard this card".to_string());
        }

        if self.any_number {
            return None;
        }

        let count = match self.count {
            Value::Fixed(n) if n > 0 => n as usize,
            _ => return None,
        };
        if let Some(filter) = &self.card_filter {
            let mut unrendered = filter.clone();
            unrendered.zone = None;
            unrendered.card_types.clear();
            unrendered.subtypes.clear();
            if unrendered != ObjectFilter::default() || filter.subtypes.len() > 1 {
                // The full typed renderer retains colors, historic, mana-value,
                // and linked identity predicates; never label these a plain card.
                return None;
            }
        }
        let card_types = self
            .card_filter
            .as_ref()
            .map(|f| f.card_types.clone())
            .unwrap_or_default();
        let mut type_phrase = format_discard_card_type_phrase(&card_types);
        if let Some(subtype) = self
            .card_filter
            .as_ref()
            .and_then(|f| f.subtypes.first().copied())
        {
            type_phrase = format!("{} {type_phrase}", subtype.display_name());
        }
        let random_suffix = if self.random { " at random" } else { "" };
        Some(if count == 1 {
            format!("Discard a {type_phrase}{random_suffix}")
        } else {
            format!("Discard {count} {type_phrase}s{random_suffix}")
        })
    }
}

impl CostExecutableEffect for DiscardEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        CostExecutableEffect::can_execute_as_cost_with_reason(
            self,
            game,
            source,
            controller,
            crate::costs::PaymentReason::Other,
        )
    }

    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), crate::effects::CostValidationError> {
        let ctx = ExecutionContext::new_default(source, controller);
        self.check_cost_with_context(game, &ctx, reason, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, CardBuilder};
    use crate::effect::ExecutionFact;
    use crate::events::cards::DiscardEvent;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_spell_card(card_id: u32, name: &str) -> Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Instant])
            .build()
    }

    fn add_card_to_hand(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_spell_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Hand);
        game.add_object(obj); // add_object automatically updates player.hand for Zone::Hand
        id
    }

    fn add_card_to_hand_with_mana_value(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        mana_value: u8,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
                mana_value,
            )]]))
            .card_types(vec![CardType::Instant])
            .build();
        game.add_object(Object::from_card(id, &card, owner, Zone::Hand));
        id
    }

    #[test]
    fn wrapped_discard_cost_preserves_payment_reason_and_excludes_cast_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = add_card_to_hand(&mut game, "Source", alice);
        let effect = crate::effect::Effect::new(crate::effects::WithIdEffect::new(
            crate::effect::EffectId(0),
            crate::effect::Effect::new(DiscardEffect::you(1)),
        ));
        assert!(matches!(
            effect.0.can_execute_as_cost_with_reason(
                &game,
                source,
                alice,
                crate::costs::PaymentReason::CastSpell,
            ),
            Err(crate::effects::CostValidationError::NotEnoughCards)
        ));
        assert!(
            effect
                .0
                .can_execute_as_cost_with_reason(
                    &game,
                    source,
                    alice,
                    crate::costs::PaymentReason::Other,
                )
                .is_ok(),
            "noncasting costs can discard their source"
        );
        add_card_to_hand(&mut game, "Other card", alice);
        assert!(
            effect
                .0
                .can_execute_as_cost_with_reason(
                    &game,
                    source,
                    alice,
                    crate::costs::PaymentReason::CastSpell,
                )
                .is_ok()
        );
    }

    #[test]
    fn test_discard_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_card_to_hand(&mut game, "Card 1", alice);
        add_card_to_hand(&mut game, "Card 2", alice);
        add_card_to_hand(&mut game, "Card 3", alice);

        assert_eq!(game.player(alice).unwrap().hand.len(), 3);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DiscardEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    }

    #[test]
    fn test_discard_more_than_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_card_to_hand(&mut game, "Card 1", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DiscardEffect::you(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only discarded 1 card (all that was in hand)
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert!(game.player(alice).unwrap().hand.is_empty());
    }

    #[test]
    fn test_discard_empty_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = DiscardEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_discard_variable_amount() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        add_card_to_hand(&mut game, "Card 1", alice);
        add_card_to_hand(&mut game, "Card 2", alice);
        add_card_to_hand(&mut game, "Card 3", alice);
        add_card_to_hand(&mut game, "Card 4", alice);

        let mut ctx = ExecutionContext::new_default(source, alice).with_x(2);
        let effect = DiscardEffect::new(Value::X, PlayerFilter::You, false);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
    }

    #[test]
    fn one_or_more_discard_requires_a_nonempty_choice_when_cards_are_available() {
        #[derive(Default)]
        struct SelectNone;

        impl crate::decision::DecisionMaker for SelectNone {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                _ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                Vec::new()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        add_card_to_hand(&mut game, "Card 1", alice);
        add_card_to_hand(&mut game, "Card 2", alice);

        let mut decisions = SelectNone;
        let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
        let effect = DiscardEffect::new_with_filter(
            Value::Fixed(0).with_surface_hint(ironsmith_core::ValueSurfaceHint::OneOrMoreChoice),
            PlayerFilter::You,
            false,
            None,
        )
        .with_any_number(true);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    }

    #[test]
    fn test_discard_clone_box() {
        let effect = DiscardEffect::you(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("DiscardEffect"));
    }

    #[test]
    fn test_discard_can_execute_as_cost_requires_enough_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let effect = DiscardEffect::you_random(1);
        let can_pay =
            crate::effects::EffectExecutor::can_execute_as_cost(&effect, &game, source, alice);
        assert_eq!(
            can_pay,
            Err(crate::effects::CostValidationError::NotEnoughCards)
        );

        add_card_to_hand(&mut game, "Card 1", alice);
        let can_pay =
            crate::effects::EffectExecutor::can_execute_as_cost(&effect, &game, source, alice);
        assert!(can_pay.is_ok(), "expected discard cost to be payable");
    }

    #[test]
    fn test_discard_cost_description_random() {
        let effect = DiscardEffect::you_random(1);
        assert_eq!(
            effect.cost_description().as_deref(),
            Some("Discard a card at random")
        );
    }

    #[test]
    fn test_discard_source_cost_description_uses_generic_effect() {
        let effect = DiscardEffect::new_with_filter(
            1,
            PlayerFilter::You,
            false,
            Some(crate::filter::ObjectFilter::source().in_zone(Zone::Hand)),
        );
        assert_eq!(
            effect.cost_description().as_deref(),
            Some("Discard this card")
        );
    }

    #[test]
    fn test_discard_effect_cost_validation_respects_source_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = add_card_to_hand(&mut game, "Source", alice);
        add_card_to_hand(&mut game, "Other", alice);

        let discard_other = DiscardEffect::new_with_filter(
            1,
            PlayerFilter::You,
            false,
            Some(
                crate::filter::ObjectFilter::default()
                    .in_zone(Zone::Hand)
                    .other(),
            ),
        );
        assert!(
            crate::effects::EffectExecutor::can_execute_as_cost(
                &discard_other,
                &game,
                source,
                alice,
            )
            .is_ok()
        );

        let discard_source = DiscardEffect::new_with_filter(
            1,
            PlayerFilter::You,
            false,
            Some(crate::filter::ObjectFilter::source().in_zone(Zone::Hand)),
        );
        assert!(
            crate::effects::EffectExecutor::can_execute_as_cost(
                &discard_source,
                &game,
                source,
                alice,
            )
            .is_ok()
        );

        let effect = DiscardEffect::new_with_filter(
            1,
            PlayerFilter::You,
            false,
            Some(crate::filter::ObjectFilter::source().in_zone(Zone::Hand)),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert!(!game.player(alice).unwrap().hand.contains(&source));
    }

    #[test]
    fn test_discard_emits_events_and_object_facts() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = add_card_to_hand(&mut game, "Card 1", alice);
        let second = add_card_to_hand(&mut game, "Card 2", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![
            crate::effects::ResolvedTarget::Object(first),
            crate::effects::ResolvedTarget::Object(second),
        ];

        let effect = DiscardEffect::you(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert!(
            result
                .execution_facts()
                .contains(&ExecutionFact::ChosenObjects(vec![first, second]))
        );
        assert!(
            result
                .execution_facts()
                .contains(&ExecutionFact::AffectedObjects(vec![first, second]))
        );
        assert_eq!(result.events.len(), 4);
        assert_eq!(
            result.events[0]
                .downcast::<DiscardEvent>()
                .expect("discard event")
                .player,
            alice
        );
        assert_eq!(
            result.events[1]
                .downcast::<CardDiscardedEvent>()
                .expect("card discarded event")
                .player,
            alice
        );
    }

    #[test]
    fn test_discarding_source_as_cost_preserves_x_on_moved_object() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source_id = add_card_to_hand(&mut game, "Cycling Test", alice);
        let stable_id = game
            .object(source_id)
            .expect("source card should exist in hand")
            .stable_id;

        let effect = DiscardEffect::new_with_filter(
            1,
            PlayerFilter::You,
            false,
            Some(crate::filter::ObjectFilter::source().in_zone(Zone::Hand)),
        );
        let mut ctx = ExecutionContext::new_default(source_id, alice).with_x(3);

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("discarding the source card as a cost should succeed");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));

        let moved_id = game
            .find_object_by_stable_id(stable_id)
            .expect("discarded source card should still be reachable by stable id");
        let moved_obj = game
            .object(moved_id)
            .expect("discarded source card should still exist");
        assert_eq!(moved_obj.zone, Zone::Graveyard);
        assert_eq!(
            moved_obj.x_value,
            Some(3),
            "discarding the source card as part of an X cost should preserve the chosen X on the new object"
        );
    }

    fn tagged_hand_filter(tag: &str) -> ObjectFilter {
        ObjectFilter::tagged(TagKey::from(tag)).in_zone(Zone::Hand)
    }

    fn tag_hand_cards(ctx: &mut ExecutionContext, game: &GameState, tag: &str, cards: &[ObjectId]) {
        let snapshots = cards
            .iter()
            .filter_map(|card| game.object(*card))
            .map(|object| ObjectSnapshot::from_object(object, game))
            .collect();
        ctx.tag_objects(tag, snapshots);
    }

    #[test]
    fn tagged_selected_hand_discard_ignores_untagged_cards_and_unrelated_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let untagged_first = add_card_to_hand(&mut game, "Untouched First", alice);
        let selected_one = add_card_to_hand(&mut game, "Selected One", alice);
        let untagged_last = add_card_to_hand(&mut game, "Untouched Last", alice);
        let selected_two = add_card_to_hand(&mut game, "Selected Two", alice);

        let selected_filter = tagged_hand_filter("selected_hand");
        let mut ctx = ExecutionContext::new_default(source, alice);
        tag_hand_cards(
            &mut ctx,
            &game,
            "selected_hand",
            &[selected_one, selected_two],
        );
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(untagged_first)];

        let effect = DiscardEffect::new_with_filter(
            Value::Count(selected_filter.clone()),
            PlayerFilter::You,
            false,
            Some(selected_filter),
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        let hand = &game.player(alice).unwrap().hand;
        assert!(hand.contains(&untagged_first));
        assert!(hand.contains(&untagged_last));
        assert!(!hand.contains(&selected_one));
        assert!(!hand.contains(&selected_two));
    }

    #[test]
    fn tagged_up_to_x_subset_discards_only_the_cards_actually_selected() {
        struct SelectOne;

        impl crate::decision::DecisionMaker for SelectOne {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                ctx.candidates
                    .iter()
                    .find(|candidate| candidate.legal)
                    .map(|candidate| vec![candidate.id])
                    .unwrap_or_default()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let selected = add_card_to_hand(&mut game, "Chosen For Up To X", alice);
        let not_selected_one = add_card_to_hand(&mut game, "Not Chosen One", alice);
        let not_selected_two = add_card_to_hand(&mut game, "Not Chosen Two", alice);

        let choose = crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::default()
                .in_zone(Zone::Hand)
                .owned_by(PlayerFilter::You),
            crate::effect::ChoiceCount::up_to_dynamic_x(),
            PlayerFilter::You,
            "up_to_x_selection",
        )
        .in_zone(Zone::Hand);
        let mut decision_maker = SelectOne;
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker).with_x(3);
        let choice_outcome = choose.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(
            choice_outcome.value,
            crate::effect::OutcomeValue::Objects(vec![selected])
        );

        let selected_filter = tagged_hand_filter("up_to_x_selection");
        let effect = DiscardEffect::new_with_filter(
            Value::Count(selected_filter.clone()),
            PlayerFilter::You,
            false,
            Some(selected_filter),
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        let hand = &game.player(alice).unwrap().hand;
        assert!(!hand.contains(&selected));
        assert!(hand.contains(&not_selected_one));
        assert!(hand.contains(&not_selected_two));
    }

    #[test]
    fn two_distinct_filtered_selections_accumulated_under_one_tag_both_discard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let low_nonland = add_card_to_hand(&mut game, "Low Nonland", alice);
        let high_value = add_card_to_hand(&mut game, "High Value", alice);
        let filter_miss = add_card_to_hand(&mut game, "Filter Miss", alice);

        let selected_filter = tagged_hand_filter("two_filtered_choices");
        let mut ctx = ExecutionContext::new_default(source, alice);
        tag_hand_cards(&mut ctx, &game, "two_filtered_choices", &[low_nonland]);
        tag_hand_cards(&mut ctx, &game, "two_filtered_choices", &[high_value]);
        let effect = DiscardEffect::new_with_filter(
            Value::Count(selected_filter.clone()),
            PlayerFilter::You,
            false,
            Some(selected_filter),
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        let hand = &game.player(alice).unwrap().hand;
        assert!(!hand.contains(&low_nonland));
        assert!(!hand.contains(&high_value));
        assert!(hand.contains(&filter_miss));
    }

    #[test]
    fn distinct_mana_value_choices_accumulate_then_discard_only_their_selected_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let low_selected = add_card_to_hand_with_mana_value(&mut game, "Low Selected", alice, 2);
        let low_unselected =
            add_card_to_hand_with_mana_value(&mut game, "Low Unselected", alice, 3);
        let high_selected = add_card_to_hand_with_mana_value(&mut game, "High Selected", alice, 5);
        let high_unselected =
            add_card_to_hand_with_mana_value(&mut game, "High Unselected", alice, 6);

        let tag = TagKey::from("two_mana_value_choices");
        let low_filter = ObjectFilter::nonland()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You)
            .with_mana_value(crate::filter::Comparison::LessThanOrEqual(3));
        let high_filter = ObjectFilter::default()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You)
            .with_mana_value(crate::filter::Comparison::GreaterThanOrEqual(4));
        let low_choice = crate::effects::ChooseObjectsEffect::new(
            low_filter,
            crate::effect::ChoiceCount::exactly(1),
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Hand);
        let high_choice = crate::effects::ChooseObjectsEffect::new(
            high_filter,
            crate::effect::ChoiceCount::exactly(1),
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Hand);
        let mut ctx = ExecutionContext::new_default(source, alice);
        low_choice.execute(&mut game, &mut ctx).unwrap();
        high_choice.execute(&mut game, &mut ctx).unwrap();

        let tagged = ctx
            .tagged_objects
            .get(&tag)
            .expect("both filtered choices should populate the shared tag");
        let tagged_ids = tagged
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect::<Vec<_>>();
        assert_eq!(tagged_ids, vec![low_selected, high_selected]);

        let selected_filter = tagged_hand_filter(tag.as_str());
        let discard = DiscardEffect::new_with_filter(
            Value::Count(selected_filter.clone()),
            PlayerFilter::You,
            false,
            Some(selected_filter),
        );
        let result = discard.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        let hand = &game.player(alice).unwrap().hand;
        assert!(!hand.contains(&low_selected));
        assert!(!hand.contains(&high_selected));
        assert!(hand.contains(&low_unselected));
        assert!(hand.contains(&high_unselected));
    }

    #[test]
    fn ordinary_numeric_and_random_discards_are_not_treated_as_preselected() {
        let tagged = tagged_hand_filter("not_a_preselection_count");
        assert!(!tracks_same_selected_objects(
            &Value::Fixed(1),
            Some(&tagged)
        ));

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let explicit = add_card_to_hand(&mut game, "Explicit Numeric Choice", alice);
        let other = add_card_to_hand(&mut game, "Other Card", alice);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(explicit)];
        let numeric = DiscardEffect::you(1);
        numeric.execute(&mut game, &mut ctx).unwrap();
        assert!(!game.player(alice).unwrap().hand.contains(&explicit));
        assert!(game.player(alice).unwrap().hand.contains(&other));

        let random = DiscardEffect::you_random(1);
        assert!(random.random);
        assert!(!tracks_same_selected_objects(
            &random.count,
            random.card_filter.as_ref()
        ));
    }
}

/// Report each completed discard once, including its event-time batch and destination.
fn completed_discard_events(
    game: &mut GameState,
    player_id: crate::ids::PlayerId,
    cause: crate::events::cause::EventCause,
    provenance: crate::provenance::ProvNodeId,
    successful_discards: Vec<(
        crate::ids::ObjectId,
        Option<ObjectSnapshot>,
        Zone,
        Option<crate::ids::ObjectId>,
    )>,
) -> Vec<crate::triggers::TriggerEvent> {
    let batch_cards: Vec<_> = successful_discards
        .iter()
        .map(|(card_id, _, _, _)| *card_id)
        .collect();
    let batch_snapshots: Vec<_> = successful_discards
        .iter()
        .filter_map(|(_, snapshot, _, _)| snapshot.clone())
        .collect();
    let destinations = successful_discards
        .iter()
        .map(
            |(card, _, zone, object)| crate::events::other::DiscardedCardDestination {
                card: *card,
                object: *object,
                zone: *zone,
            },
        )
        .collect::<Vec<_>>();
    let mut discard_events = Vec::new();
    for (batch_index, (card_id, pre_discard_snapshot, final_zone, _)) in
        successful_discards.into_iter().enumerate()
    {
        // Each observation needs its own identity: turn history stages
        // events by provenance before the trigger queue processes them.
        let discard_provenance =
            game.alloc_child_event_provenance(provenance, crate::events::EventKind::Discard);
        discard_events.push(crate::triggers::TriggerEvent::new_with_provenance(
            DiscardEvent::with_cause(card_id, player_id, cause.clone())
                .with_destination(final_zone),
            discard_provenance,
        ));
        let mut event = CardDiscardedEvent::with_cause(player_id, card_id, cause.clone())
            .with_batch(batch_cards.clone(), batch_snapshots.clone(), batch_index)
            .with_destinations(destinations.clone());
        if let Some(snapshot) = pre_discard_snapshot {
            event = event.with_snapshot(snapshot);
        }
        let discarded_provenance =
            game.alloc_child_event_provenance(provenance, crate::events::EventKind::CardDiscarded);
        discard_events.push(crate::triggers::TriggerEvent::new_with_provenance(
            event,
            discarded_provenance,
        ));
    }

    discard_events
}

/// Actual replacement originals belong to the original packet; additions are deferred.
fn retain_discard_completion(
    outcome: EffectOutcome,
    mut receipts: Vec<crate::events::processing::CommittedDiscardOriginal>,
) -> crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs> {
    let primary = outcome.value.clone();
    let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(outcome);
    for original in &mut receipts {
        if let Some(payload) = original.receipt.payload_outcome.take() {
            outputs = outputs.append_replacement_outputs([payload]);
        }
    }
    outputs.outcome.value = primary;
    let completion = receipts
        .iter()
        .any(|original| !original.receipt.programs.is_empty())
        .then(|| {
            Box::new(DiscardBatchCompletion {
                receipts: Some(receipts),
                frozen: None,
            }) as Box<dyn crate::effects::SimultaneousEffectCompletion>
        });
    crate::effects::SimultaneousEffectCommit {
        outcome: outputs,
        completion,
    }
}

struct DiscardBatchCompletion {
    receipts: Option<Vec<crate::events::processing::CommittedDiscardOriginal>>,
    frozen: Option<FrozenDiscardPrograms>,
}
impl crate::effects::SimultaneousEffectCompletion for DiscardBatchCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        // Producers supply completed original records; payloads were transferred
        // into the original output before constructing this additions-only owner.
        crate::effects::OriginalPhaseStatus::Complete
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        if self.frozen.is_none() {
            let receipts = self.receipts.take().ok_or_else(|| {
                ExecutionError::InternalError("discard completion lost original receipts".into())
            })?;
            self.frozen = Some(freeze_discard_receipts(game, receipts)?);
        }
        Ok(())
    }
    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.complete_from_original_outputs(
            game,
            ctx,
            crate::effects::CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let frozen = self.frozen.ok_or_else(|| {
            ExecutionError::InternalError(
                "discard completion was not frozen after originals".into(),
            )
        })?;
        finish_frozen_discard_receipts_with_outputs(game, ctx, original, frozen)
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
}

type FrozenDiscardPrograms = Vec<(
    crate::events::processing::PreparedReplacementProgram,
    crate::effects::replacement::ReplacementProgramBindings,
)>;

fn freeze_discard_receipts(
    game: &GameState,
    receipts: Vec<crate::events::processing::CommittedDiscardOriginal>,
) -> Result<FrozenDiscardPrograms, ExecutionError> {
    let mut frozen = Vec::new();
    for original in receipts {
        let crate::events::processing::CommittedDiscardOriginal {
            receipt,
            arrival_snapshot,
        } = original;
        if arrival_snapshot
            .as_ref()
            .is_some_and(|snapshot| Some(snapshot.object_id) != receipt.result.new_id)
        {
            return Err(ExecutionError::InternalError(
                "discard arrival snapshot lost its original receipt identity".into(),
            ));
        }
        if receipt.payload_outcome.is_some() {
            return Err(ExecutionError::InternalError(
                "discard original payload reached addition binding capture".into(),
            ));
        }
        let original_id = receipt
            .discarded_snapshot
            .as_ref()
            .map(|snapshot| snapshot.object_id);
        let moved_id = receipt.result.new_id;
        let lki = receipt.discarded_snapshot;
        for program in receipt.programs {
            let discarded =
                crate::events::downcast_event::<DiscardEvent>(program.context.event.inner())
                    .ok_or_else(|| {
                        ExecutionError::InternalError(
                            "discard addition lost its discard event".into(),
                        )
                    })?;
            let object = if original_id == Some(discarded.card) {
                moved_id.unwrap_or(discarded.card)
            } else {
                discarded.card
            };
            let snapshot = if original_id == Some(discarded.card) {
                // Exact commit-time arrival evidence survives later sibling changes.
                // A missing arrival falls back to the original's own pre-discard LKI.
                arrival_snapshot.clone().or_else(|| lki.clone())
            } else {
                game.object(object)
                    .map(|object| ObjectSnapshot::from_object(object, game))
                    .or_else(|| {
                        lki.as_ref()
                            .filter(|snapshot| snapshot.object_id == discarded.card)
                            .cloned()
                    })
            };
            let tags = snapshot.map_or_else(Vec::new, |snapshot| {
                vec![
                    ("it".to_owned(), vec![snapshot.clone()]),
                    ("__it__".to_owned(), vec![snapshot]),
                ]
            });
            let bindings = crate::effects::replacement::ReplacementProgramBindings {
                targets: Some(vec![crate::effects::ResolvedTarget::Object(object)]),
                object_tags: tags,
            };
            frozen.push((program, bindings));
        }
    }
    Ok(frozen)
}

fn finish_frozen_discard_receipts_with_outputs<O: crate::effects::OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    outcome: O,
    programs: FrozenDiscardPrograms,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let mut outcome = outcome.into_outputs();
    let primary = outcome.outcome.value.clone();
    if !programs.is_empty() {
        outcome.projections_complete = false;
    }
    let mut outputs =
        crate::effects::replacement::complete_replacement_programs_with_original_outputs(
            game,
            ctx,
            outcome,
            |game, ctx, original| {
                crate::effects::replacement::complete_bound_replacement_programs_with_outputs(
                    game, ctx, original, programs,
                )
            },
        )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }

    outputs.outcome.value = primary;
    outputs.synchronize_observations();
    Ok(outputs)
}

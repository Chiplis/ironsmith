//! Discard effect implementation.

use crate::effect::{EffectOutcome, ExecutionFact, OutcomeObjectMemory, Value};
use crate::effects::helpers::{normalize_object_selection, resolve_player_filter, resolve_value};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::cards::DiscardEvent;
use crate::events::other::CardDiscardedEvent;
use crate::filter::ObjectFilter;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
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
        if constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
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

impl EffectExecutor for DiscardEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::decisions::make_decision;
        use crate::decisions::specs::ChooseObjectsSpec;
        use crate::events::processing::execute_discard;
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
        let mut discarded = 0;
        let mut discarded_cards = Vec::new();
        let mut discarded_snapshots = Vec::new();
        let mut successful_discards = Vec::new();

        let mut hand_cards: Vec<_> = game
            .player(player_id)
            .map(|p| p.hand.to_vec())
            .unwrap_or_default();
        if let Some(filter) = &self.card_filter {
            let filter_ctx = ctx.filter_context(game);
            hand_cards.retain(|card_id| {
                game.object(*card_id)
                    .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
            });
        }

        let required = count.min(hand_cards.len());
        if required == 0 && !self.any_number {
            return Ok(EffectOutcome::count(0));
        }

        let explicit_cards: Vec<_> = ctx
            .targets
            .iter()
            .filter_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => Some(*id),
                crate::effects::ResolvedTarget::Player(_) => None,
            })
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
            normalize_object_selection(explicit_cards, &hand_cards, required)
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
            if one_or_more && hand_cards.is_empty() {
                return Ok(EffectOutcome::count(0));
            }
            let min_required = usize::from(one_or_more);
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
            let chosen: Vec<_> =
                make_decision(game, ctx.decision_maker, player_id, Some(ctx.source), spec);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
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
            let chosen: Vec<_> =
                make_decision(game, ctx.decision_maker, player_id, Some(ctx.source), spec);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            normalize_object_selection(chosen, &hand_cards, required)
        };

        // Discard each card using the event system. The cause is inherited from
        // the execution context so discard-as-cost stays cost-caused.
        let cause = ctx.cause.clone();
        let chosen_cards = cards_to_discard.clone();
        let chosen_memory: Vec<_> = chosen_cards
            .iter()
            .filter_map(|id| OutcomeObjectMemory::from_object_id(game, *id))
            .collect();
        let mut affected_memory = Vec::new();
        for card_id in cards_to_discard {
            let pre_memory = OutcomeObjectMemory::from_object_id(game, card_id);
            let pre_discard_snapshot = game
                .object(card_id)
                .map(|obj| ObjectSnapshot::from_object(obj, game));
            let result = execute_discard(
                game,
                card_id,
                player_id,
                cause.clone(),
                false,
                ctx.provenance,
                &mut *ctx.decision_maker,
            );
            if !result.prevented {
                if card_id == ctx.source
                    && let Some(x) = ctx.x_value
                    && let Some(new_id) = result.new_id
                    && let Some(obj) = game.object_mut(new_id)
                {
                    // Preserve the chosen X on "discard this card" costs so
                    // "when you cycle this card" triggers in the graveyard can
                    // still evaluate references like "mana value equal to X".
                    obj.x_value = Some(x);
                }
                discarded += 1;
                discarded_cards.push(card_id);
                if let Some(memory) = pre_memory {
                    affected_memory.push(memory);
                }
                successful_discards.push((card_id, pre_discard_snapshot, result.final_zone));
                let snapshot_id = result.new_id.unwrap_or(card_id);
                if let Some(obj) = game.object(snapshot_id) {
                    discarded_snapshots.push(ObjectSnapshot::from_object(obj, game));
                }
            }
        }

        let batch_cards: Vec<_> = successful_discards
            .iter()
            .map(|(card_id, _, _)| *card_id)
            .collect();
        let batch_snapshots: Vec<_> = successful_discards
            .iter()
            .filter_map(|(_, snapshot, _)| snapshot.clone())
            .collect();
        let mut discard_events = Vec::new();
        for (batch_index, (card_id, pre_discard_snapshot, final_zone)) in
            successful_discards.into_iter().enumerate()
        {
            discard_events.push(crate::triggers::TriggerEvent::new_with_provenance(
                DiscardEvent::with_cause(card_id, player_id, cause.clone())
                    .with_destination(final_zone),
                ctx.provenance,
            ));
            let mut event = CardDiscardedEvent::with_cause(player_id, card_id, cause.clone())
                .with_batch(batch_cards.clone(), batch_snapshots.clone(), batch_index);
            if let Some(snapshot) = pre_discard_snapshot {
                event = event.with_snapshot(snapshot);
            }
            discard_events.push(crate::triggers::TriggerEvent::new_with_provenance(
                event,
                ctx.provenance,
            ));
        }

        if let Some(tag) = &self.tag
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

        Ok(outcome)
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
        use crate::effects::CostValidationError;

        if !matches!(self.player, PlayerFilter::You | PlayerFilter::Specific(_)) {
            return Err(CostValidationError::Other(
                "discard cost supports only 'you' or a specific player".to_string(),
            ));
        }

        let required = match self.count {
            Value::Fixed(n) => n.max(0) as usize,
            _ => {
                return Err(CostValidationError::Other(
                    "dynamic discard cost amount is unsupported".to_string(),
                ));
            }
        };
        if required == 0 {
            return Ok(());
        }

        let player_id = match self.player {
            PlayerFilter::You => controller,
            PlayerFilter::Specific(id) => id,
            _ => unreachable!("validated above"),
        };

        let mut hand_cards: Vec<_> = game
            .player(player_id)
            .map(|p| p.hand.to_vec())
            .unwrap_or_default();

        if let Some(filter) = &self.card_filter {
            let filter_ctx = crate::filter::FilterContext::new(controller).with_source(source);
            hand_cards.retain(|card_id| {
                game.object(*card_id)
                    .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
            });
        }

        if hand_cards.len() < required {
            return Err(CostValidationError::NotEnoughCards);
        }

        Ok(())
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

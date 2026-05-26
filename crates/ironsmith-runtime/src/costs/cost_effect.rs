//! Effect-backed cost component.
//!
//! This lets costs flow through the normal effect executor/event pipeline
//! while still being represented as a first-class `Cost` inside `TotalCost`.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::effect::Effect;
use crate::effects::{CostExecutableEffect, CostValidationError};
use crate::effects::{ExecutionContext, execute_effect};
use crate::events::cause::EventCause;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;

/// Convert a CostValidationError to CostPaymentError.
fn convert_validation_error(err: CostValidationError) -> CostPaymentError {
    match err {
        CostValidationError::AlreadyTapped => CostPaymentError::AlreadyTapped,
        CostValidationError::AlreadyUntapped => CostPaymentError::AlreadyUntapped,
        CostValidationError::SummoningSickness => CostPaymentError::SummoningSickness,
        CostValidationError::NotEnoughLife => CostPaymentError::InsufficientLife,
        CostValidationError::NotEnoughCards => CostPaymentError::InsufficientCardsInHand,
        CostValidationError::CannotSacrifice => CostPaymentError::NoValidSacrificeTarget,
        CostValidationError::Other(msg) => CostPaymentError::Other(msg),
    }
}

/// A cost paid by executing a single effect.
#[derive(Debug, Clone)]
pub struct CostEffect {
    /// Effect executed as part of paying this cost.
    pub effect: Effect,
}

impl CostEffect {
    pub fn new<E: CostExecutableEffect + 'static>(effect: E) -> Self {
        Self {
            effect: Effect::new(effect),
        }
    }

    pub fn try_new(effect: Effect) -> Result<Self, String> {
        Self::from_validated_effect(effect)
    }

    pub fn from_validated_effect(effect: Effect) -> Result<Self, String> {
        if effect.0.as_cost_executable().is_some() {
            Ok(Self { effect })
        } else {
            Err(format!(
                "effect is not marked as cost-executable: {effect:?}"
            ))
        }
    }
}

impl PartialEq for CostEffect {
    fn eq(&self, _other: &Self) -> bool {
        // Effect partial-eq is intentionally behavioral/not structural.
        false
    }
}

fn tagged_sacrifice_cost_precheck(
    effect: &Effect,
    game: &GameState,
    ctx: &CostContext,
) -> Option<Result<(), CostPaymentError>> {
    let (filter, count, player) = if let Some(effect) =
        effect.downcast_ref::<crate::effects::SacrificeEffect>()
    {
        (&effect.filter, &effect.count, &effect.player)
    } else if let Some(effect) = effect.downcast_ref::<ironsmith_core::SacrificePlayerEffect>() {
        (&effect.filter, &effect.count, &effect.player)
    } else {
        return None;
    };

    if filter.tagged_constraints.is_empty() {
        return None;
    }

    if filter
        .tagged_constraints
        .iter()
        .any(|constraint| !ctx.tagged_objects.contains_key(constraint.tag.as_str()))
    {
        return Some(Err(CostPaymentError::NoValidSacrificeTarget));
    }

    if player != &crate::target::PlayerFilter::You {
        return Some(Err(CostPaymentError::Other(
            "sacrifice costs support only 'you'".to_string(),
        )));
    }

    let required = match count {
        crate::effect::Value::Fixed(count) => (*count).max(0) as usize,
        crate::effect::Value::Count(count_filter)
            if tagged_selection_tag(count_filter)
                .zip(tagged_selection_tag(filter))
                .is_some_and(|(count_tag, sacrifice_tag)| count_tag == sacrifice_tag) =>
        {
            filter
                .tagged_constraints
                .iter()
                .find(|constraint| {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                })
                .and_then(|constraint| ctx.tagged_objects.get(constraint.tag.as_str()))
                .map_or(0, Vec::len)
        }
        _ => {
            return Some(Err(CostPaymentError::Other(
                "dynamic sacrifice cost amount is unsupported".to_string(),
            )));
        }
    };

    if required == 0 {
        return Some(Ok(()));
    }

    let lands_only = ctx.reason.is_cast_or_ability_payment()
        && game.player_cant_sacrifice_nonland_to_cast_or_activate(ctx.payer);
    let filter_ctx = crate::filter::FilterContext::new(ctx.payer)
        .with_source(ctx.source)
        .with_tagged_objects(&ctx.tagged_objects);
    let available = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
        .filter(|(id, obj)| {
            game.controller_of(obj) == ctx.payer
                && (!lands_only || obj.has_card_type(crate::types::CardType::Land))
                && filter.matches(obj, &filter_ctx, game)
                && game.can_be_sacrificed(*id)
        })
        .count();

    if available < required {
        Some(Err(CostPaymentError::NoValidSacrificeTarget))
    } else {
        Some(Ok(()))
    }
}

fn tagged_selection_tag(filter: &crate::filter::ObjectFilter) -> Option<&crate::tag::TagKey> {
    filter
        .tagged_constraints
        .iter()
        .find(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        .map(|constraint| &constraint.tag)
}

fn tagged_move_to_zone_cost_precheck(
    effect: &Effect,
    ctx: &CostContext,
) -> Option<Result<(), CostPaymentError>> {
    let move_to_zone = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let tag = match move_to_zone.target.base() {
        crate::target::ChooseSpec::Tagged(tag) => tag,
        crate::target::ChooseSpec::Object(filter) => tagged_selection_tag(filter)?,
        _ => return None,
    };

    let chosen = ctx.tagged_objects.get(tag.as_str())?;
    if chosen.is_empty() {
        Some(Err(CostPaymentError::Other(
            "move-to-zone cost has no chosen object".to_string(),
        )))
    } else {
        Some(Ok(()))
    }
}

fn simple_exile_from_hand_filter(
    filter: &crate::filter::ObjectFilter,
) -> Option<Option<crate::color::ColorSet>> {
    let mut expected = crate::filter::ObjectFilter::default()
        .in_zone(crate::zone::Zone::Hand)
        .owned_by(crate::target::PlayerFilter::You)
        .other();
    if let Some(colors) = filter.colors {
        expected = expected.with_colors(colors);
    }
    (filter == &expected).then_some(filter.colors)
}

fn simple_exile_from_graveyard_filter(
    filter: &crate::filter::ObjectFilter,
) -> Option<Option<crate::types::CardType>> {
    if filter.card_types.len() > 1 {
        return None;
    }

    let card_type = filter.card_types.first().copied();
    let mut expected = crate::filter::ObjectFilter::default()
        .in_zone(crate::zone::Zone::Graveyard)
        .owned_by(crate::target::PlayerFilter::You);
    if let Some(card_type) = card_type {
        expected = expected.with_type(card_type);
    }
    (filter == &expected).then_some(card_type)
}

impl CostPayer for CostEffect {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        self.effect
            .0
            .can_execute_as_cost_with_reason(game, ctx.source, ctx.payer, ctx.reason)
            .map_err(convert_validation_error)
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        if let Some(result) = tagged_sacrifice_cost_precheck(&self.effect, game, ctx) {
            result?;
        } else if let Some(result) = tagged_move_to_zone_cost_precheck(&self.effect, ctx) {
            result?;
        } else {
            self.can_pay(game, ctx)?;
        }

        // Clone the existing tags to pass to ExecutionContext
        let existing_tags = ctx.tagged_objects.clone();
        let chosen_targets = ctx
            .pre_chosen_cards
            .iter()
            .copied()
            .map(crate::effects::ResolvedTarget::Object)
            .collect();

        let mut exec_ctx = ExecutionContext::new(ctx.source, ctx.payer, &mut *ctx.decision_maker)
            .with_cause(EventCause::from_cost(ctx.source, ctx.payer))
            .with_tagged_objects(existing_tags)
            .with_cost_choice_targets(chosen_targets)
            .with_provenance(ctx.provenance);
        if let Some(x) = ctx.x_value {
            exec_ctx = exec_ctx.with_x(x);
        }

        let outcome = execute_effect(game, &self.effect, &mut exec_ctx)
            .map_err(|e| CostPaymentError::Other(format!("{e:?}")))?;
        if let Some(move_to_zone) = self.effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
            let moved = outcome.affected_objects().map_or(0, |affected| affected.len());
            let count = move_to_zone.target.count();
            let fixed_required = count.max.filter(|max| *max == count.min).map(|_| count.min);
            if let Some(required) = fixed_required
                && moved < required
            {
                return Err(CostPaymentError::Other(format!(
                    "move-to-zone cost moved {moved} objects, required {required}"
                )));
            }
        }
        for event in outcome.events.iter().cloned() {
            game.queue_trigger_event(ctx.provenance, event);
        }

        let removed_marker_total = outcome.total_marker_changes(|event| event.is_removed());

        if ctx.x_value.is_none()
            && removed_marker_total > 0
            && (self
                .effect
                .downcast_ref::<crate::effects::RemoveCountersEffect>()
                .is_some_and(|effect| {
                    matches!(effect.target.base(), crate::target::ChooseSpec::Source)
                })
                || self
                    .effect
                    .downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()
                    .is_some()
                || self
                    .effect
                    .downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
                    .is_some())
        {
            ctx.x_value = Some(removed_marker_total);
        }

        // Copy any new tags back to CostContext for subsequent costs
        ctx.tagged_objects = exec_ctx.tagged_objects;
        ctx.pre_chosen_cards.clear();

        Ok(CostPaymentResult::Paid)
    }

    fn display(&self) -> String {
        self.effect
            .0
            .cost_description()
            .or_else(|| {
                let rendered =
                    crate::compiled_text::compile_effect_list(std::slice::from_ref(&self.effect));
                if rendered.trim().is_empty() {
                    None
                } else {
                    Some(rendered)
                }
            })
            .unwrap_or_else(|| "Perform the stated effect".to_string())
    }

    fn requires_tap(&self) -> bool {
        self.effect.0.is_tap_source_cost()
    }

    fn requires_untap(&self) -> bool {
        self.effect.0.is_untap_source_cost()
    }

    fn is_life_cost(&self) -> bool {
        self.effect.0.pay_life_amount().is_some()
    }

    fn life_amount(&self) -> Option<u32> {
        self.effect.0.pay_life_amount()
    }

    fn is_sacrifice_self(&self) -> bool {
        self.effect.0.is_sacrifice_source_cost()
    }

    fn is_sacrifice(&self) -> bool {
        self.effect
            .downcast_ref::<crate::effects::SacrificeEffect>()
            .is_some()
    }

    fn sacrifice_filter(&self) -> Option<&crate::filter::ObjectFilter> {
        self.effect
            .downcast_ref::<crate::effects::SacrificeEffect>()
            .map(|effect| &effect.filter)
    }

    fn is_discard(&self) -> bool {
        self.effect
            .downcast_ref::<crate::effects::DiscardEffect>()
            .is_some()
            || self
                .effect
                .downcast_ref::<crate::effects::DiscardHandEffect>()
                .is_some()
    }

    fn discard_details(&self) -> Option<(u32, Option<crate::types::CardType>)> {
        let effect = self
            .effect
            .downcast_ref::<crate::effects::DiscardEffect>()?;
        let crate::effect::Value::Fixed(count) = effect.count else {
            return None;
        };
        Some((
            count.max(0) as u32,
            effect
                .card_filter
                .as_ref()
                .and_then(|filter| filter.card_types.first().copied()),
        ))
    }

    fn is_exile_from_hand(&self) -> bool {
        self.exile_from_hand_details().is_some()
    }

    fn exile_from_hand_details(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        self.effect.0.exile_from_hand_cost_info()
    }

    fn is_remove_counters(&self) -> bool {
        self.effect
            .downcast_ref::<crate::effects::RemoveCountersEffect>()
            .is_some()
            || self
                .effect
                .downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()
                .is_some()
            || self
                .effect
                .downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
                .is_some()
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        use crate::costs::CostProcessingMode;
        use crate::effects::{
            DiscardEffect, DiscardHandEffect, ExileEffect, MillEffect, PayEnergyEffect,
            PutCountersEffect, RemoveAnyCountersFromSourceEffect, RemoveCountersEffect,
            ReturnToHandEffect, RevealFromHandEffect, RevealSourceFromHandEffect, SacrificeEffect,
            SacrificeTargetEffect, TapEffect, UntapEffect,
        };
        use crate::target::{ChooseSpec, PlayerFilter};

        if let Some(effect) = self.effect.downcast_ref::<TapEffect>()
            && matches!(effect.target, ChooseSpec::Source)
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<UntapEffect>()
            && matches!(effect.target, ChooseSpec::Source)
        {
            return CostProcessingMode::Immediate;
        }

        if self
            .effect
            .downcast_ref::<crate::effects::LoseLifeEffect>()
            .is_some()
            || self.effect.downcast_ref::<PayEnergyEffect>().is_some()
            || self.effect.downcast_ref::<MillEffect>().is_some()
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<PutCountersEffect>()
            && matches!(effect.target.base(), ChooseSpec::Source)
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<RemoveCountersEffect>()
            && matches!(effect.target.base(), ChooseSpec::Source)
        {
            return CostProcessingMode::Immediate;
        }

        if self
            .effect
            .downcast_ref::<RemoveAnyCountersFromSourceEffect>()
            .is_some()
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<DiscardHandEffect>()
            && effect.player == PlayerFilter::You
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<RevealFromHandEffect>() {
            return CostProcessingMode::RevealFromHand {
                count: effect.count,
                card_type: effect.card_type,
            };
        }

        if self
            .effect
            .downcast_ref::<RevealSourceFromHandEffect>()
            .is_some()
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<SacrificeTargetEffect>()
            && matches!(effect.target, ChooseSpec::Source)
        {
            return CostProcessingMode::InlineWithTriggers;
        }

        if let Some(effect) = self.effect.downcast_ref::<SacrificeEffect>()
            && effect.player == PlayerFilter::You
            && matches!(effect.count, crate::effect::Value::Fixed(1))
        {
            return CostProcessingMode::SacrificeTarget {
                filter: effect.filter.clone(),
            };
        }

        if let Some(effect) = self.effect.downcast_ref::<DiscardEffect>()
            && effect.player == PlayerFilter::You
            && !effect.random
            && let crate::effect::Value::Fixed(count) = effect.count
        {
            if effect
                .card_filter
                .as_ref()
                .is_some_and(|filter| filter.source && filter.zone == Some(crate::zone::Zone::Hand))
            {
                return CostProcessingMode::Immediate;
            }
            return CostProcessingMode::DiscardCards {
                count: count.max(0) as u32,
                card_types: effect
                    .card_filter
                    .as_ref()
                    .map(|filter| filter.card_types.clone())
                    .unwrap_or_default(),
            };
        }

        if let Some(effect) = self.effect.downcast_ref::<ExileEffect>() {
            if matches!(effect.spec.base(), ChooseSpec::Source) {
                return CostProcessingMode::Immediate;
            }

            if let ChooseSpec::Object(filter) = effect.spec.base() {
                let count = effect.spec.count();
                if count.min == 0 || count.dynamic_x {
                    return CostProcessingMode::Immediate;
                }

                if count.max == Some(count.min)
                    && let Some(color_filter) = simple_exile_from_hand_filter(filter)
                {
                    return CostProcessingMode::ExileFromHand {
                        count: count.min as u32,
                        color_filter,
                    };
                }

                if count.max == Some(count.min)
                    && let Some(card_type) = simple_exile_from_graveyard_filter(filter)
                {
                    return CostProcessingMode::ExileFromGraveyard {
                        count: count.min as u32,
                        card_type,
                    };
                }

                if let Some(zone) = filter.zone {
                    return CostProcessingMode::ExileObjects {
                        count: count.min as u32,
                        filter: filter.clone(),
                        zone,
                    };
                }
            }
        }

        if let Some(effect) = self.effect.downcast_ref::<ReturnToHandEffect>() {
            return match effect.spec.base() {
                ChooseSpec::Source => CostProcessingMode::Immediate,
                ChooseSpec::Object(filter) => CostProcessingMode::ReturnToHandTarget {
                    filter: filter.clone(),
                },
                _ => CostProcessingMode::Immediate,
            };
        }

        CostProcessingMode::Immediate
    }

    fn effect_ref(&self) -> Option<&crate::effect::Effect> {
        Some(&self.effect)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::costs::{CostContext, CostPayer, CostPaymentResult};
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effects::{MoveToZoneEffect, RemoveCountersEffect, SacrificeEffect};
    use crate::ids::{CardId, PlayerId};
    use crate::object::CounterType;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::PlayerFilter;
    use crate::types::CardType;
    use crate::{card::CardBuilder, game_state::GameState, zone::Zone};

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn remove_counters_cost_sets_x_from_marker_removal_events() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(1), "Battery")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(source) {
            obj.counters.insert(CounterType::Charge, 3);
        }

        let cost = CostEffect::new(RemoveCountersEffect::new(
            CounterType::Charge,
            2,
            crate::target::ChooseSpec::Source,
        ));
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        let result = cost
            .pay(&mut game, &mut ctx)
            .expect("cost should be payable");

        assert_eq!(result, CostPaymentResult::Paid);
        assert_eq!(ctx.x_value, Some(2));
        assert_eq!(game.counter_count(source, CounterType::Charge), 1);
    }

    #[test]
    fn tagged_sacrifice_cost_can_validate_with_cost_context_tags() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::from_raw(1), "Bone Splinters")
            .card_types(vec![CardType::Sorcery])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Stack);

        let creature_card = CardBuilder::new(CardId::from_raw(2), "Skarrgan Firebird")
            .card_types(vec![CardType::Creature])
            .build();
        let creature = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
        let snapshot = ObjectSnapshot::from_object(game.object(creature).unwrap(), &game);

        let filter = crate::target::ObjectFilter::default().match_tagged(
            "sacrificed_0",
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        );
        let cost = CostEffect::new(SacrificeEffect::new(filter, 1, PlayerFilter::You));

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);
        ctx.tagged_objects
            .insert(TagKey::from("sacrificed_0"), vec![snapshot]);

        let result = cost
            .pay(&mut game, &mut ctx)
            .expect("tagged sacrifice cost should be payable");

        assert_eq!(result, CostPaymentResult::Paid);
        assert!(!game.battlefield.contains(&creature));
        assert!(
            game.player(alice)
                .unwrap()
                .graveyard
                .iter()
                .filter_map(|id| game.object(*id))
                .any(|obj| obj.name == "Skarrgan Firebird")
        );
    }

    #[test]
    fn counted_exile_object_cost_honors_preselected_cost_choices() {
        use crate::color::{Color, ColorSet};
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::target::ChooseSpec;

        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::from_raw(10), "Craft Source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

        let mut add_material = |name: &str, mana_cost: ManaCost, card_type: CardType| {
            let card = CardBuilder::new(CardId::new(), name)
                .mana_cost(mana_cost)
                .card_types(vec![card_type])
                .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard)
        };

        let first_red = add_material(
            "Arc Lightning",
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::Red]]),
            CardType::Sorcery,
        );
        let chosen_red_one = add_material(
            "Lightning Helix",
            ManaCost::from_pips(vec![vec![ManaSymbol::Red], vec![ManaSymbol::White]]),
            CardType::Instant,
        );
        let chosen_red_two = add_material(
            "Lightning Bolt",
            ManaCost::from_pips(vec![vec![ManaSymbol::Red]]),
            CardType::Instant,
        );
        let blue_instant = add_material(
            "Opt",
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]),
            CardType::Instant,
        );

        let material_filter = crate::filter::ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .with_colors(ColorSet::from_color(Color::Red))
            .with_type(CardType::Instant)
            .with_type(CardType::Sorcery);
        let cost = crate::costs::Cost::validated_effect(crate::effect::Effect::exile(
            ChooseSpec::Object(material_filter).with_count(crate::effect::ChoiceCount::at_least(2)),
        ));
        assert!(matches!(
            cost.processing_mode(),
            crate::costs::CostProcessingMode::ExileObjects { count: 2, .. }
        ));

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm)
            .with_pre_chosen_cards(vec![chosen_red_one, chosen_red_two]);

        cost.pay(&mut game, &mut ctx)
            .expect("preselected red material cost should be payable");

        let exiled_names = game
            .exile
            .iter()
            .filter_map(|id| game.object(*id).map(|object| object.name.as_str()))
            .collect::<Vec<_>>();
        assert!(exiled_names.contains(&"Lightning Helix"));
        assert!(exiled_names.contains(&"Lightning Bolt"));

        assert_eq!(game.object(first_red).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(blue_instant).unwrap().zone, Zone::Graveyard);
    }

    #[test]
    fn tagged_move_to_zone_cost_can_validate_with_cost_context_tags() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_card = CardBuilder::new(CardId::from_raw(1), "Oracle of Dust")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

        let creature_card = CardBuilder::new(CardId::from_raw(2), "Silvercoat Lion")
            .card_types(vec![CardType::Creature])
            .build();
        let exiled = game.create_object_from_card(&creature_card, bob, Zone::Exile);
        let snapshot = ObjectSnapshot::from_object(game.object(exiled).unwrap(), &game);

        let tag = TagKey::from("graveyard_cost_0");
        let cost = CostEffect::new(MoveToZoneEffect::to_graveyard(
            crate::target::ChooseSpec::Tagged(tag.clone()),
        ));

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);
        ctx.tagged_objects.insert(tag, vec![snapshot]);

        let result = cost
            .pay(&mut game, &mut ctx)
            .expect("tagged move-to-zone cost should be payable");

        assert_eq!(result, CostPaymentResult::Paid);
        assert!(!game.exile.contains(&exiled));
        assert!(
            game.player(bob)
                .unwrap()
                .graveyard
                .iter()
                .filter_map(|id| game.object(*id))
                .any(|obj| obj.name == "Silvercoat Lion")
        );
    }
}

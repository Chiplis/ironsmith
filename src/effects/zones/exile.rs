//! Exile effect implementation.

use crate::color::{Color, ColorSet};
use crate::effect::{ChoiceCount, EffectOutcome, OutcomeStatus};
use crate::effects::helpers::{
    ObjectApplyResultPolicy, apply_single_target_object_from_context, apply_to_selected_objects,
};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::event_processor::EventOutcome;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::filter::FilterContext;
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

use super::apply_zone_change_with_additional_effects;

/// Effect that exiles permanents.
///
/// Exile moves an object to the exile zone, subject to replacement effects.
/// Unlike destroy, exile is not affected by indestructible.
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Exile target creature (targeted - can fizzle)
/// let effect = ExileEffect::target(ChooseSpec::creature());
///
/// // Exile all creatures (non-targeted - cannot fizzle)
/// let effect = ExileEffect::all(ObjectFilter::creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ExileEffect {
    /// What to exile - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
    /// Whether exiled objects should be turned face down in exile.
    pub face_down: bool,
}

impl ExileEffect {
    /// Create an exile effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self {
            spec,
            face_down: false,
        }
    }

    /// Mark exiled cards as face down.
    pub fn with_face_down(mut self, face_down: bool) -> Self {
        self.face_down = face_down;
        self
    }

    /// Create a targeted exile effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self::with_spec(ChooseSpec::target(spec))
    }

    /// Create a targeted exile effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self::with_spec(ChooseSpec::target(spec).with_count(count))
    }

    /// Create a non-targeted exile effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self::with_spec(ChooseSpec::all(filter))
    }

    /// Create an exile effect targeting a single creature.
    pub fn creature() -> Self {
        Self::target(ChooseSpec::creature())
    }

    /// Create an exile effect targeting a single permanent.
    pub fn permanent() -> Self {
        Self::target(ChooseSpec::permanent())
    }

    /// Create an exile effect targeting any number of targets.
    pub fn any_number(target: ChooseSpec) -> Self {
        Self::targets(target, ChoiceCount::any_number())
    }

    /// Create an exile effect for a specific object.
    pub fn specific(object_id: crate::ids::ObjectId) -> Self {
        Self::with_spec(ChooseSpec::SpecificObject(object_id))
    }

    /// Helper for convenience constructors that mirror ExileAllEffect.
    pub fn creatures() -> Self {
        Self::all(ObjectFilter::creature())
    }

    /// Create an effect that exiles all nonland permanents.
    pub fn nonland_permanents() -> Self {
        Self::all(ObjectFilter::nonland_permanent())
    }

    /// Helper to exile a single object (shared logic).
    fn exile_object(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: crate::ids::ObjectId,
        face_down: bool,
    ) -> Result<Option<OutcomeStatus>, ExecutionError> {
        if let Some(obj) = game.object(object_id) {
            let from_zone = obj.zone;
            let additional_effects = ctx.additional_replacement_effects_snapshot();

            // Process through replacement effects with decision maker.
            let result = apply_zone_change_with_additional_effects(
                game,
                object_id,
                from_zone,
                Zone::Exile,
                ctx.cause.clone(),
                &mut ctx.decision_maker,
                &additional_effects,
            );

            match result {
                EventOutcome::Prevented => {
                    return Ok(Some(crate::effect::OutcomeStatus::Prevented));
                }
                EventOutcome::Proceed(result) => {
                    if result.final_zone == Zone::Exile {
                        for &new_id in &result.new_object_ids {
                            if face_down {
                                game.set_face_down(new_id);
                                if let Some(viewers) = ctx.face_down_exile_viewers_for(object_id) {
                                    for &viewer in viewers {
                                        game.grant_face_down_exile_view(new_id, viewer);
                                    }
                                }
                            }
                            game.add_exiled_with_source_link(ctx.source, new_id);
                        }
                    }
                    return Ok(None); // Successfully exiled
                }
                EventOutcome::Replaced => {
                    // Replacement effects already executed
                    return Ok(Some(crate::effect::OutcomeStatus::Replaced));
                }
                EventOutcome::NotApplicable => {
                    return Ok(Some(crate::effect::OutcomeStatus::TargetInvalid));
                }
            }
        }
        // Object doesn't exist - target is invalid
        Ok(Some(crate::effect::OutcomeStatus::TargetInvalid))
    }

    /// Check if spec uses ctx.targets (Object/Player/AnyTarget filters)
    fn uses_ctx_targets(&self) -> bool {
        matches!(
            self.spec.base(),
            ChooseSpec::Object(_)
                | ChooseSpec::Player(_)
                | ChooseSpec::AnyTarget
                | ChooseSpec::AnyOtherTarget
        )
    }

    fn fixed_cost_filter(&self) -> Option<(&ObjectFilter, u32)> {
        let ChooseSpec::Object(filter) = self.spec.base() else {
            return None;
        };
        let count = self.spec.count();
        if count.min == 0 || count.max != Some(count.min) {
            return None;
        }
        Some((filter, count.min as u32))
    }

    fn exile_from_hand_cost_filter(&self) -> Option<(&ObjectFilter, u32)> {
        let (filter, count) = self.fixed_cost_filter()?;
        (filter.zone == Some(Zone::Hand)).then_some((filter, count))
    }

    fn exile_from_graveyard_cost_filter(&self) -> Option<(&ObjectFilter, u32)> {
        let (filter, count) = self.fixed_cost_filter()?;
        (filter.zone == Some(Zone::Graveyard)).then_some((filter, count))
    }

    fn matching_cost_candidates(
        &self,
        game: &GameState,
        filter: &ObjectFilter,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Vec<crate::ids::ObjectId> {
        let filter_ctx = FilterContext::new(controller).with_source(source);
        let candidate_ids: Vec<_> = match filter.zone {
            Some(Zone::Hand) => game
                .players
                .iter()
                .flat_map(|player| player.hand.iter().copied())
                .collect(),
            Some(Zone::Graveyard) => game
                .players
                .iter()
                .flat_map(|player| player.graveyard.iter().copied())
                .collect(),
            Some(Zone::Battlefield) => game.battlefield.clone(),
            Some(Zone::Library) => game
                .players
                .iter()
                .flat_map(|player| player.library.iter().copied())
                .collect(),
            Some(Zone::Stack) => game.stack.iter().map(|entry| entry.object_id).collect(),
            Some(Zone::Exile) => game.exile.clone(),
            Some(Zone::Command) => game.command_zone.clone(),
            None => Vec::new(),
        };

        candidate_ids
            .into_iter()
            .filter(|id| {
                game.object(*id)
                    .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
            })
            .collect()
    }
}

impl EffectExecutor for ExileEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle targeted effects with special single-target behavior
        // BUT skip for special specs (Tagged, Source, SpecificObject) which don't use ctx.targets
        if self.spec.is_target() && self.uses_ctx_targets() {
            let count = self.spec.count();
            if count.is_single() {
                return apply_single_target_object_from_context(
                    game,
                    ctx,
                    |game, ctx, object_id| Self::exile_object(game, ctx, object_id, self.face_down),
                );
            }
            // Multi-target with count - handle "any number" specially
            if count.min == 0 {
                // "any number" effects - 0 targets is valid
                let mut exiled_count = 0;
                for target in ctx.targets.clone() {
                    if let ResolvedTarget::Object(object_id) = target
                        && Self::exile_object(game, ctx, object_id, self.face_down)?.is_none()
                    {
                        exiled_count += 1;
                    }
                }
                return Ok(EffectOutcome::count(exiled_count));
            }
        }

        // For all/non-targeted effects and special specs (Tagged, Source, etc.),
        // count successful moves to exile.
        let apply_result = match apply_to_selected_objects(
            game,
            ctx,
            &self.spec,
            ObjectApplyResultPolicy::CountApplied,
            |game, ctx, object_id| {
                let Some(from_zone) = game.object(object_id).map(|obj| obj.zone) else {
                    return Ok(false);
                };
                let additional_effects = ctx.additional_replacement_effects_snapshot();
                match apply_zone_change_with_additional_effects(
                    game,
                    object_id,
                    from_zone,
                    Zone::Exile,
                    ctx.cause.clone(),
                    &mut ctx.decision_maker,
                    &additional_effects,
                ) {
                    EventOutcome::Proceed(result) => {
                        if !result.new_object_ids.is_empty() {
                            for &new_id in &result.new_object_ids {
                                if self.face_down && result.final_zone == Zone::Exile {
                                    game.set_face_down(new_id);
                                    if let Some(viewers) =
                                        ctx.face_down_exile_viewers_for(object_id)
                                    {
                                        for &viewer in viewers {
                                            game.grant_face_down_exile_view(new_id, viewer);
                                        }
                                    }
                                }
                                if result.final_zone == Zone::Exile {
                                    game.add_exiled_with_source_link(ctx.source, new_id);
                                }
                            }
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    }
                    EventOutcome::Prevented
                    | EventOutcome::Replaced
                    | EventOutcome::NotApplicable => Ok(false),
                }
            },
        ) {
            Ok(result) => result,
            Err(_) => return Ok(EffectOutcome::target_invalid()),
        };

        Ok(apply_result.outcome)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.spec.is_target() {
            Some(&self.spec)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "target to exile"
    }

    fn exile_from_hand_cost_info(&self) -> Option<(u32, Option<ColorSet>)> {
        let (filter, count) = self.exile_from_hand_cost_filter()?;
        Some((count, filter.colors))
    }

    fn cost_description(&self) -> Option<String> {
        if matches!(self.spec.base(), ChooseSpec::Source) {
            return Some("Exile ~".to_string());
        }

        if let Some((filter, count)) = self.exile_from_hand_cost_filter() {
            let color_prefix = filter
                .colors
                .map(|colors| {
                    let mut pieces = Vec::new();
                    if colors.contains(Color::White) {
                        pieces.push("white");
                    }
                    if colors.contains(Color::Blue) {
                        pieces.push("blue");
                    }
                    if colors.contains(Color::Black) {
                        pieces.push("black");
                    }
                    if colors.contains(Color::Red) {
                        pieces.push("red");
                    }
                    if colors.contains(Color::Green) {
                        pieces.push("green");
                    }
                    if pieces.is_empty() {
                        String::new()
                    } else {
                        format!("{} ", pieces.join(" and "))
                    }
                })
                .unwrap_or_default();
            let amount = if count == 1 {
                "a".to_string()
            } else {
                count.to_string()
            };
            let noun = if count == 1 { "card" } else { "cards" };
            return Some(format!(
                "Exile {amount} {color_prefix}{noun} from your hand"
            ));
        }

        if let Some((filter, count)) = self.exile_from_graveyard_cost_filter() {
            let type_str = filter
                .card_types
                .first()
                .map(|card_type| card_type.card_phrase().to_string())
                .unwrap_or_else(|| "card".to_string());
            return Some(if count == 1 {
                format!("Exile a {type_str} from your graveyard")
            } else {
                format!("Exile {count} {type_str}s from your graveyard")
            });
        }

        None
    }
}

impl CostExecutableEffect for ExileEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        if matches!(self.spec.base(), ChooseSpec::Source) && game.object(source).is_some() {
            return Ok(());
        }

        if let Some((filter, count)) = self.fixed_cost_filter()
            && matches!(filter.zone, Some(Zone::Hand | Zone::Graveyard))
        {
            let matching = self.matching_cost_candidates(game, filter, source, controller);
            if matching.len() < count as usize {
                return Err(crate::effects::CostValidationError::NotEnoughCards);
            }
            return Ok(());
        }

        Err(crate::effects::CostValidationError::Other(
            "unsupported exile cost".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, CardBuilder};
    use crate::effect::ChoiceCount;
    use crate::effects::ChooseObjectsEffect;
    use crate::effects::LookAtTopCardsEffect;
    use crate::executor::{ExecutionContext, ResolvedTarget};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::tag::TagKey;
    use crate::target::PlayerFilter;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_card(
        card_id: u32,
        name: &str,
        mana_symbols: Vec<ManaSymbol>,
        card_type: CardType,
    ) -> Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![mana_symbols]))
            .card_types(vec![card_type])
            .build()
    }

    fn add_card_to_zone(
        game: &mut GameState,
        owner: PlayerId,
        zone: Zone,
        name: &str,
        mana_symbols: Vec<ManaSymbol>,
        card_type: CardType,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_card(id.0 as u32, name, mana_symbols, card_type);
        let obj = Object::from_card(id, &card, owner, zone);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_exile_from_hand_cost_uses_generic_exile_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = add_card_to_zone(
            &mut game,
            alice,
            Zone::Hand,
            "Source",
            vec![ManaSymbol::Blue],
            CardType::Instant,
        );
        add_card_to_zone(
            &mut game,
            alice,
            Zone::Hand,
            "Pitch",
            vec![ManaSymbol::Blue],
            CardType::Instant,
        );

        let effect = ExileEffect::with_spec(
            ChooseSpec::Object(
                ObjectFilter::default()
                    .in_zone(Zone::Hand)
                    .owned_by(crate::target::PlayerFilter::You)
                    .with_colors(ColorSet::from(Color::Blue))
                    .other(),
            )
            .with_count(ChoiceCount::exactly(1)),
        );

        assert!(
            crate::effects::EffectExecutor::can_execute_as_cost(&effect, &game, source, alice)
                .is_ok()
        );
        assert_eq!(
            effect.exile_from_hand_cost_info(),
            Some((1, Some(ColorSet::from(Color::Blue))))
        );
    }

    #[test]
    fn test_exile_from_graveyard_cost_executes_via_generic_exile_effect() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card_id = add_card_to_zone(
            &mut game,
            alice,
            Zone::Graveyard,
            "Spell",
            vec![ManaSymbol::Generic(1)],
            CardType::Instant,
        );

        let effect = ExileEffect::with_spec(
            ChooseSpec::Object(
                ObjectFilter::default()
                    .in_zone(Zone::Graveyard)
                    .owned_by(crate::target::PlayerFilter::You)
                    .with_type(CardType::Instant),
            )
            .with_count(ChoiceCount::exactly(1)),
        );
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(card_id)]);

        let result = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.exile.len(), 1);
    }

    #[test]
    fn choose_from_library_then_exile_face_down_grants_searcher_visibility() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let card_id = add_card_to_zone(
            &mut game,
            bob,
            Zone::Library,
            "Hidden Trophy",
            vec![ManaSymbol::Generic(2)],
            CardType::Artifact,
        );

        let choose = ChooseObjectsEffect::new(
            ObjectFilter::default()
                .in_zone(Zone::Library)
                .owned_by(PlayerFilter::Specific(bob)),
            1,
            PlayerFilter::Specific(alice),
            "chosen",
        )
        .in_zone(Zone::Library)
        .as_search();
        let exile =
            ExileEffect::with_spec(ChooseSpec::Tagged(TagKey::from("chosen"))).with_face_down(true);
        let mut ctx = ExecutionContext::new_default(source, alice);

        choose
            .execute(&mut game, &mut ctx)
            .expect("choose should resolve");
        exile
            .execute(&mut game, &mut ctx)
            .expect("exile should resolve");

        let exiled_id = *game.exile.last().expect("card should be in exile");
        assert_ne!(exiled_id, card_id, "exiled card should be a new object");
        assert!(game.is_face_down(exiled_id));
        assert!(
            game.can_player_look_at_face_down_exiled_card(exiled_id, alice),
            "searcher should keep access to the chosen face-down exiled card"
        );
        assert!(
            !game.can_player_look_at_face_down_exiled_card(exiled_id, bob),
            "library owner should not automatically gain access to an opponent-chosen face-down exiled card"
        );
    }

    #[test]
    fn look_at_top_then_exile_face_down_grants_viewer_visibility() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card_id = add_card_to_zone(
            &mut game,
            alice,
            Zone::Library,
            "Hidden Insight",
            vec![ManaSymbol::Blue],
            CardType::Instant,
        );

        let look = LookAtTopCardsEffect::new(PlayerFilter::You, 1, "looked");
        let exile =
            ExileEffect::with_spec(ChooseSpec::Tagged(TagKey::from("looked"))).with_face_down(true);
        let mut ctx = ExecutionContext::new_default(source, alice);

        look.execute(&mut game, &mut ctx)
            .expect("look should resolve");
        exile
            .execute(&mut game, &mut ctx)
            .expect("exile should resolve");

        let exiled_id = *game.exile.last().expect("card should be in exile");
        assert_ne!(exiled_id, card_id, "exiled card should be a new object");
        assert!(game.is_face_down(exiled_id));
        assert!(
            game.can_player_look_at_face_down_exiled_card(exiled_id, alice),
            "player who looked at the card should keep access after it is exiled face down"
        );
    }

    #[test]
    fn reexiling_face_down_card_in_exile_preserves_face_down_and_visibility() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card_id = add_card_to_zone(
            &mut game,
            alice,
            Zone::Exile,
            "Still Hidden",
            vec![ManaSymbol::Generic(1)],
            CardType::Artifact,
        );
        game.set_face_down(card_id);
        game.grant_face_down_exile_view(card_id, alice);

        let new_id = game
            .move_object_by_effect(card_id, Zone::Exile)
            .expect("re-exile should create a new object");

        assert_ne!(new_id, card_id);
        assert!(game.is_face_down(new_id));
        assert!(
            game.can_player_look_at_face_down_exiled_card(new_id, alice),
            "re-exiled face-down cards should keep their existing look permission"
        );
        assert!(game.object(card_id).is_none(), "old object should be gone");
    }
}

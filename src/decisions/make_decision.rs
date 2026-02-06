//! Generic decision-making function.
//!
//! This module provides the `make_decision` function, which is the primary
//! entry point for making player decisions using the new spec-based system.

use crate::color::Color;
use crate::decision::{DecisionMaker, FallbackStrategy, LegalAction};
use crate::decisions::context::DecisionContext;
use crate::decisions::spec::{AttackerDeclaration, BlockerDeclaration, DecisionSpec};
use crate::decisions::specs::{
    CounterRemovalResponse, DistributeResponse, ManaColorsSpec, NumberSpec, OptionalCostsResponse,
};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::zone::Zone;

// ============================================================================
// The Generic make_decision Function
// ============================================================================

/// Make a player decision using the decision maker.
///
/// This is the primary entry point for the new spec-based decision system.
/// It takes a typed `DecisionSpec` and returns the appropriate response type.
///
/// # Arguments
///
/// * `game` - The current game state
/// * `dm` - Optional mutable reference to a decision maker
/// * `player` - The player making the decision
/// * `source` - Optional source of the effect (for display)
/// * `spec` - The decision specification
///
/// # Returns
///
/// The typed response for this decision spec.
///
/// # Example
///
/// ```ignore
/// use crate::decisions::{make_decision, specs::MaySpec};
///
/// let spec = MaySpec::new(source, "draw a card");
/// let should_draw = make_decision(game, &mut dm, player, Some(source), spec);
/// ```
pub fn make_decision<T: DecisionSpec>(
    game: &GameState,
    dm: &mut (impl DecisionMaker + ?Sized),
    player: PlayerId,
    source: Option<ObjectId>,
    spec: T,
) -> T::Response
where
    T::Response: FromPrimitiveResponse,
{
    // Use the context-based approach
    let ctx = spec.build_context(player, source, game);
    let fallback = spec.default_response(FallbackStrategy::Decline);
    make_decision_from_context(game, dm, ctx, fallback)
}

/// Make a decision using the new context-based approach.
///
/// This function dispatches to the appropriate DecisionMaker method based on context type
fn make_decision_from_context<R: FromPrimitiveResponse>(
    game: &GameState,
    dm: &mut (impl DecisionMaker + ?Sized),
    ctx: DecisionContext,
    fallback: R,
) -> R {
    match ctx {
        DecisionContext::Boolean(ctx) => {
            let result = dm.decide_boolean(game, &ctx);
            R::from_bool(result, fallback)
        }
        DecisionContext::Number(ctx) => {
            let result = dm.decide_number(game, &ctx);
            R::from_number(result, fallback)
        }
        DecisionContext::SelectObjects(ctx) => {
            let result = dm.decide_objects(game, &ctx);
            R::from_objects(result, fallback)
        }
        DecisionContext::SelectOptions(ctx) => {
            let result = dm.decide_options(game, &ctx);
            let descriptions: Vec<String> =
                ctx.options.iter().map(|o| o.description.clone()).collect();
            R::from_options_with_descriptions(result, &descriptions, fallback)
        }
        DecisionContext::Order(ctx) => {
            let result = dm.decide_order(game, &ctx);
            R::from_order(result, fallback)
        }
        DecisionContext::Attackers(ctx) => {
            let result = dm.decide_attackers(game, &ctx);
            R::from_attackers(result, fallback)
        }
        DecisionContext::Blockers(ctx) => {
            let result = dm.decide_blockers(game, &ctx);
            R::from_blockers(result, fallback)
        }
        DecisionContext::Distribute(ctx) => {
            let result = dm.decide_distribute(game, &ctx);
            R::from_distribute(result, fallback)
        }
        DecisionContext::Colors(ctx) => {
            let result = dm.decide_colors(game, &ctx);
            R::from_colors(result, fallback)
        }
        DecisionContext::Counters(ctx) => {
            let result = dm.decide_counters(game, &ctx);
            R::from_counters(result, fallback)
        }
        DecisionContext::Partition(ctx) => {
            let result = dm.decide_partition(game, &ctx);
            R::from_partition(result, fallback)
        }
        DecisionContext::Proliferate(ctx) => {
            let result = dm.decide_proliferate(game, &ctx);
            R::from_proliferate(result, fallback)
        }
        DecisionContext::Priority(ctx) => {
            let result = dm.decide_priority(game, &ctx);
            R::from_priority(result, fallback)
        }
        DecisionContext::Targets(ctx) => {
            let result = dm.decide_targets(game, &ctx);
            R::from_targets(result, fallback)
        }
        DecisionContext::Modes(ctx) => {
            // Modes use the same selection mechanism as options
            let options: Vec<super::context::SelectableOption> = ctx
                .spec
                .modes
                .iter()
                .map(|m| {
                    super::context::SelectableOption::with_legality(
                        m.index,
                        m.description.clone(),
                        m.legal,
                    )
                })
                .collect();
            let select_ctx = super::context::SelectOptionsContext::new(
                ctx.player,
                ctx.source,
                format!("Choose mode for {}", ctx.spell_name),
                options,
                ctx.spec.min_modes,
                ctx.spec.max_modes,
            );
            let result = dm.decide_options(game, &select_ctx);
            R::from_options(result, fallback)
        }
        DecisionContext::HybridChoice(ctx) => {
            // Hybrid/Phyrexian payment choice uses option selection
            let options: Vec<super::context::SelectableOption> = ctx
                .options
                .iter()
                .map(|o| super::context::SelectableOption::new(o.index, o.label.clone()))
                .collect();
            let select_ctx = super::context::SelectOptionsContext::new(
                ctx.player,
                ctx.source,
                format!(
                    "Choose how to pay pip {} of {}",
                    ctx.pip_number, ctx.spell_name
                ),
                options,
                1, // Exactly 1 choice required
                1,
            );
            let result = dm.decide_options(game, &select_ctx);
            R::from_options(result, fallback)
        }
    }
}

/// Make a decision with a specific fallback strategy.
pub fn make_decision_with_fallback<T: DecisionSpec>(
    game: &GameState,
    dm: &mut (impl DecisionMaker + ?Sized),
    player: PlayerId,
    source: Option<ObjectId>,
    spec: T,
    fallback_strategy: FallbackStrategy,
) -> T::Response
where
    T::Response: FromPrimitiveResponse,
{
    // Use the context-based approach
    let ctx = spec.build_context(player, source, game);
    let fallback = spec.default_response(fallback_strategy);
    make_decision_from_context(game, dm, ctx, fallback)
}

// ============================================================================
// FromPrimitiveResponse Trait
// ============================================================================

/// Trait for converting typed decision primitive results to a spec response.
///
/// This trait allows the generic make_decision function to work with
/// different response types for different specs.
/// Default implementations return the provided fallback response.
pub trait FromPrimitiveResponse: Sized {
    /// Convert from a boolean result.
    fn from_bool(result: bool, fallback: Self) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from a number result.
    fn from_number(result: u32, fallback: Self) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from an object selection result.
    fn from_objects(result: Vec<ObjectId>, fallback: Self) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from an option selection result.
    fn from_options(result: Vec<usize>, fallback: Self) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from an option selection result with option descriptions.
    /// This allows types like Zone to map indices back to values.
    fn from_options_with_descriptions(
        result: Vec<usize>,
        descriptions: &[String],
        fallback: Self,
    ) -> Self {
        let _ = descriptions;
        Self::from_options(result, fallback)
    }

    /// Convert from an ordering result.
    fn from_order(result: Vec<ObjectId>, fallback: Self) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from an attackers result.
    fn from_attackers(result: Vec<AttackerDeclaration>, fallback: Self) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from a blockers result.
    fn from_blockers(result: Vec<BlockerDeclaration>, fallback: Self) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from a distribution result.
    fn from_distribute(result: Vec<(Target, u32)>, fallback: Self) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from a colors result.
    fn from_colors(result: Vec<Color>, fallback: Self) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from a counters result.
    fn from_counters(result: Vec<(crate::object::CounterType, u32)>, fallback: Self) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from a partition result.
    fn from_partition(result: Vec<ObjectId>, fallback: Self) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from a proliferate result.
    fn from_proliferate(
        result: crate::decisions::specs::ProliferateResponse,
        fallback: Self,
    ) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from a priority result.
    fn from_priority(result: LegalAction, fallback: Self) -> Self {
        let _ = result;
        fallback
    }

    /// Convert from a targets result.
    fn from_targets(result: Vec<Target>, fallback: Self) -> Self {
        let _ = result;
        fallback
    }
}

// Implement for bool (MayChoice, Boolean primitives)
impl FromPrimitiveResponse for bool {
    fn from_bool(result: bool, _fallback: Self) -> Self {
        result
    }
}

// Implement for u32 (Number primitives)
impl FromPrimitiveResponse for u32 {
    fn from_number(result: u32, _fallback: Self) -> Self {
        result
    }
}

// Implement for ObjectId (single object selection)
impl FromPrimitiveResponse for ObjectId {
    fn from_objects(result: Vec<ObjectId>, fallback: Self) -> Self {
        result.into_iter().next().unwrap_or(fallback)
    }
}

// Implement for Option<ObjectId> (optional object selection)
impl FromPrimitiveResponse for Option<ObjectId> {
    fn from_objects(result: Vec<ObjectId>, _fallback: Self) -> Self {
        result.into_iter().next()
    }
}

// Implement for Vec<ObjectId> (multiple object selection)
impl FromPrimitiveResponse for Vec<ObjectId> {
    fn from_objects(result: Vec<ObjectId>, _fallback: Self) -> Self {
        result
    }

    fn from_order(result: Vec<ObjectId>, _fallback: Self) -> Self {
        result
    }

    fn from_partition(result: Vec<ObjectId>, _fallback: Self) -> Self {
        result
    }
}

// Implement for Vec<usize> (option index selection)
impl FromPrimitiveResponse for Vec<usize> {
    fn from_options(result: Vec<usize>, _fallback: Self) -> Self {
        result
    }
}

// Implement for usize (single option selection)
impl FromPrimitiveResponse for usize {
    fn from_options(result: Vec<usize>, fallback: Self) -> Self {
        result.first().copied().unwrap_or(fallback)
    }
}

// Implement for LegalAction (priority)
impl FromPrimitiveResponse for LegalAction {
    fn from_priority(result: LegalAction, _fallback: Self) -> Self {
        result
    }
}

// Implement for Vec<Target> (target selection)
impl FromPrimitiveResponse for Vec<Target> {
    fn from_targets(result: Vec<Target>, _fallback: Self) -> Self {
        result
    }
}

// Implement for Vec<Color> (mana color selection)
impl FromPrimitiveResponse for Vec<Color> {
    fn from_colors(result: Vec<Color>, _fallback: Self) -> Self {
        result
    }
}

// Implement for Zone (discard destination)
impl FromPrimitiveResponse for Zone {
    fn from_options_with_descriptions(
        result: Vec<usize>,
        descriptions: &[String],
        fallback: Self,
    ) -> Self {
        if let Some(&idx) = result.first()
            && let Some(desc) = descriptions.get(idx)
        {
            match desc.as_str() {
                "Graveyard" => return Zone::Graveyard,
                "Library" => return Zone::Library,
                "Hand" => return Zone::Hand,
                "Exile" => return Zone::Exile,
                "Battlefield" => return Zone::Battlefield,
                "Stack" => return Zone::Stack,
                "Command" => return Zone::Command,
                _ => {}
            }
        }
        fallback
    }
}

// Implement for Vec<AttackerDeclaration> (declare attackers)
impl FromPrimitiveResponse for Vec<AttackerDeclaration> {
    fn from_attackers(result: Vec<AttackerDeclaration>, _fallback: Self) -> Self {
        result
    }
}

// Implement for Vec<BlockerDeclaration> (declare blockers)
impl FromPrimitiveResponse for Vec<BlockerDeclaration> {
    fn from_blockers(result: Vec<BlockerDeclaration>, _fallback: Self) -> Self {
        result
    }
}

// Implement for DistributeResponse
impl FromPrimitiveResponse for DistributeResponse {
    fn from_distribute(result: Vec<(Target, u32)>, _fallback: Self) -> Self {
        result
    }
}

// Implement for CounterRemovalResponse
impl FromPrimitiveResponse for CounterRemovalResponse {
    fn from_counters(result: Vec<(crate::object::CounterType, u32)>, _fallback: Self) -> Self {
        result
    }
}

// Implement for OptionalCostsResponse
impl FromPrimitiveResponse for OptionalCostsResponse {
    fn from_options(result: Vec<usize>, _fallback: Self) -> Self {
        result.into_iter().map(|idx| (idx, 1)).collect()
    }
}

// Implement for ProliferateResponse
impl FromPrimitiveResponse for crate::decisions::specs::ProliferateResponse {
    fn from_proliferate(
        result: crate::decisions::specs::ProliferateResponse,
        _fallback: Self,
    ) -> Self {
        result
    }
}

// ============================================================================
// Specialized make_decision functions for common cases
// ============================================================================

/// Make a boolean (yes/no) decision.
pub fn make_boolean_decision(
    game: &GameState,
    dm: &mut impl DecisionMaker,
    player: PlayerId,
    source: ObjectId,
    description: impl Into<String>,
    fallback: FallbackStrategy,
) -> bool {
    use crate::decisions::specs::MaySpec;
    let spec = MaySpec::new(source, description);
    make_decision_with_fallback(game, dm, player, Some(source), spec, fallback)
}

/// Make a number selection decision.
pub fn make_number_decision(
    game: &GameState,
    dm: &mut impl DecisionMaker,
    player: PlayerId,
    source: ObjectId,
    min: u32,
    max: u32,
    description: impl Into<String>,
    fallback: FallbackStrategy,
) -> u32 {
    let spec = NumberSpec::new(source, min, max, description);
    make_decision_with_fallback(game, dm, player, Some(source), spec, fallback)
}

/// Make a mana color selection decision.
pub fn make_mana_color_decision(
    game: &GameState,
    dm: &mut impl DecisionMaker,
    player: PlayerId,
    source: ObjectId,
    count: u32,
    same_color: bool,
) -> Vec<Color> {
    let spec = ManaColorsSpec::any_color(source, count, same_color);
    make_decision(game, dm, player, Some(source), spec)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decisions::specs::MaySpec;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_make_decision_no_dm_uses_fallback() {
        let game = setup_game();
        let player = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let spec = MaySpec::new(source, "draw a card");
        // AutoPassDecisionMaker returns false for boolean decisions,
        // which matches the "Decline" fallback behavior
        let mut dm = crate::decision::AutoPassDecisionMaker;

        let result = make_decision(&game, &mut dm, player, Some(source), spec);
        // AutoPassDecisionMaker returns false for boolean decisions
        assert!(!result);
    }

    #[test]
    fn test_make_decision_with_accept_fallback() {
        let game = setup_game();
        let player = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let spec = MaySpec::new(source, "draw a card");
        // AutoPassDecisionMaker returns false, but this test is about
        // verifying that a decision maker that accepts works correctly
        struct AlwaysAcceptDm;
        impl DecisionMaker for AlwaysAcceptDm {
            fn decide_boolean(
                &mut self,
                _game: &GameState,
                _ctx: &crate::decisions::context::BooleanContext,
            ) -> bool {
                true
            }
        }
        let mut dm = AlwaysAcceptDm;

        let result = make_decision_with_fallback(
            &game,
            &mut dm,
            player,
            Some(source),
            spec,
            FallbackStrategy::Accept,
        );
        assert!(result);
    }

    #[test]
    fn test_make_boolean_decision() {
        let game = setup_game();
        let player = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);
        // Use a decision maker that accepts
        struct AlwaysAcceptDm;
        impl DecisionMaker for AlwaysAcceptDm {
            fn decide_boolean(
                &mut self,
                _game: &GameState,
                _ctx: &crate::decisions::context::BooleanContext,
            ) -> bool {
                true
            }
        }
        let mut dm = AlwaysAcceptDm;

        let result = make_boolean_decision(
            &game,
            &mut dm,
            player,
            source,
            "do something",
            FallbackStrategy::Accept,
        );
        assert!(result);
    }

    #[test]
    fn test_make_number_decision() {
        let game = setup_game();
        let player = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);
        // Use a decision maker that returns max value
        struct MaxNumberDm;
        impl DecisionMaker for MaxNumberDm {
            fn decide_number(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::NumberContext,
            ) -> u32 {
                ctx.max
            }
        }
        let mut dm = MaxNumberDm;

        let result = make_number_decision(
            &game,
            &mut dm,
            player,
            source,
            1,
            5,
            "choose a number",
            FallbackStrategy::Maximum,
        );
        assert_eq!(result, 5);
    }
}

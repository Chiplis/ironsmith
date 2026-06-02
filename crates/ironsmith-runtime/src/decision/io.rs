use super::legal_actions::commander_action_indices;
use super::*;
use crate::ability::ActivatedAbilityRuntimeExt as _;
use std::io;

// ============================================================================
// Decision Maker Trait (for convenience)
// ============================================================================

/// Trait for something that can make player decisions.
///
/// This is a convenience trait for driving the game loop synchronously.
/// Implementations could be: AI, test harness, etc.
///
/// The trait is fully primitive-based: each `decide_*` method takes a typed
/// context and returns a typed response.
///
/// Default implementations provide deterministic minimal behavior, and
/// implementors can override the relevant methods for interactive or AI control.
pub trait DecisionMaker {
    /// Called when a player auto-passes (had no actions available).
    /// Default implementation does nothing.
    fn on_auto_pass(&mut self, _game: &GameState, _player: PlayerId) {}

    /// Called when an action chain is cancelled due to an invalid choice.
    /// The game state is restored to the checkpoint before the action started.
    /// Default implementation does nothing.
    fn on_action_cancelled(&mut self, _game: &GameState, _reason: &str) {}

    /// True when the decision maker is only surfacing a prompt and execution
    /// should avoid committing fallback choices to game state.
    fn awaiting_choice(&self) -> bool {
        false
    }

    // ========================================================================
    // Primitive-specific methods
    // ========================================================================
    // These methods take typed context structs and return typed responses.
    // Default implementations return minimal/declining choices.
    // Implementers should override these methods for meaningful behavior.

    /// Boolean decisions (may, ward, miracle, madness).
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        // Default: decline optional actions
        false
    }

    /// Number selection (X value, choose number).
    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        // Default: choose minimum value
        ctx.min
    }

    /// Free-form text entry decisions.
    fn decide_text(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        ctx.initial_value.clone().unwrap_or_default()
    }

    /// Object selection (sacrifice, discard, search, etc.).
    /// Returns IDs of selected objects.
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        // Default: select minimum required from legal candidates
        ctx.candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .take(ctx.min)
            .collect()
    }

    /// Option selection (modes, choices, priority actions, mana payment).
    /// Returns indices of selected options.
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        // Default: select legal options until the required point total is met.
        if ctx.min == 0 {
            return Vec::new();
        }

        let mut selected = Vec::new();
        let mut total = 0usize;
        for option in ctx.options.iter().filter(|o| o.legal) {
            let cost = option.point_cost.max(1) as usize;
            if total.saturating_add(cost) > ctx.max {
                continue;
            }
            selected.push(option.index);
            total += cost;
            if total >= ctx.min {
                break;
            }
        }
        selected
    }

    /// Ordering (blockers, attackers, scry, surveil).
    /// Returns the items in the desired order.
    fn decide_order(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        // Default: keep original order
        ctx.items.iter().map(|(id, _)| *id).collect()
    }

    /// View cards in a private zone (e.g., look at a player's hand).
    /// Default implementation does nothing.
    fn view_cards(
        &mut self,
        _game: &GameState,
        _viewer: PlayerId,
        _cards: &[ObjectId],
        _ctx: &crate::decisions::context::ViewCardsContext,
    ) {
    }

    /// Combat - attackers.
    fn decide_attackers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        // Default: don't attack with anything
        Vec::new()
    }

    /// Combat - blockers.
    fn decide_blockers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        // Default: don't block with anything
        Vec::new()
    }

    /// Distribution (damage, counters).
    fn decide_distribute(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        // Default: empty distribution
        Vec::new()
    }

    /// Color selection.
    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        // Default: green for all requested colors
        vec![crate::color::Color::Green; ctx.count as usize]
    }

    /// Counter removal.
    fn decide_counters(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        // Default: don't remove any counters
        Vec::new()
    }

    /// Partition (scry top/bottom, surveil library/graveyard).
    /// Returns the items to put in the "secondary" destination.
    fn decide_partition(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        // Default: keep all cards in primary destination
        Vec::new()
    }

    /// Proliferate (mixed objects and players).
    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        // Default: don't proliferate anything
        crate::decisions::specs::ProliferateResponse::default()
    }

    /// Priority decisions (choose action).
    fn decide_priority(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        // Default: pass priority if possible, otherwise take first action
        ctx.actions
            .iter()
            .find(|a| matches!(a, LegalAction::PassPriority))
            .cloned()
            .unwrap_or_else(|| {
                ctx.actions
                    .first()
                    .cloned()
                    .unwrap_or(LegalAction::PassPriority)
            })
    }

    /// Target selection for spells and abilities.
    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        normalize_targets_for_requirements(&ctx.requirements, Vec::new()).unwrap_or_default()
    }
}

/// Routes decisions to the controlling player's DecisionMaker.
///
/// This is used for effects that let one player control another player's decisions.
pub struct DecisionRouter {
    per_player: HashMap<PlayerId, Box<dyn DecisionMaker>>,
    default: Box<dyn DecisionMaker>,
}

impl DecisionRouter {
    /// Create a router with a default DecisionMaker.
    pub fn new(default: Box<dyn DecisionMaker>) -> Self {
        Self {
            per_player: HashMap::new(),
            default,
        }
    }

    /// Register a DecisionMaker for a specific player.
    pub fn with_player(mut self, player: PlayerId, dm: Box<dyn DecisionMaker>) -> Self {
        self.per_player.insert(player, dm);
        self
    }

    /// Replace the DecisionMaker for a specific player.
    pub fn set_player(&mut self, player: PlayerId, dm: Box<dyn DecisionMaker>) {
        self.per_player.insert(player, dm);
    }

    fn dm_for<'a>(&'a mut self, game: &GameState, player: PlayerId) -> &'a mut dyn DecisionMaker {
        let controller = game.controlling_player_for(player);
        if let Some(dm) = self.per_player.get_mut(&controller) {
            return dm.as_mut();
        }
        self.default.as_mut()
    }
}

impl DecisionMaker for DecisionRouter {
    fn on_auto_pass(&mut self, game: &GameState, player: PlayerId) {
        self.dm_for(game, player).on_auto_pass(game, player);
    }

    fn on_action_cancelled(&mut self, game: &GameState, reason: &str) {
        self.default.on_action_cancelled(game, reason);
        for dm in self.per_player.values_mut() {
            dm.on_action_cancelled(game, reason);
        }
    }

    fn decide_boolean(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.dm_for(game, ctx.player).decide_boolean(game, ctx)
    }

    fn decide_number(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        self.dm_for(game, ctx.player).decide_number(game, ctx)
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.dm_for(game, ctx.player).decide_objects(game, ctx)
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.dm_for(game, ctx.player).decide_options(game, ctx)
    }

    fn decide_text(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        self.dm_for(game, ctx.player).decide_text(game, ctx)
    }

    fn view_cards(
        &mut self,
        game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        self.dm_for(game, viewer)
            .view_cards(game, viewer, cards, ctx);
    }

    fn decide_order(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        self.dm_for(game, ctx.player).decide_order(game, ctx)
    }

    fn decide_attackers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        if let Some(controller) = game.combat_choice_controller_for_attackers() {
            if let Some(dm) = self.per_player.get_mut(&controller) {
                return dm.decide_attackers(game, ctx);
            }
            return self.default.decide_attackers(game, ctx);
        }
        self.dm_for(game, ctx.player).decide_attackers(game, ctx)
    }

    fn decide_blockers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        if let Some(controller) = game.combat_choice_controller_for_blockers() {
            if let Some(dm) = self.per_player.get_mut(&controller) {
                return dm.decide_blockers(game, ctx);
            }
            return self.default.decide_blockers(game, ctx);
        }
        self.dm_for(game, ctx.player).decide_blockers(game, ctx)
    }

    fn decide_distribute(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        self.dm_for(game, ctx.player).decide_distribute(game, ctx)
    }

    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        self.dm_for(game, ctx.player).decide_colors(game, ctx)
    }

    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        self.dm_for(game, ctx.player).decide_counters(game, ctx)
    }

    fn decide_partition(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        self.dm_for(game, ctx.player).decide_partition(game, ctx)
    }

    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        self.dm_for(game, ctx.player).decide_proliferate(game, ctx)
    }

    fn decide_priority(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        self.dm_for(game, ctx.player).decide_priority(game, ctx)
    }

    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        self.dm_for(game, ctx.player).decide_targets(game, ctx)
    }
}

// ============================================================================
// Blanket implementations
// ============================================================================

/// Blanket impl so `&mut D` implements `DecisionMaker` where `D: DecisionMaker`.
/// This allows passing `&mut dyn DecisionMaker` to functions expecting `impl DecisionMaker`.
impl<D: DecisionMaker + ?Sized> DecisionMaker for &mut D {
    fn on_auto_pass(&mut self, game: &GameState, player: PlayerId) {
        (*self).on_auto_pass(game, player)
    }

    fn on_action_cancelled(&mut self, game: &GameState, reason: &str) {
        (*self).on_action_cancelled(game, reason)
    }

    fn awaiting_choice(&self) -> bool {
        (**self).awaiting_choice()
    }

    fn decide_boolean(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        (*self).decide_boolean(game, ctx)
    }

    fn decide_number(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        (*self).decide_number(game, ctx)
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        (*self).decide_objects(game, ctx)
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        (*self).decide_options(game, ctx)
    }

    fn decide_text(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        (*self).decide_text(game, ctx)
    }

    fn decide_order(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        (*self).decide_order(game, ctx)
    }

    fn view_cards(
        &mut self,
        game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        (*self).view_cards(game, viewer, cards, ctx)
    }

    fn decide_attackers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        (*self).decide_attackers(game, ctx)
    }

    fn decide_blockers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        (*self).decide_blockers(game, ctx)
    }

    fn decide_distribute(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        (*self).decide_distribute(game, ctx)
    }

    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        (*self).decide_colors(game, ctx)
    }

    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        (*self).decide_counters(game, ctx)
    }

    fn decide_partition(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        (*self).decide_partition(game, ctx)
    }

    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        (*self).decide_proliferate(game, ctx)
    }

    fn decide_priority(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        (*self).decide_priority(game, ctx)
    }

    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        (*self).decide_targets(game, ctx)
    }
}

/// Blanket impl so `Box<D>` implements `DecisionMaker` where `D: DecisionMaker`.
/// This allows using `Box<dyn DecisionMaker>` in struct fields.
impl<D: DecisionMaker + ?Sized> DecisionMaker for Box<D> {
    fn on_auto_pass(&mut self, game: &GameState, player: PlayerId) {
        (**self).on_auto_pass(game, player)
    }

    fn on_action_cancelled(&mut self, game: &GameState, reason: &str) {
        (**self).on_action_cancelled(game, reason)
    }

    fn awaiting_choice(&self) -> bool {
        (**self).awaiting_choice()
    }

    fn decide_boolean(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        (**self).decide_boolean(game, ctx)
    }

    fn decide_number(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        (**self).decide_number(game, ctx)
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        (**self).decide_objects(game, ctx)
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        (**self).decide_options(game, ctx)
    }

    fn decide_text(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        (**self).decide_text(game, ctx)
    }

    fn decide_order(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        (**self).decide_order(game, ctx)
    }

    fn view_cards(
        &mut self,
        game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        (**self).view_cards(game, viewer, cards, ctx)
    }

    fn decide_attackers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        (**self).decide_attackers(game, ctx)
    }

    fn decide_blockers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        (**self).decide_blockers(game, ctx)
    }

    fn decide_distribute(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        (**self).decide_distribute(game, ctx)
    }

    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        (**self).decide_colors(game, ctx)
    }

    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        (**self).decide_counters(game, ctx)
    }

    fn decide_partition(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        (**self).decide_partition(game, ctx)
    }

    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        (**self).decide_proliferate(game, ctx)
    }

    fn decide_priority(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        (**self).decide_priority(game, ctx)
    }

    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        (**self).decide_targets(game, ctx)
    }
}

/// A decision maker that always passes priority and makes minimal choices.
///
/// Useful for testing basic game flow.
#[derive(Debug, Default)]
pub struct AutoPassDecisionMaker;

fn auto_select_option_indices(
    ctx: &crate::decisions::context::SelectOptionsContext,
    desired_count: usize,
) -> Vec<usize> {
    let legal: Vec<&crate::decisions::context::SelectableOption> =
        ctx.options.iter().filter(|o| o.legal).collect();
    if legal.is_empty() || desired_count == 0 {
        return Vec::new();
    }

    let mut selected = Vec::new();
    let mut counts: HashMap<usize, usize> = HashMap::new();

    while selected.len() < desired_count {
        let mut added = false;
        for option in &legal {
            let current = counts.get(&option.index).copied().unwrap_or(0);
            let limit = if option.repeatable {
                option
                    .max_count
                    .map(|count| count as usize)
                    .unwrap_or(usize::MAX)
            } else {
                1
            };
            if current >= limit {
                continue;
            }
            selected.push(option.index);
            counts.insert(option.index, current + 1);
            added = true;
            break;
        }
        if !added {
            break;
        }
    }

    selected
}

impl DecisionMaker for AutoPassDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        // Auto-pass: decline all optional actions
        false
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        // Auto-pass: choose maximum value (most common for "up to" effects)
        ctx.max
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        // Auto-pass: select minimum required, using first legal candidates
        let legal: Vec<ObjectId> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .collect();
        legal.into_iter().take(ctx.min).collect()
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        // Auto-pass: select minimum required, using first legal options
        auto_select_option_indices(ctx, ctx.min)
    }

    fn decide_text(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        ctx.initial_value.clone().unwrap_or_default()
    }

    fn decide_order(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        // Auto-pass: keep original order
        ctx.items.iter().map(|(id, _)| *id).collect()
    }

    fn decide_attackers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        // Auto-pass: don't attack
        Vec::new()
    }

    fn decide_blockers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        // Auto-pass: don't block
        Vec::new()
    }

    fn decide_distribute(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(crate::game_state::Target, u32)> {
        // Auto-pass: don't distribute anything
        Vec::new()
    }

    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        // Auto-pass: default to green for each mana
        let default_color = ctx
            .available_colors
            .as_ref()
            .and_then(|colors| colors.first().copied())
            .unwrap_or(crate::color::Color::Green);
        vec![default_color; ctx.count as usize]
    }

    fn decide_counters(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        // Auto-pass: remove as many counters as possible
        let mut remaining = ctx.max_total;
        let mut selections = Vec::new();
        for (counter_type, available) in &ctx.available_counters {
            if remaining == 0 {
                break;
            }
            let to_remove = (*available).min(remaining);
            if to_remove > 0 {
                selections.push((*counter_type, to_remove));
                remaining -= to_remove;
            }
        }
        selections
    }

    fn decide_partition(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        // Auto-pass: keep all cards in primary destination (top of library)
        Vec::new()
    }

    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        // Auto-pass: proliferate all eligible targets
        crate::decisions::specs::ProliferateResponse {
            permanents: ctx.eligible_permanents.iter().map(|(id, _)| *id).collect(),
            players: ctx.eligible_players.iter().map(|(id, _)| *id).collect(),
        }
    }

    fn decide_priority(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        // Auto-pass: always pass priority
        LegalAction::PassPriority
    }
}

/// A decision maker that always selects the first available option.
///
/// Unlike `AutoPassDecisionMaker` which selects the minimum required (often 0),
/// this decision maker always selects the first legal option when available.
/// Useful for testing effects where you want to verify behavior when a choice is made.
#[derive(Debug, Default)]
pub struct SelectFirstDecisionMaker;

impl DecisionMaker for SelectFirstDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        // Select first: accept optional actions
        true
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        // Select first: choose maximum value
        ctx.max
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        // Select first: select first legal option (up to max)
        let legal: Vec<ObjectId> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .collect();
        let count = ctx.max.unwrap_or(1).min(legal.len());
        legal.into_iter().take(count).collect()
    }

    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        let mut proposed = Vec::new();
        for requirement in &ctx.requirements {
            let count = requirement
                .max_targets
                .unwrap_or(1)
                .max(requirement.min_targets)
                .min(requirement.legal_targets.len());
            proposed.extend(requirement.legal_targets.iter().take(count).copied());
        }
        normalize_targets_for_requirements(&ctx.requirements, proposed).unwrap_or_default()
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        // Select first: select first legal option (up to max)
        auto_select_option_indices(ctx, ctx.max)
    }

    fn decide_order(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        // Keep original order
        ctx.items.iter().map(|(id, _)| *id).collect()
    }

    fn decide_attackers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        Vec::new()
    }

    fn decide_blockers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        Vec::new()
    }

    fn decide_distribute(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(crate::game_state::Target, u32)> {
        Vec::new()
    }

    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        let default_color = ctx
            .available_colors
            .as_ref()
            .and_then(|colors| colors.first().copied())
            .unwrap_or(crate::color::Color::Green);
        vec![default_color; ctx.count as usize]
    }

    fn decide_counters(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        let mut remaining = ctx.max_total;
        let mut selections = Vec::new();
        for (counter_type, available) in &ctx.available_counters {
            if remaining == 0 {
                break;
            }
            let to_remove = (*available).min(remaining);
            if to_remove > 0 {
                selections.push((*counter_type, to_remove));
                remaining -= to_remove;
            }
        }
        selections
    }

    fn decide_partition(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        Vec::new()
    }

    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        crate::decisions::specs::ProliferateResponse {
            permanents: ctx.eligible_permanents.iter().map(|(id, _)| *id).collect(),
            players: ctx.eligible_players.iter().map(|(id, _)| *id).collect(),
        }
    }

    fn decide_priority(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        LegalAction::PassPriority
    }
}

/// A decision maker that interprets numeric string inputs the same way the CLI does.
///
/// This allows tests to use the same input format as recorded sessions from `--record`.
/// Empty strings are treated as "pass" or "no selection" depending on context.
///
/// # Example
///
/// ```ignore
/// // Simulate: pass priority, then take action 0, then pass again
/// let mut dm = NumericInputDecisionMaker::new(vec!["".to_string(), "0".to_string(), "".to_string()]);
/// ```
#[derive(Debug)]
pub struct NumericInputDecisionMaker {
    inputs: Vec<String>,
    index: usize,
    debug: bool,
}

impl NumericInputDecisionMaker {
    /// Create a new numeric input decision maker with the given inputs.
    pub fn new(inputs: Vec<String>) -> Self {
        Self {
            inputs,
            index: 0,
            debug: false,
        }
    }

    /// Create from a slice of string references.
    pub fn from_strs(inputs: &[&str]) -> Self {
        Self {
            inputs: inputs.iter().map(|s| s.to_string()).collect(),
            index: 0,
            debug: false,
        }
    }

    /// Enable debug output for tracing decisions.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Get the next input, or empty string if exhausted.
    fn next_input(&mut self) -> String {
        if self.index < self.inputs.len() {
            let input = self.inputs[self.index].clone();
            self.index += 1;
            input
        } else {
            String::new()
        }
    }
}

impl DecisionMaker for NumericInputDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision boolean[{}]: {} | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                input
            );
        }

        // "y"/"yes"/"1" = true, empty/"n"/"no" = false
        matches!(trimmed.to_lowercase().as_str(), "y" | "yes" | "1")
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision number[{}]: {} (min={}, max={}) | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.min,
                ctx.max,
                input
            );
        }

        if let Ok(n) = trimmed.parse::<u32>()
            && n >= ctx.min
            && n <= ctx.max
        {
            return n;
        }
        ctx.min
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision objects[{}]: {} ({} candidates, min={}, max={}) | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.candidates.len(),
                ctx.min,
                ctx.max
                    .map(|max| max.to_string())
                    .unwrap_or_else(|| "any".to_string()),
                input
            );
        }

        if trimmed.is_empty() && (ctx.min == 0 || ctx.allow_partial_completion) {
            return Vec::new();
        }

        let legal: Vec<ObjectId> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .collect();

        let mut selected = Vec::new();
        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>()
                && idx < legal.len()
            {
                if let Some(max) = ctx.max {
                    if selected.len() < max {
                        selected.push(legal[idx]);
                    }
                } else {
                    selected.push(legal[idx]);
                }
            }
        }

        // If we didn't select enough, auto-select from beginning
        if !ctx.allow_partial_completion {
            while selected.len() < ctx.min && selected.len() < legal.len() {
                if !selected.contains(&legal[selected.len()]) {
                    selected.push(legal[selected.len()]);
                } else {
                    break;
                }
            }
        }

        selected
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision options[{}]: {} ({} options, min={}, max={}) | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.options.len(),
                ctx.min,
                ctx.max,
                input
            );
        }

        if trimmed.is_empty() && ctx.min == 0 {
            return Vec::new();
        }

        let legal: Vec<usize> = ctx
            .options
            .iter()
            .filter(|o| o.legal)
            .map(|o| o.index)
            .collect();

        let mut selected = Vec::new();
        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>()
                && idx < legal.len()
                && selected.len() < ctx.max
            {
                selected.push(legal[idx]);
            }
        }

        // If we didn't select enough, auto-select from beginning
        while selected.len() < ctx.min && selected.len() < legal.len() {
            if !selected.contains(&legal[selected.len()]) {
                selected.push(legal[selected.len()]);
            } else {
                break;
            }
        }

        selected
    }

    fn decide_order(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision order[{}]: {} ({} items) | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.items.len(),
                input
            );
        }

        let items: Vec<ObjectId> = ctx.items.iter().map(|(id, _)| *id).collect();

        if trimmed.is_empty() {
            return items; // Keep original order
        }

        // Parse comma-separated indices to reorder
        let mut ordered = Vec::new();
        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>()
                && idx < items.len()
                && !ordered.contains(&items[idx])
            {
                ordered.push(items[idx]);
            }
        }

        // Add any remaining items not specified
        for id in items {
            if !ordered.contains(&id) {
                ordered.push(id);
            }
        }

        ordered
    }

    fn decide_attackers(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision attackers[{}]: {} options | input '{}'",
                self.index.saturating_sub(1),
                ctx.attacker_options.len(),
                input
            );
        }

        if trimmed.is_empty() {
            return Vec::new();
        }

        // Parse comma-separated indices
        let mut declarations = Vec::new();
        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>()
                && idx < ctx.attacker_options.len()
            {
                let opt = &ctx.attacker_options[idx];
                // Pick first valid target (usually opponent)
                if let Some(target) = opt.valid_targets.first() {
                    declarations.push(crate::decisions::spec::AttackerDeclaration {
                        creature: opt.creature,
                        target: target.clone(),
                    });
                }
            }
        }
        declarations
    }

    fn decide_blockers(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision blockers[{}]: {} attacker options | input '{}'",
                self.index.saturating_sub(1),
                ctx.blocker_options.len(),
                input
            );
        }

        if trimmed.is_empty() {
            return Vec::new();
        }

        // Parse "blocker_idx:attacker_idx,..." format
        let mut declarations = Vec::new();
        for part in trimmed.split(',') {
            if let Some((b_str, a_str)) = part.split_once(':')
                && let (Ok(b_idx), Ok(a_idx)) =
                    (b_str.trim().parse::<usize>(), a_str.trim().parse::<usize>())
                && a_idx < ctx.blocker_options.len()
            {
                let opt = &ctx.blocker_options[a_idx];
                if b_idx < opt.valid_blockers.len() {
                    declarations.push(crate::decisions::spec::BlockerDeclaration {
                        blocker: opt.valid_blockers[b_idx].0,
                        blocking: opt.attacker,
                    });
                }
            }
        }
        declarations
    }

    fn decide_distribute(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(crate::game_state::Target, u32)> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision distribute[{}]: {} (total={}, {} targets) | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.total,
                ctx.targets.len(),
                input
            );
        }

        if trimmed.is_empty() || ctx.targets.is_empty() {
            // Default: put all on first target
            if let Some(first) = ctx.targets.first() {
                return vec![(first.target, ctx.total)];
            }
            return Vec::new();
        }

        // Parse "amount:target_idx,amount:target_idx,..." format
        let mut distribution = Vec::new();
        let mut remaining = ctx.total;

        for part in trimmed.split(',') {
            if remaining == 0 {
                break;
            }
            if let Some((amt_str, idx_str)) = part.split_once(':')
                && let (Ok(amount), Ok(idx)) = (
                    amt_str.trim().parse::<u32>(),
                    idx_str.trim().parse::<usize>(),
                )
                && idx < ctx.targets.len()
            {
                let to_distribute = amount.min(remaining);
                if to_distribute >= ctx.min_per_target {
                    distribution.push((ctx.targets[idx].target, to_distribute));
                    remaining -= to_distribute;
                }
            }
        }

        // If nothing was distributed, put all on first target
        if distribution.is_empty() && !ctx.targets.is_empty() {
            distribution.push((ctx.targets[0].target, ctx.total));
        }

        distribution
    }

    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision colors[{}]: count={}, same_color={} | input '{}'",
                self.index.saturating_sub(1),
                ctx.count,
                ctx.same_color,
                input
            );
        }

        use crate::color::Color;
        let mut colors = Vec::new();

        for c in trimmed.to_uppercase().chars() {
            match c {
                'W' => colors.push(Color::White),
                'U' => colors.push(Color::Blue),
                'B' => colors.push(Color::Black),
                'R' => colors.push(Color::Red),
                'G' => colors.push(Color::Green),
                ' ' => continue,
                _ => {} // Ignore invalid characters
            }
        }

        // Determine default color
        let default_color = ctx
            .available_colors
            .as_ref()
            .and_then(|colors| colors.first().copied())
            .unwrap_or(Color::Green);

        // Pad with default if not enough colors provided
        while colors.len() < ctx.count as usize {
            colors.push(default_color);
        }

        // Truncate if too many
        colors.truncate(ctx.count as usize);
        colors
    }

    fn decide_counters(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision counters[{}]: max_total={}, {} types | input '{}'",
                self.index.saturating_sub(1),
                ctx.max_total,
                ctx.available_counters.len(),
                input
            );
        }

        if trimmed.is_empty() {
            return Vec::new();
        }

        // Parse "count:type_idx,count:type_idx,..." format
        let mut selections = Vec::new();
        let mut remaining = ctx.max_total;

        for part in trimmed.split(',') {
            if remaining == 0 {
                break;
            }
            if let Some((count_str, idx_str)) = part.split_once(':')
                && let (Ok(count), Ok(idx)) = (
                    count_str.trim().parse::<u32>(),
                    idx_str.trim().parse::<usize>(),
                )
                && idx < ctx.available_counters.len()
            {
                let (counter_type, available) = ctx.available_counters[idx];
                let to_remove = count.min(available).min(remaining);
                if to_remove > 0 {
                    selections.push((counter_type, to_remove));
                    remaining -= to_remove;
                }
            }
        }

        selections
    }

    fn decide_partition(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision partition[{}]: {} ({} cards) | input '{}'",
                self.index.saturating_sub(1),
                ctx.description,
                ctx.cards.len(),
                input
            );
        }

        if trimmed.is_empty() {
            return Vec::new(); // Keep all in primary destination
        }

        let cards: Vec<ObjectId> = ctx.cards.iter().map(|(id, _)| *id).collect();

        // Parse comma-separated indices for secondary destination
        let mut to_secondary = Vec::new();
        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>()
                && idx < cards.len()
                && !to_secondary.contains(&cards[idx])
            {
                to_secondary.push(cards[idx]);
            }
        }

        to_secondary
    }

    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision proliferate[{}]: {} permanents, {} players | input '{}'",
                self.index.saturating_sub(1),
                ctx.eligible_permanents.len(),
                ctx.eligible_players.len(),
                input
            );
        }

        if trimmed.is_empty() {
            // Default: select all
            return crate::decisions::specs::ProliferateResponse {
                permanents: ctx.eligible_permanents.iter().map(|(id, _)| *id).collect(),
                players: ctx.eligible_players.iter().map(|(id, _)| *id).collect(),
            };
        }

        // Parse "p:idx,o:idx,..." format where p=permanent, o=player
        let mut permanents = Vec::new();
        let mut players = Vec::new();

        for part in trimmed.split(',') {
            if let Some((kind, idx_str)) = part.split_once(':')
                && let Ok(idx) = idx_str.trim().parse::<usize>()
            {
                match kind.trim().to_lowercase().as_str() {
                    "p" | "perm" | "permanent" => {
                        if idx < ctx.eligible_permanents.len() {
                            permanents.push(ctx.eligible_permanents[idx].0);
                        }
                    }
                    "o" | "player" => {
                        if idx < ctx.eligible_players.len() {
                            players.push(ctx.eligible_players[idx].0);
                        }
                    }
                    _ => {}
                }
            }
        }

        crate::decisions::specs::ProliferateResponse {
            permanents,
            players,
        }
    }

    fn decide_priority(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision priority[{}]: {} actions | input '{}'",
                self.index.saturating_sub(1),
                ctx.actions.len(),
                input
            );
        }

        // Empty input means pass priority
        if trimmed.is_empty() {
            return LegalAction::PassPriority;
        }

        // Check for commander action (C, c, C0, c0, C1, c1, etc.)
        let lower = trimmed.to_lowercase();
        let commander_indices = commander_action_indices(&ctx.actions);
        if lower == "c" && commander_indices.len() == 1 {
            return ctx.actions[commander_indices[0]].clone();
        }
        if lower.starts_with('c')
            && let Ok(idx) = lower[1..].parse::<usize>()
            && idx < commander_indices.len()
        {
            return ctx.actions[commander_indices[idx]].clone();
        }

        // Parse as index
        if let Ok(idx) = trimmed.parse::<usize>()
            && idx < ctx.actions.len()
        {
            return ctx.actions[idx].clone();
        }

        // Fallback to pass
        LegalAction::PassPriority
    }

    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        let input = self.next_input();
        let trimmed = input.trim();

        if self.debug {
            eprintln!(
                "Decision targets[{}]: {} requirements | input '{}'",
                self.index.saturating_sub(1),
                ctx.requirements.len(),
                input
            );
            for (i, req) in ctx.requirements.iter().enumerate() {
                eprintln!(
                    "  target requirement[{}]: {} legal targets",
                    i,
                    req.legal_targets.len()
                );
            }
        }

        // Each requirement gets a target selection
        // Input format: "target_idx" for single target, or "idx1,idx2,..." for multiple requirements
        let mut targets = Vec::new();
        let indices: Vec<usize> = trimmed
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .collect();

        for (req_idx, req) in ctx.requirements.iter().enumerate() {
            if req.min_targets == 0 && (indices.is_empty() || req_idx >= indices.len()) {
                // Optional target with no input, skip
                continue;
            }

            // Get the target index for this requirement
            let target_idx = indices.get(req_idx).copied().unwrap_or(0);

            // Select the target at that index
            if target_idx < req.legal_targets.len() {
                targets.push(req.legal_targets[target_idx]);
            } else if !req.legal_targets.is_empty() && req.min_targets > 0 {
                // Fallback to first legal target if required
                targets.push(req.legal_targets[0]);
            }
        }

        targets
    }
}

// ============================================================================
// CLI Decision Maker
// ============================================================================

/// A decision maker that prompts the user via CLI.
pub struct CliDecisionMaker;

impl DecisionMaker for CliDecisionMaker {
    fn on_auto_pass(&mut self, game: &GameState, player: PlayerId) {
        let phase = format_phase(&game.turn.phase, &game.turn.step);
        println!("({} auto-passes: {})", player_name(game, player), phase);
    }

    fn on_action_cancelled(&mut self, _game: &GameState, reason: &str) {
        println!("\n*** Action cancelled: {} ***", reason);
        println!("(Game state restored to before the action started)\n");
    }

    fn decide_priority(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        display_game_state(game);
        println!("\n--- {} has priority ---", player_name(game, ctx.player));
        prompt_priority_action(game, &ctx.actions)
    }

    fn decide_boolean(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        let source_info = if let Some(name) = &ctx.source_name {
            format!(" ({})", name)
        } else if let Some(source_id) = ctx.source {
            game.object(source_id)
                .map(|o| format!(" ({})", o.name))
                .unwrap_or_default()
        } else {
            String::new()
        };
        println!(
            "\n--- {} chooses{}: {} ---",
            player_name(game, ctx.player),
            source_info,
            ctx.description
        );
        prompt_boolean_choice()
    }

    fn decide_number(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        let source_info = ctx
            .source
            .and_then(|id| game.object(id))
            .map(|o| format!(" for {}", o.name))
            .unwrap_or_default();
        println!(
            "\n--- {} chooses a number{} ---",
            player_name(game, ctx.player),
            source_info
        );
        println!("{}", ctx.description);
        prompt_number_choice(ctx.min, ctx.max)
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        println!(
            "\n--- {} selects objects ---",
            player_name(game, ctx.player)
        );
        println!("{}", ctx.description);
        prompt_select_objects(
            game,
            &ctx.candidates,
            ctx.min,
            ctx.max,
            ctx.allow_partial_completion,
        )
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        println!(
            "\n--- {} chooses option(s) ---",
            player_name(game, ctx.player)
        );
        println!("{}", ctx.description);
        prompt_select_options(&ctx.options, ctx.min, ctx.max)
    }

    fn view_cards(
        &mut self,
        game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        let viewer_name = player_name(game, viewer);
        let subject_name = player_name(game, ctx.subject);
        let zone_label = ctx.zone.to_string();

        println!(
            "\n--- {} looks at {}'s {} ---",
            viewer_name, subject_name, zone_label
        );
        println!("{}", ctx.description);

        if cards.is_empty() {
            println!("(no cards)");
            return;
        }

        for (idx, card_id) in cards.iter().enumerate() {
            let name = game
                .object(*card_id)
                .map(|obj| obj.name.clone())
                .unwrap_or_else(|| format!("Unknown ({})", card_id.0));
            println!("{}. {}", idx + 1, name);
        }
    }

    fn decide_order(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        println!("\n--- {} orders items ---", player_name(game, ctx.player));
        println!("{}", ctx.description);
        // For simplicity, use the default order (items as given)
        ctx.items.iter().map(|(id, _)| *id).collect()
    }

    fn decide_attackers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        display_game_state(game);
        println!(
            "\n--- {} declares attackers ---",
            player_name(game, ctx.player)
        );
        prompt_declare_attackers(game, ctx)
    }

    fn decide_blockers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        display_game_state(game);
        println!(
            "\n--- {} declares blockers ---",
            player_name(game, ctx.player)
        );
        prompt_declare_blockers(game, ctx)
    }

    fn decide_distribute(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        println!(
            "\n--- {} distributes {} ---",
            player_name(game, ctx.player),
            ctx.total
        );
        println!("{}", ctx.description);
        prompt_distribute(game, &ctx.targets, ctx.total, ctx.min_per_target)
    }

    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        println!(
            "\n--- {} chooses {} mana color(s){} ---",
            player_name(game, ctx.player),
            ctx.count,
            if ctx.same_color {
                " (must be same)"
            } else {
                ""
            }
        );
        prompt_choose_colors(ctx.count, ctx.same_color)
    }

    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(CounterType, u32)> {
        let target_name = game
            .object(ctx.target)
            .map(|o| o.name.as_str())
            .unwrap_or("permanent");
        println!(
            "\n--- {} chooses counters to remove from {} (up to {} total) ---",
            player_name(game, ctx.player),
            target_name,
            ctx.max_total
        );
        prompt_choose_counters(&ctx.available_counters, ctx.max_total)
    }

    fn decide_partition(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        println!(
            "\n--- {} partitions {} card(s) ---",
            player_name(game, ctx.player),
            ctx.cards.len()
        );
        println!("{}", ctx.description);
        prompt_partition(game, &ctx.cards, &ctx.primary_label, &ctx.secondary_label)
    }

    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        println!(
            "\n--- {} chooses proliferate targets ---",
            player_name(game, ctx.player)
        );
        prompt_proliferate(game, &ctx.eligible_permanents, &ctx.eligible_players)
    }

    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        let source_name = game
            .object(ctx.source)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell/ability".to_string());
        println!(
            "\n--- {} chooses targets for {} ---",
            player_name(game, ctx.player),
            source_name
        );
        prompt_choose_targets(game, &ctx.requirements)
    }
}

fn player_name(game: &GameState, player: PlayerId) -> &str {
    game.player(player)
        .map(|p| p.name.as_str())
        .unwrap_or("Unknown")
}

fn display_game_state(game: &GameState) {
    println!(
        "\n=== Turn {}: {} ({}) ===",
        game.turn.turn_number,
        player_name(game, game.turn.active_player),
        format_phase(&game.turn.phase, &game.turn.step)
    );

    // Display players side by side
    for player in &game.players {
        let status = if player.has_lost {
            " [LOST]"
        } else if player.has_won {
            " [WON]"
        } else {
            ""
        };
        let mana = format_mana_pool(&player.mana_pool);
        print!(
            "[{}]{} Life:{} Mana:{} Hand:{} Lib:{} | ",
            player.name,
            status,
            player.life,
            mana,
            player.hand.len(),
            player.library.len()
        );
    }
    println!();

    // Show active player's hand compactly
    let active = game.turn.active_player;
    if let Some(player) = game.player(active) {
        let hand: Vec<String> = player
            .hand
            .iter()
            .filter_map(|&id| {
                game.object(id)
                    .map(|o| format!("{}({})", o.name, format_mana_cost(o)))
            })
            .collect();
        if !hand.is_empty() {
            println!("Hand: {}", hand.join(", "));
        }
    }

    // Show graveyards compactly (if non-empty)
    for player in &game.players {
        if !player.graveyard.is_empty() {
            let gy: Vec<String> = player
                .graveyard
                .iter()
                .filter_map(|&id| game.object(id).map(|o| o.name.clone()))
                .collect();
            println!(
                "{}'s Graveyard ({}): {}",
                player.name,
                gy.len(),
                gy.join(", ")
            );
        }
    }

    // Display battlefield compactly
    if !game.battlefield.is_empty() {
        let perms: Vec<String> = game
            .battlefield
            .iter()
            .filter_map(|&id| {
                game.object(id).map(|obj| {
                    let tapped = if game.is_tapped(id) { "[T]" } else { "" };
                    let pt = if game.current_is_creature(id) {
                        // Use calculated power/toughness (includes +1/+1 counters, anthems, etc.)
                        let power = game.current_power(id).unwrap_or(0);
                        let toughness = game.current_toughness(id).unwrap_or(0);
                        format!(" {}/{}", power, toughness)
                    } else {
                        String::new()
                    };
                    format!(
                        "{}{}{}({})",
                        obj.name,
                        pt,
                        tapped,
                        player_name(game, game.controller_of(obj))
                            .chars()
                            .next()
                            .unwrap_or('?')
                    )
                })
            })
            .collect();
        println!("Field: {}", perms.join(", "));
    }

    // Display stack compactly
    if !game.stack.is_empty() {
        let stack: Vec<String> = game
            .stack
            .iter()
            .rev()
            .map(|entry| {
                // Use source_name if available (for abilities), otherwise look up the object
                if entry.is_ability {
                    if let Some(name) = &entry.source_name {
                        format!("{} (ability)", name)
                    } else if let Some(obj) = game.object(entry.object_id) {
                        format!("{} (ability)", obj.name)
                    } else {
                        "[Triggered Ability]".to_string()
                    }
                } else if let Some(obj) = game.object(entry.object_id) {
                    obj.name.clone()
                } else {
                    "[Unknown]".to_string()
                }
            })
            .collect();
        println!("Stack: {}", stack.join(" -> "));
    }
}

fn format_phase(phase: &Phase, step: &Option<Step>) -> String {
    let phase_str = match phase {
        Phase::Beginning => "Beginning",
        Phase::FirstMain => "Precombat Main",
        Phase::Combat => "Combat",
        Phase::NextMain => "Postcombat Main",
        Phase::Ending => "Ending",
    };

    if let Some(step) = step {
        let step_str = match step {
            Step::Untap => "Untap",
            Step::Upkeep => "Upkeep",
            Step::Draw => "Draw",
            Step::BeginCombat => "Begin Combat",
            Step::DeclareAttackers => "Declare Attackers",
            Step::DeclareBlockers => "Declare Blockers",
            Step::CombatDamage => "Combat Damage",
            Step::EndCombat => "End Combat",
            Step::End => "End",
            Step::Cleanup => "Cleanup",
        };
        format!("{} - {}", phase_str, step_str)
    } else {
        phase_str.to_string()
    }
}

fn format_mana_pool(pool: &crate::ManaPool) -> String {
    let mut parts = Vec::new();
    if pool.white > 0 {
        parts.push(format!("{}W", pool.white));
    }
    if pool.blue > 0 {
        parts.push(format!("{}U", pool.blue));
    }
    if pool.black > 0 {
        parts.push(format!("{}B", pool.black));
    }
    if pool.red > 0 {
        parts.push(format!("{}R", pool.red));
    }
    if pool.green > 0 {
        parts.push(format!("{}G", pool.green));
    }
    if pool.colorless > 0 {
        parts.push(format!("{}C", pool.colorless));
    }
    if parts.is_empty() {
        "empty".to_string()
    } else {
        parts.join(" ")
    }
}

fn format_mana_cost(obj: &crate::Object) -> String {
    if let Some(ref cost) = obj.mana_cost {
        let mut parts = Vec::new();
        for pip in cost.pips() {
            if pip.len() == 1 {
                parts.push(format_symbol(&pip[0]));
            } else {
                // Hybrid - show alternatives
                let alts: Vec<String> = pip.iter().map(format_symbol).collect();
                parts.push(format!("({})", alts.join("/")));
            }
        }
        if parts.is_empty() {
            "0".to_string()
        } else {
            parts.join("")
        }
    } else {
        "0".to_string()
    }
}

fn format_symbol(symbol: &ManaSymbol) -> String {
    match symbol {
        ManaSymbol::White => "W".to_string(),
        ManaSymbol::Blue => "U".to_string(),
        ManaSymbol::Black => "B".to_string(),
        ManaSymbol::Red => "R".to_string(),
        ManaSymbol::Green => "G".to_string(),
        ManaSymbol::Colorless => "C".to_string(),
        ManaSymbol::Generic(n) => n.to_string(),
        ManaSymbol::Snow => "S".to_string(),
        ManaSymbol::Life(n) => format!("P{}", n),
        ManaSymbol::X => "X".to_string(),
    }
}

fn format_mana_cost_from_cost(cost: &crate::ManaCost) -> String {
    let mut parts = Vec::new();
    for pip in cost.pips() {
        if pip.len() == 1 {
            parts.push(format_symbol(&pip[0]));
        } else {
            let alts: Vec<String> = pip.iter().map(format_symbol).collect();
            parts.push(format!("({})", alts.join("/")));
        }
    }
    if parts.is_empty() {
        "0".to_string()
    } else {
        parts.join("")
    }
}

fn format_non_mana_costs(costs: &[crate::costs::Cost]) -> String {
    if costs.is_empty() {
        return "Free".to_string();
    }
    costs
        .iter()
        .map(|cost| cost.display())
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

/// New version of prompt_priority_action that returns LegalAction directly
/// (used by the new decide_priority method).
fn prompt_priority_action(game: &GameState, actions: &[LegalAction]) -> LegalAction {
    let commander_indices = commander_action_indices(actions);

    // Format actions compactly
    let action_strs: Vec<String> = actions
        .iter()
        .enumerate()
        .map(|(i, a)| format!("{}:{}", i, format_action_short(game, a)))
        .collect();
    println!("Actions: {}", action_strs.join(" | "));

    // Display commander actions separately with 'C' prefix
    if !commander_indices.is_empty() {
        let commander_strs: Vec<String> = commander_indices
            .iter()
            .enumerate()
            .map(|(i, action_index)| {
                let action = &actions[*action_index];
                if commander_indices.len() == 1 {
                    format!("C:{}", format_action_short(game, action))
                } else {
                    format!("C{}:{}", i, format_action_short(game, action))
                }
            })
            .collect();
        println!("Commander: {}", commander_strs.join(" | "));
    }

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        // Empty input = pass priority if available
        if trimmed.is_empty()
            && let Some(pass_action) = actions
                .iter()
                .find(|a| matches!(a, LegalAction::PassPriority))
        {
            return pass_action.clone();
        }

        // Check for commander action (C, c, C0, c0, C1, c1, etc.)
        let lower = trimmed.to_lowercase();
        if lower == "c" && commander_indices.len() == 1 {
            return actions[commander_indices[0]].clone();
        }
        if lower.starts_with('c')
            && let Ok(idx) = lower[1..].parse::<usize>()
            && idx < commander_indices.len()
        {
            return actions[commander_indices[idx]].clone();
        }

        if let Ok(idx) = trimmed.parse::<usize>()
            && idx < actions.len()
        {
            return actions[idx].clone();
        }
        println!("Invalid (0-{})", actions.len() - 1);
    }
}

fn zone_label(zone: Zone) -> &'static str {
    match zone {
        Zone::Battlefield => "battlefield",
        Zone::Hand => "hand",
        Zone::Library => "library",
        Zone::Graveyard => "graveyard",
        Zone::Exile => "exile",
        Zone::Stack => "stack",
        Zone::Command => "command zone",
        Zone::OutsideGame => "outside the game",
    }
}

pub(crate) fn format_action_short(game: &GameState, action: &LegalAction) -> String {
    match action {
        LegalAction::PassPriority => "Pass".to_string(),
        LegalAction::KeepOpeningHand => "Keep hand".to_string(),
        LegalAction::TakeMulligan => "Mulligan".to_string(),
        LegalAction::ContinuePregame | LegalAction::BeginGame => "Pregame".to_string(),
        LegalAction::UsePregameAction { card_id, .. } => {
            let name = game
                .object(*card_id)
                .map(|o| o.name.as_str())
                .unwrap_or("pregame card");
            format!("Begin with {}", name)
        }
        LegalAction::PlayLand { land_id } => {
            let name = game.object(*land_id).map_or_else(
                || "?".to_string(),
                |object| {
                    crate::decision::linked_other_face_land_definition(game, object)
                        .map(|def| def.card.name)
                        .unwrap_or_else(|| object.name.clone())
                },
            );
            format!("Play {}", name)
        }
        LegalAction::CastSpell {
            spell_id,
            casting_method,
            ..
        } => {
            if let Some(obj) = game.object(*spell_id) {
                match casting_method {
                    crate::alternative_cast::CastingMethod::Normal => {
                        format!("{} ({})", obj.name, format_mana_cost(obj))
                    }
                    crate::alternative_cast::CastingMethod::FaceDown => {
                        format!("{} [Face down] ({})", obj.name, "{3}")
                    }
                    crate::alternative_cast::CastingMethod::SplitOtherHalf => {
                        if let Some(other_def) = game.linked_face_definition_by_name_or_id(
                            obj.other_face_name.as_deref(),
                            obj.other_face,
                        ) {
                            let cost = other_def
                                .card
                                .mana_cost
                                .as_ref()
                                .map(format_mana_cost_from_cost)
                                .unwrap_or_else(|| "0".to_string());
                            format!("{} ({})", other_def.card.name, cost)
                        } else {
                            format!("{} [other half]", obj.name)
                        }
                    }
                    crate::alternative_cast::CastingMethod::SplitOtherHalfPlayFrom {
                        zone,
                        use_alternative,
                        ..
                    } => {
                        let acting_player = game.turn.priority_player.unwrap_or(obj.owner);
                        let alt_method = resolve_play_from_alternative_method(
                            game,
                            acting_player,
                            obj,
                            *zone,
                            *use_alternative,
                        );
                        let method_name = alt_method
                            .as_ref()
                            .map(|method| method.name())
                            .unwrap_or("Alternative");
                        let cost_desc = if let Some(method) = alt_method.as_ref() {
                            let costs = method.non_mana_costs();
                            if !costs.is_empty() {
                                let effects_desc = format_non_mana_costs(&costs);
                                if let Some(mana) = method.mana_cost() {
                                    format!(
                                        "{}, {}",
                                        format_mana_cost_from_cost(mana),
                                        effects_desc
                                    )
                                } else {
                                    effects_desc
                                }
                            } else if let Some(mana_cost) = method.mana_cost() {
                                format_mana_cost_from_cost(mana_cost)
                            } else {
                                format_mana_cost(obj)
                            }
                        } else {
                            format_mana_cost(obj)
                        };
                        let name = game
                            .linked_face_definition_by_name_or_id(
                                obj.other_face_name.as_deref(),
                                obj.other_face,
                            )
                            .map(|other_def| other_def.card.name)
                            .unwrap_or_else(|| obj.name.clone());
                        format!(
                            "{} [from {}, {}] ({})",
                            name,
                            zone_label(*zone),
                            method_name,
                            cost_desc
                        )
                    }
                    crate::alternative_cast::CastingMethod::Fuse => {
                        let fused_cost = spell_mana_cost_for_cast(
                            game,
                            game.turn.priority_player.unwrap_or(obj.owner),
                            obj,
                            casting_method,
                            Zone::Hand,
                        )
                        .as_ref()
                        .map(format_mana_cost_from_cost)
                        .unwrap_or_else(|| format_mana_cost(obj));
                        if let Some(other_def) = game.linked_face_definition_by_name_or_id(
                            obj.other_face_name.as_deref(),
                            obj.other_face,
                        ) {
                            format!(
                                "{} // {} [Fuse] ({})",
                                obj.name, other_def.card.name, fused_cost
                            )
                        } else {
                            format!("{} [Fuse] ({})", obj.name, fused_cost)
                        }
                    }
                    crate::alternative_cast::CastingMethod::Alternative(idx) => {
                        // Get the alternative cost description
                        if let Some(alt_method) = obj.alternative_casts.get(*idx) {
                            let costs = alt_method.non_mana_costs();
                            let cost_desc = if !costs.is_empty() {
                                let effects_desc = format_non_mana_costs(&costs);
                                if let Some(mana) = alt_method.mana_cost() {
                                    format!(
                                        "{}, {}",
                                        format_mana_cost_from_cost(mana),
                                        effects_desc
                                    )
                                } else {
                                    effects_desc
                                }
                            } else if let Some(mana_cost) = alt_method.mana_cost() {
                                // For flashback/escape/etc., show the mana cost
                                format_mana_cost_from_cost(mana_cost)
                            } else {
                                format_mana_cost(obj)
                            };
                            if matches!(
                                alt_method,
                                crate::alternative_cast::AlternativeCastingMethod::Disturb { .. }
                            ) && let Some(other_def) = game.linked_face_definition_by_name_or_id(
                                obj.other_face_name.as_deref(),
                                obj.other_face,
                            ) {
                                return format!(
                                    "{} [{}] ({})",
                                    other_def.card.name,
                                    alt_method.name(),
                                    cost_desc
                                );
                            }
                            format!("{} [{}] ({})", obj.name, alt_method.name(), cost_desc)
                        } else {
                            format!("{} [Alt] ({})", obj.name, format_mana_cost(obj))
                        }
                    }
                    crate::alternative_cast::CastingMethod::GrantedEscape { .. } => {
                        format!("{} [Escape] ({})", obj.name, format_mana_cost(obj))
                    }
                    crate::alternative_cast::CastingMethod::GrantedFlashback => {
                        format!("{} [Flashback] ({})", obj.name, format_mana_cost(obj))
                    }
                    crate::alternative_cast::CastingMethod::PlayFrom {
                        zone,
                        use_alternative: None,
                        ..
                    } => {
                        format!(
                            "{} [from {}] ({})",
                            obj.name,
                            zone_label(*zone),
                            format_mana_cost(obj)
                        )
                    }
                    crate::alternative_cast::CastingMethod::PlayFrom {
                        zone,
                        use_alternative: Some(idx),
                        ..
                    } => {
                        let acting_player = game.turn.priority_player.unwrap_or(obj.owner);
                        if let Some(alt_method) = resolve_play_from_alternative_method(
                            game,
                            acting_player,
                            obj,
                            *zone,
                            *idx,
                        ) {
                            let costs = alt_method.non_mana_costs();
                            let cost_desc = if !costs.is_empty() {
                                let effects_desc = format_non_mana_costs(&costs);
                                if let Some(mana) = alt_method.mana_cost() {
                                    format!(
                                        "{}, {}",
                                        format_mana_cost_from_cost(mana),
                                        effects_desc
                                    )
                                } else {
                                    effects_desc
                                }
                            } else if let Some(mana_cost) = alt_method.mana_cost() {
                                format_mana_cost_from_cost(mana_cost)
                            } else {
                                format_mana_cost(obj)
                            };
                            format!(
                                "{} [from {}, {}] ({})",
                                obj.name,
                                zone_label(*zone),
                                alt_method.name(),
                                cost_desc
                            )
                        } else {
                            format!(
                                "{} [from {}, Alt] ({})",
                                obj.name,
                                zone_label(*zone),
                                format_mana_cost(obj)
                            )
                        }
                    }
                }
            } else {
                "Cast".to_string()
            }
        }
        LegalAction::ActivateAbility { source, .. } => {
            let name = game.object(*source).map(|o| o.name.as_str()).unwrap_or("?");
            format!("Activate {}", name)
        }
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => {
            let name = game.object(*source).map(|o| o.name.as_str()).unwrap_or("?");

            // Check if this ability requires tapping
            if let (Some(ability), Some(object)) = (
                game.current_ability(*source, *ability_index),
                game.object(*source),
            ) {
                let controller = game.controller_of(object);
                if let crate::AbilityKind::Activated(mana_ability) = &ability.kind
                    && mana_ability.is_runtime_mana_ability(game, *source, controller)
                {
                    if mana_ability.has_tap_cost() {
                        format!("Tap {}", name)
                    } else {
                        format!("Activate {}", name)
                    }
                } else {
                    format!("Activate {}", name)
                }
            } else {
                format!("Tap {}", name)
            }
        }
        LegalAction::TurnFaceUp {
            creature_id,
            method,
        } => {
            let name = game
                .object(*creature_id)
                .map(|o| o.name.as_str())
                .unwrap_or("?");
            let cost_prefix =
                crate::special_actions::turn_face_up_cost_display(game, *creature_id, *method)
                    .map(|cost| format!("{cost}: "))
                    .unwrap_or_default();
            format!("{cost_prefix}Turn this face-down permanent face up. ({name})")
        }
        LegalAction::SpecialAction(special) => match special {
            crate::special_actions::SpecialAction::PlayLand { .. } => "Play land".to_string(),
            crate::special_actions::SpecialAction::TurnFaceUp {
                permanent_id,
                method,
            } => {
                let cost_prefix =
                    crate::special_actions::turn_face_up_cost_display(game, *permanent_id, *method)
                        .map(|cost| format!("{cost}: "))
                        .unwrap_or_default();
                format!("{cost_prefix}Turn this face-down permanent face up.")
            }
            crate::special_actions::SpecialAction::Suspend { .. } => "Suspend".to_string(),
            crate::special_actions::SpecialAction::Foretell { .. } => "Foretell".to_string(),
            crate::special_actions::SpecialAction::Plot { .. } => "Plot".to_string(),
            crate::special_actions::SpecialAction::ActivateManaAbility { .. } => {
                "Activate mana ability".to_string()
            }
        },
    }
}

fn prompt_declare_attackers(
    _game: &GameState,
    ctx: &crate::decisions::context::AttackersContext,
) -> Vec<crate::decisions::spec::AttackerDeclaration> {
    if ctx.attacker_options.is_empty() {
        println!("No creatures can attack.");
        return Vec::new();
    }

    println!("\nCreatures that can attack:");
    for (i, opt) in ctx.attacker_options.iter().enumerate() {
        let must = if opt.must_attack {
            " [MUST ATTACK]"
        } else {
            ""
        };
        println!("  {}: {}{}", i, opt.creature_name, must);
    }

    println!("\nEnter attacking creatures (comma-separated indices, or empty for none):");
    print!("> ");
    io::stdout().flush().unwrap();

    let input = read_input().unwrap_or_default();
    let input = input.trim();

    if input.is_empty() {
        return Vec::new();
    }

    let mut declarations = Vec::new();
    for part in input.split(',') {
        if let Ok(idx) = part.trim().parse::<usize>()
            && idx < ctx.attacker_options.len()
        {
            // Default to attacking the first opponent
            if let Some(target) = ctx.attacker_options[idx].valid_targets.first() {
                declarations.push(crate::decisions::spec::AttackerDeclaration {
                    creature: ctx.attacker_options[idx].creature,
                    target: target.clone(),
                });
            }
        }
    }

    declarations
}

fn prompt_declare_blockers(
    _game: &GameState,
    ctx: &crate::decisions::context::BlockersContext,
) -> Vec<crate::decisions::spec::BlockerDeclaration> {
    if ctx.blocker_options.is_empty() {
        println!("No attackers to block.");
        return Vec::new();
    }

    // Build a list of all valid blockers (creatures that can block at least one attacker)
    let mut all_valid_blockers: Vec<crate::ObjectId> = Vec::new();
    for opt in &ctx.blocker_options {
        for &(blocker_id, _) in &opt.valid_blockers {
            if !all_valid_blockers.contains(&blocker_id) {
                all_valid_blockers.push(blocker_id);
            }
        }
    }

    println!("\nAttackers:");
    for (i, opt) in ctx.blocker_options.iter().enumerate() {
        println!("  Attacker {}: {}", i, opt.attacker_name);
    }

    println!("\nAvailable blockers:");
    for (i, blocker_id) in all_valid_blockers.iter().enumerate() {
        let blocker_name = ctx
            .blocker_options
            .iter()
            .flat_map(|opt| opt.valid_blockers.iter())
            .find(|(id, _)| id == blocker_id)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| format!("Object {}", blocker_id.0));
        // Show which attackers this creature can block
        let can_block: Vec<usize> = ctx
            .blocker_options
            .iter()
            .enumerate()
            .filter(|(_, opt)| opt.valid_blockers.iter().any(|(id, _)| id == blocker_id))
            .map(|(i, _)| i)
            .collect();
        let can_block_text = if can_block.is_empty() {
            "none".to_string()
        } else {
            can_block
                .iter()
                .map(|idx| idx.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!(
            "  Blocker {}: {} (can block attackers: {})",
            i, blocker_name, can_block_text
        );
    }

    println!(
        "\nEnter blocks as 'blocker_idx:attacker_idx' pairs (comma-separated, or empty for none):"
    );
    println!(
        "Example: '0:0,1:0' means blocker 0 blocks attacker 0, blocker 1 also blocks attacker 0"
    );
    print!("> ");
    io::stdout().flush().unwrap();

    let input = read_input().unwrap_or_default();
    let input = input.trim();

    if input.is_empty() {
        return Vec::new();
    }

    let mut declarations = Vec::new();
    for part in input.split(',') {
        let part = part.trim();
        if let Some((blocker_str, attacker_str)) = part.split_once(':') {
            if let (Ok(blocker_idx), Ok(attacker_idx)) = (
                blocker_str.trim().parse::<usize>(),
                attacker_str.trim().parse::<usize>(),
            ) {
                // Validate indices
                if blocker_idx < all_valid_blockers.len()
                    && attacker_idx < ctx.blocker_options.len()
                {
                    let blocker_id = all_valid_blockers[blocker_idx];
                    let attacker_id = ctx.blocker_options[attacker_idx].attacker;

                    // Check if this blocker can actually block this attacker
                    if ctx.blocker_options[attacker_idx]
                        .valid_blockers
                        .iter()
                        .any(|(id, _)| *id == blocker_id)
                    {
                        declarations.push(crate::decisions::spec::BlockerDeclaration {
                            blocker: blocker_id,
                            blocking: attacker_id,
                        });
                    } else {
                        println!(
                            "Warning: Blocker {} cannot block attacker {}, skipping",
                            blocker_idx, attacker_idx
                        );
                    }
                } else {
                    println!("Warning: Invalid indices {}, skipping", part);
                }
            } else {
                println!("Warning: Could not parse '{}', skipping", part);
            }
        } else {
            println!(
                "Warning: Invalid format '{}', expected 'blocker:attacker'",
                part
            );
        }
    }

    declarations
}

// ============================================================================
// New typed prompt functions for primitive-specific DecisionMaker methods
// ============================================================================

/// Prompt for a boolean (yes/no) choice, returning bool directly.
fn prompt_boolean_choice() -> bool {
    loop {
        print!("Choose (y/n): ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" | "1" => return true,
            "n" | "no" | "0" => return false,
            _ => println!("Please enter 'y' or 'n'."),
        }
    }
}

/// Prompt for a number in a range, returning u32 directly.
fn prompt_number_choice(min: u32, max: u32) -> u32 {
    println!("Choose a number from {} to {}", min, max);
    loop {
        print!("Enter number: ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();

        if let Ok(n) = input.trim().parse::<u32>()
            && n >= min
            && n <= max
        {
            return n;
        }
        println!("Please enter a number between {} and {}.", min, max);
    }
}

/// Prompt for selecting objects from a list, returning Vec<ObjectId> directly.
fn prompt_select_objects(
    game: &GameState,
    candidates: &[crate::decisions::context::SelectableObject],
    min: usize,
    max: Option<usize>,
    allow_partial_completion: bool,
) -> Vec<ObjectId> {
    if candidates.is_empty() {
        return vec![];
    }

    println!("Selectable objects:");
    for (i, candidate) in candidates.iter().enumerate() {
        // Use the candidate's name if available, otherwise look it up
        let name = if candidate.name.is_empty() {
            game.object(candidate.id)
                .map(|o| o.name.as_str())
                .unwrap_or("?")
                .to_string()
        } else {
            candidate.name.clone()
        };
        let legal_marker = if candidate.legal { "" } else { " [ILLEGAL]" };
        println!("  {}: {}{}", i, name, legal_marker);
    }

    let max_display = max
        .map(|m| m.to_string())
        .unwrap_or_else(|| "any".to_string());
    if allow_partial_completion {
        println!(
            "Select up to {} objects (comma-separated indices, or empty for none):",
            max_display
        );
    } else {
        println!(
            "Select {} to {} objects (comma-separated indices, or empty for none):",
            min, max_display
        );
    }

    loop {
        print!("Selection: ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        // Handle empty input
        if trimmed.is_empty() {
            if min == 0 || allow_partial_completion {
                return vec![];
            }
            println!("Must select at least {} object(s).", min);
            continue;
        }

        // Parse comma-separated indices
        let mut selected = vec![];
        let mut valid = true;
        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>() {
                if idx < candidates.len() {
                    selected.push(candidates[idx].id);
                } else {
                    println!("Invalid index: {}", idx);
                    valid = false;
                    break;
                }
            } else {
                println!("Invalid input: {}", part);
                valid = false;
                break;
            }
        }

        if !valid {
            continue;
        }

        // Validate count
        if !allow_partial_completion && selected.len() < min {
            println!("Must select at least {} object(s).", min);
            continue;
        }
        if let Some(m) = max
            && selected.len() > m
        {
            println!("Cannot select more than {} object(s).", m);
            continue;
        }

        return selected;
    }
}

/// Prompt for selecting options by index, returning Vec<usize> directly.
fn prompt_select_options(
    options: &[crate::decisions::context::SelectableOption],
    min: usize,
    max: usize,
) -> Vec<usize> {
    println!("Available options:");
    let weighted = options.iter().any(|option| option.point_cost != 1);
    for opt in options {
        let legal_marker = if opt.legal { "" } else { " [ILLEGAL]" };
        let cost_marker = if weighted {
            format!(" [{} points]", opt.point_cost)
        } else {
            String::new()
        };
        println!(
            "  {}: {}{}{}",
            opt.index, opt.description, cost_marker, legal_marker
        );
    }

    if min == max && min == 1 {
        println!("Select one option:");
    } else {
        let unit = if weighted { "option point(s)" } else { "option(s)" };
        println!("Select {} to {} {} (comma-separated indices):", min, max, unit);
    }

    loop {
        print!("Selection: ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        // Parse indices
        let mut selected = vec![];
        let mut valid = true;

        if trimmed.is_empty() {
            if min == 0 {
                return vec![];
            }
            println!("Must select at least {} option(s).", min);
            continue;
        }

        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>() {
                // Find option with this index
                if options.iter().any(|o| o.index == idx && o.legal) {
                    selected.push(idx);
                } else {
                    println!("Invalid or illegal option: {}", idx);
                    valid = false;
                    break;
                }
            } else {
                println!("Invalid input: {}", part);
                valid = false;
                break;
            }
        }

        if !valid {
            continue;
        }

        // Validate point total. Unweighted options cost 1, preserving count-based behavior.
        let selected_total: usize = selected
            .iter()
            .filter_map(|idx| options.iter().find(|option| option.index == *idx))
            .map(|option| option.point_cost.max(1) as usize)
            .sum();
        if selected_total < min {
            println!("Must select at least {} option point(s).", min);
            continue;
        }
        if selected_total > max {
            println!("Cannot select more than {} option point(s).", max);
            continue;
        }

        return selected;
    }
}

/// Prompt for distributing an amount among targets, returning Vec<(Target, u32)> directly.
fn prompt_distribute(
    game: &GameState,
    targets: &[crate::decisions::context::DistributeTarget],
    total: u32,
    min_per_target: u32,
) -> Vec<(Target, u32)> {
    if targets.is_empty() {
        return vec![];
    }

    println!(
        "Distribute {} total (min {} per target):",
        total, min_per_target
    );
    for (i, target) in targets.iter().enumerate() {
        // Use the target's name if available, otherwise look it up
        let name = if !target.name.is_empty() {
            target.name.as_str()
        } else {
            match target.target {
                Target::Object(id) => game.object(id).map(|o| o.name.as_str()).unwrap_or("?"),
                Target::Player(pid) => game.player(pid).map(|p| p.name.as_str()).unwrap_or("?"),
            }
        };
        println!("  {}: {}", i, name);
    }

    // For simplicity, put all on the first target.
    // A full implementation would prompt for amounts per target
    if let Some(first) = targets.first() {
        vec![(first.target, total)]
    } else {
        vec![]
    }
}

/// Prompt for choosing colors, returning Vec<Color> directly.
fn prompt_choose_colors(count: u32, same_color: bool) -> Vec<crate::color::Color> {
    use crate::color::Color;

    println!("Choose {} color(s):", count);
    println!("  0: White");
    println!("  1: Blue");
    println!("  2: Black");
    println!("  3: Red");
    println!("  4: Green");

    let mut result = vec![];

    for i in 0..count {
        loop {
            print!("Color {}: ", i + 1);
            io::stdout().flush().unwrap();

            let input = read_input().unwrap_or_default();

            let normalized = input.trim().to_ascii_lowercase();
            let color = match normalized.as_str() {
                "0" => Some(Color::White),
                "1" => Some(Color::Blue),
                "2" => Some(Color::Black),
                "3" => Some(Color::Red),
                "4" => Some(Color::Green),
                _ => Color::from_mana_code_or_name(normalized.as_str()),
            };

            if let Some(c) = color {
                // Check same_color constraint
                if same_color && !result.is_empty() && result[0] != c {
                    println!("All colors must be the same.");
                    continue;
                }
                result.push(c);
                break;
            }
            println!("Invalid color. Please enter 0-4 or w/u/b/r/g.");
        }
    }

    result
}

/// Prompt for choosing counters to remove, returning Vec<(CounterType, u32)> directly.
fn prompt_choose_counters(
    available_counters: &[(CounterType, u32)],
    max_total: u32,
) -> Vec<(CounterType, u32)> {
    if available_counters.is_empty() {
        return vec![];
    }

    println!("Available counters (up to {} total to remove):", max_total);
    for (i, (counter_type, count)) in available_counters.iter().enumerate() {
        println!(
            "  {}: {} ({} available)",
            i,
            counter_type.description(),
            count
        );
    }

    println!("Enter index and amount pairs (e.g., '0:2,1:1' for 2 of type 0 and 1 of type 1):");
    println!("Or press enter to remove none.");

    loop {
        print!("Counters: ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return vec![];
        }

        let mut result = vec![];
        let mut total_removed = 0u32;
        let mut valid = true;

        for part in trimmed.split(',') {
            let parts: Vec<&str> = part.trim().split(':').collect();
            if parts.len() != 2 {
                println!("Invalid format. Use 'index:amount'.");
                valid = false;
                break;
            }

            let idx: usize = match parts[0].parse() {
                Ok(i) => i,
                Err(_) => {
                    println!("Invalid index: {}", parts[0]);
                    valid = false;
                    break;
                }
            };

            let amount: u32 = match parts[1].parse() {
                Ok(a) => a,
                Err(_) => {
                    println!("Invalid amount: {}", parts[1]);
                    valid = false;
                    break;
                }
            };

            if idx >= available_counters.len() {
                println!("Index {} out of range.", idx);
                valid = false;
                break;
            }

            if amount > available_counters[idx].1 {
                println!(
                    "Cannot remove {} counters, only {} available.",
                    amount, available_counters[idx].1
                );
                valid = false;
                break;
            }

            total_removed += amount;
            result.push((available_counters[idx].0, amount));
        }

        if !valid {
            continue;
        }

        if total_removed > max_total {
            println!("Total {} exceeds maximum {}.", total_removed, max_total);
            continue;
        }

        return result;
    }
}

/// Prompt for partitioning cards, returning Vec<ObjectId> for the secondary destination.
fn prompt_partition(
    game: &GameState,
    cards: &[(ObjectId, String)],
    primary_label: &str,
    secondary_label: &str,
) -> Vec<ObjectId> {
    if cards.is_empty() {
        return vec![];
    }

    println!("Cards to partition:");
    for (i, (id, name)) in cards.iter().enumerate() {
        let display_name = if name.is_empty() {
            game.object(*id)
                .map(|o| o.name.as_str())
                .unwrap_or("?")
                .to_string()
        } else {
            name.clone()
        };
        println!("  {}: {}", i, display_name);
    }

    println!(
        "Enter indices of cards to put on {} (comma-separated):",
        secondary_label
    );
    println!("Remaining cards go to {}.", primary_label);
    println!("Press enter to put all on {}.", primary_label);

    loop {
        print!("To {}: ", secondary_label);
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return vec![];
        }

        let mut result = vec![];
        let mut valid = true;

        for part in trimmed.split(',') {
            if let Ok(idx) = part.trim().parse::<usize>() {
                if idx < cards.len() {
                    result.push(cards[idx].0);
                } else {
                    println!("Invalid index: {}", idx);
                    valid = false;
                    break;
                }
            } else {
                println!("Invalid input: {}", part);
                valid = false;
                break;
            }
        }

        if valid {
            return result;
        }
    }
}

/// Prompt for proliferate targets, returning ProliferateResponse directly.
fn prompt_proliferate(
    game: &GameState,
    eligible_permanents: &[(ObjectId, String)],
    eligible_players: &[(PlayerId, String)],
) -> crate::decisions::specs::ProliferateResponse {
    println!("Eligible permanents:");
    for (i, (id, name)) in eligible_permanents.iter().enumerate() {
        let display_name = if name.is_empty() {
            game.object(*id)
                .map(|o| o.name.as_str())
                .unwrap_or("?")
                .to_string()
        } else {
            name.clone()
        };
        println!("  p{}: {}", i, display_name);
    }

    println!("Eligible players:");
    for (i, (id, name)) in eligible_players.iter().enumerate() {
        let display_name = if name.is_empty() {
            game.player(*id)
                .map(|p| p.name.as_str())
                .unwrap_or("?")
                .to_string()
        } else {
            name.clone()
        };
        println!("  P{}: {}", i, display_name);
    }

    println!("Enter targets to proliferate (e.g., 'p0,p2,P1' for permanents 0,2 and player 1):");
    println!("Press enter to proliferate nothing.");

    loop {
        print!("Proliferate: ");
        io::stdout().flush().unwrap();

        let input = read_input().unwrap_or_default();
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return crate::decisions::specs::ProliferateResponse::default();
        }

        let mut permanents = vec![];
        let mut players = vec![];
        let mut valid = true;

        for part in trimmed.split(',') {
            let part = part.trim();
            if part.starts_with('p') && part.len() > 1 {
                // Permanent
                if let Ok(idx) = part[1..].parse::<usize>() {
                    if idx < eligible_permanents.len() {
                        permanents.push(eligible_permanents[idx].0);
                    } else {
                        println!("Invalid permanent index: {}", idx);
                        valid = false;
                        break;
                    }
                } else {
                    println!("Invalid permanent: {}", part);
                    valid = false;
                    break;
                }
            } else if part.starts_with('P') && part.len() > 1 {
                // Player
                if let Ok(idx) = part[1..].parse::<usize>() {
                    if idx < eligible_players.len() {
                        players.push(eligible_players[idx].0);
                    } else {
                        println!("Invalid player index: {}", idx);
                        valid = false;
                        break;
                    }
                } else {
                    println!("Invalid player: {}", part);
                    valid = false;
                    break;
                }
            } else {
                println!(
                    "Invalid target: {}. Use p# for permanents, P# for players.",
                    part
                );
                valid = false;
                break;
            }
        }

        if valid {
            return crate::decisions::specs::ProliferateResponse {
                permanents,
                players,
            };
        }
    }
}

/// Prompt for target selection, returning Vec<Target> directly.
fn prompt_choose_targets(
    game: &GameState,
    requirements: &[crate::decisions::context::TargetRequirementContext],
) -> Vec<Target> {
    let mut selected_targets = Vec::new();

    for req in requirements.iter() {
        if req.min_targets == 0 && req.legal_targets.is_empty() {
            // Optional targeting with no legal targets - skip
            continue;
        }

        println!("Select target for: {}", req.description);
        println!("Available targets:");

        for (i, target) in req.legal_targets.iter().enumerate() {
            let display = match target {
                Target::Object(id) => {
                    if let Some(obj) = game.object(*id) {
                        let controller_name = game
                            .player(game.controller_of(obj))
                            .map(|p| p.name.chars().next().unwrap_or('?'))
                            .unwrap_or('?');
                        if game.current_is_creature(*id) {
                            let power = game.current_power(*id).unwrap_or(0);
                            let toughness = game.current_toughness(*id).unwrap_or(0);
                            format!("{} {}/{} ({})", obj.name, power, toughness, controller_name)
                        } else {
                            format!("{} ({})", obj.name, controller_name)
                        }
                    } else {
                        format!("Object #{}", id.0)
                    }
                }
                Target::Player(id) => game
                    .player(*id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| format!("Player {}", id.0)),
            };
            println!("  {}: {}", i, display);
        }

        let max_display = req
            .max_targets
            .map(|m| m.to_string())
            .unwrap_or_else(|| "any".to_string());
        println!(
            "Select {} to {} target(s) (enter index, or comma-separated for multiple):",
            req.min_targets, max_display
        );

        loop {
            print!("Selection: ");
            io::stdout().flush().unwrap();

            let input = read_input().unwrap_or_default();
            let trimmed = input.trim();

            // Handle empty input
            if trimmed.is_empty() {
                if req.min_targets == 0 {
                    // Optional targeting - skip this requirement
                    break;
                } else {
                    println!("Must select at least {} target(s).", req.min_targets);
                    continue;
                }
            }

            // Parse selected indices
            let mut valid = true;
            let mut req_targets = Vec::new();

            for part in trimmed.split(',') {
                if let Ok(idx) = part.trim().parse::<usize>() {
                    if idx < req.legal_targets.len() {
                        req_targets.push(req.legal_targets[idx]);
                    } else {
                        println!("Invalid index: {}", idx);
                        valid = false;
                        break;
                    }
                } else {
                    println!("Invalid input: {}", part);
                    valid = false;
                    break;
                }
            }

            if !valid {
                continue;
            }

            // Validate count
            if req_targets.len() < req.min_targets {
                println!(
                    "Must select at least {} target(s), got {}.",
                    req.min_targets,
                    req_targets.len()
                );
                continue;
            }
            if let Some(max) = req.max_targets
                && req_targets.len() > max
            {
                println!(
                    "Can select at most {} target(s), got {}.",
                    max,
                    req_targets.len()
                );
                continue;
            }

            selected_targets.extend(req_targets);
            break;
        }
    }

    selected_targets
}

/// Read a line using the global input manager.
/// In replay mode, exits the program when inputs are exhausted.
pub fn read_input() -> io::Result<String> {
    INPUT_MANAGER.with(|im| {
        let result = im.borrow_mut().read_line();
        if result.is_err() && im.borrow().is_replay_exhausted() {
            println!("\n=== Replay inputs exhausted, exiting ===");
            std::process::exit(0);
        }
        result
    })
}

// ============================================================================
// Input Manager for recording/replaying inputs
// ============================================================================

thread_local! {
    static INPUT_MANAGER: RefCell<InputManager> = RefCell::new(InputManager::new_interactive());
}

/// Manages input for the CLI - can read from stdin, record to file, or replay from file.
struct InputManager {
    mode: InputMode,
}

enum InputMode {
    /// Normal interactive mode - read from stdin
    Interactive,
    /// Record mode - read from stdin and write to file
    Record { file: BufWriter<File> },
    /// Replay mode - read from file
    Replay { lines: Vec<String>, index: usize },
}

impl InputManager {
    fn new_interactive() -> Self {
        Self {
            mode: InputMode::Interactive,
        }
    }

    fn new_record(path: &str) -> io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            mode: InputMode::Record {
                file: BufWriter::new(file),
            },
        })
    }

    fn new_replay(path: &str) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        // Keep empty lines (they're meaningful - e.g., "no attackers"), only skip comments
        let lines: Vec<String> = reader
            .lines()
            .map_while(Result::ok)
            .filter(|l| !l.trim().starts_with('#'))
            .collect();
        Ok(Self {
            mode: InputMode::Replay { lines, index: 0 },
        })
    }

    /// Read a line of input (from stdin or replay file).
    /// In record mode, also writes to the record file.
    fn read_line(&mut self) -> io::Result<String> {
        match &mut self.mode {
            InputMode::Interactive => {
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                Ok(input)
            }
            InputMode::Record { file } => {
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                // Write the trimmed input to the record file
                writeln!(file, "{}", input.trim())?;
                file.flush()?;
                Ok(input)
            }
            InputMode::Replay { lines, index } => {
                if *index < lines.len() {
                    let line = lines[*index].clone();
                    *index += 1;
                    // Print the replayed input for visibility
                    println!("{}", line);
                    Ok(format!("{}\n", line))
                } else {
                    // Out of replay inputs - return empty to trigger end
                    Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Replay inputs exhausted",
                    ))
                }
            }
        }
    }

    /// Check if we're in replay mode and have exhausted inputs.
    fn is_replay_exhausted(&self) -> bool {
        matches!(&self.mode, InputMode::Replay { lines, index } if *index >= lines.len())
    }
}

/// Initialize the global input manager.
pub fn init_input_manager(record_file: Option<&str>, replay_file: Option<&str>) {
    INPUT_MANAGER.with(|im| {
        let manager = if let Some(path) = replay_file {
            InputManager::new_replay(path).unwrap_or_else(|e| {
                eprintln!("Failed to open replay file '{}': {}", path, e);
                std::process::exit(1);
            })
        } else if let Some(path) = record_file {
            InputManager::new_record(path).unwrap_or_else(|e| {
                eprintln!("Failed to create record file '{}': {}", path, e);
                std::process::exit(1);
            })
        } else {
            InputManager::new_interactive()
        };
        *im.borrow_mut() = manager;
    });
}

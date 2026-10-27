//! A private, authenticated choice of a mana-eligible face-down cast.
pub use ironsmith_core::CastFaceDownUsingActivationManaEffect;
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, CompletedEffectOutputs};
use crate::game_state::{GameState, FaceDownCastPermission};
use crate::filter::ObjectFilterExt;
use crate::zone::Zone;
use super::runtime_helpers::{EffectDrivenCastOption, EffectDrivenCastPayment};
impl EffectExecutor for CastFaceDownUsingActivationManaEffect {
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> { Some(crate::effect::PriorEffectAction::Cast) }
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx).map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(game, ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)), |game, ctx| {
            let player = crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
            let mut filter = self.filter.clone();
            let mut budget = ctx.mana.activation_mana_budget.clone();
            // Generic activation taxes may be funded with any remaining units;
            // only a subset of at most the announced X can belong to X itself.
            budget.max_usable_units = Some(ctx.x_value.unwrap_or(0));
            filter.mana_cost_payable_with = Some(budget);
            let filter_ctx = ctx.filter_context(game);
            let hand = game.player(player).map(|player| player.hand.to_vec()).unwrap_or_default();
            let mut candidates = hand.iter().copied().filter(|id| game.object(*id).is_some_and(|card| filter.matches(card, &filter_ctx, game))).collect::<Vec<_>>();
            for id in game.hidden_hand_placeholder_candidates(&filter, &filter_ctx, hand) {
                if !candidates.contains(&id) { candidates.push(id); }
            }
            let spec = crate::decisions::specs::ChooseObjectsSpec::new(ctx.source,
                "Choose a creature card whose mana cost could be paid with the mana spent on this activation",
                candidates.clone(), 0, Some(1)).require_explicit_choice()
                .with_selection_reveal_policy(crate::decisions::context::SelectionRevealPolicy::None);
            let selected: Vec<crate::ids::ObjectId> = crate::decisions::make_decision(game, ctx.decision_maker, player, Some(ctx.source), spec);
            if ctx.decision_maker.awaiting_choice() { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); }
            let Some(card) = selected.into_iter().find(|id| candidates.contains(id)) else { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); };
            game.record_hidden_identity_obligations(&[card], &filter, &filter_ctx, "chosen card's mana cost is payable by retained activation mana");
            let accepted = ctx.decision_maker.decide_boolean(game, &crate::decisions::context::BooleanContext::new(player, Some(ctx.source), "Cast that card face down without paying its mana cost?"));
            if ctx.decision_maker.awaiting_choice() || !accepted { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); }
            let previous = game.face_down_cast_permissions().iter().filter(|permission| permission.source == ctx.source).cloned().collect::<Vec<_>>();
            game.grant_face_down_cast_permission(FaceDownCastPermission { source: ctx.source, player: player,
                zone: Zone::Hand, filter, description: "Cast the chosen mana-eligible card face down".into(),
                requires_source_on_battlefield: false, expires_after_turn: Some(game.turn.turn_number), single_use: true });
            game.set_hidden_face_down_cast_claim(card, crate::game_state::FaceDownCastKind::Permission { source: ctx.source });
            let option = EffectDrivenCastOption { object_id: card, from_zone: Zone::Hand,
                casting_method: crate::alternative_cast::CastingMethod::FaceDown, requirement: None, label: "Cast chosen card face down".into() };
            let cast = super::runtime_helpers::cast_effect_driven_spell_with_payment_policy(game, ctx, player,
                &option, EffectDrivenCastPayment::WithoutPayingManaCost, true)?;
            game.revoke_face_down_cast_permissions_from(ctx.source);
            for permission in previous { game.grant_face_down_cast_permission(permission); }
            if ctx.decision_maker.awaiting_choice() { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); }
            let Some(cast) = cast else { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); };
            if self.turn_face_up_before_action { game.register_turn_face_up_before_action(cast.new_id); }
            super::runtime_helpers::complete_effect_driven_cast_with_outputs(EffectOutcome::with_objects(vec![cast.new_id]), game, cast, player, ctx.provenance)
        })
    }
}

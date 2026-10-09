//! Duration-bound damage replacement created by a resolving spell/ability.
use crate::effect::EffectOutcome;
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::static_abilities::StaticAbilityKind;
pub type RegisterDamageMultiplierEffect = ironsmith_core::RegisterDamageMultiplierEffect;

/// A resolution-registered replacement outlives the resolution that names
/// "that player" or "that creature" (CR 611.2c, 614.1a): those references are
/// fixed to the objects and players they denote now, so the replacement keeps
/// watching them for its whole duration.
fn freeze_player_reference(
    game: &GameState,
    ctx: &ExecutionContext,
    filter: &crate::target::PlayerFilter,
) -> Result<crate::target::PlayerFilter, ExecutionError> {
    use crate::target::PlayerFilter;
    Ok(match filter {
        // "that player" after "Whenever ... deals combat damage to a player"
        // is the damaged player when no iteration names one.
        PlayerFilter::IteratedPlayer => PlayerFilter::Specific(
            crate::effects::helpers::resolve_player_filter(game, filter, ctx).or_else(|_| {
                crate::effects::helpers::resolve_player_filter(
                    game,
                    &PlayerFilter::DamagedPlayer,
                    ctx,
                )
            })?,
        ),
        PlayerFilter::Target(_)
        | PlayerFilter::AliasedTarget(_)
        | PlayerFilter::TaggedPlayer(_)
        | PlayerFilter::ChosenPlayer
        | PlayerFilter::DamagedPlayer => PlayerFilter::Specific(
            crate::effects::helpers::resolve_player_filter(game, filter, ctx)?,
        ),
        other => other.clone(),
    })
}

fn freeze_object_reference(
    game: &GameState,
    ctx: &ExecutionContext,
    filter: &crate::target::ObjectFilter,
) -> Result<crate::target::ObjectFilter, ExecutionError> {
    let mut frozen = filter.clone();
    if let Some(controller) = &filter.controller {
        frozen.controller = Some(freeze_player_reference(game, ctx, controller)?);
    }
    if let Some(owner) = &filter.owner {
        frozen.owner = Some(freeze_player_reference(game, ctx, owner)?);
    }
    frozen.any_of = filter.any_of.iter()
        .map(|branch| freeze_object_reference(game, ctx, branch))
        .collect::<Result<_, _>>()?;
    // Bind identity only. A relation such as "shares a color with that
    // creature" must not become "is that creature", and binding identity
    // must retain the source/recipient's other restrictions.
    if let [constraint] = filter.tagged_constraints.as_slice()
        && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        && let Some(snapshots) = ctx.get_tagged_all(constraint.tag.as_str())
        && let [snapshot] = snapshots.as_slice()
    {
        frozen.specific = Some(snapshot.object_id);
        frozen.tagged_constraints.clear();
    }
    Ok(frozen)
}

impl EffectExecutor for RegisterDamageMultiplierEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_filter = freeze_object_reference(game, ctx, &self.source_filter)?;
        let target_player_filter = self
            .target_player_filter
            .as_ref()
            .map(|filter| freeze_player_reference(game, ctx, filter))
            .transpose()?;
        let target_object_filter = self
            .target_object_filter
            .as_ref()
            .map(|filter| freeze_object_reference(game, ctx, filter))
            .transpose()?;
        if let Some(modifier) = self.amount_override {
            // A set or halved amount, through the shared amount-modifier
            // replacement (CR 616.1 ordering with any other modifiers).
            let replacement = crate::static_abilities::EventAmountReplacement::new(
                ironsmith_core::AmountEventSpec::Damage {
                    source_filter: Some(source_filter),
                    player: target_player_filter,
                    object: target_object_filter,
                    combat_only: self.combat_only,
                    minimum: self.minimum,
                },
                modifier,
                false,
                "Resolved damage amount replacement",
            )
            .generate_replacement_effect(ctx.source, ctx.controller)
            .expect("an amount replacement always creates a replacement");
            return ApplyReplacementEffect {
                effect: replacement,
                mode: self.mode,
            }
            .execute_child(game, ctx);
        }
        let mut ability = crate::static_abilities::DoubleDamageAmountReplacement::new(
            source_filter,
            target_player_filter,
            target_object_filter,
            self.factor,
            self.combat_only,
            "Resolved damage multiplier",
        );
        if self.noncombat_only {
            ability = ability.noncombat_only();
        }
        let replacement = ability
            .generate_replacement_effect(ctx.source, ctx.controller)
            .expect("damage multiplier always creates a replacement");
        ApplyReplacementEffect {
            effect: replacement,
            mode: self.mode,
        }
        .execute_child(game, ctx)
    }
    fn primary_execution_category(&self) -> crate::effects::EffectExecutionCategory {
        crate::effects::EffectExecutionCategory::ReplacementRegistration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CardId, GameState, PlayerId, Zone};
    use crate::target::ObjectFilter;

    #[test]
    fn frozen_identity_keeps_recipient_qualities() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = crate::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Reference")
            .card_types(vec![crate::CardType::Creature]).build();
        let source = game.create_object_from_definition(&card, alice, Zone::Battlefield);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_id(&game, source).unwrap();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_object("recipient", snapshot);
        let mut filter = ObjectFilter::tagged("recipient");
        filter.controller = Some(crate::target::PlayerFilter::You);
        filter.card_types.push(crate::CardType::Creature);
        let frozen = freeze_object_reference(&game, &ctx, &filter).unwrap();
        assert_eq!(frozen.specific, Some(source));
        assert_eq!(frozen.controller, filter.controller);
        assert_eq!(frozen.card_types, filter.card_types);
        assert!(frozen.tagged_constraints.is_empty());
    }
}

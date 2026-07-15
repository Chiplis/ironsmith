//! Phase-out effect implementation.

use crate::effect::{ChoiceCount, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{ObjectApplyResultPolicy, apply_to_selected_objects};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::target::SourceReferenceSurface;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

/// How long an effect keeps a permanent phased out.
pub type PhaseOutDuration = ironsmith_core::PhaseOutDuration;

fn source_known_and_not_on_battlefield(
    game: &GameState,
    source: ObjectId,
    source_snapshot: Option<&ObjectSnapshot>,
) -> bool {
    if let Some(source) = game.object(source) {
        return source.zone != Zone::Battlefield;
    }
    let Some(snapshot) = source_snapshot else {
        return false;
    };
    game.find_object_by_stable_id(snapshot.stable_id)
        .and_then(|current_id| game.object(current_id))
        .is_none_or(|source| source.zone != Zone::Battlefield)
}

/// Effect that phases permanents out.
#[derive(Debug, Clone, PartialEq)]
pub struct PhaseOutEffect {
    /// What to phase out - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
    /// When the permanent is allowed to phase in again.
    pub duration: PhaseOutDuration,
    /// Oracle-facing source wording for source-linked durations.
    pub source_surface: Option<SourceReferenceSurface>,
}

impl PhaseOutEffect {
    /// Create a phase-out effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self {
            spec,
            duration: PhaseOutDuration::UntilNextUntap,
            source_surface: None,
        }
    }

    /// Keep selected permanents phased out until the resolving ability's source leaves.
    pub fn until_source_leaves(mut self) -> Self {
        self.duration = PhaseOutDuration::UntilSourceLeaves;
        self
    }

    /// Preserve the printed way the duration refers to its source.
    pub fn with_source_surface(mut self, surface: SourceReferenceSurface) -> Self {
        self.source_surface = Some(surface);
        self
    }

    /// Create a targeted phase-out effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
            duration: PhaseOutDuration::UntilNextUntap,
            source_surface: None,
        }
    }

    /// Create a targeted phase-out effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
            duration: PhaseOutDuration::UntilNextUntap,
            source_surface: None,
        }
    }

    /// Create a non-targeted phase-out effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
            duration: PhaseOutDuration::UntilNextUntap,
            source_surface: None,
        }
    }

    /// Create a phase-out effect that phases out the source permanent.
    pub fn source() -> Self {
        Self {
            spec: ChooseSpec::Source,
            duration: PhaseOutDuration::UntilNextUntap,
            source_surface: None,
        }
    }
}

impl EffectExecutor for PhaseOutEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.duration == PhaseOutDuration::UntilSourceLeaves
            && source_known_and_not_on_battlefield(game, ctx.source, ctx.source_snapshot.as_ref())
        {
            return Ok(EffectOutcome::count(0));
        }
        let result_policy = if self.spec.is_target() && self.spec.is_single() {
            ObjectApplyResultPolicy::SingleTargetResolvedOrInvalid
        } else {
            ObjectApplyResultPolicy::CountApplied
        };

        let apply_result = apply_to_selected_objects(
            game,
            ctx,
            &self.spec,
            result_policy,
            |game, ctx, object_id| {
                if game
                    .object(object_id)
                    .is_some_and(|object| object.zone == Zone::Battlefield)
                    && !game.is_phased_out(object_id)
                    && game.can_phase_out(object_id)
                {
                    game.phase_out(object_id);
                    if self.duration == PhaseOutDuration::UntilSourceLeaves {
                        game.hold_phased_out_until_source_leaves(object_id, ctx.source);
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        )?;

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
        "permanent to phase out"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;

    #[test]
    fn phase_out_effect_respects_cant_phase_out_restriction() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(21), "Rooted Relic")
            .card_types(vec![CardType::Artifact])
            .build();
        let permanent_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.effect_store
            .cant_effects
            .cant_phase_out
            .insert(permanent_id);

        let mut ctx = ExecutionContext::new_default(permanent_id, alice);
        PhaseOutEffect::source()
            .execute(&mut game, &mut ctx)
            .expect("phase-out effect should resolve");

        assert!(
            !game.is_phased_out(permanent_id),
            "restricted permanent should not phase out"
        );
    }

    #[test]
    fn source_linked_phase_out_stays_held_then_releases_when_source_leaves() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::from_raw(22), "Fading Seal")
            .card_types(vec![CardType::Enchantment])
            .build();
        let creature_card = CardBuilder::new(CardId::from_raw(23), "Patient Bear")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let creature = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(source, alice);
        PhaseOutEffect::with_spec(ChooseSpec::Object(ObjectFilter::specific(creature)))
            .until_source_leaves()
            .with_source_surface(SourceReferenceSurface::ThisPermanentType(
                "this enchantment".to_string(),
            ))
            .execute(&mut game, &mut ctx)
            .expect("creature should phase out until the source leaves");
        assert!(game.is_phased_out(creature));
        assert!(
            !game
                .directly_phased_out_under(alice)
                .any(|candidate| candidate == creature),
            "held permanents must not phase in during their controller's untap step"
        );

        game.move_object_by_effect(source, Zone::Graveyard)
            .expect("source should leave the battlefield");
        assert!(
            !game.is_phased_out(creature),
            "held permanent should phase in as soon as the source leaves"
        );
    }
}

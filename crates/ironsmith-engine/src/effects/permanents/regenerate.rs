//! Regenerate effect implementation.

use crate::effect::{Effect, EffectOutcome, Until};
use crate::effects::{ApplyReplacementEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::events::permanents::matchers::RegenerationShieldMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::zone::Zone;
pub type RegenerateEffect = ironsmith_core::RegenerateEffect<crate::effect::Effect>;

/// Effect that regenerates a target creature.
///
/// Creates a "regeneration shield" as a one-shot replacement effect that lasts
/// for the specified duration. When the creature would be destroyed, instead:
/// - Tap it
/// - Remove all damage from it
/// - Remove it from combat (if applicable)
/// - The replacement effect is consumed
///
/// The regeneration shield is implemented as a proper replacement effect rather
/// than a counter, which aligns with the MTG rules and allows it to interact
/// correctly with other replacement effects.
///
/// # Fields
///
/// * `target` - The creature to regenerate
///
/// # Example
///
/// ```ignore
/// // Regenerate target creature
/// let effect = RegenerateEffect::new(ChooseSpec::creature(), Until::EndOfTurn);
///
/// // Regenerate this creature (source)
/// let effect = RegenerateEffect::source(Until::EndOfTurn);
/// ```
impl EffectExecutor for RegenerateEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        // The subject is bound to an exact incarnation at shield registration.
        let follow_up =
            bind_regeneration_follow_up(&self.follow_up_effects, self.follow_up_player.clone());
        for effect in regeneration_body(
            ChooseSpec::Source,
            &follow_up,
            !self.follow_up_effects.is_empty(),
        ) {
            visitor(&effect);
        }
    }

    fn visit_resolution_child_effects(&self, _visitor: &mut dyn FnMut(&Effect)) {
        // Shield payloads run only when destruction is replaced.
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.duration != Until::EndOfTurn {
            return Err(ExecutionError::Impossible(
                "RegenerateEffect currently supports only Until::EndOfTurn".to_string(),
            ));
        }

        // Resolve all matching targets. This supports both traditional
        // "target creature" regeneration and "regenerate each/all ..." forms.
        let targets = crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target)
            .map_err(|_| ExecutionError::InvalidTarget)?;
        if targets.is_empty() {
            return Err(ExecutionError::InvalidTarget);
        }

        // "that player" in a follow-up is fixed now, from this resolution's
        // targets and choices; the shield's program runs without them.
        let player = self
            .follow_up_player
            .as_ref()
            .filter(|_| !self.follow_up_effects.is_empty())
            .map(|filter| {
                crate::effects::helpers::resolve_player_filter(game, filter, ctx)
                    .map(crate::target::PlayerFilter::Specific)
            })
            .transpose()?;
        let follow_up_effects = bind_regeneration_follow_up(&self.follow_up_effects, player);

        let mut outcomes = Vec::new();
        for target_id in targets {
            // CR 701.19a: any permanent can be regenerated, not only creatures
            // (e.g. Welding Jar regenerating a noncreature artifact).
            let Some(obj) = game.object(target_id) else {
                continue;
            };
            if obj.zone != Zone::Battlefield {
                continue;
            }
            if !game.can_be_regenerated(target_id) {
                continue;
            }
            let controller = ctx.controller;

            // Follow-ups run in the replacement program with the shield's
            // controller and source; a reflexive "when it regenerates this
            // way" trigger keys off the damage removal's result id.
            let replacement_effects = regeneration_body(
                ChooseSpec::SpecificObject(target_id),
                &follow_up_effects,
                !self.follow_up_effects.is_empty(),
            );

            let matcher = RegenerationShieldMatcher::new(target_id);
            let replacement_effect = ReplacementEffect::with_matcher(
                target_id, // source is the creature itself
                controller,
                matcher,
                ReplacementAction::Instead(replacement_effects),
            );

            let apply = ApplyReplacementEffect::one_shot(replacement_effect);
            outcomes.push(execute_effect(game, &Effect::new(apply), ctx)?);
        }

        if outcomes.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }
        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature to regenerate"
    }
}

fn regeneration_body(target: ChooseSpec, follow_up: &[Effect], record_use: bool) -> Vec<Effect> {
    let clear = Effect::clear_damage(target.clone());
    let clear = if record_use {
        Effect::with_id(RegenerateEffect::SHIELD_USED_ID.0, clear)
    } else {
        clear
    };
    let mut effects = vec![
        Effect::tap(target.clone()).tag(TagKey::from("__it__")),
        clear,
        Effect::new(crate::effects::RemoveFromCombatEffect::with_spec(target)),
    ];
    effects.extend_from_slice(follow_up);
    effects
}

#[cfg(test)]
mod composition_tests {
    use super::*;

    #[test]
    fn regeneration_payload_is_visible_but_not_executed_at_registration() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = crate::ids::PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Subject")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let subject = game.create_object_from_card(&card, player, Zone::Battlefield);
        let mut effect =
            RegenerateEffect::new(ChooseSpec::SpecificObject(subject), Until::EndOfTurn);
        effect.follow_up_effects = vec![Effect::draw(1)];
        let mut structural = 0;
        effect.visit_child_effects(&mut |_| structural += 1);
        assert_eq!(structural, 4);
        let mut immediate = 0;
        effect.visit_resolution_child_effects(&mut |_| immediate += 1);
        assert_eq!(immediate, 0);
        game.create_object_from_card(&card, player, Zone::Library);
        let before = game.player(player).unwrap().hand.len();
        let mut ctx = ExecutionContext::new_default(subject, player);
        effect.execute(&mut game, &mut ctx).unwrap();
        assert!(!game.is_tapped(subject));
        assert_eq!(game.player(player).unwrap().hand.len(), before);
        let destroy = Effect::destroy(ChooseSpec::SpecificObject(subject));
        crate::effects::execute_effect(&mut game, &destroy, &mut ctx).unwrap();
        assert_eq!(game.object(subject).unwrap().zone, Zone::Battlefield);
        assert!(game.is_tapped(subject));
        assert_eq!(game.player(player).unwrap().hand.len(), before + 1);
        crate::effects::execute_effect(&mut game, &destroy, &mut ctx).unwrap();
        assert!(
            game.object(subject).is_none(),
            "the shield is consumed exactly once"
        );
    }
}

fn bind_regeneration_follow_up(
    effects: &[Effect],
    player: Option<crate::target::PlayerFilter>,
) -> Vec<Effect> {
    match player {
        Some(player) if !effects.is_empty() => vec![Effect::for_players(player, effects.to_vec())],
        _ => effects.to_vec(),
    }
}

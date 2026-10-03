//! Grant abilities to a target creature until a duration.

use crate::continuous::{EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::static_abilities::StaticAbility;
use crate::target::ChooseSpec;

/// Effect that grants one or more abilities to a target creature.
pub type GrantAbilitiesTargetEffect = ironsmith_core::GrantAbilitiesTargetEffect<StaticAbility>;

impl EffectExecutor for GrantAbilitiesTargetEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        for ability in &self.abilities {
            crate::ability::visit_static_owned_effects(ability, visitor);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if spec_names_player(&self.target) {
            return grant_player_protections(self, game, ctx);
        }
        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;
        if self.abilities.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let mut outcomes = Vec::new();
        for ability in &self.abilities {
            let apply = ApplyContinuousEffect::new(
                EffectTarget::Specific(target_id),
                Modification::AddAbility(ability.clone()),
                self.duration.clone(),
            );
            outcomes.push(execute_effect(game, &Effect::new(apply), ctx)?);
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}

fn spec_names_player(spec: &ChooseSpec) -> bool {
    matches!(
        spec.base(),
        ChooseSpec::SourceController
            | ChooseSpec::SourceOwner
            | ChooseSpec::Player(_)
            | ChooseSpec::SpecificPlayer(_)
            | ChooseSpec::EachPlayer(_)
    )
}

/// The sources a player's "protection from [quality]" covers, as a filter.
fn player_protection_source_filter(
    protection: &crate::ability::ProtectionFrom,
) -> Option<crate::target::ObjectFilter> {
    use crate::ability::ProtectionFrom;
    let mut filter = crate::target::ObjectFilter::default();
    match protection {
        ProtectionFrom::Color(colors) => filter.colors = Some(*colors),
        ProtectionFrom::AllColors => {
            filter.colors = Some(
                crate::color::Color::ALL
                    .into_iter()
                    .collect::<crate::color::ColorSet>(),
            )
        }
        ProtectionFrom::Colorless => filter.colorless = true,
        ProtectionFrom::Creatures => filter.card_types = vec![crate::types::CardType::Creature],
        ProtectionFrom::CardType(card_type) => filter.card_types = vec![*card_type],
        ProtectionFrom::Permanents(permanents) => filter = permanents.clone(),
        ProtectionFrom::Everything => {}
        _ => return None,
    }
    Some(filter)
}

/// "You gain protection from [quality] until ..." (Seht's Tiger): a player
/// can't have an object ability, so the grant becomes the player-protection
/// consequences (CR 702.16b/e): the player can't be the target of matching
/// sources, and damage from them is prevented, for the grant's duration.
fn grant_player_protections(
    effect: &GrantAbilitiesTargetEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    let players = match crate::effects::helpers::resolve_players_from_spec(game, &effect.target, ctx)
    {
        Ok(players) => players,
        Err(_) => return Ok(EffectOutcome::target_invalid()),
    };
    let mut outcomes = Vec::new();
    for player in players {
        for ability in &effect.abilities {
            let Some(source_filter) = ability
                .protection_from()
                .and_then(player_protection_source_filter)
            else {
                continue;
            };
            let cant = crate::effects::CantEffect::new(
                crate::effect::Restriction::be_targeted_player_from(
                    crate::target::PlayerFilter::Specific(player),
                    source_filter.clone(),
                ),
                effect.duration.clone(),
            );
            outcomes.push(execute_effect(game, &Effect::new(cant), ctx)?);
            let mut damage_filter = crate::prevention::DamageFilter::all();
            damage_filter.from_source = Some(source_filter);
            let prevent = crate::effects::PreventAllDamageToTargetEffect::new(
                ChooseSpec::SpecificPlayer(player),
                effect.duration.clone(),
            )
            .with_filter(damage_filter);
            outcomes.push(execute_effect(game, &Effect::new(prevent), ctx)?);
        }
    }
    if outcomes.is_empty() {
        return Ok(EffectOutcome::target_invalid());
    }
    Ok(EffectOutcome::aggregate(outcomes))
}

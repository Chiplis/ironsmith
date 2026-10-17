//! Grant abilities to a target creature until a duration.

use crate::continuous::{EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::static_abilities::StaticAbility;
use crate::target::ChooseSpec;

/// Effect that grants one or more abilities to a target creature.
pub type GrantAbilitiesTargetEffect = ironsmith_core::GrantAbilitiesTargetEffect<StaticAbility>;

impl EffectExecutor for GrantAbilitiesTargetEffect {
    fn own_preflight_object_specs(&self) -> Vec<ChooseSpec> {
        std::iter::once(self.target.clone()).chain(self.activated_from.iter().cloned()).collect()
    }
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
        // "Each Horror you control gains all activated abilities of target
        // artifact" (Grell Philosopher): every recipient in the set fixed at
        // resolution (CR 611.2c) receives the same frozen copies.
        let recipients = if self.activated_from.is_some()
            && matches!(self.target.base(), ChooseSpec::All(_))
        {
            crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target)?
        } else {
            vec![resolve_single_object_for_effect(game, ctx, &self.target)?]
        };
        if self.abilities.is_empty() && self.activated_from.is_none() {
            return Ok(EffectOutcome::resolved());
        }

        let abilities = expand_sacrificed_land_type_landwalks(&self.abilities, ctx);
        let mut modifications = Vec::new();
        if let Some(source) = &self.activated_from {
            let source_id = resolve_single_object_for_effect(game, ctx, source)?;
            game.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;
            // Freeze the abilities now. Losing the donor or changing its
            // abilities later does not change this resolving grant.
            let copied = game.current_abilities(source_id).unwrap_or_default()
                .into_iter().filter(|ability| matches!(ability.kind, crate::ability::AbilityKind::Activated(_)))
                .map(|ability| match self.activated_from_symbol_as_any_color {
                    Some(symbol) => with_symbol_paying_any_color(ability, symbol),
                    None => ability,
                })
                .collect::<Vec<_>>();
            modifications.extend(copied.into_iter().map(Modification::AddAbilityGeneric));
        }
        modifications.extend(abilities.into_iter().map(Modification::AddAbility));
        let mut outcome = EffectOutcome::resolved();
        for target_id in recipients {
            outcome = crate::effects::continuous::apply_captured_modifications(
                game, ctx, EffectTarget::Specific(target_id), None, modifications.clone(), self.duration.clone(),
            )?;
        }
        Ok(outcome)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }
}

/// "You may spend blue mana as though it were mana of any color to activate
/// those abilities" (CR 609.4b): in the copied activation costs, every
/// colored symbol may also be paid with mana of `symbol`. Only these copies
/// change, so the permission covers exactly "those abilities".
fn with_symbol_paying_any_color(
    mut ability: crate::ability::Ability,
    symbol: crate::mana::ManaSymbol,
) -> crate::ability::Ability {
    use crate::mana::ManaSymbol;
    let crate::ability::AbilityKind::Activated(activated) = &mut ability.kind else {
        return ability;
    };
    let widen = |cost: &crate::mana::ManaCost| -> Option<crate::mana::ManaCost> {
        let mut changed = false;
        let pips = cost.pips().iter().map(|pip| {
            let colored = pip.iter().any(|alternative| matches!(alternative,
                ManaSymbol::White | ManaSymbol::Blue | ManaSymbol::Black | ManaSymbol::Red | ManaSymbol::Green));
            let mut pip = pip.clone();
            if colored && !pip.contains(&symbol) {
                pip.push(symbol);
                changed = true;
            }
            pip
        }).collect::<Vec<_>>();
        changed.then(|| crate::mana::ManaCost::from_pips(pips))
    };
    activated.mana_cost = activated.mana_cost.clone().try_map(|cost| {
        let widened = cost.mana_cost_ref().and_then(|mana| widen(mana));
        Ok::<_, std::convert::Infallible>(match widened {
            Some(widened) => crate::costs::Cost::mana(widened),
            None => cost,
        })
    }).unwrap_or_else(|never| match never {});
    ability
}

/// "Target creature gains landwalk of each of the land types of the
/// sacrificed land until end of turn" (Excavator): one landwalk ability per
/// land type the sacrificed cost land had as it was sacrificed (CR 702.14a,
/// last-known information). A sacrificed land with no land type grants none.
fn expand_sacrificed_land_type_landwalks(
    abilities: &[StaticAbility],
    ctx: &ExecutionContext,
) -> Vec<StaticAbility> {
    let mut expanded = Vec::with_capacity(abilities.len());
    for ability in abilities {
        if ability.landwalk_kind()
            != Some(crate::static_abilities::LandwalkKind::SacrificedLandTypes)
        {
            expanded.push(ability.clone());
            continue;
        }
        let land_types = crate::effects::continuous::sacrificed_cost_land_types(ctx);
        expanded.extend(land_types.into_iter().map(StaticAbility::landwalk));
    }
    expanded
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

//! Grant abilities to a target creature until a duration.

use crate::continuous::{EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::filter::ObjectFilterExt as _;
use crate::static_abilities::StaticAbility;
use crate::target::ChooseSpec;

/// Effect that grants one or more abilities to a target creature.
pub type GrantAbilitiesTargetEffect = ironsmith_core::GrantAbilitiesTargetEffect<StaticAbility>;

impl EffectExecutor for GrantAbilitiesTargetEffect {
    fn supports_additional_target_role(&self,spec:&ChooseSpec)->bool {self.target==*spec}

    fn own_preflight_object_specs(&self) -> Vec<ChooseSpec> {
        std::iter::once(self.target.clone()).chain(self.activated_from.iter().cloned())
            .chain(self.keyword_from.iter().filter(|source|!source.last_known).map(|source|source.source.clone())).collect()
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
            if let Some(source)=&self.keyword_from {
                let mut expanded=self.clone();
                expanded.abilities.extend(keyword_abilities_from(game,ctx,source)?);
                expanded.keyword_from=None;
                return grant_player_protections(&expanded,game,ctx);
            }
            return grant_player_protections(self, game, ctx);
        }
        // "Each Horror you control gains all activated abilities of target
        // artifact" (Grell Philosopher): every recipient in the set fixed at
        // resolution (CR 611.2c) receives the same frozen copies.
        let recipients = if self.target.is_target() || ((self.activated_from.is_some() || self.keyword_from.is_some())
            && matches!(self.target.base(), ChooseSpec::All(_)))
        {
            crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target)?
        } else {
            vec![resolve_single_object_for_effect(game, ctx, &self.target)?]
        };
        if self.abilities.is_empty() && self.activated_from.is_none() && self.keyword_from.is_none() {
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
        if let Some(source)=&self.keyword_from {
            modifications.extend(keyword_abilities_from(game,ctx,source)?.into_iter().map(Modification::AddAbility));
        }
        modifications.extend(abilities.into_iter().map(Modification::AddAbility));
        if recipients.is_empty() {return Ok(if self.target.is_target(){EffectOutcome::target_invalid()}else{EffectOutcome::resolved()});}
        let mut modifications=modifications.into_iter();
        let Some(first)=modifications.next() else {return Ok(EffectOutcome::resolved());};
        let mut apply=crate::effects::continuous::ApplyContinuousEffect::new(EffectTarget::AllPermanents,first,self.duration.clone());
        apply.target_spec=Some(self.target.clone());
        apply.additional_modifications=modifications.collect();
        apply.execute(game,ctx)
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

/// Snapshot operands are reference-local. A later live incarnation or a
/// different alias of the same physical card cannot replace this evidence.
fn keyword_abilities_from(game:&mut GameState,ctx:&mut ExecutionContext,source:&ironsmith_core::KeywordAbilitySource)->Result<Vec<StaticAbility>,ExecutionError>{
    let abilities=if source.last_known {
        let snapshots:Vec<&crate::snapshot::ObjectSnapshot>=match source.source.base() {
            ChooseSpec::Tagged(tag)=>ctx.get_tagged_all(tag).ok_or_else(||ExecutionError::IncompleteEvidence("keyword donor snapshot is missing".into()))?.iter().collect(),
            ChooseSpec::Source=>vec![ctx.source_snapshot.as_ref().filter(|snapshot|snapshot.object_id==ctx.source)
                .ok_or_else(||ExecutionError::IncompleteEvidence("source keyword donor snapshot is missing".into()))?],
            ChooseSpec::Object(filter)|ChooseSpec::All(filter) if filter.any_of.is_empty()=>{
                let mut anchors=filter.tagged_constraints.iter().filter(|constraint|constraint.relation==crate::target::TaggedOpbjectRelation::IsTaggedObject);
                let anchor=anchors.next().filter(|_|anchors.next().is_none()).ok_or_else(||ExecutionError::IncompleteEvidence("qualified keyword donor requires one retained reference".into()))?;
                let packet=ctx.get_tagged_all(&anchor.tag).ok_or_else(||ExecutionError::IncompleteEvidence("qualified keyword donor snapshot is missing".into()))?;
                // Qualification itself may inspect abilities. Missing native
                // evidence must not become a false nonmatch/empty donor set.
                if packet.iter().any(|snapshot|snapshot.ability_origins.as_ref().is_none_or(|origins|origins.len()!=snapshot.abilities.len())) {
                    return Err(ExecutionError::IncompleteEvidence("qualified keyword donor executable evidence is unavailable".into()));
                }
                let filter_ctx=ctx.filter_context(game);
                packet.iter().filter(|snapshot|filter.matches_snapshot(snapshot,&filter_ctx,game)).collect()
            }
            _=>return Err(ExecutionError::IncompleteEvidence("historical keyword donor requires an explicit retained reference".into())),
        };
        if snapshots.iter().any(|snapshot|snapshot.ability_origins.as_ref().is_none_or(|origins|origins.len()!=snapshot.abilities.len())) {
            return Err(ExecutionError::IncompleteEvidence("keyword donor executable ability evidence is unavailable".into()));
        }
        snapshots.into_iter().flat_map(|snapshot|snapshot.abilities.iter().cloned()).collect::<Vec<_>>()
    }else{
        let source_id=resolve_single_object_for_effect(game,ctx,&source.source)?;
        game.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;
        game.current_abilities(source_id).unwrap_or_default()
    };
    Ok(abilities.into_iter().filter_map(|ability|match ability.kind {
        crate::ability::AbilityKind::Static(keyword) if keyword.id().is_keyword()
            && (source.keywords.contains(&keyword.id())
                || keyword.id()==crate::static_abilities::StaticAbilityId::HexproofFrom && source.keywords.contains(&crate::static_abilities::StaticAbilityId::Hexproof))=>Some(keyword),
        _=>None,
    }).collect())
}

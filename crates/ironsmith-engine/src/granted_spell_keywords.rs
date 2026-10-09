//! Discovery of keywords granted to spells as they are cast (CR 601.2b).
//!
//! A typed `GrantSpellKeyword` static ability ("Each instant and sorcery spell
//! you cast has replicate. The replicate cost is equal to its mana cost.")
//! applies to every spell its filter matches. It is found either on a
//! permanent (a battlefield grant) or on the spell itself (a one-shot "the
//! next spell you cast has ..." grant attached to that spell, whose filter
//! is the spell itself).
//!
//! While the spell is cast, every applicable optional-cost keyword becomes
//! an optional cost announced on that spell (CR 601.2b) and paid with its
//! other costs (CR 601.2f-h):
//! - replicate (CR 702.56) is the native `Replicate` cost whose copy trigger
//!   the trigger system creates per paid instance;
//! - conspire (CR 702.78) is the native `GrantedConspire` tap cost and trigger;
//! - offspring (CR 702.175) also attaches its "when this permanent enters, if
//!   its offspring cost was paid" trigger to that spell, linked to that one
//!   instance's payment (CR 702.175b).
//!
//! Demonstrate (CR 702.144) has no cost; the trigger system creates one
//! demonstrate trigger per applicable instance when the spell is cast.

use crate::cost::{OptionalCost, OptionalCostKind, OptionalCostRef, TotalCost};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::zone::Zone;
use ironsmith_core::{GrantedSpellKeyword, GrantedSpellKeywordKind, GrantedSpellKeywordPrice};

/// One applicable instance of a granted keyword.
#[derive(Debug, Clone)]
pub(crate) struct GrantedSpellKeywordInstance {
    /// Stable identity of this instance across repeated discovery passes:
    /// the granting object and the ordinal of the grant among its abilities.
    pub(crate) identity: String,
    pub(crate) keyword: GrantedSpellKeyword<crate::costs::Cost>,
}

fn grant_payload(
    static_ability: &crate::static_abilities::StaticAbility,
) -> Option<(&crate::target::ObjectFilter, &GrantedSpellKeyword<crate::costs::Cost>)> {
    let model = static_ability.compiled_model()?;
    let payload = match &model.payload {
        ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => &ability.payload,
        payload => payload,
    };
    match payload {
        ironsmith_core::StaticAbilityPayload::GrantSpellKeyword { filter, keyword } => {
            Some((filter, keyword))
        }
        _ => None,
    }
}

/// Every granted-keyword instance that applies to `spell_id`, cast by
/// `caster`. `prospective` matches the spell as the cast being proposed
/// (CR 601.2), before it is recorded as cast.
pub(crate) fn granted_spell_keywords(
    game: &GameState,
    spell_id: ObjectId,
    caster: PlayerId,
    prospective: bool,
) -> Vec<GrantedSpellKeywordInstance> {
    let Some(spell) = game.object(spell_id) else {
        return Vec::new();
    };
    let mut spell_for_filter = spell.clone();
    if let Some(chars) = game.current_characteristics(spell_id) {
        spell_for_filter.name = chars.name;
        spell_for_filter.card_types = chars.card_types;
        spell_for_filter.subtypes = chars.subtypes;
        spell_for_filter.supertypes = chars.supertypes;
        spell_for_filter.color_override = Some(chars.colors);
    }
    let mut instances = Vec::new();
    let consider = |source: ObjectId,
                        ordinal: usize,
                        static_ability: &crate::static_abilities::StaticAbility,
                        instances: &mut Vec<GrantedSpellKeywordInstance>| {
        let Some((filter, keyword)) = grant_payload(static_ability) else {
            return;
        };
        if !static_ability.is_active(game, source) {
            return;
        }
        let Some(source_object) = game.object(source) else {
            return;
        };
        let mut ctx = game
            .filter_context_for(game.controller_of(source_object), Some(source))
            .with_caster(Some(caster));
        if prospective {
            ctx = ctx.with_prospective_cast(spell_id);
        }
        // The grant names spells; the spell being cast is on the stack
        // (CR 601.2a), so the zone qualifier is already satisfied.
        let mut filter = filter.clone();
        filter.zone = None;
        if !filter.matches_non_recursive(&spell_for_filter, &ctx, game) {
            return;
        }
        instances.push(GrantedSpellKeywordInstance {
            identity: format!("granted-{}-{ordinal}", source_object.stable_id.0.0),
            keyword: keyword.clone(),
        });
    };

    let view = crate::derived_view::DerivedGameView::new(game);
    for &permanent in &game.battlefield {
        let Some(static_abilities) = view.static_abilities_rc(permanent) else {
            continue;
        };
        for (ordinal, static_ability) in static_abilities.iter().enumerate() {
            consider(permanent, ordinal, static_ability, &mut instances);
        }
    }
    // A grant attached to the spell itself ("the next spell you cast has
    // conspire") functions on the stack.
    for (ordinal, ability) in spell.abilities.iter().enumerate() {
        if !ability.functions_in(&Zone::Stack) {
            continue;
        }
        if let crate::ability::AbilityKind::Static(static_ability) = &ability.kind {
            consider(spell_id, ordinal, static_ability, &mut instances);
        }
    }
    instances
}

fn granted_optional_cost(
    instance: &GrantedSpellKeywordInstance,
    spell: &crate::object::Object,
) -> Option<OptionalCost> {
    let price = match &instance.keyword.price {
        GrantedSpellKeywordPrice::Fixed(cost) => Some(cost.clone()),
        // "The replicate cost is equal to its mana cost." A spell with no
        // mana cost has no such cost to pay (CR 202.1b), so nothing is offered.
        GrantedSpellKeywordPrice::SpellManaCost => {
            Some(TotalCost::mana(spell.mana_cost.as_deref().cloned()?))
        }
        GrantedSpellKeywordPrice::Intrinsic => None,
    };
    let (mut cost, kind) = match instance.keyword.kind {
        GrantedSpellKeywordKind::Replicate => {
            (OptionalCost::replicate(price?), OptionalCostKind::Replicate)
        }
        GrantedSpellKeywordKind::Offspring => {
            (OptionalCost::offspring(price?), OptionalCostKind::Offspring)
        }
        GrantedSpellKeywordKind::Conspire => (
            OptionalCost::custom(
                "Granted Conspire",
                TotalCost::from_cost(crate::costs::Cost::effect(
                    crate::effects::ConspireCostEffect::new(),
                )),
            ),
            OptionalCostKind::GrantedConspire,
        ),
        GrantedSpellKeywordKind::Demonstrate => return None,
    };
    cost.kind = kind.clone();
    cost.reference = OptionalCostRef::with_discriminator(kind, instance.identity.clone());
    Some(cost)
}

/// CR 702.175a: "When this permanent enters, if its offspring cost was paid,
/// create a token that's a copy of it, except it's 1/1." Granted to the spell
/// for this one offspring instance; the spell keeps it as it becomes a
/// permanent.
fn offspring_entry_trigger_grant(
    reference: OptionalCostRef,
) -> crate::static_abilities::StaticAbility {
    use crate::ability::{Ability, AbilityKind, TriggeredAbility};
    let trigger = Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: crate::triggers::Trigger::this_enters_battlefield(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                crate::effect::Effect::new(
                    crate::effects::CreateTokenCopyEffect::new(
                        crate::target::ChooseSpec::Source,
                        crate::effect::Value::Fixed(1),
                        crate::target::PlayerFilter::You,
                    )
                    .set_base_power_toughness(1, 1),
                ),
            ]),
            choices: vec![],
            intervening_if: Some(crate::ConditionExpr::ThisSpellPaidLabel(reference)),
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    };
    crate::static_abilities::StaticAbility::grant_object_ability_for_filter(
        crate::target::ObjectFilter::source(),
        trigger,
        "Offspring".to_string(),
    )
}

/// Announce every applicable granted optional-cost keyword on the spell being
/// cast (CR 601.2b). Instances already announced by an earlier pass over the
/// same proposal are not added again. Returns whether the spell changed.
pub(crate) fn ensure_granted_spell_keyword_optional_costs(
    game: &mut GameState,
    spell_id: ObjectId,
    caster: PlayerId,
) -> bool {
    let instances = granted_spell_keywords(game, spell_id, caster, true);
    if instances.is_empty() {
        return false;
    }
    let Some(spell) = game.object(spell_id) else {
        return false;
    };
    let new_costs: Vec<OptionalCost> = instances
        .iter()
        .filter_map(|instance| granted_optional_cost(instance, spell))
        .filter(|cost| {
            !spell
                .optional_costs
                .iter()
                .any(|existing| existing.reference == cost.reference)
        })
        .collect();
    if new_costs.is_empty() {
        return false;
    }
    let mut entry_grants = Vec::new();
    {
        let Some(spell) = game.object_mut(spell_id) else {
            return false;
        };
        for cost in new_costs {
            if cost.kind == OptionalCostKind::Offspring {
                entry_grants.push(offspring_entry_trigger_grant(cost.reference.clone()));
            }
            spell.optional_costs.push(cost);
        }
    }
    for grant in entry_grants {
        game.grant_incarnation_static_ability(spell_id, grant);
    }
    true
}

/// CR 702.144a: "When you cast this spell, you may copy it. If you do, choose
/// an opponent to also copy it. Players may choose new targets for their
/// copies."
pub(crate) fn demonstrate_triggered_ability() -> crate::ability::TriggeredAbility {
    use crate::effect::{Effect, EffectId};
    use crate::target::{ChooseSpec, PlayerFilter};
    let opponent_tag = crate::tag::TagKey::from("demonstrate_opponent");
    let opponent = PlayerFilter::TaggedPlayer(opponent_tag.clone());
    crate::ability::TriggeredAbility {
        trigger: crate::triggers::Trigger::you_cast_this_spell(),
        effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::may(vec![
            Effect::with_id(0, Effect::copy_spell(ChooseSpec::Source)),
            Effect::new(crate::effects::ChoosePlayerEffect::new(
                PlayerFilter::You,
                PlayerFilter::Opponent,
                opponent_tag.clone(),
            )),
            Effect::with_id(
                1,
                Effect::new(crate::effects::CopySpellEffect::new_for_player(
                    ChooseSpec::Source,
                    1,
                    opponent.clone(),
                )),
            ),
            Effect::may_choose_new_targets_player(EffectId(0), PlayerFilter::You),
            Effect::may_choose_new_targets_player(EffectId(1), opponent),
        ])]),
        choices: vec![],
        intervening_if: None,
        presentation_label: None,
    }
}

//! Shared presentation-poison probes for regressions and the corpus audit.
#![allow(dead_code)]
use ironsmith::{
    CardDefinition, ConditionExpr, ability::AbilityKind, static_abilities::StaticAbility,
};
use ironsmith_core::StaticAbilityPayload as P;
/// Replace retained presentation prose in a condition with POISON.
fn poison_condition(condition: &mut ConditionExpr) {
    match condition {
        ConditionExpr::CountComparison { display, .. }
        | ConditionExpr::CountParity { display, .. } => {
            if display.is_some() {
                *display = Some(POISON.into());
            }
        }
        ConditionExpr::AttachmentCount { display, .. } => *display = POISON.into(),
        ConditionExpr::Not(inner) => poison_condition(inner),
        ConditionExpr::And(left, right) | ConditionExpr::Or(left, right) => {
            poison_condition(left);
            poison_condition(right);
        }
        _ => {}
    }
}

pub(crate) const POISON: &str = "POISON";

/// Replace retained presentation prose in every display-bearing static
/// payload field (`display`, `*_surface`, and the displays of nested
/// conditions). Arms were generated from every `StaticAbilityPayload` variant
/// with such a field; extend them when a variant gains one.
fn poison_model(model: &mut ironsmith::static_abilities::CompiledStaticAbility) -> bool {
    poison_model_with(model, true)
}

/// [`poison_model`], optionally keeping `label`: ability-word labels print
/// as presentation prefixes and legacy `keyword:` labels select behavior, so
/// a caller judging individual audit lines keeps them.
fn poison_model_with(
    model: &mut ironsmith::static_abilities::CompiledStaticAbility,
    labels: bool,
) -> bool {
    if labels {
        model.label = POISON.into();
    }
    match &mut model.payload {
        P::Anthem(anthem) => anthem.condition.iter_mut().for_each(poison_condition),
        P::AttachedAbilityGrant(grant) => {
            grant.display = POISON.into();
            grant.condition.iter_mut().for_each(poison_condition);
        }
        P::GrantObjectAbilityForFilter(grant) => {
            if !grant.derived_ability_display {
                grant.display = POISON.into();
            }
            grant.condition.iter_mut().for_each(poison_condition);
        }
        P::Grants(grant) => {
            if grant.filtered_zone_surface.is_some() {
                grant.filtered_zone_surface = Some(POISON.into());
            }
        }
        P::CountersRemainAcrossZoneChanges { display, .. } => {
            *display = POISON.into();
        }
        P::AnyPlayerMayPayManaToIgnoreSourceEffectUntilEndOfTurn { display, .. } => {
            *display = POISON.into();
        }
        P::Conditional {
            condition, ability, ..
        } => {
            poison_condition(condition);
            poison_model_with(ability, labels);
        }
        P::CostIncreaseLife { display, .. } => {
            *display = POISON.into();
        }
        P::ActivateAbilitiesAsThoughHaste { display, .. } => {
            *display = POISON.into();
        }
        P::ThisSpellCastRestriction { display, .. } => {
            *display = POISON.into();
        }
        P::ThisSpellXMaximum { display, .. } => {
            *display = POISON.into();
        }
        P::ThisSpellXMinimum { display, .. } => {
            *display = POISON.into();
        }
        P::RuleRestriction { display, .. } => {
            *display = POISON.into();
        }
        P::CounterLimit { display, .. } => {
            *display = POISON.into();
        }
        P::CantAttackUnlessCondition { display, .. } => {
            *display = POISON.into();
        }
        P::AttackCost { display, .. } => {
            *display = POISON.into();
        }
        P::BlockCost { display, .. } => {
            *display = POISON.into();
        }
        P::UntapDuringEachOtherPlayersUntapStep { display, .. } => {
            *display = POISON.into();
        }
        P::UntapStepLimit { display, .. } => {
            *display = POISON.into();
        }
        P::UntapStepChosenCardTypeOnly { display, .. } => {
            *display = POISON.into();
        }
        P::SearchLimitedToTopCards { display, .. } => {
            *display = POISON.into();
        }
        P::PlayerProtectionFrom { display, .. } => {
            *display = POISON.into();
        }
        P::AddChosenCreatureType { display, .. } => {
            *display = POISON.into();
        }
        P::AddChosenBasicLandType { display, .. } => {
            *display = POISON.into();
        }
        P::SourceHasAbilityStickerAbilitiesOf { display, .. } => {
            *display = POISON.into();
        }
        P::AddChosenColor { display, .. } => {
            *display = POISON.into();
        }
        P::SetChosenColor { display, .. } => {
            *display = POISON.into();
        }
        P::DuplicateMatchingTriggeredAbilities { display, .. } => {
            *display = POISON.into();
        }
        P::SuppressMatchingTriggeredAbilities { display, .. } => {
            *display = POISON.into();
        }
        P::ExertAttack { display, .. } => {
            *display = POISON.into();
        }
        P::EnlistAttack { display, .. } => {
            *display = POISON.into();
        }
        P::RemoveObjectAbilitiesForFilter { display, .. } => {
            *display = POISON.into();
        }
        P::RemoveCardTypes { condition, .. } => {
            condition.iter_mut().for_each(poison_condition);
        }
        P::ActivatedAbilityCostIncrease {
            condition, display, ..
        } => {
            condition.iter_mut().for_each(poison_condition);
            if display.is_some() {
                *display = Some(POISON.into());
            }
        }
        P::ChooseColorAsEnters { display, .. } => {
            *display = POISON.into();
        }
        P::ChoosePlayerAsEnters { display, .. } => {
            *display = POISON.into();
        }
        P::RevealFromHandAsEnters { display, .. } => {
            *display = POISON.into();
        }
        P::ChooseCardNameAsEnters { display, .. } => {
            *display = POISON.into();
        }
        P::ChooseNamedOptionAsEnters { display, .. } => {
            *display = POISON.into();
        }
        P::ChoosePowerToughnessAsEntersOrTurnsFaceUp { display, .. } => {
            *display = POISON.into();
        }
        P::EnterAsCopyAsEnters { display, .. } => {
            *display = POISON.into();
        }
        P::RedirectDamageToSourceController { display, .. } => {
            *display = POISON.into();
        }
        P::ExileWouldDieInstead {
            damager_filter_surface,
            ..
        } => {
            if damager_filter_surface.is_some() {
                *damager_filter_surface = Some(POISON.into());
            }
        }
        P::ModifyDamageAmountReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::MinimumDamageAmountReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::DoubleDamageAmountReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::PreventHalfDamageReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::DoubleCountersReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::AddCountersPlacementReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::PlayerCounterPerTurnLimitReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::DoubleTokenCreationReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::MultiplyTokenCreationReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::AddTokenCreationReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::KeywordActionReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::ConditionalDrawReplacement {
            condition, display, ..
        } => {
            poison_condition(condition);
            *display = POISON.into();
        }
        P::CreateOneOfEachTokenReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::RedirectDrawReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::DrawReplacementWithEffects { display, .. } => {
            *display = POISON.into();
        }
        P::DrawExtraCardsReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::LoseGameReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::DrawReplacementRevealTopMatchingToHandRestBottom { display, .. } => {
            *display = POISON.into();
        }
        P::RedirectWouldEnter { display, .. } => {
            *display = POISON.into();
        }
        P::ManaProductionReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::ManaProductionMultiplierReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::DoubleLifeChangeReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::ManaSpendPermission { display, .. } => {
            *display = POISON.into();
        }
        P::PreventDamageToSelfPutCountersInstead { display, .. } => {
            *display = POISON.into();
        }
        P::HealDamageBeforeDamageToSelf { display, .. } => {
            *display = POISON.into();
        }
        P::PreventConstrainedDamageToSelfPutCountersInstead { display, .. } => {
            *display = POISON.into();
        }
        P::PreventDamageToYouFromSourceFilter { display, .. } => {
            *display = POISON.into();
        }
        P::ReplaceDamageWithCountersInstead { display, .. } => {
            *display = POISON.into();
        }
        P::EntersTappedUnlessCondition {
            condition, display, ..
        } => {
            poison_condition(condition);
            *display = POISON.into();
        }
        P::EntersWithCountersIfCondition {
            condition, display, ..
        } => {
            poison_condition(condition);
            *display = POISON.into();
        }
        P::EntersWithCounterChoice {
            condition, display, ..
        } => {
            condition.iter_mut().for_each(poison_condition);
            *display = POISON.into();
        }
        P::AddLifeGainReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::TokenCreationTemplates { display, .. } => {
            *display = POISON.into();
        }
        P::ActorCountersAddition { display, .. } => {
            *display = POISON.into();
        }
        P::ManaProductionRewrite { display, .. } => {
            *display = POISON.into();
        }
        P::EventReplacementWithEffects { display, .. } => {
            *display = POISON.into();
        }
        P::EventAmountReplacement { display, .. } => {
            *display = POISON.into();
        }
        P::EchoCostAlternative { display, .. } => {
            *display = POISON.into();
        }
        P::SourceHasFullTextOfTopGraveyardCard { display, .. } => {
            *display = POISON.into();
        }
        P::RevealActionOffer { display, .. } => { *display = POISON.into(); }
        P::CopySourceSelectionReplacement {display,..}=>{*display=POISON.into();}
        P::CardCreationSelectionReplacement {display,..}=>{*display=POISON.into();}
        P::MaySkipDrawStepWithFollowup { display, .. } => {
            *display = POISON.into();
        }
        P::PlayerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn { display, .. } => {
            *display = POISON.into();
        }
        P::CastTargetRestriction { display, .. } => {
            *display = POISON.into();
        }
        P::EntersAttachedToChosenObject { display, .. } => {
            *display = POISON.into();
        }
        P::AdditionalSacrificePerManaSymbol { display, .. } => {
            *display = POISON.into();
        }
        P::ControllerCounterExilePermission { display, .. } => {
            *display = POISON.into();
        }
        P::AttackingPlayerChoosesBlocks { display, .. } => {
            *display = POISON.into();
        }
        P::AttachedControllerMayPayCostToIgnoreSourceEffectUntilEndOfTurn { display, .. } => {
            *display = POISON.into();
        }
        P::ChooseBasicLandTypesAsEnters { display, .. } => {
            *display = POISON.into();
        }
        P::FirstChosenLandTypeBecomesSecond { display, .. } => {
            *display = POISON.into();
        }
        P::DamageBetweenChosenPlayers { display, .. } => {
            *display = POISON.into();
        }
        P::AddSubtypesAndGrantAbilities { display, .. } => {
            *display = POISON.into();
        }
        P::ActivatedAbilityCostReduction { display, .. } => {
            if display.is_some() {
                *display = Some(POISON.into());
            }
        }
        P::PregameAction { text, .. } => {
            *text = POISON.into();
        }
        _ => {}
    }
    true
}

pub(crate) fn map_static_models(
    def: &CardDefinition,
    mut f: impl FnMut(&mut ironsmith::static_abilities::CompiledStaticAbility) -> bool,
) -> (CardDefinition, bool) {
    let mut changed = def.clone();
    let mut any = false;
    for ability in &mut changed.abilities {
        let AbilityKind::Static(s) = &ability.kind else {
            continue;
        };
        let Some(model) = s.compiled_model() else {
            continue;
        };
        let mut model = model.clone();
        if f(&mut model) {
            any = true;
            ability.kind = AbilityKind::Static(StaticAbility::from_model(model));
        }
    }
    (changed, any)
}

fn poison_effect(effect: ironsmith::Effect) -> ironsmith::Effect {
    use ironsmith::effects as fx;
    let poison_all = |effects: &mut Vec<ironsmith::Effect>| {
        *effects = std::mem::take(effects)
            .into_iter()
            .map(poison_effect)
            .collect();
    };
    if let Some(conditional) = effect.downcast_ref::<fx::ConditionalEffect>() {
        let mut conditional = conditional.clone();
        poison_condition(&mut conditional.condition);
        poison_all(&mut conditional.if_true);
        poison_all(&mut conditional.if_false);
        return ironsmith::Effect::new(conditional);
    }
    if let Some(with_id) = effect.downcast_ref::<fx::WithIdEffect>() {
        let mut with_id = with_id.clone();
        with_id.effect = Box::new(poison_effect(*with_id.effect));
        return ironsmith::Effect::new(with_id);
    }
    if let Some(tagged) = effect.downcast_ref::<fx::TaggedEffect>() {
        let mut tagged = tagged.clone();
        tagged.effect = Box::new(poison_effect(*tagged.effect));
        return ironsmith::Effect::new(tagged);
    }
    if let Some(sequence) = effect.downcast_ref::<fx::SequenceEffect>() {
        let mut sequence = sequence.clone();
        poison_all(&mut sequence.effects);
        return ironsmith::Effect::new(sequence);
    }
    if let Some(may) = effect.downcast_ref::<fx::MayEffect>() {
        let mut may = may.clone();
        poison_all(&mut may.effects);
        return ironsmith::Effect::new(may);
    }
    if let Some(for_players) = effect.downcast_ref::<fx::ForPlayersEffect>() {
        let mut for_players = for_players.clone();
        poison_all(&mut for_players.effects);
        return ironsmith::Effect::new(for_players);
    }
    // Resolving registrations that retain their authored sentence.
    if let Some(register) = effect.downcast_ref::<fx::RegisterManaSpendPermissionEffect>() {
        let mut register = register.clone();
        register.display = POISON.into();
        return ironsmith::Effect::new(register);
    }
    if let Some(register) = effect.downcast_ref::<fx::RegisterManaRewriteEffect>() {
        let mut register = register.clone();
        register.display = POISON.into();
        return ironsmith::Effect::new(register);
    }
    if let Some(register) = effect.downcast_ref::<fx::RegisterDrawReplacementEffect>() {
        let mut register = register.clone();
        if register.display.is_some() {
            register.display = Some(POISON.into());
        }
        poison_all(&mut register.replacement_effects);
        return ironsmith::Effect::new(register);
    }
    if let Some(redirect) = effect.downcast_ref::<fx::RedirectAllDamageThisTurnToTargetEffect>()
        && redirect.scope.is_some()
    {
        let mut redirect = redirect.clone();
        if let Some(scope) = &mut redirect.scope {
            scope.display = POISON.into();
        }
        return ironsmith::Effect::new(redirect);
    }
    effect
}

fn poison_program(program: &mut ironsmith::resolution::ResolutionProgram) {
    let original = std::mem::take(program);
    *program = original
        .try_map_effects(|effect| Ok::<_, std::convert::Infallible>(poison_effect(effect)))
        .expect("infallible poison transform");
}

/// Poison the supported retained-text fields the audit renderer must not print:
/// static payloads, intervening-if conditions, and resolution conditions.
pub(crate) fn poison_definition(def: &CardDefinition) -> CardDefinition {
    poison_definition_with(def, true)
}

/// [`poison_definition`] keeping static labels (see [`poison_model_with`]):
/// an audit line that changes under this poison read retained prose.
pub(crate) fn poison_presentation_prose(def: &CardDefinition) -> CardDefinition {
    poison_definition_with(def, false)
}

fn poison_definition_with(def: &CardDefinition, labels: bool) -> CardDefinition {
    // Without label poisoning only a model whose prose actually changed is
    // rebuilt, so a rebuilt-from-model static never stands in for an
    // unchanged original.
    let (mut poisoned, _) = map_static_models(def, |model| {
        if labels {
            return poison_model_with(model, true);
        }
        let original = model.clone();
        poison_model_with(model, false);
        *model != original
    });
    if let Some(program) = &mut poisoned.spell_effect {
        poison_program(program);
    }
    for ability in &mut poisoned.abilities {
        match &mut ability.kind {
            AbilityKind::Triggered(triggered) => {
                triggered
                    .intervening_if
                    .iter_mut()
                    .for_each(poison_condition);
                poison_program(&mut triggered.effects);
            }
            AbilityKind::Activated(activated) => poison_program(&mut activated.effects),
            AbilityKind::Static(_) => {}
        }
    }
    poisoned
}

/// One independent mutation per static ability, covering executable gates,
/// quantities, costs, replacement amounts, and permission zones. Unsupported
/// families are counted as untested by the caller, not silently certified.
pub(crate) fn executable_mutations(def: &CardDefinition) -> Vec<(usize, String, CardDefinition)> {
    fn value(v: &mut ironsmith_core::Value) {
        *v = match v {
            ironsmith_core::Value::Fixed(n) => ironsmith_core::Value::Fixed(n.saturating_add(7)),
            _ => ironsmith_core::Value::Fixed(9),
        };
    }
    fn condition(c: &mut ConditionExpr) {
        use ironsmith_core::Comparison as C;
        match c {
            ConditionExpr::CountComparison { comparison, .. }
            | ConditionExpr::AttachmentCount { comparison, .. } => match comparison {
                C::GreaterThan(n)
                | C::GreaterThanOrEqual(n)
                | C::Equal(n)
                | C::LessThan(n)
                | C::LessThanOrEqual(n)
                | C::NotEqual(n) => *n = n.saturating_add(7),
                _ => *comparison = C::Equal(19),
            },
            ConditionExpr::ValueComparison { right, .. } => value(right),
            ConditionExpr::CountParity { even, .. } => *even = !*even,
            ConditionExpr::Not(inner) => condition(inner),
            ConditionExpr::And(a, _) | ConditionExpr::Or(a, _) => condition(a),
            _ => *c = ConditionExpr::Not(Box::new(c.clone())),
        }
    }
    fn mutate(
        model: &mut ironsmith::static_abilities::CompiledStaticAbility,
    ) -> Option<&'static str> {
        use ironsmith_core::{AmountModifierSpec, AnthemValue};
        match &mut model.payload {
            P::SetBasePowerToughness { power, .. } | P::SetBasePower { power, .. } => {
                *power = power.saturating_add(7);
                Some("base power")
            }
            P::SetBaseToughness { toughness, .. } => {
                *toughness = toughness.saturating_add(7);
                Some("base toughness")
            }
            P::SetBasePowerToughnessValue { power, .. } => {
                value(power);
                Some("base power value")
            }
            P::AddCardTypes { filter, .. } | P::SetCardTypes { filter, .. } => {
                filter.controller = Some(if filter.controller == Some(ironsmith_core::PlayerFilter::You) {
                    ironsmith_core::PlayerFilter::Opponent
                } else {
                    ironsmith_core::PlayerFilter::You
                });
                Some("type-change controller")
            }
            P::Conditional { condition: c, .. }
            | P::EntersTappedUnlessCondition { condition: c, .. }
            | P::EntersWithCountersIfCondition { condition: c, .. } => {
                condition(c);
                Some("condition")
            }
            P::Anthem(a) => {
                if let Some(c) = &mut a.condition {
                    condition(c);
                    Some("anthem condition")
                } else {
                    a.power = match &a.power {
                        AnthemValue::Fixed(n) => AnthemValue::Fixed(n.saturating_add(7)),
                        _ => AnthemValue::Fixed(9),
                    };
                    Some("anthem power")
                }
            }
            P::AttachedAbilityGrant(g) => {
                condition(g.condition.as_mut()?);
                Some("attached condition")
            }
            P::GrantObjectAbilityForFilter(g) => {
                if let Some(gate) = &mut g.condition {
                    condition(gate);
                    Some("filter grant condition")
                } else if let ironsmith_core::AbilityKind::Activated(activated) = &mut g.ability.kind
                    && let Some(mana) = &mut activated.mana_output
                {
                    mana.push(ironsmith::mana::ManaSymbol::Colorless);
                    Some("granted mana output")
                } else {
                    g.filter.controller = Some(if g.filter.controller == Some(ironsmith_core::PlayerFilter::You) {
                        ironsmith_core::PlayerFilter::Opponent
                    } else {
                        ironsmith_core::PlayerFilter::You
                    });
                    Some("grant recipient controller")
                }
            }
            P::AddSubtypesAndGrantAbilities { filter, .. } => {
                filter.controller = Some(if filter.controller == Some(ironsmith_core::PlayerFilter::You) {
                    ironsmith_core::PlayerFilter::Opponent
                } else {
                    ironsmith_core::PlayerFilter::You
                });
                Some("subtype grant recipient controller")
            }
            P::DoubleTokenCreationReplacement { controller, .. } => {
                *controller = if *controller == ironsmith_core::PlayerFilter::You {
                    ironsmith_core::PlayerFilter::Opponent
                } else {
                    ironsmith_core::PlayerFilter::You
                };
                Some("token replacement controller")
            }
            P::TokenCreationTemplates { mode, .. } => {
                use ironsmith_core::TokenCreationTemplateMode as Mode;
                *mode = if *mode == Mode::AppendOnce { Mode::AppendForEach } else { Mode::AppendOnce };
                Some("token replacement multiplicity")
            }
            P::CreateOneOfEachTokenReplacement { kinds, .. } => {
                let squirrel = ironsmith_core::AdditionalTokenKind::Squirrel;
                if kinds.contains(&squirrel) {
                    kinds.retain(|kind| *kind != squirrel);
                } else {
                    kinds.push(squirrel);
                }
                Some("replacement token kinds")
            }
            P::ReplaceDamageWithCountersInstead { combat_only, .. } => {
                *combat_only = Some(!combat_only.unwrap_or(false));
                Some("replacement combat scope")
            }
            P::EventAmountReplacement { modifier, .. } => {
                *modifier = match modifier {
                    AmountModifierSpec::SetTo(n) => AmountModifierSpec::SetTo(n.saturating_add(7)),
                    _ => AmountModifierSpec::SetTo(9),
                };
                Some("event amount")
            }
            P::CostReduction(c) => {
                value(&mut c.amount);
                Some("cost reduction")
            }
            P::CostIncrease(c) => {
                value(&mut c.amount);
                Some("cost increase")
            }
            P::ActivatedAbilityCostIncrease { increase, .. }
            | P::AttackCost { cost: increase, .. }
            | P::BlockCost { cost: increase, .. } => {
                let nine =
                    ironsmith_core::TotalCost::mana(ironsmith::mana::ManaCost::from_symbols(vec![
                        ironsmith::mana::ManaSymbol::Generic(9),
                    ]));
                if *increase == nine {
                    return None;
                }
                *increase = nine;
                Some("activation cost")
            }
            P::Grants(g) => {
                // Alternative prices use their explicit origin at runtime;
                // their enclosing grant zone does not constrain cast origin.
                if let ironsmith_core::Grantable::AlternativePrice { origin, .. } = &mut g.grantable {
                    *origin = Some(if *origin == Some(ironsmith::zone::Zone::Hand) {
                        ironsmith::zone::Zone::Exile
                    } else {
                        ironsmith::zone::Zone::Hand
                    });
                    return Some("alternative price origin");
                }
                g.zone = if g.zone == ironsmith::zone::Zone::Graveyard {
                    ironsmith::zone::Zone::Exile
                } else {
                    ironsmith::zone::Zone::Graveyard
                };
                Some("grant zone")
            }
            P::MaxCreaturesCanAttackYouEachCombat(n) => {
                *n = n.saturating_add(7);
                Some("attack limit")
            }
            P::CounterLimit { maximum: n, .. }
            | P::MultiplyTokenCreationReplacement { factor: n, .. }
            | P::SearchLimitedToTopCards { count: n, .. }
            | P::UntapStepLimit { max: n, .. }
            | P::MorphCostIncrease {
                generic_increase: n,
            }
            | P::UnlockCostReduction {
                generic_reduction: n,
            }
            | P::CostIncreaseLife { amount: n, .. }
            | P::PlayerCounterPerTurnLimitReplacement { maximum: n, .. } => {
                *n = n.saturating_add(7);
                Some("quantity")
            }
            P::ThisSpellXMinimum { minimum, .. } => {
                value(minimum);
                Some("minimum X")
            }
            P::ThisSpellXMaximum { maximum, .. } => {
                value(maximum);
                Some("maximum X")
            }
            P::MinimumDamageAmountReplacement { floor, .. } => {
                value(floor);
                Some("damage floor")
            }
            P::AddLifeGainReplacement {
                additional: amount, ..
            }
            | P::AddTokenCreationReplacement {
                additional: amount, ..
            } => {
                *amount = amount.saturating_add(7);
                Some("amount")
            }
            P::DeckConstructionCopyLimit { maximum: Some(n) } => {
                *n = n.saturating_add(7);
                Some("deck copy limit")
            }
            _ => None,
        }
    }
    let mut out = Vec::new();
    for (index, ability) in def.abilities.iter().enumerate() {
        let AbilityKind::Static(s) = &ability.kind else {
            continue;
        };
        let Some(original) = s.compiled_model() else {
            continue;
        };
        let mut model = original.clone();
        let Some(kind) = mutate(&mut model) else {
            continue;
        };
        if format!("{model:?}") == format!("{original:?}") {
            continue;
        }
        let mut changed = def.clone();
        changed.abilities[index].kind = AbilityKind::Static(StaticAbility::from_model(model));
        out.push((index, kind.to_string(), changed));
    }
    out
}

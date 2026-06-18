use super::*;
use crate::ability::ActivatedAbilityRuntimeExt;
use crate::filter::ObjectFilterExt as _;

// ============================================================================
// Pip-by-Pip Mana Payment Helpers
// ============================================================================

pub(super) fn decision_context_name(
    ctx: &crate::decisions::context::DecisionContext,
) -> &'static str {
    use crate::decisions::context::DecisionContext;

    match ctx {
        DecisionContext::Boolean(_) => "boolean",
        DecisionContext::TextInput(_) => "text input",
        DecisionContext::SelectObjects(_) => "select objects",
        DecisionContext::SelectOptions(_) => "select options",
        DecisionContext::Targets(_) => "targets",
        DecisionContext::Number(_) => "number",
        DecisionContext::Priority(_) => "priority",
        DecisionContext::Attackers(_) => "attackers",
        DecisionContext::Blockers(_) => "blockers",
        DecisionContext::Order(_) => "order",
        DecisionContext::Modes(_) => "modes",
        DecisionContext::HybridChoice(_) => "hybrid choice",
        DecisionContext::Distribute(_) => "distribute",
        DecisionContext::Colors(_) => "colors",
        DecisionContext::Counters(_) => "counters",
        DecisionContext::Partition(_) => "partition",
        DecisionContext::Proliferate(_) => "proliferate",
    }
}

fn pay_selected_cost(
    game: &mut GameState,
    cost: &crate::costs::Cost,
    source: ObjectId,
    payer: PlayerId,
    reason: crate::costs::PaymentReason,
    provenance: crate::provenance::ProvNodeId,
    chosen_id: ObjectId,
    choice_tag: Option<&crate::tag::TagKey>,
    tagged_objects: &mut std::collections::HashMap<
        crate::tag::TagKey,
        Vec<crate::snapshot::ObjectSnapshot>,
    >,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    let processing_mode = cost.processing_mode();
    let effective_choice_tag = choice_tag.cloned().or_else(|| match &processing_mode {
        crate::costs::CostProcessingMode::ExileFromHand { .. }
        | crate::costs::CostProcessingMode::ExileFromGraveyard { .. }
        | crate::costs::CostProcessingMode::ExileObjects { .. } => {
            Some(crate::tag::TagKey::from("exile_cost"))
        }
        _ => None,
    });
    let preserve_chosen_snapshot = matches!(
        processing_mode,
        crate::costs::CostProcessingMode::SacrificeTarget { .. }
    );

    let mut cost_ctx = crate::costs::CostContext::new(source, payer, decision_maker)
        .with_reason(reason)
        .with_pre_chosen_cards(vec![chosen_id])
        .with_provenance(provenance);
    cost_ctx.tagged_objects = tagged_objects.clone();
    let chosen_snapshot = game.object(chosen_id).map(|obj| {
        if preserve_chosen_snapshot {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
        } else {
            crate::snapshot::ObjectSnapshot::from_object(obj, game)
        }
    });
    if let Some(tag) = effective_choice_tag.as_ref()
        && let Some(snapshot) = chosen_snapshot.clone()
    {
        cost_ctx
            .tagged_objects
            .entry(tag.clone())
            .or_default()
            .push(snapshot);
    }

    match cost.pay(game, &mut cost_ctx) {
        Ok(crate::costs::CostPaymentResult::Paid) => {
            if !preserve_chosen_snapshot
                && let Some(tag) = effective_choice_tag.as_ref()
                && let Some(snapshot) = chosen_snapshot.as_ref()
                && let Some(current_id) = game.find_object_by_stable_id(snapshot.stable_id)
                && let Some(current) = game.object(current_id)
            {
                let current_snapshot = crate::snapshot::ObjectSnapshot::from_object(current, game);
                let tagged = cost_ctx.tagged_objects.entry(tag.clone()).or_default();
                tagged.retain(|existing| existing.stable_id != snapshot.stable_id);
                tagged.push(current_snapshot);
            }
            *tagged_objects = cost_ctx.tagged_objects;
            Ok(())
        }
        Ok(crate::costs::CostPaymentResult::NeedsChoice(_)) => Err(GameLoopError::InvalidState(
            "Cost still needed a choice after preselection".to_string(),
        )),
        Err(err) => Err(GameLoopError::InvalidState(format!(
            "Failed to pay cost: {err}"
        ))),
    }
}

/// Expand a ManaCost into individual pips, expanding X pips by the chosen value.
/// Also applies hybrid_choices to replace multi-symbol pips with the chosen symbol.
pub(super) fn expand_mana_cost_to_pips(
    cost: &crate::mana::ManaCost,
    x_value: usize,
    hybrid_choices: &[(usize, crate::mana::ManaSymbol)],
) -> Vec<Vec<crate::mana::ManaSymbol>> {
    use crate::mana::ManaSymbol;

    let mut colored_pips = Vec::new();
    let mut generic_pips = Vec::new();

    for (pip_idx, pip) in cost.pips().iter().enumerate() {
        // Check if this is an X pip
        if pip.iter().any(|s| matches!(s, ManaSymbol::X)) {
            // Expand X into x_value generic pips
            for _ in 0..x_value {
                generic_pips.push(vec![ManaSymbol::Generic(1)]);
            }
        } else if pip.iter().all(|s| matches!(s, ManaSymbol::Generic(0))) {
            // Skip Generic(0) pips - they represent zero cost
            continue;
        } else if pip.len() == 1 {
            // Single-symbol pip - check if it's Generic(N) that needs expansion
            if let ManaSymbol::Generic(n) = pip[0] {
                if n > 1 {
                    // Expand Generic(N) into N individual Generic(1) pips
                    for _ in 0..n {
                        generic_pips.push(vec![ManaSymbol::Generic(1)]);
                    }
                    continue;
                } else if n == 1 {
                    generic_pips.push(pip.clone());
                    continue;
                }
            }
            // Colored pip
            colored_pips.push(pip.clone());
        } else {
            // Multi-symbol pip (e.g., hybrid like {B/P} or {W/U})
            // Check if a choice was made during announcement stage
            if let Some((_, chosen_symbol)) = hybrid_choices.iter().find(|(idx, _)| *idx == pip_idx)
            {
                // Use the chosen symbol instead of the full alternatives
                colored_pips.push(vec![*chosen_symbol]);
            } else {
                // No choice made, keep all alternatives (shouldn't happen if announcement worked)
                colored_pips.push(pip.clone());
            }
        }
    }

    // Return colored pips first (more constrained), then generic pips (more flexible)
    colored_pips.extend(generic_pips);
    colored_pips
}

/// Expand a ManaCost into display pips for the UI overlay.
///
/// This keeps original hybrid/Phyrexian symbols intact so the UI can render the
/// printed-looking cost while still following the engine's payment order
/// (colored/constrained pips first, generic pips last).
pub fn expand_mana_cost_to_display_pips(
    cost: &crate::mana::ManaCost,
    x_value: usize,
) -> Vec<Vec<crate::mana::ManaSymbol>> {
    use crate::mana::ManaSymbol;

    let mut colored_pips = Vec::new();
    let mut generic_pips = Vec::new();

    for pip in cost.pips() {
        if pip.iter().any(|s| matches!(s, ManaSymbol::X)) {
            for _ in 0..x_value {
                generic_pips.push(vec![ManaSymbol::Generic(1)]);
            }
            continue;
        }

        if pip.iter().all(|s| matches!(s, ManaSymbol::Generic(0))) {
            continue;
        }

        if pip.len() == 1 {
            if let ManaSymbol::Generic(n) = pip[0] {
                if n > 1 {
                    for _ in 0..n {
                        generic_pips.push(vec![ManaSymbol::Generic(1)]);
                    }
                    continue;
                }
                if n == 1 {
                    generic_pips.push(vec![ManaSymbol::Generic(1)]);
                    continue;
                }
            }
        }

        colored_pips.push(pip.clone());
    }

    colored_pips.extend(generic_pips);
    colored_pips
}

pub(super) fn current_display_pip<'a>(
    display_pips: &'a [Vec<crate::mana::ManaSymbol>],
    remaining_pips: &[Vec<crate::mana::ManaSymbol>],
) -> Option<&'a [crate::mana::ManaSymbol]> {
    let current_index = display_pips.len().checked_sub(remaining_pips.len())?;
    display_pips.get(current_index).map(Vec::as_slice)
}

pub(super) fn preferred_auto_pip_choice(
    state: &PriorityLoopState,
    options: &[ManaPipPaymentOption],
) -> Option<usize> {
    if options.is_empty() {
        return None;
    }

    if state.auto_choose_single_pip_payment && options.len() == 1 {
        return Some(0);
    }

    if options
        .iter()
        .all(|opt| matches!(opt.action, ManaPipPaymentAction::PayViaAlternative { .. }))
    {
        return Some(0);
    }

    None
}

/// Build payment options for a single mana pip.
pub(super) fn build_pip_payment_options(
    game: &GameState,
    player: PlayerId,
    pip: &[crate::mana::ManaSymbol],
    display_pip: Option<&[crate::mana::ManaSymbol]>,
    mana_spend_policy: &crate::player::ManaSpendPolicy,
    allow_black_life: bool,
    source_for_pip_alternatives: Option<ObjectId>,
    payment_reason: crate::costs::PaymentReason,
    decision_maker: &mut impl DecisionMaker,
) -> Vec<ManaPipPaymentOption> {
    use crate::mana::ManaSymbol;

    let mut options = Vec::new();
    let mut index = 0;
    let mut added_pool_symbols = Vec::new();

    // Get the player's mana pool
    let pool = game.player(player).map(|p| &p.mana_pool);

    // For each alternative in the pip, check what can pay it
    for symbol in pip {
        match symbol {
            ManaSymbol::White => {
                add_policy_pool_options_for_required(
                    game,
                    player,
                    ManaSymbol::White,
                    source_for_pip_alternatives,
                    payment_reason,
                    mana_spend_policy,
                    &mut options,
                    &mut index,
                    &mut added_pool_symbols,
                );
            }
            ManaSymbol::Blue => {
                add_policy_pool_options_for_required(
                    game,
                    player,
                    ManaSymbol::Blue,
                    source_for_pip_alternatives,
                    payment_reason,
                    mana_spend_policy,
                    &mut options,
                    &mut index,
                    &mut added_pool_symbols,
                );
            }
            ManaSymbol::Black => {
                add_policy_pool_options_for_required(
                    game,
                    player,
                    ManaSymbol::Black,
                    source_for_pip_alternatives,
                    payment_reason,
                    mana_spend_policy,
                    &mut options,
                    &mut index,
                    &mut added_pool_symbols,
                );
            }
            ManaSymbol::Red => {
                add_policy_pool_options_for_required(
                    game,
                    player,
                    ManaSymbol::Red,
                    source_for_pip_alternatives,
                    payment_reason,
                    mana_spend_policy,
                    &mut options,
                    &mut index,
                    &mut added_pool_symbols,
                );
            }
            ManaSymbol::Green => {
                add_policy_pool_options_for_required(
                    game,
                    player,
                    ManaSymbol::Green,
                    source_for_pip_alternatives,
                    payment_reason,
                    mana_spend_policy,
                    &mut options,
                    &mut index,
                    &mut added_pool_symbols,
                );
            }
            ManaSymbol::Colorless => {
                add_policy_pool_options_for_required(
                    game,
                    player,
                    ManaSymbol::Colorless,
                    source_for_pip_alternatives,
                    payment_reason,
                    mana_spend_policy,
                    &mut options,
                    &mut index,
                    &mut added_pool_symbols,
                );
            }
            ManaSymbol::Generic(_) => {
                // Generic can be paid with any mana in the pool
                add_any_color_pool_options(
                    game,
                    player,
                    source_for_pip_alternatives,
                    payment_reason,
                    &mut options,
                    &mut index,
                );
            }
            ManaSymbol::Life(amount) => {
                // Can always pay life (if player has enough)
                let has_life = game
                    .player(player)
                    .map(|p| p.life >= *amount as i32)
                    .unwrap_or(false);
                if has_life {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: format!("Pay {} life", amount),
                        action: ManaPipPaymentAction::PayLife(*amount as u32),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Snow => {
                // Snow mana - for now treat like generic
                if let Some(p) = pool
                    && p.total() > 0
                {
                    // Just offer any available mana
                    if p.colorless > 0 {
                        options.push(ManaPipPaymentOption {
                            index,
                            description: "Use {C} from mana pool".to_string(),
                            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Colorless),
                        });
                        index += 1;
                    }
                }
            }
            ManaSymbol::X => {
                // X should have been expanded already
            }
        }
    }

    let krrik_can_pay_this_pip = allow_black_life
        && display_pip.is_some_and(|display| display.len() == 1 && display[0] == ManaSymbol::Black)
        && game.can_pay_life(player, 2);
    if krrik_can_pay_this_pip {
        options.push(ManaPipPaymentOption {
            index,
            description: "Pay 2 life".to_string(),
            action: ManaPipPaymentAction::PayLife(2),
        });
        index += 1;
    }

    add_pip_alternative_payment_options(
        game,
        player,
        pip,
        source_for_pip_alternatives,
        &mut options,
        &mut index,
    );

    // Check if this is a Phyrexian pip (has a Life alternative)
    let is_phyrexian = pip.iter().any(|s| matches!(s, ManaSymbol::Life(_)));

    // Check if we have any "use from pool" options (not just Life options)
    let has_pool_options = options
        .iter()
        .any(|opt| matches!(opt.action, ManaPipPaymentAction::UseFromPool(_)));

    // Add mana abilities if:
    // - We don't have pool options, OR
    // - This is a Phyrexian pip (always give choice between mana and life)
    if !has_pool_options || is_phyrexian {
        let mana_abilities = get_available_mana_abilities(game, player, decision_maker);
        for (perm_id, ability_index, description) in mana_abilities {
            let mut source_policy = mana_spend_policy.clone();
            source_policy.allow_any_color |= game.can_spend_mana_as_any_color_from_mana_source(
                player,
                source_for_pip_alternatives,
                perm_id,
            );
            // Check if this ability produces mana that can pay this pip
            if mana_ability_can_pay_pip_with_reason(
                game,
                perm_id,
                ability_index,
                source_for_pip_alternatives,
                payment_reason,
                pip,
                &source_policy,
            ) {
                options.push(ManaPipPaymentOption {
                    index,
                    description: format!(
                        "Tap {}: {}",
                        describe_permanent(game, perm_id),
                        description
                    ),
                    action: ManaPipPaymentAction::ActivateManaAbility {
                        source_id: perm_id,
                        ability_index,
                    },
                });
                index += 1;
            }
        }
    }

    options
}

pub(super) fn add_pip_alternative_payment_options(
    game: &GameState,
    player: PlayerId,
    pip: &[crate::mana::ManaSymbol],
    source_for_pip_alternatives: Option<ObjectId>,
    options: &mut Vec<ManaPipPaymentOption>,
    index: &mut usize,
) {
    let Some(source) = source_for_pip_alternatives else {
        return;
    };
    let Some(spell) = game.object(source) else {
        return;
    };

    if crate::decision::has_convoke(spell) {
        for (creature_id, colors) in crate::decision::get_convoke_creatures(game, player) {
            if convoke_can_pay_pip(colors, pip) {
                options.push(ManaPipPaymentOption {
                    index: *index,
                    description: format!(
                        "Tap {} to pay this pip (Convoke)",
                        describe_permanent(game, creature_id)
                    ),
                    action: ManaPipPaymentAction::PayViaAlternative {
                        permanent_id: creature_id,
                        effect: AlternativePaymentEffect::Convoke,
                    },
                });
                *index += 1;
            }
        }
    }

    if crate::decision::has_improvise(spell) && improvise_can_pay_pip(pip) {
        for artifact_id in crate::decision::get_improvise_artifacts(game, player) {
            options.push(ManaPipPaymentOption {
                index: *index,
                description: format!(
                    "Tap {} to pay this pip (Improvise)",
                    describe_permanent(game, artifact_id)
                ),
                action: ManaPipPaymentAction::PayViaAlternative {
                    permanent_id: artifact_id,
                    effect: AlternativePaymentEffect::Improvise,
                },
            });
            *index += 1;
        }
    }
}

pub(super) fn convoke_can_pay_pip(
    colors: crate::color::ColorSet,
    pip: &[crate::mana::ManaSymbol],
) -> bool {
    pip.iter().any(|symbol| match symbol {
        crate::mana::ManaSymbol::Generic(_) => true,
        crate::mana::ManaSymbol::White => colors.contains(crate::color::Color::White),
        crate::mana::ManaSymbol::Blue => colors.contains(crate::color::Color::Blue),
        crate::mana::ManaSymbol::Black => colors.contains(crate::color::Color::Black),
        crate::mana::ManaSymbol::Red => colors.contains(crate::color::Color::Red),
        crate::mana::ManaSymbol::Green => colors.contains(crate::color::Color::Green),
        crate::mana::ManaSymbol::Colorless
        | crate::mana::ManaSymbol::Life(_)
        | crate::mana::ManaSymbol::Snow
        | crate::mana::ManaSymbol::X => false,
    })
}

pub(super) fn improvise_can_pay_pip(pip: &[crate::mana::ManaSymbol]) -> bool {
    pip.iter()
        .any(|symbol| matches!(symbol, crate::mana::ManaSymbol::Generic(_)))
}

pub(super) fn add_any_color_pool_options(
    game: &GameState,
    player: PlayerId,
    payment_source: Option<ObjectId>,
    payment_reason: crate::costs::PaymentReason,
    options: &mut Vec<ManaPipPaymentOption>,
    index: &mut usize,
) {
    use crate::mana::ManaSymbol;

    if pool_symbol_count_with_reason(
        game,
        player,
        ManaSymbol::White,
        payment_source,
        payment_reason,
    ) > 0
    {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {W} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::White),
        });
        *index += 1;
    }
    if pool_symbol_count_with_reason(
        game,
        player,
        ManaSymbol::Blue,
        payment_source,
        payment_reason,
    ) > 0
    {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {U} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Blue),
        });
        *index += 1;
    }
    if pool_symbol_count_with_reason(
        game,
        player,
        ManaSymbol::Black,
        payment_source,
        payment_reason,
    ) > 0
    {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {B} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Black),
        });
        *index += 1;
    }
    if pool_symbol_count_with_reason(
        game,
        player,
        ManaSymbol::Red,
        payment_source,
        payment_reason,
    ) > 0
    {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {R} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Red),
        });
        *index += 1;
    }
    if pool_symbol_count_with_reason(
        game,
        player,
        ManaSymbol::Green,
        payment_source,
        payment_reason,
    ) > 0
    {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {G} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Green),
        });
        *index += 1;
    }
    if pool_symbol_count_with_reason(
        game,
        player,
        ManaSymbol::Colorless,
        payment_source,
        payment_reason,
    ) > 0
    {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {C} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Colorless),
        });
        *index += 1;
    }
}

fn add_policy_pool_options_for_required(
    game: &GameState,
    player: PlayerId,
    required: crate::mana::ManaSymbol,
    payment_source: Option<ObjectId>,
    payment_reason: crate::costs::PaymentReason,
    mana_spend_policy: &crate::player::ManaSpendPolicy,
    options: &mut Vec<ManaPipPaymentOption>,
    index: &mut usize,
    added_symbols: &mut Vec<crate::mana::ManaSymbol>,
) {
    use crate::mana::ManaSymbol;

    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        if added_symbols.contains(&symbol)
            || !mana_spend_policy.can_pay_symbol(symbol, required)
            || pool_symbol_count_with_reason(game, player, symbol, payment_source, payment_reason)
                == 0
        {
            continue;
        }

        options.push(ManaPipPaymentOption {
            index: *index,
            description: format!(
                "Use {} from mana pool",
                crate::mana::ManaCost::from_symbols(vec![symbol]).to_oracle()
            ),
            action: ManaPipPaymentAction::UseFromPool(symbol),
        });
        *index += 1;
        added_symbols.push(symbol);
    }
}

#[derive(Clone)]
pub(super) struct SpentManaInfo {
    symbol: crate::mana::ManaSymbol,
    source: ObjectId,
    source_chosen_creature_type: Option<crate::types::Subtype>,
    restrictions: Vec<crate::ability::ManaUsageRestriction>,
}

fn cast_spell_mana_rule_matches_payment_source(
    game: &GameState,
    unit: &crate::ability::RestrictedManaUnit,
    card_types: &[crate::types::CardType],
    subtype_requirement: &Option<crate::ability::ManaUsageSubtypeRequirement>,
    payment_source: Option<ObjectId>,
) -> bool {
    let Some(source_id) = payment_source else {
        return false;
    };
    let Some(source_obj) = game.object(source_id) else {
        return false;
    };

    if source_obj.zone != Zone::Stack {
        return false;
    }
    if !card_types
        .iter()
        .all(|card_type| game.current_has_card_type(source_obj.id, *card_type))
    {
        return false;
    }

    let required_subtype = match subtype_requirement {
        Some(crate::ability::ManaUsageSubtypeRequirement::Exact(subtype)) => Some(*subtype),
        Some(crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource) => {
            unit.source_chosen_creature_type
        }
        None => None,
    };
    required_subtype.is_none_or(|subtype| game.current_has_subtype(source_obj.id, subtype))
}

fn cast_spell_filter_matches_payment_source(
    game: &GameState,
    unit: &crate::ability::RestrictedManaUnit,
    filter: &crate::target::ObjectFilter,
    payment_source: Option<ObjectId>,
) -> bool {
    let Some(source_id) = payment_source else {
        return false;
    };
    let Some(source_obj) = game.object(source_id) else {
        return false;
    };
    if source_obj.zone != Zone::Stack {
        return false;
    }

    let Some(mana_source) = game.object(unit.source) else {
        return false;
    };
    let filter_ctx = game.filter_context_for(game.controller_of(mana_source), Some(unit.source));
    filter.matches(source_obj, &filter_ctx, game)
}

fn activate_ability_source_filter_matches_payment_source(
    game: &GameState,
    unit: &crate::ability::RestrictedManaUnit,
    filter: &crate::target::ObjectFilter,
    payment_source: Option<ObjectId>,
) -> bool {
    let Some(source_id) = payment_source else {
        return false;
    };
    let Some(source_obj) = game.object(source_id) else {
        return false;
    };
    if source_obj.zone == Zone::Stack {
        return false;
    }

    let Some(mana_source) = game.object(unit.source) else {
        return false;
    };
    let filter_ctx = game.filter_context_for(game.controller_of(mana_source), Some(unit.source));
    filter.matches(source_obj, &filter_ctx, game)
}

fn restriction_requires_matching_spell(restriction: &crate::ability::ManaUsageRestriction) -> bool {
    match restriction {
        crate::ability::ManaUsageRestriction::CastSpell {
            restrict_to_matching_spell,
            ..
        }
        | crate::ability::ManaUsageRestriction::CastSpellMatching {
            restrict_to_matching_spell,
            ..
        } => *restrict_to_matching_spell,
        crate::ability::ManaUsageRestriction::CastSpellOrActivateAbilitySourceMatching {
            ..
        } => true,
        crate::ability::ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp { .. } => true,
        crate::ability::ManaUsageRestriction::ActivateAbility => true,
    }
}

fn restriction_bonus_applies_to_payment_source(
    game: &GameState,
    unit: &crate::ability::RestrictedManaUnit,
    restriction: &crate::ability::ManaUsageRestriction,
    payment_source: Option<ObjectId>,
) -> bool {
    match restriction {
        crate::ability::ManaUsageRestriction::CastSpell {
            card_types,
            subtype_requirement,
            grant_uncounterable,
            enters_with_counters,
            granted_abilities,
            ..
        } => {
            if !*grant_uncounterable
                && enters_with_counters.is_empty()
                && granted_abilities.is_empty()
            {
                return false;
            }
            cast_spell_mana_rule_matches_payment_source(
                game,
                unit,
                card_types,
                subtype_requirement,
                payment_source,
            )
        }
        crate::ability::ManaUsageRestriction::CastSpellMatching {
            filter,
            grant_uncounterable,
            enters_with_counters,
            granted_abilities,
            ..
        } => {
            if !*grant_uncounterable
                && enters_with_counters.is_empty()
                && granted_abilities.is_empty()
            {
                return false;
            }
            cast_spell_filter_matches_payment_source(game, unit, filter, payment_source)
        }
        crate::ability::ManaUsageRestriction::CastSpellOrActivateAbilitySourceMatching {
            ..
        } => false,
        crate::ability::ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp { .. } => false,
        crate::ability::ManaUsageRestriction::ActivateAbility => false,
    }
}

fn restricted_unit_priority(
    game: &GameState,
    unit: &crate::ability::RestrictedManaUnit,
    payment_source: Option<ObjectId>,
) -> u8 {
    if unit
        .restrictions
        .iter()
        .any(restriction_requires_matching_spell)
    {
        return 0;
    }
    if unit.restrictions.iter().any(|restriction| {
        restriction_bonus_applies_to_payment_source(game, unit, restriction, payment_source)
    }) {
        return 1;
    }
    2
}

pub(super) fn payment_source_matches_restriction(
    game: &GameState,
    unit: &crate::ability::RestrictedManaUnit,
    restriction: &crate::ability::ManaUsageRestriction,
    payment_source: Option<ObjectId>,
) -> bool {
    let Some(source_id) = payment_source else {
        return false;
    };
    let Some(source_obj) = game.object(source_id) else {
        return false;
    };

    match restriction {
        crate::ability::ManaUsageRestriction::CastSpell {
            card_types,
            subtype_requirement,
            restrict_to_matching_spell,
            ..
        } => {
            if !*restrict_to_matching_spell {
                return true;
            }
            cast_spell_mana_rule_matches_payment_source(
                game,
                unit,
                card_types,
                subtype_requirement,
                Some(source_obj.id),
            )
        }
        crate::ability::ManaUsageRestriction::CastSpellMatching {
            filter,
            restrict_to_matching_spell,
            ..
        } => {
            if !*restrict_to_matching_spell {
                return true;
            }
            cast_spell_filter_matches_payment_source(game, unit, filter, Some(source_obj.id))
        }
        crate::ability::ManaUsageRestriction::CastSpellOrActivateAbilitySourceMatching {
            spell_filter,
            ability_source_filter,
        } => {
            cast_spell_filter_matches_payment_source(game, unit, spell_filter, Some(source_obj.id))
                || activate_ability_source_filter_matches_payment_source(
                    game,
                    unit,
                    ability_source_filter,
                    Some(source_obj.id),
                )
        }
        crate::ability::ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp {
            spell_filter,
        } => {
            cast_spell_filter_matches_payment_source(game, unit, spell_filter, Some(source_obj.id))
                || matches_allowed_turn_face_up_payment_source(game, source_obj.id)
                || matches_allowed_unlock_door_payment_source(game, source_obj.id)
        }
        crate::ability::ManaUsageRestriction::ActivateAbility => source_obj.zone != Zone::Stack,
    }
}

fn matches_allowed_turn_face_up_payment_source(game: &GameState, source_id: ObjectId) -> bool {
    let Some(source_obj) = game.object(source_id) else {
        return false;
    };
    source_obj.zone == Zone::Battlefield && game.is_face_down(source_id)
}

fn matches_allowed_unlock_door_payment_source(game: &GameState, source_id: ObjectId) -> bool {
    game.object_is_room_unlock_payment_source(source_id)
}

pub(super) fn restricted_unit_is_payable(
    game: &GameState,
    unit: &crate::ability::RestrictedManaUnit,
    payment_source: Option<ObjectId>,
) -> bool {
    unit.restrictions.iter().all(|restriction| {
        payment_source_matches_restriction(game, unit, restriction, payment_source)
    })
}

#[cfg(test)]
pub(super) fn pool_symbol_count(
    game: &GameState,
    player: PlayerId,
    symbol: crate::mana::ManaSymbol,
    payment_source: Option<ObjectId>,
) -> u32 {
    pool_symbol_count_source_only(game, player, symbol, payment_source)
}

#[cfg(test)]
fn pool_symbol_count_source_only(
    game: &GameState,
    player: PlayerId,
    symbol: crate::mana::ManaSymbol,
    payment_source: Option<ObjectId>,
) -> u32 {
    pool_symbol_count_filtered(game, player, symbol, |unit| {
        restricted_unit_is_payable(game, unit, payment_source)
    })
}

fn pool_symbol_count_with_reason(
    game: &GameState,
    player: PlayerId,
    symbol: crate::mana::ManaSymbol,
    payment_source: Option<ObjectId>,
    payment_reason: crate::costs::PaymentReason,
) -> u32 {
    pool_symbol_count_filtered(game, player, symbol, |unit| {
        game.restricted_mana_unit_is_payable_for_reason(unit, payment_source, payment_reason)
    })
}

fn pool_symbol_count_filtered(
    game: &GameState,
    player: PlayerId,
    symbol: crate::mana::ManaSymbol,
    mut restricted_is_payable: impl FnMut(&crate::ability::RestrictedManaUnit) -> bool,
) -> u32 {
    let Some(player_obj) = game.player(player) else {
        return 0;
    };

    let total = player_obj.mana_pool.amount(symbol);
    if total == 0 {
        return 0;
    }

    let restricted_total = player_obj
        .restricted_mana
        .iter()
        .filter(|unit| unit.symbol == symbol)
        .count() as u32;
    let restricted_payable = player_obj
        .restricted_mana
        .iter()
        .filter(|unit| unit.symbol == symbol)
        .filter(|unit| restricted_is_payable(unit))
        .count() as u32;

    total
        .saturating_sub(restricted_total)
        .saturating_add(restricted_payable)
}

#[cfg(test)]
pub(super) fn spend_pool_symbol(
    game: &mut GameState,
    player: PlayerId,
    symbol: crate::mana::ManaSymbol,
    payment_source: Option<ObjectId>,
) -> Option<SpentManaInfo> {
    spend_pool_symbol_source_only(game, player, symbol, payment_source)
}

#[cfg(test)]
fn spend_pool_symbol_source_only(
    game: &mut GameState,
    player: PlayerId,
    symbol: crate::mana::ManaSymbol,
    payment_source: Option<ObjectId>,
) -> Option<SpentManaInfo> {
    spend_pool_symbol_common(game, player, symbol, payment_source, None)
}

fn spend_pool_symbol_with_reason(
    game: &mut GameState,
    player: PlayerId,
    symbol: crate::mana::ManaSymbol,
    payment_source: Option<ObjectId>,
    payment_reason: crate::costs::PaymentReason,
) -> Option<SpentManaInfo> {
    spend_pool_symbol_common(game, player, symbol, payment_source, Some(payment_reason))
}

fn spend_pool_symbol_common(
    game: &mut GameState,
    player: PlayerId,
    symbol: crate::mana::ManaSymbol,
    payment_source: Option<ObjectId>,
    payment_reason: Option<crate::costs::PaymentReason>,
) -> Option<SpentManaInfo> {
    let unrestricted_available = game.player(player).is_some_and(|player_obj| {
        let total = player_obj.mana_pool.amount(symbol);
        let restricted_total = player_obj
            .restricted_mana
            .iter()
            .filter(|unit| unit.symbol == symbol)
            .count() as u32;
        total > restricted_total
    });

    let payable_restricted = game.player(player).and_then(|player_obj| {
        player_obj
            .restricted_mana
            .iter()
            .enumerate()
            .filter(|(_, unit)| {
                unit.symbol == symbol
                    && if let Some(reason) = payment_reason {
                        game.restricted_mana_unit_is_payable_for_reason(
                            unit,
                            payment_source,
                            reason,
                        )
                    } else {
                        restricted_unit_is_payable(game, unit, payment_source)
                    }
            })
            .min_by_key(|(_, unit)| restricted_unit_priority(game, unit, payment_source))
            .map(|(idx, unit)| (idx, restricted_unit_priority(game, unit, payment_source)))
    });

    let player_obj = game.player_mut(player)?;
    if let Some((idx, priority)) = payable_restricted
        && !(unrestricted_available && priority >= 2)
    {
        if !player_obj.mana_pool.remove(symbol, 1) {
            return None;
        }
        let unit = player_obj.restricted_mana.remove(idx);
        return Some(SpentManaInfo {
            symbol,
            source: unit.source,
            source_chosen_creature_type: unit.source_chosen_creature_type,
            restrictions: unit.restrictions,
        });
    }

    if unrestricted_available && player_obj.mana_pool.remove(symbol, 1) {
        return Some(SpentManaInfo {
            symbol,
            source: ObjectId::from_raw(0),
            source_chosen_creature_type: None,
            restrictions: Vec::new(),
        });
    }

    None
}

pub(super) fn apply_spent_mana_bonuses(
    game: &mut GameState,
    payment_source: Option<ObjectId>,
    spent: &SpentManaInfo,
) {
    let Some(source_id) = payment_source else {
        return;
    };
    let unit = crate::ability::RestrictedManaUnit {
        symbol: spent.symbol,
        source: spent.source,
        source_chosen_creature_type: spent.source_chosen_creature_type,
        restrictions: spent.restrictions.clone(),
    };

    for restriction in &spent.restrictions {
        if !restriction_bonus_applies_to_payment_source(game, &unit, restriction, payment_source) {
            continue;
        }

        let (grant_uncounterable, enters_with_counters, granted_abilities) = match restriction {
            crate::ability::ManaUsageRestriction::CastSpell {
                grant_uncounterable,
                enters_with_counters,
                granted_abilities,
                ..
            }
            | crate::ability::ManaUsageRestriction::CastSpellMatching {
                grant_uncounterable,
                enters_with_counters,
                granted_abilities,
                ..
            } => (
                *grant_uncounterable,
                enters_with_counters,
                granted_abilities,
            ),
            crate::ability::ManaUsageRestriction::CastSpellOrActivateAbilitySourceMatching {
                ..
            } => continue,
            crate::ability::ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp { .. } => {
                continue;
            }
            crate::ability::ManaUsageRestriction::ActivateAbility => continue,
        };

        if grant_uncounterable || !enters_with_counters.is_empty() {
            let Some(source_obj) = game.object_mut(source_id) else {
                return;
            };
            if grant_uncounterable {
                let already_uncounterable = source_obj.abilities.iter().any(|ability| {
                    matches!(
                        &ability.kind,
                        crate::ability::AbilityKind::Static(static_ability)
                            if static_ability.cant_be_countered()
                    )
                });
                if !already_uncounterable {
                    source_obj
                        .abilities
                        .push(crate::ability::Ability::static_ability(
                            crate::static_abilities::StaticAbility::uncounterable(),
                        ));
                }
            }
            for (counter_type, count) in enters_with_counters {
                source_obj
                    .abilities
                    .push(crate::ability::Ability::static_ability(
                        crate::static_abilities::StaticAbility::enters_with_counters(
                            *counter_type,
                            *count,
                        ),
                    ));
            }
        }

        for ability in granted_abilities {
            game.grant_temporary_static_ability_to_object_until_end_of_turn(source_id, *ability);
        }
    }
}

/// Check if a mana ability can produce mana that pays the given pip.
#[cfg(test)]
pub(super) fn mana_ability_can_pay_pip(
    game: &GameState,
    perm_id: ObjectId,
    ability_index: usize,
    payment_source: Option<ObjectId>,
    pip: &[crate::mana::ManaSymbol],
    mana_spend_policy: &crate::player::ManaSpendPolicy,
) -> bool {
    mana_ability_can_pay_pip_source_only(
        game,
        perm_id,
        ability_index,
        payment_source,
        pip,
        mana_spend_policy,
    )
}

#[cfg(test)]
fn mana_ability_can_pay_pip_source_only(
    game: &GameState,
    perm_id: ObjectId,
    ability_index: usize,
    payment_source: Option<ObjectId>,
    pip: &[crate::mana::ManaSymbol],
    mana_spend_policy: &crate::player::ManaSpendPolicy,
) -> bool {
    mana_ability_can_pay_pip_filtered(
        game,
        perm_id,
        ability_index,
        payment_source,
        pip,
        mana_spend_policy,
        |game, unit, payment_source| restricted_unit_is_payable(game, unit, payment_source),
    )
}

fn mana_ability_can_pay_pip_with_reason(
    game: &GameState,
    perm_id: ObjectId,
    ability_index: usize,
    payment_source: Option<ObjectId>,
    payment_reason: crate::costs::PaymentReason,
    pip: &[crate::mana::ManaSymbol],
    mana_spend_policy: &crate::player::ManaSpendPolicy,
) -> bool {
    mana_ability_can_pay_pip_filtered(
        game,
        perm_id,
        ability_index,
        payment_source,
        pip,
        mana_spend_policy,
        |game, unit, payment_source| {
            game.restricted_mana_unit_is_payable_for_reason(unit, payment_source, payment_reason)
        },
    )
}

fn mana_ability_can_pay_pip_filtered(
    game: &GameState,
    perm_id: ObjectId,
    ability_index: usize,
    payment_source: Option<ObjectId>,
    pip: &[crate::mana::ManaSymbol],
    mana_spend_policy: &crate::player::ManaSpendPolicy,
    restricted_is_payable: impl Fn(
        &GameState,
        &crate::ability::RestrictedManaUnit,
        Option<ObjectId>,
    ) -> bool,
) -> bool {
    use crate::ability::AbilityKind;
    use crate::mana::ManaSymbol;

    let Some(obj) = game.object(perm_id) else {
        return false;
    };

    let Some(ability) = game.current_ability(perm_id, ability_index) else {
        return false;
    };

    let AbilityKind::Activated(mana_ability) = &ability.kind else {
        return false;
    };
    if !mana_ability.is_runtime_mana_ability(game, perm_id, game.controller_of(obj)) {
        return false;
    }
    if !mana_ability.mana_usage_restrictions.is_empty() {
        let unit = crate::ability::RestrictedManaUnit {
            symbol: ManaSymbol::Colorless,
            source: perm_id,
            source_chosen_creature_type: game.chosen_creature_type(perm_id),
            restrictions: mana_ability.mana_usage_restrictions.clone(),
        };
        if !restricted_is_payable(game, &unit, payment_source) {
            return false;
        }
    }

    // Check what mana this ability can produce.
    let produced_symbols =
        mana_ability.inferred_mana_symbols(game, perm_id, game.controller_of(obj));

    for produced in &produced_symbols {
        for pip_symbol in pip {
            match (produced, pip_symbol) {
                // Any mana can pay generic
                (_, ManaSymbol::Generic(_)) => return true,
                (_, ManaSymbol::White)
                | (_, ManaSymbol::Blue)
                | (_, ManaSymbol::Black)
                | (_, ManaSymbol::Red)
                | (_, ManaSymbol::Green)
                | (_, ManaSymbol::Colorless) => {
                    if mana_spend_policy.can_pay_symbol(*produced, *pip_symbol) {
                        return true;
                    }
                }
                _ => {}
            }
        }
    }

    false
}

/// Returns true when a mana ability activation is safe to expose as "undo".
///
/// Undo-safe mana abilities are intentionally narrow:
/// - activated mana ability
/// - all activation cost components are tap costs
/// - every runtime effect is mana-production-only
///
/// Anything else (counters, sacrifice, life, non-mana side effects, etc.)
/// is treated as irreversible for UI undo purposes.
pub fn mana_ability_is_undo_safe(game: &GameState, source: ObjectId, ability_index: usize) -> bool {
    use crate::ability::AbilityKind;

    let Some(object) = game.object(source) else {
        return false;
    };
    let Some(ability) = game.current_ability(source, ability_index) else {
        return false;
    };
    let AbilityKind::Activated(mana_ability) = &ability.kind else {
        return false;
    };
    if !mana_ability.is_runtime_mana_ability(game, source, game.controller_of(object)) {
        return false;
    }

    let costs = mana_ability.mana_cost.costs();
    if costs.is_empty() || !costs.iter().all(|cost| cost.requires_tap()) {
        return false;
    }

    mana_ability.effects.iter().all(|effect| {
        effect
            .producible_mana_symbols(game, source, game.controller_of(object))
            .is_some()
    })
}

pub(super) fn pip_mana_color_restriction(
    pip: &[crate::mana::ManaSymbol],
    mana_spend_policy: &crate::player::ManaSpendPolicy,
) -> Option<Vec<crate::color::Color>> {
    use crate::color::Color;
    use crate::mana::ManaSymbol;

    let mut colors = Vec::new();
    let mut has_non_colored_mana_alternative = false;

    for symbol in pip {
        match symbol {
            ManaSymbol::White
            | ManaSymbol::Blue
            | ManaSymbol::Black
            | ManaSymbol::Red
            | ManaSymbol::Green
            | ManaSymbol::Colorless => {
                for (produced, color) in [
                    (ManaSymbol::White, Color::White),
                    (ManaSymbol::Blue, Color::Blue),
                    (ManaSymbol::Black, Color::Black),
                    (ManaSymbol::Red, Color::Red),
                    (ManaSymbol::Green, Color::Green),
                ] {
                    if mana_spend_policy.can_pay_symbol(produced, *symbol) {
                        colors.push(color);
                    }
                }
            }
            ManaSymbol::Generic(_) | ManaSymbol::Snow => {
                has_non_colored_mana_alternative = true;
            }
            ManaSymbol::Life(_) | ManaSymbol::X => {}
        }
    }

    if has_non_colored_mana_alternative {
        return None;
    }

    colors.sort_unstable_by_key(|color| match color {
        Color::White => 0u8,
        Color::Blue => 1u8,
        Color::Black => 2u8,
        Color::Red => 3u8,
        Color::Green => 4u8,
    });
    colors.dedup();

    if colors.is_empty() {
        None
    } else {
        Some(colors)
    }
}

pub(super) fn record_pip_payment_action(trace: &mut Vec<CostStep>, action: &ManaPipPaymentAction) {
    let _ = trace;
    let _ = action;
}

pub(super) fn record_immediate_cost_payment(
    trace: &mut Vec<CostStep>,
    cost: &crate::costs::Cost,
    source: ObjectId,
) {
    let _ = trace;
    let _ = cost;
    let _ = source;
}

pub(super) fn record_cast_mana_ability_payment(
    pending: &mut PendingCast,
    source: ObjectId,
    ability_index: usize,
) {
    let _ = pending;
    let _ = source;
    let _ = ability_index;
}

pub(super) fn record_activation_mana_ability_payment(
    pending: &mut PendingActivation,
    source: ObjectId,
    ability_index: usize,
) {
    let _ = pending;
    let _ = source;
    let _ = ability_index;
}

/// Execute a pip payment action.
/// Execute a pip payment action.
/// Returns true if the pip was actually paid (mana consumed or life paid),
/// false if we only generated mana (need to continue processing this pip).
pub(super) fn execute_pip_payment_action(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    player: PlayerId,
    source: Option<ObjectId>,
    payment_reason: crate::costs::PaymentReason,
    pip: &[crate::mana::ManaSymbol],
    mana_spend_policy: &crate::player::ManaSpendPolicy,
    action: &ManaPipPaymentAction,
    decision_maker: &mut impl DecisionMaker,
    payment_trace: &mut Vec<CostStep>,
    mut mana_spent_to_cast: Option<&mut ManaPool>,
) -> Result<bool, GameLoopError> {
    match action {
        ManaPipPaymentAction::UseFromPool(symbol) => {
            let spent_info =
                spend_pool_symbol_with_reason(game, player, *symbol, source, payment_reason)
                    .ok_or_else(|| {
                        GameLoopError::InvalidState(format!(
                            "Not enough {} mana in the pool",
                            crate::mana::ManaCost::from_symbols(vec![*symbol]).to_oracle()
                        ))
                    })?;
            if let Some(spent) = mana_spent_to_cast.as_deref_mut() {
                track_spent_mana_symbol(spent, spent_info.symbol);
            }
            apply_spent_mana_bonuses(game, source, &spent_info);
            record_pip_payment_action(payment_trace, action);
            Ok(true) // Pip was paid
        }
        ManaPipPaymentAction::ActivateManaAbility {
            source_id,
            ability_index,
        } => {
            let before_pool = game
                .player(player)
                .map(|player_obj| player_obj.mana_pool.clone());
            let mut source_policy = mana_spend_policy.clone();
            source_policy.allow_any_color |=
                game.can_spend_mana_as_any_color_from_mana_source(player, source, *source_id);
            let mana_color_restriction = pip_mana_color_restriction(pip, &source_policy);
            let emitted_events =
                crate::special_actions::perform_activate_mana_ability_restricted_colors_with_events(
                game,
                player,
                *source_id,
                *ability_index,
                mana_color_restriction,
                decision_maker,
            )?;
            for event in emitted_events {
                let include_delayed = event
                    .downcast::<crate::events::AbilityActivatedEvent>()
                    .is_some();
                queue_triggers_from_event(game, trigger_queue, event, include_delayed);
            }
            record_pip_payment_action(payment_trace, action);

            let produced_symbols = before_pool
                .as_ref()
                .and_then(|before| {
                    game.player(player)
                        .map(|player_obj| mana_pool_delta_symbols(before, &player_obj.mana_pool))
                })
                .unwrap_or_default();

            if let Some(spent_info) = spend_pool_mana_for_pip_with_reason(
                game,
                player,
                source,
                payment_reason,
                pip,
                &source_policy,
                &produced_symbols,
            ) {
                if let Some(spent) = mana_spent_to_cast.as_deref_mut() {
                    track_spent_mana_symbol(spent, spent_info.symbol);
                }
                apply_spent_mana_bonuses(game, source, &spent_info);
                record_pip_payment_action(
                    payment_trace,
                    &ManaPipPaymentAction::UseFromPool(spent_info.symbol),
                );
                return Ok(true);
            }

            Ok(false)
        }
        ManaPipPaymentAction::PayLife(amount) => {
            if let Some(player_obj) = game.player_mut(player) {
                player_obj.lose_life(*amount);
            }
            record_pip_payment_action(payment_trace, action);
            Ok(true) // Pip was paid
        }
        ManaPipPaymentAction::PayViaAlternative {
            permanent_id,
            effect,
        } => {
            tap_permanent_with_trigger(game, trigger_queue, *permanent_id);
            if let Some(source_id) = source {
                let event_provenance = game
                    .provenance_graph_mut()
                    .alloc_root_event(crate::events::EventKind::KeywordAction);
                let event = TriggerEvent::new_with_provenance(
                    KeywordActionEvent::new(
                        keyword_action_from_alternative_effect(*effect),
                        player,
                        source_id,
                        1,
                    ),
                    event_provenance,
                );
                queue_triggers_from_event(game, trigger_queue, event, true);
            }
            record_pip_payment_action(payment_trace, action);
            Ok(true) // Pip was paid
        }
    }
}

pub(super) fn mana_pool_delta_symbols(
    before: &ManaPool,
    after: &ManaPool,
) -> Vec<crate::mana::ManaSymbol> {
    use crate::mana::ManaSymbol;

    let mut produced = Vec::new();
    for (symbol, delta) in [
        (ManaSymbol::White, after.white.saturating_sub(before.white)),
        (ManaSymbol::Blue, after.blue.saturating_sub(before.blue)),
        (ManaSymbol::Black, after.black.saturating_sub(before.black)),
        (ManaSymbol::Red, after.red.saturating_sub(before.red)),
        (ManaSymbol::Green, after.green.saturating_sub(before.green)),
        (
            ManaSymbol::Colorless,
            after.colorless.saturating_sub(before.colorless),
        ),
    ] {
        for _ in 0..delta {
            produced.push(symbol);
        }
    }
    produced
}

fn spend_pool_mana_for_pip_with_reason(
    game: &mut GameState,
    player: PlayerId,
    payment_source: Option<ObjectId>,
    payment_reason: crate::costs::PaymentReason,
    pip: &[crate::mana::ManaSymbol],
    mana_spend_policy: &crate::player::ManaSpendPolicy,
    preferred_symbols: &[crate::mana::ManaSymbol],
) -> Option<SpentManaInfo> {
    spend_pool_mana_for_pip_filtered(
        game,
        player,
        payment_source,
        pip,
        mana_spend_policy,
        preferred_symbols,
        |game, player, symbol, payment_source| {
            spend_pool_symbol_with_reason(game, player, symbol, payment_source, payment_reason)
        },
    )
}

fn spend_pool_mana_for_pip_filtered(
    game: &mut GameState,
    player: PlayerId,
    payment_source: Option<ObjectId>,
    pip: &[crate::mana::ManaSymbol],
    mana_spend_policy: &crate::player::ManaSpendPolicy,
    preferred_symbols: &[crate::mana::ManaSymbol],
    mut spend_symbol: impl FnMut(
        &mut GameState,
        PlayerId,
        crate::mana::ManaSymbol,
        Option<ObjectId>,
    ) -> Option<SpentManaInfo>,
) -> Option<SpentManaInfo> {
    use crate::mana::ManaSymbol;

    let mut candidates = Vec::new();

    for &symbol in preferred_symbols {
        if !matches!(
            symbol,
            ManaSymbol::White
                | ManaSymbol::Blue
                | ManaSymbol::Black
                | ManaSymbol::Red
                | ManaSymbol::Green
                | ManaSymbol::Colorless
        ) {
            continue;
        }
        if symbol_can_pay_pip(symbol, pip, mana_spend_policy) && !candidates.contains(&symbol) {
            candidates.push(symbol);
        }
    }

    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        if symbol_can_pay_pip(symbol, pip, mana_spend_policy) && !candidates.contains(&symbol) {
            candidates.push(symbol);
        }
    }

    for symbol in candidates {
        if let Some(spent_info) = spend_symbol(game, player, symbol, payment_source) {
            return Some(spent_info);
        }
    }

    None
}

pub(super) fn symbol_can_pay_pip(
    symbol: crate::mana::ManaSymbol,
    pip: &[crate::mana::ManaSymbol],
    mana_spend_policy: &crate::player::ManaSpendPolicy,
) -> bool {
    use crate::mana::ManaSymbol;

    pip.iter().any(|candidate| match candidate {
        ManaSymbol::Generic(_) | ManaSymbol::Snow => matches!(
            symbol,
            ManaSymbol::White
                | ManaSymbol::Blue
                | ManaSymbol::Black
                | ManaSymbol::Red
                | ManaSymbol::Green
                | ManaSymbol::Colorless
        ),
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green
        | ManaSymbol::Colorless => mana_spend_policy.can_pay_symbol(symbol, *candidate),
        ManaSymbol::Life(_) | ManaSymbol::X => false,
    })
}

pub(super) fn track_spent_mana_symbol(pool: &mut ManaPool, symbol: crate::mana::ManaSymbol) {
    use crate::mana::ManaSymbol;
    match symbol {
        ManaSymbol::White
        | ManaSymbol::Blue
        | ManaSymbol::Black
        | ManaSymbol::Red
        | ManaSymbol::Green
        | ManaSymbol::Colorless => pool.add(symbol, 1),
        ManaSymbol::Generic(_) | ManaSymbol::Snow | ManaSymbol::Life(_) | ManaSymbol::X => {}
    }
}

/// Format a pip for display.
pub(super) fn format_pip(pip: &[crate::mana::ManaSymbol]) -> String {
    use crate::mana::ManaSymbol;

    if pip.len() == 1 {
        // Single symbol
        match &pip[0] {
            ManaSymbol::White => "{W}".to_string(),
            ManaSymbol::Blue => "{U}".to_string(),
            ManaSymbol::Black => "{B}".to_string(),
            ManaSymbol::Red => "{R}".to_string(),
            ManaSymbol::Green => "{G}".to_string(),
            ManaSymbol::Colorless => "{C}".to_string(),
            ManaSymbol::Generic(n) => format!("{{{}}}", n),
            ManaSymbol::Snow => "{S}".to_string(),
            ManaSymbol::Life(n) => format!("{{Pay {} life}}", n),
            ManaSymbol::X => "{X}".to_string(),
        }
    } else {
        // Hybrid/Phyrexian - show alternatives
        let parts: Vec<String> = pip
            .iter()
            .map(|s| match s {
                ManaSymbol::White => "W".to_string(),
                ManaSymbol::Blue => "U".to_string(),
                ManaSymbol::Black => "B".to_string(),
                ManaSymbol::Red => "R".to_string(),
                ManaSymbol::Green => "G".to_string(),
                ManaSymbol::Colorless => "C".to_string(),
                ManaSymbol::Generic(n) => format!("{}", n),
                ManaSymbol::Snow => "S".to_string(),
                ManaSymbol::Life(n) => format!("{} life", n),
                ManaSymbol::X => "X".to_string(),
            })
            .collect();
        format!("{{{}}}", parts.join("/"))
    }
}

/// Apply a modes response to the pending cast or activation.
///
/// This handles mode selection for modal spells and activated abilities.
pub(super) fn apply_modes_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    modes: &[usize],
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if state.pending_cast.is_none()
        && let Some(mut pending) = state.pending_activation.take()
    {
        let has_legal_targets = spell_program_has_legal_targets_with_modes(
            game,
            &pending.effects,
            pending.activator,
            Some(pending.source),
            Some(modes),
        );

        if !has_legal_targets {
            return Err(GameLoopError::InvalidState(
                "Selected mode combination has no legal targets".to_string(),
            ));
        }

        pending.chosen_modes = Some(modes.to_vec());
        pending.remaining_requirements = extract_target_requirements_from_program_with_modes(
            game,
            &pending.effects,
            pending.activator,
            Some(pending.source),
            Some(modes),
        );
        pending.stage = activation_stage_after_modes(&pending);
        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast or activation for modes response".to_string())
    })?;

    let has_legal_targets = game
        .object(pending.spell_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .map(|program| {
            spell_program_has_legal_targets_with_modes(
                game,
                program,
                pending.caster,
                Some(pending.spell_id),
                Some(modes),
            )
        })
        .unwrap_or_else(|| {
            let effects = game
                .object(pending.spell_id)
                .and_then(|obj| obj.spell_effect.as_deref())
                .unwrap_or(&[]);
            spell_has_legal_targets_with_modes(
                game,
                effects,
                pending.caster,
                Some(pending.spell_id),
                Some(modes),
            )
        });

    if !has_legal_targets {
        return Err(GameLoopError::InvalidState(
            "Selected mode combination has no legal targets".to_string(),
        ));
    }

    // Store the chosen modes
    pending.chosen_modes = Some(modes.to_vec());
    pending.remaining_requirements = game
        .object(pending.spell_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .map(|program| {
            extract_target_requirements_from_program_with_modes(
                game,
                program,
                pending.caster,
                Some(pending.spell_id),
                Some(modes),
            )
        })
        .unwrap_or_else(|| {
            let effects = game
                .object(pending.spell_id)
                .and_then(|obj| obj.spell_effect.as_deref())
                .unwrap_or(&[]);
            extract_target_requirements_with_modes(
                game,
                effects,
                pending.caster,
                Some(pending.spell_id),
                Some(modes),
            )
        });

    // Continue to optional costs
    check_optional_costs_or_continue(game, trigger_queue, state, pending, decision_maker)
}

/// Apply an optional costs response to the pending cast.
pub(super) fn apply_optional_costs_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choices: &[(usize, u32)],
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for optional costs response".to_string())
    })?;

    // Store the optional costs paid
    for &(index, times) in choices {
        pending.optional_costs_paid.pay_times(index, times);
    }

    if let Some(spell) = game.object_mut(pending.spell_id) {
        spell.optional_costs_paid = pending.optional_costs_paid.clone();
    }

    if pending.optional_costs_paid.was_entwined()
        && let Some(modal_spec) =
            extract_modal_spec_from_spell(game, pending.spell_id, pending.caster)
    {
        pending.chosen_modes = Some((0..modal_spec.mode_descriptions.len()).collect());
    }

    let has_legal_targets = game
        .object(pending.spell_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .map(|program| {
            spell_program_has_legal_targets_with_modes(
                game,
                program,
                pending.caster,
                Some(pending.spell_id),
                pending.chosen_modes.as_deref(),
            )
        })
        .unwrap_or_else(|| {
            let effects = game
                .object(pending.spell_id)
                .and_then(|obj| obj.spell_effect.as_deref())
                .unwrap_or(&[]);
            spell_has_legal_targets_with_modes(
                game,
                effects,
                pending.caster,
                Some(pending.spell_id),
                pending.chosen_modes.as_deref(),
            )
        });

    if !has_legal_targets {
        return Err(GameLoopError::InvalidState(
            "Selected optional costs leave the spell with no legal targets".to_string(),
        ));
    }

    pending.remaining_requirements = game
        .object(pending.spell_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .map(|program| {
            extract_target_requirements_from_program_with_modes(
                game,
                program,
                pending.caster,
                Some(pending.spell_id),
                pending.chosen_modes.as_deref(),
            )
        })
        .unwrap_or_else(|| {
            let effects = game
                .object(pending.spell_id)
                .and_then(|obj| obj.spell_effect.as_deref())
                .unwrap_or(&[]);
            extract_target_requirements_with_modes(
                game,
                effects,
                pending.caster,
                Some(pending.spell_id),
                pending.chosen_modes.as_deref(),
            )
        });

    // Continue to targeting or finalization
    continue_to_targeting_or_finalize(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a hybrid/Phyrexian mana choice response to a pending cast or activation.
///
/// Per MTG rule 601.2b (and 602.2b for abilities), players announce how they'll pay
/// hybrid/Phyrexian costs before choosing targets. This handler stores the choice
/// and either prompts for the next pip or continues to target selection.
pub(super) fn apply_next_hybrid_choice(
    pending_hybrid_pips: &mut Vec<(usize, Vec<crate::mana::ManaSymbol>)>,
    hybrid_choices: &mut Vec<(usize, crate::mana::ManaSymbol)>,
    choice: usize,
    context_label: &str,
) -> Result<(), GameLoopError> {
    if pending_hybrid_pips.is_empty() {
        return Err(GameLoopError::InvalidState(format!(
            "No pending hybrid pips for hybrid choice response{context_label}",
        )));
    }

    let (pip_idx, alternatives) = pending_hybrid_pips.remove(0);
    if choice >= alternatives.len() {
        return Err(GameLoopError::InvalidState(format!(
            "Invalid hybrid choice {} for pip with {} alternatives{context_label}",
            choice,
            alternatives.len()
        )));
    }

    hybrid_choices.push((pip_idx, alternatives[choice]));
    Ok(())
}

pub(super) fn apply_hybrid_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check if this is for a pending cast (spell) or pending activation (ability)
    if let Some(mut pending) = state.pending_cast.take() {
        if let Err(err) = apply_next_hybrid_choice(
            &mut pending.pending_hybrid_pips,
            &mut pending.hybrid_choices,
            choice,
            "",
        ) {
            state.pending_cast = Some(pending);
            return Err(err);
        }

        if !pending.pending_hybrid_pips.is_empty() {
            return prompt_for_next_hybrid_pip(game, state, pending);
        }

        return continue_to_targets_or_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }

    if let Some(mut pending) = state.pending_activation.take() {
        if let Err(err) = apply_next_hybrid_choice(
            &mut pending.pending_hybrid_pips,
            &mut pending.hybrid_choices,
            choice,
            " (activation)",
        ) {
            state.pending_activation = Some(pending);
            return Err(err);
        }

        // Keep stage as AnnouncingCost and let continue_activation handle the transition
        // This ensures the validation logic runs when all pips have been announced
        pending.stage = ActivationStage::AnnouncingCost;
        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    Err(GameLoopError::InvalidState(
        "No pending cast or activation for hybrid choice response".to_string(),
    ))
}

/// Apply a mana payment response to the pending cast.
///
/// The choice index corresponds to either:
/// - A mana ability to activate (index < num_mana_abilities)
/// - The "pay mana cost" option (last option)
pub(super) fn apply_mana_payment_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::special_actions::{SpecialAction, perform};

    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for mana payment response".to_string())
    })?;

    // Get the available mana abilities to determine what the choice means
    let mana_abilities = get_available_mana_abilities(game, pending.caster, decision_maker);

    if choice < mana_abilities.len() {
        // Player chose to activate a mana ability
        let (perm_id, ability_index, _) = mana_abilities[choice];

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: perm_id,
            ability_index,
        };

        // Perform the mana ability
        if let Err(e) = perform(action, game, pending.caster, &mut *decision_maker) {
            return Err(GameLoopError::InvalidState(format!(
                "Failed to activate mana ability: {e}"
            )));
        }
        drain_pending_trigger_events(game, trigger_queue);

        queue_ability_activated_event(
            game,
            trigger_queue,
            &mut *decision_maker,
            perm_id,
            pending.caster,
            true,
            None,
        );

        pending.undo_locked_by_mana |= !mana_ability_is_undo_safe(game, perm_id, ability_index);

        // Record the mana ability activation in the payment trace.
        record_cast_mana_ability_payment(&mut pending, perm_id, ability_index);

        continue_spell_cast_mana_payment(game, trigger_queue, state, pending, decision_maker)
    } else {
        // Player chose to pay mana cost.
        // Route to pip-by-pip payment for deterministic trace.
        continue_spell_cast_mana_payment(game, trigger_queue, state, pending, decision_maker)
    }
}

/// Apply a mana payment response for a pending mana ability activation.
///
/// Mana abilities don't use the stack, so when the player can pay,
/// we immediately execute the ability.
pub(super) fn apply_mana_payment_response_mana_ability(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::ability::AbilityKind;
    use crate::special_actions::{SpecialAction, perform};

    let mut pending = state.pending_mana_ability.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending mana ability for payment response".to_string())
    })?;

    // Get available mana abilities, excluding the one we're paying for
    // and filtered to only those that can help pay the cost
    let mana_spend_policy = game.mana_spend_policy(pending.activator, Some(pending.source));
    let mana_abilities: Vec<_> =
        get_available_mana_abilities(game, pending.activator, decision_maker)
            .into_iter()
            .filter(|(perm_id, ability_index, _)| {
                // Exclude mana abilities on the same source while paying this
                // source's own activation cost to prevent recursive payment loops.
                if *perm_id == pending.source {
                    return false;
                }

                // Check if this ability can help pay the cost
                if game.object(*perm_id).is_some()
                    && let Some(ability) = game.current_ability(*perm_id, *ability_index)
                    && let AbilityKind::Activated(mana_ability) = &ability.kind
                    && mana_ability.is_runtime_mana_ability(game, *perm_id, pending.activator)
                {
                    let produced =
                        mana_ability.inferred_mana_symbols(game, *perm_id, pending.activator);
                    mana_can_help_pay_cost(
                        &produced,
                        &pending.mana_cost,
                        game,
                        pending.activator,
                        &mana_spend_policy,
                    )
                } else {
                    true // If we can't determine, include it
                }
            })
            .collect();

    if choice < mana_abilities.len() {
        // Player chose to activate a mana ability to generate mana
        let (perm_id, ability_index, _) = mana_abilities[choice].clone();

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: perm_id,
            ability_index,
        };

        // Perform the mana ability
        if let Err(e) = perform(action, game, pending.activator, decision_maker) {
            return Err(GameLoopError::InvalidState(format!(
                "Failed to activate mana ability: {e}"
            )));
        }
        drain_pending_trigger_events(game, trigger_queue);

        queue_ability_activated_event(
            game,
            trigger_queue,
            &mut *decision_maker,
            perm_id,
            pending.activator,
            true,
            None,
        );

        pending.undo_locked_by_mana |= !mana_ability_is_undo_safe(game, perm_id, ability_index);

        // Check if player can now pay
        let can_pay_now = game.can_pay_mana_cost_with_reason(
            pending.activator,
            Some(pending.source),
            &pending.mana_cost,
            0,
            crate::costs::PaymentReason::ActivateManaAbility,
        );

        if can_pay_now {
            // Execute the pending mana ability
            execute_pending_mana_ability(game, trigger_queue, &pending, decision_maker)?;
            // Player retains priority after activating mana ability
            advance_priority_with_dm(game, trigger_queue, decision_maker)
        } else {
            // Still need more mana, show options again
            let options = compute_mana_ability_payment_options(
                game,
                pending.activator,
                &pending,
                &mut *decision_maker,
            );
            let source = pending.source;
            let player = pending.activator;
            let ability_name = game
                .object(source)
                .map(|o| format!("{}'s ability", o.name))
                .unwrap_or_else(|| "ability".to_string());
            state.pending_mana_ability = Some(pending);

            // Convert ManaPaymentOption to SelectableOption
            let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
                .iter()
                .map(|opt| {
                    crate::decisions::context::SelectableOption::new(opt.index, &opt.description)
                })
                .collect();

            let ctx = crate::decisions::context::SelectOptionsContext::mana_payment(
                player,
                source,
                ability_name,
                selectable_options,
            );
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ))
        }
    } else {
        // Player chose to pay mana cost
        // Verify they can actually pay
        if !game.can_pay_mana_cost_with_reason(
            pending.activator,
            Some(pending.source),
            &pending.mana_cost,
            0,
            crate::costs::PaymentReason::ActivateManaAbility,
        ) {
            return Err(GameLoopError::InvalidState(
                "Cannot pay mana cost - insufficient mana".to_string(),
            ));
        }

        // Execute the pending mana ability
        execute_pending_mana_ability(game, trigger_queue, &pending, decision_maker)?;
        // Player retains priority after activating mana ability
        advance_priority_with_dm(game, trigger_queue, decision_maker)
    }
}

/// Execute a pending mana ability after its mana cost has been paid.
pub(super) fn execute_pending_mana_ability(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    pending: &PendingManaAbility,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    use crate::costs::CostContext;
    use crate::effects::ExecutionContext;

    // Pay the mana cost
    if !game.try_pay_mana_cost_with_reason(
        pending.activator,
        Some(pending.source),
        &pending.mana_cost,
        0,
        crate::costs::PaymentReason::ActivateManaAbility,
    ) {
        return Err(GameLoopError::InvalidState(
            "Failed to pay mana cost".to_string(),
        ));
    }

    // Pay other costs from TotalCost
    let mut cost_ctx = CostContext::new(pending.source, pending.activator, decision_maker)
        .with_reason(crate::costs::PaymentReason::ActivateManaAbility)
        .with_provenance(pending.provenance);
    for c in &pending.other_costs {
        crate::special_actions::pay_cost_component_with_choice(game, c, &mut cost_ctx)
            .map_err(|e| GameLoopError::InvalidState(format!("Failed to pay cost: {e}")))?;
    }
    drain_pending_trigger_events(game, trigger_queue);

    // Add fixed mana to player's pool
    let source_snapshot = game
        .object(pending.source)
        .map(|obj| ObjectSnapshot::from_object(obj, game));
    let mana_to_add = crate::events::mana::apply_mana_replacements(
        game,
        pending.source,
        pending.activator,
        pending.activator,
        pending.mana_to_add.clone(),
        pending.mana_production_provenance,
        source_snapshot.clone(),
        decision_maker,
    );
    if !mana_to_add.is_empty() {
        if let Some(player_obj) = game.player_mut(pending.activator) {
            for symbol in &mana_to_add {
                if pending.mana_usage_restrictions.is_empty() {
                    player_obj.mana_pool.add(*symbol, 1);
                } else {
                    player_obj.add_restricted_mana(crate::ability::RestrictedManaUnit {
                        symbol: *symbol,
                        source: pending.source,
                        source_chosen_creature_type: pending.mana_source_chosen_creature_type,
                        restrictions: pending.mana_usage_restrictions.clone(),
                    });
                }
            }
        }
        let event = crate::events::ManaAddedEvent::new(
            pending.source,
            pending.activator,
            pending.activator,
            mana_to_add,
        )
        .with_production_provenance(pending.mana_production_provenance)
        .with_snapshot(source_snapshot.clone())
        .into_trigger_event();
        queue_triggers_from_event(game, trigger_queue, event, false);
    }

    // Execute additional effects (for complex mana abilities)
    if !pending.effects.is_empty() {
        let mut ctx = ExecutionContext::new(pending.source, pending.activator, decision_maker)
            .with_provenance(pending.provenance)
            .with_mana_usage_restrictions(pending.mana_usage_restrictions.clone())
            .with_mana_source_chosen_creature_type(pending.mana_source_chosen_creature_type)
            .with_mana_production_provenance(pending.mana_production_provenance);
        if let Some(snapshot) = source_snapshot.clone() {
            ctx = ctx.with_source_snapshot(snapshot);
        }
        let emitted_events = crate::game_loop::execute_resolution_program(
            game,
            &mut ctx,
            pending.activator,
            pending.source,
            &pending.effects,
            None,
            &[],
        )
        .map_err(|err| GameLoopError::InvalidState(err.to_string()))?;
        queue_triggers_for_events(game, trigger_queue, emitted_events);
        drain_pending_trigger_events(game, trigger_queue);
    }

    game.record_ability_activation(pending.source, pending.ability_index);

    queue_ability_activated_event(
        game,
        trigger_queue,
        &mut *decision_maker,
        pending.source,
        pending.activator,
        true,
        None,
    );

    Ok(())
}

/// Apply a mana payment response for a pending activation.
pub(super) fn apply_mana_payment_response_activation(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::special_actions::{SpecialAction, perform};

    let mut pending = state.pending_activation.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending activation for mana payment response".to_string())
    })?;

    let mana_abilities = get_available_mana_abilities(game, pending.activator, decision_maker);

    if choice < mana_abilities.len() {
        // Player chose to activate a mana ability
        let (perm_id, ability_index, _) = mana_abilities[choice];

        let action = SpecialAction::ActivateManaAbility {
            permanent_id: perm_id,
            ability_index,
        };

        // Perform the mana ability
        if let Err(e) = perform(action, game, pending.activator, &mut *decision_maker) {
            return Err(GameLoopError::InvalidState(format!(
                "Failed to activate mana ability: {e}"
            )));
        }
        drain_pending_trigger_events(game, trigger_queue);

        queue_ability_activated_event(
            game,
            trigger_queue,
            &mut *decision_maker,
            perm_id,
            pending.activator,
            true,
            None,
        );

        pending.undo_locked_by_mana |= !mana_ability_is_undo_safe(game, perm_id, ability_index);

        // Record the mana ability activation in the payment trace.
        record_activation_mana_ability_payment(&mut pending, perm_id, ability_index);

        // Stay in PayingMana stage, continue activation
        continue_activation(game, trigger_queue, state, pending, decision_maker)
    } else {
        // Player chose to pay mana cost
        // Verify they can actually pay
        let x_value = pending.x_value.unwrap_or(0) as u32;
        if let Some(ref cost) = pending.mana_cost_to_pay
            && !game.can_pay_mana_cost_with_reason(
                pending.activator,
                Some(pending.source),
                cost,
                x_value,
                pending.payment_reason,
            )
        {
            return Err(GameLoopError::InvalidState(
                "Cannot pay mana cost - insufficient mana".to_string(),
            ));
        }

        // Pay the mana and finalize
        let mut pending = pending;
        if let Some(ref cost) = pending.mana_cost_to_pay {
            if !game.try_pay_mana_cost_with_reason(
                pending.activator,
                Some(pending.source),
                cost,
                x_value,
                pending.payment_reason,
            ) {
                return Err(GameLoopError::InvalidState(
                    "Cannot pay mana cost - insufficient mana".to_string(),
                ));
            }
        }
        pending.stage = ActivationStage::ReadyToFinalize;
        continue_activation(game, trigger_queue, state, pending, decision_maker)
    }
}

/// Apply a pip payment response for a pending activation.
pub(super) fn apply_pip_payment_response_activation(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_activation.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending activation for pip payment response".to_string())
    })?;

    // Get the current pip being paid
    if pending.remaining_mana_pips.is_empty() {
        return Err(GameLoopError::InvalidState(
            "No remaining pips to pay".to_string(),
        ));
    }

    let pip = pending.remaining_mana_pips[0].clone();
    let display_pip = current_display_pip(&pending.display_mana_pips, &pending.remaining_mana_pips);

    // Rebuild the options to get the action for this choice
    let mana_spend_policy = game.mana_spend_policy(pending.activator, Some(pending.source));
    let allow_black_life = game.player_can_pay_black_with_life_for_reason(
        pending.activator,
        Some(pending.source),
        pending.payment_reason,
    );
    let options = build_pip_payment_options(
        game,
        pending.activator,
        &pip,
        display_pip,
        &mana_spend_policy,
        allow_black_life,
        Some(pending.source),
        pending.payment_reason,
        &mut *decision_maker,
    );

    if choice >= options.len() {
        return Err(GameLoopError::InvalidState(format!(
            "Invalid pip payment choice: {} >= {}",
            choice,
            options.len()
        )));
    }

    let action = &options[choice].action;

    // Execute the payment action
    let pip_paid = execute_pip_payment_action(
        game,
        trigger_queue,
        pending.activator,
        Some(pending.source),
        pending.payment_reason,
        &pip,
        &mana_spend_policy,
        action,
        &mut *decision_maker,
        &mut pending.payment_trace,
        None,
    )?;
    queue_mana_ability_event_for_action(
        game,
        trigger_queue,
        &mut *decision_maker,
        action,
        pending.activator,
    );
    drain_pending_trigger_events(game, trigger_queue);

    if let ManaPipPaymentAction::ActivateManaAbility {
        source_id,
        ability_index,
    } = action
    {
        pending.undo_locked_by_mana |= !mana_ability_is_undo_safe(game, *source_id, *ability_index);
    }

    // Only remove the pip if it was actually paid (not just mana generated)
    if pip_paid {
        pending.remaining_mana_pips.remove(0);
    }

    // Continue activation (will process next pip or finalize)
    continue_activation(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a pip payment response for a pending spell cast.
pub(super) fn apply_pip_payment_response_cast(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for pip payment response".to_string())
    })?;

    // Get the current pip being paid
    if pending.remaining_mana_pips.is_empty() {
        return Err(GameLoopError::InvalidState(
            "No remaining pips to pay".to_string(),
        ));
    }
    let mut perf = super::priority_apply::ManaPipPaymentPerfMetrics {
        pending_kind: "cast".to_string(),
        remaining_pips_before: pending.remaining_mana_pips.len(),
        ..super::priority_apply::ManaPipPaymentPerfMetrics::default()
    };

    let pip = pending.remaining_mana_pips[0].clone();
    let display_pip = current_display_pip(&pending.display_mana_pips, &pending.remaining_mana_pips);

    let mana_spend_policy = game.mana_spend_policy(pending.caster, Some(pending.spell_id));
    let allow_black_life = game.player_can_pay_black_with_life_for_reason(
        pending.caster,
        Some(pending.spell_id),
        crate::costs::PaymentReason::CastSpell,
    );
    let cached_options = std::mem::take(&mut pending.current_pip_payment_options);
    perf.cached_option_count = cached_options.len();
    perf.used_cached_options = !cached_options.is_empty();
    let build_options_started_at = crate::perf::PerfTimer::start();
    let options = if cached_options.is_empty() {
        build_pip_payment_options(
            game,
            pending.caster,
            &pip,
            display_pip,
            &mana_spend_policy,
            allow_black_life,
            Some(pending.spell_id),
            crate::costs::PaymentReason::CastSpell,
            &mut *decision_maker,
        )
    } else {
        cached_options
    };
    perf.build_options_ms = build_options_started_at.elapsed_ms();
    perf.built_option_count = options.len();

    if choice >= options.len() {
        return Err(GameLoopError::InvalidState(format!(
            "Invalid pip payment choice: {} >= {}",
            choice,
            options.len()
        )));
    }

    let action = &options[choice].action;

    // Execute the payment action
    let execute_started_at = crate::perf::PerfTimer::start();
    let pip_paid = execute_pip_payment_action(
        game,
        trigger_queue,
        pending.caster,
        Some(pending.spell_id),
        crate::costs::PaymentReason::CastSpell,
        &pip,
        &mana_spend_policy,
        action,
        &mut *decision_maker,
        &mut pending.payment_trace,
        Some(&mut pending.mana_spent_to_cast),
    )?;
    perf.execute_payment_ms = execute_started_at.elapsed_ms();
    let queue_event_started_at = crate::perf::PerfTimer::start();
    queue_mana_ability_event_for_action(
        game,
        trigger_queue,
        &mut *decision_maker,
        action,
        pending.caster,
    );
    perf.queue_mana_event_ms = queue_event_started_at.elapsed_ms();
    let drain_started_at = crate::perf::PerfTimer::start();
    drain_pending_trigger_events(game, trigger_queue);
    perf.drain_triggers_ms = drain_started_at.elapsed_ms();

    if let ManaPipPaymentAction::ActivateManaAbility {
        source_id,
        ability_index,
    } = action
    {
        pending.undo_locked_by_mana |= !mana_ability_is_undo_safe(game, *source_id, *ability_index);
    }

    // Only remove the pip if it was actually paid (not just mana generated)
    if pip_paid {
        record_keyword_payment_contribution(&mut pending.keyword_payment_contributions, action);
        pending.remaining_mana_pips.remove(0);
    }
    perf.pip_paid = pip_paid;
    perf.remaining_pips_after = pending.remaining_mana_pips.len();

    // Continue spell cast mana payment (will process next pip or finalize)
    let continue_started_at = crate::perf::PerfTimer::start();
    let result =
        continue_spell_cast_mana_payment(game, trigger_queue, state, pending, decision_maker);
    perf.continue_cast_ms = continue_started_at.elapsed_ms();
    perf.result_kind = match &result {
        Ok(GameProgress::NeedsDecisionCtx(ctx)) => decision_context_name(ctx).to_string(),
        Ok(GameProgress::Continue) => "continue".to_string(),
        Ok(GameProgress::StackResolved) => "stack_resolved".to_string(),
        Ok(GameProgress::GameOver(_)) => "game_over".to_string(),
        Err(_) => "error".to_string(),
    };
    super::priority_apply::store_mana_pip_payment_perf(perf);
    result
}

pub(super) fn apply_next_cost_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if let Some(mut pending) = state.pending_activation.take() {
        if !matches!(pending.stage, ActivationStage::ChoosingNextCost) {
            state.pending_activation = Some(pending);
            return Err(GameLoopError::InvalidState(
                "Activation next-cost response outside choosing-next-cost stage".to_string(),
            ));
        }

        let has_mana_option = pending.mana_cost_to_pay.is_some();
        if has_mana_option && choice == 0 {
            pending.stage = ActivationStage::PayingMana;
            return continue_activation(game, trigger_queue, state, pending, decision_maker);
        }

        let cost_index = choice.saturating_sub(usize::from(has_mana_option));
        if cost_index >= pending.remaining_cost_steps.len() {
            return Err(GameLoopError::InvalidState(format!(
                "Invalid activation next-cost choice: {} >= {}",
                cost_index,
                pending.remaining_cost_steps.len()
            )));
        }

        pending.remaining_cost_steps.swap(0, cost_index);
        pending.stage = ActivationStage::ProcessingCosts;
        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState(
            "No pending cast or activation for next-cost response".to_string(),
        )
    })?;
    if !matches!(pending.stage, CastStage::ChoosingNextCost) {
        state.pending_cast = Some(pending);
        return Err(GameLoopError::InvalidState(
            "Spell next-cost response outside choosing-next-cost stage".to_string(),
        ));
    }

    let has_mana_option = pending.mana_cost_to_pay.is_some();
    if has_mana_option && choice == 0 {
        pending.stage = CastStage::PayingMana;
        return continue_spell_cast_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }

    let cost_index = choice.saturating_sub(usize::from(has_mana_option));
    if cost_index >= pending.remaining_cost_steps.len() {
        return Err(GameLoopError::InvalidState(format!(
            "Invalid spell next-cost choice: {} >= {}",
            cost_index,
            pending.remaining_cost_steps.len()
        )));
    }

    pending.remaining_cost_steps.swap(0, cost_index);
    pending.stage = CastStage::ProcessingCosts;
    continue_spell_cost_payment(game, trigger_queue, state, pending, decision_maker)
}

/// Apply an object-selection response for a pending activation.
pub(super) fn apply_sacrifice_target_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    target_id: ObjectId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_activation.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending activation for object-choice response".to_string())
    })?;

    match pending.stage {
        ActivationStage::ChoosingSacrifice => {
            let (cost, filter, choice_tag) = match pending.remaining_cost_steps.first() {
                Some(ActivationCostStep::Sacrifice {
                    cost,
                    filter,
                    choice_tag,
                    ..
                }) => (cost.clone(), filter.clone(), choice_tag.clone()),
                _ => {
                    return Err(GameLoopError::InvalidState(
                        "No pending sacrifice cost for activation".to_string(),
                    ));
                }
            };
            let legal_targets = get_legal_sacrifice_targets(
                game,
                pending.activator,
                pending.source,
                &filter,
                pending.payment_reason,
            );
            if !legal_targets.contains(&target_id) {
                return Err(GameLoopError::InvalidState(
                    "Selected permanent is not a legal sacrifice cost choice".to_string(),
                ));
            }

            let choice_tag = choice_tag.unwrap_or_else(|| {
                let tag = format!("sacrifice_cost_{}", pending.next_sacrifice_cost_tag_index);
                pending.next_sacrifice_cost_tag_index += 1;
                crate::tag::TagKey::from(tag)
            });
            pay_selected_cost(
                game,
                &cost,
                pending.source,
                pending.activator,
                pending.payment_reason,
                pending.provenance,
                target_id,
                Some(&choice_tag),
                &mut pending.tagged_objects,
                decision_maker,
            )?;

            drain_pending_trigger_events(game, trigger_queue);

            pending.remaining_cost_steps.remove(0);
            pending.stage = activation_stage_after_targets(&pending);
        }
        ActivationStage::ChoosingCardCost => {
            let next_cost = pending
                .remaining_cost_steps
                .first()
                .and_then(|step| match step {
                    ActivationCostStep::CardChoice(choice) => Some(choice.clone()),
                    _ => None,
                })
                .ok_or_else(|| {
                    GameLoopError::InvalidState(
                        "No pending card choice cost for activation".to_string(),
                    )
                })?;

            match next_cost {
                ActivationCardCostChoice::Discard {
                    cost, card_types, ..
                } => {
                    let legal_cards = get_legal_discard_cards(
                        game,
                        pending.activator,
                        pending.source,
                        &card_types,
                    );
                    if !legal_cards.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal discard cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::ExileFromHand {
                    cost, color_filter, ..
                } => {
                    let legal_cards = get_legal_exile_from_hand_cards(
                        game,
                        pending.activator,
                        pending.source,
                        color_filter,
                    );
                    if !legal_cards.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal exile-from-hand cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::ExileFromGraveyard {
                    cost, card_type, ..
                } => {
                    let legal_cards =
                        get_legal_exile_from_graveyard_cards(game, pending.activator, card_type);
                    if !legal_cards.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal graveyard exile cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::ExileChosenObject {
                    cost,
                    filter,
                    zone,
                    choice_tag,
                    ..
                } => {
                    let legal_objects = get_legal_cost_choice_objects(
                        game,
                        pending.activator,
                        pending.source,
                        &filter,
                        zone,
                    );
                    if !legal_objects.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected object is not a legal exile cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        Some(&choice_tag),
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::RevealFromHand {
                    cost,
                    card_type,
                    color_filter,
                    ..
                } => {
                    let legal_cards = get_legal_reveal_from_hand_cards(
                        game,
                        pending.activator,
                        pending.source,
                        card_type,
                        color_filter,
                    );
                    if !legal_cards.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal reveal cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;
                }
                ActivationCardCostChoice::ReturnToHand {
                    cost,
                    filter,
                    choice_tag,
                    ..
                } => {
                    let legal_targets = get_legal_return_to_hand_targets(
                        game,
                        pending.activator,
                        pending.source,
                        &filter,
                    );
                    if !legal_targets.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected permanent is not a legal return-to-hand cost choice"
                                .to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        choice_tag.as_ref(),
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::MoveChosenObjectToZone {
                    cost,
                    filter,
                    source_zone,
                    choice_tag,
                    ..
                } => {
                    let legal_objects = get_legal_cost_choice_objects(
                        game,
                        pending.activator,
                        pending.source,
                        &filter,
                        source_zone,
                    );
                    if !legal_objects.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected object is not a legal move-to-zone cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        Some(&choice_tag),
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
            }

            pending.remaining_cost_steps.remove(0);
            pending.stage = activation_stage_after_targets(&pending);
        }
        _ => {
            return Err(GameLoopError::InvalidState(
                "Object-choice response outside activation object-cost stages".to_string(),
            ));
        }
    }

    // Continue activation process
    continue_activation(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a card/object choice response for a pending spell cast cost.
pub(super) fn apply_card_cost_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    chosen_id: ObjectId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for card-cost response".to_string())
    })?;

    match pending.stage {
        CastStage::ChoosingSacrifice => {
            let (cost, filter, choice_tag) = match pending.remaining_cost_steps.first() {
                Some(ActivationCostStep::Sacrifice {
                    cost,
                    filter,
                    choice_tag,
                    ..
                }) => (cost.clone(), filter.clone(), choice_tag.clone()),
                _ => {
                    return Err(GameLoopError::InvalidState(
                        "No pending sacrifice cost for spell cast".to_string(),
                    ));
                }
            };
            let legal_targets = get_legal_sacrifice_targets(
                game,
                pending.caster,
                pending.spell_id,
                &filter,
                crate::costs::PaymentReason::CastSpell,
            );
            if !legal_targets.contains(&chosen_id) {
                return Err(GameLoopError::InvalidState(
                    "Selected permanent is not a legal spell sacrifice cost choice".to_string(),
                ));
            }

            let choice_tag = choice_tag.unwrap_or_else(|| {
                let tag = format!("sacrifice_cost_{}", pending.next_sacrifice_cost_tag_index);
                pending.next_sacrifice_cost_tag_index += 1;
                crate::tag::TagKey::from(tag)
            });
            pay_selected_cost(
                game,
                &cost,
                pending.spell_id,
                pending.caster,
                crate::costs::PaymentReason::CastSpell,
                pending.provenance,
                chosen_id,
                Some(&choice_tag),
                &mut pending.tagged_objects,
                decision_maker,
            )?;

            drain_pending_trigger_events(game, trigger_queue);

            pending.remaining_cost_steps.remove(0);
            pending.stage = CastStage::ChoosingNextCost;
            continue_spell_next_cost_or_finalize(
                game,
                trigger_queue,
                state,
                pending,
                decision_maker,
            )
        }
        CastStage::ChoosingCardCost => {
            let next_cost = pending
                .remaining_cost_steps
                .first()
                .and_then(|step| match step {
                    ActivationCostStep::CardChoice(choice) => Some(choice.clone()),
                    _ => None,
                })
                .ok_or_else(|| {
                    GameLoopError::InvalidState(
                        "No pending card choice cost for spell cast".to_string(),
                    )
                })?;

            match next_cost {
                ActivationCardCostChoice::Discard {
                    cost, card_types, ..
                } => {
                    let legal_cards = get_legal_discard_cards(
                        game,
                        pending.caster,
                        pending.spell_id,
                        &card_types,
                    );
                    if !legal_cards.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal spell discard cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::ExileFromHand {
                    cost, color_filter, ..
                } => {
                    let legal_cards = get_legal_exile_from_hand_cards(
                        game,
                        pending.caster,
                        pending.spell_id,
                        color_filter,
                    );
                    if !legal_cards.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal spell exile-from-hand cost choice"
                                .to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::ExileFromGraveyard {
                    cost, card_type, ..
                } => {
                    let legal_cards =
                        get_legal_exile_from_graveyard_cards(game, pending.caster, card_type);
                    if !legal_cards.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal spell graveyard exile cost choice"
                                .to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::ExileChosenObject {
                    cost,
                    filter,
                    zone,
                    choice_tag,
                    ..
                } => {
                    let legal_objects = get_legal_cost_choice_objects(
                        game,
                        pending.caster,
                        pending.spell_id,
                        &filter,
                        zone,
                    );
                    if !legal_objects.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected object is not a legal spell exile cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        Some(&choice_tag),
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::RevealFromHand {
                    cost,
                    card_type,
                    color_filter,
                    ..
                } => {
                    let legal_cards = get_legal_reveal_from_hand_cards(
                        game,
                        pending.caster,
                        pending.spell_id,
                        card_type,
                        color_filter,
                    );
                    if !legal_cards.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal spell reveal cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;
                }
                ActivationCardCostChoice::ReturnToHand {
                    cost,
                    filter,
                    choice_tag,
                    ..
                } => {
                    let legal_targets = get_legal_return_to_hand_targets(
                        game,
                        pending.caster,
                        pending.spell_id,
                        &filter,
                    );
                    if !legal_targets.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected permanent is not a legal spell return-to-hand cost choice"
                                .to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        choice_tag.as_ref(),
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::MoveChosenObjectToZone {
                    cost,
                    filter,
                    source_zone,
                    choice_tag,
                    ..
                } => {
                    let legal_objects = get_legal_cost_choice_objects(
                        game,
                        pending.caster,
                        pending.spell_id,
                        &filter,
                        source_zone,
                    );
                    if !legal_objects.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected object is not a legal spell move-to-zone cost choice"
                                .to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        Some(&choice_tag),
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
            }

            pending.remaining_cost_steps.remove(0);
            pending.stage = CastStage::ChoosingNextCost;
            continue_spell_next_cost_or_finalize(
                game,
                trigger_queue,
                state,
                pending,
                decision_maker,
            )
        }
        _ => Err(GameLoopError::InvalidState(
            "Object-choice response outside spell object-cost stages".to_string(),
        )),
    }
}

/// Apply a casting method choice response for a pending spell with multiple methods.
pub(super) fn apply_casting_method_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice_idx: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let pending = state.pending_method_selection.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending method selection for choice response".to_string())
    })?;

    // Get the chosen method
    let chosen_option = pending
        .available_methods
        .get(choice_idx)
        .ok_or_else(|| ResponseError::IllegalChoice("Invalid casting method choice".to_string()))?;

    let casting_method = chosen_option.method.clone();

    // Now continue with the normal spell casting flow using the chosen method
    // This is essentially a copy of the CastSpell handling logic
    let player = pending.caster;
    let spell_id = pending.spell_id;
    let from_zone = pending.from_zone;

    // Move spell to stack immediately per MTG rule 601.2a
    let stack_id = propose_spell_cast(game, spell_id, from_zone, player, &casting_method)?;
    let cast_provenance =
        game.provenance_graph_mut()
            .alloc_root(ProvenanceNodeKind::EffectExecution {
                source: stack_id,
                controller: player,
            });

    // Get the spell's mana cost and effects, considering casting method
    // Note: We use stack_id now since the spell has been moved to stack
    let (mana_cost, effects) = if let Some(obj) = game.object(stack_id) {
        let cost = crate::decision::spell_mana_cost_for_cast(
            game,
            player,
            obj,
            &casting_method,
            from_zone,
        );
        (cost, obj.spell_effect.clone().unwrap_or_default())
    } else {
        (None, crate::resolution::ResolutionProgram::default())
    };

    let (needs_x, min_x, max_x) =
        compute_spell_cast_x_bounds(game, player, stack_id, &casting_method, mana_cost.as_ref());

    if needs_x {
        // Extract target requirements for later (use stack_id since spell is on stack)
        let requirements = extract_target_requirements_from_program_with_modes(
            game,
            &effects,
            player,
            Some(stack_id),
            None,
        );

        // Initialize optional costs tracker from the spell's optional costs
        let optional_costs_paid = game
            .object(stack_id)
            .map(|obj| OptionalCostsPaid::from_costs(&obj.optional_costs))
            .unwrap_or_default();

        state.pending_cast = Some(PendingCast::new(
            stack_id,
            from_zone,
            player,
            cast_provenance,
            CastStage::ChoosingX,
            None,
            requirements,
            casting_method,
            optional_costs_paid,
            None,
            stack_id,
        ));

        let ctx = crate::decisions::context::NumberContext::x_value_with_min(
            player, stack_id, // Use stack_id
            min_x, max_x,
        );
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Number(ctx),
        ))
    } else {
        // No X cost, check for optional costs then targets
        let requirements = extract_target_requirements_from_program_with_modes(
            game,
            &effects,
            player,
            Some(stack_id),
            None,
        );

        // Initialize optional costs tracker from the spell's optional costs
        let optional_costs_paid = game
            .object(stack_id)
            .map(|obj| OptionalCostsPaid::from_costs(&obj.optional_costs))
            .unwrap_or_default();

        let new_pending = PendingCast::new(
            stack_id,
            from_zone,
            player,
            cast_provenance,
            CastStage::ChoosingModes, // Will be updated by helper
            None,
            requirements,
            casting_method,
            optional_costs_paid,
            None,
            stack_id,
        );

        check_modes_or_continue(game, trigger_queue, state, new_pending, decision_maker)
    }
}

/// Move a spell to the stack at the start of casting (per MTG rule 601.2a).
///
/// This is called during the proposal phase, before any choices are made.
/// If casting fails later (e.g., can't pay costs), the spell should be reverted.
///
/// Returns the new ObjectId on the stack.
pub(crate) fn propose_spell_cast(
    game: &mut GameState,
    spell_id: ObjectId,
    _from_zone: Zone,
    caster: PlayerId,
    casting_method: &CastingMethod,
) -> Result<ObjectId, GameLoopError> {
    let selected_method = game.object(spell_id).and_then(|obj| match casting_method {
        CastingMethod::Alternative(idx) => obj.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => crate::decision::resolve_play_from_alternative_method(game, caster, obj, *zone, *idx),
        _ => None,
    });
    let selected_method_for_overlay = selected_method.clone();
    let cast_origin_snapshot = game.object(spell_id).map(|obj| {
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
    });

    let new_id = game
        .move_object_by_effect(spell_id, Zone::Stack)
        .ok_or_else(|| {
            GameLoopError::InvalidState("Failed to move spell to stack during proposal".to_string())
        })?;
    if let Some(snapshot) = cast_origin_snapshot {
        game.set_cast_origin_snapshot(new_id, snapshot);
    }
    let disturb_other_def = if matches!(
        selected_method,
        Some(crate::alternative_cast::AlternativeCastingMethod::Disturb { .. })
    ) {
        let obj = game.object(new_id).ok_or_else(|| {
            GameLoopError::InvalidState(
                "Disturb spell should exist before cast overlays".to_string(),
            )
        })?;
        Some(
            game.linked_face_definition_by_name_or_id(
                obj.other_face_name.as_deref(),
                obj.other_face,
            )
            .ok_or_else(|| {
                GameLoopError::InvalidState(
                    "Disturb back face definition could not be resolved".to_string(),
                )
            })?,
        )
    } else {
        None
    };
    let split_other_def = match casting_method {
        CastingMethod::SplitOtherHalf
        | CastingMethod::SplitOtherHalfPlayFrom { .. }
        | CastingMethod::Fuse => {
            let obj = game.object(new_id).ok_or_else(|| {
                GameLoopError::InvalidState(
                    "Split spell should exist before cast overlays".to_string(),
                )
            })?;
            Some(
                game.linked_face_definition_by_name_or_id(
                    obj.other_face_name.as_deref(),
                    obj.other_face,
                )
                .ok_or_else(|| {
                    GameLoopError::InvalidState(
                        match casting_method {
                            CastingMethod::SplitOtherHalf
                            | CastingMethod::SplitOtherHalfPlayFrom { .. } => {
                                "Split back face definition could not be resolved"
                            }
                            CastingMethod::Fuse => {
                                "Fused split back face definition could not be resolved"
                            }
                            _ => unreachable!(),
                        }
                        .to_string(),
                    )
                })?,
            )
        }
        _ => None,
    };

    let mut mark_face_down = false;
    game.set_current_controller(new_id, caster);
    if let Some(obj) = game.object_mut(new_id) {
        if let Some(method) = selected_method {
            obj.cast_alternative_method = Some(method.clone());
            if method.is_bestow() {
                obj.apply_bestow_cast_overlay();
            }
            if matches!(
                method,
                crate::alternative_cast::AlternativeCastingMethod::Composed { name, .. }
                    if name.eq_ignore_ascii_case("Prototype")
            ) && let Some(cost) = method.mana_cost().cloned()
            {
                obj.apply_prototype_cast_overlay(cost);
            }

            if let crate::alternative_cast::AlternativeCastingMethod::Disturb { .. } = method {
                let other_def = disturb_other_def
                    .as_ref()
                    .expect("disturb linked face should be resolved before mutating the spell");
                let front_colors = obj.colors();
                obj.apply_definition_face(&other_def);
                obj.cast_alternative_method = Some(method.clone());
                if obj.mana_cost.is_none()
                    && obj.color_override.is_none()
                    && !front_colors.is_empty()
                {
                    obj.color_override = Some(front_colors);
                }
            }

            if let crate::alternative_cast::AlternativeCastingMethod::Overload {
                ref effects, ..
            } = method
            {
                obj.spell_effect = Some(crate::resolution::ResolutionProgram::from_effects(
                    effects.clone(),
                ));
            }
            if let crate::alternative_cast::AlternativeCastingMethod::Awaken {
                ref effects, ..
            } = method
            {
                obj.spell_effect = Some(crate::resolution::ResolutionProgram::from_effects(
                    effects.clone(),
                ));
            }
        }

        match casting_method {
            CastingMethod::FaceDown => {
                obj.apply_face_down_cast_overlay();
                mark_face_down = true;
            }
            CastingMethod::SplitOtherHalf | CastingMethod::SplitOtherHalfPlayFrom { .. } => {
                let other_def = split_other_def
                    .as_ref()
                    .expect("split linked face should be resolved before mutating the spell");
                obj.apply_definition_face(&other_def);
                if let CastingMethod::SplitOtherHalfPlayFrom { .. } = casting_method
                    && let Some(method) = selected_method_for_overlay.clone()
                {
                    obj.cast_alternative_method = Some(method);
                }
            }
            CastingMethod::Fuse => {
                let other_def = split_other_def
                    .as_ref()
                    .expect("fuse linked face should be resolved before mutating the spell");
                obj.apply_fused_split_spell_overlay(&other_def);
            }
            _ => {}
        }

        obj.ensure_aura_cast_spell_effect();
    }

    if mark_face_down {
        game.set_face_down(new_id);
    }

    apply_play_from_cast_this_way_grants(game, new_id, caster, casting_method);

    Ok(new_id)
}

fn apply_play_from_cast_this_way_grants(
    game: &mut GameState,
    stack_id: ObjectId,
    caster: PlayerId,
    casting_method: &CastingMethod,
) {
    let (source_id, zone) = match casting_method {
        CastingMethod::PlayFrom { source, zone, .. }
        | CastingMethod::SplitOtherHalfPlayFrom { source, zone, .. } => (*source, *zone),
        _ => return,
    };
    let source = game.object(source_id).or_else(|| game.object(stack_id));
    let Some(source) = source else {
        return;
    };
    let Some(mut spell_as_cast) = game.object(stack_id).cloned() else {
        return;
    };
    spell_as_cast.zone = zone;
    let ctx = game.filter_context_for(caster, Some(source.id));
    let mut granted = Vec::new();
    for ability in &source.abilities {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        if !static_ability.is_active(game, source.id) {
            continue;
        }
        let Some(spec) = static_ability.grant_spec() else {
            continue;
        };
        if spec.zone == zone
            && matches!(spec.grantable, crate::grant::Grantable::PlayFrom)
            && !spec.cast_this_way_grants.is_empty()
            && spec.filter.matches(&spell_as_cast, &ctx, game)
        {
            granted.extend(spec.cast_this_way_grants.iter().cloned());
        }
    }
    for ability in granted {
        game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
            stack_id,
            ability.id(),
            Some(ability),
        );
    }
}

/// Revert a spell cast that failed during the casting process.
///
/// Per MTG rules, if casting fails at any point before completion,
/// the game state returns to before the cast was proposed.

/// Result of finalizing a spell cast, containing info needed for triggers.
pub(super) struct SpellCastResult {
    /// The new object ID of the spell on the stack
    pub(super) new_id: ObjectId,
    /// Who cast the spell
    pub(super) caster: PlayerId,
    /// Which zone the spell was cast from.
    pub(super) from_zone: Zone,
}

fn parse_simple_mana_marker_cost(text: &str) -> Option<crate::mana::ManaCost> {
    let mut pips = Vec::new();
    let mut rest = text;
    while let Some(open_idx) = rest.find('{') {
        let after_open = &rest[open_idx + 1..];
        let Some(close_idx) = after_open.find('}') else {
            return None;
        };
        let symbol_text = &after_open[..close_idx];
        let mut alternatives = Vec::new();
        for part in symbol_text.split('/') {
            let symbol = match part.to_ascii_uppercase().as_str() {
                "W" => crate::mana::ManaSymbol::White,
                "U" => crate::mana::ManaSymbol::Blue,
                "B" => crate::mana::ManaSymbol::Black,
                "R" => crate::mana::ManaSymbol::Red,
                "G" => crate::mana::ManaSymbol::Green,
                "C" => crate::mana::ManaSymbol::Colorless,
                "X" => crate::mana::ManaSymbol::X,
                digits => crate::mana::ManaSymbol::Generic(digits.parse::<u8>().ok()?),
            };
            alternatives.push(symbol);
        }
        if alternatives.is_empty() {
            return None;
        }
        pips.push(alternatives);
        rest = &after_open[close_idx + 1..];
    }
    (!pips.is_empty()).then(|| crate::mana::ManaCost::from_pips(pips))
}

fn spell_escalate_cost(obj: &crate::object::Object) -> Option<crate::mana::ManaCost> {
    obj.abilities.iter().find_map(|ability| {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        let display = static_ability.display();
        let tail = display
            .strip_prefix("Escalate ")
            .or_else(|| display.strip_prefix("escalate "))?;
        parse_simple_mana_marker_cost(tail)
    })
}

fn add_repeated_mana_cost(
    base: &crate::mana::ManaCost,
    add: &crate::mana::ManaCost,
    times: usize,
) -> crate::mana::ManaCost {
    if times == 0 {
        return base.clone();
    }
    let mut pips = base.pips().to_vec();
    for _ in 0..times {
        pips.extend(add.pips().iter().cloned());
    }
    crate::mana::ManaCost::from_pips(pips)
}

fn casting_method_matches_alternative_name(
    game: &GameState,
    caster: PlayerId,
    obj: &crate::object::Object,
    casting_method: &CastingMethod,
    expected_name: &str,
) -> bool {
    let method = match casting_method {
        CastingMethod::Alternative(idx) => obj.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        } => crate::decision::resolve_play_from_alternative_method(game, caster, obj, *zone, *idx),
        _ => None,
    };
    method.is_some_and(|method| method.name().eq_ignore_ascii_case(expected_name))
}

fn alternative_cast_label(
    game: &GameState,
    caster: PlayerId,
    obj_id: ObjectId,
    casting_method: &CastingMethod,
) -> Option<String> {
    let obj = game.object(obj_id)?;
    let method = match casting_method {
        CastingMethod::Alternative(idx) => obj.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => crate::decision::resolve_play_from_alternative_method(game, caster, obj, *zone, *idx)
            .or_else(|| obj.cast_alternative_method.clone()),
        _ => None,
    }?;
    let name = method.name();
    (!name.is_empty()).then(|| name.to_string())
}

/// Finalize a spell cast by paying remaining costs and creating the stack entry.
/// Returns the spell cast info for trigger checking.
///
/// `stack_id` is the spell already moved to stack during proposal (per 601.2a).
pub(super) fn finalize_spell_cast(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    _state: &mut PriorityLoopState,
    spell_id: ObjectId,
    from_zone: Zone,
    caster: PlayerId,
    targets: Vec<Target>,
    target_assignments: Vec<crate::game_state::TargetAssignment>,
    x_value: Option<u32>,
    casting_method: CastingMethod,
    mut optional_costs_paid: OptionalCostsPaid,
    chosen_modes: Option<Vec<usize>>,
    mut mana_spent_to_cast: ManaPool,
    keyword_payment_contributions: Vec<KeywordPaymentContribution>,
    stack_entry_tagged_objects: std::collections::HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>,
    stack_entry_effect_outcomes: std::collections::HashMap<
        crate::effect::EffectId,
        crate::effect::EffectOutcome,
    >,
    payment_trace: &mut Vec<CostStep>,
    mana_already_paid: bool,
    stack_id: ObjectId,
    provenance: ProvNodeId,
    _decision_maker: &mut impl DecisionMaker,
) -> Result<SpellCastResult, GameLoopError> {
    use crate::decision::calculate_effective_mana_cost_with_chosen_targets_for_casting_method;
    let _ = payment_trace;

    // Get the mana cost, alternative additional cost, and exile count based on casting method.
    let (base_mana_cost, _alternative_additional_cost, granted_escape_exile_count) =
        if let Some(obj) = game.object(spell_id) {
            let base_mana_cost = crate::decision::spell_mana_cost_for_cast(
                game,
                caster,
                obj,
                &casting_method,
                from_zone,
            );
            match &casting_method {
                CastingMethod::Normal | CastingMethod::FaceDown => {
                    (base_mana_cost, crate::cost::TotalCost::free(), None)
                }
                CastingMethod::SplitOtherHalf | CastingMethod::Fuse => {
                    (base_mana_cost, crate::cost::TotalCost::free(), None)
                }
                CastingMethod::Alternative(idx) => {
                    if let Some(method) = obj
                        .alternative_casts
                        .get(*idx)
                        .or(obj.cast_alternative_method.as_ref())
                    {
                        if let Some(total_cost) = method.total_cost() {
                            (base_mana_cost, total_cost.clone(), None)
                        } else {
                            (base_mana_cost, crate::cost::TotalCost::free(), None)
                        }
                    } else {
                        (base_mana_cost, crate::cost::TotalCost::free(), None)
                    }
                }
                CastingMethod::GrantedEscape { exile_count, .. } => (
                    base_mana_cost,
                    crate::cost::TotalCost::free(),
                    Some(*exile_count),
                ),
                CastingMethod::GrantedFlashback => {
                    (base_mana_cost, crate::cost::TotalCost::free(), None)
                }
                CastingMethod::PlayFrom {
                    use_alternative: None,
                    ..
                } => (base_mana_cost, crate::cost::TotalCost::free(), None),
                CastingMethod::PlayFrom {
                    use_alternative: Some(idx),
                    zone,
                    ..
                }
                | CastingMethod::SplitOtherHalfPlayFrom {
                    use_alternative: idx,
                    zone,
                    ..
                } => crate::decision::resolve_play_from_alternative_method(
                    game, caster, obj, *zone, *idx,
                )
                .or_else(|| obj.cast_alternative_method.clone())
                .map(|method| {
                    if let Some(total_cost) = method.total_cost() {
                        (base_mana_cost.clone(), total_cost.clone(), None)
                    } else {
                        (base_mana_cost.clone(), crate::cost::TotalCost::free(), None)
                    }
                })
                .unwrap_or_else(|| (base_mana_cost, crate::cost::TotalCost::free(), None)),
            }
        } else {
            (None, crate::cost::TotalCost::free(), None)
        };

    // Calculate effective cost and Delve exile count
    let (mut effective_cost, delve_exile_count) = if let Some(ref base_cost) = base_mana_cost {
        if let Some(obj) = game.object(spell_id) {
            let eff_cost = calculate_effective_mana_cost_with_chosen_targets_for_casting_method(
                game,
                caster,
                obj,
                base_cost,
                &targets,
                &casting_method,
            );
            let delve_count = crate::decision::calculate_delve_exile_count_with_targets(
                game,
                caster,
                obj,
                base_cost,
                targets.len(),
            );
            (Some(eff_cost), delve_count)
        } else {
            (base_mana_cost.clone(), 0)
        }
    } else {
        (None, 0)
    };

    if let (Some(cost), Some(modes)) = (effective_cost.as_ref(), chosen_modes.as_ref())
        && modes.len() > 1
        && let Some(obj) = game.object(spell_id)
        && let Some(escalate_cost) = spell_escalate_cost(obj)
    {
        effective_cost = Some(add_repeated_mana_cost(
            cost,
            &escalate_cost,
            modes.len().saturating_sub(1),
        ));
    }

    // Pay Delve cost (exile cards from graveyard)
    if delve_exile_count > 0 {
        // Collect cards to exile for Delve
        let cards_to_exile: Vec<ObjectId> = if let Some(player) = game.player(caster) {
            player
                .graveyard
                .iter()
                .filter(|&&id| id != spell_id) // Don't exile the spell being cast (shouldn't be in GY, but safety)
                .take(delve_exile_count as usize)
                .copied()
                .collect()
        } else {
            Vec::new()
        };

        // Move to exile (move_object handles removal from old zone)
        for card_id in cards_to_exile {
            game.move_object_by_effect(card_id, Zone::Exile);
        }
    }

    // Pay the mana cost (using effective cost with reductions applied)
    // Skip if mana was already paid via pip-by-pip payment
    if !mana_already_paid && let Some(cost) = effective_cost {
        let x = x_value.unwrap_or(0);
        let before_pool = game.player(caster).map(|player| player.mana_pool.clone());
        if !game.try_pay_mana_cost_with_reason(
            caster,
            Some(spell_id),
            &cost,
            x,
            crate::costs::PaymentReason::CastSpell,
        ) {
            return Err(GameLoopError::InvalidState(
                "Cannot pay mana cost".to_string(),
            ));
        }
        let after_pool = game.player(caster).map(|player| player.mana_pool.clone());
        if let (Some(before), Some(after)) = (before_pool, after_pool) {
            mana_spent_to_cast.white += before.white.saturating_sub(after.white);
            mana_spent_to_cast.blue += before.blue.saturating_sub(after.blue);
            mana_spent_to_cast.black += before.black.saturating_sub(after.black);
            mana_spent_to_cast.red += before.red.saturating_sub(after.red);
            mana_spent_to_cast.green += before.green.saturating_sub(after.green);
            mana_spent_to_cast.colorless += before.colorless.saturating_sub(after.colorless);
        }
    }

    // Pay granted escape additional cost (exile cards from graveyard)
    if let Some(exile_count) = granted_escape_exile_count {
        // First, collect cards to exile (immutable borrow)
        let cards_to_exile: Vec<ObjectId> = if let Some(player) = game.player(caster) {
            player
                .graveyard
                .iter()
                .filter(|&&id| id != spell_id)
                .take(exile_count as usize)
                .copied()
                .collect()
        } else {
            Vec::new()
        };

        if cards_to_exile.len() < exile_count as usize {
            return Err(GameLoopError::InvalidState(
                "Not enough cards in graveyard to exile for escape".to_string(),
            ));
        }

        // Move to exile (move_object handles removal from old zone)
        for card_id in cards_to_exile {
            game.move_object_by_effect(card_id, Zone::Exile);
        }
    }

    // Spell was already moved to stack during proposal (601.2a compliant).
    let mana_spent_total = mana_spent_to_cast.total();
    let new_id = stack_id;
    if let Some(spell_obj) = game.object_mut(new_id) {
        spell_obj.mana_spent_to_cast = mana_spent_to_cast;
        spell_obj.x_value = x_value;
    }
    let escaped = game.object(new_id).is_some_and(|spell_obj| {
        crate::decision::casting_method_matches_alternative_kind(
            game,
            caster,
            spell_obj,
            &casting_method,
            crate::filter::AlternativeCastKind::Escape,
        )
    });
    if escaped {
        optional_costs_paid.mark_label_paid("Escape");
    }
    let blitzed = game.object(new_id).is_some_and(|spell_obj| {
        crate::decision::casting_method_matches_alternative_kind(
            game,
            caster,
            spell_obj,
            &casting_method,
            crate::filter::AlternativeCastKind::Blitz,
        )
    });
    if blitzed {
        optional_costs_paid.mark_label_paid("Blitz");
        if let Some(spell_obj) = game.object_mut(new_id) {
            spell_obj.optional_costs_paid.mark_label_paid("Blitz");
        }
    }
    let evoked = game.object(new_id).is_some_and(|spell_obj| {
        casting_method_matches_alternative_name(game, caster, spell_obj, &casting_method, "Evoke")
    });
    if evoked {
        optional_costs_paid.mark_label_paid("Evoke");
        if let Some(spell_obj) = game.object_mut(new_id) {
            spell_obj.optional_costs_paid.mark_label_paid("Evoke");
        }
    }
    let selected_alternative_label = alternative_cast_label(game, caster, new_id, &casting_method);
    if let Some(label) = selected_alternative_label.as_deref()
        && !label.eq_ignore_ascii_case("Parsed alternative cost")
        && !matches!(
            label.to_ascii_lowercase().as_str(),
            "escape" | "blitz" | "evoke"
        )
    {
        optional_costs_paid.mark_label_paid(&label);
        if let Some(spell_obj) = game.object_mut(new_id) {
            spell_obj.optional_costs_paid.mark_label_paid(&label);
        }
    }

    if let CastingMethod::PlayFrom { source, .. } = &casting_method {
        let source_has_selected_once_grant = game.object(*source).is_some_and(|source_obj| {
            source_obj.abilities.iter().any(|ability| {
                let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                    return false;
                };
                static_ability.grant_spec().is_some_and(|spec| {
                    if matches!(
                        spec.usage_limit,
                        Some(
                            crate::grant::GrantUsageLimit::OnceEachTurn
                                | crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns
                        )
                    ) && matches!(spec.grantable, crate::grant::Grantable::PlayFrom)
                    {
                        return true;
                    }

                    let Some(label) = selected_alternative_label.as_deref() else {
                        return false;
                    };
                    matches!(
                        spec.grantable,
                        crate::grant::Grantable::DerivedAlternativeCast(ref derived)
                            if matches!(
                                derived.usage_limit(),
                                Some(
                                    crate::grant::GrantUsageLimit::OnceEachTurn
                                        | crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns
                                )
                            ) && derived.display_name().eq_ignore_ascii_case(label)
                    )
                })
            })
        });
        if source_has_selected_once_grant {
            game.turn_store
                .grant_cast_uses_this_turn
                .insert((caster, *source));
        }
    }

    // Create stack entry with targets, X value, casting method, optional costs, and chosen modes
    let mut entry = StackEntry::new(new_id, caster)
        .with_provenance(provenance)
        .with_targets(targets.clone())
        .with_target_assignments(target_assignments)
        .with_casting_method(casting_method)
        .with_optional_costs_paid(optional_costs_paid)
        .with_chosen_player(game.chosen_player(new_id))
        .with_chosen_modes(chosen_modes)
        .with_tagged_objects(stack_entry_tagged_objects)
        .with_effect_outcomes(stack_entry_effect_outcomes)
        .with_keyword_payment_contributions(keyword_payment_contributions);
    if let Some(spell_obj) = game.object(new_id).cloned() {
        entry = entry.with_source_info(spell_obj.stable_id, spell_obj.name.clone());
    }
    if let Some(x) = x_value {
        entry = entry.with_x(x);
    }
    game.push_to_stack(entry);

    if let Some(spell_obj) = game.object(new_id).cloned() {
        let current_turn = game.turn.turn_number;
        let ctx = crate::filter::FilterContext::new(caster)
            .with_source(new_id)
            .with_active_player(game.turn.active_player)
            .with_opponents(
                game.turn_store
                    .turn_order
                    .iter()
                    .copied()
                    .filter(|player_id| *player_id != caster)
                    .collect(),
            )
            .with_caster(Some(caster));
        let matching_effects = game
            .effect_store
            .temporary_spell_cost_reductions
            .iter()
            .enumerate()
            .filter_map(|(idx, effect)| {
                if effect.player != caster
                    || effect.is_expired(current_turn, game.turn.active_player)
                {
                    return None;
                }
                let mut cast_filter = effect.filter.clone();
                cast_filter.targets_player = None;
                cast_filter.targets_object = None;
                cast_filter.alternative_cast = None;
                cast_filter.matches(&spell_obj, &ctx, game).then_some(idx)
            })
            .collect::<Vec<_>>();
        for idx in matching_effects {
            if let Some(effect) = game
                .effect_store
                .temporary_spell_cost_reductions
                .get_mut(idx)
                && effect.remaining_uses > 0
                && !effect.applies_to_all_matching_this_turn
            {
                effect.remaining_uses -= 1;
            }
        }
    }
    queue_becomes_targeted_events(
        game,
        trigger_queue,
        &targets,
        new_id,
        caster,
        false,
        provenance,
    );

    if from_zone == Zone::Command {
        game.record_commander_cast_from_command_zone(new_id);
    }

    // Expend: "You expend N as you spend your Nth total mana to cast spells during a turn."
    let prev_mana_spent = game
        .turn_store
        .turn_history
        .mana_spent_to_cast_spells_this_turn
        .get(&caster)
        .copied()
        .unwrap_or(0);
    if mana_spent_total > 0 {
        let new_mana_spent_total = prev_mana_spent.saturating_add(mana_spent_total);
        game.turn_store
            .turn_history
            .mana_spent_to_cast_spells_this_turn
            .insert(caster, new_mana_spent_total);

        for threshold in (prev_mana_spent.saturating_add(1))..=new_mana_spent_total {
            let expend_event_provenance = game
                .alloc_child_event_provenance(provenance, crate::events::EventKind::KeywordAction);
            queue_triggers_from_event(
                game,
                trigger_queue,
                TriggerEvent::new_with_provenance(
                    KeywordActionEvent::new(KeywordActionKind::Expend, caster, new_id, threshold),
                    expend_event_provenance,
                ),
                true,
            );
        }
    }

    Ok(SpellCastResult {
        new_id,
        caster,
        from_zone,
    })
}

/// Run the priority loop using a DecisionMaker (convenience wrapper).
///
/// This drives the priority loop to completion using the provided decision maker.
/// Auto-passes priority when PassPriority is the only available action.
#[allow(clippy::never_loop)] // Loop structure is intentional for clarity
pub fn run_priority_loop_with<D: DecisionMaker>(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut D,
) -> Result<GameProgress, GameLoopError> {
    let mut state = PriorityLoopState::new(game.players_in_game());

    loop {
        // Use decision maker for triggered ability target selection
        let progress = advance_priority_with_dm(game, trigger_queue, decision_maker)?;

        match progress {
            GameProgress::NeedsDecisionCtx(ctx) => {
                // Handle context-based decisions in a loop
                let mut current_ctx = ctx;
                loop {
                    let auto_passed = should_auto_pass_ctx(&current_ctx);
                    let result = if auto_passed {
                        apply_priority_action_with_dm(
                            game,
                            trigger_queue,
                            &mut state,
                            &LegalAction::PassPriority,
                            decision_maker,
                        )
                    } else {
                        apply_decision_context_with_dm(
                            game,
                            trigger_queue,
                            &mut state,
                            &current_ctx,
                            decision_maker,
                        )
                    };

                    // Notify decision maker about auto-pass
                    if auto_passed && let Some(player) = get_priority_player_from_ctx(&current_ctx)
                    {
                        decision_maker.on_auto_pass(game, player);
                    }

                    // Handle errors with checkpoint rollback
                    let result = match result {
                        Ok(progress) => progress,
                        Err(e) => {
                            // Check if we have a checkpoint to restore
                            if let Some(checkpoint) = state.checkpoint.take() {
                                // Notify the decision maker about the rollback
                                decision_maker.on_action_cancelled(game, &format!("{}", e));
                                // Restore game state from checkpoint
                                *game = checkpoint;
                                // Clear any pending action state
                                state.pending_cast = None;
                                state.pending_activation = None;
                                state.pending_method_selection = None;
                                state.pending_mana_ability = None;
                                // Break from inner loop to restart with fresh priority
                                break;
                            } else {
                                // No checkpoint - propagate the error
                                return Err(e);
                            }
                        }
                    };

                    match result {
                        GameProgress::Continue => return Ok(GameProgress::Continue),
                        GameProgress::GameOver(result) => {
                            return Ok(GameProgress::GameOver(result));
                        }
                        GameProgress::NeedsDecisionCtx(next_ctx) => {
                            current_ctx = next_ctx; // Continue the context loop
                        }
                        GameProgress::StackResolved => {
                            // Stack resolved, break from inner loop to re-run advance_priority_with_dm
                            // in the outer loop with the proper decision maker for trigger targeting
                            break;
                        }
                    }
                }
            }
            GameProgress::Continue => return Ok(GameProgress::Continue),
            GameProgress::GameOver(result) => return Ok(GameProgress::GameOver(result)),
            GameProgress::StackResolved => {
                // This shouldn't happen from advance_priority_with_dm, but handle it by continuing
                continue;
            }
        }
    }
}

/// Apply a context-based decision directly using typed decision primitives.
pub fn apply_decision_context_with_dm<D: DecisionMaker>(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    ctx: &crate::decisions::context::DecisionContext,
    decision_maker: &mut D,
) -> Result<GameProgress, GameLoopError> {
    use crate::decisions::context::DecisionContext;

    match ctx {
        DecisionContext::Priority(priority_ctx) => {
            let action = decision_maker.decide_priority(game, priority_ctx);
            apply_priority_action_with_dm(game, trigger_queue, state, &action, decision_maker)
        }
        DecisionContext::Number(number_ctx) => {
            let value = decision_maker.decide_number(game, number_ctx);
            apply_x_value_response(game, trigger_queue, state, value, decision_maker)
        }
        DecisionContext::Targets(targets_ctx) => {
            let targets = decision_maker.decide_targets(game, targets_ctx);
            apply_targets_response(game, trigger_queue, state, &targets, decision_maker)
        }
        DecisionContext::Modes(modes_ctx) => {
            let options: Vec<crate::decisions::context::SelectableOption> = modes_ctx
                .spec
                .modes
                .iter()
                .map(|m| {
                    crate::decisions::context::SelectableOption::with_legality(
                        m.index,
                        m.description.clone(),
                        m.legal,
                    )
                    .with_point_cost(m.point_cost)
                    .with_repeatability(
                        modes_ctx.spec.allow_repeated_modes,
                        Some(modes_ctx.spec.max_modes.min(u32::MAX as usize) as u32),
                    )
                })
                .collect();
            let select_ctx = crate::decisions::context::SelectOptionsContext::new(
                modes_ctx.player,
                modes_ctx.source,
                format!("Choose mode for {}", modes_ctx.spell_name),
                options,
                modes_ctx.spec.min_modes,
                modes_ctx.spec.max_modes,
            );
            let modes = decision_maker.decide_options(game, &select_ctx);
            apply_modes_response(game, trigger_queue, state, &modes, decision_maker)
        }
        DecisionContext::HybridChoice(hybrid_ctx) => {
            let options: Vec<crate::decisions::context::SelectableOption> = hybrid_ctx
                .options
                .iter()
                .map(|o| crate::decisions::context::SelectableOption::new(o.index, o.label.clone()))
                .collect();
            let select_ctx = crate::decisions::context::SelectOptionsContext::new(
                hybrid_ctx.player,
                hybrid_ctx.source,
                format!(
                    "Choose how to pay pip {} of {}",
                    hybrid_ctx.pip_number, hybrid_ctx.spell_name
                ),
                options,
                1,
                1,
            );
            let result = decision_maker.decide_options(game, &select_ctx);
            let choice = result.first().copied().ok_or_else(|| {
                GameLoopError::InvalidState("No hybrid payment choice selected".to_string())
            })?;
            apply_hybrid_choice_response(game, trigger_queue, state, choice, decision_maker)
        }
        DecisionContext::SelectObjects(objects_ctx) => {
            let result = decision_maker.decide_objects(game, objects_ctx);
            let chosen = result.first().copied().ok_or_else(|| {
                GameLoopError::InvalidState("No object selected for required choice".to_string())
            })?;

            if state.pending_activation.as_ref().is_some_and(|pending| {
                matches!(
                    pending.stage,
                    ActivationStage::ChoosingSacrifice | ActivationStage::ChoosingCardCost
                )
            }) {
                apply_sacrifice_target_response(game, trigger_queue, state, chosen, decision_maker)
            } else if state.pending_cast.as_ref().is_some_and(|pending| {
                matches!(
                    pending.stage,
                    CastStage::ChoosingSacrifice | CastStage::ChoosingCardCost
                )
            }) {
                apply_card_cost_choice_response(game, trigger_queue, state, chosen, decision_maker)
            } else {
                Err(GameLoopError::InvalidState(
                    "Unsupported SelectObjects decision in priority loop".to_string(),
                ))
            }
        }
        DecisionContext::SelectOptions(options_ctx) => {
            let result = decision_maker.decide_options(game, options_ctx);

            if game.effect_store.pending_replacement_choice.is_some() {
                let Some(choice) = result.first().copied() else {
                    return Err(GameLoopError::InvalidState(
                        "replacement effect choice requires one selected option".to_string(),
                    ));
                };
                return apply_replacement_choice_response(
                    game,
                    trigger_queue,
                    choice,
                    decision_maker,
                );
            }
            if state.pending_method_selection.is_some() {
                let Some(choice) = result.first().copied() else {
                    return Err(GameLoopError::InvalidState(
                        "casting method choice requires one selected option".to_string(),
                    ));
                };
                return apply_casting_method_choice_response(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }
            if state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingOptionalCosts))
            {
                let choices: Vec<(usize, u32)> = result.into_iter().map(|idx| (idx, 1)).collect();
                return apply_optional_costs_response(
                    game,
                    trigger_queue,
                    state,
                    &choices,
                    decision_maker,
                );
            }
            if state.pending_mana_ability.is_some() {
                let Some(choice) = result.first().copied() else {
                    return Err(GameLoopError::InvalidState(
                        "mana payment choice requires one selected option".to_string(),
                    ));
                };
                return apply_mana_payment_response_mana_ability(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }
            if state
                .pending_activation
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, ActivationStage::ChoosingNextCost))
                || state
                    .pending_cast
                    .as_ref()
                    .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingNextCost))
            {
                let Some(choice) = result.first().copied() else {
                    return Err(GameLoopError::InvalidState(
                        "next cost choice requires one selected option".to_string(),
                    ));
                };
                return apply_next_cost_choice_response(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }
            if state
                .pending_activation
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, ActivationStage::PayingMana))
            {
                let Some(choice) = result.first().copied() else {
                    return Err(GameLoopError::InvalidState(
                        "activation mana pip choice requires one selected option".to_string(),
                    ));
                };
                return apply_pip_payment_response_activation(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }
            if state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::PayingMana))
            {
                let Some(choice) = result.first().copied() else {
                    return Err(GameLoopError::InvalidState(
                        "spell mana pip choice requires one selected option".to_string(),
                    ));
                };
                return apply_pip_payment_response_cast(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }

            Err(GameLoopError::InvalidState(
                "Unsupported SelectOptions decision in priority loop".to_string(),
            ))
        }
        DecisionContext::Distribute(_) | DecisionContext::Counters(_) => {
            if state.pending_activation.as_ref().is_some_and(|pending| {
                pending.pending_remove_counters_among.is_some()
                    || matches!(
                        pending.remaining_cost_steps.first(),
                        Some(ActivationCostStep::Cost(cost))
                            if remove_any_counters_among_effect(cost).is_some()
                    )
            }) {
                let pending = state.pending_activation.take().ok_or_else(|| {
                    GameLoopError::InvalidState(
                        "No pending activation for staged counter-cost decision".to_string(),
                    )
                })?;
                return continue_activation_remove_counters_among_payment(
                    game,
                    trigger_queue,
                    state,
                    pending,
                    decision_maker,
                    Some(ctx),
                );
            }

            let activation_debug = state.pending_activation.as_ref().map(|pending| {
                format!(
                    "stage={}, staged_remove={}, remaining_costs={}",
                    pending.stage,
                    pending.pending_remove_counters_among.is_some(),
                    pending.remaining_cost_steps.len()
                )
            });
            Err(GameLoopError::InvalidState(format!(
                "Unsupported decision context in priority loop: {} (pending_activation={activation_debug:?}, pending_cast={}, pending_mana_ability={})",
                decision_context_name(ctx),
                state.pending_cast.is_some(),
                state.pending_mana_ability.is_some()
            )))
        }
        DecisionContext::Boolean(_)
        | DecisionContext::TextInput(_)
        | DecisionContext::Order(_)
        | DecisionContext::Attackers(_)
        | DecisionContext::Blockers(_)
        | DecisionContext::Colors(_)
        | DecisionContext::Partition(_)
        | DecisionContext::Proliferate(_) => Err(GameLoopError::InvalidState(format!(
            "Unsupported decision context in priority loop: {}",
            decision_context_name(ctx)
        ))),
    }
}

pub(super) fn apply_priority_action_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    action: &LegalAction,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    match action {
        LegalAction::PassPriority => {
            let result = pass_priority(game, &mut state.tracker);

            match result {
                PriorityResult::Continue => {
                    // Next player gets priority, advance again
                    // Use decision maker for triggered ability targeting if available
                    advance_priority_with_dm(game, trigger_queue, decision_maker)
                }
                PriorityResult::StackResolves => {
                    // Resolve top of stack, passing decision maker for ETB replacements, choices, etc.
                    resolve_stack_entry_with_dm_and_triggers(game, decision_maker, trigger_queue)?;
                    // Reset priority to active player
                    reset_priority(game, &mut state.tracker);
                    // Signal that stack resolved - outer loop will call advance_priority_with_dm
                    // with the proper decision maker for trigger target selection
                    Ok(GameProgress::StackResolved)
                }
                PriorityResult::PhaseEnds => Ok(GameProgress::Continue),
            }
        }
        _ => apply_priority_response_with_dm(
            game,
            trigger_queue,
            state,
            &PriorityResponse::PriorityAction(action.clone()),
            decision_maker,
        ),
    }
}

/// Check if we should auto-pass priority for a context-based decision.
/// Returns true if this is a Priority decision with only PassPriority available.
pub(super) fn should_auto_pass_ctx(ctx: &crate::decisions::context::DecisionContext) -> bool {
    if let crate::decisions::context::DecisionContext::Priority(pctx) = ctx {
        pctx.actions.len() == 1 && matches!(pctx.actions[0], LegalAction::PassPriority)
    } else {
        false
    }
}

/// Get the player from a context-based decision, if it's a Priority decision.
pub(super) fn get_priority_player_from_ctx(
    ctx: &crate::decisions::context::DecisionContext,
) -> Option<PlayerId> {
    if let crate::decisions::context::DecisionContext::Priority(pctx) = ctx {
        Some(pctx.player)
    } else {
        None
    }
}

#[cfg(test)]
mod priority_mana_tests {
    use super::*;
    use crate::ability::{
        Ability, AbilityKind, ActivatedAbility, ManaUsageRestriction, ManaUsageSubtypeRequirement,
        RestrictedManaUnit,
    };
    use crate::cards::CardDefinitionBuilder;
    use crate::cards::definitions::{
        basic_mountain, basic_swamp, blood_celebrant, command_tower, ornithopter, phyrexian_tower,
        wall_of_roots, yawgmoth_thran_physician,
    };
    use crate::cards::tokens::treasure_token_definition;
    use crate::color::Color;
    use crate::cost::TotalCost;
    use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use crate::game_state::Phase;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::{StaticAbility, StaticAbilityId};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn arena_style_land_definition() -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Arena Style Land")
            .card_types(vec![CardType::Land])
            .parse_text(
                "{R}, {T}, Exert this land: Add {R}{R}. If that mana is spent on a creature spell, it gains haste until end of turn.",
            )
            .expect("Arena-style mana ability should parse")
    }

    fn jasmine_dragon_tea_shop_definition() -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Jasmine Dragon Tea Shop")
            .card_types(vec![CardType::Land])
            .parse_text(
                "{T}: Add {C}.\n\
                 {T}: Add one mana of any color. Spend this mana only to cast an Ally spell or activate an ability of an Ally source.\n\
                 {5}, {T}: Create a 1/1 white Ally creature token.",
            )
            .expect("Jasmine Dragon Tea Shop should parse")
    }

    fn jasmine_dragon_tea_shop_restricted_mana_game() -> (GameState, PlayerId, ObjectId) {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let tea_shop = jasmine_dragon_tea_shop_definition();
        let tea_shop_id = game.create_object_from_definition(&tea_shop, alice, Zone::Battlefield);
        let restriction = game
            .object(tea_shop_id)
            .expect("Jasmine Dragon Tea Shop should exist")
            .abilities
            .iter()
            .find_map(|ability| {
                let AbilityKind::Activated(activated) = &ability.kind else {
                    return None;
                };
                activated.mana_usage_restrictions.first().cloned()
            })
            .expect("Jasmine Dragon Tea Shop should have restricted mana");
        game.player_mut(alice)
            .expect("alice should exist")
            .add_restricted_mana(RestrictedManaUnit {
                symbol: ManaSymbol::Green,
                source: tea_shop_id,
                source_chosen_creature_type: None,
                restrictions: vec![restriction],
            });
        (game, alice, tea_shop_id)
    }

    fn restricted_mana_ability_index(game: &GameState, source: ObjectId) -> usize {
        game.object(source)
            .expect("source should exist")
            .abilities
            .iter()
            .enumerate()
            .find_map(|(idx, ability)| {
                if matches!(
                    &ability.kind,
                    AbilityKind::Activated(activated)
                        if !activated.mana_usage_restrictions.is_empty()
                ) {
                    Some(idx)
                } else {
                    None
                }
            })
            .expect("source should have a restricted mana ability")
    }

    #[test]
    fn test_variable_mana_ability_can_pay_colored_pip() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let treasure = treasure_token_definition();
        let treasure_id = game.create_object_from_definition(&treasure, alice, Zone::Battlefield);

        assert!(
            mana_ability_can_pay_pip(
                &game,
                treasure_id,
                0,
                None,
                &[ManaSymbol::Black],
                &crate::player::ManaSpendPolicy::default(),
            ),
            "Treasure should be considered able to pay a colored pip"
        );
    }

    #[test]
    fn test_single_flexible_mana_source_cannot_pay_two_colored_pips() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        game.create_object_from_definition(&treasure_token_definition(), alice, Zone::Battlefield);
        let two_color_spell = CardDefinitionBuilder::new(CardId::new(), "Two Color Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::White],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![CardType::Sorcery])
            .build();
        let spell_id = game.create_object_from_definition(&two_color_spell, alice, Zone::Hand);

        let actions = crate::decision::compute_legal_actions(&game, alice);

        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )),
            "one any-color mana source should not make a two-colored-pip spell legal"
        );
    }

    #[test]
    fn test_single_flexible_mana_source_can_pay_one_colored_pip() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        game.create_object_from_definition(&treasure_token_definition(), alice, Zone::Battlefield);
        let one_color_spell = CardDefinitionBuilder::new(CardId::new(), "One Color Probe")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Sorcery])
            .build();
        let spell_id = game.create_object_from_definition(&one_color_spell, alice, Zone::Hand);

        let actions = crate::decision::compute_legal_actions(&game, alice);

        assert!(
            actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )),
            "one any-color mana source should still make a one-colored-pip spell legal"
        );
    }

    #[test]
    fn test_tapped_lands_do_not_make_spell_castable() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let plains_one = game.create_object_from_definition(
            &crate::cards::definitions::basic_plains(),
            alice,
            Zone::Battlefield,
        );
        let plains_two = game.create_object_from_definition(
            &crate::cards::definitions::basic_plains(),
            alice,
            Zone::Battlefield,
        );
        game.tap(plains_one);
        game.tap(plains_two);

        let creature = CardDefinitionBuilder::new(CardId::new(), "Two Mana White Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .build();
        let spell_id = game.create_object_from_definition(&creature, alice, Zone::Hand);

        let actions = crate::decision::compute_legal_actions(&game, alice);

        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )),
            "two tapped Plains should not make a {{1}}{{W}} creature legal to cast"
        );
    }

    #[test]
    fn test_tapped_lands_plus_one_floating_mana_do_not_make_two_mana_spell_castable() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let plains_one = game.create_object_from_definition(
            &crate::cards::definitions::basic_plains(),
            alice,
            Zone::Battlefield,
        );
        let plains_two = game.create_object_from_definition(
            &crate::cards::definitions::basic_plains(),
            alice,
            Zone::Battlefield,
        );
        game.tap(plains_one);
        game.tap(plains_two);
        game.player_mut(alice)
            .expect("Alice should exist")
            .mana_pool
            .add(ManaSymbol::White, 1);

        let creature = CardDefinitionBuilder::new(CardId::new(), "Two Mana White Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .build();
        let spell_id = game.create_object_from_definition(&creature, alice, Zone::Hand);

        let actions = crate::decision::compute_legal_actions(&game, alice);

        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )),
            "one floating white and tapped lands should not make a {{1}}{{W}} creature legal"
        );
    }

    #[test]
    fn test_restricted_mana_for_chosen_type_creature_spell_grants_uncounterable() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let cavern = CardDefinitionBuilder::new(CardId::new(), "Cavern Test")
            .card_types(vec![CardType::Land])
            .build();
        let cavern_id = game.create_object_from_definition(&cavern, alice, Zone::Battlefield);

        let restriction = ManaUsageRestriction::CastSpell {
            card_types: vec![CardType::Creature],
            subtype_requirement: Some(ManaUsageSubtypeRequirement::ChosenTypeOfSource),
            restrict_to_matching_spell: true,
            grant_uncounterable: true,
            enters_with_counters: vec![],
            granted_abilities: vec![],
        };
        game.object_mut(cavern_id)
            .expect("cavern test land should exist")
            .abilities
            .push(Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: TotalCost::free(),
                    effects: crate::resolution::ResolutionProgram::default(),
                    choices: vec![],
                    timing: crate::ability::ActivationTiming::AnyTime,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: Some(vec![ManaSymbol::Green]),
                    activation_condition: None,
                    mana_usage_restrictions: vec![restriction.clone()],
                    is_loyalty_ability: false,
                }),
                functional_zones: vec![Zone::Battlefield],
            });
        game.set_chosen_creature_type(cavern_id, Subtype::Giant);

        let matching_spell = CardDefinitionBuilder::new(CardId::new(), "Matching Giant")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Giant])
            .build();
        let matching_spell_id =
            game.create_object_from_definition(&matching_spell, alice, Zone::Stack);
        assert!(
            mana_ability_can_pay_pip(
                &game,
                cavern_id,
                0,
                Some(matching_spell_id),
                &[ManaSymbol::Green],
                &crate::player::ManaSpendPolicy::default(),
            ),
            "restricted mana ability should pay for a creature spell of the chosen type"
        );

        let nonmatching_spell = CardDefinitionBuilder::new(CardId::new(), "Wrong Type")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf])
            .build();
        let nonmatching_spell_id =
            game.create_object_from_definition(&nonmatching_spell, alice, Zone::Stack);
        assert!(
            !mana_ability_can_pay_pip(
                &game,
                cavern_id,
                0,
                Some(nonmatching_spell_id),
                &[ManaSymbol::Green],
                &crate::player::ManaSpendPolicy::default(),
            ),
            "restricted mana ability should reject creature spells of the wrong subtype"
        );

        game.player_mut(alice)
            .expect("alice should exist")
            .add_restricted_mana(RestrictedManaUnit {
                symbol: ManaSymbol::Green,
                source: cavern_id,
                source_chosen_creature_type: Some(Subtype::Giant),
                restrictions: vec![restriction.clone()],
            });
        let spent = spend_pool_symbol(&mut game, alice, ManaSymbol::Green, Some(matching_spell_id))
            .expect("restricted mana should be spendable on matching spell");
        apply_spent_mana_bonuses(&mut game, Some(matching_spell_id), &spent);

        assert!(
            game.object(matching_spell_id)
                .expect("matching spell should still be on stack")
                .abilities
                .iter()
                .any(|ability| matches!(
                    &ability.kind,
                    AbilityKind::Static(static_ability)
                        if static_ability.id() == StaticAbilityId::CantBeCountered
                )),
            "spending restricted Cavern-style mana should make the matching spell uncounterable"
        );
    }

    #[test]
    fn james_wandering_dad_follow_him_restricted_mana_only_pays_for_activated_abilities() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let james = CardDefinitionBuilder::new(CardId::new(), "James, Wandering Dad // Follow Him")
            .card_types(vec![CardType::Creature])
            .parse_text("{T}: Add {C}{C}. Spend this mana only to activate abilities.")
            .expect("James mana ability text should parse");

        let restriction = james
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated)
                    if !activated.mana_usage_restrictions.is_empty() =>
                {
                    activated.mana_usage_restrictions.first().cloned()
                }
                _ => None,
            })
            .expect("James mana ability should include a usage restriction");

        let source_id = game.new_object_id();
        game.player_mut(alice)
            .expect("alice should exist")
            .add_restricted_mana(RestrictedManaUnit {
                symbol: ManaSymbol::Colorless,
                source: source_id,
                source_chosen_creature_type: None,
                restrictions: vec![restriction],
            });

        let mana_rock = CardDefinitionBuilder::new(CardId::new(), "Ability Target")
            .card_types(vec![CardType::Artifact])
            .build();
        let mana_rock_id = game.create_object_from_definition(&mana_rock, alice, Zone::Battlefield);
        assert!(
            spend_pool_symbol(&mut game, alice, ManaSymbol::Colorless, Some(mana_rock_id))
                .is_some(),
            "James restricted mana should be spendable to activate abilities"
        );

        let mut game = setup_game();
        game.player_mut(alice)
            .expect("alice should exist")
            .add_restricted_mana(RestrictedManaUnit {
                symbol: ManaSymbol::Colorless,
                source: source_id,
                source_chosen_creature_type: None,
                restrictions: vec![crate::ability::ManaUsageRestriction::ActivateAbility],
            });

        let spell = CardDefinitionBuilder::new(CardId::new(), "Spell Target")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_definition(&spell, alice, Zone::Stack);
        assert!(
            spend_pool_symbol(&mut game, alice, ManaSymbol::Colorless, Some(spell_id)).is_none(),
            "James restricted mana should not be spendable to cast spells"
        );
    }

    #[test]
    fn jasmine_dragon_tea_shop_restricted_mana_pays_only_ally_spells_or_ally_source_abilities() {
        let (mut game, alice, _) = jasmine_dragon_tea_shop_restricted_mana_game();
        let ally_spell = CardDefinitionBuilder::new(CardId::new(), "Ally Spell")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Ally])
            .build();
        let ally_spell_id = game.create_object_from_definition(&ally_spell, alice, Zone::Stack);
        assert!(
            spend_pool_symbol(&mut game, alice, ManaSymbol::Green, Some(ally_spell_id)).is_some(),
            "Jasmine Dragon Tea Shop restricted mana should pay for Ally spells"
        );

        let (mut game, alice, _) = jasmine_dragon_tea_shop_restricted_mana_game();
        let non_ally_spell = CardDefinitionBuilder::new(CardId::new(), "Non-Ally Spell")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf])
            .build();
        let non_ally_spell_id =
            game.create_object_from_definition(&non_ally_spell, alice, Zone::Stack);
        assert!(
            spend_pool_symbol(&mut game, alice, ManaSymbol::Green, Some(non_ally_spell_id))
                .is_none(),
            "Jasmine Dragon Tea Shop restricted mana should reject non-Ally spells"
        );

        let (mut game, alice, _) = jasmine_dragon_tea_shop_restricted_mana_game();
        let ally_source = CardDefinitionBuilder::new(CardId::new(), "Ally Ability Source")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Ally])
            .parse_text("{1}: You gain 1 life.")
            .expect("Ally ability source should parse");
        let ally_source_id =
            game.create_object_from_definition(&ally_source, alice, Zone::Battlefield);
        assert!(
            spend_pool_symbol(&mut game, alice, ManaSymbol::Green, Some(ally_source_id)).is_some(),
            "Jasmine Dragon Tea Shop restricted mana should pay for abilities of Ally sources"
        );

        let (mut game, alice, _) = jasmine_dragon_tea_shop_restricted_mana_game();
        let non_ally_source = CardDefinitionBuilder::new(CardId::new(), "Non-Ally Ability Source")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf])
            .parse_text("{1}: You gain 1 life.")
            .expect("non-Ally ability source should parse");
        let non_ally_source_id =
            game.create_object_from_definition(&non_ally_source, alice, Zone::Battlefield);
        assert!(
            spend_pool_symbol(
                &mut game,
                alice,
                ManaSymbol::Green,
                Some(non_ally_source_id)
            )
            .is_none(),
            "Jasmine Dragon Tea Shop restricted mana should reject abilities of non-Ally sources"
        );
    }

    #[test]
    fn stacked_activated_ability_preserves_mana_usage_restrictions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = CardDefinitionBuilder::new(CardId::new(), "Sarkhan Test")
            .card_types(vec![CardType::Planeswalker])
            .build();
        let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);

        let restriction = ManaUsageRestriction::CastSpellMatching {
            filter: ObjectFilter::default().with_subtype(Subtype::Dragon),
            restrict_to_matching_spell: true,
            grant_uncounterable: false,
            enters_with_counters: vec![],
            granted_abilities: vec![],
        };
        let entry = StackEntry::ability(
            source_id,
            alice,
            crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::add_mana_of_any_color_restricted(
                    crate::effect::Value::Fixed(2),
                    crate::color::Color::ALL.to_vec(),
                ),
            ]),
        )
        .with_mana_usage_restrictions(vec![restriction], None);
        game.push_to_stack(entry);

        let mut dm = crate::decision::AutoPassDecisionMaker;
        resolve_stack_entry_with(&mut game, &mut dm)
            .expect("stacked loyalty-style mana ability should resolve");

        let restricted_units = game
            .player(alice)
            .expect("player should exist")
            .restricted_mana
            .clone();
        assert_eq!(restricted_units.len(), 2);
        let produced_symbol = restricted_units[0].symbol;

        let dragon_spell = CardDefinitionBuilder::new(CardId::new(), "Dragon Spell")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Dragon])
            .build();
        let dragon_spell_id = game.create_object_from_definition(&dragon_spell, alice, Zone::Stack);
        assert!(
            spend_pool_symbol(&mut game, alice, produced_symbol, Some(dragon_spell_id)).is_some(),
            "restricted mana produced by the stacked ability should pay for Dragon spells"
        );

        let elf_spell = CardDefinitionBuilder::new(CardId::new(), "Elf Spell")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf])
            .build();
        let elf_spell_id = game.create_object_from_definition(&elf_spell, alice, Zone::Stack);
        assert!(
            spend_pool_symbol(&mut game, alice, produced_symbol, Some(elf_spell_id)).is_none(),
            "restricted mana produced by the stacked ability should reject non-Dragon spells"
        );
    }

    #[test]
    fn test_bonus_mana_can_still_pay_noncreature_spells_but_only_buffs_matching_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source_id = game.new_object_id();
        let restriction = ManaUsageRestriction::CastSpell {
            card_types: vec![CardType::Creature],
            subtype_requirement: None,
            restrict_to_matching_spell: false,
            grant_uncounterable: false,
            enters_with_counters: vec![(crate::object::CounterType::PlusOnePlusOne, 1)],
            granted_abilities: vec![],
        };

        game.player_mut(alice)
            .expect("alice should exist")
            .add_restricted_mana(RestrictedManaUnit {
                symbol: ManaSymbol::Green,
                source: source_id,
                source_chosen_creature_type: None,
                restrictions: vec![restriction.clone()],
            });

        let creature_spell = CardDefinitionBuilder::new(CardId::new(), "Creature Spell")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_spell_id =
            game.create_object_from_definition(&creature_spell, alice, Zone::Stack);
        assert!(
            pool_symbol_count(&game, alice, ManaSymbol::Green, Some(creature_spell_id)) >= 1,
            "bonus-bearing mana should still be available for creature spells"
        );

        let spent = spend_pool_symbol(&mut game, alice, ManaSymbol::Green, Some(creature_spell_id))
            .expect("bonus-bearing mana should be spendable on creature spells");
        apply_spent_mana_bonuses(&mut game, Some(creature_spell_id), &spent);
        assert!(
            game.object(creature_spell_id)
                .expect("creature spell should remain on stack")
                .abilities
                .iter()
                .any(|ability| matches!(
                    &ability.kind,
                    AbilityKind::Static(static_ability)
                        if static_ability.id() == StaticAbilityId::EnterWithCounters
                )),
            "spending bonus-bearing mana on a creature spell should grant an ETB counter bonus"
        );

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice)
            .expect("alice should exist")
            .add_restricted_mana(RestrictedManaUnit {
                symbol: ManaSymbol::Green,
                source: source_id,
                source_chosen_creature_type: None,
                restrictions: vec![restriction],
            });

        let noncreature_spell = CardDefinitionBuilder::new(CardId::new(), "Noncreature Spell")
            .card_types(vec![CardType::Sorcery])
            .build();
        let noncreature_spell_id =
            game.create_object_from_definition(&noncreature_spell, alice, Zone::Stack);
        let spent = spend_pool_symbol(
            &mut game,
            alice,
            ManaSymbol::Green,
            Some(noncreature_spell_id),
        )
        .expect("bonus-bearing mana should still be spendable on noncreature spells");
        apply_spent_mana_bonuses(&mut game, Some(noncreature_spell_id), &spent);
        assert!(
            game.object(noncreature_spell_id)
                .expect("noncreature spell should remain on stack")
                .abilities
                .iter()
                .all(|ability| !matches!(
                    &ability.kind,
                    AbilityKind::Static(static_ability)
                        if static_ability.id() == StaticAbilityId::EnterWithCounters
                )),
            "bonus-bearing mana should not add ETB counter text to noncreature spells"
        );
    }

    #[test]
    fn test_bonus_mana_grants_temporary_static_ability_to_matching_spell() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source_id = game.new_object_id();
        let restriction = ManaUsageRestriction::CastSpell {
            card_types: vec![CardType::Creature],
            subtype_requirement: None,
            restrict_to_matching_spell: false,
            grant_uncounterable: false,
            enters_with_counters: vec![],
            granted_abilities: vec![StaticAbilityId::Haste],
        };

        game.player_mut(alice)
            .expect("alice should exist")
            .add_restricted_mana(RestrictedManaUnit {
                symbol: ManaSymbol::Red,
                source: source_id,
                source_chosen_creature_type: None,
                restrictions: vec![restriction],
            });

        let creature_spell = CardDefinitionBuilder::new(CardId::new(), "Creature Spell")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_spell_id =
            game.create_object_from_definition(&creature_spell, alice, Zone::Stack);
        let spent = spend_pool_symbol(&mut game, alice, ManaSymbol::Red, Some(creature_spell_id))
            .expect("bonus-bearing mana should be spendable on creature spells");
        apply_spent_mana_bonuses(&mut game, Some(creature_spell_id), &spent);

        assert!(
            game.current_has_static_ability_id(creature_spell_id, StaticAbilityId::Haste),
            "matching spell should gain haste from spent mana"
        );

        let permanent_id = game
            .move_object_by_effect(creature_spell_id, Zone::Battlefield)
            .expect("creature spell should resolve to the battlefield");
        assert!(
            game.current_has_static_ability_id(permanent_id, StaticAbilityId::Haste),
            "stack-to-battlefield movement should preserve the temporary haste grant"
        );

        game.cleanup_temporary_object_static_ability_grants_end_of_turn();
        assert!(
            !game.current_has_static_ability_id(permanent_id, StaticAbilityId::Haste),
            "temporary haste grant should expire at end of turn"
        );
    }

    #[test]
    fn arena_style_exert_mana_grants_haste_through_cast_flow() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let arena = arena_style_land_definition();
        let arena_id = game.create_object_from_definition(&arena, alice, Zone::Battlefield);
        let arena_ability_index = restricted_mana_ability_index(&game, arena_id);

        game.player_mut(alice)
            .expect("alice should exist")
            .mana_pool
            .add(ManaSymbol::Red, 1);

        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut decision_maker = SelectFirstDecisionMaker;
        apply_priority_response_with_dm(
            &mut game,
            &mut trigger_queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::ActivateManaAbility {
                source: arena_id,
                ability_index: arena_ability_index,
            }),
            &mut decision_maker,
        )
        .expect("Arena-style mana ability should activate");

        let restricted_red = game
            .player(alice)
            .expect("alice should exist")
            .restricted_mana
            .iter()
            .filter(|unit| unit.symbol == ManaSymbol::Red)
            .count();
        assert_eq!(
            restricted_red, 2,
            "Arena-style ability should produce two restricted red mana"
        );

        let creature = CardDefinitionBuilder::new(CardId::new(), "Arena-Funded Warrior")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Creature])
            .build();
        let creature_id = game.create_object_from_definition(&creature, alice, Zone::Hand);

        apply_priority_response_with_dm(
            &mut game,
            &mut trigger_queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::CastSpell {
                spell_id: creature_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            }),
            &mut decision_maker,
        )
        .expect("creature spell should be cast with Arena mana");

        let stack_creature_id = game
            .stack
            .last()
            .expect("creature spell should be on the stack")
            .object_id;
        assert!(
            game.current_has_static_ability_id(stack_creature_id, StaticAbilityId::Haste),
            "creature spell should gain haste while on the stack from Arena mana"
        );

        resolve_stack_entry(&mut game).expect("creature spell should resolve");
        let permanent_id = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.name == "Arena-Funded Warrior")
            })
            .expect("creature should resolve to the battlefield");
        assert!(
            game.current_has_static_ability_id(permanent_id, StaticAbilityId::Haste),
            "creature permanent should keep haste after resolving"
        );
    }

    #[test]
    fn arena_style_mana_paid_with_nested_mana_ability_keeps_restrictions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        let mountain_id =
            game.create_object_from_definition(&basic_mountain(), alice, Zone::Battlefield);
        let arena = arena_style_land_definition();
        let arena_id = game.create_object_from_definition(&arena, alice, Zone::Battlefield);
        let arena_ability_index = restricted_mana_ability_index(&game, arena_id);

        let mut trigger_queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut decision_maker = SelectFirstDecisionMaker;
        let progress = apply_priority_response_with_dm(
            &mut game,
            &mut trigger_queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::ActivateManaAbility {
                source: arena_id,
                ability_index: arena_ability_index,
            }),
            &mut decision_maker,
        )
        .expect("Arena-style mana ability should ask how to pay its red activation cost");
        assert!(
            matches!(
                progress,
                GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::SelectOptions(_)
                )
            ),
            "Arena-style mana ability should need a mana-payment decision when no red is floating"
        );

        apply_priority_response_with_dm(
            &mut game,
            &mut trigger_queue,
            &mut state,
            &PriorityResponse::ManaPayment(0),
            &mut decision_maker,
        )
        .expect("mountain mana should pay Arena's activation cost");

        assert!(
            game.is_tapped(mountain_id),
            "nested mana ability should tap the Mountain used to pay Arena's activation cost"
        );
        let restricted_red = game
            .player(alice)
            .expect("alice should exist")
            .restricted_mana
            .iter()
            .filter(|unit| unit.symbol == ManaSymbol::Red)
            .count();
        assert_eq!(
            restricted_red, 2,
            "Arena-style ability should still produce two restricted red mana after a nested mana-payment decision"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_mana_ability_undo_safe_for_basic_tap_sources() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mountain_id =
            game.create_object_from_definition(&basic_mountain(), alice, Zone::Battlefield);
        assert!(
            mana_ability_is_undo_safe(&game, mountain_id, 0),
            "basic tap-for-mana land should be undo-safe"
        );

        let command_tower_id =
            game.create_object_from_definition(&command_tower(), alice, Zone::Battlefield);
        assert!(
            mana_ability_is_undo_safe(&game, command_tower_id, 0),
            "tap-for-any-color mana ability should be undo-safe"
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn test_mana_ability_undo_not_safe_for_stateful_activations() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let wall_id =
            game.create_object_from_definition(&wall_of_roots(), alice, Zone::Battlefield);
        let wall_mana_index = game
            .object(wall_id)
            .and_then(|obj| {
                obj.abilities
                    .iter()
                    .position(|ability| ability.is_mana_ability())
            })
            .expect("wall of roots should have a mana ability");
        assert!(
            !mana_ability_is_undo_safe(&game, wall_id, wall_mana_index),
            "Wall of Roots-style counter costs should not be undo-safe"
        );

        let blood_celebrant_id =
            game.create_object_from_definition(&blood_celebrant(), alice, Zone::Battlefield);
        let blood_celebrant_mana_index = game
            .object(blood_celebrant_id)
            .and_then(|obj| {
                obj.abilities
                    .iter()
                    .position(|ability| ability.is_mana_ability())
            })
            .expect("blood celebrant should have a mana ability");
        assert!(
            !mana_ability_is_undo_safe(&game, blood_celebrant_id, blood_celebrant_mana_index),
            "mana abilities with non-mana side effects should not be undo-safe"
        );

        let treasure_id = game.create_object_from_definition(
            &treasure_token_definition(),
            alice,
            Zone::Battlefield,
        );
        assert!(
            !mana_ability_is_undo_safe(&game, treasure_id, 0),
            "tap+sacrifice mana abilities should not be undo-safe"
        );
    }

    #[test]
    fn test_pip_payment_mana_ability_restricts_any_color_choice() {
        struct AlwaysRedDecisionMaker;
        impl DecisionMaker for AlwaysRedDecisionMaker {
            fn decide_colors(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::ColorsContext,
            ) -> Vec<Color> {
                vec![Color::Red; ctx.count as usize]
            }
        }

        let mut game = setup_game();
        let mut trigger_queue = TriggerQueue::new();
        let alice = PlayerId::from_index(0);
        let mut dm = AlwaysRedDecisionMaker;

        let treasure = treasure_token_definition();
        let treasure_id = game.create_object_from_definition(&treasure, alice, Zone::Battlefield);

        let action = ManaPipPaymentAction::ActivateManaAbility {
            source_id: treasure_id,
            ability_index: 0,
        };
        let mut payment_trace = Vec::new();
        let mut mana_spent = ManaPool::default();
        let black_pip = vec![ManaSymbol::Black];

        let pip_paid = execute_pip_payment_action(
            &mut game,
            &mut trigger_queue,
            alice,
            None,
            crate::costs::PaymentReason::Other,
            &black_pip,
            &crate::player::ManaSpendPolicy::default(),
            &action,
            &mut dm,
            &mut payment_trace,
            Some(&mut mana_spent),
        )
        .expect("mana ability activation during pip payment should succeed");

        assert!(
            pip_paid,
            "activating a mana ability for a pip should immediately spend usable mana"
        );

        let pool = &game.player(alice).expect("alice exists").mana_pool;
        assert_eq!(
            pool.black, 0,
            "generated mana should be consumed for the pip"
        );
        assert_eq!(pool.red, 0, "disallowed color should not be produced");
        assert_eq!(
            mana_spent.black, 1,
            "spent mana tracking should reflect the auto-paid pip"
        );
        assert!(
            !game.battlefield.contains(&treasure_id),
            "treasure should be sacrificed as part of activation cost"
        );
        let _ = payment_trace;
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn test_black_pip_payment_options_include_phyrexian_tower_sacrifice_ability() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let yawgmoth_id = game.create_object_from_definition(
            &yawgmoth_thran_physician(),
            alice,
            Zone::Battlefield,
        );
        game.create_object_from_definition(&basic_swamp(), alice, Zone::Battlefield);
        game.create_object_from_definition(&phyrexian_tower(), alice, Zone::Battlefield);
        game.create_object_from_definition(&ornithopter(), alice, Zone::Battlefield);

        let options = build_pip_payment_options(
            &game,
            alice,
            &[ManaSymbol::Black],
            Some(&[ManaSymbol::Black]),
            &crate::player::ManaSpendPolicy::default(),
            false,
            Some(yawgmoth_id),
            crate::costs::PaymentReason::ActivateAbility,
            &mut dm,
        );

        let potential = crate::decision::compute_potential_mana(&game, alice);
        assert!(
            potential.black >= 2,
            "potential mana should include Phyrexian Tower's black sacrifice output"
        );

        let descriptions: Vec<_> = options
            .iter()
            .map(|option| option.description.as_str())
            .collect();
        assert!(
            descriptions
                .iter()
                .any(|description| description.contains("Tap Swamp: Add {B}")),
            "sanity check: Swamp should be offered, got {descriptions:?}"
        );
        assert!(
            descriptions
                .iter()
                .any(|description| description.contains("Tap Phyrexian Tower: Add {B}{B}")),
            "Phyrexian Tower's sacrifice mana ability should be offered for a black pip, got {descriptions:?}"
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn test_phyrexian_tower_alternative_mana_abilities_are_one_payment_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);

        game.create_object_from_definition(&phyrexian_tower(), alice, Zone::Battlefield);
        game.create_object_from_definition(&ornithopter(), alice, Zone::Battlefield);

        let spell = CardDefinitionBuilder::new(CardId::new(), "Tower Overcount Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Colorless],
            ]))
            .card_types(vec![CardType::Sorcery])
            .build();
        let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);

        let actions = crate::decision::compute_legal_actions(&game, alice);

        assert!(
            !actions.iter().any(|action| matches!(
                action,
                LegalAction::CastSpell {
                    spell_id: id,
                    from_zone: Zone::Hand,
                    casting_method: CastingMethod::Normal,
                } if *id == spell_id
            )),
            "Phyrexian Tower can activate either its {{C}} ability or its sacrifice-for-{{B}}{{B}} ability, not both"
        );
    }

    #[test]
    fn test_build_pip_payment_options_adds_krrik_life_for_plain_black_pip() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let helper = CardDefinitionBuilder::new(CardId::new(), "Krrik Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let helper_id = game.create_object_from_definition(&helper, alice, Zone::Battlefield);
        game.object_mut(helper_id)
            .expect("helper should exist")
            .abilities
            .push(Ability::static_ability(
                StaticAbility::krrik_black_mana_may_be_paid_with_life(),
            ));

        let options = build_pip_payment_options(
            &game,
            alice,
            &[ManaSymbol::Black],
            Some(&[ManaSymbol::Black]),
            &crate::player::ManaSpendPolicy::default(),
            game.player_can_pay_black_with_life_for_reason(
                alice,
                Some(helper_id),
                crate::costs::PaymentReason::CastSpell,
            ),
            None,
            crate::costs::PaymentReason::CastSpell,
            &mut dm,
        );

        assert!(
            options
                .iter()
                .any(|option| matches!(option.action, ManaPipPaymentAction::PayLife(2))),
            "a printed {{B}} pip should offer Krrik's pay-2-life option"
        );
    }

    #[test]
    fn test_build_pip_payment_options_does_not_add_krrik_life_to_announced_phyrexian_black() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let mut dm = crate::decision::SelectFirstDecisionMaker;

        let options = build_pip_payment_options(
            &game,
            alice,
            &[ManaSymbol::Black],
            Some(&[ManaSymbol::Black, ManaSymbol::Life(2)]),
            &crate::player::ManaSpendPolicy::default(),
            true,
            None,
            crate::costs::PaymentReason::CastSpell,
            &mut dm,
        );

        assert!(
            options
                .iter()
                .all(|option| !matches!(option.action, ManaPipPaymentAction::PayLife(2))),
            "Krrik should not create a second life-payment option for a printed Phyrexian pip"
        );
    }
}

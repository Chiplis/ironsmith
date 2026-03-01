/// Collect all available casting methods for a spell.
/// Returns a list of CastingMethodOption structs for each method that can be used.
fn collect_available_casting_methods(
    game: &GameState,
    player: PlayerId,
    spell_id: ObjectId,
    from_zone: Zone,
) -> Vec<crate::decision::CastingMethodOption> {
    use crate::decision::{
        CastingMethodOption, can_cast_spell, can_cast_with_alternative_from_hand,
    };

    let mut methods = Vec::new();

    let Some(spell) = game.object(spell_id) else {
        return methods;
    };

    // Check normal casting method
    if from_zone == Zone::Hand && can_cast_spell(game, player, spell, &CastingMethod::Normal) {
        let cost_desc = spell
            .mana_cost
            .as_ref()
            .map(format_mana_cost_simple)
            .unwrap_or_else(|| "0".to_string());
        methods.push(CastingMethodOption {
            method: CastingMethod::Normal,
            name: "Normal".to_string(),
            cost_description: cost_desc,
        });
    }

    // Check alternative casting methods from hand
    if from_zone == Zone::Hand {
        for (idx, alt_cast) in spell.alternative_casts.iter().enumerate() {
            if alt_cast.cast_from_zone() == Zone::Hand
                && can_cast_with_alternative_from_hand(game, player, spell, spell_id, alt_cast)
            {
                let (name, cost_desc) = format_alternative_method(alt_cast, spell);
                methods.push(CastingMethodOption {
                    method: CastingMethod::Alternative(idx),
                    name,
                    cost_description: cost_desc,
                });
            }
        }
    }

    methods
}

/// Format a mana cost in simple text form (e.g., "{3}{U}{U}").
fn format_mana_cost_simple(cost: &crate::mana::ManaCost) -> String {
    use crate::mana::ManaSymbol;

    let mut parts = Vec::new();
    for pip in cost.pips() {
        if pip.len() == 1 {
            parts.push(match &pip[0] {
                ManaSymbol::Generic(n) => format!("{{{}}}", n),
                ManaSymbol::Colorless => "{C}".to_string(),
                ManaSymbol::White => "{W}".to_string(),
                ManaSymbol::Blue => "{U}".to_string(),
                ManaSymbol::Black => "{B}".to_string(),
                ManaSymbol::Red => "{R}".to_string(),
                ManaSymbol::Green => "{G}".to_string(),
                ManaSymbol::Snow => "{S}".to_string(),
                ManaSymbol::X => "{X}".to_string(),
                ManaSymbol::Life(n) => format!("{{{}/P}}", n),
            });
        } else {
            let alts: Vec<String> = pip
                .iter()
                .map(|s| match s {
                    ManaSymbol::Generic(n) => format!("{}", n),
                    ManaSymbol::Colorless => "C".to_string(),
                    ManaSymbol::White => "W".to_string(),
                    ManaSymbol::Blue => "U".to_string(),
                    ManaSymbol::Black => "B".to_string(),
                    ManaSymbol::Red => "R".to_string(),
                    ManaSymbol::Green => "G".to_string(),
                    ManaSymbol::Snow => "S".to_string(),
                    ManaSymbol::X => "X".to_string(),
                    ManaSymbol::Life(n) => format!("P/{}", n),
                })
                .collect();
            parts.push(format!("{{{}}}", alts.join("/")));
        }
    }
    if parts.is_empty() {
        "0".to_string()
    } else {
        parts.join("")
    }
}

fn cost_effects_for_casting_method(
    game: &GameState,
    caster: PlayerId,
    spell: &crate::object::Object,
    casting_method: &CastingMethod,
) -> Vec<Effect> {
    match casting_method {
        CastingMethod::Alternative(idx) => spell
            .alternative_casts
            .get(*idx)
            .map(|method| method.cost_effects().to_vec())
            .unwrap_or_default(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        } => crate::decision::resolve_play_from_alternative_method(game, caster, spell, *zone, *idx)
            .map(|method| method.cost_effects().to_vec())
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn effect_references_x_for_cost(effect: &Effect) -> bool {
    use crate::effect::Value;

    if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
        return sacrifice.count == Value::X;
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        return choose.count.dynamic_x;
    }

    false
}

fn max_x_from_cost_effects(
    game: &GameState,
    caster: PlayerId,
    source: ObjectId,
    effects: &[Effect],
) -> Option<u32> {
    use crate::effect::Value;
    use crate::effects::helpers::resolve_player_filter;

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let ctx = ExecutionContext::new(source, caster, &mut dm);
    let filter_ctx = ctx.filter_context(game);

    let mut max_x: Option<u32> = None;

    for effect in effects {
        if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
            if sacrifice.count != Value::X {
                continue;
            }

            let player_id = match resolve_player_filter(game, &sacrifice.player, &ctx) {
                Ok(id) => id,
                Err(_) => caster,
            };

            let matching = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                .filter(|(id, obj)| {
                    obj.controller == player_id
                        && sacrifice.filter.matches(obj, &filter_ctx, game)
                        && game.can_be_sacrificed(*id)
                })
                .count() as u32;

            max_x = Some(max_x.map_or(matching, |prev| prev.min(matching)));
            continue;
        }

        if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
            if !choose.count.dynamic_x {
                continue;
            }

            let chooser_id = match resolve_player_filter(game, &choose.chooser, &ctx) {
                Ok(id) => id,
                Err(_) => caster,
            };
            let zone = choose.filter.zone.unwrap_or(choose.zone);

            let mut matches = |id: &ObjectId| -> bool {
                let Some(obj) = game.object(*id) else {
                    return false;
                };
                if choose.filter.other && obj.id == source {
                    return false;
                }
                choose.filter.matches(obj, &filter_ctx, game)
            };

            let matching = match zone {
                Zone::Battlefield => game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(&mut matches)
                    .count(),
                Zone::Hand => game
                    .player(chooser_id)
                    .map(|player| player.hand.iter().copied().filter(&mut matches).count())
                    .unwrap_or(0),
                Zone::Graveyard => game
                    .player(chooser_id)
                    .map(|player| {
                        if choose.top_only {
                            player
                                .graveyard
                                .iter()
                                .copied()
                                .rev()
                                .find(|id| matches(id))
                                .map(|_| 1usize)
                                .unwrap_or(0)
                        } else {
                            player
                                .graveyard
                                .iter()
                                .copied()
                                .filter(&mut matches)
                                .count()
                        }
                    })
                    .unwrap_or(0),
                Zone::Library => game
                    .player(chooser_id)
                    .map(|player| {
                        if choose.top_only {
                            player
                                .library
                                .last()
                                .copied()
                                .filter(|id| matches(id))
                                .map(|_| 1usize)
                                .unwrap_or(0)
                        } else {
                            player.library.iter().copied().filter(&mut matches).count()
                        }
                    })
                    .unwrap_or(0),
                _ => 0,
            } as u32;

            max_x = Some(max_x.map_or(matching, |prev| prev.min(matching)));
        }
    }

    max_x
}

fn compute_spell_cast_x_bounds(
    game: &GameState,
    caster: PlayerId,
    stack_id: ObjectId,
    casting_method: &CastingMethod,
    mana_cost_to_pay: Option<&crate::mana::ManaCost>,
) -> (bool, u32) {
    let Some(spell) = game.object(stack_id) else {
        return (false, 0);
    };

    let printed_has_x = spell.mana_cost.as_ref().is_some_and(|cost| cost.has_x());
    let pay_has_x = mana_cost_to_pay.is_some_and(|cost| cost.has_x());

    let mut cost_effects = cost_effects_for_casting_method(game, caster, spell, casting_method);
    cost_effects.extend(spell.cost_effects.iter().cloned());

    let effects_need_x = cost_effects.iter().any(effect_references_x_for_cost);
    let needs_x = printed_has_x || pay_has_x || effects_need_x;
    if !needs_x {
        return (false, 0);
    }

    let mut max_x = None;

    if pay_has_x && let Some(cost) = mana_cost_to_pay {
        let allow_any_color = game.can_spend_mana_as_any_color(caster, Some(stack_id));
        max_x = Some(
            compute_potential_mana(game, caster)
                .max_x_for_cost_with_any_color(cost, allow_any_color),
        );
    }

    if let Some(max_cost) = max_x_from_cost_effects(game, caster, stack_id, &cost_effects) {
        max_x = Some(max_x.map_or(max_cost, |prev| prev.min(max_cost)));
    }

    (true, max_x.unwrap_or(0))
}

/// Format an alternative casting method's name and cost description.
fn format_alternative_method(
    method: &crate::alternative_cast::AlternativeCastingMethod,
    spell: &crate::object::Object,
) -> (String, String) {
    use crate::alternative_cast::AlternativeCastingMethod;

    match method {
        AlternativeCastingMethod::Flashback { cost, .. } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Flashback".to_string(), cost_desc)
        }
        AlternativeCastingMethod::JumpStart => {
            // Jump-start uses the spell's mana cost plus discarding a card
            let cost_desc = spell
                .mana_cost
                .as_ref()
                .map(format_mana_cost_simple)
                .unwrap_or_else(|| "0".to_string());
            (
                "Jump-Start".to_string(),
                format!("{}, Discard a card", cost_desc),
            )
        }
        AlternativeCastingMethod::Escape { cost, exile_count } => {
            let cost_desc = cost
                .as_ref()
                .map(format_mana_cost_simple)
                .or_else(|| spell.mana_cost.as_ref().map(format_mana_cost_simple))
                .unwrap_or_else(|| "0".to_string());
            (
                "Escape".to_string(),
                format!("{}, Exile {} cards from graveyard", cost_desc, exile_count),
            )
        }
        AlternativeCastingMethod::Bestow { cost, cost_effects } => {
            let mut parts = vec![format_mana_cost_simple(cost)];
            for effect in cost_effects {
                parts.push(format!("{:?}", effect));
            }
            ("Bestow".to_string(), parts.join(", "))
        }
        AlternativeCastingMethod::Composed { .. } => {
            let mana_cost = method.mana_cost();
            let cost_effects = method.cost_effects();
            let name = method.name();
            let mut parts = Vec::new();
            if let Some(mana) = mana_cost {
                parts.push(format_mana_cost_simple(mana));
            }
            for effect in cost_effects {
                parts.push(format!("{:?}", effect));
            }
            let cost_desc = if parts.is_empty() {
                "Free".to_string()
            } else {
                parts.join(", ")
            };
            (name.to_string(), cost_desc)
        }
        AlternativeCastingMethod::MindbreakTrap {
            cost, condition, ..
        } => {
            let cost_desc = format_mana_cost_simple(cost);
            let condition_desc = match condition {
                crate::alternative_cast::TrapCondition::OpponentCastSpells { count } => {
                    format!("If opponent cast {}+ spells this turn", count)
                }
                crate::alternative_cast::TrapCondition::OpponentSearchedLibrary => {
                    "If opponent searched their library".to_string()
                }
                crate::alternative_cast::TrapCondition::OpponentCreatureEntered => {
                    "If opponent had a creature enter".to_string()
                }
                crate::alternative_cast::TrapCondition::CreatureDealtDamageToYou => {
                    "If a creature dealt damage to you".to_string()
                }
            };
            (
                "Trap".to_string(),
                format!("{} ({})", cost_desc, condition_desc),
            )
        }
        AlternativeCastingMethod::Madness { cost } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Madness".to_string(), cost_desc)
        }
        AlternativeCastingMethod::Miracle { cost } => {
            let cost_desc = format_mana_cost_simple(cost);
            ("Miracle".to_string(), cost_desc)
        }
    }
}

/// Helper to extract modal spec from a spell's effects.
///
/// Searches through the spell's effects to find if it has a modal effect (ChooseModeEffect).
/// For compositional effects like ConditionalEffect, this evaluates conditions at cast time
/// to determine which branch's modal spec to use (e.g., Akroma's Will checking YouControlCommander).
/// Returns the modal specification if found.
fn extract_modal_spec_from_spell(
    game: &GameState,
    spell_id: ObjectId,
    controller: PlayerId,
) -> Option<crate::effects::ModalSpec> {
    let obj = game.object(spell_id)?;

    // Check spell effects with context to handle conditional effects like Akroma's Will
    if let Some(ref effects) = obj.spell_effect {
        for effect in effects {
            // Try context-aware extraction first (handles ConditionalEffect)
            if let Some(spec) = effect
                .0
                .get_modal_spec_with_context(game, controller, spell_id)
            {
                return Some(spec);
            }
            // Fall back to simple extraction for direct modal effects
            if let Some(spec) = effect.0.get_modal_spec() {
                return Some(spec);
            }
        }
    }

    None
}

/// Check for modal effects and either prompt for mode selection or continue to optional costs.
///
/// Per MTG rule 601.2b, modes must be chosen before targets.
/// This is called after the spell is proposed (moved to stack).
fn check_modes_or_continue(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check if the spell has modal effects (with context for conditional effects like Akroma's Will)
    if let Some(modal_spec) = extract_modal_spec_from_spell(game, pending.spell_id, pending.caster)
    {
        let player = pending.caster;
        let source = pending.spell_id;

        // Resolve min/max mode counts
        let max_modes = match &modal_spec.max_modes {
            crate::effect::Value::Fixed(n) => *n as usize,
            _ => 1, // Default to 1 for dynamic values during casting
        };
        let min_modes = match &modal_spec.min_modes {
            crate::effect::Value::Fixed(n) => *n as usize,
            _ => max_modes, // Default to max for exact choice
        };

        let spell_name = game
            .object(source)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell".to_string());

        // Set up pending cast for modes stage
        let mut pending = pending;
        pending.stage = CastStage::ChoosingModes;
        state.pending_cast = Some(pending);

        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Modes(
                crate::decisions::context::ModesContext {
                    player,
                    source: Some(source),
                    spell_name,
                    spec: crate::decisions::ModesSpec::new(
                        source,
                        modal_spec
                            .mode_descriptions
                            .iter()
                            .enumerate()
                            .map(|(i, desc)| {
                                crate::decisions::specs::ModeOption::with_legality(
                                    i,
                                    desc.clone(),
                                    true,
                                )
                            })
                            .collect(),
                        min_modes,
                        max_modes,
                    ),
                },
            ),
        ))
    } else {
        // No modal effects, continue to optional costs
        check_optional_costs_or_continue(game, trigger_queue, state, pending, decision_maker)
    }
}

/// Check for optional costs and either prompt for them or continue to targeting/finalization.
///
/// This is called after X value is chosen (or when there's no X cost).
/// Returns the next decision needed or continues the cast.
fn check_optional_costs_or_continue(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check if the spell has optional costs
    let optional_costs = if let Some(obj) = game.object(pending.spell_id) {
        obj.optional_costs.clone()
    } else {
        Vec::new()
    };

    if optional_costs.is_empty() {
        // No optional costs, continue to targeting or finalization
        continue_to_targeting_or_finalize(game, trigger_queue, state, pending, decision_maker)
    } else {
        // Build the optional cost options for the decision
        let player = pending.caster;
        let source = pending.spell_id;

        // Check which costs the player can afford (using potential mana)
        let options: Vec<OptionalCostOption> = optional_costs
            .iter()
            .enumerate()
            .map(|(index, opt_cost)| {
                // Check if player can afford this cost with potential mana
                let affordable = if let Some(mana_cost) = opt_cost.cost.mana_cost() {
                    crate::decision::can_potentially_pay(game, player, mana_cost, 0)
                } else {
                    // For non-mana costs, use the regular check
                    crate::cost::can_pay_cost(game, source, player, &opt_cost.cost).is_ok()
                };

                // Format the cost description
                let cost_description = if let Some(mana) = opt_cost.cost.mana_cost() {
                    format!("{}", mana.mana_value())
                } else {
                    "special".to_string()
                };

                OptionalCostOption {
                    index,
                    label: opt_cost.label,
                    repeatable: opt_cost.repeatable,
                    affordable,
                    cost_description,
                }
            })
            .collect();

        // Set up pending cast for optional costs stage
        let mut pending = pending;
        pending.stage = CastStage::ChoosingOptionalCosts;
        state.pending_cast = Some(pending);

        // Convert to SelectOptionsContext for optional cost selection
        let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
            .iter()
            .map(|opt| {
                crate::decisions::context::SelectableOption::with_legality(
                    opt.index,
                    format!("{}: {}", opt.label, opt.cost_description),
                    opt.affordable,
                )
            })
            .collect();
        let spell_name = game
            .object(source)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell".to_string());
        let ctx = crate::decisions::context::SelectOptionsContext::new(
            player,
            Some(source),
            format!("Choose optional costs for {}", spell_name),
            selectable_options,
            0,             // min - optional costs are optional
            options.len(), // max - can select all
        );
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ))
    }
}

/// Get the effective mana cost for a spell being cast.
///
/// This is called during casting to determine hybrid/Phyrexian pips.
fn get_spell_mana_cost(
    game: &GameState,
    spell_id: ObjectId,
    caster: PlayerId,
    casting_method: &CastingMethod,
) -> Option<crate::mana::ManaCost> {
    let obj = game.object(spell_id)?;
    match casting_method {
        CastingMethod::Normal => obj.mana_cost.clone(),
        CastingMethod::Alternative(idx) => {
            if let Some(method) = obj.alternative_casts.get(*idx) {
                // For composed alternative methods (with cost effects), use mana_cost directly (even if None).
                // For other methods (flashback, etc.), fall back to spell's cost.
                if !method.cost_effects().is_empty() {
                    method.mana_cost().cloned()
                } else {
                    method
                        .mana_cost()
                        .cloned()
                        .or_else(|| obj.mana_cost.clone())
                }
            } else {
                obj.mana_cost.clone()
            }
        }
        CastingMethod::GrantedEscape { .. } => obj.mana_cost.clone(),
        CastingMethod::GrantedFlashback => obj.mana_cost.clone(),
        CastingMethod::PlayFrom {
            use_alternative: None,
            ..
        } => obj.mana_cost.clone(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        } => crate::decision::resolve_play_from_alternative_method(
            game, caster, obj, *zone, *idx,
        )
        .map(|method| {
            if !method.cost_effects().is_empty() {
                method.mana_cost().cloned()
            } else {
                method.mana_cost().cloned().or_else(|| obj.mana_cost.clone())
            }
        })
        .unwrap_or_else(|| obj.mana_cost.clone()),
    }
}

/// Get pips that require announcement (hybrid/Phyrexian pips with multiple options).
///
/// Returns a list of (pip_index, alternatives) for each pip that has multiple payment options.
/// Per MTG rule 601.2b, the player must announce how they will pay these during casting.
fn get_pips_requiring_announcement(
    cost: &crate::mana::ManaCost,
) -> Vec<(usize, Vec<crate::mana::ManaSymbol>)> {
    cost.pips()
        .iter()
        .enumerate()
        .filter(|(_, pip)| pip.len() > 1) // Multiple options = needs choice
        .map(|(i, pip)| (i, pip.clone()))
        .collect()
}

/// Continue the casting process to targeting or mana payment.
///
/// Called when there are no optional costs or after optional costs are chosen.
/// Per MTG rule 601.2b, checks for hybrid/Phyrexian pips first.
fn continue_to_targeting_or_finalize(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Per MTG 601.2b: Check for hybrid/Phyrexian pips that need announcement BEFORE targets
    // Skip if we already have hybrid choices (coming back from AnnouncingCost stage)
    if pending.hybrid_choices.is_empty()
        && let Some(mana_cost) =
            get_spell_mana_cost(game, pending.spell_id, pending.caster, &pending.casting_method)
    {
        let pips_to_announce = get_pips_requiring_announcement(&mana_cost);
        if !pips_to_announce.is_empty() {
            // Need to announce hybrid/Phyrexian choices
            return check_hybrid_announcement_or_continue(
                game,
                trigger_queue,
                state,
                pending,
                pips_to_announce,
                decision_maker,
            );
        }
    }

    // No hybrid/Phyrexian pips (or already announced), continue to targets
    continue_to_targets_or_mana_payment(game, trigger_queue, state, pending, decision_maker)
}

/// Check for hybrid/Phyrexian pips and prompt for announcements.
///
/// Per MTG rule 601.2b, the player announces how they will pay hybrid/Phyrexian costs
/// before targets are chosen.
fn check_hybrid_announcement_or_continue(
    game: &mut GameState,
    _trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    pips_to_announce: Vec<(usize, Vec<crate::mana::ManaSymbol>)>,
    _decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = pending;
    pending.stage = CastStage::AnnouncingCost;
    pending.pending_hybrid_pips = pips_to_announce;

    // Prompt for the first pip
    prompt_for_next_hybrid_pip(game, state, pending)
}

/// Prompt the player for the next hybrid/Phyrexian pip choice.
fn prompt_for_next_hybrid_pip(
    game: &GameState,
    state: &mut PriorityLoopState,
    pending: PendingCast,
) -> Result<GameProgress, GameLoopError> {
    // Get the next pip to announce
    if let Some((pip_idx, alternatives)) = pending.pending_hybrid_pips.first().cloned() {
        let player = pending.caster;
        let source = pending.spell_id;
        let spell_name = game
            .object(source)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell".to_string());

        // Build hybrid options for each alternative
        let options: Vec<crate::decisions::context::HybridOption> = alternatives
            .iter()
            .enumerate()
            .map(|(i, sym)| crate::decisions::context::HybridOption {
                index: i,
                label: format_mana_symbol_for_choice(sym),
                symbol: *sym,
            })
            .collect();

        state.pending_cast = Some(pending);

        // Create a HybridChoice decision for this pip
        let ctx = crate::decisions::context::HybridChoiceContext::new(
            player,
            Some(source),
            spell_name,
            pip_idx + 1, // 1-based for display
            options,
        );
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::HybridChoice(ctx),
        ))
    } else {
        // No more pips to announce - this shouldn't happen, but handle gracefully
        state.pending_cast = Some(pending);
        Err(GameLoopError::InvalidState(
            "No pending hybrid pips to announce".to_string(),
        ))
    }
}

/// Format a mana symbol for display in hybrid/Phyrexian choice.
fn format_mana_symbol_for_choice(sym: &crate::mana::ManaSymbol) -> String {
    use crate::mana::ManaSymbol;
    match sym {
        ManaSymbol::White => "{W} (White mana)".to_string(),
        ManaSymbol::Blue => "{U} (Blue mana)".to_string(),
        ManaSymbol::Black => "{B} (Black mana)".to_string(),
        ManaSymbol::Red => "{R} (Red mana)".to_string(),
        ManaSymbol::Green => "{G} (Green mana)".to_string(),
        ManaSymbol::Colorless => "{C} (Colorless mana)".to_string(),
        ManaSymbol::Generic(n) => format!("{{{}}} ({} generic mana)", n, n),
        ManaSymbol::Snow => "{S} (Snow mana)".to_string(),
        ManaSymbol::Life(n) => format!("{} life (Phyrexian)", n),
        ManaSymbol::X => "{X}".to_string(),
    }
}

/// Continue to target selection or mana payment.
///
/// Called after hybrid/Phyrexian choices are made (or when none are needed).
fn continue_to_targets_or_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Validate that we can still pay the cost after hybrid choices
    // This is necessary because max_x was calculated assuming life payment for Phyrexian pips,
    // but the player may have chosen mana payment instead
    if let Some(ref cost) = pending.mana_cost_to_pay {
        let x_value = pending.x_value.unwrap_or(0);
        let expanded_pips =
            expand_mana_cost_to_pips(cost, x_value as usize, &pending.hybrid_choices);
        let potential = compute_potential_mana(game, pending.caster);

        // Check if we can pay all the expanded pips (excluding life payments)
        let total_mana_needed: usize = expanded_pips
            .iter()
            .filter(|pip| {
                !pip.iter()
                    .any(|s| matches!(s, crate::mana::ManaSymbol::Life(_)))
            })
            .count();

        if potential.total() < total_mana_needed as u32 {
            return Err(GameLoopError::InvalidState(format!(
                "Cannot afford spell: need {} mana but only have {} available. \
                Consider paying life for Phyrexian mana or choosing a lower X value.",
                total_mana_needed,
                potential.total()
            )));
        }
    }

    if pending.remaining_requirements.is_empty() {
        // No targets needed, go to mana payment
        continue_to_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            Vec::new(),
            decision_maker,
        )
    } else {
        // Need to select targets
        let mut pending = pending;
        pending.stage = CastStage::ChoosingTargets;
        let requirements = pending.remaining_requirements.clone();
        let player = pending.caster;
        let source = pending.spell_id;
        let context = game
            .object(source)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| "spell".to_string());

        state.pending_cast = Some(pending);

        // Convert to TargetsContext
        let ctx = crate::decisions::context::TargetsContext::new(
            player,
            source,
            context,
            requirements
                .into_iter()
                .map(|r| crate::decisions::context::TargetRequirementContext {
                    description: r.description,
                    legal_targets: r.legal_targets,
                    min_targets: r.min_targets,
                    max_targets: r.max_targets,
                })
                .collect(),
        );
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(ctx),
        ))
    }
}

/// Continue the casting process to mana payment stage.
///
/// Called after targets are chosen (or when no targets needed).
/// Computes the effective mana cost and checks if payment is possible.
fn continue_to_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    pending: PendingCast,
    targets: Vec<Target>,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::decision::calculate_effective_mana_cost_for_payment_with_chosen_targets;

    let mut pending = pending;
    pending.chosen_targets = targets;

    // Compute the effective mana cost for this spell
    let effective_cost = if let Some(obj) = game.object(pending.spell_id) {
        // Get base cost from casting method
        let base_cost = match &pending.casting_method {
            CastingMethod::Normal => obj.mana_cost.clone(),
            CastingMethod::Alternative(idx) => {
                if let Some(method) = obj.alternative_casts.get(*idx) {
                    // For composed alternative methods (with cost effects), use mana_cost directly (even if None).
                    // For other methods (flashback, etc.), fall back to spell's cost.
                    if !method.cost_effects().is_empty() {
                        method.mana_cost().cloned()
                    } else {
                        method
                            .mana_cost()
                            .cloned()
                            .or_else(|| obj.mana_cost.clone())
                    }
                } else {
                    obj.mana_cost.clone()
                }
            }
            CastingMethod::GrantedEscape { .. } => obj.mana_cost.clone(),
            CastingMethod::GrantedFlashback => obj.mana_cost.clone(),
            CastingMethod::PlayFrom {
                use_alternative: None,
                ..
            } => obj.mana_cost.clone(),
            CastingMethod::PlayFrom {
                use_alternative: Some(idx),
                zone,
                ..
            } => crate::decision::resolve_play_from_alternative_method(
                game, pending.caster, obj, *zone, *idx,
            )
            .map(|method| {
                if !method.cost_effects().is_empty() {
                    method.mana_cost().cloned()
                } else {
                    method.mana_cost().cloned().or_else(|| obj.mana_cost.clone())
                }
            })
            .unwrap_or_else(|| obj.mana_cost.clone()),
        };

        // Apply cost reductions (affinity, delve, convoke, improvise)
        base_cost.map(|bc| {
            calculate_effective_mana_cost_for_payment_with_chosen_targets(
                game,
                pending.caster,
                obj,
                &bc,
                &pending.chosen_targets,
            )
        })
    } else {
        None
    };

    pending.mana_cost_to_pay = effective_cost.clone();

    // Check for ExileFromHand costs that need player choice
    let exile_from_hand_choice_needed =
        if let CastingMethod::Alternative(idx) = &pending.casting_method {
            if let Some(obj) = game.object(pending.spell_id) {
                if let Some(method) = obj.alternative_casts.get(*idx) {
                    // Check for exile from hand requirement in cost_effects
                    if let Some((count, color_filter)) = method.exile_from_hand_requirement() {
                        // Check if there are multiple legal cards to choose from
                        if let Some(player) = game.player(pending.caster) {
                            let matching_cards: Vec<ObjectId> = player
                                .hand
                                .iter()
                                .filter(|&&card_id| {
                                    if card_id == pending.spell_id {
                                        return false;
                                    }
                                    if let Some(filter) = color_filter {
                                        if let Some(card) = game.object(card_id) {
                                            let card_colors = card.colors();
                                            !card_colors.intersection(filter).is_empty()
                                        } else {
                                            false
                                        }
                                    } else {
                                        true
                                    }
                                })
                                .copied()
                                .collect();
                            // Need choice if there are more cards than required
                            matching_cards.len() > count as usize
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

    // If we need to choose cards to exile, prompt for that first
    if exile_from_hand_choice_needed && pending.cards_to_exile.is_empty() {
        // Find the legal cards to exile
        let legal_cards = if let CastingMethod::Alternative(idx) = &pending.casting_method {
            if let Some(obj) = game.object(pending.spell_id) {
                if let Some(method) = obj.alternative_casts.get(*idx) {
                    if let Some((_, color_filter)) = method.exile_from_hand_requirement() {
                        let mut cards = Vec::new();
                        if let Some(player) = game.player(pending.caster) {
                            for &card_id in &player.hand {
                                if card_id == pending.spell_id {
                                    continue;
                                }
                                let matches = if let Some(filter) = color_filter {
                                    if let Some(card) = game.object(card_id) {
                                        let card_colors = card.colors();
                                        !card_colors.intersection(filter).is_empty()
                                    } else {
                                        false
                                    }
                                } else {
                                    true
                                };
                                if matches {
                                    cards.push(card_id);
                                }
                            }
                        }
                        cards
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        pending.stage = CastStage::ChoosingExileFromHand;
        let player = pending.caster;
        let source = pending.spell_id;
        state.pending_cast = Some(pending);

        // Convert to SelectObjectsContext for card to exile selection
        let candidates: Vec<crate::decisions::context::SelectableObject> = legal_cards
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| format!("Card #{}", id.0));
                crate::decisions::context::SelectableObject::new(id, name)
            })
            .collect();
        let ctx = crate::decisions::context::SelectObjectsContext::new(
            player,
            Some(source),
            "Exile a blue card from your hand",
            candidates,
            1,
            Some(1),
        );
        return Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(ctx),
        ));
    }

    pending.stage = CastStage::PayingMana;

    // Store the mana cost in pending for pip-by-pip processing
    pending.mana_cost_to_pay = effective_cost;

    // Continue with pip-by-pip mana payment
    continue_spell_cast_mana_payment(game, trigger_queue, state, pending, decision_maker)
}

/// Continue processing spell cast mana payment pip-by-pip.
fn continue_spell_cast_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let x_value = pending.x_value.unwrap_or(0);

    // Initialize remaining_mana_pips from mana_cost_to_pay if not already done
    // We use take() to clear mana_cost_to_pay so we don't re-populate on recursive calls
    if pending.remaining_mana_pips.is_empty()
        && let Some(cost) = pending.mana_cost_to_pay.take()
    {
        pending.remaining_mana_pips =
            expand_mana_cost_to_pips(&cost, x_value as usize, &pending.hybrid_choices);
    }

    // If no remaining pips, we're done with mana payment - finalize the spell
    if pending.remaining_mana_pips.is_empty() {
        let mana_spent_to_cast = pending.mana_spent_to_cast.clone();
        let result = finalize_spell_cast(
            game,
            trigger_queue,
            state,
            pending.spell_id,
            pending.from_zone,
            pending.caster,
            pending.chosen_targets,
            pending.x_value,
            pending.casting_method,
            pending.optional_costs_paid,
            pending.chosen_modes,
            pending.cards_to_exile,
            mana_spent_to_cast,
            pending.keyword_payment_contributions,
            &mut pending.payment_trace,
            true, // mana_already_paid via pip-by-pip
            pending.stack_id,
            &mut *decision_maker,
        )?;

        // Generate SpellCast event and check for triggers
        let event = TriggerEvent::new(SpellCastEvent::new(
            result.new_id,
            result.caster,
            result.from_zone,
        ));
        let triggers = check_triggers(game, &event);
        for trigger in triggers {
            trigger_queue.add(trigger);
        }

        // Clear checkpoint - spell cast completed successfully
        state.clear_checkpoint();
        reset_priority(game, &mut state.tracker);
        return advance_priority_with_dm(game, trigger_queue, decision_maker);
    }

    // Get the first pip to pay
    let pip = pending.remaining_mana_pips[0].clone();
    let remaining_count = pending.remaining_mana_pips.len() - 1;

    // Build payment options for this pip
    let player_id = pending.caster;
    let source = pending.spell_id;
    let context = game
        .object(source)
        .map(|o| o.name.clone())
        .unwrap_or_else(|| "spell".to_string());

    let allow_any_color = game.can_spend_mana_as_any_color(player_id, Some(source));
    let options = build_pip_payment_options(
        game,
        player_id,
        &pip,
        allow_any_color,
        Some(source),
        &mut *decision_maker,
    );

    // If no options available (shouldn't happen if we validated correctly), error
    if options.is_empty() {
        return Err(GameLoopError::InvalidState(
            "No payment options available for mana pip".to_string(),
        ));
    }

    // Auto-select deterministic pip choices when possible.
    if let Some(auto_choice) = preferred_auto_pip_choice(state, &options) {
        let action = options[auto_choice].action.clone();
        let pip_paid = execute_pip_payment_action(
            game,
            trigger_queue,
            player_id,
            Some(source),
            &action,
            &mut *decision_maker,
            &mut pending.payment_trace,
            Some(&mut pending.mana_spent_to_cast),
        )?;
        queue_mana_ability_event_for_action(game, trigger_queue, &action, player_id);
        drain_pending_trigger_events(game, trigger_queue);
        if pip_paid {
            record_keyword_payment_contribution(
                &mut pending.keyword_payment_contributions,
                &action,
            );
            pending.remaining_mana_pips.remove(0);
        }
        return continue_spell_cast_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }

    let pip_description = format_pip(&pip);

    state.pending_cast = Some(pending);

    // Convert ManaPipPaymentOption to SelectableOption
    let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
        .iter()
        .map(|opt| crate::decisions::context::SelectableOption::new(opt.index, &opt.description))
        .collect();

    let ctx = crate::decisions::context::SelectOptionsContext::mana_pip_payment(
        player_id,
        source,
        context,
        pip_description,
        remaining_count,
        selectable_options,
    );
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::SelectOptions(ctx),
    ))
}

/// Compute available mana payment options for a player during mana ability activation.
///
/// This returns options for:
/// - Available mana abilities that can be activated (excluding the one being paid for)
///   and that can help pay the remaining cost
/// - Option to pay (if enough mana is in pool)
fn compute_mana_ability_payment_options(
    game: &GameState,
    player: PlayerId,
    pending: &PendingManaAbility,
    decision_maker: &mut impl DecisionMaker,
) -> Vec<ManaPaymentOption> {
    use crate::ability::AbilityKind;

    let mut options = Vec::new();

    // Get available mana abilities the player can activate
    // Exclude the mana ability we're trying to pay for
    let mana_abilities = get_available_mana_abilities(game, player, decision_maker);

    // Filter to only abilities that can help pay the cost
    let mut option_index = 0;
    for (perm_id, ability_index, description) in mana_abilities.iter() {
        // Skip the ability we're trying to pay for to avoid infinite loop
        if *perm_id == pending.source && *ability_index == pending.ability_index {
            continue;
        }

        // Get the mana this ability produces and check if it can help pay the cost
        let allow_any_color = game.can_spend_mana_as_any_color(player, Some(pending.source));
        let can_help = if let Some(perm) = game.object(*perm_id)
            && let Some(ability) = perm.abilities.get(*ability_index)
            && let AbilityKind::Activated(mana_ability) = &ability.kind
            && mana_ability.is_mana_ability()
        {
            mana_can_help_pay_cost(
                mana_ability.mana_symbols(),
                &pending.mana_cost,
                game,
                player,
                allow_any_color,
            )
        } else {
            // If we can't determine, include it
            true
        };

        if can_help {
            options.push(ManaPaymentOption {
                index: option_index,
                description: format!(
                    "Tap {}: {}",
                    describe_permanent(game, *perm_id),
                    description
                ),
            });
            option_index += 1;
        }
    }

    // Add option to pay if player has enough mana
    if game.can_pay_mana_cost(player, Some(pending.source), &pending.mana_cost, 0) {
        options.push(ManaPaymentOption {
            index: options.len(),
            description: "Pay mana cost".to_string(),
        });
    }

    options
}

/// Check if mana produced by an ability can help pay a mana cost.
///
/// Returns true if any of the mana symbols can pay any pip in the cost,
/// considering the player's current mana pool.
fn mana_can_help_pay_cost(
    mana_produced: &[crate::mana::ManaSymbol],
    cost: &crate::mana::ManaCost,
    game: &GameState,
    player: PlayerId,
    allow_any_color: bool,
) -> bool {
    use crate::mana::ManaSymbol;

    // Get current mana pool to see what's already available
    let pool = game.player(player).map(|p| &p.mana_pool);

    // Check each pip in the cost to see if the produced mana can help
    for pip in cost.pips() {
        for alternative in pip {
            match alternative {
                // Generic mana can be paid by any colored mana
                ManaSymbol::Generic(_) => {
                    // Any mana helps with generic costs
                    if !mana_produced.is_empty() {
                        return true;
                    }
                }
                // Colored mana must match
                ManaSymbol::White
                | ManaSymbol::Blue
                | ManaSymbol::Black
                | ManaSymbol::Red
                | ManaSymbol::Green => {
                    // If any-color spending is allowed, any mana helps with colored pips
                    if allow_any_color {
                        if !mana_produced.is_empty() {
                            return true;
                        }
                    } else if mana_produced.contains(alternative) {
                        return true;
                    }
                }
                // Colorless mana can only be paid by colorless
                ManaSymbol::Colorless => {
                    if mana_produced.contains(&ManaSymbol::Colorless) {
                        return true;
                    }
                }
                // Snow, life, X - less common, be permissive
                _ => return true,
            }
        }
    }

    // Also check if this mana could help after we pay some colored pips
    // (e.g., we might need {W}{W} and only have one white, so any mana helps with the first)
    // For simplicity, if the cost has any generic component that's not yet payable, any mana helps
    if pool.is_some() {
        let generic_needed = cost
            .pips()
            .iter()
            .filter(|pip| pip.iter().any(|s| matches!(s, ManaSymbol::Generic(_))))
            .count();

        // Very rough heuristic: if there are generic costs and the ability produces any mana
        if generic_needed > 0 && !mana_produced.is_empty() {
            return true;
        }
    }

    false
}

/// Get available mana abilities for a player that can be activated.
///
/// Returns a list of (permanent_id, ability_index, description) tuples.
fn get_available_mana_abilities(
    game: &GameState,
    player: PlayerId,
    decision_maker: &mut impl DecisionMaker,
) -> Vec<(ObjectId, usize, String)> {
    use crate::special_actions::{SpecialAction, can_perform};

    let mut abilities = Vec::new();

    for &perm_id in &game.battlefield {
        let Some(perm) = game.object(perm_id) else {
            continue;
        };

        if perm.controller != player {
            continue;
        }

        for (i, ability) in perm.abilities.iter().enumerate() {
            if ability.is_mana_ability() {
                let action = SpecialAction::ActivateManaAbility {
                    permanent_id: perm_id,
                    ability_index: i,
                };

                if can_perform(&action, game, player, &mut *decision_maker).is_ok() {
                    let desc = describe_mana_ability(&ability.kind);
                    abilities.push((perm_id, i, desc));
                }
            }
        }
    }

    abilities
}

/// Describe a mana ability for display.
fn describe_mana_ability(kind: &crate::ability::AbilityKind) -> String {
    use crate::ability::AbilityKind;
    use crate::mana::ManaSymbol;

    if let AbilityKind::Activated(mana_ability) = kind
        && mana_ability.is_mana_ability()
    {
        let mana_strs: Vec<&str> = mana_ability
            .mana_symbols()
            .iter()
            .map(|m| match m {
                ManaSymbol::White => "{W}",
                ManaSymbol::Blue => "{U}",
                ManaSymbol::Black => "{B}",
                ManaSymbol::Red => "{R}",
                ManaSymbol::Green => "{G}",
                ManaSymbol::Colorless => "{C}",
                _ => "mana",
            })
            .collect();
        format!("Add {}", mana_strs.join(""))
    } else {
        "Add mana".to_string()
    }
}

/// Describe a permanent for display.
fn describe_permanent(game: &GameState, id: ObjectId) -> String {
    game.object(id)
        .map(|obj| obj.name.clone())
        .unwrap_or_else(|| "Unknown".to_string())
}

/// Get legal sacrifice targets for a filter.
fn get_legal_sacrifice_targets(
    game: &GameState,
    player: PlayerId,
    source: ObjectId,
    filter: &ObjectFilter,
) -> Vec<ObjectId> {
    let ctx = FilterContext {
        you: Some(player),
        source: Some(source),
        ..Default::default()
    };
    game.battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id)
                .is_some_and(|obj| filter.matches(obj, &ctx, game))
        })
        .collect()
}

/// Continue the activation process based on current stage.
fn continue_activation(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingActivation,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // No re-validation needed: costs have already been paid (tap, sacrifice, exile,
    // etc.) and all ability data is captured in the PendingActivation. Per MTG rule
    // 602.2, once activation begins and costs are paid, it completes.

    match pending.stage {
        ActivationStage::ChoosingX => {
            // Need to choose X value first
            let max_x = if let Some(ref cost) = pending.mana_cost_to_pay {
                let allow_any_color =
                    game.can_spend_mana_as_any_color(pending.activator, Some(pending.source));
                compute_potential_mana(game, pending.activator)
                    .max_x_for_cost_with_any_color(cost, allow_any_color)
            } else {
                0
            };

            state.pending_activation = Some(pending.clone());

            let ctx = crate::decisions::context::NumberContext::x_value(
                pending.activator,
                pending.source,
                max_x,
            );
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Number(ctx),
            ))
        }
        ActivationStage::ChoosingSacrifice => {
            // Get the next sacrifice cost to pay
            if let Some((filter, description)) = pending.remaining_sacrifice_costs.first().cloned()
            {
                let legal_targets =
                    get_legal_sacrifice_targets(game, pending.activator, pending.source, &filter);

                if legal_targets.is_empty() {
                    // No valid targets - this shouldn't happen if can_pay_cost was checked
                    return Err(GameLoopError::InvalidState(
                        "No valid sacrifice targets".to_string(),
                    ));
                }

                let player = pending.activator;
                let source = pending.source;
                state.pending_activation = Some(pending);

                // Convert to SelectObjectsContext for sacrifice selection
                let candidates: Vec<crate::decisions::context::SelectableObject> = legal_targets
                    .iter()
                    .map(|&id| {
                        let name = game
                            .object(id)
                            .map(|o| o.name.clone())
                            .unwrap_or_else(|| format!("Permanent #{}", id.0));
                        crate::decisions::context::SelectableObject::new(id, name)
                    })
                    .collect();
                let ctx = crate::decisions::context::SelectObjectsContext::new(
                    player,
                    Some(source),
                    format!("Choose a creature to sacrifice: {}", description),
                    candidates,
                    1,
                    Some(1),
                );
                Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::SelectObjects(ctx),
                ))
            } else {
                // No more sacrifice costs - recompute target requirements with current game state
                // This ensures sacrificed creatures are no longer in the legal targets list
                pending.remaining_requirements = extract_target_requirements(
                    game,
                    &pending.effects,
                    pending.activator,
                    Some(pending.source),
                );

                // Per MTG rule 602.2b (which references 601.2b), check for hybrid/Phyrexian pips
                // These must be announced BEFORE targets are chosen
                if pending.hybrid_choices.is_empty()
                    && let Some(ref mana_cost) = pending.mana_cost_to_pay
                {
                    let pips_to_announce = get_pips_requiring_announcement(mana_cost);
                    if !pips_to_announce.is_empty() {
                        pending.pending_hybrid_pips = pips_to_announce;
                        pending.stage = ActivationStage::AnnouncingCost;
                        return continue_activation(
                            game,
                            trigger_queue,
                            state,
                            pending,
                            decision_maker,
                        );
                    }
                }

                // Move to next stage
                if !pending.remaining_requirements.is_empty() {
                    pending.stage = ActivationStage::ChoosingTargets;
                } else if pending.mana_cost_to_pay.is_some() {
                    pending.stage = ActivationStage::PayingMana;
                } else {
                    pending.stage = ActivationStage::ReadyToFinalize;
                }
                continue_activation(game, trigger_queue, state, pending, decision_maker)
            }
        }
        ActivationStage::AnnouncingCost => {
            // Handle hybrid/Phyrexian mana announcement (per MTG rule 601.2b via 602.2b)
            if pending.pending_hybrid_pips.is_empty() {
                // All hybrid pips announced - validate that we can still pay the cost
                // This is necessary because max_x was calculated assuming life payment for Phyrexian pips,
                // but the player may have chosen mana payment instead
                if let Some(ref cost) = pending.mana_cost_to_pay {
                    let x_value = pending.x_value.unwrap_or(0);
                    let expanded_pips =
                        expand_mana_cost_to_pips(cost, x_value, &pending.hybrid_choices);
                    let potential = compute_potential_mana(game, pending.activator);

                    // Check if we can pay all the expanded pips
                    let total_mana_needed: usize = expanded_pips
                        .iter()
                        .filter(|pip| {
                            !pip.iter()
                                .any(|s| matches!(s, crate::mana::ManaSymbol::Life(_)))
                        })
                        .count();

                    if potential.total() < total_mana_needed as u32 {
                        return Err(GameLoopError::InvalidState(format!(
                            "Cannot afford ability: need {} mana but only have {} available. \
                            Consider paying life for Phyrexian mana or choosing a lower X value.",
                            total_mana_needed,
                            potential.total()
                        )));
                    }
                }

                // All hybrid pips announced, move to targets
                if !pending.remaining_requirements.is_empty() {
                    pending.stage = ActivationStage::ChoosingTargets;
                } else if pending.mana_cost_to_pay.is_some() {
                    pending.stage = ActivationStage::PayingMana;
                } else {
                    pending.stage = ActivationStage::ReadyToFinalize;
                }
                return continue_activation(game, trigger_queue, state, pending, decision_maker);
            }

            // Prompt for the next hybrid pip
            let (pip_idx, alternatives) = pending.pending_hybrid_pips[0].clone();
            let player = pending.activator;
            let source = pending.source;
            let ability_name = game
                .object(source)
                .map(|o| format!("{}'s ability", o.name))
                .unwrap_or_else(|| "ability".to_string());

            // Build hybrid options for each alternative
            let options: Vec<crate::decisions::context::HybridOption> = alternatives
                .iter()
                .enumerate()
                .map(|(i, sym)| crate::decisions::context::HybridOption {
                    index: i,
                    label: format_mana_symbol_for_choice(sym),
                    symbol: *sym,
                })
                .collect();

            state.pending_activation = Some(pending);

            // Create a HybridChoice decision for this pip
            let ctx = crate::decisions::context::HybridChoiceContext::new(
                player,
                Some(source),
                ability_name,
                pip_idx + 1, // 1-based for display
                options,
            );
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::HybridChoice(ctx),
            ))
        }
        ActivationStage::ChoosingTargets => {
            if pending.remaining_requirements.is_empty() {
                // No more targets needed
                if pending.mana_cost_to_pay.is_some() {
                    pending.stage = ActivationStage::PayingMana;
                } else {
                    pending.stage = ActivationStage::ReadyToFinalize;
                }
                continue_activation(game, trigger_queue, state, pending, decision_maker)
            } else {
                let requirements = pending.remaining_requirements.clone();
                let player = pending.activator;
                let source = pending.source;
                let context = game
                    .object(source)
                    .map(|o| format!("{}'s ability", o.name))
                    .unwrap_or_else(|| "ability".to_string());

                state.pending_activation = Some(pending);

                // Convert to TargetsContext
                let ctx = crate::decisions::context::TargetsContext::new(
                    player,
                    source,
                    context,
                    requirements
                        .into_iter()
                        .map(|r| crate::decisions::context::TargetRequirementContext {
                            description: r.description,
                            legal_targets: r.legal_targets,
                            min_targets: r.min_targets,
                            max_targets: r.max_targets,
                        })
                        .collect(),
                );
                Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Targets(ctx),
                ))
            }
        }
        ActivationStage::PayingMana => {
            let x_value = pending.x_value.unwrap_or(0);

            // Initialize remaining_mana_pips from mana_cost_to_pay if not already done
            // We use take() to clear mana_cost_to_pay so we don't re-populate on recursive calls
            if pending.remaining_mana_pips.is_empty()
                && let Some(cost) = pending.mana_cost_to_pay.take()
            {
                pending.remaining_mana_pips =
                    expand_mana_cost_to_pips(&cost, x_value, &pending.hybrid_choices);
            }

            // If no remaining pips, we're done with mana payment
            if pending.remaining_mana_pips.is_empty() {
                pending.stage = ActivationStage::ReadyToFinalize;
                return continue_activation(game, trigger_queue, state, pending, decision_maker);
            }

            // Get the first pip to pay
            let pip = pending.remaining_mana_pips[0].clone();
            let remaining_count = pending.remaining_mana_pips.len() - 1;

            // Build payment options for this pip
            let player_id = pending.activator;
            let source = pending.source;
            let context = game
                .object(source)
                .map(|o| format!("{}'s ability", o.name))
                .unwrap_or_else(|| "ability".to_string());

            let allow_any_color = game.can_spend_mana_as_any_color(player_id, Some(source));
            let options = build_pip_payment_options(
                game,
                player_id,
                &pip,
                allow_any_color,
                None,
                &mut *decision_maker,
            );

            // If no options available (shouldn't happen if we validated correctly), error
            if options.is_empty() {
                return Err(GameLoopError::InvalidState(
                    "No payment options available for mana pip".to_string(),
                ));
            }

            // Auto-select deterministic pip choices when possible.
            if let Some(auto_choice) = preferred_auto_pip_choice(state, &options) {
                let action = options[auto_choice].action.clone();
                let pip_paid = execute_pip_payment_action(
                    game,
                    trigger_queue,
                    player_id,
                    Some(source),
                    &action,
                    &mut *decision_maker,
                    &mut pending.payment_trace,
                    None,
                )?;
                queue_mana_ability_event_for_action(game, trigger_queue, &action, player_id);
                drain_pending_trigger_events(game, trigger_queue);
                if pip_paid {
                    pending.remaining_mana_pips.remove(0);
                }
                return continue_activation(game, trigger_queue, state, pending, decision_maker);
            }

            let pip_description = format_pip(&pip);

            state.pending_activation = Some(pending);

            // Convert ManaPipPaymentOption to SelectableOption
            let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
                .iter()
                .map(|opt| {
                    crate::decisions::context::SelectableOption::new(opt.index, &opt.description)
                })
                .collect();

            let ctx = crate::decisions::context::SelectOptionsContext::mana_pip_payment(
                player_id,
                source,
                context,
                pip_description,
                remaining_count,
                selectable_options,
            );
            Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ))
        }
        ActivationStage::ReadyToFinalize => {
            // Record activation for per-turn-limited abilities
            if pending.is_once_per_turn {
                game.record_ability_activation(pending.source, pending.ability_index);
            }

            // Create ability stack entry with targets
            let mut entry =
                StackEntry::ability(pending.source, pending.activator, pending.effects.clone())
                    .with_source_info(pending.source_stable_id, pending.source_name.clone())
                    .with_source_snapshot(pending.source_snapshot.clone());
            entry.targets = pending.chosen_targets.clone();

            // Pass X value to stack entry so it's available during resolution
            if let Some(x) = pending.x_value {
                entry = entry.with_x(x as u32);
            }

            game.push_to_stack(entry);
            queue_becomes_targeted_events(
                game,
                trigger_queue,
                &pending.chosen_targets,
                pending.source,
                pending.activator,
                true,
            );
            queue_ability_activated_event(
                game,
                trigger_queue,
                pending.source,
                pending.activator,
                false,
                Some(pending.source_stable_id),
            );

            // Clear pending state and checkpoint - action completed successfully
            state.pending_activation = None;
            state.clear_checkpoint();
            reset_priority(game, &mut state.tracker);
            advance_priority_with_dm(game, trigger_queue, decision_maker)
        }
    }
}

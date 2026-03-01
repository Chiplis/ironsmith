// ============================================================================
// Pip-by-Pip Mana Payment Helpers
// ============================================================================

/// Expand a ManaCost into individual pips, expanding X pips by the chosen value.
/// Also applies hybrid_choices to replace multi-symbol pips with the chosen symbol.
fn expand_mana_cost_to_pips(
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

fn preferred_auto_pip_choice(
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
fn build_pip_payment_options(
    game: &GameState,
    player: PlayerId,
    pip: &[crate::mana::ManaSymbol],
    allow_any_color: bool,
    source_for_pip_alternatives: Option<ObjectId>,
    decision_maker: &mut impl DecisionMaker,
) -> Vec<ManaPipPaymentOption> {
    use crate::mana::ManaSymbol;

    let mut options = Vec::new();
    let mut index = 0;
    let mut added_any_color_options = false;

    // Get the player's mana pool
    let pool = game.player(player).map(|p| &p.mana_pool);

    // For each alternative in the pip, check what can pay it
    for symbol in pip {
        match symbol {
            ManaSymbol::White => {
                if allow_any_color {
                    if !added_any_color_options && let Some(p) = pool {
                        add_any_color_pool_options(&mut options, &mut index, p);
                        added_any_color_options = true;
                    }
                } else if pool.map(|p| p.white > 0).unwrap_or(false) {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: "Use {W} from mana pool".to_string(),
                        action: ManaPipPaymentAction::UseFromPool(ManaSymbol::White),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Blue => {
                if allow_any_color {
                    if !added_any_color_options && let Some(p) = pool {
                        add_any_color_pool_options(&mut options, &mut index, p);
                        added_any_color_options = true;
                    }
                } else if pool.map(|p| p.blue > 0).unwrap_or(false) {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: "Use {U} from mana pool".to_string(),
                        action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Blue),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Black => {
                if allow_any_color {
                    if !added_any_color_options && let Some(p) = pool {
                        add_any_color_pool_options(&mut options, &mut index, p);
                        added_any_color_options = true;
                    }
                } else if pool.map(|p| p.black > 0).unwrap_or(false) {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: "Use {B} from mana pool".to_string(),
                        action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Black),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Red => {
                if allow_any_color {
                    if !added_any_color_options && let Some(p) = pool {
                        add_any_color_pool_options(&mut options, &mut index, p);
                        added_any_color_options = true;
                    }
                } else if pool.map(|p| p.red > 0).unwrap_or(false) {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: "Use {R} from mana pool".to_string(),
                        action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Red),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Green => {
                if allow_any_color {
                    if !added_any_color_options && let Some(p) = pool {
                        add_any_color_pool_options(&mut options, &mut index, p);
                        added_any_color_options = true;
                    }
                } else if pool.map(|p| p.green > 0).unwrap_or(false) {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: "Use {G} from mana pool".to_string(),
                        action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Green),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Colorless => {
                if pool.map(|p| p.colorless > 0).unwrap_or(false) {
                    options.push(ManaPipPaymentOption {
                        index,
                        description: "Use {C} from mana pool".to_string(),
                        action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Colorless),
                    });
                    index += 1;
                }
            }
            ManaSymbol::Generic(_) => {
                // Generic can be paid with any mana in the pool
                if let Some(p) = pool {
                    add_any_color_pool_options(&mut options, &mut index, p);
                }
            }
            ManaSymbol::Life(amount) => {
                // Can always pay life (if player has enough)
                let has_life = game
                    .player(player)
                    .map(|p| p.life > *amount as i32)
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
            // Check if this ability produces mana that can pay this pip
            if mana_ability_can_pay_pip(game, perm_id, ability_index, pip, allow_any_color) {
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

fn add_pip_alternative_payment_options(
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

fn convoke_can_pay_pip(colors: crate::color::ColorSet, pip: &[crate::mana::ManaSymbol]) -> bool {
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

fn improvise_can_pay_pip(pip: &[crate::mana::ManaSymbol]) -> bool {
    pip.iter()
        .any(|symbol| matches!(symbol, crate::mana::ManaSymbol::Generic(_)))
}

fn add_any_color_pool_options(
    options: &mut Vec<ManaPipPaymentOption>,
    index: &mut usize,
    pool: &crate::player::ManaPool,
) {
    use crate::mana::ManaSymbol;

    if pool.white > 0 {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {W} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::White),
        });
        *index += 1;
    }
    if pool.blue > 0 {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {U} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Blue),
        });
        *index += 1;
    }
    if pool.black > 0 {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {B} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Black),
        });
        *index += 1;
    }
    if pool.red > 0 {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {R} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Red),
        });
        *index += 1;
    }
    if pool.green > 0 {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {G} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Green),
        });
        *index += 1;
    }
    if pool.colorless > 0 {
        options.push(ManaPipPaymentOption {
            index: *index,
            description: "Use {C} from mana pool".to_string(),
            action: ManaPipPaymentAction::UseFromPool(ManaSymbol::Colorless),
        });
        *index += 1;
    }
}

/// Check if a mana ability can produce mana that pays the given pip.
fn mana_ability_can_pay_pip(
    game: &GameState,
    perm_id: ObjectId,
    ability_index: usize,
    pip: &[crate::mana::ManaSymbol],
    allow_any_color: bool,
) -> bool {
    use crate::ability::AbilityKind;
    use crate::mana::ManaSymbol;

    let Some(obj) = game.object(perm_id) else {
        return false;
    };

    let Some(ability) = obj.abilities.get(ability_index) else {
        return false;
    };

    let AbilityKind::Activated(mana_ability) = &ability.kind else {
        return false;
    };
    if !mana_ability.is_mana_ability() {
        return false;
    }

    // Check what mana this ability can produce.
    let produced_symbols = mana_ability.inferred_mana_symbols(game, perm_id, obj.controller);

    let pip_has_colored = pip.iter().any(|s| {
        matches!(
            s,
            ManaSymbol::White
                | ManaSymbol::Blue
                | ManaSymbol::Black
                | ManaSymbol::Red
                | ManaSymbol::Green
        )
    });

    for produced in &produced_symbols {
        for pip_symbol in pip {
            match (produced, pip_symbol) {
                // Any mana can pay generic
                (_, ManaSymbol::Generic(_)) => return true,
                // Exact color matches
                (ManaSymbol::White, ManaSymbol::White) => return true,
                (ManaSymbol::Blue, ManaSymbol::Blue) => return true,
                (ManaSymbol::Black, ManaSymbol::Black) => return true,
                (ManaSymbol::Red, ManaSymbol::Red) => return true,
                (ManaSymbol::Green, ManaSymbol::Green) => return true,
                (ManaSymbol::Colorless, ManaSymbol::Colorless) => return true,
                // Any-color spending: any produced mana can pay colored pips
                _ if allow_any_color && pip_has_colored => return true,
                _ => {}
            }
        }
    }

    false
}

#[cfg(feature = "net")]
fn record_pip_payment_action(trace: &mut Vec<CostStep>, action: &ManaPipPaymentAction) {
    match action {
        ManaPipPaymentAction::UseFromPool(symbol) => {
            trace.push(CostStep::Mana(ManaSymbolSpec::from(*symbol)));
        }
        ManaPipPaymentAction::PayLife(amount) => {
            let capped = (*amount).min(u8::MAX as u32) as u8;
            trace.push(CostStep::Mana(ManaSymbolSpec {
                symbol: ManaSymbolCode::Life,
                value: capped,
            }));
        }
        ManaPipPaymentAction::ActivateManaAbility {
            source_id,
            ability_index,
        } => {
            trace.push(CostStep::Payment(CostPayment::ActivateManaAbility {
                source: GameObjectId(source_id.0),
                ability_index: (*ability_index).min(u32::MAX as usize) as u32,
            }));
        }
        ManaPipPaymentAction::PayViaAlternative { permanent_id, .. } => {
            trace.push(CostStep::Payment(CostPayment::Tap {
                objects: vec![GameObjectId(permanent_id.0)],
            }));
        }
    }
}

#[cfg(not(feature = "net"))]
fn record_pip_payment_action(_trace: &mut Vec<CostStep>, _action: &ManaPipPaymentAction) {}

#[cfg(feature = "net")]
fn record_immediate_cost_payment(
    trace: &mut Vec<CostStep>,
    cost: &crate::costs::Cost,
    source: ObjectId,
) {
    let source_id = GameObjectId(source.0);

    if cost.requires_tap() {
        trace.push(CostStep::Payment(CostPayment::Tap {
            objects: vec![source_id],
        }));
        return;
    }

    if cost.requires_untap() {
        trace.push(CostStep::Payment(CostPayment::Untap {
            objects: vec![source_id],
        }));
        return;
    }

    if cost.is_life_cost() {
        if let Some(amount) = cost.life_amount() {
            trace.push(CostStep::Payment(CostPayment::Life { amount }));
            return;
        }
    }

    if cost.is_sacrifice_self() {
        trace.push(CostStep::Payment(CostPayment::Sacrifice {
            objects: vec![source_id],
        }));
        return;
    }

    // Fallback: preserve order with an opaque payment tag.
    trace.push(CostStep::Payment(CostPayment::Other {
        tag: 0,
        data: cost.display().into_bytes(),
    }));
}

#[cfg(not(feature = "net"))]
fn record_immediate_cost_payment(
    _trace: &mut Vec<CostStep>,
    _cost: &crate::costs::Cost,
    _source: ObjectId,
) {
}

#[cfg(feature = "net")]
fn record_cast_mana_ability_payment(
    pending: &mut PendingCast,
    source: ObjectId,
    ability_index: usize,
) {
    pending
        .payment_trace
        .push(CostStep::Payment(CostPayment::ActivateManaAbility {
            source: GameObjectId(source.0),
            ability_index: ability_index.min(u32::MAX as usize) as u32,
        }));
}

#[cfg(not(feature = "net"))]
fn record_cast_mana_ability_payment(
    _pending: &mut PendingCast,
    _source: ObjectId,
    _ability_index: usize,
) {
}

#[cfg(feature = "net")]
fn record_activation_mana_ability_payment(
    pending: &mut PendingActivation,
    source: ObjectId,
    ability_index: usize,
) {
    pending
        .payment_trace
        .push(CostStep::Payment(CostPayment::ActivateManaAbility {
            source: GameObjectId(source.0),
            ability_index: ability_index.min(u32::MAX as usize) as u32,
        }));
}

#[cfg(not(feature = "net"))]
fn record_activation_mana_ability_payment(
    _pending: &mut PendingActivation,
    _source: ObjectId,
    _ability_index: usize,
) {
}

/// Execute a pip payment action.
/// Execute a pip payment action.
/// Returns true if the pip was actually paid (mana consumed or life paid),
/// false if we only generated mana (need to continue processing this pip).
fn execute_pip_payment_action(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    player: PlayerId,
    source: Option<ObjectId>,
    action: &ManaPipPaymentAction,
    decision_maker: &mut impl DecisionMaker,
    payment_trace: &mut Vec<CostStep>,
    mut mana_spent_to_cast: Option<&mut ManaPool>,
) -> Result<bool, GameLoopError> {
    match action {
        ManaPipPaymentAction::UseFromPool(symbol) => {
            if let Some(player_obj) = game.player_mut(player) {
                let success = player_obj.mana_pool.remove(*symbol, 1);
                if !success {
                    return Err(GameLoopError::InvalidState(format!(
                        "Not enough {:?} mana in pool",
                        symbol
                    )));
                }
            }
            if let Some(spent) = mana_spent_to_cast.as_deref_mut() {
                track_spent_mana_symbol(spent, *symbol);
            }
            record_pip_payment_action(payment_trace, action);
            Ok(true) // Pip was paid
        }
        ManaPipPaymentAction::ActivateManaAbility {
            source_id,
            ability_index,
        } => {
            // Activate the mana ability - this just generates mana, doesn't pay the pip
            crate::special_actions::perform_activate_mana_ability(
                game,
                player,
                *source_id,
                *ability_index,
                decision_maker,
            )?;
            record_pip_payment_action(payment_trace, action);
            Ok(false) // Pip not yet paid, just generated mana
        }
        ManaPipPaymentAction::PayLife(amount) => {
            if let Some(player_obj) = game.player_mut(player) {
                player_obj.life -= *amount as i32;
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
                let event = TriggerEvent::new(KeywordActionEvent::new(
                    keyword_action_from_alternative_effect(*effect),
                    player,
                    source_id,
                    1,
                ));
                queue_triggers_from_event(game, trigger_queue, event, true);
            }
            record_pip_payment_action(payment_trace, action);
            Ok(true) // Pip was paid
        }
    }
}

fn track_spent_mana_symbol(pool: &mut ManaPool, symbol: crate::mana::ManaSymbol) {
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
fn format_pip(pip: &[crate::mana::ManaSymbol]) -> String {
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

/// Apply a modes response to the pending cast.
///
/// This handles the player's mode selection for modal spells per MTG rule 601.2b.
fn apply_modes_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    modes: &[usize],
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for modes response".to_string())
    })?;

    let effects = game
        .object(pending.spell_id)
        .and_then(|obj| obj.spell_effect.as_deref())
        .unwrap_or(&[]);

    if !spell_has_legal_targets_with_modes(
        game,
        effects,
        pending.caster,
        Some(pending.spell_id),
        Some(modes),
    ) {
        return Err(GameLoopError::InvalidState(
            "Selected mode combination has no legal targets".to_string(),
        ));
    }

    // Store the chosen modes
    pending.chosen_modes = Some(modes.to_vec());
    pending.remaining_requirements = extract_target_requirements_with_modes(
        game,
        effects,
        pending.caster,
        Some(pending.spell_id),
        Some(modes),
    );

    // Continue to optional costs
    check_optional_costs_or_continue(game, trigger_queue, state, pending, decision_maker)
}

/// Apply an optional costs response to the pending cast.
fn apply_optional_costs_response(
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

    // Continue to targeting or finalization
    continue_to_targeting_or_finalize(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a hybrid/Phyrexian mana choice response to a pending cast or activation.
///
/// Per MTG rule 601.2b (and 602.2b for abilities), players announce how they'll pay
/// hybrid/Phyrexian costs before choosing targets. This handler stores the choice
/// and either prompts for the next pip or continues to target selection.
fn apply_hybrid_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check if this is for a pending cast (spell) or pending activation (ability)
    if let Some(mut pending) = state.pending_cast.take() {
        // Handle spell casting hybrid choice
        if pending.pending_hybrid_pips.is_empty() {
            state.pending_cast = Some(pending);
            return Err(GameLoopError::InvalidState(
                "No pending hybrid pips for hybrid choice response".to_string(),
            ));
        }

        let (pip_idx, alternatives) = pending.pending_hybrid_pips.remove(0);

        if choice >= alternatives.len() {
            state.pending_cast = Some(pending);
            return Err(GameLoopError::InvalidState(format!(
                "Invalid hybrid choice {} for pip with {} alternatives",
                choice,
                alternatives.len()
            )));
        }

        let chosen_symbol = alternatives[choice];
        pending.hybrid_choices.push((pip_idx, chosen_symbol));

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
        // Handle ability activation hybrid choice (per MTG rule 602.2b)
        if pending.pending_hybrid_pips.is_empty() {
            state.pending_activation = Some(pending);
            return Err(GameLoopError::InvalidState(
                "No pending hybrid pips for hybrid choice response (activation)".to_string(),
            ));
        }

        let (pip_idx, alternatives) = pending.pending_hybrid_pips.remove(0);

        if choice >= alternatives.len() {
            state.pending_activation = Some(pending);
            return Err(GameLoopError::InvalidState(format!(
                "Invalid hybrid choice {} for pip with {} alternatives (activation)",
                choice,
                alternatives.len()
            )));
        }

        let chosen_symbol = alternatives[choice];
        pending.hybrid_choices.push((pip_idx, chosen_symbol));

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
fn apply_mana_payment_response(
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
                "Failed to activate mana ability: {:?}",
                e
            )));
        }
        drain_pending_trigger_events(game, trigger_queue);

        queue_ability_activated_event(game, trigger_queue, perm_id, pending.caster, true, None);

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
fn apply_mana_payment_response_mana_ability(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::ability::AbilityKind;
    use crate::special_actions::{SpecialAction, perform};

    let pending = state.pending_mana_ability.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending mana ability for payment response".to_string())
    })?;

    // Get available mana abilities, excluding the one we're paying for
    // and filtered to only those that can help pay the cost
    let allow_any_color = game.can_spend_mana_as_any_color(pending.activator, Some(pending.source));
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
                if let Some(perm) = game.object(*perm_id)
                    && let Some(ability) = perm.abilities.get(*ability_index)
                    && let AbilityKind::Activated(mana_ability) = &ability.kind
                    && mana_ability.is_mana_ability()
                {
                    let produced =
                        mana_ability.inferred_mana_symbols(game, *perm_id, pending.activator);
                    mana_can_help_pay_cost(
                        &produced,
                        &pending.mana_cost,
                        game,
                        pending.activator,
                        allow_any_color,
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
                "Failed to activate mana ability: {:?}",
                e
            )));
        }
        drain_pending_trigger_events(game, trigger_queue);

        queue_ability_activated_event(game, trigger_queue, perm_id, pending.activator, true, None);

        // Check if player can now pay
        let can_pay_now = game.can_pay_mana_cost(
            pending.activator,
            Some(pending.source),
            &pending.mana_cost,
            0,
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
        if !game.can_pay_mana_cost(
            pending.activator,
            Some(pending.source),
            &pending.mana_cost,
            0,
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
fn execute_pending_mana_ability(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    pending: &PendingManaAbility,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    use crate::costs::CostContext;
    use crate::executor::ExecutionContext;

    // Pay the mana cost
    if !game.try_pay_mana_cost(
        pending.activator,
        Some(pending.source),
        &pending.mana_cost,
        0,
    ) {
        return Err(GameLoopError::InvalidState(
            "Failed to pay mana cost".to_string(),
        ));
    }

    // Pay other costs from TotalCost (not cost_effects)
    let mut cost_ctx = CostContext::new(pending.source, pending.activator, decision_maker);
    for c in &pending.other_costs {
        crate::special_actions::pay_cost_component_with_choice(game, c, &mut cost_ctx)
            .map_err(|e| GameLoopError::InvalidState(format!("Failed to pay cost: {:?}", e)))?;
    }
    drain_pending_trigger_events(game, trigger_queue);

    // Add fixed mana to player's pool
    if !pending.mana_to_add.is_empty() {
        if let Some(player_obj) = game.player_mut(pending.activator) {
            for symbol in &pending.mana_to_add {
                player_obj.mana_pool.add(*symbol, 1);
            }
        }
    }

    // Execute additional effects (for complex mana abilities)
    if !pending.effects.is_empty() {
        let mut ctx = ExecutionContext::new(pending.source, pending.activator, decision_maker);
        let mut emitted_events = Vec::new();

        for effect in &pending.effects {
            if let Ok(outcome) = execute_effect(game, effect, &mut ctx) {
                emitted_events.extend(outcome.events);
            }
        }
        queue_triggers_for_events(game, trigger_queue, emitted_events);
        drain_pending_trigger_events(game, trigger_queue);
    }

    game.record_ability_activation(pending.source, pending.ability_index);

    queue_ability_activated_event(
        game,
        trigger_queue,
        pending.source,
        pending.activator,
        true,
        None,
    );

    Ok(())
}

/// Apply a mana payment response for a pending activation.
fn apply_mana_payment_response_activation(
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
                "Failed to activate mana ability: {:?}",
                e
            )));
        }
        drain_pending_trigger_events(game, trigger_queue);

        queue_ability_activated_event(game, trigger_queue, perm_id, pending.activator, true, None);

        // Record the mana ability activation in the payment trace.
        record_activation_mana_ability_payment(&mut pending, perm_id, ability_index);

        // Stay in PayingMana stage, continue activation
        continue_activation(game, trigger_queue, state, pending, decision_maker)
    } else {
        // Player chose to pay mana cost
        // Verify they can actually pay
        let x_value = pending.x_value.unwrap_or(0) as u32;
        if let Some(ref cost) = pending.mana_cost_to_pay
            && !game.can_pay_mana_cost(pending.activator, Some(pending.source), cost, x_value)
        {
            return Err(GameLoopError::InvalidState(
                "Cannot pay mana cost - insufficient mana".to_string(),
            ));
        }

        // Pay the mana and finalize
        let mut pending = pending;
        if let Some(ref cost) = pending.mana_cost_to_pay {
            let allow_any_color =
                game.can_spend_mana_as_any_color(pending.activator, Some(pending.source));
            let life_to_pay_preview = {
                let Some(player) = game.player(pending.activator) else {
                    return Err(GameLoopError::InvalidState(
                        "Cannot pay mana cost - payer not found".to_string(),
                    ));
                };
                let mut preview_pool = player.mana_pool.clone();
                let (_, life_to_pay) =
                    preview_pool.try_pay_tracking_life_with_any_color(cost, x_value, allow_any_color);
                life_to_pay
            };
            if life_to_pay_preview > 0 && !game.can_pay_life(pending.activator, life_to_pay_preview)
            {
                return Err(GameLoopError::InvalidState(
                    "Cannot pay mana cost - insufficient life for Phyrexian payment".to_string(),
                ));
            }
            let mut life_to_pay = 0u32;
            if let Some(player) = game.player_mut(pending.activator) {
                // Pay mana and track life for Phyrexian costs
                let (_, paid_life) = player.mana_pool.try_pay_tracking_life_with_any_color(
                    cost,
                    x_value,
                    allow_any_color,
                );
                life_to_pay = paid_life;
            }
            // Deduct life for Phyrexian mana that couldn't be paid with mana.
            if life_to_pay > 0 && !game.pay_life(pending.activator, life_to_pay) {
                return Err(GameLoopError::InvalidState(
                    "Cannot pay mana cost - insufficient life for Phyrexian payment".to_string(),
                ));
            }
        }
        pending.stage = ActivationStage::ReadyToFinalize;
        continue_activation(game, trigger_queue, state, pending, decision_maker)
    }
}

/// Apply a pip payment response for a pending activation.
fn apply_pip_payment_response_activation(
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

    // Rebuild the options to get the action for this choice
    let allow_any_color = game.can_spend_mana_as_any_color(pending.activator, Some(pending.source));
    let options = build_pip_payment_options(
        game,
        pending.activator,
        &pip,
        allow_any_color,
        None,
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
        action,
        &mut *decision_maker,
        &mut pending.payment_trace,
        None,
    )?;
    queue_mana_ability_event_for_action(game, trigger_queue, action, pending.activator);
    drain_pending_trigger_events(game, trigger_queue);

    // Only remove the pip if it was actually paid (not just mana generated)
    if pip_paid {
        pending.remaining_mana_pips.remove(0);
    }

    // Continue activation (will process next pip or finalize)
    continue_activation(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a pip payment response for a pending spell cast.
fn apply_pip_payment_response_cast(
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

    let pip = pending.remaining_mana_pips[0].clone();

    // Rebuild the options to get the action for this choice
    let allow_any_color = game.can_spend_mana_as_any_color(pending.caster, Some(pending.spell_id));
    let options = build_pip_payment_options(
        game,
        pending.caster,
        &pip,
        allow_any_color,
        Some(pending.spell_id),
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
        pending.caster,
        Some(pending.spell_id),
        action,
        &mut *decision_maker,
        &mut pending.payment_trace,
        Some(&mut pending.mana_spent_to_cast),
    )?;
    queue_mana_ability_event_for_action(game, trigger_queue, action, pending.caster);
    drain_pending_trigger_events(game, trigger_queue);

    // Only remove the pip if it was actually paid (not just mana generated)
    if pip_paid {
        record_keyword_payment_contribution(&mut pending.keyword_payment_contributions, action);
        pending.remaining_mana_pips.remove(0);
    }

    // Continue spell cast mana payment (will process next pip or finalize)
    continue_spell_cast_mana_payment(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a sacrifice target response for a pending activation.
fn apply_sacrifice_target_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    target_id: ObjectId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_activation.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending activation for sacrifice response".to_string())
    })?;

    // Sacrifice the chosen permanent
    if game.object(target_id).is_some() {
        let snapshot = game
            .object(target_id)
            .map(|obj| ObjectSnapshot::from_object(obj, game));
        let sacrificing_player = snapshot
            .as_ref()
            .map(|snap| snap.controller)
            .or(Some(pending.activator));
        game.move_object(target_id, Zone::Graveyard);
        game.queue_trigger_event(TriggerEvent::new(
            SacrificeEvent::new(target_id, Some(pending.source))
                .with_snapshot(snapshot, sacrificing_player),
        ));

        #[cfg(feature = "net")]
        {
            // Record sacrifice payment for deterministic trace
            pending
                .payment_trace
                .push(CostStep::Payment(CostPayment::Sacrifice {
                    objects: vec![GameObjectId(target_id.0)],
                }));
        }
        drain_pending_trigger_events(game, trigger_queue);
    }

    // Remove the satisfied sacrifice cost
    if !pending.remaining_sacrifice_costs.is_empty() {
        pending.remaining_sacrifice_costs.remove(0);
    }

    // Continue activation process
    continue_activation(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a card to exile response for a pending spell cast with alternative cost.
fn apply_card_to_exile_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    card_id: ObjectId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for card to exile response".to_string())
    })?;

    // Add the chosen card to the exile list
    pending.cards_to_exile.push(card_id);

    // Continue with mana payment (or finalize if no mana needed)
    // We need to re-run the logic from continue_to_mana_payment
    use crate::decision::calculate_effective_mana_cost_for_payment_with_chosen_targets;

    // Compute the effective mana cost for this spell
    let effective_cost = if let Some(obj) = game.object(pending.spell_id) {
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
            } => {
                crate::decision::resolve_play_from_alternative_method(
                    game,
                    pending.caster,
                    obj,
                    *zone,
                    *idx,
                )
                .map(|method| {
                    if !method.cost_effects().is_empty() {
                        method.mana_cost().cloned()
                    } else {
                        method.mana_cost().cloned().or_else(|| obj.mana_cost.clone())
                    }
                })
                .unwrap_or_else(|| obj.mana_cost.clone())
            }
        };
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
    pending.stage = CastStage::PayingMana;

    continue_spell_cast_mana_payment(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a casting method choice response for a pending spell with multiple methods.
fn apply_casting_method_choice_response(
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

    // Get the spell's mana cost and effects, considering casting method
    // Note: We use stack_id now since the spell has been moved to stack
    let (mana_cost, effects) = if let Some(obj) = game.object(stack_id) {
        let cost = match &casting_method {
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
            } => {
                crate::decision::resolve_play_from_alternative_method(
                    game, player, obj, *zone, *idx,
                )
                .map(|method| {
                    if !method.cost_effects().is_empty() {
                        method.mana_cost().cloned()
                    } else {
                        method.mana_cost().cloned().or_else(|| obj.mana_cost.clone())
                    }
                })
                .unwrap_or_else(|| obj.mana_cost.clone())
            }
        };
        (cost, obj.spell_effect.clone().unwrap_or_default())
    } else {
        (None, Vec::new())
    };

    let (needs_x, max_x) =
        compute_spell_cast_x_bounds(game, player, stack_id, &casting_method, mana_cost.as_ref());

    if needs_x {
        // Extract target requirements for later (use stack_id since spell is on stack)
        let requirements = extract_target_requirements(game, &effects, player, Some(stack_id));

        // Initialize optional costs tracker from the spell's optional costs
        let optional_costs_paid = game
            .object(stack_id)
            .map(|obj| OptionalCostsPaid::from_costs(&obj.optional_costs))
            .unwrap_or_default();

        state.pending_cast = Some(PendingCast {
            spell_id: stack_id, // Use stack_id since spell is now on stack
            from_zone,
            caster: player,
            stage: CastStage::ChoosingX,
            x_value: None,
            chosen_targets: Vec::new(),
            remaining_requirements: requirements,
            casting_method,
            optional_costs_paid,
            payment_trace: Vec::new(),
            mana_spent_to_cast: ManaPool::default(),
            mana_cost_to_pay: None,
            remaining_mana_pips: Vec::new(),
            cards_to_exile: Vec::new(),
            chosen_modes: None,
            hybrid_choices: Vec::new(),
            pending_hybrid_pips: Vec::new(),
            stack_id,
            keyword_payment_contributions: Vec::new(),
        });

        let ctx = crate::decisions::context::NumberContext::x_value(
            player, stack_id, // Use stack_id
            max_x,
        );
        Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Number(ctx),
        ))
    } else {
        // No X cost, check for optional costs then targets
        let requirements = extract_target_requirements(game, &effects, player, Some(stack_id));

        // Initialize optional costs tracker from the spell's optional costs
        let optional_costs_paid = game
            .object(stack_id)
            .map(|obj| OptionalCostsPaid::from_costs(&obj.optional_costs))
            .unwrap_or_default();

        let new_pending = PendingCast {
            spell_id: stack_id, // Use stack_id since spell is now on stack
            from_zone,
            caster: player,
            stage: CastStage::ChoosingModes, // Will be updated by helper
            x_value: None,
            chosen_targets: Vec::new(),
            remaining_requirements: requirements,
            casting_method,
            optional_costs_paid,
            payment_trace: Vec::new(),
            mana_spent_to_cast: ManaPool::default(),
            mana_cost_to_pay: None,
            remaining_mana_pips: Vec::new(),
            cards_to_exile: Vec::new(),
            chosen_modes: None,
            hybrid_choices: Vec::new(),
            pending_hybrid_pips: Vec::new(),
            stack_id,
            keyword_payment_contributions: Vec::new(),
        };

        check_modes_or_continue(game, trigger_queue, state, new_pending, decision_maker)
    }
}

/// Move a spell to the stack at the start of casting (per MTG rule 601.2a).
///
/// This is called during the proposal phase, before any choices are made.
/// If casting fails later (e.g., can't pay costs), the spell should be reverted.
///
/// Returns the new ObjectId on the stack.
fn propose_spell_cast(
    game: &mut GameState,
    spell_id: ObjectId,
    _from_zone: Zone,
    caster: PlayerId,
    casting_method: &CastingMethod,
) -> Result<ObjectId, GameLoopError> {
    let new_id = game.move_object(spell_id, Zone::Stack).ok_or_else(|| {
        GameLoopError::InvalidState("Failed to move spell to stack during proposal".to_string())
    })?;

    let cast_as_bestow = game.object(new_id).is_some_and(|obj| match casting_method {
        CastingMethod::Alternative(idx) => obj
            .alternative_casts
            .get(*idx)
            .is_some_and(crate::alternative_cast::AlternativeCastingMethod::is_bestow),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        } => crate::decision::resolve_play_from_alternative_method(game, caster, obj, *zone, *idx)
            .is_some_and(|method| method.is_bestow()),
        _ => false,
    });
    if cast_as_bestow && let Some(obj) = game.object_mut(new_id) {
        obj.apply_bestow_cast_overlay();
    }

    Ok(new_id)
}

/// Revert a spell cast that failed during the casting process.
///
/// Per MTG rules, if casting fails at any point before completion,
/// the game state returns to before the cast was proposed.
#[allow(dead_code)]
fn revert_spell_cast(game: &mut GameState, stack_id: ObjectId, original_zone: Zone) {
    // Move spell back to original zone
    game.move_object(stack_id, original_zone);
    // Note: Mana abilities activated during casting are NOT reverted per rules
    // (they happen in a special window and their effects stay)
}

/// Result of finalizing a spell cast, containing info needed for triggers.
struct SpellCastResult {
    /// The new object ID of the spell on the stack
    new_id: ObjectId,
    /// Who cast the spell
    caster: PlayerId,
    /// Which zone the spell was cast from.
    from_zone: Zone,
}

/// Finalize a spell cast by paying remaining costs and creating the stack entry.
/// Returns the spell cast info for trigger checking.
///
/// `stack_id` is the spell already moved to stack during proposal (per 601.2a).
fn finalize_spell_cast(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    _state: &mut PriorityLoopState,
    spell_id: ObjectId,
    from_zone: Zone,
    caster: PlayerId,
    targets: Vec<Target>,
    x_value: Option<u32>,
    casting_method: CastingMethod,
    optional_costs_paid: OptionalCostsPaid,
    chosen_modes: Option<Vec<usize>>,
    pre_chosen_exile_cards: Vec<ObjectId>,
    mut mana_spent_to_cast: ManaPool,
    keyword_payment_contributions: Vec<KeywordPaymentContribution>,
    payment_trace: &mut Vec<CostStep>,
    mana_already_paid: bool,
    stack_id: ObjectId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<SpellCastResult, GameLoopError> {
    use crate::decision::calculate_effective_mana_cost_with_chosen_targets;
    #[cfg(not(feature = "net"))]
    let _ = payment_trace;

    // Get the mana cost, alternative cost effects, and exile count based on casting method
    let (base_mana_cost, alternative_cost_effects, granted_escape_exile_count) =
        if let Some(obj) = game.object(spell_id) {
            match &casting_method {
                CastingMethod::Normal => (obj.mana_cost.clone(), Vec::new(), None),
                CastingMethod::Alternative(idx) => {
                    if let Some(method) = obj.alternative_casts.get(*idx) {
                        let cost_effects = method.cost_effects().to_vec();
                        if !cost_effects.is_empty() {
                            // Composed alternative method (Force of Will style) - uses cost_effects.
                            let mana = method.mana_cost().cloned();
                            (mana, cost_effects, None)
                        } else {
                            // Other alternative methods (flashback, escape, etc.) - uses mana cost only
                            let mana = method
                                .mana_cost()
                                .cloned()
                                .or_else(|| obj.mana_cost.clone());
                            (mana, Vec::new(), None)
                        }
                    } else {
                        (obj.mana_cost.clone(), Vec::new(), None)
                    }
                }
                CastingMethod::GrantedEscape { exile_count, .. } => {
                    (obj.mana_cost.clone(), Vec::new(), Some(*exile_count))
                }
                CastingMethod::GrantedFlashback => (obj.mana_cost.clone(), Vec::new(), None),
                CastingMethod::PlayFrom {
                    use_alternative: None,
                    ..
                } => {
                    // Yawgmoth's Will normal cost
                    (obj.mana_cost.clone(), Vec::new(), None)
                }
                CastingMethod::PlayFrom {
                    use_alternative: Some(idx),
                    zone,
                    ..
                } => {
                    crate::decision::resolve_play_from_alternative_method(
                        game, caster, obj, *zone, *idx,
                    )
                    .map(|method| {
                        let cost_effects = method.cost_effects().to_vec();
                        if !cost_effects.is_empty() {
                            (method.mana_cost().cloned(), cost_effects, None)
                        } else {
                            (
                                method.mana_cost().cloned().or_else(|| obj.mana_cost.clone()),
                                Vec::new(),
                                None,
                            )
                        }
                    })
                    .unwrap_or_else(|| (obj.mana_cost.clone(), Vec::new(), None))
                }
            }
        } else {
            (None, Vec::new(), None)
        };

    // Execute alternative cost effects (Force of Will style) if present
    if !alternative_cost_effects.is_empty() {
        // Build pre-chosen cards for ExileFromHand cost effects if not already provided
        let cards_for_exile = if !pre_chosen_exile_cards.is_empty() {
            pre_chosen_exile_cards.clone()
        } else {
            // Auto-select: find matching cards for any exile from hand cost effects
            let mut auto_selected = Vec::new();
            for effect in &alternative_cost_effects {
                if let Some((count, color_filter)) = effect.0.exile_from_hand_cost_info()
                    && let Some(player) = game.player(caster)
                {
                    let matching_cards: Vec<ObjectId> = player
                        .hand
                        .iter()
                        .filter(|&&card_id| {
                            if card_id == spell_id {
                                return false;
                            }
                            // Don't include cards already selected
                            if auto_selected.contains(&card_id) {
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
                        .take(count as usize)
                        .copied()
                        .collect();
                    auto_selected.extend(matching_cards);
                }
            }
            auto_selected
        };

        #[cfg(feature = "net")]
        {
            // Record deterministic cost payments for alternative cost effects
            let mut exile_cursor = 0usize;
            for effect in &alternative_cost_effects {
                let mut recorded = false;

                if let Some(amount) = effect.0.pay_life_amount() {
                    payment_trace.push(CostStep::Payment(CostPayment::Life { amount }));
                    recorded = true;
                }

                if effect.0.is_tap_source_cost() {
                    payment_trace.push(CostStep::Payment(CostPayment::Tap {
                        objects: vec![GameObjectId(spell_id.0)],
                    }));
                    recorded = true;
                }

                if effect.0.is_sacrifice_source_cost() {
                    payment_trace.push(CostStep::Payment(CostPayment::Sacrifice {
                        objects: vec![GameObjectId(spell_id.0)],
                    }));
                    recorded = true;
                }

                if let Some((count, _)) = effect.0.exile_from_hand_cost_info() {
                    let end = (exile_cursor + count as usize).min(cards_for_exile.len());
                    let slice = &cards_for_exile[exile_cursor..end];
                    if !slice.is_empty() {
                        payment_trace.push(CostStep::Payment(CostPayment::Exile {
                            objects: slice.iter().map(|id| GameObjectId(id.0)).collect(),
                            from_zone: ZoneCode::Hand,
                        }));
                        recorded = true;
                    }
                    exile_cursor = end;
                }

                if !recorded {
                    if let Some(desc) = effect.0.cost_description() {
                        payment_trace.push(CostStep::Payment(CostPayment::Other {
                            tag: 1,
                            data: desc.into_bytes(),
                        }));
                    }
                }
            }
        }

        // Execute all cost effects with EventCause::from_cost for proper trigger handling
        let mut ctx = ExecutionContext::new(spell_id, caster, decision_maker)
            .with_cause(EventCause::from_cost(spell_id, caster));
        if let Some(x) = x_value {
            ctx = ctx.with_x(x);
        }
        // Note: cards_for_exile contains pre-selected cards for exile from hand costs.
        // The effect will auto-select the same cards when executed.
        let _ = cards_for_exile; // Silence unused variable warning

        let mut emitted_events = Vec::new();
        for effect in &alternative_cost_effects {
            let outcome = execute_effect(game, effect, &mut ctx).map_err(|err| {
                GameLoopError::InvalidState(format!(
                    "Failed to execute alternative cost effect: {err:?}"
                ))
            })?;
            if outcome.result.is_failure() {
                return Err(GameLoopError::InvalidState(format!(
                    "Alternative cost effect failed: {:?}",
                    outcome.result
                )));
            }
            emitted_events.extend(outcome.events);
        }
        queue_triggers_for_events(game, trigger_queue, emitted_events);
        drain_pending_trigger_events(game, trigger_queue);
    }

    // Calculate effective cost and Delve exile count
    let (effective_cost, delve_exile_count) = if let Some(ref base_cost) = base_mana_cost {
        if let Some(obj) = game.object(spell_id) {
            let eff_cost = calculate_effective_mana_cost_with_chosen_targets(
                game,
                caster,
                obj,
                base_cost,
                &targets,
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

        #[cfg(feature = "net")]
        {
            if !cards_to_exile.is_empty() {
                payment_trace.push(CostStep::Payment(CostPayment::Exile {
                    objects: cards_to_exile.iter().map(|id| GameObjectId(id.0)).collect(),
                    from_zone: ZoneCode::Graveyard,
                }));
            }
        }

        // Move to exile (move_object handles removal from old zone)
        for card_id in cards_to_exile {
            game.move_object(card_id, Zone::Exile);
        }
    }

    // Pay the mana cost (using effective cost with reductions applied)
    // Skip if mana was already paid via pip-by-pip payment
    if !mana_already_paid && let Some(cost) = effective_cost {
        let x = x_value.unwrap_or(0);
        let before_pool = game.player(caster).map(|player| player.mana_pool.clone());
        if !game.try_pay_mana_cost(caster, Some(spell_id), &cost, x) {
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

        #[cfg(feature = "net")]
        {
            if !cards_to_exile.is_empty() {
                payment_trace.push(CostStep::Payment(CostPayment::Exile {
                    objects: cards_to_exile.iter().map(|id| GameObjectId(id.0)).collect(),
                    from_zone: ZoneCode::Graveyard,
                }));
            }
        }

        // Move to exile (move_object handles removal from old zone)
        for card_id in cards_to_exile {
            game.move_object(card_id, Zone::Exile);
        }
    }

    // Execute cost effects (new unified cost model)
    // Cost effects use EventCause::from_cost to enable triggers on cost-related events
    let cost_effects = game
        .object(spell_id)
        .map(|o| o.cost_effects.clone())
        .unwrap_or_default();
    if !cost_effects.is_empty() {
        #[cfg(feature = "net")]
        {
            for effect in &cost_effects {
                let mut recorded = false;

                if let Some(amount) = effect.0.pay_life_amount() {
                    payment_trace.push(CostStep::Payment(CostPayment::Life { amount }));
                    recorded = true;
                }

                if effect.0.is_tap_source_cost() {
                    payment_trace.push(CostStep::Payment(CostPayment::Tap {
                        objects: vec![GameObjectId(spell_id.0)],
                    }));
                    recorded = true;
                }

                if effect.0.is_sacrifice_source_cost() {
                    payment_trace.push(CostStep::Payment(CostPayment::Sacrifice {
                        objects: vec![GameObjectId(spell_id.0)],
                    }));
                    recorded = true;
                }

                if !recorded {
                    if let Some(desc) = effect.0.cost_description() {
                        payment_trace.push(CostStep::Payment(CostPayment::Other {
                            tag: 2,
                            data: desc.into_bytes(),
                        }));
                    }
                }
            }
        }

        let mut ctx = ExecutionContext::new(spell_id, caster, decision_maker)
            .with_cause(EventCause::from_cost(spell_id, caster));
        if let Some(x) = x_value {
            ctx = ctx.with_x(x);
        }
        let mut emitted_events = Vec::new();
        for effect in &cost_effects {
            let outcome = execute_effect(game, effect, &mut ctx).map_err(|err| {
                GameLoopError::InvalidState(format!("Failed to execute cost effect: {err:?}"))
            })?;
            if outcome.result.is_failure() {
                return Err(GameLoopError::InvalidState(format!(
                    "Cost effect failed: {:?}",
                    outcome.result
                )));
            }
            emitted_events.extend(outcome.events);
        }
        queue_triggers_for_events(game, trigger_queue, emitted_events);
        drain_pending_trigger_events(game, trigger_queue);
    }

    // Spell was already moved to stack during proposal (601.2a compliant).
    let mana_spent_total = mana_spent_to_cast.total();
    let new_id = stack_id;
    if let Some(spell_obj) = game.object_mut(new_id) {
        spell_obj.mana_spent_to_cast = mana_spent_to_cast;
        spell_obj.x_value = x_value;
    }

    // Create stack entry with targets, X value, casting method, optional costs, and chosen modes
    let mut entry = StackEntry::new(new_id, caster)
        .with_targets(targets.clone())
        .with_casting_method(casting_method)
        .with_optional_costs_paid(optional_costs_paid)
        .with_chosen_modes(chosen_modes)
        .with_keyword_payment_contributions(keyword_payment_contributions);
    if let Some(x) = x_value {
        entry = entry.with_x(x);
    }
    game.push_to_stack(entry);
    queue_becomes_targeted_events(game, trigger_queue, &targets, new_id, caster, false);

    // Track that a spell was cast this turn (per-caster)
    *game.spells_cast_this_turn.entry(caster).or_insert(0) += 1;
    game.spells_cast_this_turn_total = game.spells_cast_this_turn_total.saturating_add(1);
    game.spell_cast_order_this_turn
        .insert(new_id, game.spells_cast_this_turn_total);
    if let Some(obj) = game.object(new_id) {
        game.spells_cast_this_turn_snapshots
            .push(ObjectSnapshot::from_object(obj, game));
    }

    // Expend: "You expend N as you spend your Nth total mana to cast spells during a turn."
    let prev_mana_spent = game
        .mana_spent_to_cast_spells_this_turn
        .get(&caster)
        .copied()
        .unwrap_or(0);
    if mana_spent_total > 0 {
        let new_mana_spent_total = prev_mana_spent.saturating_add(mana_spent_total);
        game.mana_spent_to_cast_spells_this_turn
            .insert(caster, new_mana_spent_total);

        for threshold in (prev_mana_spent.saturating_add(1))..=new_mana_spent_total {
            queue_triggers_from_event(
                game,
                trigger_queue,
                TriggerEvent::new(KeywordActionEvent::new(
                    KeywordActionKind::Expend,
                    caster,
                    new_id,
                    threshold,
                )),
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
fn apply_decision_context_with_dm<D: DecisionMaker>(
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

            if state.pending_activation.is_some() {
                apply_sacrifice_target_response(game, trigger_queue, state, chosen, decision_maker)
            } else if state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingExileFromHand))
            {
                apply_card_to_exile_response(game, trigger_queue, state, chosen, decision_maker)
            } else {
                Err(GameLoopError::InvalidState(
                    "Unsupported SelectObjects decision in priority loop".to_string(),
                ))
            }
        }
        DecisionContext::SelectOptions(options_ctx) => {
            let result = decision_maker.decide_options(game, options_ctx);

            if game.pending_replacement_choice.is_some() {
                let choice = result.first().copied().unwrap_or(0);
                return apply_replacement_choice_response(
                    game,
                    trigger_queue,
                    choice,
                    decision_maker,
                );
            }
            if state.pending_method_selection.is_some() {
                let choice = result.first().copied().unwrap_or(0);
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
                let choice = result.first().copied().unwrap_or(0);
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
                .is_some_and(|pending| matches!(pending.stage, ActivationStage::PayingMana))
            {
                let choice = result.first().copied().unwrap_or(0);
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
                let choice = result.first().copied().unwrap_or(0);
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
        DecisionContext::Boolean(_)
        | DecisionContext::Order(_)
        | DecisionContext::Attackers(_)
        | DecisionContext::Blockers(_)
        | DecisionContext::Distribute(_)
        | DecisionContext::Colors(_)
        | DecisionContext::Counters(_)
        | DecisionContext::Partition(_)
        | DecisionContext::Proliferate(_) => Err(GameLoopError::InvalidState(format!(
            "Unsupported decision context in priority loop: {:?}",
            std::mem::discriminant(ctx)
        ))),
    }
}

fn apply_priority_action_with_dm(
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
fn should_auto_pass_ctx(ctx: &crate::decisions::context::DecisionContext) -> bool {
    if let crate::decisions::context::DecisionContext::Priority(pctx) = ctx {
        pctx.actions.len() == 1 && matches!(pctx.actions[0], LegalAction::PassPriority)
    } else {
        false
    }
}

/// Get the player from a context-based decision, if it's a Priority decision.
fn get_priority_player_from_ctx(
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
    use crate::cards::tokens::treasure_token_definition;
    use crate::mana::ManaSymbol;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_variable_mana_ability_can_pay_colored_pip() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let treasure = treasure_token_definition();
        let treasure_id = game.create_object_from_definition(&treasure, alice, Zone::Battlefield);

        assert!(
            mana_ability_can_pay_pip(&game, treasure_id, 0, &[ManaSymbol::Black], false),
            "Treasure should be considered able to pay a colored pip"
        );
    }
}

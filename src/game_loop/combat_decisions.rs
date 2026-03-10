// ============================================================================
// Combat Decision Handling
// ============================================================================

/// Get a decision context for declaring attackers.
pub fn get_declare_attackers_decision(
    game: &GameState,
    combat: &CombatState,
) -> crate::decisions::context::DecisionContext {
    let player = game.turn.active_player;
    let legal_attackers = compute_legal_attackers(game, combat);

    // Convert to AttackersContext
    let attacker_options: Vec<crate::decisions::context::AttackerOptionContext> = legal_attackers
        .into_iter()
        .map(|opt| {
            let creature_name = game
                .object(opt.creature)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| format!("Creature #{}", opt.creature.0));
            crate::decisions::context::AttackerOptionContext {
                creature: opt.creature,
                creature_name,
                valid_targets: opt.valid_targets,
                must_attack: opt.must_attack,
            }
        })
        .collect();

    crate::decisions::context::DecisionContext::Attackers(
        crate::decisions::context::AttackersContext::new(player, attacker_options),
    )
}

fn generic_mana_cost(amount: u32) -> crate::mana::ManaCost {
    use crate::mana::ManaSymbol;

    if amount == 0 {
        return crate::mana::ManaCost::new();
    }

    let mut pips = Vec::new();
    let mut remaining = amount;
    while remaining > 0 {
        let chunk = remaining.min(u8::MAX as u32) as u8;
        pips.push(vec![ManaSymbol::Generic(chunk)]);
        remaining -= chunk as u32;
    }
    crate::mana::ManaCost::from_pips(pips)
}

fn object_label(game: &GameState, id: ObjectId, fallback: &str) -> String {
    game.object(id)
        .map(|o| o.name.clone())
        .unwrap_or_else(|| format!("{fallback} #{}", id.0))
}

fn static_abilities_for_object_with_effects(
    game: &GameState,
    object_id: ObjectId,
    effects: &[crate::continuous::ContinuousEffect],
) -> Vec<crate::static_abilities::StaticAbility> {
    if let Some(calc) = game.calculated_characteristics_with_effects(object_id, effects) {
        return calc.static_abilities;
    }
    game.object(object_id)
        .map(|object| {
            object
                .abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    AbilityKind::Static(static_ability) => Some(static_ability.clone()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn generic_attack_tax_per_attacker_against_player(
    game: &GameState,
    defending_player: PlayerId,
    effects: &[crate::continuous::ContinuousEffect],
) -> u32 {
    let mut tax = 0u32;

    for &object_id in &game.battlefield {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        if object.controller != defending_player {
            continue;
        }

        let abilities = static_abilities_for_object_with_effects(game, object_id, effects);

        for ability in abilities {
            if let Some(per_attacker_tax) = ability.generic_attack_tax_per_attacker_against_you(
                game,
                object_id,
                defending_player,
            ) {
                tax = tax.saturating_add(per_attacker_tax);
            }
        }
    }

    tax
}

/// Apply attacker declarations to the combat state.
pub fn apply_attacker_declarations(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    declarations: &[AttackerDeclaration],
) -> Result<(), GameLoopError> {
    use crate::combat_state::AttackerInfo;
    use crate::triggers::AttackEventTarget;
    use std::collections::{HashMap, HashSet};

    let all_effects = game.all_continuous_effects();

    // Validate that all creatures with "must attack if able" are declared
    let legal_attackers = compute_legal_attackers(game, combat);
    let declared_creatures: HashSet<ObjectId> = declarations.iter().map(|d| d.creature).collect();
    let attacking_creatures: Vec<ObjectId> = declarations.iter().map(|d| d.creature).collect();
    let mut attacker_static_abilities: HashMap<
        ObjectId,
        Vec<crate::static_abilities::StaticAbility>,
    > = HashMap::new();

    if declarations.len() == 1 && !game.can_attack_alone(declarations[0].creature) {
        return Err(ResponseError::InvalidAttackers(
            "This creature can't attack alone".to_string(),
        )
        .into());
    }

    for attacker in &legal_attackers {
        if attacker.must_attack && !declared_creatures.contains(&attacker.creature) {
            return Err(CombatError::MustAttackNotDeclared(attacker.creature).into());
        }
    }

    let mut attackers_per_defending_player: HashMap<PlayerId, u32> = HashMap::new();
    let mut additional_attack_mana_cost = 0u32;
    for decl in declarations {
        let Some(legal_option) = legal_attackers
            .iter()
            .find(|option| option.creature == decl.creature)
        else {
            return Err(
                ResponseError::InvalidAttackers("Creature cannot attack".to_string()).into(),
            );
        };
        if !legal_option.valid_targets.contains(&decl.target) {
            return Err(ResponseError::InvalidAttackers(
                "Creature cannot attack the chosen target".to_string(),
            )
            .into());
        }

        let Some(creature) = game.object(decl.creature) else {
            return Err(ResponseError::InvalidAttackers(format!(
                "Creature #{} not found",
                decl.creature.0
            ))
            .into());
        };
        if creature.controller != game.turn.active_player {
            return Err(ResponseError::InvalidAttackers(
                "Can only attack with creatures you control".to_string(),
            )
            .into());
        }
        if !creature.is_creature() {
            return Err(ResponseError::InvalidAttackers("Not a creature".to_string()).into());
        }

        let abilities = static_abilities_for_object_with_effects(game, creature.id, &all_effects);
        for ability in &abilities {
            if let Some(can_attack) = ability.can_attack_with_attacking_group(
                game,
                creature.id,
                creature.controller,
                &attacking_creatures,
            ) && !can_attack
            {
                return Err(
                    ResponseError::InvalidAttackers(format!("{}", ability.display())).into(),
                );
            }
            if let Some(can_pay) =
                ability.can_pay_attack_cost(game, creature.id, creature.controller)
                && !can_pay
            {
                return Err(
                    ResponseError::InvalidAttackers(format!("{}", ability.display())).into(),
                );
            }
            if let Some(cost) =
                ability.generic_attack_mana_cost_for_source(game, creature.id, creature.controller)
            {
                additional_attack_mana_cost = additional_attack_mana_cost.saturating_add(cost);
            }
        }
        attacker_static_abilities.insert(creature.id, abilities);

        if let AttackTarget::Player(defending_player) = &decl.target {
            *attackers_per_defending_player
                .entry(*defending_player)
                .or_default() += 1;
        }
    }

    let total_attack_tax = attackers_per_defending_player.into_iter().fold(
        0u32,
        |acc, (defending_player, attackers)| {
            let per_attacker_tax = generic_attack_tax_per_attacker_against_player(
                game,
                defending_player,
                &all_effects,
            );
            acc.saturating_add(per_attacker_tax.saturating_mul(attackers))
        },
    );
    let total_generic_attack_mana_cost =
        total_attack_tax.saturating_add(additional_attack_mana_cost);

    for decl in declarations {
        let Some(creature) = game.object(decl.creature) else {
            return Err(
                ResponseError::InvalidAttackers("Creature cannot attack".to_string()).into(),
            );
        };
        let creature_source = creature.id;
        let creature_controller = creature.controller;
        let abilities = attacker_static_abilities
            .get(&creature_source)
            .cloned()
            .unwrap_or_else(|| {
                static_abilities_for_object_with_effects(game, creature_source, &all_effects)
            });
        for ability in abilities {
            if let Some(result) =
                ability.pay_non_mana_attack_cost(game, creature_source, creature_controller)
            {
                if let Err(msg) = result {
                    return Err(ResponseError::InvalidAttackers(msg).into());
                }
            }
        }
    }

    if total_generic_attack_mana_cost > 0 {
        let tax_cost = generic_mana_cost(total_generic_attack_mana_cost);
        if !game.can_pay_mana_cost(game.turn.active_player, None, &tax_cost, 0) {
            return Err(ResponseError::InvalidAttackers(format!(
                "Cannot pay required attack cost of {{{total_generic_attack_mana_cost}}}"
            ))
            .into());
        }
        if !game.try_pay_mana_cost(game.turn.active_player, None, &tax_cost, 0) {
            return Err(ResponseError::InvalidAttackers(format!(
                "Failed to pay required attack cost of {{{total_generic_attack_mana_cost}}}"
            ))
            .into());
        }
    }

    // Clear any existing attackers
    combat.attackers.clear();
    if !declarations.is_empty() {
        game.players_attacked_this_turn
            .insert(game.turn.active_player);
    }

    for decl in declarations {
        let Some(creature) = game.object(decl.creature) else {
            return Err(
                ResponseError::InvalidAttackers("Creature cannot attack".to_string()).into(),
            );
        };

        // Add to combat state
        combat.attackers.push(AttackerInfo {
            creature: decl.creature,
            target: decl.target.clone(),
        });

        // Tap the creature (unless it has vigilance)
        if !crate::rules::combat::has_vigilance(creature) {
            tap_permanent_with_trigger(game, trigger_queue, decl.creature);
        }

        game.mark_creature_attacked_this_turn(decl.creature);

        // Generate attack trigger
        let event_target = match &decl.target {
            AttackTarget::Player(pid) => AttackEventTarget::Player(*pid),
            AttackTarget::Planeswalker(oid) => AttackEventTarget::Planeswalker(*oid),
        };

        let event_provenance = game
            .provenance_graph
            .alloc_root_event(crate::events::EventKind::CreatureAttacked);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                decl.creature,
                event_target,
                declarations.len(),
            ),
            event_provenance,
        );
        let event = game.ensure_trigger_event_provenance(event);
        let triggers = check_triggers(game, &event);
        for trigger in triggers {
            trigger_queue.add(trigger);
        }
    }

    Ok(())
}

/// Get a decision context for declaring blockers.
pub fn get_declare_blockers_decision(
    game: &GameState,
    combat: &CombatState,
    defending_player: PlayerId,
) -> crate::decisions::context::DecisionContext {
    let attacker_options = compute_legal_blockers(game, combat, defending_player);

    // Convert to BlockersContext
    let blocker_options: Vec<crate::decisions::context::BlockerOptionContext> = attacker_options
        .into_iter()
        .map(|opt| {
            let attacker_name = game
                .object(opt.attacker)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| format!("Attacker #{}", opt.attacker.0));
            let valid_blockers: Vec<(ObjectId, String)> = opt
                .valid_blockers
                .into_iter()
                .map(|id| {
                    let name = game
                        .object(id)
                        .map(|o| o.name.clone())
                        .unwrap_or_else(|| format!("Creature #{}", id.0));
                    (id, name)
                })
                .collect();
            crate::decisions::context::BlockerOptionContext {
                attacker: opt.attacker,
                attacker_name,
                valid_blockers,
                min_blockers: opt.min_blockers,
            }
        })
        .collect();

    crate::decisions::context::DecisionContext::Blockers(
        crate::decisions::context::BlockersContext::new(defending_player, blocker_options),
    )
}

/// Apply blocker declarations to the combat state.
pub fn apply_blocker_declarations(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    declarations: &[BlockerDeclaration],
    defending_player: PlayerId,
) -> Result<(), GameLoopError> {
    // Clear existing blockers.
    combat.blockers.clear();

    // Pre-validate controlling player constraints (combat_state::declare_blockers does not).
    let mut pairs: Vec<(ObjectId, ObjectId)> = Vec::with_capacity(declarations.len());
    for decl in declarations {
        let Some(blocker) = game.object(decl.blocker) else {
            return Err(ResponseError::InvalidBlockers(format!(
                "Blocker #{} not found",
                decl.blocker.0
            ))
            .into());
        };
        if blocker.controller != defending_player {
            return Err(ResponseError::InvalidBlockers(
                "Can only block with creatures you control".to_string(),
            )
            .into());
        }
        if game.object(decl.blocking).is_none() {
            return Err(ResponseError::InvalidBlockers(format!(
                "Attacker #{} not found",
                decl.blocking.0
            ))
            .into());
        }
        pairs.push((decl.blocker, decl.blocking));
    }

    // Validate and apply using the combat rules engine (handles menace, max blockers,
    // and "can block additional attackers").
    if let Err(err) = crate::combat_state::declare_blockers(game, combat, pairs.clone()) {
        return Err(ResponseError::InvalidBlockers(err.to_string()).into());
    }

    // Emit block triggers (per declaration).
    for (blocker, attacker) in &pairs {
        let event_provenance = game
            .provenance_graph
            .alloc_root_event(crate::events::EventKind::CreatureBlocked);
        let event = TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::new(*blocker, *attacker),
            event_provenance,
        );
        let event = game.ensure_trigger_event_provenance(event);
        let triggers = check_triggers(game, &event);
        for trigger in triggers {
            trigger_queue.add(trigger);
        }
    }

    if declarations.len() == 1 && !game.can_block_alone(declarations[0].blocker) {
        return Err(
            ResponseError::InvalidBlockers("This creature can't block alone".to_string()).into(),
        );
    }

    // Generate "becomes blocked" triggers for blocked attackers
    for (attacker_id, blockers) in &combat.blockers {
        if !blockers.is_empty() {
            let attack_target =
                get_attack_target(combat, *attacker_id).map(|target| match target {
                    AttackTarget::Player(player_id) => {
                        crate::triggers::AttackEventTarget::Player(*player_id)
                    }
                    AttackTarget::Planeswalker(planeswalker_id) => {
                        crate::triggers::AttackEventTarget::Planeswalker(*planeswalker_id)
                    }
                });
            let event_provenance = game
                .provenance_graph
                .alloc_root_event(crate::events::EventKind::CreatureBecameBlocked);
            let event = TriggerEvent::new_with_provenance(
                match attack_target {
                    Some(target) => CreatureBecameBlockedEvent::with_target(
                        *attacker_id,
                        blockers.len() as u32,
                        target,
                    ),
                    None => CreatureBecameBlockedEvent::new(*attacker_id, blockers.len() as u32),
                },
                event_provenance,
            );
            let event = game.ensure_trigger_event_provenance(event);
            let triggers = check_triggers(game, &event);
            for trigger in triggers {
                trigger_queue.add(trigger);
            }
        }
    }

    // Generate "attacks and isn't blocked" triggers for unblocked attackers
    for info in &combat.attackers {
        if !is_unblocked(combat, info.creature) {
            continue;
        }

        let attack_target = match info.target {
            AttackTarget::Player(player_id) => {
                crate::triggers::AttackEventTarget::Player(player_id)
            }
            AttackTarget::Planeswalker(planeswalker_id) => {
                crate::triggers::AttackEventTarget::Planeswalker(planeswalker_id)
            }
        };

        let event_provenance = game
            .provenance_graph
            .alloc_root_event(crate::events::EventKind::CreatureAttackedAndUnblocked);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedAndUnblockedEvent::new(info.creature, attack_target),
            event_provenance,
        );
        let event = game.ensure_trigger_event_provenance(event);
        let triggers = check_triggers(game, &event);
        for trigger in triggers {
            trigger_queue.add(trigger);
        }
    }

    for (attacker_id, blockers) in &combat.blockers {
        let Some(attacker) = game.object(*attacker_id) else {
            return Err(ResponseError::InvalidBlockers(format!(
                "Attacker #{} not found",
                attacker_id.0
            ))
            .into());
        };

        let min = minimum_blockers(attacker);
        if !blockers.is_empty() && blockers.len() < min {
            return Err(ResponseError::InvalidBlockers(format!(
                "{} needs at least {} blockers",
                object_label(game, *attacker_id, "Attacker"),
                min
            ))
            .into());
        }

        if let Some(max) = maximum_blockers(attacker, game)
            && blockers.len() > max
        {
            return Err(ResponseError::InvalidBlockers(format!(
                "{} can't be blocked by more than {} creature(s)",
                object_label(game, *attacker_id, "Attacker"),
                max
            ))
            .into());
        }
    }

    Ok(())
}

/// Get a decision context for ordering blockers (damage assignment order).
pub fn get_blocker_order_decision(
    game: &GameState,
    combat: &CombatState,
    attacker: ObjectId,
) -> Option<crate::decisions::context::DecisionContext> {
    // Get the blockers for this attacker
    let blockers = combat.blockers.get(&attacker)?;

    // Only need to order if there are multiple blockers
    if blockers.len() <= 1 {
        return None;
    }

    // The attacking creature's controller orders the blockers
    let attacker_obj = game.object(attacker)?;
    let attacking_player = attacker_obj.controller;

    // Convert blockers to items with names
    let items: Vec<(ObjectId, String)> = blockers
        .iter()
        .map(|&id| {
            let name = game
                .object(id)
                .map(|o| o.name.clone())
                .unwrap_or_else(|| format!("Blocker #{}", id.0));
            (id, name)
        })
        .collect();

    let attacker_name = attacker_obj.name.clone();
    let ctx = crate::decisions::context::OrderContext::new(
        attacking_player,
        Some(attacker),
        format!("Order blockers for {}", attacker_name),
        items,
    );

    Some(crate::decisions::context::DecisionContext::Order(ctx))
}

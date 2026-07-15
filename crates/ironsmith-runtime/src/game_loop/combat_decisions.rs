use super::*;
use crate::derived_view::DerivedGameView;

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
                .map(|o| o.name.to_string())
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

pub(super) fn generic_mana_cost(amount: u32) -> crate::mana::ManaCost {
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

pub(super) fn static_abilities_for_object_with_effects(
    game: &GameState,
    object_id: ObjectId,
    effects: &[crate::continuous::ContinuousEffect],
) -> Vec<crate::static_abilities::StaticAbility> {
    if let Some(calc) = game.calculated_characteristics_with_effects(object_id, effects) {
        return calc.static_abilities.to_vec();
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

fn attack_requirement_score_for_target(
    game: &GameState,
    attacker: &crate::object::Object,
    abilities: &[crate::static_abilities::StaticAbility],
    target: &AttackTarget,
) -> usize {
    let controller = game.controller_of(attacker);
    let attacks_player_other_than = |goading_player| matches!(target, AttackTarget::Player(defender) if *defender != goading_player);
    let mut score = abilities
        .iter()
        .filter(|ability| ability.id() == crate::static_abilities::StaticAbilityId::MustAttack)
        .count();

    for effect in &game.effect_store.goad_effects {
        if effect.creature == attacker.id && effect.is_active(game, game.turn.turn_number) {
            score += 1;
            score += usize::from(attacks_player_other_than(effect.goaded_by));
        }
    }
    for ability in abilities {
        if let Some(goading_player) = ability.goaded_by_player(game, attacker.id, controller) {
            score += 1;
            score += usize::from(attacks_player_other_than(goading_player));
        }
        if let Some(required_player) = ability.required_attack_player(game, attacker.id, controller)
        {
            score += usize::from(
                matches!(target, AttackTarget::Player(defender) if *defender == required_player),
            );
        }
    }

    score
}

pub(super) fn player_assigns_combat_damage_of_creatures_attacking_them(
    game: &GameState,
    player: PlayerId,
) -> bool {
    let all_effects = game.all_continuous_effects();
    for &source_id in &game.battlefield {
        let Some(source) = game.object(source_id) else {
            continue;
        };
        if game.controller_of(source) != player {
            continue;
        }
        for ability in static_abilities_for_object_with_effects(game, source_id, &all_effects) {
            if ability.id()
                == crate::static_abilities::StaticAbilityId::YouAssignCombatDamageOfCreaturesAttackingYou
            {
                return true;
            }
        }
    }
    false
}

pub(super) fn defender_assigns_combat_damage_for_attacker(
    game: &GameState,
    combat: &CombatState,
    attacker: ObjectId,
) -> bool {
    if let Some(crate::combat_state::AttackTarget::Player(defender)) =
        crate::combat_state::get_attack_target(combat, attacker)
    {
        return player_assigns_combat_damage_of_creatures_attacking_them(game, *defender);
    }

    false
}

pub fn combat_damage_assignment_player_for_attacker(
    game: &GameState,
    combat: &CombatState,
    attacker: ObjectId,
) -> Option<PlayerId> {
    let attacker_obj = game.object(attacker)?;
    let attacking_player = game.controller_of(attacker_obj);
    if defender_assigns_combat_damage_for_attacker(game, combat, attacker) {
        if let crate::combat_state::AttackTarget::Player(defender) =
            crate::combat_state::get_attack_target(combat, attacker)?
        {
            return Some(*defender);
        }
    }

    Some(attacking_player)
}

pub(super) fn generic_attack_tax_per_attacker_against_player(
    game: &GameState,
    defending_player: PlayerId,
    effects: &[crate::continuous::ContinuousEffect],
) -> u32 {
    let mut tax = 0u32;

    for &object_id in &game.battlefield {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        if game.controller_of(object) != defending_player {
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

fn required_attack_cost_message_for_unpreviewed_attack(
    game: &GameState,
    all_effects: &[crate::continuous::ContinuousEffect],
    creature_id: ObjectId,
    target: &AttackTarget,
) -> Option<String> {
    let creature = game.object(creature_id)?;
    let defending_player = match target {
        AttackTarget::Player(player_id) => Some(*player_id),
        AttackTarget::Planeswalker(object_id) => {
            game.object(*object_id).map(|obj| game.controller_of(obj))
        }
    }?;
    let view = DerivedGameView::new(game);
    if !crate::rules::combat::can_attack_defending_player_with_view(
        creature,
        defending_player,
        game,
        &view,
    ) {
        return None;
    }

    let abilities = static_abilities_for_object_with_effects(game, creature.id, all_effects);
    if abilities.iter().any(|ability| {
        ability
            .can_pay_attack_cost(game, creature.id, game.controller_of(creature))
            .is_some_and(|can_pay| !can_pay)
    }) {
        return Some("Cannot pay required attack cost".to_string());
    }

    let total_generic_attack_mana_cost = abilities.iter().fold(
        generic_attack_tax_per_attacker_against_player(game, defending_player, all_effects),
        |acc, ability| {
            acc.saturating_add(
                ability
                    .generic_attack_mana_cost_for_source(
                        game,
                        creature.id,
                        game.controller_of(creature),
                    )
                    .unwrap_or(0),
            )
        },
    );

    if total_generic_attack_mana_cost == 0 {
        return None;
    }

    Some(format!(
        "Cannot pay required attack cost of {{{total_generic_attack_mana_cost}}}"
    ))
}

#[derive(Debug, Clone)]
struct PreparedAttackerDeclaration {
    declaration: AttackerDeclaration,
    controller: PlayerId,
    abilities: Vec<crate::static_abilities::StaticAbility>,
    optional_attack_cost_prompts: Vec<(usize, crate::decisions::context::BooleanContext)>,
    has_vigilance: bool,
    had_to_attack_this_combat: bool,
}

#[derive(Debug, Clone)]
struct PreparedAttackDeclarations {
    declarations: Vec<PreparedAttackerDeclaration>,
    total_generic_attack_mana_cost: u32,
    has_post_tap_attack_costs: bool,
}

fn prepare_attacker_declarations(
    game: &GameState,
    combat: &CombatState,
    declarations: &[AttackerDeclaration],
) -> Result<PreparedAttackDeclarations, GameLoopError> {
    prepare_attacker_declarations_internal(game, combat, declarations, true)
}

fn prepare_attacker_declarations_internal(
    game: &GameState,
    combat: &CombatState,
    declarations: &[AttackerDeclaration],
    enforce_requirements: bool,
) -> Result<PreparedAttackDeclarations, GameLoopError> {
    use std::collections::{HashMap, HashSet};

    let declaration_view = DerivedGameView::new(game);
    let all_effects = declaration_view.effects();
    let legal_attackers =
        crate::decision::compute_legal_attackers_with_view(game, combat, &declaration_view);
    let mut declared_creatures = HashSet::with_capacity(declarations.len());
    for declaration in declarations {
        if !declared_creatures.insert(declaration.creature) {
            return Err(ResponseError::InvalidAttackers(
                "A creature can't be declared as an attacker more than once".to_string(),
            )
            .into());
        }
    }
    let attacking_creatures: Vec<ObjectId> = declarations.iter().map(|d| d.creature).collect();
    let legal_attackers_by_creature: crate::FxMap<ObjectId, &crate::decision::AttackerOption> =
        legal_attackers
            .iter()
            .map(|option| (option.creature, option))
            .collect();

    if declarations.len() == 1 && !game.can_attack_alone(declarations[0].creature) {
        return Err(ResponseError::InvalidAttackers(
            "This creature can't attack alone".to_string(),
        )
        .into());
    }

    let mut attackers_per_defending_player: HashMap<PlayerId, u32> = HashMap::new();
    let mut additional_attack_mana_cost = 0u32;
    let mut has_post_tap_attack_costs = false;
    let mut requirements_obeyed = 0usize;
    let mut prepared = Vec::with_capacity(declarations.len());

    for decl in declarations {
        let Some(&legal_option) = legal_attackers_by_creature.get(&decl.creature) else {
            if let Some(msg) = required_attack_cost_message_for_unpreviewed_attack(
                game,
                all_effects,
                decl.creature,
                &decl.target,
            ) {
                return Err(ResponseError::InvalidAttackers(msg).into());
            }
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
        if game.controller_of(creature) != game.turn.active_player {
            return Err(ResponseError::InvalidAttackers(
                "Can only attack with creatures you control".to_string(),
            )
            .into());
        }
        if !game.current_is_creature(creature.id) {
            return Err(ResponseError::InvalidAttackers("Not a creature".to_string()).into());
        }

        let abilities = declaration_view
            .calculated_characteristics_arc(creature.id)
            .map(|chars| chars.static_abilities.to_vec())
            .unwrap_or_else(|| {
                creature
                    .abilities
                    .iter()
                    .filter_map(|ability| match &ability.kind {
                        AbilityKind::Static(static_ability) => Some(static_ability.clone()),
                        _ => None,
                    })
                    .collect()
            });
        let creature_controller = game.controller_of(creature);
        let mut optional_attack_cost_prompts = Vec::new();
        for (ability_index, ability) in abilities.iter().enumerate() {
            if let Some(can_attack) = ability.can_attack_with_attacking_group(
                game,
                creature.id,
                creature_controller,
                &attacking_creatures,
            ) && !can_attack
            {
                return Err(
                    ResponseError::InvalidAttackers(format!("{}", ability.display())).into(),
                );
            }
            let can_pay_attack_cost =
                ability.can_pay_attack_cost(game, creature.id, creature_controller);
            if can_pay_attack_cost.is_some_and(|can_pay| !can_pay) {
                return Err(
                    ResponseError::InvalidAttackers(format!("{}", ability.display())).into(),
                );
            }
            if let Some(cost) =
                ability.generic_attack_mana_cost_for_source(game, creature.id, creature_controller)
            {
                additional_attack_mana_cost = additional_attack_mana_cost.saturating_add(cost);
            }

            let optional_prompt =
                ability.optional_attack_cost_prompt(game, creature.id, creature_controller);
            has_post_tap_attack_costs |= can_pay_attack_cost.is_some() || optional_prompt.is_some();
            if let Some(prompt) = optional_prompt {
                optional_attack_cost_prompts.push((ability_index, prompt));
            }
        }
        requirements_obeyed +=
            attack_requirement_score_for_target(game, creature, &abilities, &decl.target);

        prepared.push(PreparedAttackerDeclaration {
            declaration: decl.clone(),
            controller: creature_controller,
            optional_attack_cost_prompts,
            has_vigilance: abilities
                .iter()
                .any(|ability| ability.id() == crate::static_abilities::StaticAbilityId::Vigilance),
            abilities,
            had_to_attack_this_combat: legal_option.must_attack,
        });

        if let Some(defending_player) =
            crate::combat_state::defending_player_for_attack_target(game, &decl.target)
        {
            *attackers_per_defending_player
                .entry(defending_player)
                .or_default() += 1;
        }
    }

    if let Some(maximum) = crate::combat_state::max_creatures_can_attack_each_combat(game)
        && declarations.len() > maximum
    {
        return Err(CombatError::TooManyAttackers {
            maximum,
            provided: declarations.len(),
        }
        .into());
    }

    for (&defending_player, &provided) in &attackers_per_defending_player {
        if let Some(maximum) =
            crate::combat_state::max_creatures_can_attack_defending_player_each_combat(
                game,
                defending_player,
            )
            && provided as usize > maximum
        {
            return Err(CombatError::TooManyAttackers {
                maximum,
                provided: provided as usize,
            }
            .into());
        }
    }

    if enforce_requirements {
        if attack_declaration_obeying_more_requirements_exists(
            game,
            combat,
            &legal_attackers,
            declarations,
            requirements_obeyed,
        ) {
            if let Some(omitted) = legal_attackers
                .iter()
                .find(|option| option.must_attack && !declared_creatures.contains(&option.creature))
                .map(|option| option.creature)
            {
                return Err(CombatError::MustAttackNotDeclared(omitted).into());
            }
            return Err(ResponseError::InvalidAttackers(
                "The declaration obeys fewer attack requirements than another legal declaration"
                    .to_string(),
            )
            .into());
        }
    }

    let total_attack_tax = attackers_per_defending_player.into_iter().fold(
        0u32,
        |acc, (defending_player, attackers)| {
            let per_attacker_tax =
                generic_attack_tax_per_attacker_against_player(game, defending_player, all_effects);
            acc.saturating_add(per_attacker_tax.saturating_mul(attackers))
        },
    );

    let total_generic_attack_mana_cost =
        total_attack_tax.saturating_add(additional_attack_mana_cost);

    Ok(PreparedAttackDeclarations {
        declarations: prepared,
        total_generic_attack_mana_cost,
        has_post_tap_attack_costs: has_post_tap_attack_costs || total_generic_attack_mana_cost > 0,
    })
}

fn attack_declaration_obeying_more_requirements_exists(
    game: &GameState,
    combat: &CombatState,
    legal_attackers: &[crate::decision::AttackerOption],
    current_declarations: &[AttackerDeclaration],
    baseline: usize,
) -> bool {
    struct ScoredAttackOption<'a> {
        option: &'a crate::decision::AttackerOption,
        target_scores: Vec<usize>,
        target_requires_cost: Vec<bool>,
        maximum_score: usize,
    }

    let view = DerivedGameView::new(game);
    let mut options = legal_attackers
        .iter()
        .filter_map(|option| {
            let attacker = game.object(option.creature)?;
            let abilities =
                static_abilities_for_object_with_effects(game, attacker.id, view.effects());
            let target_scores = option
                .valid_targets
                .iter()
                .map(|target| {
                    attack_requirement_score_for_target(game, attacker, &abilities, target)
                })
                .collect::<Vec<_>>();
            let target_requires_cost = option
                .valid_targets
                .iter()
                .map(|target| {
                    let has_creature_cost = abilities.iter().any(|ability| {
                        ability
                            .can_pay_attack_cost(game, attacker.id, game.controller_of(attacker))
                            .is_some()
                            || ability
                                .generic_attack_mana_cost_for_source(
                                    game,
                                    attacker.id,
                                    game.controller_of(attacker),
                                )
                                .is_some_and(|cost| cost > 0)
                    });
                    let has_defender_tax =
                        crate::combat_state::defending_player_for_attack_target(game, target)
                            .is_some_and(|defending_player| {
                                generic_attack_tax_per_attacker_against_player(
                                    game,
                                    defending_player,
                                    view.effects(),
                                ) > 0
                            });
                    has_creature_cost || has_defender_tax
                })
                .collect::<Vec<_>>();
            let maximum_score = target_scores.iter().copied().max().unwrap_or(0);
            Some(ScoredAttackOption {
                option,
                target_scores,
                target_requires_cost,
                maximum_score,
            })
        })
        .collect::<Vec<_>>();
    let global_cap =
        crate::combat_state::max_creatures_can_attack_each_combat(game).unwrap_or(usize::MAX);
    let mut unconstrained_scores = options
        .iter()
        .map(|option| option.maximum_score)
        .collect::<Vec<_>>();
    unconstrained_scores.sort_unstable_by(|left, right| right.cmp(left));
    let unconstrained_maximum = unconstrained_scores
        .into_iter()
        .take(global_cap)
        .sum::<usize>();
    if baseline >= unconstrained_maximum {
        return false;
    }

    options.sort_by_key(|option| std::cmp::Reverse(option.maximum_score));
    let remaining_requirements = (0..=options.len())
        .map(|index| {
            options[index..]
                .iter()
                .map(|option| option.maximum_score)
                .sum()
        })
        .collect::<Vec<_>>();

    fn search(
        game: &GameState,
        combat: &CombatState,
        options: &[ScoredAttackOption<'_>],
        current_declarations: &[AttackerDeclaration],
        remaining_requirements: &[usize],
        index: usize,
        declarations: &mut Vec<AttackerDeclaration>,
        requirements_obeyed: usize,
        baseline: usize,
        global_cap: usize,
    ) -> bool {
        if requirements_obeyed > baseline
            && prepare_attacker_declarations_internal(game, combat, declarations, false).is_ok()
        {
            return true;
        }
        if index == options.len() {
            return false;
        }
        if requirements_obeyed <= baseline
            && requirements_obeyed + remaining_requirements[index] <= baseline
        {
            return false;
        }

        let option = &options[index];
        if declarations.len() < global_cap {
            for (target_index, target) in option.option.valid_targets.iter().enumerate() {
                if option.target_requires_cost[target_index]
                    && !current_declarations.contains(&AttackerDeclaration {
                        creature: option.option.creature,
                        target: target.clone(),
                    })
                {
                    continue;
                }
                let target_is_within_cap = crate::combat_state::defending_player_for_attack_target(
                    game, target,
                )
                .is_none_or(|defending_player| {
                    let already_attacking = declarations
                        .iter()
                        .filter(|declaration| {
                            crate::combat_state::defending_player_for_attack_target(
                                game,
                                &declaration.target,
                            ) == Some(defending_player)
                        })
                        .count();
                    crate::combat_state::max_creatures_can_attack_defending_player_each_combat(
                        game,
                        defending_player,
                    )
                    .is_none_or(|maximum| already_attacking < maximum)
                });
                if !target_is_within_cap {
                    continue;
                }

                declarations.push(AttackerDeclaration {
                    creature: option.option.creature,
                    target: target.clone(),
                });
                if search(
                    game,
                    combat,
                    options,
                    current_declarations,
                    remaining_requirements,
                    index + 1,
                    declarations,
                    requirements_obeyed + option.target_scores[target_index],
                    baseline,
                    global_cap,
                ) {
                    return true;
                }
                declarations.pop();
            }
        }

        search(
            game,
            combat,
            options,
            current_declarations,
            remaining_requirements,
            index + 1,
            declarations,
            requirements_obeyed,
            baseline,
            global_cap,
        )
    }

    search(
        game,
        combat,
        &options,
        current_declarations,
        &remaining_requirements,
        0,
        &mut Vec::new(),
        0,
        baseline,
        global_cap,
    )
}

fn collect_optional_attack_cost_prompts(
    _game: &GameState,
    prepared: &PreparedAttackDeclarations,
) -> Vec<crate::decisions::context::BooleanContext> {
    let mut prompts = Vec::new();
    for prepared_decl in &prepared.declarations {
        prompts.extend(
            prepared_decl
                .optional_attack_cost_prompts
                .iter()
                .map(|(_, prompt)| prompt.clone()),
        );
    }
    prompts
}

pub fn preview_optional_attack_cost_prompts(
    game: &GameState,
    combat: &CombatState,
    declarations: &[AttackerDeclaration],
) -> Result<Vec<crate::decisions::context::BooleanContext>, GameLoopError> {
    let prepared = prepare_attacker_declarations(game, combat, declarations)?;
    Ok(collect_optional_attack_cost_prompts(game, &prepared))
}

pub fn preview_required_attack_mana_cost(
    game: &GameState,
    combat: &CombatState,
    declarations: &[AttackerDeclaration],
) -> Result<u32, GameLoopError> {
    Ok(prepare_attacker_declarations(game, combat, declarations)?.total_generic_attack_mana_cost)
}

#[derive(Debug, Clone)]
pub(crate) struct AttackDeclarationTransaction {
    prepared: PreparedAttackDeclarations,
    game_checkpoint: Box<GameState>,
    combat_checkpoint: CombatState,
    trigger_queue_checkpoint: TriggerQueue,
    tapped_events: Vec<TriggerEvent>,
    queued_tapped_events_before_costs: bool,
}

fn tap_prepared_attackers(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    prepared: &PreparedAttackDeclarations,
) -> (Vec<TriggerEvent>, bool) {
    // CR 508.1f taps every chosen attacker before attack costs are paid. Use
    // the pre-cost vigilance result prepared from the same derived state as
    // attack legality; paying a cost can remove the source of that ability.
    let mut tapped_events = Vec::new();
    for prepared_decl in &prepared.declarations {
        let creature = prepared_decl.declaration.creature;
        if prepared_decl.has_vigilance
            || game.object(creature).is_none()
            || game.is_tapped(creature)
        {
            continue;
        }

        game.tap(creature);
        let event_provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::PermanentTapped);
        tapped_events.push(TriggerEvent::new_with_provenance(
            crate::events::PermanentTappedEvent::new(creature),
            event_provenance,
        ));
    }

    // If costs can change the battlefield or other trigger-relevant state,
    // match the simultaneous tap events against the pre-cost state. Otherwise
    // they can share the post-declaration refresh below.
    let queued_tapped_events_before_costs =
        prepared.has_post_tap_attack_costs && !tapped_events.is_empty();
    if queued_tapped_events_before_costs {
        game.refresh_continuous_state();
        for event in tapped_events.iter().cloned() {
            queue_triggers_from_event(game, trigger_queue, event, true);
        }
    }

    (tapped_events, queued_tapped_events_before_costs)
}

fn apply_prepared_attacker_declarations_with_dm(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    prepared: PreparedAttackDeclarations,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    let (tapped_events, queued_tapped_events_before_costs) =
        tap_prepared_attackers(game, trigger_queue, &prepared);
    apply_prepared_attacker_declarations_after_tapping_with_dm(
        game,
        combat,
        trigger_queue,
        prepared,
        tapped_events,
        queued_tapped_events_before_costs,
        decision_maker,
    )
}

fn apply_prepared_attacker_declarations_after_tapping_with_dm(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    prepared: PreparedAttackDeclarations,
    tapped_events: Vec<TriggerEvent>,
    queued_tapped_events_before_costs: bool,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    use crate::combat_state::AttackerInfo;
    use crate::triggers::AttackEventTarget;

    for prepared_decl in &prepared.declarations {
        let creature_source = prepared_decl.declaration.creature;
        let creature_controller = prepared_decl.controller;

        for ability in &prepared_decl.abilities {
            if let Some(result) =
                ability.pay_non_mana_attack_cost(game, creature_source, creature_controller)
            {
                if let Err(msg) = result {
                    return Err(ResponseError::InvalidAttackers(msg).into());
                }
            }
        }
        for (ability_index, prompt) in &prepared_decl.optional_attack_cost_prompts {
            if !decision_maker.decide_boolean(game, prompt) {
                continue;
            }
            let ability = &prepared_decl.abilities[*ability_index];
            if let Some(result) = ability.pay_optional_attack_cost(
                game,
                creature_source,
                creature_controller,
                trigger_queue,
            ) && let Err(msg) = result
            {
                return Err(ResponseError::InvalidAttackers(msg).into());
            }
        }
    }

    if prepared.total_generic_attack_mana_cost > 0 {
        let tax_cost = generic_mana_cost(prepared.total_generic_attack_mana_cost);
        if !game.can_pay_mana_cost(game.turn.active_player, None, &tax_cost, 0) {
            return Err(ResponseError::InvalidAttackers(format!(
                "Cannot pay required attack cost of {{{}}}",
                prepared.total_generic_attack_mana_cost
            ))
            .into());
        }
        if !game.try_pay_mana_cost(game.turn.active_player, None, &tax_cost, 0) {
            return Err(ResponseError::InvalidAttackers(format!(
                "Failed to pay required attack cost of {{{}}}",
                prepared.total_generic_attack_mana_cost
            ))
            .into());
        }
    }

    // Costs may have removed or changed control of chosen creatures. Build the
    // final combat state off to the side, then publish it once so every attack
    // trigger observes the complete declaration.
    let mut next_combat = combat.clone();
    next_combat.attackers.clear();
    next_combat.had_to_attack_this_combat.clear();
    let post_cost_view = DerivedGameView::new(game);
    let post_cost_candidates = prepared
        .declarations
        .iter()
        .map(|prepared_decl| prepared_decl.declaration.creature)
        .collect::<Vec<_>>();
    post_cost_view.prewarm_characteristics(&post_cost_candidates);
    let mut surviving_declarations = Vec::with_capacity(prepared.declarations.len());
    for prepared_decl in &prepared.declarations {
        let decl = &prepared_decl.declaration;
        let remains_controlled_battlefield_creature =
            game.object(decl.creature).is_some_and(|obj| {
                obj.zone == Zone::Battlefield && game.controller_of(obj) == prepared_decl.controller
            }) && post_cost_view.object_has_card_type(decl.creature, CardType::Creature);
        if !remains_controlled_battlefield_creature {
            continue;
        }

        next_combat.attackers.push(AttackerInfo {
            creature: decl.creature,
            target: decl.target.clone(),
        });
        if prepared_decl.had_to_attack_this_combat {
            next_combat.had_to_attack_this_combat.insert(decl.creature);
        }
        surviving_declarations.push(prepared_decl);
    }

    *combat = next_combat;
    game.combat = Some(combat.clone());

    if !surviving_declarations.is_empty() {
        let active_player = game.turn.active_player;
        let history = &mut game.turn_store.turn_history;
        history.players_attacked_this_turn.insert(active_player);
        for prepared_decl in &surviving_declarations {
            let creature = prepared_decl.declaration.creature;
            history.creatures_attacked_this_turn.insert(creature);
            *history
                .creature_attack_counts_this_turn
                .entry(creature)
                .or_insert(0) += 1;
        }
    }

    game.mark_continuous_state_dirty();
    game.refresh_continuous_state();

    if !queued_tapped_events_before_costs {
        for event in tapped_events {
            queue_triggers_from_event(game, trigger_queue, event, true);
        }
    }

    let total_attackers = surviving_declarations.len();
    let mut attack_events = Vec::with_capacity(total_attackers);
    for prepared_decl in surviving_declarations {
        let decl = &prepared_decl.declaration;

        let event_target = match &decl.target {
            AttackTarget::Player(pid) => AttackEventTarget::Player(*pid),
            AttackTarget::Planeswalker(oid) => AttackEventTarget::Planeswalker(*oid),
        };

        let event_provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::CreatureAttacked);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                decl.creature,
                event_target,
                total_attackers,
            ),
            event_provenance,
        );
        attack_events.push(event);
    }
    queue_triggers_for_simultaneous_events(game, trigger_queue, attack_events);

    Ok(())
}

pub(crate) fn begin_attack_declaration_transaction(
    game: &mut GameState,
    combat: &CombatState,
    trigger_queue: &mut TriggerQueue,
    declarations: &[AttackerDeclaration],
) -> Result<AttackDeclarationTransaction, GameLoopError> {
    let prepared = prepare_attacker_declarations(game, combat, declarations)?;
    if !prepared.has_post_tap_attack_costs {
        return Err(GameLoopError::InvalidState(
            "attack declaration transaction requested without an attack cost".to_string(),
        ));
    }

    let game_checkpoint = Box::new(game.clone());
    let combat_checkpoint = combat.clone();
    let trigger_queue_checkpoint = trigger_queue.clone();
    let (tapped_events, queued_tapped_events_before_costs) =
        tap_prepared_attackers(game, trigger_queue, &prepared);

    Ok(AttackDeclarationTransaction {
        prepared,
        game_checkpoint,
        combat_checkpoint,
        trigger_queue_checkpoint,
        tapped_events,
        queued_tapped_events_before_costs,
    })
}

pub(crate) fn finish_attack_declaration_transaction(
    transaction: AttackDeclarationTransaction,
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    let AttackDeclarationTransaction {
        prepared,
        game_checkpoint,
        combat_checkpoint,
        trigger_queue_checkpoint,
        tapped_events,
        queued_tapped_events_before_costs,
    } = transaction;

    let result = apply_prepared_attacker_declarations_after_tapping_with_dm(
        game,
        combat,
        trigger_queue,
        prepared,
        tapped_events,
        queued_tapped_events_before_costs,
        decision_maker,
    );
    if result.is_err() {
        *game = *game_checkpoint;
        *combat = combat_checkpoint;
        *trigger_queue = trigger_queue_checkpoint;
    }
    result
}

/// Apply attacker declarations to the combat state.
pub fn apply_attacker_declarations(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    declarations: &[AttackerDeclaration],
) -> Result<(), GameLoopError> {
    let mut decision_maker = crate::decision::AutoPassDecisionMaker;
    apply_attacker_declarations_with_dm(
        game,
        combat,
        trigger_queue,
        declarations,
        &mut decision_maker,
    )
}

/// Apply attacker declarations to the combat state using the provided decision maker.
pub fn apply_attacker_declarations_with_dm(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    declarations: &[AttackerDeclaration],
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    let prepared = prepare_attacker_declarations(game, combat, declarations)?;

    // Once preparation succeeds, a declaration without attack costs has no
    // fallible step after attackers are tapped. Keep that common path free of
    // checkpoint cloning, especially for very large attack batches.
    if !prepared.has_post_tap_attack_costs {
        return apply_prepared_attacker_declarations_with_dm(
            game,
            combat,
            trigger_queue,
            prepared,
            decision_maker,
        );
    }

    // Attack costs are announced and paid as one declaration transaction.
    // Individual affordability checks are insufficient because declarations
    // can compete for the same resource (for example, two attackers that each
    // require sacrificing an artifact). Preserve every mutable output before
    // CR 508.1f taps attackers or any required/optional cost is paid, then
    // restore it if any later payment or validation fails.
    let game_checkpoint = game.clone();
    let combat_checkpoint = combat.clone();
    let trigger_queue_checkpoint = trigger_queue.clone();

    let result = apply_prepared_attacker_declarations_with_dm(
        game,
        combat,
        trigger_queue,
        prepared,
        decision_maker,
    );
    if result.is_err() {
        *game = game_checkpoint;
        *combat = combat_checkpoint;
        *trigger_queue = trigger_queue_checkpoint;
    }
    result
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
                .map(|o| o.name.to_string())
                .unwrap_or_else(|| format!("Attacker #{}", opt.attacker.0));
            let valid_blockers: Vec<(ObjectId, String)> = opt
                .valid_blockers
                .into_iter()
                .map(|id| {
                    let name = game
                        .object(id)
                        .map(|o| o.name.to_string())
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
    apply_blocker_declarations_internal(
        game,
        combat,
        trigger_queue,
        declarations,
        Some(defending_player),
    )
}

/// Apply the declarations collected from every attacked defending player as
/// one declare-blockers turn-based action.
pub fn apply_multiplayer_blocker_declarations(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    declarations: &[BlockerDeclaration],
) -> Result<(), GameLoopError> {
    apply_blocker_declarations_internal(game, combat, trigger_queue, declarations, None)
}

fn apply_blocker_declarations_internal(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    declarations: &[BlockerDeclaration],
    expected_defending_player: Option<PlayerId>,
) -> Result<(), GameLoopError> {
    // Pre-validate constraints not covered by combat_state::declare_blockers.
    // Do all validation against an unpublished combat clone so an invalid
    // response cannot partially clear or replace the current declarations.
    let mut pairs: Vec<(ObjectId, ObjectId)> = Vec::with_capacity(declarations.len());
    for decl in declarations {
        let Some(blocker) = game.object(decl.blocker) else {
            return Err(ResponseError::InvalidBlockers(format!(
                "Blocker #{} not found",
                decl.blocker.0
            ))
            .into());
        };
        let blocker_controller = game.controller_of(blocker);
        if expected_defending_player.is_some_and(|player| player != blocker_controller) {
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
        if crate::combat_state::defending_player_for_attacker(game, combat, decl.blocking)
            != Some(blocker_controller)
        {
            return Err(ResponseError::InvalidBlockers(
                "A defending player can block only creatures attacking that player or a planeswalker they control"
                    .to_string(),
            )
            .into());
        }
        pairs.push((decl.blocker, decl.blocking));
    }

    if declarations.len() == 1 && !game.can_block_alone(declarations[0].blocker) {
        return Err(
            ResponseError::InvalidBlockers("This creature can't block alone".to_string()).into(),
        );
    }

    // Validate and apply using the combat rules engine (handles menace, max blockers,
    // and "can block additional attackers").
    let mut next_combat = combat.clone();
    next_combat.blockers.clear();
    if let Err(err) = crate::combat_state::declare_blockers(game, &mut next_combat, pairs.clone()) {
        return Err(ResponseError::InvalidBlockers(err.to_string()).into());
    }

    // Block triggers can depend on the complete set of blockers declared together.
    *combat = next_combat;
    game.combat = Some(combat.clone());
    game.mark_continuous_state_dirty();
    game.refresh_continuous_state();

    // Capture every required event snapshot before checking any trigger. A
    // trigger check can update pass-local state, but all declaration events
    // must describe the same completed blocking configuration.
    let mut seen_snapshot_ids = std::collections::HashSet::new();
    let mut snapshot_ids = Vec::new();
    let mut remember_snapshot = |id| {
        if seen_snapshot_ids.insert(id) {
            snapshot_ids.push(id);
        }
    };
    for (blocker, attacker) in &pairs {
        remember_snapshot(*blocker);
        remember_snapshot(*attacker);
    }
    for attacker_info in &combat.attackers {
        let attacker = attacker_info.creature;
        let Some(blockers) = combat.blockers.get(&attacker) else {
            continue;
        };
        if blockers.is_empty() {
            continue;
        }
        remember_snapshot(attacker);
        for &blocker in blockers {
            remember_snapshot(blocker);
        }
    }
    game.prewarm_calculated_characteristics(&snapshot_ids);
    let snapshots = snapshot_ids
        .into_iter()
        .filter_map(|id| {
            game.object(id).map(|object| {
                (
                    id,
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        object, game,
                    ),
                )
            })
        })
        .collect::<crate::FxMap<_, _>>();

    let mut block_events = Vec::with_capacity(
        pairs
            .len()
            .saturating_mul(2)
            .saturating_add(combat.attackers.len()),
    );

    // Emit block triggers (per declaration).
    for (blocker, attacker) in &pairs {
        let event_provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::CreatureBlocked);
        let blocker_snapshot = snapshots.get(blocker).cloned();
        let attacker_snapshot = snapshots.get(attacker).cloned();
        let blocked_event = match (blocker_snapshot, attacker_snapshot) {
            (Some(blocker_snapshot), Some(attacker_snapshot)) => {
                CreatureBlockedEvent::with_snapshots(
                    *blocker,
                    *attacker,
                    blocker_snapshot,
                    attacker_snapshot,
                )
            }
            _ => CreatureBlockedEvent::new(*blocker, *attacker),
        };
        let event = TriggerEvent::new_with_provenance(blocked_event, event_provenance);
        block_events.push(event);
    }

    // Generate "becomes blocked" triggers for blocked attackers
    for attacker_info in &combat.attackers {
        let attacker_id = attacker_info.creature;
        let Some(blockers) = combat.blockers.get(&attacker_id) else {
            continue;
        };
        if !blockers.is_empty() {
            let attack_target = Some(match &attacker_info.target {
                AttackTarget::Player(player_id) => {
                    crate::triggers::AttackEventTarget::Player(*player_id)
                }
                AttackTarget::Planeswalker(planeswalker_id) => {
                    crate::triggers::AttackEventTarget::Planeswalker(*planeswalker_id)
                }
            });
            let event_provenance = game
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::CreatureBecameBlocked);
            let attacker_snapshot = snapshots.get(&attacker_id).cloned();
            let blocker_snapshots = blockers
                .iter()
                .filter_map(|blocker| snapshots.get(blocker).cloned())
                .collect::<Vec<_>>();
            let event = TriggerEvent::new_with_provenance(
                CreatureBecameBlockedEvent::with_target_and_blockers(
                    attacker_id,
                    blockers.clone(),
                    attack_target,
                    attacker_snapshot,
                    blocker_snapshots,
                ),
                event_provenance,
            );
            block_events.push(event);
        }
    }

    // Generate "attacks and isn't blocked" triggers for unblocked attackers
    for info in &combat.attackers {
        // The loop already proves this object is attacking; only the O(1)
        // blocker lookup is needed here.
        if is_blocked(combat, info.creature) {
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
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::CreatureAttackedAndUnblocked);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedAndUnblockedEvent::new(info.creature, attack_target),
            event_provenance,
        );
        block_events.push(event);
    }

    queue_triggers_for_simultaneous_events(game, trigger_queue, block_events);

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

    let attacker_obj = game.object(attacker)?;
    let assignment_player = combat_damage_assignment_player_for_attacker(game, combat, attacker)?;

    // Convert blockers to items with names
    let items: Vec<(ObjectId, String)> = blockers
        .iter()
        .map(|&id| {
            let name = game
                .object(id)
                .map(|o| o.name.to_string())
                .unwrap_or_else(|| format!("Blocker #{}", id.0));
            (id, name)
        })
        .collect();

    let attacker_name = attacker_obj.name.to_string();
    let ctx = crate::decisions::context::OrderContext::new(
        assignment_player,
        Some(attacker),
        format!("Order blockers for {}", attacker_name),
        items,
    );

    Some(crate::decisions::context::DecisionContext::Order(ctx))
}

#[cfg(test)]
mod declaration_batch_tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::filter::ObjectFilterExt as _;
    use crate::ids::CardId;
    use crate::static_abilities::{
        AttackCostCondition, CantAttackUnlessConditionSpec, StaticAbility,
    };

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_attacker(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        vigilance: bool,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let id = game.create_object_from_card(&card, controller, Zone::Battlefield);
        if vigilance {
            game.object_mut(id)
                .expect("attacker exists")
                .abilities_mut()
                .push(Ability::static_ability(StaticAbility::vigilance()));
        }
        game.remove_summoning_sickness(id);
        id
    }

    #[test]
    fn duplicate_attacker_declaration_is_rejected_before_mutation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_attacker(&mut game, alice, "Only Once", false);
        game.refresh_continuous_state();

        let declarations = vec![
            AttackerDeclaration {
                creature: attacker,
                target: AttackTarget::Player(bob),
            },
            AttackerDeclaration {
                creature: attacker,
                target: AttackTarget::Player(bob),
            },
        ];
        let mut combat = CombatState::default();
        let mut trigger_queue = TriggerQueue::new();

        assert!(
            apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations,)
                .is_err()
        );
        assert!(combat.attackers.is_empty());
        assert!(!game.is_tapped(attacker));
        assert!(!game.creature_attacked_this_turn(attacker));
    }

    #[test]
    fn attack_requirements_use_the_maximum_satisfiable_declaration() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let required = create_attacker(&mut game, alice, "Required Attacker", false);
        let optional = create_attacker(&mut game, alice, "Optional Attacker", false);
        for attacker in [required] {
            game.object_mut(attacker)
                .expect("attacker exists")
                .abilities_mut()
                .push(Ability::static_ability(StaticAbility::must_attack()));
        }

        let limiter = CardBuilder::new(CardId::new(), "One Attacker Limit")
            .card_types(vec![CardType::Enchantment])
            .build();
        let limiter = game.create_object_from_card(&limiter, bob, Zone::Battlefield);
        game.object_mut(limiter)
            .expect("limiter exists")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::max_attackers_each_combat(1),
            ));
        game.refresh_continuous_state();

        let mut trigger_queue = TriggerQueue::new();
        let optional_only = [AttackerDeclaration {
            creature: optional,
            target: AttackTarget::Player(bob),
        }];
        assert!(
            apply_attacker_declarations(
                &mut game,
                &mut CombatState::default(),
                &mut trigger_queue,
                &optional_only,
            )
            .is_err(),
            "attacking with an optional creature obeys fewer requirements than possible"
        );

        apply_attacker_declarations(
            &mut game,
            &mut CombatState::default(),
            &mut trigger_queue,
            &[AttackerDeclaration {
                creature: required,
                target: AttackTarget::Player(bob),
            }],
        )
        .expect("the declaration obeying the one satisfiable requirement should be legal");
    }

    #[test]
    fn conflicting_attack_requirements_do_not_each_become_mandatory() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first = create_attacker(&mut game, alice, "First Required Attacker", false);
        let second = create_attacker(&mut game, alice, "Second Required Attacker", false);
        for attacker in [first, second] {
            game.object_mut(attacker)
                .expect("attacker exists")
                .abilities_mut()
                .push(Ability::static_ability(StaticAbility::must_attack()));
        }

        let limiter = CardBuilder::new(CardId::new(), "One Attacker Limit")
            .card_types(vec![CardType::Enchantment])
            .build();
        let limiter = game.create_object_from_card(&limiter, bob, Zone::Battlefield);
        game.object_mut(limiter)
            .expect("limiter exists")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::max_attackers_each_combat(1),
            ));
        game.refresh_continuous_state();

        apply_attacker_declarations(
            &mut game,
            &mut CombatState::default(),
            &mut TriggerQueue::new(),
            &[AttackerDeclaration {
                creature: first,
                target: AttackTarget::Player(bob),
            }],
        )
        .expect("either one of two conflicting attack requirements should satisfy the maximum");
    }

    #[test]
    fn attack_optimizer_counts_multiple_requirements_on_one_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let two_requirements = create_attacker(&mut game, alice, "Twice Required Attacker", false);
        let one_requirement = create_attacker(&mut game, alice, "Once Required Attacker", false);
        for _ in 0..2 {
            game.object_mut(two_requirements)
                .expect("attacker exists")
                .abilities_mut()
                .push(Ability::static_ability(StaticAbility::must_attack()));
        }
        game.object_mut(one_requirement)
            .expect("attacker exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::must_attack()));

        let limiter = CardBuilder::new(CardId::new(), "One Attacker Limit")
            .card_types(vec![CardType::Enchantment])
            .build();
        let limiter = game.create_object_from_card(&limiter, bob, Zone::Battlefield);
        game.object_mut(limiter)
            .expect("limiter exists")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::max_attackers_each_combat(1),
            ));
        game.refresh_continuous_state();

        let low_score = [AttackerDeclaration {
            creature: one_requirement,
            target: AttackTarget::Player(bob),
        }];
        assert!(
            apply_attacker_declarations(
                &mut game,
                &mut CombatState::default(),
                &mut TriggerQueue::new(),
                &low_score,
            )
            .is_err(),
            "one obeyed requirement is illegal when two can be obeyed"
        );

        apply_attacker_declarations(
            &mut game,
            &mut CombatState::default(),
            &mut TriggerQueue::new(),
            &[AttackerDeclaration {
                creature: two_requirements,
                target: AttackTarget::Player(bob),
            }],
        )
        .expect("the creature obeying two requirements should be the legal attacker");
    }

    #[test]
    fn attack_requirement_does_not_force_payment_of_an_attack_cost() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let required = create_attacker(&mut game, alice, "Taxed Required Attacker", false);
        game.object_mut(required)
            .expect("attacker exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::must_attack()));

        let tax = CardBuilder::new(CardId::new(), "Attack Tax")
            .card_types(vec![CardType::Enchantment])
            .build();
        let tax = game.create_object_from_card(&tax, bob, Zone::Battlefield);
        game.object_mut(tax)
            .expect("tax exists")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::cant_attack_you_unless_controller_pays_per_attacker(1),
            ));
        game.player_mut(alice)
            .expect("attacking player exists")
            .mana_pool
            .add(crate::mana::ManaSymbol::Colorless, 1);
        game.refresh_continuous_state();

        apply_attacker_declarations(
            &mut game,
            &mut CombatState::default(),
            &mut TriggerQueue::new(),
            &[],
        )
        .expect("a must-attack requirement never forces its controller to pay an attack cost");
        assert_eq!(
            game.player(alice)
                .expect("attacking player exists")
                .mana_pool
                .total(),
            1,
            "declining to attack must not spend the available mana"
        );
    }

    #[test]
    fn vigilance_is_snapshotted_before_paying_attack_costs() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_attacker(&mut game, alice, "Costly Attacker", false);

        let artifact_card = CardBuilder::new(CardId::new(), "Vigilance Battery")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact = game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);
        let originating_ability = StaticAbility::vigilance();
        game.object_mut(artifact)
            .expect("artifact exists")
            .abilities_mut()
            .push(Ability::static_ability(originating_ability.clone()));
        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(
                artifact,
                alice,
                EffectTarget::Specific(attacker),
                Modification::AddAbility(StaticAbility::vigilance()),
            )
            .with_originating_static_ability(originating_ability),
        );
        game.object_mut(attacker)
            .expect("attacker exists")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::cant_attack_unless_condition(
                    CantAttackUnlessConditionSpec::AttackCost(
                        AttackCostCondition::SacrificePermanents {
                            filter: crate::filter::ObjectFilter::artifact(),
                            count: 1,
                        },
                    ),
                    "Can't attack unless you sacrifice an artifact",
                ),
            ));
        game.refresh_continuous_state();
        assert!(game.object_has_ability(attacker, &StaticAbility::vigilance()));

        let mut combat = CombatState::default();
        let mut trigger_queue = TriggerQueue::new();
        apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &[AttackerDeclaration {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
        )
        .expect("attack should succeed after sacrificing the artifact");

        assert!(game.object(artifact).is_none(), "zone change gets a new ID");
        assert_eq!(game.player(alice).expect("Alice exists").graveyard.len(), 1);
        assert_eq!(combat.attackers.len(), 1);
        assert!(
            !game.is_tapped(attacker),
            "CR 508.1f checks vigilance before the attack cost removes its source"
        );
    }

    #[test]
    fn competing_attack_costs_fail_without_mutating_the_game() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first_attacker = create_attacker(&mut game, alice, "First Costly Attacker", false);
        let second_attacker = create_attacker(&mut game, alice, "Second Costly Attacker", false);

        let artifact_card = CardBuilder::new(CardId::new(), "Only Sacrifice Resource")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact = game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);
        for attacker in [first_attacker, second_attacker] {
            game.object_mut(attacker)
                .expect("attacker exists")
                .abilities_mut()
                .push(Ability::static_ability(
                    StaticAbility::cant_attack_unless_condition(
                        CantAttackUnlessConditionSpec::AttackCost(
                            AttackCostCondition::SacrificePermanents {
                                filter: crate::filter::ObjectFilter::artifact(),
                                count: 1,
                            },
                        ),
                        "Can't attack unless you sacrifice an artifact",
                    ),
                ));
        }
        game.refresh_continuous_state();

        let declarations = [first_attacker, second_attacker].map(|creature| AttackerDeclaration {
            creature,
            target: AttackTarget::Player(bob),
        });
        let mut combat = CombatState::default();
        let mut trigger_queue = TriggerQueue::new();
        let pending_trigger_events_before = game.effect_store.pending_trigger_events.len();
        let graveyard_before = game.player(alice).expect("Alice exists").graveyard.len();

        let result =
            apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations);

        assert!(result.is_err(), "one artifact cannot pay two attack costs");
        assert!(
            game.object(artifact)
                .is_some_and(|object| object.zone == Zone::Battlefield),
            "the first attacker's sacrifice must roll back"
        );
        assert_eq!(
            game.player(alice).expect("Alice exists").graveyard.len(),
            graveyard_before,
            "failed declaration must not leave the sacrificed permanent in the graveyard"
        );
        assert!(!game.is_tapped(first_attacker));
        assert!(!game.is_tapped(second_attacker));
        assert!(combat.attackers.is_empty());
        assert!(game.combat.is_none());
        assert!(!game.creature_attacked_this_turn(first_attacker));
        assert!(!game.creature_attacked_this_turn(second_attacker));
        assert!(trigger_queue.entries.is_empty());
        assert_eq!(
            game.effect_store.pending_trigger_events.len(),
            pending_trigger_events_before,
            "sacrifice and tap events must not leak from a failed declaration"
        );
    }

    #[test]
    fn many_vigilant_attackers_publish_with_one_static_effect_refresh() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut declarations = Vec::new();

        for index in 0..64 {
            let attacker =
                create_attacker(&mut game, alice, &format!("Batch Attacker {index}"), true);
            declarations.push(AttackerDeclaration {
                creature: attacker,
                target: AttackTarget::Player(bob),
            });
        }
        game.refresh_continuous_state();

        let before = game.work_counters();
        let mut combat = CombatState::default();
        let mut trigger_queue = TriggerQueue::new();
        apply_attacker_declarations(&mut game, &mut combat, &mut trigger_queue, &declarations)
            .expect("batched declaration should succeed");
        let after = game.work_counters();

        assert_eq!(combat.attackers.len(), declarations.len());
        assert!(
            after.derived_view_rebuilds - before.derived_view_rebuilds <= 4,
            "legality, post-cost verification, and trigger matching should use O(1) derived views"
        );
        assert_eq!(
            after.static_ability_regens - before.static_ability_regens,
            1,
            "publishing a declaration must not regenerate static effects per attacker"
        );
    }

    #[test]
    fn invalid_blocker_declaration_does_not_clear_existing_combat_state() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_attacker(&mut game, alice, "Attacker", false);
        let blocker = create_attacker(&mut game, bob, "Lone Blocker", false);
        game.effect_store
            .cant_effects
            .cant_block_alone
            .insert(blocker);

        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        combat.blockers.insert(attacker, vec![blocker]);
        let before = combat.clone();
        let mut trigger_queue = TriggerQueue::new();

        let result = apply_blocker_declarations(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &[BlockerDeclaration {
                blocker,
                blocking: attacker,
            }],
            bob,
        );

        assert!(result.is_err());
        assert_eq!(combat.blockers, before.blockers);
        assert_eq!(
            combat.damage_assignment_order,
            before.damage_assignment_order
        );
        assert_eq!(combat.attacking_bands, before.attacking_bands);
        assert_eq!(
            combat.had_to_attack_this_combat,
            before.had_to_attack_this_combat
        );
        assert_eq!(combat.attackers.len(), before.attackers.len());
        for (actual, expected) in combat.attackers.iter().zip(&before.attackers) {
            assert_eq!(actual.creature, expected.creature);
            assert_eq!(actual.target, expected.target);
        }
        assert!(trigger_queue.entries.is_empty());
    }

    #[test]
    fn simultaneous_life_loss_events_queue_one_inherent_speed_trigger() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        assert!(game.start_engines(alice));
        let events = (0..2)
            .map(|_| {
                TriggerEvent::new_with_provenance(
                    crate::events::LifeLossEvent::new(bob, 1, false),
                    crate::provenance::ProvNodeId::default(),
                )
            })
            .collect();
        let mut trigger_queue = TriggerQueue::new();

        queue_triggers_for_simultaneous_events(&mut game, &mut trigger_queue, events);

        assert_eq!(
            trigger_queue
                .entries
                .iter()
                .filter(|entry| crate::triggers::check::is_speed_rule_trigger(entry))
                .count(),
            1
        );
        assert!(game.speed_increase_triggered_this_turn(alice));
    }
}

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

    for restriction in &game.effect_store.restriction_effects {
        if restriction.controller != defending_player
            || !restriction.is_active(game, game.turn.turn_number)
        {
            continue;
        }
        if let crate::effect::Restriction::AttackYouUnlessControllerPaysPerAttacker(
            per_attacker_tax,
        ) = &restriction.restriction
        {
            tax = tax.saturating_add(*per_attacker_tax);
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
        AttackTarget::Battle(object_id) => game.battle_protector(*object_id),
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
    optional_attack_cost_prompts: Vec<(usize, crate::decisions::context::DecisionContext)>,
    has_vigilance: bool,
    had_to_attack_this_combat: bool,
}

#[derive(Debug, Clone)]
struct PreparedAttackDeclarations {
    declarations: Vec<PreparedAttackerDeclaration>,
    total_generic_attack_mana_cost: u32,
    generic_attack_mana_costs: std::collections::HashMap<PlayerId, u32>,
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
    let mut attackers_per_controller_and_defender: HashMap<(PlayerId, PlayerId), u32> =
        HashMap::new();
    let mut generic_attack_mana_costs: HashMap<PlayerId, u32> = HashMap::new();
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
        if !game.is_active_player(game.controller_of(creature)) {
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
                let entry = generic_attack_mana_costs
                    .entry(creature_controller)
                    .or_default();
                *entry = entry.saturating_add(cost);
            }

            let optional_prompt = ability.optional_attack_cost_prompt(
                game,
                creature.id,
                creature_controller,
                &attacking_creatures,
            );
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
            *attackers_per_controller_and_defender
                .entry((creature_controller, defending_player))
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

    for ((controller, defending_player), attackers) in attackers_per_controller_and_defender {
        let per_attacker_tax =
            generic_attack_tax_per_attacker_against_player(game, defending_player, all_effects);
        let entry = generic_attack_mana_costs.entry(controller).or_default();
        *entry = entry.saturating_add(per_attacker_tax.saturating_mul(attackers));
    }

    let total_generic_attack_mana_cost = generic_attack_mana_costs
        .values()
        .copied()
        .fold(0u32, u32::saturating_add);

    Ok(PreparedAttackDeclarations {
        declarations: prepared,
        total_generic_attack_mana_cost,
        generic_attack_mana_costs,
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
) -> Vec<crate::decisions::context::DecisionContext> {
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
) -> Result<Vec<crate::decisions::context::DecisionContext>, GameLoopError> {
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

    let attacking_creatures = prepared
        .declarations
        .iter()
        .map(|prepared_decl| prepared_decl.declaration.creature)
        .collect::<Vec<_>>();

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
        for (ability_index, _) in &prepared_decl.optional_attack_cost_prompts {
            let ability = &prepared_decl.abilities[*ability_index];
            if let Some(result) = ability.pay_optional_attack_cost(
                game,
                creature_source,
                creature_controller,
                &attacking_creatures,
                trigger_queue,
                decision_maker,
            ) && let Err(msg) = result
            {
                return Err(ResponseError::InvalidAttackers(msg).into());
            }
        }
    }

    for payer in game.turn_players() {
        let amount = prepared
            .generic_attack_mana_costs
            .get(&payer)
            .copied()
            .unwrap_or(0);
        if amount == 0 {
            continue;
        }
        let tax_cost = generic_mana_cost(amount);
        if !game.can_pay_mana_cost(payer, None, &tax_cost, 0) {
            return Err(ResponseError::InvalidAttackers(format!(
                "Cannot pay required attack cost of {{{}}}",
                amount
            ))
            .into());
        }
        if !game.try_pay_mana_cost(payer, None, &tax_cost, 0) {
            return Err(ResponseError::InvalidAttackers(format!(
                "Failed to pay required attack cost of {{{}}}",
                amount
            ))
            .into());
        }
    }

    // Costs may have removed or changed control of chosen creatures. Build the
    // final combat state off to the side, then publish it once so every attack
    // trigger observes the complete declaration.
    let mut next_combat = combat.clone();
    next_combat.attackers.clear();
    next_combat.attacking_bands.clear();
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
        let attacking_players = surviving_declarations
            .iter()
            .map(|declaration| declaration.controller)
            .collect::<std::collections::HashSet<_>>();
        let history = &mut game.turn_store.turn_history;
        history.players_attacked_this_turn.extend(attacking_players);
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
            AttackTarget::Battle(oid) => AttackEventTarget::Battle(*oid),
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
    let mut decision_maker = crate::decision::AutoPassDecisionMaker;
    apply_blocker_declarations_with_dm(
        game,
        combat,
        trigger_queue,
        declarations,
        Some(defending_player),
        &mut decision_maker,
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
    let mut decision_maker = crate::decision::AutoPassDecisionMaker;
    apply_blocker_declarations_with_dm(
        game,
        combat,
        trigger_queue,
        declarations,
        None,
        &mut decision_maker,
    )
}

#[derive(Debug, Clone)]
struct LockedBlockCost {
    payer: PlayerId,
    source: ObjectId,
    cost: crate::cost::TotalCost,
    display: String,
}

fn lock_block_cost(
    game: &GameState,
    source: ObjectId,
    ability_controller: PlayerId,
    cost: crate::cost::TotalCost,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<crate::cost::TotalCost, crate::cost::CostPaymentError> {
    let mut execution_ctx =
        crate::effects::ExecutionContext::new(source, ability_controller, decision_maker);
    cost.try_map(|component| {
        let Some(dynamic_mana) = component.dynamic_mana_cost_ref() else {
            return Ok(component);
        };
        let resolved = crate::special_actions::resolve_dynamic_mana_cost(
            game,
            dynamic_mana,
            &mut execution_ctx,
        )?;
        Ok(crate::costs::Cost::mana(resolved))
    })
}

fn locked_block_costs_for_declarations(
    game: &GameState,
    pairs: &[(ObjectId, ObjectId)],
    decision_maker: &mut dyn DecisionMaker,
) -> Result<Vec<LockedBlockCost>, GameLoopError> {
    use std::collections::HashSet;

    let view = DerivedGameView::new(game);
    let mut blockers = Vec::new();
    let mut seen_blockers = HashSet::new();
    for &(blocker, _) in pairs {
        if seen_blockers.insert(blocker) {
            blockers.push(blocker);
        }
    }

    let mut locked = Vec::new();
    for &source in &game.battlefield {
        let Some(source_object) = game.object(source) else {
            continue;
        };
        let ability_controller = game.controller_of(source_object);
        let abilities = static_abilities_for_object_with_effects(game, source, view.effects());
        for ability in abilities {
            for &blocker in &blockers {
                let imposed = pairs
                    .iter()
                    .filter(|(candidate, _)| *candidate == blocker)
                    .find_map(|(_, attacker)| {
                        ability.block_cost_for_declaration(
                            game,
                            source,
                            ability_controller,
                            blocker,
                            *attacker,
                        )
                    });
                let Some(cost) = imposed else {
                    continue;
                };
                let payer = game
                    .object(blocker)
                    .map(|object| game.controller_of(object))
                    .ok_or_else(|| {
                        GameLoopError::InvalidState(format!(
                            "Blocker #{} disappeared while locking blocking costs",
                            blocker.0
                        ))
                    })?;
                let display = ability.display();
                let cost = lock_block_cost(game, source, ability_controller, cost, decision_maker)
                    .map_err(|error| {
                        ResponseError::InvalidBlockers(format!(
                            "Could not determine required blocking cost ({display}): {error}"
                        ))
                    })?;
                locked.push(LockedBlockCost {
                    payer,
                    source,
                    cost,
                    display,
                });
            }
        }
    }
    Ok(locked)
}

fn ordered_locked_block_cost(
    game: &GameState,
    locked: &LockedBlockCost,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<crate::cost::TotalCost, GameLoopError> {
    let ironsmith_core::TotalCostKind::All(components) = locked.cost.kind() else {
        return Ok(locked.cost.clone());
    };
    if components.len() < 2 {
        return Ok(locked.cost.clone());
    }

    let options = components
        .iter()
        .enumerate()
        .map(|(index, component)| {
            crate::decisions::context::SelectableOption::new(index, component.display())
        })
        .collect();
    let context = crate::decisions::context::SelectOptionsContext::new(
        locked.payer,
        Some(locked.source),
        "Choose the order to pay blocking-cost components",
        options,
        components.len(),
        components.len(),
    );
    let order = decision_maker.decide_options(game, &context);
    let unique = order
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    if order.len() != components.len()
        || unique.len() != components.len()
        || order.iter().any(|index| *index >= components.len())
    {
        return Err(ResponseError::InvalidBlockers(
            "Invalid blocking-cost payment order".to_string(),
        )
        .into());
    }
    Ok(crate::cost::TotalCost::from_costs(
        order
            .into_iter()
            .map(|index| components[index].clone())
            .collect(),
    ))
}

fn order_locked_block_costs(
    game: &GameState,
    locked_costs: Vec<LockedBlockCost>,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<Vec<LockedBlockCost>, GameLoopError> {
    let mut payer_order = Vec::new();
    for locked in &locked_costs {
        if !payer_order.contains(&locked.payer) {
            payer_order.push(locked.payer);
        }
    }

    let mut ordered = Vec::with_capacity(locked_costs.len());
    for payer in payer_order {
        let payer_costs = locked_costs
            .iter()
            .enumerate()
            .filter(|(_, locked)| locked.payer == payer)
            .collect::<Vec<_>>();
        if payer_costs.len() < 2 {
            ordered.extend(payer_costs.into_iter().map(|(_, locked)| locked.clone()));
            continue;
        }
        let options = payer_costs
            .iter()
            .enumerate()
            .map(|(option_index, (_, locked))| {
                crate::decisions::context::SelectableOption::new(
                    option_index,
                    format!("{}: {}", locked.display, locked.cost.display()),
                )
            })
            .collect();
        let context = crate::decisions::context::SelectOptionsContext::new(
            payer,
            None,
            "Choose the order to pay blocking costs",
            options,
            payer_costs.len(),
            payer_costs.len(),
        );
        let choice = decision_maker.decide_options(game, &context);
        let unique = choice
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        if choice.len() != payer_costs.len()
            || unique.len() != payer_costs.len()
            || choice.iter().any(|index| *index >= payer_costs.len())
        {
            return Err(ResponseError::InvalidBlockers(
                "Invalid blocking-cost payment order".to_string(),
            )
            .into());
        }
        ordered.extend(choice.into_iter().map(|index| payer_costs[index].1.clone()));
    }
    Ok(ordered)
}

#[derive(Debug, Clone)]
pub struct BlockDeclarationTransaction {
    game_checkpoint: GameState,
    combat_checkpoint: CombatState,
    trigger_queue_checkpoint: TriggerQueue,
    prepared: PreparedBlockerDeclarations,
}

impl BlockDeclarationTransaction {
    pub fn mana_cost_payers(&self) -> Vec<PlayerId> {
        fn requires_mana(cost: &crate::cost::TotalCost) -> bool {
            match cost.kind() {
                ironsmith_core::TotalCostKind::All(components) => {
                    components.iter().any(crate::costs::Cost::is_mana_cost)
                }
                ironsmith_core::TotalCostKind::OneOf(branches) => {
                    branches.iter().any(requires_mana)
                }
            }
        }

        let mut payers = Vec::new();
        for locked in &self.prepared.locked_costs {
            if requires_mana(&locked.cost) && !payers.contains(&locked.payer) {
                payers.push(locked.payer);
            }
        }
        payers
    }

    pub fn declaration_source(&self) -> Option<ObjectId> {
        self.prepared
            .locked_costs
            .first()
            .map(|locked| locked.source)
            .or_else(|| self.prepared.pairs.first().map(|(blocker, _)| *blocker))
    }

    pub fn defending_player(&self) -> Option<PlayerId> {
        self.prepared.defending_player
    }

    /// Make the complete proposed declaration visible to filters and mana
    /// abilities used during CR 509.1e-f without publishing triggers or
    /// mutating the caller's authoritative `CombatState`. The transaction's
    /// checkpoint restores this view if payment fails.
    pub fn stage_proposed_combat_for_payment(&self, game: &mut GameState) {
        game.combat = Some(self.prepared.next_combat.clone());
        game.mark_continuous_state_dirty();
        game.refresh_continuous_state();
    }
}

pub fn begin_blocker_declaration_transaction(
    game: &GameState,
    combat: &CombatState,
    trigger_queue: &TriggerQueue,
    declarations: &[BlockerDeclaration],
    defending_player: PlayerId,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<BlockDeclarationTransaction, GameLoopError> {
    let prepared = prepare_blocker_declarations(
        game,
        combat,
        declarations,
        Some(defending_player),
        decision_maker,
    )?;
    Ok(BlockDeclarationTransaction {
        game_checkpoint: game.clone(),
        combat_checkpoint: combat.clone(),
        trigger_queue_checkpoint: trigger_queue.clone(),
        prepared,
    })
}

pub fn begin_multiplayer_blocker_declaration_transaction(
    game: &GameState,
    combat: &CombatState,
    trigger_queue: &TriggerQueue,
    declarations: &[BlockerDeclaration],
    decision_maker: &mut dyn DecisionMaker,
) -> Result<BlockDeclarationTransaction, GameLoopError> {
    let prepared = prepare_blocker_declarations(game, combat, declarations, None, decision_maker)?;
    Ok(BlockDeclarationTransaction {
        game_checkpoint: game.clone(),
        combat_checkpoint: combat.clone(),
        trigger_queue_checkpoint: trigger_queue.clone(),
        prepared,
    })
}

pub fn finish_blocker_declaration_transaction(
    transaction: BlockDeclarationTransaction,
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), GameLoopError> {
    let BlockDeclarationTransaction {
        game_checkpoint,
        combat_checkpoint,
        trigger_queue_checkpoint,
        prepared,
    } = transaction;
    let result =
        apply_prepared_blocker_declarations(game, combat, trigger_queue, prepared, decision_maker);
    if result.is_err() {
        *game = game_checkpoint;
        *combat = combat_checkpoint;
        *trigger_queue = trigger_queue_checkpoint;
    }
    result
}

fn apply_blocker_declarations_with_dm(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    declarations: &[BlockerDeclaration],
    expected_defending_player: Option<PlayerId>,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), GameLoopError> {
    let game_checkpoint = game.clone();
    let combat_checkpoint = combat.clone();
    let trigger_queue_checkpoint = trigger_queue.clone();
    let result = apply_blocker_declarations_internal(
        game,
        combat,
        trigger_queue,
        declarations,
        expected_defending_player,
        decision_maker,
    );
    if result.is_err() {
        *game = game_checkpoint;
        *combat = combat_checkpoint;
        *trigger_queue = trigger_queue_checkpoint;
    }
    result
}

fn apply_blocker_declarations_internal(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    declarations: &[BlockerDeclaration],
    expected_defending_player: Option<PlayerId>,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), GameLoopError> {
    let prepared = prepare_blocker_declarations(
        game,
        combat,
        declarations,
        expected_defending_player,
        decision_maker,
    )?;
    apply_prepared_blocker_declarations(game, combat, trigger_queue, prepared, decision_maker)
}

#[derive(Debug, Clone)]
struct PreparedBlockerDeclarations {
    pairs: Vec<(ObjectId, ObjectId)>,
    next_combat: CombatState,
    locked_costs: Vec<LockedBlockCost>,
    defending_player: Option<PlayerId>,
}

fn prepare_blocker_declarations(
    game: &GameState,
    combat: &CombatState,
    declarations: &[BlockerDeclaration],
    expected_defending_player: Option<PlayerId>,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<PreparedBlockerDeclarations, GameLoopError> {
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
        if expected_defending_player.is_some_and(|player| {
            player != blocker_controller
                && !(game.shared_team_turns_enabled()
                    && game.are_teammates(player, blocker_controller))
        }) {
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
            .is_none_or(|defender| {
                defender != blocker_controller
                    && !(game.shared_team_turns_enabled()
                        && game.are_teammates(defender, blocker_controller))
            })
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
    let validation_pairs = if let Some(defending_player) = expected_defending_player {
        let mut combined = combat
            .blockers
            .iter()
            .flat_map(|(&attacker, blockers)| {
                blockers
                    .iter()
                    .copied()
                    .map(move |blocker| (blocker, attacker))
            })
            .filter(|(blocker, _)| {
                game.object(*blocker)
                    .is_some_and(|object| game.controller_of(object) != defending_player)
            })
            .collect::<Vec<_>>();
        combined.extend(pairs.iter().copied());
        combined
    } else {
        pairs.clone()
    };
    let mut next_combat = combat.clone();
    next_combat.blockers.clear();
    let validation = if game.shared_team_turns_enabled() {
        crate::combat_state::declare_blockers(game, &mut next_combat, validation_pairs)
    } else if let Some(defending_player) = expected_defending_player {
        crate::combat_state::declare_blockers_for_defending_player(
            game,
            &mut next_combat,
            validation_pairs,
            defending_player,
        )
    } else {
        crate::combat_state::declare_blockers(game, &mut next_combat, validation_pairs)
    };
    if let Err(err) = validation {
        return Err(ResponseError::InvalidBlockers(err.to_string()).into());
    }

    // CR 509.1d: determine and lock every cost only after the complete proposed
    // declaration is legal. A single ability charges a blocking creature once,
    // even if that creature is blocking more than one attacker.
    let mut declaration_view = game.clone();
    declaration_view.combat = Some(next_combat.clone());
    declaration_view.mark_continuous_state_dirty();
    declaration_view.refresh_continuous_state();
    let locked_costs =
        locked_block_costs_for_declarations(&declaration_view, &pairs, decision_maker)?;
    Ok(PreparedBlockerDeclarations {
        pairs,
        next_combat,
        locked_costs,
        defending_player: expected_defending_player,
    })
}

fn apply_prepared_blocker_declarations(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    prepared: PreparedBlockerDeclarations,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), GameLoopError> {
    let PreparedBlockerDeclarations {
        pairs,
        next_combat,
        locked_costs,
        defending_player,
    } = prepared;
    game.combat = Some(next_combat.clone());
    game.mark_continuous_state_dirty();
    game.refresh_continuous_state();
    for locked in order_locked_block_costs(game, locked_costs, decision_maker)? {
        let ordered = ordered_locked_block_cost(game, &locked, decision_maker)?;
        crate::special_actions::pay_total_cost_with_choice(
            game,
            locked.payer,
            locked.source,
            &ordered,
            crate::costs::PaymentReason::Other,
            decision_maker,
        )
        .map_err(|error| {
            ResponseError::InvalidBlockers(format!(
                "Cannot pay required blocking cost ({}): {error}",
                locked.display
            ))
        })?;
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
        if defending_player.is_some_and(|player| {
            crate::combat_state::defending_player_for_attacker(game, combat, attacker)
                != Some(player)
        }) {
            continue;
        }
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
        if defending_player.is_some_and(|player| {
            crate::combat_state::defending_player_for_attacker(game, combat, attacker_id)
                != Some(player)
        }) {
            continue;
        }
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
                AttackTarget::Battle(battle_id) => {
                    crate::triggers::AttackEventTarget::Battle(*battle_id)
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
        if defending_player.is_some_and(|player| {
            crate::combat_state::defending_player_for_attacker(game, combat, info.creature)
                != Some(player)
        }) {
            continue;
        }
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
            AttackTarget::Battle(battle_id) => {
                crate::triggers::AttackEventTarget::Battle(battle_id)
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
    use crate::cards::CardDefinitionBuilder;
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::decisions::context::DecisionContext;
    use crate::effect::Until;
    use crate::filter::ObjectFilterExt as _;
    use crate::ids::CardId;
    use crate::static_abilities::{
        AttackCostCondition, CantAttackUnlessConditionSpec, StaticAbility,
    };
    use crate::target::PlayerFilter;

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

    fn create_enlist_attacker(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        instances: usize,
    ) -> ObjectId {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2));
        for _ in 0..instances {
            builder = builder.enlist();
        }
        let definition = builder.build();
        let id = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
        game.remove_summoning_sickness(id);
        id
    }

    struct EnlistChoices {
        choices: Vec<Vec<ObjectId>>,
        next: usize,
    }

    impl DecisionMaker for EnlistChoices {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let choice = self.choices.get(self.next).cloned().unwrap_or_default();
            self.next += 1;
            choice
        }
    }

    #[test]
    fn enlist_pays_during_declaration_and_each_instance_triggers_once() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_enlist_attacker(&mut game, alice, "Double Enlister", 2);
        let first_support = create_attacker(&mut game, alice, "First Support", false);
        let second_support = create_attacker(&mut game, alice, "Second Support", false);
        game.refresh_continuous_state();

        let declarations = [AttackerDeclaration {
            creature: attacker,
            target: AttackTarget::Player(bob),
        }];
        let mut combat = CombatState::default();
        let mut trigger_queue = TriggerQueue::new();
        let mut decisions = EnlistChoices {
            choices: vec![vec![first_support], vec![second_support]],
            next: 0,
        };
        apply_attacker_declarations_with_dm(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &declarations,
            &mut decisions,
        )
        .expect("both independent enlist costs should be payable");

        assert!(game.is_tapped(attacker), "508.1f taps the attacker first");
        assert!(game.is_tapped(first_support));
        assert!(game.is_tapped(second_support));
        assert_eq!(
            game.calculated_power(attacker),
            Some(2),
            "the linked boosts do not resolve during attack-cost payment"
        );
        assert_eq!(trigger_queue.entries.len(), 2);

        let enlist_events = game
            .turn_store
            .turn_history
            .projected_records()
            .filter_map(|record| record.event.downcast::<crate::events::KeywordActionEvent>())
            .filter(|event| event.action == crate::events::KeywordActionKind::Enlist)
            .collect::<Vec<_>>();
        assert_eq!(enlist_events.len(), 2, "each paid instance enlists once");
        assert!(enlist_events.iter().all(|event| {
            event.source == attacker
                && event.combat_phase == Some(0)
                && event
                    .object_tags
                    .get(&crate::tag::TagKey::from("enlisted_creature"))
                    .is_some_and(|objects| objects.len() == 1)
        }));

        let enlisted_this_combat = crate::effect::Condition::TurnHistory(
            ironsmith_core::TurnHistoryCondition::TriggeringObjectEnlistedThisCombat,
        );
        let attack_event = TriggerEvent::new(
            crate::events::CreatureAttackedEvent::new(
                attacker,
                crate::triggers::AttackEventTarget::Player(bob),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let attack_ctx = crate::effects::ExecutionContext::new_default(attacker, alice)
            .with_triggering_event(attack_event);
        assert!(
            crate::condition_eval::evaluate_condition_resolution(
                &game,
                &enlisted_this_combat,
                &attack_ctx,
            )
            .expect("enlist history predicate should evaluate")
        );

        let other_attack_event = TriggerEvent::new(
            crate::events::CreatureAttackedEvent::new(
                first_support,
                crate::triggers::AttackEventTarget::Player(bob),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let other_attack_ctx = crate::effects::ExecutionContext::new_default(first_support, alice)
            .with_triggering_event(other_attack_event);
        assert!(
            !crate::condition_eval::evaluate_condition_resolution(
                &game,
                &enlisted_this_combat,
                &other_attack_ctx,
            )
            .expect("another attacker should not inherit enlist history")
        );
        game.turn_store.combat_phases_started_this_turn = 1;
        assert!(
            !crate::condition_eval::evaluate_condition_resolution(
                &game,
                &enlisted_this_combat,
                &attack_ctx,
            )
            .expect("enlist history should be scoped to one combat")
        );
        game.turn_store.combat_phases_started_this_turn = 0;

        crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("linked enlist triggers should go on the stack");
        crate::game_loop::resolve_stack_entry(&mut game)
            .expect("first enlist trigger should resolve");
        crate::game_loop::resolve_stack_entry(&mut game)
            .expect("second enlist trigger should resolve");
        assert_eq!(game.calculated_power(attacker), Some(6));
    }

    #[test]
    fn enlist_prompt_excludes_attackers_self_and_summoning_sick_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_enlist_attacker(&mut game, alice, "Enlister", 1);
        let other_attacker = create_attacker(&mut game, alice, "Other Attacker", false);
        let ready_support = create_attacker(&mut game, alice, "Ready Support", false);
        let sick_card = CardBuilder::new(CardId::new(), "New Support")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let sick_support = game.create_object_from_card(&sick_card, alice, Zone::Battlefield);
        game.set_summoning_sick(sick_support);
        let haste_card = CardBuilder::new(CardId::new(), "Hasty New Support")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let haste_support = game.create_object_from_card(&haste_card, alice, Zone::Battlefield);
        game.object_mut(haste_support)
            .expect("hasty support exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::haste()));
        game.set_summoning_sick(haste_support);
        game.refresh_continuous_state();

        let declarations = [
            AttackerDeclaration {
                creature: attacker,
                target: AttackTarget::Player(bob),
            },
            AttackerDeclaration {
                creature: other_attacker,
                target: AttackTarget::Player(bob),
            },
        ];
        let prompts =
            preview_optional_attack_cost_prompts(&game, &CombatState::default(), &declarations)
                .expect("declaration should be legal");
        let DecisionContext::SelectObjects(prompt) = &prompts[0] else {
            panic!("enlist should request an object selection")
        };
        let candidates = prompt
            .candidates
            .iter()
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();
        assert_eq!(candidates, vec![ready_support, haste_support]);
        assert!(!candidates.contains(&attacker));
        assert!(!candidates.contains(&other_attacker));
        assert!(!candidates.contains(&sick_support));
        assert_eq!(prompt.min, 0, "declining enlist remains legal");
        assert_eq!(prompt.max, Some(1));
    }

    #[test]
    fn enlist_keyword_action_fires_source_enlists_trigger() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Enlist Trigger Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .enlist()
            .with_ability(Ability::triggered(
                crate::triggers::Trigger::keyword_action_from_source(
                    crate::events::KeywordActionKind::Enlist,
                    crate::target::PlayerFilter::You,
                ),
                vec![crate::effect::Effect::scry(2)],
            ))
            .build();
        let attacker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker);
        let support = create_attacker(&mut game, alice, "Support", false);
        game.refresh_continuous_state();

        let mut combat = CombatState::default();
        let mut trigger_queue = TriggerQueue::new();
        let mut decisions = EnlistChoices {
            choices: vec![vec![support]],
            next: 0,
        };
        apply_attacker_declarations_with_dm(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &[AttackerDeclaration {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
            &mut decisions,
        )
        .expect("enlist declaration should succeed");
        assert_eq!(
            trigger_queue.entries.len(),
            1,
            "linked boost is queued directly"
        );

        crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);
        assert_eq!(
            trigger_queue.entries.len(),
            2,
            "the ordinary source-enlists trigger should also fire"
        );
        assert!(
            trigger_queue
                .entries
                .iter()
                .any(|entry| { format!("{:?}", entry.ability.effects).contains("ScryEffect") })
        );
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
    fn resolved_attack_tax_survives_its_source_until_the_controllers_next_turn() {
        use crate::effects::{EffectExecutor as _, ExecutionContext};

        let mut game = setup_game();
        let bob = PlayerId::from_index(1);
        let source_card = CardBuilder::new(CardId::new(), "Temporary Attack Tax")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&source_card, bob, Zone::Battlefield);
        let tax = crate::effects::CantEffect::new(
            crate::effect::Restriction::attack_you_unless_controller_pays_per_attacker(2),
            Until::YourNextTurn,
        );
        tax.execute(&mut game, &mut ExecutionContext::new_default(source, bob))
            .expect("the temporary attack tax resolves");

        game.move_object_by_effect(source, Zone::Graveyard)
            .expect("the source can leave after its trigger resolves");
        assert_eq!(
            generic_attack_tax_per_attacker_against_player(&game, bob, &[]),
            2,
            "the resolving restriction is independent of its source remaining on the battlefield"
        );

        game.turn.turn_number = game.turn.turn_number.saturating_add(1);
        game.turn.active_player = bob;
        assert_eq!(
            generic_attack_tax_per_attacker_against_player(&game, bob, &[]),
            0,
            "the tax expires as the protected player's next turn begins"
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
    fn blocker_declaration_batches_flanking_per_attacker_blocker_relationship() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_attacker(&mut game, alice, "Double Flanker", false);
        let first_blocker = create_attacker(&mut game, bob, "First Blocker", false);
        let second_blocker = create_attacker(&mut game, bob, "Second Blocker", false);
        for _ in 0..2 {
            game.object_mut(attacker)
                .expect("attacker exists")
                .abilities_mut()
                .push(Ability::static_ability(StaticAbility::flanking()));
        }
        game.refresh_continuous_state();

        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        let declarations = [
            BlockerDeclaration {
                blocker: first_blocker,
                blocking: attacker,
            },
            BlockerDeclaration {
                blocker: second_blocker,
                blocking: attacker,
            },
        ];
        let mut trigger_queue = TriggerQueue::new();

        apply_blocker_declarations(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &declarations,
            bob,
        )
        .expect("both blockers should be declared in one turn-based action");

        assert_eq!(
            trigger_queue.entries.len(),
            4,
            "two Flanking instances trigger for each of two blocking relationships"
        );
        for blocker in [first_blocker, second_blocker] {
            assert_eq!(
                trigger_queue
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry
                            .triggering_event
                            .downcast::<CreatureBlockedEvent>()
                            .is_some_and(|event| {
                                event.attacker == attacker && event.blocker == blocker
                            })
                    })
                    .count(),
                2,
                "each relationship receives both independent triggers"
            );
        }
    }

    #[test]
    fn blocker_declaration_requires_and_transactionally_pays_locked_cost() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_attacker(&mut game, alice, "Taxed Attacker", false);
        let blocker = create_attacker(&mut game, bob, "Paying Blocker", false);
        let tax_source = CardBuilder::new(CardId::new(), "Blocking Tax")
            .card_types(vec![CardType::Enchantment])
            .build();
        let tax_source = game.create_object_from_card(&tax_source, alice, Zone::Battlefield);
        game.object_mut(tax_source)
            .expect("tax source exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::block_cost(
                ObjectFilter::default(),
                ObjectFilter::default(),
                crate::cost::TotalCost::mana(generic_mana_cost(1)),
                "Creatures can't block unless their controller pays {1} for each blocking creature they control.",
            )));
        game.refresh_continuous_state();

        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        let declarations = [BlockerDeclaration {
            blocker,
            blocking: attacker,
        }];
        let mut trigger_queue = TriggerQueue::new();

        let unpaid = apply_blocker_declarations(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &declarations,
            bob,
        );
        assert!(
            unpaid.is_err(),
            "an unpaid blocking cost must reject the declaration"
        );
        assert!(
            combat.blockers.is_empty(),
            "failed payment must not publish blockers"
        );
        assert!(
            game.combat.is_none(),
            "failed payment must roll back the proposed declaration view"
        );
        assert!(trigger_queue.entries.is_empty());

        game.player_mut(bob)
            .expect("defending player exists")
            .mana_pool
            .add(crate::mana::ManaSymbol::Colorless, 1);
        apply_blocker_declarations(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &declarations,
            bob,
        )
        .expect("the declaration should succeed after its locked cost is paid");
        assert_eq!(combat.blockers.get(&attacker), Some(&vec![blocker]));
        assert_eq!(
            game.player(bob)
                .expect("defending player exists")
                .mana_pool
                .total(),
            0,
            "the blocking cost must be paid before blockers are published"
        );
    }

    #[test]
    fn tap_block_cost_cannot_choose_a_creature_in_the_proposed_declaration() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_attacker(&mut game, alice, "Attacker", false);
        let blocker = create_attacker(&mut game, bob, "Hollow Warrior", false);
        let payment_helper = create_attacker(&mut game, bob, "Payment Helper", false);

        let mut eligible = ObjectFilter::creature().you_control();
        eligible.untapped = true;
        eligible.nonattacking = true;
        eligible.nonblocking = true;
        let tag = crate::tag::TagKey::from("hollow_warrior_tap_cost");
        let cost = crate::cost::TotalCost::from_costs(vec![
            crate::costs::Cost::validated_effect(crate::effect::Effect::choose_objects(
                eligible,
                crate::effect::ChoiceCount::exactly(1),
                PlayerFilter::You,
                tag.clone(),
            )),
            crate::costs::Cost::validated_effect(crate::effect::Effect::tap(
                crate::target::ChooseSpec::tagged(tag),
            )),
        ]);
        game.object_mut(blocker)
            .expect("blocker exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::block_cost(
                ObjectFilter::source(),
                ObjectFilter::creature(),
                cost,
                "This creature can't block unless you tap an eligible creature",
            )));
        game.refresh_continuous_state();

        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        apply_blocker_declarations(
            &mut game,
            &mut combat,
            &mut TriggerQueue::new(),
            &[BlockerDeclaration {
                blocker,
                blocking: attacker,
            }],
            bob,
        )
        .expect("helper creature should pay the blocking cost");

        assert!(game.is_tapped(payment_helper));
        assert!(
            !game.is_tapped(blocker),
            "the creature being declared as a blocker is not an eligible tap payment"
        );
        assert_eq!(combat.blockers.get(&attacker), Some(&vec![blocker]));
    }

    #[test]
    fn one_block_cost_ability_charges_a_multi_blocker_once() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first_attacker = create_attacker(&mut game, alice, "First Attacker", false);
        let second_attacker = create_attacker(&mut game, alice, "Second Attacker", false);
        let blocker = create_attacker(&mut game, bob, "Wide Blocker", false);
        game.object_mut(blocker)
            .expect("blocker exists")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::can_block_additional_creature_each_combat(1),
            ));
        let tax_source = CardBuilder::new(CardId::new(), "Blocking Tax")
            .card_types(vec![CardType::Enchantment])
            .build();
        let tax_source = game.create_object_from_card(&tax_source, alice, Zone::Battlefield);
        game.object_mut(tax_source)
            .expect("tax source exists")
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::block_cost(
                ObjectFilter::default(),
                ObjectFilter::default(),
                crate::cost::TotalCost::mana(generic_mana_cost(1)),
                "Creatures can't block unless their controller pays {1} for each blocking creature they control.",
            )));
        game.player_mut(bob)
            .expect("defending player exists")
            .mana_pool
            .add(crate::mana::ManaSymbol::Colorless, 1);
        game.refresh_continuous_state();

        let mut combat = CombatState::default();
        for attacker in [first_attacker, second_attacker] {
            combat.attackers.push(crate::combat_state::AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            });
        }
        let declarations = [
            BlockerDeclaration {
                blocker,
                blocking: first_attacker,
            },
            BlockerDeclaration {
                blocker,
                blocking: second_attacker,
            },
        ];
        let mut trigger_queue = TriggerQueue::new();

        apply_blocker_declarations(
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &declarations,
            bob,
        )
        .expect("one mana pays once for the one blocking creature");
        assert_eq!(
            game.player(bob)
                .expect("defending player exists")
                .mana_pool
                .total(),
            0
        );
        assert_eq!(combat.blockers.get(&first_attacker), Some(&vec![blocker]));
        assert_eq!(combat.blockers.get(&second_attacker), Some(&vec![blocker]));
    }

    #[test]
    fn dynamic_block_cost_is_locked_before_payment_state_changes() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_attacker(&mut game, alice, "Dynamic Tax Source", false);
        for name in ["First Hand Card", "Second Hand Card"] {
            let card = CardBuilder::new(CardId::new(), name).build();
            game.create_object_from_card(&card, alice, Zone::Hand);
        }
        let dynamic = ironsmith_core::DynamicManaCost::generic_equal_to(
            crate::effect::Value::CardsInHand(PlayerFilter::You),
        );
        let cost = crate::cost::TotalCost::from_cost(crate::costs::Cost::dynamic_mana(dynamic));
        let mut decision_maker = crate::decision::AutoPassDecisionMaker;

        let locked = lock_block_cost(&game, source, alice, cost, &mut decision_maker)
            .expect("the declaration-time value should resolve");
        assert_eq!(
            locked
                .mana_cost()
                .expect("locked fixed mana cost")
                .generic_mana_total(),
            2
        );
        assert!(locked.dynamic_mana_cost().is_none());
    }

    #[test]
    fn attached_block_cost_matches_attachment_but_keeps_aura_as_cost_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_attacker(&mut game, alice, "Attacker", false);
        let blocker = create_attacker(&mut game, bob, "Enchanted Blocker", false);
        let aura = CardBuilder::new(CardId::new(), "Blocking Aura")
            .card_types(vec![CardType::Enchantment])
            .build();
        let aura = game.create_object_from_card(&aura, alice, Zone::Battlefield);
        let aura_object = game.object_mut(aura).expect("aura exists");
        aura_object.attached_to = Some(crate::object::AttachmentTarget::Object(blocker));
        aura_object.abilities_mut().push(Ability::static_ability(
            StaticAbility::attached_block_cost(
                ObjectFilter::creature(),
                ObjectFilter::creature(),
                crate::cost::TotalCost::mana(generic_mana_cost(1)),
                "Enchanted creature can't block unless its controller pays {1}.",
            ),
        ));
        game.player_mut(bob)
            .expect("defending player exists")
            .mana_pool
            .add(crate::mana::ManaSymbol::Colorless, 1);
        game.refresh_continuous_state();

        let mut combat = CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        let mut trigger_queue = TriggerQueue::new();
        let mut decision_maker = crate::decision::AutoPassDecisionMaker;
        let transaction = begin_blocker_declaration_transaction(
            &game,
            &combat,
            &trigger_queue,
            &[BlockerDeclaration {
                blocker,
                blocking: attacker,
            }],
            bob,
            &mut decision_maker,
        )
        .expect("the attached blocking cost should lock");
        assert_eq!(transaction.mana_cost_payers(), vec![bob]);
        assert_eq!(transaction.declaration_source(), Some(aura));
        finish_blocker_declaration_transaction(
            transaction,
            &mut game,
            &mut combat,
            &mut trigger_queue,
            &mut decision_maker,
        )
        .expect("the attached creature's controller should pay the Aura's locked cost");
        assert_eq!(
            game.player(bob)
                .expect("defending player exists")
                .mana_pool
                .total(),
            0
        );
    }

    #[test]
    fn blocking_cost_components_follow_the_payers_selected_order() {
        struct ReverseOrder;

        impl DecisionMaker for ReverseOrder {
            fn decide_options(
                &mut self,
                _game: &GameState,
                context: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                (0..context.options.len()).rev().collect()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_attacker(&mut game, alice, "Ordered Cost Source", false);
        let locked = LockedBlockCost {
            payer: bob,
            source,
            cost: crate::cost::TotalCost::from_costs(vec![
                crate::costs::Cost::mana(generic_mana_cost(1)),
                crate::costs::Cost::life(1),
            ]),
            display: "Pay {1} and 1 life to block".to_string(),
        };
        let mut decision_maker = ReverseOrder;

        let ordered = ordered_locked_block_cost(&game, &locked, &mut decision_maker)
            .expect("a complete permutation should be accepted");
        let components = ordered
            .as_all()
            .expect("the conjunction remains a conjunction");
        assert_eq!(components[0].life_amount(), Some(1));
        assert!(components[1].is_mana_cost());
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

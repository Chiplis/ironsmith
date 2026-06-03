//! Runtime orchestration for `ChooseModeEffect`.

use crate::ability::AbilityKind;
use crate::decisions::{ModesSpec, make_decision, specs::ModeOption};
use crate::effect::{EffectMode, EffectOutcome, ExecutionFact};
use crate::effects::helpers::resolve_value;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect, rebase_target_scope};
use crate::game_state::GameState;
use crate::game_state::TargetAssignment;
use crate::ids::{ObjectId, PlayerId};
use crate::targeting::compute_legal_targets;

use super::choose_mode::ChooseModeEffect;

fn check_mode_legal(
    game: &GameState,
    mode: &EffectMode,
    controller: PlayerId,
    source: ObjectId,
) -> bool {
    for effect in &mode.effects {
        if let Some(profile) = effect.target_selection_profile() {
            if !crate::game_loop::requires_target_selection(profile.spec) {
                continue;
            }
            let legal_targets = compute_legal_targets(game, profile.spec, controller, Some(source));
            // If effect requires targets (min > 0) and none exist, mode is illegal.
            if legal_targets.len() < profile.min_targets {
                return false;
            }
        }
    }
    true
}

fn related_object_ids_for_mode(
    game: &GameState,
    mode: &EffectMode,
    ctx: &ExecutionContext,
) -> Option<Vec<ObjectId>> {
    let mut saw_preview = false;
    let mut ids = Vec::new();

    for effect in &mode.effects {
        let Some(mut effect_ids) = effect.0.related_object_ids_for_decision(game, ctx) else {
            continue;
        };
        saw_preview = true;
        ids.append(&mut effect_ids);
    }

    if !saw_preview {
        return None;
    }

    ids.sort();
    ids.dedup();
    Some(ids)
}

fn find_source_activated_ability_index(
    game: &GameState,
    source: ObjectId,
    choose_mode: &ChooseModeEffect,
) -> Option<usize> {
    let source_object = game.object(source)?;
    let mut exact_indices = Vec::new();
    let mut fallback_indices = Vec::new();

    for (idx, ability) in source_object.abilities.iter().enumerate() {
        let AbilityKind::Activated(activated) = &ability.kind else {
            continue;
        };

        let mut has_disallow_choose_mode = false;
        let mut has_exact_choose_mode = false;
        for effect in &activated.effects {
            if let Some(candidate) = effect.downcast_ref::<ChooseModeEffect>() {
                if candidate.disallow_previously_chosen_modes {
                    has_disallow_choose_mode = true;
                }
                if candidate == choose_mode {
                    has_exact_choose_mode = true;
                }
            }
        }

        if has_exact_choose_mode {
            exact_indices.push(idx);
        }
        if has_disallow_choose_mode {
            fallback_indices.push(idx);
        }
    }

    if exact_indices.len() == 1 {
        return exact_indices.first().copied();
    }
    if exact_indices.is_empty() && fallback_indices.len() == 1 {
        return fallback_indices.first().copied();
    }
    None
}

fn mode_point_cost(effect: &ChooseModeEffect, mode_idx: usize) -> usize {
    effect
        .mode_point_costs
        .get(mode_idx)
        .copied()
        .unwrap_or(1)
        .max(1) as usize
}

fn selected_mode_point_total(effect: &ChooseModeEffect, mode_indices: &[usize]) -> usize {
    mode_indices
        .iter()
        .map(|idx| mode_point_cost(effect, *idx))
        .sum()
}

fn active_target_assignments_for_inner_effect(
    game: &GameState,
    effect: &crate::effect::Effect,
    ctx: &ExecutionContext,
    consumed_modal_selection: &mut bool,
    assignments: &[TargetAssignment],
    cursor: &mut usize,
) -> Vec<TargetAssignment> {
    let requirements = crate::game_loop::extract_target_requirements_for_effect_with_state(
        game,
        effect,
        ctx.controller,
        Some(ctx.source),
        ctx.chosen_modes.as_deref(),
        consumed_modal_selection,
    );
    let count = requirements.len();
    let start = *cursor;
    let end = start.saturating_add(count).min(assignments.len());
    *cursor = end;
    assignments[start..end].to_vec()
}

pub(crate) fn run_choose_mode(
    effect: &ChooseModeEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    let mut max_modes = resolve_value(game, &effect.choose_count, ctx)?.max(0) as usize;
    let mut min_modes = resolve_value(game, &effect.min_choose_count, ctx)?.max(0) as usize;
    if ctx.optional_costs_paid.was_entwined() {
        max_modes = effect.modes.len();
        min_modes = effect.modes.len();
    }

    if effect.modes.is_empty() || max_modes == 0 {
        return Ok(EffectOutcome::resolved());
    }

    let source_ability_index = if effect.disallow_previously_chosen_modes {
        find_source_activated_ability_index(game, ctx.source, effect)
    } else {
        None
    };
    let is_mode_available = |mode_idx: usize| {
        mode_idx < effect.modes.len()
            && !source_ability_index.is_some_and(|ability_index| {
                game.ability_mode_was_chosen(
                    ctx.source,
                    ability_index,
                    mode_idx,
                    effect.disallow_previously_chosen_modes_this_turn,
                )
            })
    };
    let is_mode_legal = |mode_idx: usize| {
        is_mode_available(mode_idx)
            && effect
                .modes
                .get(mode_idx)
                .is_some_and(|mode| check_mode_legal(game, mode, ctx.controller, ctx.source))
    };

    // Per MTG rule 601.2b, modes are chosen during casting (before targets).
    // Check if modes were pre-chosen during the casting process.
    let chosen_indices: Vec<usize> = if let Some(ref pre_chosen) = ctx.chosen_modes {
        pre_chosen.clone()
    } else if effect.random {
        let mut randomized_modes: Vec<usize> = (0..effect.modes.len())
            .filter(|idx| is_mode_legal(*idx))
            .collect();
        let legal_mode_count = randomized_modes.len();
        if legal_mode_count < min_modes {
            return Err(ExecutionError::Impossible(
                "Not enough legal modes available".to_string(),
            ));
        }
        game.shuffle_slice(&mut randomized_modes);
        let mut selected = Vec::new();
        let mut point_total = 0usize;
        for idx in randomized_modes {
            let point_cost = mode_point_cost(effect, idx);
            if point_total.saturating_add(point_cost) > max_modes {
                continue;
            }
            selected.push(idx);
            point_total += point_cost;
            if point_total >= min_modes {
                break;
            }
        }
        selected
    } else {
        let mode_options: Vec<ModeOption> = effect
            .modes
            .iter()
            .enumerate()
            .map(|(i, mode)| {
                let option =
                    ModeOption::with_legality(i, mode.description.clone(), is_mode_legal(i));
                if let Some(object_ids) = related_object_ids_for_mode(game, mode, ctx) {
                    option.with_related_objects(object_ids)
                } else {
                    option
                }
            })
            .collect();

        let legal_mode_count = mode_options.iter().filter(|m| m.legal).count();
        if legal_mode_count < min_modes {
            return Err(ExecutionError::Impossible(
                "Not enough legal modes available".to_string(),
            ));
        }

        let spec = ModesSpec::new(
            ctx.source,
            mode_options,
            min_modes,
            max_modes,
            effect.allow_repeated_modes,
            effect.mode_point_costs.clone(),
        );
        make_decision(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            spec,
        )
    };
    if ctx.decision_maker.awaiting_choice() {
        return Ok(EffectOutcome::count(0));
    }

    // Validate selected mode indices while preserving selection order.
    let mut valid_chosen_indices: Vec<usize> = Vec::new();
    let mut chosen_point_total = 0usize;
    for idx in chosen_indices {
        if !is_mode_legal(idx) {
            return Err(ExecutionError::Impossible(
                "Selected mode is not legal".to_string(),
            ));
        }
        if !effect.allow_repeated_modes && valid_chosen_indices.contains(&idx) {
            return Err(ExecutionError::Impossible(
                "Selected mode cannot be repeated".to_string(),
            ));
        }
        let point_cost = mode_point_cost(effect, idx);
        if chosen_point_total.saturating_add(point_cost) > max_modes {
            return Err(ExecutionError::Impossible(
                "Selected modes exceed the modal point limit".to_string(),
            ));
        }
        valid_chosen_indices.push(idx);
        chosen_point_total += point_cost;
    }

    if selected_mode_point_total(effect, &valid_chosen_indices) < min_modes {
        return Err(ExecutionError::Impossible(
            "Not enough legal modes available".to_string(),
        ));
    }

    if let Some(ability_index) = source_ability_index {
        for &mode_idx in &valid_chosen_indices {
            game.record_ability_mode_choice(
                ctx.source,
                ability_index,
                mode_idx,
                effect.disallow_previously_chosen_modes_this_turn,
            );
        }
    }

    let mut outcomes = Vec::new();
    let available_assignments = ctx.target_assignments.clone();
    let mut assignment_cursor = 0usize;
    let mut consumed_modal_selection = false;
    for &idx in &valid_chosen_indices {
        if let Some(mode) = effect.modes.get(idx) {
            let mut active_scope: Option<(
                Vec<crate::effects::ResolvedTarget>,
                Vec<TargetAssignment>,
            )> = None;
            for inner in &mode.effects {
                let inner_target_assignments = active_target_assignments_for_inner_effect(
                    game,
                    inner,
                    ctx,
                    &mut consumed_modal_selection,
                    &available_assignments,
                    &mut assignment_cursor,
                );
                if !inner_target_assignments.is_empty() {
                    let (inner_targets, inner_target_assignments) =
                        rebase_target_scope(&ctx.targets, &inner_target_assignments);
                    active_scope = Some((inner_targets, inner_target_assignments));
                }
                let outcome = if let Some((inner_targets, inner_target_assignments)) = &active_scope
                {
                    ctx.with_temp_targets(inner_targets.clone(), |ctx| {
                        ctx.with_temp_target_assignments(inner_target_assignments.clone(), |ctx| {
                            execute_effect(game, inner, ctx)
                        })
                    })
                } else {
                    execute_effect(game, inner, ctx)
                }?;
                outcomes.push(outcome);
            }
        }
    }

    Ok(EffectOutcome::aggregate(outcomes)
        .with_execution_fact(ExecutionFact::ChosenOptions(valid_chosen_indices)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::decisions::SelectOptionsContext;
    use crate::effect::{Effect, EffectMode};
    use crate::effects::ChooseModeEffect;
    use crate::game_state::TargetAssignment;
    use crate::ids::CardId;
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn squirrel_token() -> crate::cards::CardDefinition {
        crate::cards::CardDefinitionBuilder::new(CardId::from_raw(6_100), "Squirrel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Squirrel])
            .power_toughness(crate::card::PowerToughness::fixed(1, 1))
            .build()
    }

    #[derive(Default)]
    struct CapturingOptionsDecisionMaker {
        captured: Option<SelectOptionsContext>,
    }

    impl DecisionMaker for CapturingOptionsDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.captured.is_some()
        }

        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            self.captured = Some(ctx.clone());
            Vec::new()
        }
    }

    #[test]
    fn choose_mode_records_selected_modes_in_execution_facts() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_chosen_modes(Some(vec![1]));

        let effect = ChooseModeEffect::choose_one(vec![
            EffectMode::new("Gain 1 life", vec![Effect::gain_life(1)]),
            EffectMode::new("Gain 2 life", vec![Effect::gain_life(2)]),
        ]);

        let result = run_choose_mode(&effect, &mut game, &mut ctx).expect("choose mode resolves");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert!(
            result
                .execution_facts()
                .contains(&ExecutionFact::ChosenOptions(vec![1]))
        );
        assert_eq!(game.player(alice).expect("alice").life, 22);
    }

    #[test]
    fn random_choose_mode_selects_legal_mode_without_prompting() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let random_count_before = game.irreversible_random_count();
        let mut decisions = CapturingOptionsDecisionMaker::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut decisions);

        let effect = ChooseModeEffect::choose_one(vec![
            EffectMode::new("Gain 1 life", vec![Effect::gain_life(1)]),
            EffectMode::new("Gain 2 life", vec![Effect::gain_life(2)]),
        ])
        .with_random_mode_choice();

        let result = run_choose_mode(&effect, &mut game, &mut ctx).expect("choose mode resolves");
        let chosen = result
            .execution_facts()
            .iter()
            .find_map(|fact| match fact {
                ExecutionFact::ChosenOptions(indices) => Some(indices.as_slice()),
                _ => None,
            })
            .expect("random modal choice should record the selected mode");

        assert_eq!(chosen.len(), 1, "random choose-one should select exactly one mode");
        assert!(!ctx.decision_maker.awaiting_choice(), "random modal choice should not prompt");
        assert_eq!(
            game.irreversible_random_count(),
            random_count_before + 1,
            "random modal choice should consume deterministic game RNG"
        );
        assert!(
            matches!(game.player(alice).expect("alice").life, 21 | 22),
            "one of the random gain-life modes should resolve"
        );
    }

    #[test]
    fn choose_mode_scopes_targets_per_selected_mode() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let creature_card =
            crate::card::CardBuilder::new(CardId::from_raw(6_000), "Marked Creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
        let creature = game.create_object_from_card(&creature_card, bob, Zone::Battlefield);
        let land_card = crate::card::CardBuilder::new(CardId::from_raw(6_001), "Marked Land")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_card(&land_card, bob, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_chosen_modes(Some(vec![0, 1]))
            .with_targets(vec![
                crate::effects::ResolvedTarget::Object(creature),
                crate::effects::ResolvedTarget::Object(land),
            ])
            .with_target_assignments(vec![
                TargetAssignment {
                    spec: ChooseSpec::target(ChooseSpec::creature()),
                    range: 0..1,
                },
                TargetAssignment {
                    spec: ChooseSpec::target(ChooseSpec::Object(
                        crate::filter::ObjectFilter::land(),
                    )),
                    range: 1..2,
                },
            ]);

        let effect = ChooseModeEffect::choose_exactly(
            2,
            vec![
                EffectMode::new(
                    "Destroy target creature",
                    vec![Effect::new(crate::effects::DestroyEffect::target(
                        ChooseSpec::creature(),
                    ))],
                ),
                EffectMode::new(
                    "Destroy target land",
                    vec![Effect::new(crate::effects::DestroyEffect::target(
                        ChooseSpec::Object(crate::filter::ObjectFilter::land()),
                    ))],
                ),
            ],
        );

        run_choose_mode(&effect, &mut game, &mut ctx).expect("choose mode resolves");

        assert!(!game.battlefield.contains(&creature));
        assert!(!game.battlefield.contains(&land));
    }

    #[test]
    fn choose_mode_scopes_tagged_damage_target_after_bounce_mode() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let bounce_card =
            crate::card::CardBuilder::new(CardId::from_raw(6_002), "Bounced Creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 4))
                .build();
        let bounced = game.create_object_from_card(&bounce_card, bob, Zone::Battlefield);
        let creature_card =
            crate::card::CardBuilder::new(CardId::from_raw(6_003), "Damaged Creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
        let creature = game.create_object_from_card(&creature_card, bob, Zone::Battlefield);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_chosen_modes(Some(vec![0, 1]))
            .with_targets(vec![
                crate::effects::ResolvedTarget::Object(bounced),
                crate::effects::ResolvedTarget::Object(creature),
            ])
            .with_target_assignments(vec![
                TargetAssignment {
                    spec: ChooseSpec::target(ChooseSpec::creature()),
                    range: 0..1,
                },
                TargetAssignment {
                    spec: ChooseSpec::target(ChooseSpec::creature()),
                    range: 1..2,
                },
            ]);

        let effect = ChooseModeEffect::choose_exactly(
            2,
            vec![
                EffectMode::new(
                    "Return target creature",
                    vec![Effect::return_to_hand(
                        crate::filter::ObjectFilter::creature(),
                    )],
                ),
                EffectMode::new(
                    "Deal damage to target creature",
                    vec![Effect::deal_damage(2, ChooseSpec::creature()).tag("damaged_0")],
                ),
            ],
        );

        run_choose_mode(&effect, &mut game, &mut ctx).expect("choose mode resolves");

        assert!(!game.battlefield.contains(&bounced));
        assert_eq!(game.damage_on(creature), 2);
    }

    #[test]
    fn choose_mode_scopes_player_targets_for_filter_based_inner_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let token_count_before = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .is_some_and(|obj| game.controller_of(obj) == alice)
            })
            .count();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_chosen_modes(Some(vec![0, 1]))
            .with_targets(vec![
                crate::effects::ResolvedTarget::Player(alice),
                crate::effects::ResolvedTarget::Player(bob),
            ])
            .with_target_assignments(vec![
                TargetAssignment {
                    spec: ChooseSpec::target_player(),
                    range: 0..1,
                },
                TargetAssignment {
                    spec: ChooseSpec::target_player(),
                    range: 1..2,
                },
            ]);

        let effect = ChooseModeEffect::choose_exactly(
            2,
            vec![
                EffectMode::new(
                    "Target player creates a Squirrel",
                    vec![Effect::create_tokens_player(
                        squirrel_token(),
                        1,
                        PlayerFilter::target_player(),
                    )],
                ),
                EffectMode::new(
                    "Target player gains 3 life",
                    vec![Effect::new(crate::effects::GainLifeEffect::target_player(
                        3,
                    ))],
                ),
            ],
        );

        run_choose_mode(&effect, &mut game, &mut ctx).expect("choose mode resolves");

        let token_count_after = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .is_some_and(|obj| game.controller_of(obj) == alice)
            })
            .count();

        assert_eq!(token_count_after, token_count_before + 1);
        assert_eq!(game.player(alice).expect("alice").life, 20);
        assert_eq!(game.player(bob).expect("bob").life, 23);
    }

    #[test]
    fn choose_mode_previews_related_objects_from_effect_target_spec() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let creature_card =
            crate::card::CardBuilder::new(CardId::from_raw(6_002), "Mode Preview Creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
        let creature = game.create_object_from_card(&creature_card, bob, Zone::Battlefield);

        let effect = ChooseModeEffect::choose_one(vec![
            EffectMode::new(
                "Destroy target creature an opponent controls",
                vec![Effect::new(crate::effects::DestroyEffect::target(
                    ChooseSpec::Object(
                        crate::filter::ObjectFilter::creature()
                            .controlled_by(PlayerFilter::Opponent),
                    ),
                ))],
            ),
            EffectMode::new("Gain 3 life", vec![Effect::gain_life(3)]),
        ]);

        let mut decision_maker = CapturingOptionsDecisionMaker::default();
        {
            let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
            let result =
                run_choose_mode(&effect, &mut game, &mut ctx).expect("choose mode prompts");
            assert_eq!(result.count_or_zero(), 0);
            assert!(ctx.decision_maker.awaiting_choice());
        }

        let captured = decision_maker.captured.expect("mode prompt captured");
        assert_eq!(
            captured.options[0].related_object_ids.as_deref(),
            Some([creature].as_slice())
        );
        assert_eq!(captured.options[1].related_object_ids, None);
    }
}

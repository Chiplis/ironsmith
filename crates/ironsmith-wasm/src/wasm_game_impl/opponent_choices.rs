// Local opponent choices use the complete native contracts, including aggregate
// bounds and relations that are deliberately absent from the display projection.
fn first_legal_object_selection(
    game: &GameState,
    ctx: &ironsmith::decisions::context::SelectObjectsContext,
) -> Option<Vec<ObjectId>> {
    let candidates: Vec<_> = ctx
        .candidates
        .iter()
        .filter(|candidate| candidate.legal)
        .map(|candidate| candidate.id)
        .collect();
    let min = if ctx.allow_partial_completion {
        0
    } else {
        ctx.min
    };
    let max = ctx.max.unwrap_or(candidates.len()).min(candidates.len());
    let bounds = if let Some(constraint) = &ctx.aggregate_constraint {
        let ironsmith::effect::Value::Fixed(maximum) = constraint.maximum.unhinted() else {
            return None;
        };
        let minimum = match constraint.minimum.as_ref().map(|value| value.unhinted()) {
            Some(ironsmith::effect::Value::Fixed(minimum)) => *minimum,
            Some(_) => return None,
            None => i32::MIN,
        };
        Some((constraint.metric, minimum, *maximum))
    } else {
        None
    };
    let legal = |selected: &[ObjectId]| {
        if selected.len() < min
            || selected.len() > max
            || !ctx.selection_satisfies_relation_filter(game, selected)
        {
            return false;
        }
        bounds.is_none_or(|(metric, minimum, maximum)| {
            let total = ironsmith::targeting::aggregate_object_set_value(
                game,
                selected.iter().copied(),
                metric,
            );
            total >= minimum && total <= maximum
        })
    };
    let initial = candidates.iter().copied().take(ctx.min).collect::<Vec<_>>();
    if legal(&initial) {
        return Some(initial);
    }
    if let Some(selected) = ctx.legal_relation_selection(game, max)
        && legal(&selected)
    {
        return Some(selected);
    }
    // Lower aggregate bounds (collect evidence, for example) usually have a
    // large pool. Try the largest contributions first before searching subsets.
    if let Some((metric, minimum, _)) = bounds {
        let mut ranked = candidates.clone();
        ranked.sort_by_key(|id| {
            let value = ironsmith::targeting::aggregate_object_value(game, *id, metric);
            if minimum == i32::MIN {
                i64::from(value)
            } else {
                -i64::from(value)
            }
        });
        for count in min..=max {
            let selected = &ranked[..count];
            if legal(selected) {
                return Some(selected.to_vec());
            }
        }
    }
    fn search(
        candidates: &[ObjectId],
        start: usize,
        remaining: usize,
        selected: &mut Vec<ObjectId>,
        legal: &impl Fn(&[ObjectId]) -> bool,
    ) -> Option<Vec<ObjectId>> {
        if remaining == 0 {
            return legal(selected).then(|| selected.clone());
        }
        if candidates.len().saturating_sub(start) < remaining {
            return None;
        }
        for index in start..=candidates.len() - remaining {
            selected.push(candidates[index]);
            if let Some(result) = search(candidates, index + 1, remaining - 1, selected, legal) {
                return Some(result);
            }
            selected.pop();
        }
        None
    }
    (min..=max).find_map(|count| search(&candidates, 0, count, &mut Vec::new(), &legal))
}

fn first_legal_target_selection(
    requirements: &[ironsmith::decisions::context::TargetRequirementContext],
) -> Option<Vec<Target>> {
    if let Some(selected) = normalize_targets_for_requirements(requirements, Vec::new())
        && validate_flat_target_assignment(requirements, &selected)
    {
        return Some(selected);
    }
    fn search(
        requirements: &[ironsmith::decisions::context::TargetRequirementContext],
        role: usize,
        selected: &mut Vec<Target>,
    ) -> Option<Vec<Target>> {
        if role == requirements.len() {
            return validate_flat_target_assignment(requirements, selected)
                .then(|| selected.clone());
        }
        let req = &requirements[role];
        fn combinations(
            requirements: &[ironsmith::decisions::context::TargetRequirementContext],
            role: usize,
            start: usize,
            remaining: usize,
            selected: &mut Vec<Target>,
        ) -> Option<Vec<Target>> {
            if remaining == 0 {
                if !validate_flat_target_assignment(&requirements[..=role], selected) {
                    return None;
                }
                return search(requirements, role + 1, selected);
            }
            let pool = &requirements[role].legal_targets;
            if pool.len().saturating_sub(start) < remaining {
                return None;
            }
            for index in start..=pool.len() - remaining {
                selected.push(pool[index]);
                if let Some(result) =
                    combinations(requirements, role, index + 1, remaining - 1, selected)
                {
                    return Some(result);
                }
                selected.pop();
            }
            None
        }
        let max = req
            .max_targets
            .unwrap_or(req.legal_targets.len())
            .min(req.legal_targets.len());
        (req.min_targets..=max)
            .find_map(|count| combinations(requirements, role, 0, count, selected))
    }
    search(requirements, 0, &mut Vec::new())
}

fn first_legal_attacker_selection(
    game: &GameState,
    ctx: &ironsmith::decisions::context::AttackersContext,
) -> Option<Vec<AttackerDeclarationInput>> {
    fn search(
        game: &GameState,
        options: &[ironsmith::decisions::context::AttackerOptionContext],
        start: usize,
        selected: &mut Vec<(ObjectId, AttackTarget)>,
    ) -> Option<Vec<(ObjectId, AttackTarget)>> {
        let mut trial_game = game.clone();
        let mut combat = game.combat.clone().unwrap_or_default();
        let counters = snapshot_id_counters();
        let valid = ironsmith::combat_state::declare_attackers(
            &mut trial_game,
            &mut combat,
            selected.clone(),
        )
        .is_ok();
        restore_id_counters(counters);
        if valid
            && options.iter().all(|option| {
                !option.must_attack
                    || selected
                        .iter()
                        .any(|(creature, _)| *creature == option.creature)
            })
        {
            return Some(selected.clone());
        }
        for index in start..options.len() {
            for target in &options[index].valid_targets {
                selected.push((options[index].creature, target.clone()));
                if let Some(result) = search(game, options, index + 1, selected) {
                    return Some(result);
                }
                selected.pop();
            }
        }
        None
    }
    search(game, &ctx.attacker_options, 0, &mut Vec::new()).map(|selected| {
        selected
            .into_iter()
            .filter_map(|(creature, target)| {
                Some(AttackerDeclarationInput {
                    creature: creature.0,
                    target: match target {
                        AttackTarget::Player(player) => {
                            AttackTargetInput::Player { player: player.0 }
                        }
                        AttackTarget::Planeswalker(object) => {
                            AttackTargetInput::Planeswalker { object: object.0 }
                        }
                        AttackTarget::Battle(object) => {
                            AttackTargetInput::Battle { object: object.0 }
                        }
                        AttackTarget::Nothing { .. } => return None,
                    },
                })
            })
            .collect()
    })
}

fn first_legal_blocker_selection(
    game: &GameState,
    ctx: &ironsmith::decisions::context::BlockersContext,
) -> Option<Vec<BlockerDeclarationInput>> {
    let combat = game.combat.as_ref()?;
    let edges: Vec<_> = ctx
        .blocker_options
        .iter()
        .flat_map(|option| {
            option
                .valid_blockers
                .iter()
                .map(move |(blocker, _)| (*blocker, option.attacker))
        })
        .collect();
    fn search(
        game: &GameState,
        combat: &ironsmith::combat_state::CombatState,
        ctx: &ironsmith::decisions::context::BlockersContext,
        edges: &[(ObjectId, ObjectId)],
        start: usize,
        selected: &mut Vec<(ObjectId, ObjectId)>,
    ) -> Option<Vec<(ObjectId, ObjectId)>> {
        let counts_valid = ctx.blocker_options.iter().all(|option| {
            let count = selected
                .iter()
                .filter(|(_, attacker)| *attacker == option.attacker)
                .count();
            count == 0 || count >= option.min_blockers
        });
        if counts_valid
            && ironsmith::combat_state::declare_blockers(
                game,
                &mut combat.clone(),
                selected.clone(),
            )
            .is_ok()
        {
            return Some(selected.clone());
        }
        for index in start..edges.len() {
            selected.push(edges[index]);
            if let Some(result) = search(game, combat, ctx, edges, index + 1, selected) {
                return Some(result);
            }
            selected.pop();
        }
        None
    }
    search(game, combat, ctx, &edges, 0, &mut Vec::new()).map(|selected| {
        selected
            .into_iter()
            .map(|(blocker, attacker)| BlockerDeclarationInput {
                blocker: blocker.0,
                blocking: attacker.0,
            })
            .collect()
    })
}

#[wasm_bindgen]
impl WasmGame {
    /// A read-only suggestion for local automation; dispatch still validates it.
    #[wasm_bindgen(js_name = getDefaultSelectionCommand)]
    pub fn get_default_selection_command(&self) -> Result<JsValue, JsValue> {
        let game = self.pending_decision_game.as_deref().unwrap_or(&self.game);
        let command = match self.pending_decision.as_ref() {
            Some(DecisionContext::SelectObjects(ctx)) => first_legal_object_selection(game, ctx)
                .map(|selected| UiCommand::SelectObjects {
                    object_ids: selected.into_iter().map(|id| id.0).collect(),
                    object_stable_ids: Vec::new(),
                    object_hidden_refs: Vec::new(),
                }),
            Some(DecisionContext::Partition(_)) => Some(UiCommand::SelectObjects {
                object_ids: Vec::new(),
                object_stable_ids: Vec::new(),
                object_hidden_refs: Vec::new(),
            }),
            Some(DecisionContext::Targets(ctx)) => first_legal_target_selection(&ctx.requirements)
                .map(|selected| UiCommand::SelectTargets {
                    targets: selected
                        .into_iter()
                        .map(|target| match target {
                            Target::Player(player) => TargetInput::Player { player: player.0 },
                            Target::Object(object) => TargetInput::Object { object: object.0 },
                        })
                        .collect(),
                }),
            Some(DecisionContext::Blockers(ctx)) => first_legal_blocker_selection(game, ctx)
                .map(|declarations| UiCommand::DeclareBlockers { declarations }),
            Some(DecisionContext::Attackers(ctx)) => {
                first_legal_attacker_selection(game, ctx).map(|declarations| {
                    UiCommand::DeclareAttackers {
                        declarations,
                        bands: Vec::new(),
                    }
                })
            }
            _ => None,
        };
        serde_wasm_bindgen::to_value(&command)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }
}

#[cfg(test)]
mod opponent_choice_tests {
    use super::*;
    use ironsmith::decisions::context::{
        SelectObjectsContext, SelectableObject, TargetRequirementContext,
    };

    #[test]
    fn blocker_choice_satisfies_must_block_and_menace_together() {
        let _ids = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = ironsmith::card::CardBuilder::new(CardId::from_raw(1), "Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::new(
                ironsmith::card::PtValue::Fixed(2),
                ironsmith::card::PtValue::Fixed(2),
            ))
            .build();
        let attacker = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let required = game.create_object_from_card(&card, bob, Zone::Battlefield);
        let optional = game.create_object_from_card(&card, bob, Zone::Battlefield);
        game.object_mut(attacker).unwrap().abilities_mut().push(
            ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::menace(),
            ),
        );
        game.object_mut(required).unwrap().abilities_mut().push(
            ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::must_block(),
            ),
        );
        game.remove_summoning_sickness(attacker);
        game.object_mut(attacker).unwrap().abilities_mut().push(
            ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::must_attack(),
            ),
        );
        let _ = game.refresh_continuous_state();
        let attack_ctx = ironsmith::decisions::context::AttackersContext::new(
            alice,
            vec![ironsmith::decisions::context::AttackerOptionContext {
                creature: attacker,
                creature_name: "Creature".into(),
                must_attack: true,
                valid_targets: vec![AttackTarget::Player(bob)],
            }],
        );
        let before_ids = snapshot_id_counters();
        let attack =
            first_legal_attacker_selection(&game, &attack_ctx).expect("required attack exists");
        assert_eq!(attack.len(), 1);
        let after_ids = snapshot_id_counters();
        assert_eq!(
            (before_ids.player, before_ids.object, before_ids.card),
            (after_ids.player, after_ids.object, after_ids.card)
        );
        assert!(
            !game.is_tapped(attacker),
            "suggestions do not tap attackers"
        );
        let mut combat = ironsmith::combat_state::CombatState::default();
        ironsmith::combat_state::declare_attackers(
            &mut game,
            &mut combat,
            vec![(attacker, AttackTarget::Player(bob))],
        )
        .unwrap();
        game.combat = Some(combat);
        let ctx = ironsmith::decisions::context::BlockersContext::new(
            bob,
            vec![ironsmith::decisions::context::BlockerOptionContext {
                attacker,
                attacker_name: "Creature".into(),
                min_blockers: 2,
                valid_blockers: vec![(required, "Required".into()), (optional, "Optional".into())],
            }],
        );
        let selected = first_legal_blocker_selection(&game, &ctx).expect("required blocks exist");
        assert_eq!(selected.len(), 2);
        assert!(
            selected
                .iter()
                .all(|declaration| declaration.blocking == attacker.0)
        );
        assert!(
            game.combat.as_ref().unwrap().blockers.is_empty(),
            "suggestions do not mutate combat"
        );
    }

    #[test]
    fn target_choice_backtracks_when_first_target_blocks_a_required_later_role() {
        let a = Target::Player(PlayerId::from_index(0));
        let b = Target::Player(PlayerId::from_index(1));
        let mut first = TargetRequirementContext::single("first", vec![a, b]);
        let mut second = TargetRequirementContext::single("different", vec![a]);
        first.distinct_player_group = Some(0);
        second.distinct_player_group = Some(0);
        let requirements = vec![first, second];
        assert_eq!(
            first_legal_target_selection(&requirements),
            Some(vec![b, a])
        );
    }

    #[test]
    fn target_choice_honors_aggregate_and_legal_set_constraints() {
        let a = Target::Object(ObjectId::from_raw(1));
        let b = Target::Object(ObjectId::from_raw(2));
        let c = Target::Object(ObjectId::from_raw(3));
        let mut req = TargetRequirementContext::single("pair", vec![a, b, c]);
        req.min_targets = 2;
        req.max_targets = Some(2);
        req.legal_target_sets = vec![vec![a, c], vec![b, c]];
        req.aggregate_constraint = Some(ironsmith::targeting::ResolvedTargetAggregateConstraint {
            metric: ironsmith::effect::ChoiceAggregateMetric::ManaValue,
            maximum: 3,
            target_values: vec![(a, 5), (b, 1), (c, 2)],
        });
        assert_eq!(first_legal_target_selection(&[req]), Some(vec![b, c]));
    }

    #[test]
    fn object_choice_reaches_required_aggregate_even_when_minimum_count_is_zero() {
        let _ids = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let player = PlayerId::from_index(1);
        let mut candidates = Vec::new();
        for (index, value) in [1, 4, 3].into_iter().enumerate() {
            let card = ironsmith::card::CardBuilder::new(
                CardId::from_raw(index as u32 + 1),
                format!("Evidence {index}"),
            )
            .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![vec![
                ironsmith::mana::ManaSymbol::Generic(value),
            ]]))
            .card_types(vec![CardType::Sorcery])
            .build();
            let id = game.create_object_from_card(&card, player, Zone::Graveyard);
            candidates.push(SelectableObject::new(id, card.name.clone()));
        }
        let ctx =
            SelectObjectsContext::new(player, None, "Collect evidence 6", candidates, 0, None)
                .with_aggregate_constraint(
                    ironsmith::effect::ChoiceAggregateConstraint::total_mana_value_at_least(6),
                );
        let selected = first_legal_object_selection(&game, &ctx).expect("payable evidence choice");
        assert_eq!(selected.len(), 2);
        assert_eq!(
            ironsmith::targeting::aggregate_object_set_value(
                &game,
                selected.into_iter(),
                ironsmith::effect::ChoiceAggregateMetric::ManaValue
            ),
            7
        );
    }
}

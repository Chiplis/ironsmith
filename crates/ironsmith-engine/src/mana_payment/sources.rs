//! Read-only mana-source projection, separate from payment execution.
//!
//! Reviewed tap abilities evaluate production, immediate triggers and pure
//! replacements without committing an activation. Credits retain their source
//! metadata for payment qualification. Unknown dependencies keep authoritative
//! simulation, and every selected sequence is executed and validated natively.
use super::planner::ActivationChoice;
use super::resources::{ManaCredit, ManaCreditContext};
use crate::ability::AbilityKind;
use crate::color::Color;
use crate::derived_view::DerivedGameView;
#[cfg(test)]
use crate::effect::Value;
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::player::ManaPool;
#[cfg(test)]
use crate::target::PlayerFilter;

pub(super) struct ManaSourceAnalysis<'a> {
    pub view: DerivedGameView<'a>,
    game: &'a GameState,
    has_mana_triggers: bool,
    projection_safe: bool,
    replacements: Option<super::replacement_program::CompiledManaReplacements>,
    branch_cache: std::cell::RefCell<
        std::collections::HashMap<
            (crate::ids::ObjectId, usize, Option<Vec<Color>>),
            Option<std::rc::Rc<Vec<ProjectedManaBranch>>>,
        >,
    >,
}

pub(super) struct ProjectedMana {
    pub output: ManaPool,
    pub credits: Vec<ManaCredit>,
    /// A color selection not fixed by the proposed activation. Such an option
    /// may be predicted by a chooser, but cannot be deferred by the payment UI.
    pub needs_choice: bool,
}

pub(super) struct ProjectedManaBranch {
    pub output: ManaPool,
    pub credits: Vec<ManaCredit>,
    pub witnesses: Vec<super::ManaReplacementWitness>,
}

impl<'a> ManaSourceAnalysis<'a> {
    pub fn new(game: &'a GameState) -> Self {
        let view = DerivedGameView::new(game);
        view.prewarm_characteristics(&game.battlefield);
        // Registered effects alone are insufficient: generated replacement
        // abilities may not have been synchronized since the last mutation.
        let has_mana_triggers = has_potential_mana_triggers(game, &view);
        let replacements = super::replacement_program::CompiledManaReplacements::compile(game);
        let projection_safe = (!has_mana_triggers
            || tap_mana_triggers_are_projectable(game, &view))
            && !view.has_activated_ability_cost_modifiers()
            && !game.continuous_effects_are_tap_sensitive()
            && replacements.is_some();
        Self {
            view,
            game,
            has_mana_triggers,
            projection_safe,
            replacements,
            branch_cache: Default::default(),
        }
    }

    pub fn project(&self, choice: &ActivationChoice) -> Option<ProjectedMana> {
        if let Some(witnesses) = &choice.replacement_witnesses {
            return self
                .branches(choice)?
                .iter()
                .find(|branch| branch.witnesses == *witnesses)
                .map(|branch| ProjectedMana {
                    output: branch.output.clone(),
                    credits: branch.credits.clone(),
                    needs_choice: false,
                });
        }
        self.project_inner(choice)
    }

    pub fn branches(
        &self,
        choice: &ActivationChoice,
    ) -> Option<std::rc::Rc<Vec<ProjectedManaBranch>>> {
        // The analysis borrows one immutable game. All witnesses for the same
        // source/domain share this expansion; validating N selected outputs
        // must not execute the entire N-way event tree N times.
        let key = (
            choice.source,
            choice.ability_index,
            choice.color_restriction.clone(),
        );
        if let Some(cached) = self.branch_cache.borrow().get(&key) {
            return cached.clone();
        }
        let result = self.expand_branches(choice).map(std::rc::Rc::new);
        self.branch_cache.borrow_mut().insert(key, result.clone());
        result
    }

    fn expand_branches(&self, choice: &ActivationChoice) -> Option<Vec<ProjectedManaBranch>> {
        use crate::effects::mana::production_resolution::ResolvedManaOutput;
        if !self.projection_safe {
            return None;
        }
        let ability = self
            .view
            .abilities_rc(choice.source)?
            .get(choice.ability_index)?
            .clone();
        let AbilityKind::Activated(activated) = &ability.kind else {
            return None;
        };
        // The source controller and the ability activator can differ. This
        // projection's event context binds the former; retain full execution
        // for public activator permissions until it carries an explicit actor.
        if activated.allows_any_player_to_activate() {
            return None;
        }
        let costs = activated.mana_cost.as_all()?;
        if costs.len() != 1
            || !costs[0].requires_tap()
            || !activated.choices.is_empty()
            || activated.is_exhaust_ability()
            || activated
                .effects
                .segments
                .iter()
                .any(|segment| !segment.self_replacements.is_empty())
        {
            return None;
        }
        let controller = self.view.current_controller(choice.source)?;
        let object = self.game.object(choice.source)?;
        let chars = self.view.calculated_characteristics_arc(choice.source)?;
        let mut snapshot = crate::snapshot::ObjectSnapshot::from_object_with_known_characteristics(
            object,
            self.game,
            Some(&chars),
        );
        let activation = crate::events::AbilityActivatedEvent::from_effective_ability(
            choice.source,
            controller,
            true,
            Some(ability.clone()),
            Some(snapshot.clone()),
        )
        .with_activation_cost_has_tap(true);
        snapshot.tapped = true;
        let make_event = |player, symbols| {
            crate::events::ManaAddedEvent::new(choice.source, controller, player, symbols)
                .with_snapshot(Some(snapshot.clone()))
                .with_production_provenance(
                    crate::events::mana::ManaProductionProvenance::TappedSourceForMana,
                )
                .into_trigger_event()
        };
        let mut batches = vec![Vec::new()];
        if let Some(symbols) = &activated.mana_output {
            if !symbols.is_empty() {
                batches[0].push(make_event(controller, symbols.clone()));
            }
        }
        for effect in activated.effects.iter() {
            let mut resolved =
                effect
                    .mana_production()?
                    .stable_resolved(self.game, choice.source, controller)?;
            if let ResolvedManaOutput::Choice { available, .. } = &mut resolved.output {
                if resolved.player != controller {
                    return None;
                }
                if let Some(colors) = &choice.color_restriction {
                    available.retain(|symbol| {
                        colors
                            .iter()
                            .any(|color| crate::mana::ManaSymbol::from_color(*color) == *symbol)
                    });
                }
            }
            let outputs = resolved.output.alternatives(128)?;
            if batches.len().checked_mul(outputs.len())? > 128 {
                return None;
            }
            batches = batches
                .into_iter()
                .flat_map(|prefix| {
                    outputs
                        .iter()
                        .map(|symbols| {
                            let mut next = prefix.clone();
                            if !symbols.is_empty() {
                                next.push(make_event(resolved.player, symbols.clone()));
                            }
                            next
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
        }
        let mut result = Vec::new();
        for mut events in batches {
            events.push(crate::triggers::TriggerEvent::new(
                activation.clone(),
                crate::provenance::ProvNodeId::default(),
            ));
            for branch in super::event_program::evaluate_branches_with_context(
                self.game,
                &self.view,
                events,
                choice.source,
                512,
                self.replacements.as_ref()?,
                &super::replacement_program::ReplacementResources::default(),
                ManaCreditContext::from_activation(self.game, choice.source, activated),
            )? {
                if branch.witnesses.iter().any(|witness| {
                    witness.original.player != controller && !witness.decisions.is_empty()
                }) {
                    return None;
                }
                let mut output = ManaPool::new();
                for credit in &branch.result.credits {
                    if credit.event.player == controller {
                        for &symbol in &credit.event.mana {
                            output.add(symbol, 1);
                        }
                    }
                }
                result.push(ProjectedManaBranch {
                    output,
                    credits: branch.result.credits,
                    witnesses: branch
                        .witnesses
                        .into_iter()
                        .map(|witness| {
                            super::ManaReplacementWitness::from_event(
                                &witness.original,
                                witness.decisions,
                            )
                        })
                        .collect(),
                });
                if result.len() > 512 {
                    return None;
                }
            }
        }
        Some(result)
    }

    fn project_inner(&self, choice: &ActivationChoice) -> Option<ProjectedMana> {
        if !self.projection_safe {
            return None;
        }
        let ability = self
            .view
            .abilities_rc(choice.source)?
            .get(choice.ability_index)?
            .clone();
        let AbilityKind::Activated(activated) = &ability.kind else {
            return None;
        };
        if activated.allows_any_player_to_activate() {
            return None;
        }
        // Tapping is the only state change allowed before the fixed production.
        // Untap, sacrifice, life, counters, filters, exhaust, and player choices
        // remain simulations. Collection has already checked activation legality.
        let costs = activated.mana_cost.as_all()?;
        if costs.len() != 1
            || !costs[0].requires_tap()
            || !activated.choices.is_empty()
            || activated.is_exhaust_ability()
            || activated
                .effects
                .segments
                .iter()
                .any(|segment| !segment.self_replacements.is_empty())
        {
            return None;
        }
        let mut needs_choice = false;
        let mut production_events = Vec::new();
        if let Some(symbols) = &activated.mana_output {
            if !symbols.is_empty() {
                production_events.push(symbols.clone());
            }
        }
        for effect in activated.effects.iter() {
            let production = effect.mana_production()?.stable_event(
                self.game,
                choice.source,
                self.view.current_controller(choice.source)?,
                choice.color_restriction.as_deref(),
            )?;
            needs_choice |= production.needs_choice;
            if !production.symbols.is_empty() {
                production_events.push(production.symbols);
            }
        }
        let controller = self.view.current_controller(choice.source)?;
        let object = self.game.object(choice.source)?;
        let chars = self.view.calculated_characteristics_arc(choice.source)?;
        let mut snapshot = crate::snapshot::ObjectSnapshot::from_object_with_known_characteristics(
            object,
            self.game,
            Some(&chars),
        );
        let activation = crate::events::AbilityActivatedEvent::from_effective_ability(
            choice.source,
            controller,
            true,
            Some(ability.clone()),
            Some(snapshot.clone()),
        )
        .with_activation_cost_has_tap(true);
        snapshot.tapped = true;
        let credit_context =
            ManaCreditContext::from_activation(self.game, choice.source, activated);
        let initial = production_events
            .into_iter()
            .map(|symbols| {
                crate::events::ManaAddedEvent::new(choice.source, controller, controller, symbols)
                    .with_snapshot(Some(snapshot.clone()))
                    .with_production_provenance(
                        crate::events::mana::ManaProductionProvenance::TappedSourceForMana,
                    )
            })
            .collect::<Vec<_>>();
        let credits = if self.has_mana_triggers || !self.replacements.as_ref()?.is_empty() {
            let mut events = initial
                .into_iter()
                .map(|event| event.into_trigger_event())
                .collect::<Vec<_>>();
            events.push(crate::triggers::TriggerEvent::new(
                activation,
                crate::provenance::ProvNodeId::default(),
            ));
            let evaluated = super::event_program::evaluate_with_context(
                self.game,
                &self.view,
                events,
                choice.source,
                512,
                self.replacements.as_ref()?,
                credit_context,
            )?;
            // Ordinary triggers remain pending for authoritative replay.
            evaluated.credits
        } else {
            initial
                .into_iter()
                .map(|event| ManaCredit {
                    event,
                    context: credit_context.clone(),
                })
                .collect()
        };
        let mut output = ManaPool::new();
        for credit in &credits {
            if credit.event.player == controller {
                for &symbol in &credit.event.mana {
                    output.add(symbol, 1);
                }
            }
        }
        // A zero-output activation is not a mana contribution. Source-specific
        // restrictions and snow provenance are kept on the authoritative plan.
        Some(ProjectedMana {
            output,
            credits,
            needs_choice,
        })
    }
}

/// Until a trigger is represented in the compact model it must participate in
/// exact simulation, and a printed-output bound cannot prove unpayability.
pub(crate) fn has_potential_mana_triggers(game: &GameState, view: &DerivedGameView<'_>) -> bool {
    has_unprojected_queued_mana_triggers(game)
        || game.objects_map().values().any(|object| {
            view.abilities_rc(object.id).is_some_and(|abilities| abilities.iter().any(|ability| {
                ability.functions_in(&object.zone) && matches!(&ability.kind, AbilityKind::Triggered(trigger)
                    if trigger.effects.all_effects().into_iter().any(|effect| effect.contains_mana_production()))
            }))
        })
}

/// Compile the state-independent immediate trigger sublanguage. Any unknown
/// matcher, condition, choice or side effect keeps authoritative simulation.
fn tap_mana_triggers_are_projectable(game: &GameState, view: &DerivedGameView<'_>) -> bool {
    !has_unprojected_queued_mana_triggers(game)
        && super::event_program::triggers_are_independent(game, view)
}

// Ordinary delayed and pending triggers wait for stack resolution and cannot
// contribute to this payment. In particular, a delayed zone-change trigger
// installed by animating a land must not disable projection of its mana ability.
fn has_unprojected_queued_mana_triggers(game: &GameState) -> bool {
    game.effect_store.delayed_triggers.iter().any(|trigger| {
        trigger
            .effects
            .all_effects()
            .into_iter()
            .any(|effect| effect.contains_mana_production())
    }) || game
        .effect_store
        .pending_trigger_entries
        .iter()
        .any(|entry| {
            entry
                .ability
                .effects
                .all_effects()
                .into_iter()
                .any(|effect| effect.contains_mana_production())
        })
}

pub(super) fn pool_units(pool: &ManaPool) -> Vec<ManaSymbol> {
    [
        (pool.white, ManaSymbol::White),
        (pool.blue, ManaSymbol::Blue),
        (pool.black, ManaSymbol::Black),
        (pool.red, ManaSymbol::Red),
        (pool.green, ManaSymbol::Green),
        (pool.colorless, ManaSymbol::Colorless),
    ]
    .into_iter()
    .flat_map(|(count, symbol)| std::iter::repeat_n(symbol, count as usize))
    .collect()
}

pub(crate) fn has_mana_modifying_replacements(game: &GameState) -> bool {
    use crate::events::EventKind;
    has_replacements_for_events(
        game,
        &[
            EventKind::BecomeTapped,
            EventKind::ManaAdded,
            EventKind::AbilityActivated,
        ],
    )
}

pub(super) fn has_replacements_for_events(
    game: &GameState,
    kinds: &[crate::events::EventKind],
) -> bool {
    let unrelated = |effect: &crate::replacement::ReplacementEffect| {
        effect.matcher.as_ref().is_some_and(|matcher| {
            kinds
                .iter()
                .all(|&kind| !matcher.may_match_event_kind(kind))
        })
    };
    game.effect_store
        .replacement_effects
        .effects()
        .iter()
        .any(|effect| !unrelated(effect))
        || crate::replacement_ability_processor::generate_replacement_effects_from_abilities(game)
            .map_or(true, |effects| {
                effects.iter().any(|effect| !unrelated(effect))
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::costs::{Cost, PaymentReason};
    use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use crate::effect::Effect;
    use crate::effects::AddManaEffect;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaCost;
    use crate::mana_payment::planner::collect_activation_choices;
    use crate::mana_payment::{
        ManaPaymentRequest, mana_payment_activation_inventory,
        mana_payment_ready_activation_inventory,
    };
    use crate::types::CardType;
    use crate::{Ability, CardBuilder, TotalCost, Zone};

    fn fixture(
        costs: TotalCost,
        output: Vec<ManaSymbol>,
        effects: Vec<Effect>,
    ) -> (GameState, ManaPaymentRequest) {
        let mut game = GameState::new(vec!["Alice".into()], 20);
        let payer = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Test mana source")
            .card_types(vec![CardType::Land])
            .build();
        let source = game.create_object_from_card(&card, payer, Zone::Battlefield);
        let mut activated =
            crate::ability::ActivatedAbility::mana_with_costs(costs, vec![], output);
        activated.effects = effects.into();
        let ability = Ability {
            kind: AbilityKind::Activated(activated),
            functional_zones: vec![Zone::Battlefield],
        };
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(ability);
        game.refresh_continuous_state().unwrap();
        let request = ManaPaymentRequest::new(
            payer,
            source,
            PaymentReason::Effect,
            ManaCost::new().add_generic(1),
        );
        (game, request)
    }

    fn check_builtin_basic_land_projection(with_protection: bool) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        // The actual compiled Yawgmoth board remains covered by the owning
        // web-session test. Engine-only tests cannot invoke its text parser.
        let registry = crate::cards::CardRegistry::with_builtin_cards_for_names(["Swamp"]);
        let mut builder = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Projection subject")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 4));
        if with_protection {
            builder = builder.with_ability(Ability::static_ability(crate::static_abilities::StaticAbility::protection(
                crate::ability::ProtectionFrom::Permanents(crate::filter::ObjectFilter {
                    subtypes: vec![crate::types::Subtype::Human], ..crate::filter::ObjectFilter::creature()
                }))));
        }
        let source = game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
        let mut lands = Vec::new();
        for _ in 0..8 {
            lands.push(game.create_object_from_definition(registry.get("Swamp").unwrap(), alice, Zone::Battlefield));
        }
        game.refresh_continuous_state().unwrap();
        let request = ManaPaymentRequest::new(alice, source, PaymentReason::ActivateAbility, ManaCost::new().add_generic(4));
        let analysis = ManaSourceAnalysis::new(&game);
        let choices = collect_activation_choices(&game, &request);
        assert_eq!(choices.len(), 8);
        for choice in &choices {
            let projected = analysis.project(choice);
            assert!(projected.is_some(), "with_protection={with_protection}, projection_safe={}, mana_triggers={}, modifiers={}, tap_sensitive={}, ability={:?}, registered={:?}, generated={:?}",
                analysis.projection_safe, analysis.has_mana_triggers, analysis.view.has_activated_ability_cost_modifiers(),
                game.continuous_effects_are_tap_sensitive(), game.current_ability(choice.source, choice.ability_index),
                game.effect_store.replacement_effects.effects(),
                crate::replacement_ability_processor::generate_replacement_effects_from_abilities(&game));
            assert_eq!(projected.unwrap().output.black, 1);
        }
        let plan = crate::mana_payment::plan_first_mana_payment(&game, &request).unwrap();
        assert!(plan.payable);
        assert_eq!(plan.mana_ability_steps.len(), 4);
        let perf = crate::mana_payment::last_mana_payment_perf();
        assert_eq!(perf.searched_selections, 0);
        assert_eq!(perf.visited_nodes, 0);
        assert!(perf.analytic_selections > 0);
        assert!(lands.iter().all(|land| !game.is_tapped(*land)));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }
    #[test]
    fn ward_does_not_disable_independent_mana_projection() {
        let (mut game, request) = fixture(
            TotalCost::from_cost(Cost::tap()),
            vec![ManaSymbol::Blue],
            vec![],
        );
        game.object_mut(request.source).unwrap().abilities_mut().push(
            Ability::static_ability(crate::static_abilities::StaticAbility::ward(
                TotalCost::from_cost(Cost::mana(ManaCost::new().add_generic(4))),
            )),
        );
        game.refresh_continuous_state().unwrap();
        assert!(!game.continuous_effects_are_tap_sensitive(),
            "Ward triggers on becoming targeted; it does not generate continuous characteristics");
        let analysis = ManaSourceAnalysis::new(&game);
        let choices = collect_activation_choices(&game, &request);
        assert!(!choices.is_empty());
        assert!(choices.iter().all(|choice| analysis.project(choice).is_some()));
        let wards = crate::targeting::collect_ward_costs(
            &game, &[request.source], PlayerId::from_index(1),
        );
        assert_eq!(wards.len(), 1, "opponent targeting still incurs Ward");
        assert!(crate::targeting::collect_ward_costs(
            &game, &[request.source], request.payer,
        ).is_empty(), "controller targeting does not incur Ward");
        assert!(crate::mana_payment::plan_first_mana_payment(&game, &request).unwrap().payable);
    }

    #[test]
    fn unrelated_draw_replacement_preserves_mana_projection() {
        for conditional in [false, true] {
            let (mut game, request) = fixture(
                TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Blue], vec![],
            );
            let mut model = crate::static_abilities::CompiledStaticAbility::draw_extra_cards_replacement(1, "Draw an extra card");
            if conditional {
                model = model.with_condition(crate::ConditionExpr::PlayerCardsInHandOrFewer {
                    player: PlayerFilter::You, count: 1,
                });
            }
            game.object_mut(request.source).unwrap().abilities_mut().push(
                Ability::static_ability(crate::static_abilities::StaticAbility::from_model(model)),
            );
            game.refresh_continuous_state().unwrap();
            assert!(!has_mana_modifying_replacements(&game),
                "a draw-only replacement cannot observe tap, mana or activation events");
            assert!(!game.continuous_effects_are_tap_sensitive());
            let analysis = ManaSourceAnalysis::new(&game);
            let choices = collect_activation_choices(&game, &request);
            assert!(!choices.is_empty());
            assert!(choices.iter().all(|choice| analysis.project(choice).is_some()));
            let effects = crate::replacement_ability_processor::generate_replacement_effects_from_abilities(&game).unwrap();
            assert_eq!(effects.len(), 1);
            let matcher = effects[0].matcher.as_ref().unwrap();
            let ctx = crate::events::EventContext::for_replacement_effect(request.payer, request.source, &game);
            let mut draw = crate::events::DrawEvent::new(request.payer, 1, true);
            assert!(matcher.may_match_event_kind(crate::events::EventKind::Draw));
            assert!(matcher.matches_event(&draw, &ctx).unwrap());
            draw.first_of_instruction = false;
            assert!(!matcher.matches_event(&draw, &ctx).unwrap());
        }
    }

    #[test]
    fn entry_only_replacements_preserve_mana_projection() {
        for ability in [
            crate::static_abilities::StaticAbility::enters_tapped_unless_control_two_or_fewer_other_lands(),
            crate::static_abilities::StaticAbility::pay_life_or_enter_tapped(2),
            crate::static_abilities::StaticAbility::affinity_for_artifacts(),
        ] {
            let (mut game, request) = fixture(
                TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Blue], vec![],
            );
            game.object_mut(request.source).unwrap().abilities_mut().push(Ability::static_ability(ability));
            game.refresh_continuous_state().unwrap();
            assert!(!has_mana_modifying_replacements(&game));
            assert!(!game.continuous_effects_are_tap_sensitive());
            let analysis = ManaSourceAnalysis::new(&game);
            let choices = collect_activation_choices(&game, &request);
            assert!(!choices.is_empty());
            assert!(choices.iter().all(|choice| analysis.project(choice).is_some()));
        }
    }

    #[test]
    fn fixed_color_and_subtype_changes_preserve_only_independent_projection() {
        for tapped in [false, true] {
            for subtype in [false, true] {
                let (mut game, request) = fixture(
                    TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Blue], vec![],
                );
                let mut filter = crate::target::ObjectFilter::source();
                filter.tapped = tapped;
                let model = if subtype {
                    crate::static_abilities::CompiledStaticAbility::add_subtypes(filter, vec![crate::types::Subtype::Forest])
                } else {
                    crate::static_abilities::CompiledStaticAbility::make_colorless(filter)
                };
                game.object_mut(request.source).unwrap().abilities_mut().push(Ability::static_ability(
                    crate::static_abilities::StaticAbility::from_model(model),
                ));
                game.refresh_continuous_state().unwrap();
                assert_eq!(game.continuous_effects_are_tap_sensitive(), tapped);
                let analysis = ManaSourceAnalysis::new(&game);
                let choices = collect_activation_choices(&game, &request);
                assert!(!choices.is_empty());
                assert_eq!(choices.iter().all(|choice| analysis.project(choice).is_some()), !tapped);
            }
        }
    }

    #[test] fn builtin_basic_lands_use_fixed_projection() { check_builtin_basic_land_projection(false); }
    #[test] fn builtin_basic_lands_project_with_unrelated_protection() { check_builtin_basic_land_projection(true); }

    #[test]
    fn derived_player_restrictions_do_not_invalidate_refresh() {
        let (mut game, request) = fixture(
            TotalCost::from_cost(Cost::tap()),
            vec![ManaSymbol::Green],
            vec![],
        );
        game.object_mut(request.source).unwrap().abilities_mut().push(Ability::static_ability(
            crate::static_abilities::StaticAbility::additional_land_plays(1),
        ));
        game.object_mut(request.source).unwrap().abilities_mut().push(Ability::static_ability(
            crate::static_abilities::StaticAbility::no_maximum_hand_size(),
        ));
        game.refresh_continuous_state().unwrap();
        assert!(game.continuous_state_is_clean());
        assert_eq!(game.player(request.payer).unwrap().land_plays_per_turn, 2);
        assert_eq!(game.player(request.payer).unwrap().max_hand_size, i32::MAX);
        assert!(!game.continuous_effects_are_tap_sensitive(),
            "derived hand and land limits must not block mana projection: {:?}",
            game.current_characteristics(request.source).unwrap().static_abilities.iter()
                .map(|a| (a.display(), a.may_generate_continuous_effects(), format!("{:?}", a.compiled_model())))
                .collect::<Vec<_>>());
        let before = game.work_counters();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.work_counters().static_ability_regens, before.static_ability_regens);
        assert_eq!(game.work_counters().continuous_global_invalidations, before.continuous_global_invalidations);
        assert_eq!(game.player(request.payer).unwrap().land_plays_per_turn, 2);
        // An actual player mutation must still invalidate the snapshot.
        game.player_mut(request.payer).unwrap().life -= 1;
        assert!(!game.continuous_state_is_clean());
        game.refresh_continuous_state().unwrap();
        assert!(game.continuous_state_is_clean());
        assert_eq!(game.player(request.payer).unwrap().land_plays_per_turn, 2);
        game.object_mut(request.source).unwrap().abilities_mut().retain(|ability| {
            !matches!(ability.kind, crate::ability::AbilityKind::Static(_))
        });
        game.refresh_continuous_state().unwrap();
        assert!(game.continuous_state_is_clean());
        assert_eq!(game.player(request.payer).unwrap().land_plays_per_turn, 1);
        assert_eq!(game.player(request.payer).unwrap().max_hand_size, 7);
    }

    #[test]
    fn mana_credit_preserves_only_proven_clean_characteristics() {
        use crate::effects::{EffectExecutor, ExecutionContext};
        let (mut game, request) = fixture(
            TotalCost::from_cost(Cost::tap()),
            vec![ManaSymbol::Green],
            vec![],
        );
        game.object_mut(request.source).unwrap().abilities_mut().push(Ability::static_ability(
            crate::static_abilities::StaticAbility::from_model(
                crate::static_abilities::CompiledStaticAbility::source_line_static_group(2),
            ),
        ));
        game.object_mut(request.source).unwrap().abilities_mut().push(Ability::static_ability(
            crate::static_abilities::StaticAbility::haste(),
        ));
        game.object_mut(request.source).unwrap().abilities_mut().push(Ability::static_ability(
            crate::static_abilities::StaticAbility::from_model(
                crate::static_abilities::CompiledStaticAbility::grants(
                    ironsmith_core::GrantSpec::play_lands_from_graveyard(),
                ),
            ),
        ));
        game.refresh_continuous_state().unwrap();
        game.tap(request.source);
        assert!(game.continuous_state_is_clean(), "a keyword cannot observe tapping");
        let mut context = ExecutionContext::new_default(request.source, request.payer);
        AddManaEffect::you(vec![ManaSymbol::Green])
            .execute(&mut game, &mut context)
            .unwrap();
        assert!(game.continuous_state_is_clean());
        assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 1);

        // Crediting mana must never acknowledge an unrelated player mutation.
        game.player_mut(request.payer).unwrap().life -= 1;
        game.with_player_mana_mut(request.payer, |player| {
            player.mana_pool.add(ManaSymbol::Green, 1);
        });
        assert!(!game.continuous_state_is_clean());
        game.refresh_continuous_state().unwrap();
        game.with_player_mana_mut(request.payer, |player| {
            player.mana_pool.add(ManaSymbol::Green, 1);
        });
        assert!(game.continuous_state_is_clean());
        game.player_mut(request.payer).unwrap().life -= 1;
        assert!(!game.continuous_state_is_clean());
    }

    #[test]
    fn unspent_mana_anthem_is_recomputed_after_shared_mana_credit() {
        use crate::effects::{EffectExecutor, ExecutionContext};
        use crate::static_abilities::{Anthem, AnthemCountExpression, AnthemValue, StaticAbility};
        let (mut game, request) = fixture(
            TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Green], vec![],
        );
        let card = CardBuilder::new(CardId::new(), "Mana-dependent creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, 1)).build();
        let creature = game.create_object_from_card(&card, request.payer, Zone::Battlefield);
        let value = AnthemValue::scaled(1, AnthemCountExpression::UnspentMana {
            player: PlayerFilter::You, symbol: ManaSymbol::Green,
        });
        game.object_mut(creature).unwrap().abilities_mut().push(Ability::static_ability(
            StaticAbility::new(Anthem::for_source(0, 0).with_values(value.clone(), value)),
        ));
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_characteristics(creature).unwrap().power, Some(1));
        assert!(game.continuous_effects_are_tap_sensitive());
        let mut context = ExecutionContext::new_default(request.source, request.payer);
        AddManaEffect::you(vec![ManaSymbol::Green]).execute(&mut game, &mut context).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_characteristics(creature).unwrap().power, Some(2));
    }

    #[test]
    fn copied_ability_donor_dependencies_prevent_mana_cache_retention() {
        use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
        use crate::target::ObjectFilter;
        let mut filter = ObjectFilter::default();
        filter.tapped = true;
        for modification in [
            Modification::CopyActivatedAbilities {
                filter: filter.clone(), counter: None, include_mana: true,
                only_loyalty: false, exclude_source_name: false,
                exclude_source_id: false, force_once_each_turn: false,
            },
            Modification::CopyTriggeredAbilities {
                filter: filter.clone(), exclude_source_name: false, exclude_source_id: false,
            },
            Modification::CopyStaticAbilityVariants {
                filter: filter.clone(), selectors: vec![], exclude_source_id: false,
            },
        ] {
            let (mut game, request) = fixture(
                TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Green], vec![],
            );
            game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
                request.source, request.payer, EffectTarget::Filter(ObjectFilter::creature()),
                modification,
            ));
            game.refresh_continuous_state().unwrap();
            assert!(game.continuous_state_is_clean());
            assert!(game.continuous_effects_are_tap_sensitive());
            game.tap(request.source);
            assert!(!game.continuous_state_is_clean(), "tapping must invalidate donor-dependent abilities");
            game.refresh_continuous_state().unwrap();
            game.with_player_mana_mut(request.payer, |player| {
                player.mana_pool.add(ManaSymbol::Green, 1);
            });
            assert!(!game.continuous_state_is_clean());
        }
    }

    #[test]
    fn nested_activation_history_prevents_mana_cache_retention() {
        use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
        use crate::target::ObjectFilter;
        let (mut game, request) = fixture(
            TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Green], vec![],
        );
        let mut history = ObjectFilter::default();
        history.ability_activated_this_turn = true;
        let mut filter = ObjectFilter::default();
        filter.any_of.push(history);
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
            request.source, request.payer, EffectTarget::Filter(filter),
            Modification::ModifyPowerToughness { power: 1, toughness: 1 },
        ));
        game.refresh_continuous_state().unwrap();
        assert!(game.continuous_effects_are_tap_sensitive());
        assert!(!game.retain_continuous_state_after_mana_activation());
    }

    #[test]
    fn retained_mana_state_acknowledges_the_player_mutation_cursor() {
        let (mut game, request) = fixture(
            TotalCost::from_cost(Cost::tap()),
            vec![ManaSymbol::Green],
            vec![],
        );
        game.refresh_continuous_state().unwrap();
        assert!(game.continuous_state_is_clean());
        game.player_mut(request.payer)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);
        assert!(game.retain_continuous_state_after_mana_activation());
        assert!(
            game.continuous_state_is_clean(),
            "the next query must not rediscover the acknowledged mana mutation"
        );
        // A later unrelated mutation must still invalidate the retained state.
        game.player_mut(request.payer).unwrap().life -= 1;
        assert!(!game.continuous_state_is_clean());
    }

    #[test]
    fn triggered_mana_counts_for_inventory_planning_and_execution() {
        use crate::mana_payment::{execute_mana_payment_plan, plan_first_mana_payment};
        use crate::target::ObjectFilter;
        use crate::triggers::Trigger;
        for (copies, effect_output, instructions) in [(1, false, 1), (2, false, 1), (1, true, 1), (2, true, 1), (1, true, 2), (2, true, 3)] {
            let (mut game, mut request) = fixture(
                TotalCost::from_cost(Cost::tap()),
                if effect_output {
                    vec![]
                } else {
                    vec![ManaSymbol::Green]
                },
                if effect_output {
                    vec![Effect::add_mana(vec![ManaSymbol::Green]); instructions as usize]
                } else {
                    vec![]
                },
            );
            let source = request.source;
            // A separate permanent grants the trigger, just as Cub does.
            let card = CardBuilder::new(CardId::new(), "Mana trigger")
                .card_types(vec![CardType::Enchantment])
                .build();
            for _ in 0..copies {
                let id = game.create_object_from_card(&card, request.payer, Zone::Battlefield);
                game.object_mut(id)
                    .unwrap()
                    .abilities_mut()
                    .push(Ability::triggered(
                        Trigger::player_taps_for_mana(PlayerFilter::You, ObjectFilter::land()),
                        vec![Effect::add_mana(vec![ManaSymbol::Green])],
                    ));
            }
            // This ordinary trigger must survive automatic payment, without
            // resolving early or being matched again at the priority boundary.
            game.object_mut(source)
                .unwrap()
                .abilities_mut()
                .push(Ability::triggered(
                    Trigger::player_taps_for_mana(PlayerFilter::You, ObjectFilter::land()),
                    vec![Effect::gain_life(1)],
                ));
            // The real earthbend instruction registers a delayed return
            // trigger. It must coexist with the projected mana bonus.
            use crate::effects::{
                EarthbendEffect, EffectExecutor, ExecutionContext, ResolvedTarget,
            };
            use crate::target::ChooseSpec;
            let mut context = ExecutionContext::new_default(source, request.payer)
                .with_targets(vec![ResolvedTarget::Object(source)]);
            EarthbendEffect::new(
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control())),
                1,
            )
            .execute(&mut game, &mut context)
            .unwrap();
            assert!(!game.effect_store.delayed_triggers.is_empty());
            game.take_pending_trigger_events();
            game.refresh_continuous_state().unwrap();
            request.cost = ManaCost::new().add_generic(instructions * (1 + copies));
            let analysis = ManaSourceAnalysis::new(&game);
            let choices = collect_activation_choices(&game, &request);
            assert_eq!(
                analysis
                    .project(&choices[0])
                    .expect("fixed triggered bonus projects")
                    .output
                    .green,
                instructions * (1 + copies)
            );
            let inventory = mana_payment_activation_inventory(&game, &request);
            assert_eq!(inventory.len(), 1);
            assert_eq!(inventory[0].expected_mana.green, instructions * (1 + copies));
            let plan = plan_first_mana_payment(&game, &request).expect("triggered mana pays cost");
            assert_eq!(plan.mana_ability_steps.len(), 1);
            assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
            assert!(!game.is_tapped(source));
            execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker)
                .unwrap();
            assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
            assert_eq!(game.player(request.payer).unwrap().life, 20);
            assert!(game.is_tapped(source));
            let mut queue = crate::triggers::TriggerQueue::default();
            crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
            assert_eq!(queue.take_all().len(), instructions as usize, "one ordinary trigger per production instruction");
            crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
            assert!(queue.take_all().is_empty());
        }
    }

    #[test]
    fn activation_event_mana_trigger_is_resolved_by_planner() {
        use crate::mana_payment::{execute_mana_payment_plan, plan_first_mana_payment};
        let (mut game, mut request) = fixture(
            TotalCost::from_cost(Cost::tap()),
            vec![ManaSymbol::Green],
            vec![],
        );
        game.object_mut(request.source)
            .unwrap()
            .abilities_mut()
            .push(Ability::triggered(
                crate::triggers::Trigger::ability_activated(crate::target::ObjectFilter::land()),
                vec![Effect::add_mana(vec![ManaSymbol::Blue; 2])],
            ));
        game.refresh_continuous_state().unwrap();
        let choices = collect_activation_choices(&game, &request);
        let projected = ManaSourceAnalysis::new(&game).project(&choices[0])
            .expect("activation-event mana bonus should use event projection");
        assert_eq!(projected.output.blue, 2);
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Blue], vec![ManaSymbol::Blue]]);
        assert_eq!(
            mana_payment_activation_inventory(&game, &request)[0]
                .expected_mana
                .blue,
            2
        );
        assert!(
            crate::mana_payment::manual_mana_abilities(&game, &request)
                .contains(&(request.source, 0))
        );
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        assert_eq!(plan.mana_ability_steps.len(), 1);
        execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker)
            .unwrap();
        assert_eq!(game.player(request.payer).unwrap().mana_pool.green, 1);
    }

    #[test]
    fn fixed_and_selected_color_projections_match_authoritative_execution() {
        // Check both the source menu and the actual activated result, including
        // explicit color restrictions and multi-mana bundles.
        for (output, effects) in [
            (vec![ManaSymbol::Green, ManaSymbol::Colorless], vec![]),
            (
                vec![],
                vec![Effect::add_mana(vec![ManaSymbol::Blue, ManaSymbol::Red])],
            ),
            (vec![], vec![Effect::add_mana_of_any_color(2)]),
            (vec![], vec![Effect::add_mana_of_any_one_color(3)]),
            (vec![], vec![Effect::new(crate::effects::AddManaOfChosenColorEffect::new(2, PlayerFilter::You))]),
            (vec![], vec![Effect::new(crate::effects::AddManaOfChosenColorEffect::with_fixed_option(2, PlayerFilter::You, Color::Blue))]),
            (vec![], vec![Effect::add_mana_from_commander_color_identity(2)]),
            (
                vec![],
                vec![Effect::add_mana_of_any_color_restricted(
                    2,
                    vec![Color::Red, Color::Green],
                )],
            ),
        ] {
            let (game, request) = fixture(TotalCost::from_cost(Cost::tap()), output, effects);
            let analysis = ManaSourceAnalysis::new(&game);
            let options = mana_payment_activation_inventory(&game, &request);
            let choices = collect_activation_choices(&game, &request);
            assert!(!choices.is_empty());
            assert!(choices.len() >= options.len());
            for choice in choices {
                let projected = analysis.project(&choice).expect("reviewed fixed source");
                let mut staged = game.clone();
                let mut fallback = SelectFirstDecisionMaker;
                let mut recorded = choice.replacement_witnesses.as_ref().map(|records|
                    super::super::witness::WitnessDecisionMaker::new(records, &mut fallback));
                let mut ordinary = SelectFirstDecisionMaker;
                let dm: &mut dyn DecisionMaker = match &mut recorded {
                    Some(recorded) => recorded,
                    None => &mut ordinary,
                };
                crate::special_actions::perform_activate_mana_ability_restricted_colors(
                    &mut staged,
                    request.payer,
                    choice.source,
                    choice.ability_index,
                    choice.color_restriction.clone(),
                    dm,
                )
                .unwrap();
                assert!(recorded.as_ref().is_none_or(|recorded| recorded.complete()));
                assert_eq!(
                    projected.output,
                    staged.player(request.payer).unwrap().mana_pool
                );
                assert!(staged.is_tapped(choice.source));
                assert!(!game.is_tapped(choice.source));
                assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
            }
        }
    }

    #[test]
    fn complex_activation_costs_fall_back_and_do_not_spend_live_resources() {
        for cost in [Cost::life(1), Cost::sacrifice_self()] {
            let (game, request) = fixture(
                TotalCost::from_costs(vec![Cost::tap(), cost]),
                vec![ManaSymbol::Green],
                vec![],
            );
            let analysis = ManaSourceAnalysis::new(&game);
            let choices = collect_activation_choices(&game, &request);
            assert_eq!(choices.len(), 1);
            assert!(analysis.project(&choices[0]).is_none());
            let options = mana_payment_activation_inventory(&game, &request);
            assert_eq!(options.len(), 1);
            assert_eq!(options[0].expected_mana.green, 1);
            assert!(!options[0].repeatable);
            assert_eq!(game.player(request.payer).unwrap().life, 20);
            assert!(game.battlefield.contains(&request.source));
            assert!(!game.is_tapped(request.source));
        }
    }

    #[test]
    fn projection_distinguishes_life_history_from_tap_dependencies() {
        use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
        for case in 0..3 {
            let (mut game, request) = fixture(
                TotalCost::from_cost(Cost::tap()),
                vec![ManaSymbol::Green],
                vec![],
            );
            let target = if case == 2 {
                let mut filter = crate::target::ObjectFilter::land();
                let mut branch = crate::target::ObjectFilter::default();
                branch.tapped = true;
                filter.any_of.push(branch);
                EffectTarget::Filter(filter)
            } else {
                EffectTarget::Source
            };
            let mut effect = ContinuousEffect::new(
                request.source,
                request.payer,
                target,
                Modification::AddAbility(crate::static_abilities::StaticAbility::flying()),
            );
            effect.condition = match case {
                0 => Some(crate::ConditionExpr::ValueComparison {
                    left: Value::LifeLostThisTurn(PlayerFilter::You),
                    operator: ironsmith_core::effect_model::ValueComparisonOperator::GreaterThan,
                    right: Value::Fixed(0),
                }),
                1 => Some(crate::ConditionExpr::SourceIsTapped),
                _ => None,
            };
            game.effect_store.continuous_effects.add_effect(effect);
            game.refresh_continuous_state().unwrap();
            let analysis = ManaSourceAnalysis::new(&game);
            let choices = collect_activation_choices(&game, &request);
            assert_eq!(analysis.project(&choices[0]).is_some(), case == 0);
        }
    }

    #[test]
    fn damage_shield_survives_mana_planning_and_payment_then_prevents_once() {
        use crate::effects::{EffectExecutor, ExecutionContext};
        let (mut game, request) = fixture(
            TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Green], vec![],
        );
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                request.source, request.payer,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_you(),
                crate::replacement::ReplacementAction::PreventDamage,
            ),
        );
        let choices = collect_activation_choices(&game, &request);
        assert_eq!(choices.len(), 1);
        assert_eq!(ManaSourceAnalysis::new(&game).project(&choices[0]).unwrap().output.green, 1);
        let plan = crate::mana_payment::plan_first_mana_payment(&game, &request).unwrap();
        assert!(plan.payable);
        assert!(!game.is_tapped(request.source));
        assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
        assert!(game.effect_store.replacement_effects.is_one_shot(shield));
        crate::mana_payment::execute_mana_payment_plan(
            &mut game, &request, &plan, &mut SelectFirstDecisionMaker,
        ).unwrap();
        assert!(game.is_tapped(request.source));
        assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
        assert!(game.effect_store.replacement_effects.is_one_shot(shield));
        let damage = crate::effects::DealDamageEffect::new(3,
            crate::target::ChooseSpec::Player(crate::target::PlayerFilter::You));
        let mut ctx = ExecutionContext::new_default(request.source, request.payer);
        damage.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(game.player(request.payer).unwrap().life, 20);
        assert!(!game.effect_store.replacement_effects.is_one_shot(shield));
        damage.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(game.player(request.payer).unwrap().life, 17);
    }

    #[test]
    fn planner_selects_and_executes_nondefault_replacement_order() {
        use crate::replacement::{ReplacementAction, ReplacementEffect, EventModification};
        let (mut game, mut request) = fixture(TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Green; 2], vec![]);
        // First legal order produces only one blue. The reverse order produces
        // three blue and is the only way to pay the requested cost.
        for action in [ReplacementAction::Modify(EventModification::Multiply(3)),
            ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue])] {
            game.effect_store.replacement_effects.add_effect(ReplacementEffect::with_matcher(
                request.source, request.payer,
                crate::events::mana::matchers::ManaProducedBySourceMatcher::new(crate::target::ObjectFilter::default()), action));
        }
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Blue]; 3]);
        let plan = crate::mana_payment::plan_first_mana_payment(&game, &request)
            .expect("planner must explore nondefault replacement order");
        assert_eq!(plan.mana_ability_steps.len(), 1);
        assert!(plan.mana_ability_steps[0].replacement_witnesses.is_some());
        assert_eq!(plan.mana_ability_steps[0].expected_mana.blue, 3);
        assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
        crate::mana_payment::execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.is_tapped(request.source));
        assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn unrelated_mana_producing_triggers_do_not_disable_projection() {
        let (mut game, request) = fixture(TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Green], vec![]);
        game.object_mut(request.source).unwrap().abilities_mut().push(Ability::triggered(
            crate::triggers::Trigger::enters_battlefield(crate::target::ObjectFilter::creature(), None),
            vec![Effect::add_mana_of_any_color(2)],
        ));
        game.refresh_continuous_state().unwrap();
        let choices = collect_activation_choices(&game, &request);
        let projected = ManaSourceAnalysis::new(&game).project(&choices[0])
            .expect("an unrelated event subscription cannot affect tap-only mana payment");
        assert_eq!(projected.output.green, 1);
        assert_eq!(projected.output.total(), 1);
    }

    #[test]
    fn targeted_mana_trigger_stays_pending_and_cannot_fund_payment() {
        let (mut game, mut request) = fixture(TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Green], vec![]);
        let mut trigger = Ability::triggered(
            crate::triggers::Trigger::mana_added(PlayerFilter::You),
            vec![Effect::add_mana(vec![ManaSymbol::Red])],
        );
        if let AbilityKind::Triggered(ability) = &mut trigger.kind {
            ability.choices.push(crate::target::ChooseSpec::target(
                crate::target::ChooseSpec::SpecificPlayer(request.payer)));
        }
        game.object_mut(request.source).unwrap().abilities_mut().push(trigger);
        game.refresh_continuous_state().unwrap();
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Red]]);
        assert!(crate::mana_payment::plan_first_mana_payment(&game, &request).is_err());
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Green]]);
        let choices = collect_activation_choices(&game, &request);
        let analysis = ManaSourceAnalysis::new(&game);
        let projected = analysis.project(&choices[0]).expect("ordinary triggers need no immediate simulation");
        assert_eq!(projected.output.green, 1);
        assert_eq!(projected.output.red, 0);
        let plan = crate::mana_payment::plan_first_mana_payment(&game, &request).unwrap();
        crate::mana_payment::execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
        assert_eq!(game.effect_store.pending_trigger_entries.len(), 1);
    }

    #[test]
    fn triggered_production_choices_are_projected_and_replayed() {
        for bonus in [Effect::add_mana_of_any_color(2), Effect::add_mana_of_different_colors(2)] {
            let (mut game, mut request) = fixture(TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Green], vec![]);
            game.object_mut(request.source).unwrap().abilities_mut().push(Ability::triggered(
                crate::triggers::Trigger::player_taps_for_mana(PlayerFilter::You, crate::target::ObjectFilter::land()),
                vec![bonus],
            ));
            game.refresh_continuous_state().unwrap();
            request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::White], vec![ManaSymbol::Blue]]);
            let candidates = super::super::analytic::try_projected_candidates(&game, &request)
                .expect("mixed triggered production should use compact assignment");
            assert_eq!(candidates[0].1.len(), 1);
            let plan = crate::mana_payment::plan_first_mana_payment(&game, &request).unwrap();
            let records = plan.mana_ability_steps[0].replacement_witnesses.as_ref().unwrap();
            assert_eq!(records.len(), 2);
            assert_eq!(records[1].input.len(), 2);
            assert!(records[1].input.contains(&ManaSymbol::White));
            assert!(records[1].input.contains(&ManaSymbol::Blue));
            crate::mana_payment::execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.player(request.payer).unwrap().mana_pool.green, 1);
        }
    }

    #[test]
    fn triggered_output_reads_the_actual_replaced_mana_event() {
        let (mut game, mut request) = fixture(TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Green], vec![]);
        game.object_mut(request.source).unwrap().abilities_mut().push(Ability::triggered(
            crate::triggers::Trigger::player_taps_for_mana(PlayerFilter::You, crate::target::ObjectFilter::land()),
            vec![Effect::new(crate::effects::AddManaOfLandProducedTypesEffect::from_triggering_event(
                1, PlayerFilter::You, crate::target::ObjectFilter::land(), true, true,
            ))],
        ));
        // Only the activation is rewritten; the bonus must learn blue from
        // that rewritten event rather than from the printed green ability.
        game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(
            request.source, request.payer,
            crate::events::mana::matchers::ManaProducedBySourceMatcher::new(crate::target::ObjectFilter::default()),
            crate::replacement::ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue]),
        ));
        game.refresh_continuous_state().unwrap();
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Blue]; 2]);
        let choices = collect_activation_choices(&game, &request);
        let analysis = ManaSourceAnalysis::new(&game);
        assert!(choices.iter().any(|choice| analysis.project(choice).is_some_and(|p| p.output.blue == 2)));
        let plan = crate::mana_payment::plan_first_mana_payment(&game, &request).unwrap();
        crate::mana_payment::execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn planner_replays_mixed_production_choices() {
        for effects in [
            vec![Effect::add_mana_of_any_color(2)],
            vec![Effect::add_mana_of_any_color(1), Effect::add_mana_of_any_color(1)],
        ] {
            let (mut game, mut request) = fixture(TotalCost::from_cost(Cost::tap()), vec![], effects);
            request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::White], vec![ManaSymbol::Blue]]);
            let plan = crate::mana_payment::plan_first_mana_payment(&game, &request)
                .expect("one activation must support independently selected colors");
            assert_eq!(plan.mana_ability_steps.len(), 1);
            assert!(plan.mana_ability_steps[0].replacement_witnesses.is_some());
            assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
            crate::mana_payment::execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker).unwrap();
            assert!(game.is_tapped(request.source));
            assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
        }
    }

    #[test]
    fn planner_respects_same_and_distinct_production_colors() {
        for (effect, symbols, payable) in [
            (Effect::add_mana_of_different_colors(2), vec![ManaSymbol::White, ManaSymbol::Blue], true),
            (Effect::add_mana_of_different_colors(2), vec![ManaSymbol::White, ManaSymbol::White], false),
            (Effect::add_mana_of_any_one_color(2), vec![ManaSymbol::White, ManaSymbol::Blue], false),
            (Effect::add_mana_of_any_one_color(2), vec![ManaSymbol::White, ManaSymbol::White], true),
        ] {
            let (mut game, mut request) = fixture(TotalCost::from_cost(Cost::tap()), vec![], vec![effect]);
            request.cost = ManaCost::from_pips(symbols.into_iter().map(|symbol| vec![symbol]).collect());
            let plan = crate::mana_payment::plan_first_mana_payment(&game, &request);
            assert_eq!(plan.is_ok(), payable);
            if let Ok(plan) = plan {
                crate::mana_payment::execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker).unwrap();
                assert!(game.is_tapped(request.source));
                assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
            }
        }
    }

    #[test]
    fn planner_can_decline_optional_one_shot_and_preserve_it() {
        let (mut game, mut request) = fixture(TotalCost::from_cost(Cost::tap()), vec![ManaSymbol::Green; 2], vec![]);
        let mut replacement = crate::replacement::ReplacementEffect::with_matcher(
            request.source, request.payer,
            crate::events::mana::matchers::ManaProducedBySourceMatcher::new(crate::target::ObjectFilter::default()),
            crate::replacement::ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue]));
        replacement.optional = true;
        let id = game.effect_store.replacement_effects.add_one_shot_effect(replacement);
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Green]; 2]);
        let plan = crate::mana_payment::plan_first_mana_payment(&game, &request).expect("declining replacement pays green cost");
        let witnesses = plan.mana_ability_steps[0].replacement_witnesses.as_ref().unwrap();
        assert!(witnesses.iter().flat_map(|witness| &witness.decisions).any(|decision| !decision.apply));
        crate::mana_payment::execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.effect_store.replacement_effects.get_effect(id).is_some());
        assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn unrelated_entry_replacement_keeps_plain_mana_projection() {
        let (mut game, request) = fixture(
            TotalCost::from_cost(Cost::tap()),
            vec![ManaSymbol::Green],
            vec![],
        );
        game.effect_store.replacement_effects.add_effect(
            crate::replacement::ReplacementEffect::enters_tapped(
                request.source,
                request.payer,
                crate::target::ObjectFilter::land(),
            ),
        );
        let analysis = ManaSourceAnalysis::new(&game);
        let choices = collect_activation_choices(&game, &request);
        assert_eq!(choices.len(), 1);
        assert_eq!(
            analysis
                .project(&choices[0])
                .expect("entry cannot affect mana")
                .output
                .green,
            1
        );
    }

    #[test]
    fn replacement_changes_inventory_output_via_projection() {
        let (mut game, mut request) = fixture(
            TotalCost::from_cost(Cost::tap()),
            vec![ManaSymbol::Green],
            vec![],
        );
        let replacement = crate::replacement::ReplacementEffect::with_matcher(
            request.source,
            request.payer,
            crate::events::mana::matchers::ManaProducedBySourceMatcher::new(
                crate::target::ObjectFilter::default(),
            ),
            crate::replacement::ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue; 2]),
        );
        game.effect_store
            .replacement_effects
            .add_effect(replacement);
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Blue], vec![ManaSymbol::Blue]]);
        let choices = collect_activation_choices(&game, &request);
        assert_eq!(
            ManaSourceAnalysis::new(&game).project(&choices[0])
                .expect("pure replacement is compiled").output.blue, 2,
        );
        let options = mana_payment_activation_inventory(&game, &request);
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].expected_mana.blue, 2);
        assert_eq!(options[0].expected_mana.green, 0);
        assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
        let plan = crate::mana_payment::plan_first_mana_payment(&game, &request)
            .expect("replacement production must not be rejected by printed-output bound");
        crate::mana_payment::execute_mana_payment_plan(
            &mut game,
            &request,
            &plan,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(game.player(request.payer).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn mana_filter_is_measured_after_paying_its_input() {
        let (mut game, request) = fixture(
            TotalCost::from_costs(vec![
                Cost::tap(),
                Cost::mana(ManaCost::new().add_generic(1)),
            ]),
            vec![ManaSymbol::Green; 2],
            vec![],
        );
        game.player_mut(request.payer).unwrap().mana_pool.blue = 1;
        let choices = collect_activation_choices(&game, &request);
        assert_eq!(choices.len(), 1);
        assert!(
            ManaSourceAnalysis::new(&game)
                .project(&choices[0])
                .is_none()
        );
        let options = mana_payment_activation_inventory(&game, &request);
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].expected_mana.green, 2);
        assert_eq!(options[0].expected_mana.blue, 0);
        let mut staged = game.clone();
        crate::special_actions::perform_activate_mana_ability_restricted_colors(
            &mut staged,
            request.payer,
            request.source,
            0,
            None,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(staged.player(request.payer).unwrap().mana_pool.blue, 0);
        assert_eq!(staged.player(request.payer).unwrap().mana_pool.green, 2);
        assert_eq!(game.player(request.payer).unwrap().mana_pool.blue, 1);
        assert!(!game.is_tapped(request.source));
    }

    #[test]
    fn variable_output_is_not_projected_from_its_possible_colors() {
        let (game, request) = fixture(
            TotalCost::from_cost(Cost::tap()),
            vec![],
            vec![Effect::add_mana_of_any_color(Value::X)],
        );
        let choices = collect_activation_choices(&game, &request);
        assert!(!choices.is_empty());
        let analysis = ManaSourceAnalysis::new(&game);
        assert!(
            choices
                .iter()
                .all(|choice| analysis.project(choice).is_none())
        );
    }

    #[derive(Default)]
    struct PromptColors {
        pending: bool,
    }
    impl DecisionMaker for PromptColors {
        fn decide_colors(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::ColorsContext,
        ) -> Vec<Color> {
            self.pending = true;
            vec![]
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    #[test]
    fn deferred_menu_omits_unanswered_colors_but_keeps_exact_choices() {
        let (game, request) = fixture(
            TotalCost::from_cost(Cost::tap()),
            vec![],
            vec![Effect::add_mana_of_any_color(1)],
        );
        let all = mana_payment_activation_inventory(&game, &request);
        assert!(all.iter().any(|option| option.color_restriction.is_none()));
        let ready = mana_payment_ready_activation_inventory(&game, &request, PromptColors::default);
        assert_eq!(ready.len(), Color::ALL.len());
        assert!(ready.iter().all(|option| {
            option
                .color_restriction
                .as_ref()
                .is_some_and(|c| c.len() == 1)
        }));
        for option in ready {
            let mut staged = game.clone();
            let mut dm = PromptColors::default();
            crate::special_actions::perform_activate_mana_ability_restricted_colors(
                &mut staged,
                request.payer,
                option.source,
                option.ability_index,
                option.color_restriction,
                &mut dm,
            )
            .unwrap();
            assert!(!dm.awaiting_choice());
            assert_eq!(
                staged.player(request.payer).unwrap().mana_pool,
                option.expected_mana
            );
        }
    }

    #[test]
    fn distinct_color_bundles_are_measured_without_monochromatic_approximation() {
        let (game, request) = fixture(
            TotalCost::from_cost(Cost::tap()),
            vec![],
            vec![Effect::add_mana_of_different_colors(2)],
        );
        let analysis = ManaSourceAnalysis::new(&game);
        let choices = collect_activation_choices(&game, &request);
        assert_eq!(choices.len(), 20);
        for choice in choices {
            let projected = analysis.project(&choice).expect("distinct colors have exact witnesses");
            assert_eq!(projected.output.total(), 2);
            assert!(pool_units(&projected.output).windows(2).all(|pair| pair[0] != pair[1]));
            assert!(choice.replacement_witnesses.is_some());
        }
        let options = mana_payment_activation_inventory(&game, &request);
        let unrestricted = options
            .iter()
            .find(|option| option.color_restriction.is_none())
            .unwrap();
        assert_eq!(unrestricted.expected_mana.white, 1);
        assert_eq!(unrestricted.expected_mana.blue, 1);
        assert!(!game.is_tapped(request.source));
    }
}

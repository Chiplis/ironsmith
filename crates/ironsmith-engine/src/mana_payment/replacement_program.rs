//! Compiled pure mana replacements and their resource/application history.
use crate::events::{EventKind, ManaAddedEvent};
use crate::game_state::GameState;
use crate::replacement::{ReplacementEffect, ReplacementEffectKey, ReplacementEffectSource};

pub(super) struct CompiledManaReplacements {
    effects: Vec<CompiledReplacement>,
}

struct CompiledReplacement {
    effect: ReplacementEffect,
    once: bool,
    key: ReplacementEffectKey,
}

/// An event-local replacement decision, identified by semantic registration or
/// ability occurrence rather than a transient candidate-list index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplacementDecision {
    pub key: ReplacementEffectKey,
    pub apply: bool,
    pub player: crate::ids::PlayerId,
    pub source: crate::ids::ObjectId,
    pub before: Vec<crate::mana::ManaSymbol>,
    pub color: Option<crate::mana::ManaSymbol>,
}
impl std::hash::Hash for ReplacementDecision {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use std::hash::Hash;
        // Existing deterministic witnesses keep their previous payment IDs.
        if let Some(color) = self.color { "mana-rewrite-color-v1".hash(state); color.hash(state); }
        self.key.hash(state); self.apply.hash(state); self.player.hash(state);
        self.source.hash(state); self.before.hash(state);
    }
}

#[derive(Debug, Clone)]
pub(super) struct ReplacementBranch {
    pub event: ManaAddedEvent,
    pub resources: ReplacementResources,
    pub decisions: Vec<ReplacementDecision>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct ReplacementResources {
    pub consumed: Vec<ReplacementEffectKey>,
}

impl CompiledManaReplacements {
    pub fn compile(game: &GameState) -> Option<Self> {
        let manager = &game.effect_store.replacement_effects;
        let generated_ids: std::collections::HashSet<_> = manager
            .effect_sources_snapshot()
            .into_iter()
            .filter_map(|(id, origin)| {
                (origin == ReplacementEffectSource::StaticAbility).then_some(id)
            })
            .collect();
        // Refresh generated occurrences rather than retaining stale manager
        // copies. Clone only supported relevant effects, never the whole store.
        let generated =
            crate::replacement_ability_processor::generate_replacement_effects_from_abilities(game)
                .ok()?;
        let effects = manager
            .effects()
            .iter()
            .filter(|effect| !generated_ids.contains(&effect.id.0))
            .map(|effect| (effect, manager.is_one_shot(effect.id)))
            .chain(generated.iter().map(|effect| (effect, false)));
        let mut compiled = Vec::new();
        for (effect, once) in effects {
            let matcher = effect.matcher.as_ref()?;
            if [
                EventKind::BecomeTapped,
                EventKind::ManaAdded,
                EventKind::AbilityActivated,
            ]
            .into_iter()
            .all(|kind| !matcher.may_match_event_kind(kind))
            {
                continue;
            }
            // A mana predicate alone says nothing about effects on the tap
            // cost or activation event. Those transitions must also be proven
            // unrelated before a pure production rewrite can stand in for them.
            if matcher.may_match_event_kind(EventKind::BecomeTapped)
                || matcher.may_match_event_kind(EventKind::AbilityActivated)
            {
                return None;
            }
            let predicate = matcher.mana_predicate()?;
            if GameState::filter_reads_tapped_state_or_activation_history(
                predicate.source_filter,
                true,
            ) || (effect.replacement.mana_transformation().is_none()
                && !effect.replacement.needs_mana_color_choice())
            {
                return None;
            }
            compiled.push(CompiledReplacement {
                effect: effect.clone(),
                once,
                key: effect.application_key(),
            });
        }
        Some(Self { effects: compiled })
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    fn candidates(&self, game: &GameState, branch: &ReplacementBranch) -> Option<Vec<usize>> {
        let mut candidates = Vec::new();
        for (index, compiled) in self.effects.iter().enumerate() {
            let effect = &compiled.effect;
            if branch
                .decisions
                .iter()
                .any(|decision| decision.key == compiled.key)
                || (compiled.once && branch.resources.consumed.contains(&compiled.key))
            {
                continue;
            }
            let matcher = effect.matcher.as_ref()?;
            let ctx = game.filter_context_for(effect.controller, Some(effect.source));
            if matcher.mana_predicate()?.matches(&branch.event, game, &ctx) {
                candidates.push((
                    index,
                    effect
                        .priority_override
                        .unwrap_or_else(|| matcher.priority()),
                ));
            }
        }
        let Some(priority) = candidates.iter().map(|(_, priority)| *priority).min() else {
            return Some(vec![]);
        };
        Some(
            candidates
                .into_iter()
                .filter_map(|(index, p)| (p == priority).then_some(index))
                .collect(),
        )
    }

    fn output_choices(&self, game: &GameState, branch: &ReplacementBranch, index: usize)
        -> Option<Vec<Option<crate::mana::ManaSymbol>>> {
        let effect = &self.effects[index].effect;
        if effect.replacement.needs_mana_color_choice() {
            // A compact payer plan cannot commit another player's choice.
            // Preserve unknown so the checked/native owner can request it.
            let crate::replacement::ReplacementAction::RewriteMana { output, .. } = &effect.replacement else { return None; };
            let outputs = crate::events::mana::mana_rewrite_output_choices(*output, &branch.event, game);
            let chooser = if matches!(output, ironsmith_core::ManaRewriteOutput::ByBasicLandType(_)) {
                branch.event.controller
            } else { effect.controller };
            if outputs.len() > 1 && game.controlling_player_for(chooser) != game.controlling_player_for(branch.event.player) {
                return None;
            }
            Some(outputs.into_iter().map(Some).collect())
        } else { effect.replacement.mana_transformation().map(|_| vec![None]) }
    }

    fn advance(&self, branch: &mut ReplacementBranch, index: usize, apply: bool,
        color: Option<crate::mana::ManaSymbol>) -> Option<()> {
        let compiled = &self.effects[index];
        if !apply && (!compiled.effect.optional || color.is_some()) {
            return None;
        }
        branch.decisions.push(ReplacementDecision {
            key: compiled.key.clone(),
            apply,
            player: branch.event.player,
            source: branch.event.source,
            before: branch.event.mana.clone(),
            color,
        });
        if apply {
            branch.event.mana = compiled
                .effect
                .replacement
                .mana_transformation_with_color(color)?
                .apply(&branch.event.mana);
            if compiled.once {
                branch.resources.consumed.push(compiled.key.clone());
            }
        }
        Some(())
    }

    /// Enumerate every legal replacement order and optional decline within the
    /// caller's exploration budget. Exhaustion returns unknown, not a truncated
    /// list that could incorrectly prove an unaffordable payment.
    pub fn branches(
        &self,
        game: &GameState,
        event: ManaAddedEvent,
        resources: &ReplacementResources,
        budget: usize,
    ) -> Option<Vec<ReplacementBranch>> {
        let mut pending = vec![ReplacementBranch {
            event,
            resources: resources.clone(),
            decisions: vec![],
        }];
        let mut complete = Vec::new();
        let mut visited = 0usize;
        while let Some(branch) = pending.pop() {
            visited = visited.checked_add(1)?;
            if visited > budget {
                return None;
            }
            let candidates = self.candidates(game, &branch)?;
            if candidates.is_empty() {
                complete.push(branch);
                continue;
            }
            for index in candidates.into_iter().rev() {
                if self.effects[index].effect.optional {
                    let mut declined = branch.clone();
                    self.advance(&mut declined, index, false, None)?;
                    pending.push(declined);
                }
                for color in self.output_choices(game, &branch, index)?.into_iter().rev() {
                    let mut applied = branch.clone();
                    self.advance(&mut applied, index, true, color)?;
                    pending.push(applied);
                }
            }
        }
        Some(complete)
    }

    /// Validate a selected branch against freshly offered candidates. Every
    /// recorded rewrite must still be legal and the witness must be complete.
    pub fn replay(
        &self,
        game: &GameState,
        event: ManaAddedEvent,
        resources: &ReplacementResources,
        decisions: &[ReplacementDecision],
    ) -> Option<ReplacementBranch> {
        let mut branch = ReplacementBranch {
            event,
            resources: resources.clone(),
            decisions: vec![],
        };
        for decision in decisions {
            if decision.player != branch.event.player
                || decision.source != branch.event.source
                || decision.before != branch.event.mana
            {
                return None;
            }
            let index = self
                .candidates(game, &branch)?
                .into_iter()
                .find(|index| self.effects[*index].key == decision.key)?;
            if decision.apply && !self.output_choices(game, &branch, index)?.contains(&decision.color) { return None; }
            self.advance(&mut branch, index, decision.apply, decision.color)?;
        }
        if !self.candidates(game, &branch)?.is_empty() {
            return None;
        }
        Some(branch)
    }

    /// Fast path for mandatory, unambiguous rewrites. Resource changes are
    /// committed to the compact state only when the complete event succeeds.
    pub fn deterministic_event(
        &self,
        game: &GameState,
        event: ManaAddedEvent,
        resources: &mut ReplacementResources,
    ) -> Option<ManaAddedEvent> {
        let mut branch = ReplacementBranch {
            event,
            resources: resources.clone(),
            decisions: vec![],
        };
        loop {
            let candidates = self.candidates(game, &branch)?;
            let [index] = candidates.as_slice() else {
                if candidates.is_empty() {
                    *resources = branch.resources;
                    return Some(branch.event);
                }
                return None;
            };
            if self.effects[*index].effect.optional {
                return None;
            }
            let choices = self.output_choices(game, &branch, *index)?;
            let [color] = choices.as_slice() else { return None; };
            self.advance(&mut branch, *index, true, *color)?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::replacement::ReplacementAction;

    fn effect(game: &mut GameState, action: ReplacementAction) -> ReplacementEffect {
        let player = PlayerId::from_index(0);
        let card = crate::CardBuilder::new(crate::ids::CardId::new(), "Replacement source")
            .card_types(vec![crate::types::CardType::Land])
            .build();
        let source = game.create_object_from_card(&card, player, crate::Zone::Battlefield);
        ReplacementEffect::with_matcher(
            source,
            player,
            crate::events::mana::matchers::ManaProducedBySourceMatcher::new(
                crate::target::ObjectFilter::default(),
            ),
            action,
        )
    }

    fn event(source: ObjectId) -> ManaAddedEvent {
        let player = PlayerId::from_index(0);
        ManaAddedEvent::new(source, player, player, vec![ManaSymbol::Green; 2])
    }

    #[test]
    fn one_shot_is_consumed_in_compact_state_but_not_live_game() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let replacement = effect(
            &mut game,
            ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue]),
        );
        let source = replacement.source;
        let id = game
            .effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        game.refresh_continuous_state().unwrap();
        let program = CompiledManaReplacements::compile(&game).unwrap();
        let mut resources = ReplacementResources::default();
        assert_eq!(
            program
                .deterministic_event(&game, event(source), &mut resources)
                .unwrap()
                .mana,
            vec![ManaSymbol::Blue]
        );
        assert_eq!(
            program
                .deterministic_event(&game, event(source), &mut resources)
                .unwrap()
                .mana,
            vec![ManaSymbol::Green; 2]
        );
        assert_eq!(resources.consumed.len(), 1);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(id)
                .is_some()
        );
    }

    #[test]
    fn competing_rewrites_require_choice_instead_of_arbitrary_order() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let first = effect(
            &mut game,
            ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue]),
        );
        let source = first.source;
        game.effect_store.replacement_effects.add_effect(first);
        let second = effect(
            &mut game,
            ReplacementAction::Modify(crate::replacement::EventModification::Multiply(3)),
        );
        game.effect_store.replacement_effects.add_effect(second);
        game.refresh_continuous_state().unwrap();
        let program = CompiledManaReplacements::compile(&game).unwrap();
        assert!(
            program
                .deterministic_event(&game, event(source), &mut ReplacementResources::default())
                .is_none()
        );
    }

    #[test]
    fn generated_replacements_are_deduplicated_and_rematched_after_each_rewrite() {
        use crate::static_abilities::StaticAbility;
        use crate::target::ObjectFilter;
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let temporary = effect(&mut game, ReplacementAction::ReplaceManaExact(vec![]));
        let source = temporary.source;
        game.object_mut(source).unwrap().abilities_mut().extend([
            crate::Ability::static_ability(StaticAbility::mana_production_multiplier_replacement(
                ObjectFilter::land(),
                3,
                "Triple production",
            )),
            crate::Ability::static_ability(StaticAbility::mana_production_replacement(
                ObjectFilter::land(),
                2,
                vec![ManaSymbol::Colorless],
                "Cap large production",
            )),
        ]);
        game.refresh_continuous_state().unwrap();
        let program = CompiledManaReplacements::compile(&game).unwrap();
        assert_eq!(
            program.effects.len(),
            2,
            "generated manager copies must not be applied twice"
        );
        let proposal = event(source)
            .with_mana(vec![ManaSymbol::Green])
            .with_production_provenance(
                crate::events::mana::ManaProductionProvenance::TappedSourceForMana,
            );
        let mut resources = ReplacementResources::default();
        assert_eq!(
            program
                .deterministic_event(&game, proposal, &mut resources)
                .unwrap()
                .mana,
            vec![ManaSymbol::Colorless],
            "the cap becomes applicable only after tripling"
        );
        assert!(
            resources.consumed.is_empty(),
            "persistent effects survive the event"
        );
    }

    #[test]
    fn competing_orders_have_replayable_distinct_results() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let first = effect(
            &mut game,
            ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue]),
        );
        let source = first.source;
        game.effect_store.replacement_effects.add_effect(first);
        let second = effect(
            &mut game,
            ReplacementAction::Modify(crate::replacement::EventModification::Multiply(3)),
        );
        game.effect_store.replacement_effects.add_effect(second);
        game.refresh_continuous_state().unwrap();
        let program = CompiledManaReplacements::compile(&game).unwrap();
        let resources = ReplacementResources::default();
        let branches = program
            .branches(&game, event(source), &resources, 16)
            .unwrap();
        let mut amounts = branches
            .iter()
            .map(|branch| branch.event.mana.len())
            .collect::<Vec<_>>();
        amounts.sort_unstable();
        assert_eq!(amounts, vec![1, 3]);
        for branch in branches {
            let replay = program
                .replay(&game, event(source), &resources, &branch.decisions)
                .unwrap();
            assert_eq!(replay.event.mana, branch.event.mana);
            assert_eq!(replay.decisions, branch.decisions);
            assert!(
                program
                    .replay(&game, event(source), &resources, &branch.decisions[..1])
                    .is_none()
            );
            let mut stale = branch.decisions.clone();
            stale[0].before.clear();
            assert!(
                program
                    .replay(&game, event(source), &resources, &stale)
                    .is_none()
            );
        }
        assert!(
            program
                .branches(&game, event(source), &resources, 1)
                .is_none(),
            "incomplete enumeration must never masquerade as complete alternatives"
        );
    }

    #[test]
    fn optional_decline_keeps_one_shot_for_a_later_event() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let mut replacement = effect(
            &mut game,
            ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue]),
        );
        let source = replacement.source;
        replacement.optional = true;
        let id = game
            .effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        game.refresh_continuous_state().unwrap();
        let program = CompiledManaReplacements::compile(&game).unwrap();
        let resources = ReplacementResources::default();
        let branches = program
            .branches(&game, event(source), &resources, 8)
            .unwrap();
        assert_eq!(branches.len(), 2);
        let applied = branches
            .iter()
            .find(|branch| branch.decisions[0].apply)
            .unwrap();
        let declined = branches
            .iter()
            .find(|branch| !branch.decisions[0].apply)
            .unwrap();
        assert_eq!(applied.resources.consumed.len(), 1);
        assert!(declined.resources.consumed.is_empty());
        assert_eq!(
            program
                .branches(&game, event(source), &declined.resources, 8)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            program
                .branches(&game, event(source), &applied.resources, 8)
                .unwrap()
                .len(),
            1
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(id)
                .is_some()
        );
    }

    #[test]
    fn optional_rewrite_is_not_silently_forced() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let mut replacement = effect(
            &mut game,
            ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue]),
        );
        let source = replacement.source;
        replacement.optional = true;
        game.effect_store
            .replacement_effects
            .add_effect(replacement);
        game.refresh_continuous_state().unwrap();
        let program = CompiledManaReplacements::compile(&game).unwrap();
        assert!(
            program
                .deterministic_event(&game, event(source), &mut ReplacementResources::default())
                .is_none()
        );
    }
}

#[cfg(test)]
mod typed_output_rewrite_tests {
    use super::*;
    use crate::mana::ManaSymbol;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    fn setup(output: ironsmith_core::ManaRewriteOutput, quantity: ironsmith_core::ManaRewriteQuantity)
        -> (GameState, ManaAddedEvent) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let player = crate::ids::PlayerId(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Dual producer")
            .card_types(vec![crate::types::CardType::Land])
            .subtypes(vec![crate::types::Subtype::Plains, crate::types::Subtype::Forest]).build();
        let source = game.create_object_from_definition(&definition, player, crate::Zone::Battlefield);
        let rule = ironsmith_core::ManaOutputRewrite {source_filter: ObjectFilter::land(), controller: None,
            tapped_for_mana: true, input: ironsmith_core::ManaRewriteInput::Any, output, quantity};
        game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(source, player,
            crate::events::mana::matchers::ManaRewriteMatcher {rule: rule.clone()},
            ReplacementAction::RewriteMana {input: rule.input, output, quantity}));
        let event = ManaAddedEvent::new(source, player, player, vec![ManaSymbol::Green; 2])
            .with_production_provenance(crate::events::mana::ManaProductionProvenance::TappedSourceForMana);
        (game, event)
    }
    #[test]
    fn selected_color_is_part_of_the_witness_and_unavailable_colors_fail_replay() {
        let output = ironsmith_core::ManaRewriteOutput::ByBasicLandType([
            Some(ManaSymbol::Red), None, None, None, Some(ManaSymbol::Black)]);
        let (game, event) = setup(output, ironsmith_core::ManaRewriteQuantity::Preserve);
        let program = CompiledManaReplacements::compile(&game).unwrap();
        let branches = program.branches(&game, event.clone(), &ReplacementResources::default(), 16).unwrap();
        assert_eq!(branches.len(), 2);
        for branch in branches {
            assert_eq!(branch.decisions.len(), 1, "multiple basic types are one replacement occurrence");
            let selected = branch.decisions[0].color.unwrap();
            assert!([ManaSymbol::Red, ManaSymbol::Black].contains(&selected));
            assert_eq!(branch.event.mana, vec![selected; 2]);
            assert_eq!(program.replay(&game, event.clone(), &ReplacementResources::default(), &branch.decisions).unwrap().event.mana, branch.event.mana);
            let mut corrupt = branch.decisions.clone(); corrupt[0].color = Some(ManaSymbol::Blue);
            assert!(program.replay(&game, event.clone(), &ReplacementResources::default(), &corrupt).is_none());
        }
    }
    #[test]
    fn exact_rewrite_and_multiplier_preserve_both_noncommuting_orders() {
        let (mut game, event) = setup(ironsmith_core::ManaRewriteOutput::Symbol(ManaSymbol::Black), ironsmith_core::ManaRewriteQuantity::Exact(1));
        game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(event.source, event.controller,
            crate::events::mana::matchers::ManaProducedBySourceMatcher::tapped_source_for_mana(ObjectFilter::land()),
            ReplacementAction::Modify(crate::replacement::EventModification::Multiply(3))));
        let program = CompiledManaReplacements::compile(&game).unwrap();
        let branches = program.branches(&game, event, &ReplacementResources::default(), 16).unwrap();
        let mut counts: Vec<_> = branches.iter().map(|branch| branch.event.mana.len()).collect(); counts.sort(); counts.dedup();
        assert_eq!(counts, vec![1, 3]);
        assert!(branches.iter().all(|branch| branch.event.mana.iter().all(|mana| *mana == ManaSymbol::Black)));
    }
    #[test]
    fn compact_payer_plans_do_not_choose_another_players_replacement_color() {
        let (game, mut event) = setup(ironsmith_core::ManaRewriteOutput::ChooseColor, ironsmith_core::ManaRewriteQuantity::Preserve);
        event.player = crate::ids::PlayerId(1);
        let program = CompiledManaReplacements::compile(&game).unwrap();
        assert!(program.branches(&game, event, &ReplacementResources::default(), 16).is_none(),
            "a required external decision is unknown, not an empty set of payable outputs");
    }
    #[test]
    fn selected_output_choices_share_the_existing_exploration_budget() {
        let (game, event) = setup(ironsmith_core::ManaRewriteOutput::ChooseColor, ironsmith_core::ManaRewriteQuantity::Preserve);
        let program = CompiledManaReplacements::compile(&game).unwrap();
        assert!(program.branches(&game, event.clone(), &ReplacementResources::default(), 2).is_none());
        assert_eq!(program.branches(&game, event, &ReplacementResources::default(), 16).unwrap().len(), 5);
    }
}

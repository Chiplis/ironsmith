//! Resolution forecasts: what a spell or ability on the stack would do if it
//! resolved right now, read from its instructions without executing them.

use crate::filter::{FilterContext, ObjectFilter, ObjectFilterExt as _, PlayerFilterExt as _};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::ChooseSpec;
use crate::zone::Zone;
use ironsmith_core::filter_model::WouldActOn;

/// Whether a stack object's resolution, if it resolved right now, would
/// perform `would.action` on an object matching `would.object` ("counter
/// target spell if it would destroy a land you control", Equinox). Its
/// instructions are read against the current game: conditions are evaluated,
/// the announced modes are followed, and "for each" instructions visit the
/// objects they would visit. An action that happens only if a player makes a
/// choice (a "may", an "unless ... pays", a vote) does not count, unless the
/// choice cannot avoid it: life payments that can't cover every action they
/// would prevent leave the rest forced. `would.object` is matched from the
/// asking effect's perspective (`ctx`: "a land you control").
pub(crate) fn stack_object_would_act_on(
    game: &GameState,
    object_id: ObjectId,
    would: &WouldActOn,
    ctx: &FilterContext,
) -> bool {
    game.stack
        .iter()
        .filter(|entry| entry.object_id == object_id || entry.target_id() == object_id)
        .any(|entry| {
            let program = if entry.is_ability {
                entry.ability_effects.clone()
            } else {
                // An unavailable program is propagated through the checked-action
                // latch.
                crate::filter::checked_spell_program_for_filter(game, entry.object_id).ok()
            };
            let Some(program) = program else {
                return false;
            };
            let mut decisions = crate::decision::SelectFirstDecisionMaker;
            let Ok(exec) =
                crate::game_loop::stack_entry_execution_context(game, entry, &mut decisions)
            else {
                return false;
            };
            let mut exec = exec.with_targets(
                entry
                    .targets
                    .iter()
                    .map(|target| match target {
                        crate::game_state::Target::Object(id) => {
                            crate::effects::ResolvedTarget::Object(*id)
                        }
                        crate::game_state::Target::Player(id) => {
                            crate::effects::ResolvedTarget::Player(*id)
                        }
                    })
                    .collect(),
            );
            let forecast = Forecast {
                game,
                entry,
                spell_ctx: game.filter_context_for(entry.controller, Some(entry.object_id)),
                would,
                ctx,
                choices: Default::default(),
            };
            let mut verdict = Verdict::default();
            for effect in program.all_effects() {
                forecast.visit(effect, &mut exec, &mut verdict);
            }
            verdict.is_forced(game)
        })
}

/// An action that a payer set could prevent by paying `life` life.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct LifeRansom {
    payers: Vec<crate::ids::PlayerId>,
    life: u32,
}

#[derive(Debug, Default)]
struct Verdict {
    /// The action happens to a matching object whatever choices are made.
    forced: bool,
    /// Matching actions each preventable by one life payment.
    life_ransoms: std::collections::BTreeMap<LifeRansom, usize>,
}

impl Verdict {
    /// Payments of the same ransom share each payer's life total (CR 119.4:
    /// a player can pay life only if their life total is at least that much),
    /// so when more actions hang on that ransom than the payers can
    /// afford, at least one of them happens.
    fn is_forced(&self, game: &GameState) -> bool {
        self.forced
            || self.life_ransoms.iter().any(|(ransom, &count)| {
                if ransom.life == 0 {
                    return false;
                }
                let affordable: usize = ransom
                    .payers
                    .iter()
                    .filter(|payer| {
                        game.player(**payer).is_some_and(|player| player.is_in_game())
                            && game.can_pay_life_with_reason(
                                **payer,
                                ransom.life,
                                crate::costs::PaymentReason::Effect,
                            )
                    })
                    .filter_map(|payer| game.player(*payer))
                    .map(|player| (player.life.max(0) as u32 / ransom.life) as usize)
                    .fold(0usize, usize::saturating_add);
                count > affordable
            })
    }
}

struct Forecast<'a> {
    game: &'a GameState,
    entry: &'a crate::game_state::StackEntry,
    spell_ctx: FilterContext,
    would: &'a WouldActOn,
    ctx: &'a FilterContext,
    /// Choices earlier instructions would make ("each player chooses a
    /// land"), by the tag later instructions name them with ("sacrifices
    /// the chosen land").
    choices: std::cell::RefCell<std::collections::HashMap<crate::tag::TagKey, RecordedChoice>>,
}

/// A choice of objects a player makes as the stack object resolves.
#[derive(Debug, Clone)]
struct RecordedChoice {
    candidates: Vec<ObjectId>,
    /// How many of the candidates must be chosen.
    required: usize,
}

impl Forecast<'_> {
    fn visit_all(
        &self,
        effects: &[crate::effect::Effect],
        exec: &mut crate::effects::ExecutionContext<'_>,
        verdict: &mut Verdict,
    ) {
        for effect in effects {
            self.visit(effect, exec, verdict);
        }
    }

    fn visit(
        &self,
        effect: &crate::effect::Effect,
        exec: &mut crate::effects::ExecutionContext<'_>,
        verdict: &mut Verdict,
    ) {
        use crate::effects as fx;
        if effect.0.result_action() == Some(self.would.action)
            && let Some(spec) = effect.0.result_action_object_spec()
        {
            verdict.forced |= self.spec_acts_on_matching(spec, exec);
        } else if effect.0.result_action() == Some(self.would.action)
            && let Some(chosen) = effect.0.result_action_chosen_permanents()
        {
            verdict.forced |= self.chosen_permanents_include_matching(chosen, exec);
        } else if let Some(choose) = effect.downcast_ref::<fx::ChooseObjectsEffect>() {
            self.record_choice(choose, exec);
        } else if let Some(tagged) = effect.downcast_ref::<fx::TaggedEffect>() {
            self.visit(&tagged.effect, exec, verdict);
        } else if let Some(conditional) = effect.downcast_ref::<fx::ConditionalEffect>() {
            // CR 608.2c: the branch the instruction would take right now. A
            // condition that can't be read before resolution (it names what
            // an earlier instruction did) takes neither branch.
            match crate::condition_eval::evaluate_condition_resolution(
                self.game,
                &conditional.condition,
                exec,
            ) {
                Ok(true) => self.visit_all(&conditional.if_true, exec, verdict),
                Ok(false) => self.visit_all(&conditional.if_false, exec, verdict),
                Err(_) => {}
            }
        } else if let Some(each) = effect.downcast_ref::<fx::ForEachObject>() {
            let previous = exec.iteration.iterated_object;
            for (object, _) in
                fx::composition::for_each_object_matching_objects(each, self.game, exec)
            {
                exec.iteration.iterated_object = Some(object);
                self.visit_all(&each.effects, exec, verdict);
            }
            exec.iteration.iterated_object = previous;
        } else if let Some(each) = effect.downcast_ref::<fx::ForPlayersEffect>() {
            let Ok(players) = each.selected_players(self.game, exec) else {
                return;
            };
            let previous = exec.iteration.iterated_player;
            for player in players {
                exec.iteration.iterated_player = Some(player);
                self.visit_all(&each.effects, exec, verdict);
            }
            exec.iteration.iterated_player = previous;
        } else if let Some(modal) = effect.downcast_ref::<fx::ChooseModeEffect>() {
            self.visit_all(&modal.common_prefix_effects, exec, verdict);
            let announced = modal
                .chooser
                .is_none()
                .then(|| modal.announced_modes.clone().or_else(|| exec.chosen_modes.clone()))
                .flatten();
            match announced {
                Some(chosen) => {
                    for index in chosen {
                        if let Some(mode) = modal.modes.get(index) {
                            self.visit_all(&mode.effects, exec, verdict);
                        }
                    }
                }
                // Modes chosen as it resolves: forced only if every mode is.
                None => {
                    verdict.forced |= !modal.modes.is_empty()
                        && modal.modes.iter().all(|mode| {
                            let mut inner = Verdict::default();
                            self.visit_all(&mode.effects, exec, &mut inner);
                            inner.is_forced(self.game)
                        });
                }
            }
        } else if let Some(unless) = effect.downcast_ref::<fx::UnlessPaysEffect>() {
            let mut inner = Verdict::default();
            self.visit_all(&unless.effects, exec, &mut inner);
            if !inner.forced {
                return;
            }
            // Only a life payment's affordability is judged here; any other
            // price is a choice that could prevent the action.
            let life = unless
                .cost
                .as_all()
                .and_then(|costs| match costs {
                    [cost] => cost.life_amount(),
                    _ => None,
                });
            let Some(life) = life else {
                return;
            };
            let Some(mut payers) = self.unless_payers(&unless.player, exec) else {
                return;
            };
            payers.sort();
            payers.dedup();
            let tally = verdict
                .life_ransoms
                .entry(LifeRansom { payers, life })
                .or_default();
            *tally = tally.saturating_add(1);
        } else {
            match effect.0.forecast_children() {
                fx::ForecastChildren::Immediate => {
                    effect.visit_child_effects(&mut |child| self.visit(child, exec, verdict));
                }
                fx::ForecastChildren::Chosen | fx::ForecastChildren::Deferred => {}
            }
        }
    }

    fn unless_payers(
        &self,
        payer: &crate::target::PlayerFilter,
        exec: &crate::effects::ExecutionContext<'_>,
    ) -> Option<Vec<crate::ids::PlayerId>> {
        use crate::target::PlayerFilter;
        let in_game = || {
            self.game
                .players
                .iter()
                .filter(|player| player.is_in_game())
                .map(|player| player.id)
        };
        match payer {
            PlayerFilter::Any => Some(in_game().collect()),
            PlayerFilter::Opponent => {
                let filter_ctx = exec.filter_context(self.game);
                Some(in_game().filter(|player| payer.matches_player(*player, &filter_ctx)).collect())
            }
            _ => crate::effects::helpers::resolve_player_filter(self.game, payer, exec)
                .ok()
                .map(|player| vec![player]),
        }
    }

    /// Whether performing the action on `object` is what the predicate asks
    /// about: the object matches, and the player who would perform it is the
    /// required actor ("cause you to sacrifice a land"). A permanent is
    /// sacrificed by its controller (CR 701.21a); any other action is
    /// performed by the stack object's controller (CR 608.2c).
    fn acts_on(&self, object: &crate::object::Object) -> bool {
        if !self.would.object.matches(object, self.ctx, self.game) {
            return false;
        }
        let Some(actor) = &self.would.actor else {
            return true;
        };
        let performer = if self.would.action == crate::effect::PriorEffectAction::Sacrificed {
            self.game.controller_of(object)
        } else {
            self.entry.controller
        };
        actor.matches_player(performer, self.ctx)
    }

    /// Remembers which objects a choice picks among, when they are public
    /// objects in one zone (a search of a hidden zone isn't forecast).
    fn record_choice(
        &self,
        choose: &crate::effects::ChooseObjectsEffect,
        exec: &crate::effects::ExecutionContext<'_>,
    ) {
        let game = self.game;
        let zone = choose.zone.or(choose.filter.zone).unwrap_or(Zone::Battlefield);
        let public = matches!(zone, Zone::Battlefield | Zone::Graveyard | Zone::Exile | Zone::Stack);
        if choose.is_search || !choose.additional_zones.is_empty() || !public {
            self.choices.borrow_mut().remove(&choose.tag);
            return;
        }
        let minimum = match &choose.count_value {
            Some(value) => match crate::effects::helpers::resolve_value(game, value, exec) {
                Ok(count) => count.max(0) as usize,
                Err(_) => {
                    self.choices.borrow_mut().remove(&choose.tag);
                    return;
                }
            },
            None => choose.count.min,
        };
        let filter_ctx = exec.filter_context(game);
        let candidates: Vec<ObjectId> = game
            .zone_ids(zone)
            .into_iter()
            .filter(|id| {
                game.object(*id)
                    .is_some_and(|object| choose.filter.matches(object, &filter_ctx, game))
            })
            .collect();
        let required = minimum.min(candidates.len());
        self.choices
            .borrow_mut()
            .insert(choose.tag.clone(), RecordedChoice { candidates, required });
    }

    /// The recorded choice a filter names ("the chosen land").
    fn recorded_choice_named_by(&self, filter: &ObjectFilter) -> Option<RecordedChoice> {
        let choices = self.choices.borrow();
        filter
            .tagged_constraints
            .iter()
            .filter(|constraint| {
                constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
            })
            .find_map(|constraint| choices.get(&constraint.tag).cloned())
    }

    /// Whether every way of making the choice picks a matching object.
    fn choice_includes_matching(&self, choice: &RecordedChoice) -> bool {
        let avoidable = choice
            .candidates
            .iter()
            .filter(|id| {
                !self
                    .game
                    .object(**id)
                    .is_some_and(|object| self.acts_on(object))
            })
            .count();
        choice.required > avoidable
    }

    /// Whether every choice a chooser could make includes a matching
    /// permanent: they must choose more permanents than they control that
    /// don't match ("each player sacrifices two permanents" when only one of
    /// yours isn't a land).
    fn chosen_permanents_include_matching(
        &self,
        chosen: crate::effects::ChosenPermanents<'_>,
        exec: &mut crate::effects::ExecutionContext<'_>,
    ) -> bool {
        if let Some(choice) = self.recorded_choice_named_by(chosen.filter) {
            return self.choice_includes_matching(&choice);
        }
        let game = self.game;
        let Ok(count) = crate::effects::helpers::resolve_value(game, chosen.count, exec) else {
            return false;
        };
        let count = count.max(0) as usize;
        if count == 0 {
            return false;
        }
        let choosers: Vec<crate::ids::PlayerId> = if chosen.each {
            let filter_ctx = exec.filter_context(game);
            game.players
                .iter()
                .filter(|player| player.is_in_game())
                .map(|player| player.id)
                .filter(|player| chosen.chooser.matches_player(*player, &filter_ctx))
                .collect()
        } else {
            match crate::effects::helpers::resolve_player_filter(game, chosen.chooser, exec) {
                Ok(player) => vec![player],
                Err(_) => return false,
            }
        };
        let previous = exec.iteration.iterated_player;
        let forced = choosers.into_iter().any(|chooser| {
            if chosen.each {
                exec.iteration.iterated_player = Some(chooser);
            }
            let filter_ctx = exec.filter_context(game);
            let (mut matching, mut avoidable) = (0usize, 0usize);
            for &id in &game.battlefield {
                let Some(object) = game.object(id) else {
                    continue;
                };
                if game.controller_of(object) != chooser
                    || !chosen.filter.matches(object, &filter_ctx, game)
                    || (self.would.action == crate::effect::PriorEffectAction::Sacrificed
                        && !game.can_be_sacrificed_with_cause(id, &exec.cause))
                {
                    continue;
                }
                if self.acts_on(object) {
                    matching += 1;
                } else {
                    avoidable += 1;
                }
            }
            count.min(matching + avoidable) > avoidable
        });
        exec.iteration.iterated_player = previous;
        forced
    }

    fn spec_acts_on_matching(
        &self,
        spec: &ChooseSpec,
        exec: &crate::effects::ExecutionContext<'_>,
    ) -> bool {
        let game = self.game;
        let matches = |id: crate::ids::ObjectId, candidate: Option<&ObjectFilter>| {
            game.object(id).is_some_and(|object| {
                candidate.is_none_or(|filter| filter.matches(object, &self.spell_ctx, game))
                    && self.acts_on(object)
            })
        };
        let chosen_targets = |candidate: Option<&ObjectFilter>| {
            self.entry.targets.iter().any(|target| match target {
                crate::game_state::Target::Object(id) => matches(*id, candidate),
                crate::game_state::Target::Player(_) => false,
            })
        };
        match spec {
            ChooseSpec::SurfaceHinted { spec, .. }
            | ChooseSpec::WithCount(spec, _)
            | ChooseSpec::WithCountValue(spec, _, _) => self.spec_acts_on_matching(spec, exec),
            ChooseSpec::Target(inner) => match inner.base() {
                ChooseSpec::Object(filter) => chosen_targets(Some(filter)),
                _ => chosen_targets(None),
            },
            ChooseSpec::All(filter) => game.zone_ids(Zone::Battlefield).into_iter().any(|id| {
                game.object(id).is_some_and(|object| {
                    filter.matches(object, &self.spell_ctx, game)
                        && self.acts_on(object)
                })
            }),
            // "Destroy that land" inside a "for each" names the object the
            // iteration is visiting.
            ChooseSpec::Iterated => match exec.iteration.iterated_object {
                Some(object) => matches(object, None),
                None => chosen_targets(None),
            },
            // A back-reference ("destroy it") names an object the stack
            // object chose, normally one of its targets.
            ChooseSpec::Tagged(tag) => match self.choices.borrow().get(tag).cloned() {
                Some(choice) => self.choice_includes_matching(&choice),
                None => chosen_targets(None),
            },
            ChooseSpec::Object(filter) => match self.recorded_choice_named_by(filter) {
                Some(choice) => self.choice_includes_matching(&choice),
                None => chosen_targets(None),
            },
            _ => false,
        }
    }
}

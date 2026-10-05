//! Deterministic execution smoke tests. `executed` means a bounded fixture
//! reached resolution, never that the card's rules semantics were proved.

#[path = "execution_fixtures.rs"]
mod fixtures;

use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::*;
use ironsmith::game_loop::{
    extract_target_spec, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{StackEntry, Target};
use ironsmith::triggers::{TriggerContext, TriggerQueue, check_triggers, compute_trigger_identity};
use ironsmith::{Effect, GameState, ObjectId, PlayerId, Zone};
use serde::{Deserialize, Serialize};
use std::panic::{AssertUnwindSafe, catch_unwind};

pub(super) fn create_fixture_object(
    game: &mut GameState,
    definition: &CardDefinition,
    owner: PlayerId,
    zone: Zone,
) -> ObjectId {
    // Audit fixture definitions must not reuse an identity for different
    // printed metadata. The engine can safely replace its shared-handle cache,
    // so validate the fixture before that replacement hides the collision.
    if let Some(existing) = game.retained_card_definition(definition.card.id) {
        assert!(
            existing.card.name == definition.card.name
                && existing.card.card_types == definition.card.card_types,
            "runtime audit fixture identity collision: requested {} but retained metadata is {}",
            definition.name(),
            existing.name(),
        );
    }
    let id = game.create_object_from_definition(definition, owner, zone);
    let object = game
        .object(id)
        .expect("runtime audit fixture object exists");
    assert!(
        object.name == definition.card.name && object.card_types == definition.card.card_types,
        "runtime audit fixture identity collision: requested {} but shared object metadata is {}",
        definition.name(),
        object.name
    );
    id
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionObservation {
    pub ability_path: String,
    pub scenario: String,
    pub status: String,
    pub detail: String,
}

const MAX_DECISIONS: usize = 64;
const MAX_MATCHED_SCENARIOS: usize = 24;
const LIMITATIONS: &str = "synthetic event/state; isolated printed ability; sampled choices; follow-up triggers and delayed abilities not drained; no semantic oracle";

fn observation(
    path: &str,
    scenario: &str,
    status: &str,
    detail: impl Into<String>,
) -> ExecutionObservation {
    ExecutionObservation {
        ability_path: path.to_owned(),
        scenario: scenario.to_owned(),
        status: status.to_owned(),
        detail: detail.into(),
    }
}

#[derive(Default)]
struct AuditDecisions {
    accept_optional: bool,
    decisions: usize,
    optional: usize,
    declined: usize,
    unavailable: bool,
    restricted: bool,
}

impl AuditDecisions {
    fn tick(&mut self) {
        self.decisions += 1;
        assert!(
            self.decisions <= MAX_DECISIONS,
            "runtime audit decision budget exhausted"
        );
    }

    fn summary(&self) -> String {
        format!(
            "decisions={}, optional={}, declined={}, constrained_choices={}, seed={}; {LIMITATIONS}",
            self.decisions,
            self.optional,
            self.declined,
            self.restricted,
            fixtures::SEED
        )
    }
}

impl DecisionMaker for AuditDecisions {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.tick();
        self.optional += 1;
        if !self.accept_optional {
            self.declined += 1;
        }
        self.accept_optional
    }

    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        self.tick();
        self.restricted |= ctx.min != ctx.max;
        if self.accept_optional {
            ctx.max.min(3).max(ctx.min)
        } else {
            ctx.min
        }
    }

    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.tick();
        let count = if self.accept_optional {
            ctx.min.max(1)
        } else {
            ctx.min
        };
        let selected: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .take(count.min(ctx.max.unwrap_or(count)))
            .map(|candidate| candidate.id)
            .collect();
        self.unavailable |= selected.len() < ctx.min && !ctx.allow_partial_completion;
        self.restricted |=
            ctx.candidates.len() > selected.len() || ctx.aggregate_constraint.is_some();
        if ctx.min == 0 {
            self.optional += 1;
            self.declined += usize::from(selected.is_empty());
        }
        selected
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.tick();
        let wanted = if self.accept_optional {
            ctx.min.max(1)
        } else {
            ctx.min
        };
        let mut total = 0;
        let mut selected = Vec::new();
        for option in ctx.options.iter().filter(|option| option.legal) {
            if total >= wanted {
                break;
            }
            let cost = option.point_cost.max(1) as usize;
            if total + cost <= ctx.max {
                selected.push(option.index);
                total += cost;
            }
        }
        self.unavailable |= total < ctx.min;
        self.restricted |= ctx.options.len() > selected.len();
        if ctx.min == 0 {
            self.optional += 1;
            self.declined += usize::from(selected.is_empty());
        }
        selected
    }

    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.tick();
        self.unavailable |= ctx
            .requirements
            .iter()
            .any(|requirement| requirement.legal_targets.len() < requirement.min_targets);
        self.restricted |= ctx
            .requirements
            .iter()
            .any(|requirement| requirement.legal_targets.len() > requirement.min_targets);
        SelectFirstDecisionMaker.decide_targets(game, ctx)
    }

    fn decide_order(&mut self, game: &GameState, ctx: &OrderContext) -> Vec<ObjectId> {
        self.tick();
        self.restricted = true;
        SelectFirstDecisionMaker.decide_order(game, ctx)
    }

    fn decide_partition(&mut self, game: &GameState, ctx: &PartitionContext) -> Vec<ObjectId> {
        self.tick();
        self.restricted = true;
        SelectFirstDecisionMaker.decide_partition(game, ctx)
    }

    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &ProliferateContext,
    ) -> ironsmith::decisions::specs::ProliferateResponse {
        self.tick();
        self.optional += 1;
        self.restricted = true;
        let response = if self.accept_optional {
            SelectFirstDecisionMaker.decide_proliferate(game, ctx)
        } else {
            Default::default()
        };
        self.declined += usize::from(response.permanents.is_empty() && response.players.is_empty());
        response
    }

    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &ColorsContext,
    ) -> Vec<ironsmith::color::Color> {
        self.tick();
        self.restricted = true;
        SelectFirstDecisionMaker.decide_colors(game, ctx)
    }

    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &CountersContext,
    ) -> Vec<(ironsmith::object::CounterType, u32)> {
        self.tick();
        self.restricted = true;
        SelectFirstDecisionMaker.decide_counters(game, ctx)
    }

    fn decide_distribute(
        &mut self,
        _game: &GameState,
        _ctx: &DistributeContext,
    ) -> Vec<(Target, u32)> {
        self.tick();
        self.unavailable = true;
        Vec::new()
    }

    fn decide_text(&mut self, game: &GameState, ctx: &TextInputContext) -> String {
        self.tick();
        self.restricted = true;
        SelectFirstDecisionMaker.decide_text(game, ctx)
    }
}

fn panic_text(panic: Box<dyn std::any::Any + Send>) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|text| (*text).to_owned()))
        .unwrap_or_else(|| "non-string panic".to_owned())
}

pub(super) fn guarded(
    path: &str,
    scenario: &str,
    run: impl FnOnce() -> ExecutionObservation,
) -> ExecutionObservation {
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(result) => result,
        Err(panic) => {
            let message = panic_text(panic);
            let status = if message.contains("runtime audit fixture") {
                "fixture_invalid"
            } else if message.contains("runtime audit decision budget") {
                "budget_exceeded"
            } else {
                "panicked"
            };
            observation(path, scenario, status, message)
        }
    }
}

fn finish_resolution(
    game: &mut GameState,
    dm: &mut AuditDecisions,
    path: &str,
    scenario: &str,
) -> ExecutionObservation {
    if game.stack.len() != 1 {
        return observation(
            path,
            scenario,
            "not_exercised",
            format!(
                "expected one isolated stack entry, found {}; {}",
                game.stack.len(),
                dm.summary()
            ),
        );
    }
    let result = resolve_stack_entry_with(game, dm);
    if let Some(problem) = zone_invariant_failure(game) {
        return observation(
            path,
            scenario,
            "invariant_failed",
            format!("{problem}; resolution={result:?}; {}", dm.summary()),
        );
    }
    if dm.unavailable {
        return observation(
            path,
            scenario,
            "needs_fixture",
            format!(
                "mandatory choice unavailable or unsupported distribution; result={result:?}; {}",
                dm.summary()
            ),
        );
    }
    match result {
        Err(error) => observation(
            path,
            scenario,
            "resolution_failed",
            format!("{error}; {}", dm.summary()),
        ),
        Ok(()) if dm.awaiting_choice() => {
            observation(path, scenario, "decision_required", dm.summary())
        }
        Ok(()) if dm.declined > 0 => observation(path, scenario, "optional_skipped", dm.summary()),
        Ok(()) => observation(path, scenario, "executed", dm.summary()),
    }
}

pub(super) fn zone_invariant_failure(game: &GameState) -> Option<String> {
    let mut visited = std::collections::HashSet::new();
    let mut check = |zone: Zone, ids: Vec<ObjectId>| -> Option<String> {
        for id in ids {
            let Some(object) = game.object(id) else {
                return Some(format!("{zone:?} contains missing object {id:?}"));
            };
            if object.zone != zone {
                return Some(format!(
                    "{zone:?} contains {id:?}, whose zone is {:?}",
                    object.zone
                ));
            }
            if !visited.insert(id) {
                return Some(format!(
                    "object {id:?} appears more than once across zone indexes"
                ));
            }
        }
        None
    };
    for (zone, objects) in [
        (Zone::Battlefield, &game.battlefield),
        (Zone::Exile, &game.exile),
        (Zone::Command, &game.command_zone),
    ] {
        if let Some(problem) = check(zone, objects.iter().copied().collect()) {
            return Some(problem);
        }
    }
    for player in game.players.iter() {
        for (zone, objects) in [
            (Zone::Hand, &player.hand),
            (Zone::Library, &player.library),
            (Zone::Graveyard, &player.graveyard),
        ] {
            if let Some(problem) = check(zone, objects.iter().copied().collect()) {
                return Some(problem);
            }
        }
    }
    None
}

fn effect_has_target(effect: &Effect) -> bool {
    if extract_target_spec(effect).is_some() {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| found |= effect_has_target(child));
    found
}

/// Audit actual compiled resolution programs against bounded, deterministic
/// fixtures. Failures are candidates for reproduction, not rules verdicts.
pub fn audit(definition: &CardDefinition) -> Vec<ExecutionObservation> {
    let mut observations = Vec::new();
    for (index, ability) in definition.abilities.iter().enumerate() {
        let path = format!("abilities[{index}]");
        match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                let mut isolated = definition.clone();
                isolated.abilities = vec![ability.clone()];
                isolated.spell_effect = None;
                let zone = ability
                    .functional_zones
                    .first()
                    .copied()
                    .unwrap_or(Zone::Battlefield);
                let setup = catch_unwind(AssertUnwindSafe(|| fixtures::seed(&isolated, zone)));
                let seed = match setup {
                    Ok(seed) => seed,
                    Err(panic) => {
                        let message = panic_text(panic);
                        observations.push(observation(
                            &path,
                            "fixture_setup",
                            if message.contains("runtime audit fixture") {
                                "fixture_invalid"
                            } else {
                                "panicked"
                            },
                            message,
                        ));
                        continue;
                    }
                };
                let identity = compute_trigger_identity(triggered);
                let mut matched = 0;
                let mut tried = 0;
                for fixture in fixtures::events(&seed) {
                    tried += 1;
                    let precheck = catch_unwind(AssertUnwindSafe(|| {
                        let mut game = seed.game.clone();
                        fixture.prepare(&mut game);
                        triggered.trigger.matches(
                            &fixture.event,
                            &TriggerContext::for_source(
                                seed.source,
                                PlayerId::from_index(0),
                                &game,
                            ),
                        )
                    }));
                    match precheck {
                        Ok(false) => continue,
                        Err(panic) => {
                            observations.push(observation(
                                &path,
                                &fixture.name,
                                "panicked",
                                panic_text(panic),
                            ));
                            continue;
                        }
                        Ok(true) => {}
                    }
                    if matched == MAX_MATCHED_SCENARIOS {
                        observations.push(observation(&path, "coverage", "budget_exceeded", format!("matched-fixture cap {MAX_MATCHED_SCENARIOS}; remaining event/choice fixtures not exercised")));
                        break;
                    }
                    matched += 1;
                    for accept in [true, false] {
                        let scenario = format!(
                            "{}/optional_{}",
                            fixture.name,
                            if accept { "accept" } else { "decline" }
                        );
                        let result = guarded(&path, &scenario, || {
                            let mut game = seed.game.clone();
                            fixture.prepare(&mut game);
                            let mut queue = TriggerQueue::new();
                            for entry in check_triggers(&game, &fixture.event) {
                                if entry.source == seed.source && entry.trigger_identity == identity
                                {
                                    queue.add(entry);
                                }
                            }
                            if queue.entries.is_empty() {
                                return observation(
                                    &path,
                                    &scenario,
                                    "not_matched",
                                    "matcher accepted but trigger queue did not; functional zone or intervening condition not exercised",
                                );
                            }
                            if queue.entries.len() != 1 {
                                return observation(
                                    &path,
                                    &scenario,
                                    "not_exercised",
                                    format!(
                                        "{} queue entries; grouped trigger case requires a dedicated fixture",
                                        queue.entries.len()
                                    ),
                                );
                            }
                            let mut dm = AuditDecisions {
                                accept_optional: accept,
                                ..Default::default()
                            };
                            if let Err(error) =
                                put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
                            {
                                return observation(
                                    &path,
                                    &scenario,
                                    "announcement_failed",
                                    format!("{error}; {}", dm.summary()),
                                );
                            }
                            if game.stack.is_empty() {
                                return observation(
                                    &path,
                                    &scenario,
                                    "not_exercised",
                                    format!(
                                        "no stack entry after announcement: illegal/missing targets, intervening condition, or immediate mana ability; {}",
                                        dm.summary()
                                    ),
                                );
                            }
                            finish_resolution(&mut game, &mut dm, &path, &scenario)
                        });
                        let no_choice = result.detail.contains("optional=0,");
                        observations.push(result);
                        if no_choice {
                            break;
                        }
                    }
                }
                if matched == 0 {
                    observations.push(observation(&path, "coverage", "not_matched", format!("no match among {tried} event fixtures; not an execution pass; trigger={}", triggered.trigger.display())));
                }
            }
            AbilityKind::Activated(activated) => {
                let has_targets = !activated.choices.is_empty()
                    || activated.effects.iter().any(effect_has_target);
                if has_targets {
                    observations.push(observation(&path, "direct_resolution", "needs_fixture", "activated ability target/cost announcement is not implemented by the direct-resolution fixture"));
                    continue;
                }
                for accept in [true, false] {
                    let scenario = format!(
                        "direct_resolution/optional_{}",
                        if accept { "accept" } else { "decline" }
                    );
                    let result = guarded(&path, &scenario, || {
                        let mut isolated = definition.clone();
                        isolated.abilities = vec![ability.clone()];
                        let mut seed = fixtures::seed(&isolated, Zone::Battlefield);
                        seed.game.push_to_stack(StackEntry::ability(
                            seed.source,
                            PlayerId::from_index(0),
                            activated.effects.clone(),
                        ));
                        let mut dm = AuditDecisions {
                            accept_optional: accept,
                            ..Default::default()
                        };
                        let mut result =
                            finish_resolution(&mut seed.game, &mut dm, &path, &scenario);
                        result.detail = format!(
                            "direct effect smoke only: activation legality, costs and cost-produced context bypassed; {}",
                            result.detail
                        );
                        if result.status == "resolution_failed" {
                            result.status = "direct_resolution_failed".into();
                        }
                        result
                    });
                    let no_choice = result.detail.contains("optional=0,");
                    observations.push(result);
                    if no_choice {
                        break;
                    }
                }
            }
            _ => observations.push(observation(
                &path,
                "coverage",
                "not_exercised",
                "static/mana/other ability family has no triggered or activated resolution fixture",
            )),
        }
    }
    if let Some(program) = &definition.spell_effect {
        let path = "spell_effect";
        if program.iter().any(effect_has_target) {
            observations.push(observation(
                path,
                "direct_resolution",
                "needs_fixture",
                "spell target announcement is not implemented by the direct-resolution fixture",
            ));
        } else {
            for accept in [true, false] {
                let scenario = format!(
                    "direct_resolution/optional_{}",
                    if accept { "accept" } else { "decline" }
                );
                let result = guarded(path, &scenario, || {
                    let mut isolated = definition.clone();
                    isolated.abilities.clear();
                    let mut seed = fixtures::seed(&isolated, Zone::Stack);
                    seed.game
                        .push_to_stack(StackEntry::new(seed.source, PlayerId::from_index(0)));
                    let mut dm = AuditDecisions {
                        accept_optional: accept,
                        ..Default::default()
                    };
                    let mut result = finish_resolution(&mut seed.game, &mut dm, path, &scenario);
                    result.detail = format!(
                        "direct effect smoke only: casting legality, costs, modal announcement and cost-produced context bypassed; {}",
                        result.detail
                    );
                    if result.status == "resolution_failed" {
                        result.status = "direct_resolution_failed".into();
                    }
                    result
                });
                let no_choice = result.detail.contains("optional=0,");
                observations.push(result);
                if no_choice {
                    break;
                }
            }
        }
    }
    if observations.is_empty() {
        observations.push(observation(
            "card",
            "coverage",
            "not_exercised",
            "no executable abilities or spell resolution program",
        ));
    }
    observations
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironsmith::ability::Ability;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::{EventValueSpec, Value};
    use ironsmith::target::PlayerFilter;
    use ironsmith::triggers::Trigger;
    use ironsmith::{CardId, CardType};

    #[test]
    fn shared_card_identity_fixture_mistake_is_not_reported_as_an_engine_panic() {
        let card_id = CardId::new();
        let first = CardDefinitionBuilder::new(card_id, "First fixture")
            .card_types(vec![CardType::Creature])
            .build();
        let second = CardDefinitionBuilder::new(card_id, "Different fixture")
            .card_types(vec![CardType::Land])
            .build();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        create_fixture_object(&mut game, &first, PlayerId(0), Zone::Battlefield);
        let result = guarded("fixture", "collision", || {
            create_fixture_object(&mut game, &second, PlayerId(0), Zone::Battlefield);
            observation(
                "fixture",
                "collision",
                "executed",
                "guard failed to detect alias",
            )
        });
        assert_eq!(result.status, "fixture_invalid", "{result:?}");
    }

    #[test]
    fn catches_event_amount_required_by_a_trigger_that_does_not_supply_it() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Bad event contract")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(
                Trigger::beginning_of_upkeep(PlayerFilter::Any),
                vec![Effect::draw(Value::EventValue(EventValueSpec::Amount))],
            ))
            .build();
        let observations = audit(&definition);
        assert!(
            observations.iter().any(
                |item| item.status == "resolution_failed" && item.detail.contains("EventValue")
            ),
            "{observations:#?}"
        );
    }

    #[test]
    fn optional_accept_and_decline_have_distinct_coverage_results() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Optional draw")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(
                Trigger::beginning_of_upkeep(PlayerFilter::You),
                vec![Effect::may(vec![Effect::draw(1)])],
            ))
            .build();
        let observations = audit(&definition);
        assert!(
            observations.iter().any(|item| item.status == "executed"),
            "{observations:#?}"
        );
        assert!(
            observations
                .iter()
                .any(|item| item.status == "optional_skipped"),
            "{observations:#?}"
        );
    }
}

//! Supplemental probes through legal-action discovery, announcement, costs,
//! targets, resolution, state-based actions and resulting trigger queues.
//! Successful probes are bounded coverage observations, not semantic verdicts.

use super::execution::{
    ExecutionObservation, create_fixture_object, guarded, zone_invariant_failure,
};
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::*;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};

const SEED: u64 = 0x4143_5449_4f4e;
const MAX_ACTIONS_PER_ZONE: usize = 8;
const MAX_STEPS: usize = 48;
const LIMITS: &str = "bounded main-phase fixture; all printed abilities preserved; legal discovery, costs, targets, SBA and resulting triggers; sampled choices; no opponent responses or semantic oracle";

#[derive(Default)]
struct Decisions {
    accept: bool,
    decisions: usize,
    optional: usize,
    missing: bool,
}
impl Decisions {
    fn tick(&mut self) {
        self.decisions += 1;
        assert!(
            self.decisions <= 128,
            "runtime audit decision budget exhausted"
        );
    }
}
impl DecisionMaker for Decisions {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.tick();
        self.optional += 1;
        self.accept
    }
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        self.tick();
        if self.accept {
            2.clamp(ctx.min, ctx.max)
        } else {
            ctx.min
        }
    }
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.tick();
        if ctx.min == 0 {
            self.optional += 1;
        }
        let wanted = if self.accept { ctx.min.max(1) } else { ctx.min };
        let result: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|o| o.legal)
            .take(wanted.min(ctx.max.unwrap_or(wanted)))
            .map(|o| o.id)
            .collect();
        self.missing |= result.len() < ctx.min && !ctx.allow_partial_completion;
        result
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.tick();
        if ctx.min == 0 {
            self.optional += 1;
        }
        // Preserve the standard point-budget solver for modal/spree choices.
        let result = SelectFirstDecisionMaker.decide_options(game, ctx);
        if ctx.min == 0 && self.accept && result.is_empty() {
            if let Some(option) = ctx
                .options
                .iter()
                .find(|o| o.legal && (o.point_cost.max(1) as usize) <= ctx.max)
            {
                return vec![option.index];
            }
        }
        self.missing |= ctx.min > 0 && result.is_empty();
        result
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.tick();
        let mut result = Vec::new();
        for requirement in &ctx.requirements {
            let wanted = if self.accept {
                requirement.min_targets.max(1)
            } else {
                requirement.min_targets
            };
            let wanted = wanted.min(requirement.max_targets.unwrap_or(wanted));
            self.missing |= requirement.legal_targets.len() < requirement.min_targets;
            result.extend(requirement.legal_targets.iter().take(wanted).copied());
        }
        result
    }
}

fn observation(
    path: &str,
    scenario: &str,
    status: &str,
    detail: impl Into<String>,
) -> ExecutionObservation {
    ExecutionObservation {
        ability_path: path.into(),
        scenario: scenario.into(),
        status: status.into(),
        detail: detail.into(),
    }
}

fn seed(definition: &CardDefinition, source_zone: Zone) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.set_random_seed(SEED);
    game.turn.turn_number = 3;
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    let creature = CardDefinitionBuilder::new(CardId::new(), "Action audit Soldier")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(3, 3))
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::White],
            vec![ManaSymbol::Generic(2)],
        ]))
        .build();
    let forest = CardDefinitionBuilder::new(CardId::new(), "Action audit Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .supertypes(vec![ironsmith::Supertype::Basic])
        .build();
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Action audit artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let enchantment = CardDefinitionBuilder::new(CardId::new(), "Action audit enchantment")
        .card_types(vec![CardType::Enchantment])
        .build();
    for seat in 0..3 {
        let player = PlayerId(seat);
        for card in [&creature, &forest, &artifact, &enchantment] {
            let id = create_fixture_object(&mut game, card, player, Zone::Battlefield);
            game.remove_summoning_sickness(id);
            create_fixture_object(&mut game, card, player, Zone::Hand);
            create_fixture_object(&mut game, card, player, Zone::Graveyard);
            for _ in 0..4 {
                create_fixture_object(&mut game, card, player, Zone::Library);
            }
        }
        for mana in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            game.player_mut(player).unwrap().mana_pool.add(mana, 12);
        }
    }
    let source = create_fixture_object(&mut game, definition, PlayerId(0), source_zone);
    if source_zone == Zone::Battlefield {
        game.remove_summoning_sickness(source);
    }
    // Setup is historical. Only actions below may emit tested ETB/death events.
    game.effect_store.pending_trigger_events.clear();
    (game, source)
}

fn belongs_to(action: &LegalAction, source: ObjectId) -> bool {
    match action {
        LegalAction::CastSpell { spell_id, .. } => *spell_id == source,
        LegalAction::OpenExiledCardForPlay { card_id, .. } | LegalAction::CastExiledCardFaceDown { card_id, .. } => *card_id == source,
        LegalAction::ActivateAbility { source: id, .. }
        | LegalAction::ActivateManaAbility { source: id, .. } => *id == source,
        LegalAction::PlayLand { land_id } | LegalAction::PlayLandBackFace { land_id } => {
            *land_id == source
        }
        _ => false,
    }
}

fn run_action(
    mut game: GameState,
    action: LegalAction,
    accept: bool,
    path: &str,
    scenario: &str,
) -> ExecutionObservation {
    let mut dm = Decisions {
        accept,
        ..Default::default()
    };
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action.clone()),
        &mut dm,
    );
    let mut resolutions = 0;
    for _ in 0..MAX_STEPS {
        if let Some(problem) = zone_invariant_failure(&game) {
            return observation(
                path,
                scenario,
                "invariant_failed",
                format!("{problem}; {action:?}; {LIMITS}"),
            );
        }
        let next = match progress {
            Err(error) => {
                return observation(
                    path,
                    scenario,
                    "action_or_choice_failed",
                    format!(
                        "{error}; {action:?}; missing_choice={}; {LIMITS}",
                        dm.missing
                    ),
                );
            }
            Ok(GameProgress::GameOver(result)) => {
                return observation(
                    path,
                    scenario,
                    "executed_game_ended",
                    format!("{result:?}; resolved={resolutions}; {LIMITS}"),
                );
            }
            Ok(GameProgress::NeedsDecisionCtx(ref ctx))
                if !matches!(ctx, DecisionContext::Priority(_)) =>
            {
                apply_decision_context_with_dm(&mut game, &mut queue, &mut state, ctx, &mut dm)
            }
            _ => {
                // This is the same SBA/event-drain/trigger-announcement boundary
                // used when players receive priority in the game loop.
                match advance_priority_with_dm(&mut game, &mut queue, &mut dm) {
                    Ok(GameProgress::NeedsDecisionCtx(DecisionContext::Priority(_))) => {
                        if game.stack.is_empty() {
                            return observation(
                                path,
                                scenario,
                                if dm.missing {
                                    "needs_fixture"
                                } else {
                                    "executed"
                                },
                                format!(
                                    "{action:?}; resolved={resolutions}; decisions={}; optional={}; seed={SEED}; {LIMITS}",
                                    dm.decisions, dm.optional
                                ),
                            );
                        }
                        if let Err(error) = resolve_stack_entry_with(&mut game, &mut dm) {
                            return observation(
                                path,
                                scenario,
                                "resolution_failed",
                                format!(
                                    "{error}; {action:?}; resolved_before_error={resolutions}; missing_choice={}; {LIMITS}",
                                    dm.missing
                                ),
                            );
                        }
                        resolutions += 1;
                        Ok(GameProgress::StackResolved)
                    }
                    result => result,
                }
            }
        };
        progress = next;
    }
    observation(
        path,
        scenario,
        "budget_exceeded",
        format!("action/decision/resolution cap={MAX_STEPS}; resolved={resolutions}; {LIMITS}"),
    )
}

pub fn audit(definition: &CardDefinition) -> Vec<ExecutionObservation> {
    let mut result = Vec::new();
    for zone in [Zone::Hand, Zone::Battlefield, Zone::Graveyard] {
        if zone == Zone::Battlefield && !definition.is_permanent() {
            continue;
        }
        let path = format!("legal_actions/{zone:?}");
        let mut rows = Vec::new();
        let setup = guarded(&path, "discovery", || {
            let (game, source) = seed(definition, zone);
            let discovered = match compute_legal_actions(&game, PlayerId(0)) {
                Ok(actions) => actions,
                Err(error) => return observation(&path, "discovery", "action_discovery_failed", error.to_string()),
            };
            let actions: Vec<_> = discovered.into_iter()
                .filter(|action| belongs_to(action, source))
                .collect();
            for (index, action) in actions.iter().take(MAX_ACTIONS_PER_ZONE).enumerate() {
                for accept in [true, false] {
                    let scenario = format!("action_{index}/optional_{accept}/seed_{SEED}");
                    rows.push(guarded(&path, &scenario, || {
                        run_action(game.clone(), action.clone(), accept, &path, &scenario)
                    }));
                }
            }
            observation(
                &path,
                "discovery",
                if actions.is_empty() {
                    "not_exercised"
                } else if actions.len() > MAX_ACTIONS_PER_ZONE {
                    "budget_exceeded"
                } else {
                    "action_discovery"
                },
                format!(
                    "{} source actions found; max={MAX_ACTIONS_PER_ZONE}; seed={SEED}; {LIMITS}",
                    actions.len()
                ),
            )
        });
        result.push(setup);
        result.extend(rows);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spell_is_legally_cast_and_resolved_including_targets() {
        let definition = ironsmith_registry::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Action target fixture",
        )
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .parse_text("Deal 3 damage to any target.")
        .unwrap();
        let result = audit(&definition);
        assert!(
            result
                .iter()
                .any(|row| row.status == "executed" && row.detail.contains("resolved=1")),
            "{result:#?}"
        );
        assert!(
            !result.iter().any(|row| row.status.ends_with("failed")),
            "{result:#?}"
        );
    }
    #[test]
    fn malformed_spell_value_fails_after_legal_cast() {
        let mut definition = CardDefinitionBuilder::new(CardId::new(), "Missing amount fixture")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(0)]]))
            .build();
        definition.spell_effect = Some(
            vec![ironsmith::Effect::draw(
                ironsmith::effect::Value::EventValue(ironsmith::effect::EventValueSpec::Amount),
            )]
            .into(),
        );
        let result = audit(&definition);
        assert!(
            result.iter().any(|row| row.status == "resolution_failed"),
            "{result:#?}"
        );
    }
}

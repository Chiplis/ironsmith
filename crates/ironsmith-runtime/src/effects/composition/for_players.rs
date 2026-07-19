//! ForPlayers effect implementation.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, SimultaneousEffectProposal};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::filter::PlayerFilterExt;
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::target::PlayerFilter;

/// Effect that applies effects once for each player matching a filter.
///
/// Sets `ctx.iterated_player` for each iteration, allowing inner effects
/// to reference the current player via `PlayerFilter::IteratedPlayer`.
///
/// # Fields
///
/// * `filter` - Filter for which players to iterate over
/// * `effects` - Effects to execute for each matching player
///
/// # Example
///
/// ```ignore
/// // Deal 3 damage to each opponent
/// let effect = ForPlayersEffect::new(
///     PlayerFilter::Opponent,
///     vec![Effect::deal_damage(3, ChooseSpec::Player(PlayerFilter::IteratedPlayer))],
/// );
///
/// // Each player draws a card
/// let effect = ForPlayersEffect::new(
///     PlayerFilter::Any,
///     vec![Effect::target_draws(1, PlayerFilter::IteratedPlayer)],
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ForPlayersEffect {
    /// Filter for which players to iterate over.
    pub filter: PlayerFilter,
    /// Effects to execute for each matching player.
    pub effects: Vec<Effect>,
    /// Whether iteration should begin with the effect controller and proceed in turn order.
    pub starting_with_controller: bool,
    /// Whether iteration should stop after the first player whose effects happened.
    pub stop_after_first_happened: bool,
}

impl ForPlayersEffect {
    /// Create a new ForPlayers effect.
    pub fn new(filter: PlayerFilter, effects: Vec<Effect>) -> Self {
        Self {
            filter,
            effects,
            starting_with_controller: false,
            stop_after_first_happened: false,
        }
    }

    pub fn new_starting_with_controller(filter: PlayerFilter, effects: Vec<Effect>) -> Self {
        Self {
            filter,
            effects,
            starting_with_controller: true,
            stop_after_first_happened: false,
        }
    }

    pub fn stop_after_first_happened(mut self) -> Self {
        self.stop_after_first_happened = true;
        self
    }
}

fn rotate_players_to_start(players: &mut Vec<PlayerId>, start: PlayerId) {
    if let Some(start_pos) = players.iter().position(|&player_id| player_id == start) {
        players.rotate_left(start_pos);
    }
}

fn order_selected_players_from(
    game: &GameState,
    selected_players: Vec<PlayerId>,
    start: PlayerId,
) -> Vec<PlayerId> {
    let mut turn_order = game.turn_store.turn_order.clone();
    rotate_players_to_start(&mut turn_order, start);

    let mut ordered_players = turn_order
        .into_iter()
        .filter(|player_id| selected_players.contains(player_id))
        .collect::<Vec<_>>();
    for player_id in selected_players {
        if !ordered_players.contains(&player_id) {
            ordered_players.push(player_id);
        }
    }
    ordered_players
}

/// In Two-Headed Giant, a shared-life action like a "set life total" effect
/// applies once per team: the team's primary player picks which head performs
/// it at the team's first position in APNAP order. Returns, per shared
/// effect, the acting players in that order; other effects keep ordinary
/// per-player iteration.
fn twohg_shared_action_players(
    game: &GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
    players: &[PlayerId],
) -> Result<std::collections::HashMap<usize, Vec<PlayerId>>, ExecutionError> {
    let mut shared: std::collections::HashMap<usize, Vec<PlayerId>> =
        std::collections::HashMap::new();
    if game.two_headed_giant().is_none() {
        return Ok(shared);
    }
    for (effect_index, effect) in effects.iter().enumerate() {
        if effect
            .downcast_ref::<crate::effects::SetLifeTotalEffect>()
            .is_none()
        {
            continue;
        }
        let mut seen_teams = std::collections::HashSet::new();
        for player in players.iter().copied() {
            let Some(team) = game.team_index_for(player) else {
                continue;
            };
            if !seen_teams.insert(team) {
                continue;
            }
            let candidates = game
                .team_players_for(player)
                .into_iter()
                .filter(|member| players.contains(member))
                .collect::<Vec<_>>();
            if candidates.len() == 1 {
                shared.entry(effect_index).or_default().push(candidates[0]);
                continue;
            }
            let options = candidates
                .iter()
                .filter_map(|member| {
                    game.player(*member)
                        .map(|candidate| (candidate.name.to_string(), *member))
                })
                .collect::<Vec<_>>();
            let chooser = game.primary_player_for_team(team).unwrap_or(player);
            let chosen = crate::decisions::ask_choose_one(
                game,
                &mut ctx.decision_maker,
                chooser,
                ctx.source,
                &options,
            )
            .unwrap_or(player);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(shared);
            }
            shared.entry(effect_index).or_default().push(chosen);
        }
    }
    Ok(shared)
}


/// True when this effect (or a nested child) selects objects through the
/// given context tag, i.e. it consumes what an earlier tagged effect binds.
fn effect_consumes_tag(effect: &Effect, tag: &crate::tag::TagKey) -> bool {
    fn spec_consumes(spec: &crate::target::ChooseSpec, tag: &crate::tag::TagKey) -> bool {
        match spec.base() {
            crate::target::ChooseSpec::Tagged(spec_tag) => spec_tag == tag,
            crate::target::ChooseSpec::Object(filter) => filter
                .tagged_constraints
                .iter()
                .any(|constraint| &constraint.tag == tag),
            _ => false,
        }
    }
    if effect
        .0
        .get_target_spec()
        .is_some_and(|spec| spec_consumes(spec, tag))
    {
        return true;
    }
    if effect
        .0
        .decision_related_object_specs()
        .iter()
        .any(|spec| spec_consumes(spec, tag))
    {
        return true;
    }
    let mut found = false;
    effect.0.visit_child_effects(&mut |child| {
        found |= effect_consumes_tag(child, tag);
    });
    found
}

/// The tag a wrapper effect binds for later effects, if any.
fn effect_bound_tag(effect: &Effect) -> Option<crate::tag::TagKey> {
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| tagged.tag.clone())
}

impl EffectExecutor for ForPlayersEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let filter_ctx = ctx.filter_context(game);

        // Iterate over all players that match the filter
        let mut players: Vec<PlayerId> = game
            .players
            .iter()
            .filter(|p| p.is_in_game())
            .filter(|p| self.filter.matches_player(p.id, &filter_ctx))
            .map(|p| p.id)
            .collect();

        let first_player = if self.starting_with_controller {
            ctx.controller
        } else {
            game.turn.active_player
        };
        players = order_selected_players_from(game, players, first_player);

        if players.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut outcomes = Vec::new();
        let mut outcomes_by_player = vec![Vec::new(); players.len()];

        if !self.starting_with_controller && !self.stop_after_first_happened {
            if let Some(unsupported) = self.effects.iter().find(|effect| {
                !effect.0.supports_simultaneous_player_action()
                    && !effect.0.is_read_only_simultaneous_player_action()
            }) {
                let mut description = format!("{:?}", unsupported.0);
                description.truncate(120);
                return Err(ExecutionError::Impossible(format!(
                    "generic each-player action lacks simultaneous proposal support: {description}"
                )));
            }
        }

        if self.starting_with_controller || self.stop_after_first_happened {
            // An explicit starting player describes a sequential instruction
            // ("starting with ..."), as does stopping after the first player
            // whose action happened. Preserve player-major execution there.
            for (player_index, &player_id) in players.iter().enumerate() {
                let mut stop = false;
                ctx.with_temp_iterated_player(Some(player_id), |ctx| {
                    for effect in &self.effects {
                        let outcome = execute_effect(game, effect, ctx)?;
                        outcomes_by_player[player_index].push(outcome.clone());
                        outcomes.push(outcome);
                    }
                    let count = EffectOutcome::aggregate_summing_counts(
                        outcomes_by_player[player_index].iter().cloned(),
                    )
                    .as_count()
                    .unwrap_or(0);
                    stop = self.stop_after_first_happened && count > 0;
                    Ok::<(), ExecutionError>(())
                })?;
                if stop {
                    break;
                }
            }
        } else {
            // CR 608.2f: choices for a simultaneous each-player action are
            // made in APNAP order against the pre-action game state, then the
            // whole action commits as one transaction. Decisions (including
            // read-only chooser effects that tag the execution context) run
            // player-major so one player's tags feed that player's own
            // proposal without leaking into the next player's pass; game
            // mutations are deferred to the batched commit below.
            let shared_action_players =
                twohg_shared_action_players(game, ctx, &self.effects, &players)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }

            // CR 608.2e: finish each printed action for every player before
            // beginning the next. A read-only chooser effect is not a printed
            // action of its own — it feeds the effect that follows it — so
            // effects are grouped into action units: any run of read-only
            // effects plus the next mutating effect. Within one unit, choices
            // and proposal preparation happen player-major in APNAP order
            // against the pre-action state (CR 608.2f, 101.4), keeping each
            // player's context tags scoped to their own proposal; the unit
            // then commits as one batch before the next unit begins.
            let mut units: Vec<Vec<usize>> = Vec::new();
            let mut current: Vec<usize> = Vec::new();
            for (effect_index, effect) in self.effects.iter().enumerate() {
                current.push(effect_index);
                if effect.0.is_read_only_simultaneous_player_action() {
                    continue;
                }
                // A mutating effect that binds a tag consumed by the next
                // effect is half of one printed action ("return it ... with a
                // counter on it") — keep the consumer in the same unit so the
                // per-player commit interleaving preserves the tag handoff.
                if let Some(tag) = effect_bound_tag(effect)
                    && self
                        .effects
                        .get(effect_index + 1)
                        .is_some_and(|next| effect_consumes_tag(next, &tag))
                {
                    continue;
                }
                units.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                units.push(current);
            }

            for unit in units {
                let mut prepared: Vec<(usize, Box<dyn SimultaneousEffectProposal>)> = Vec::new();
                // A shared (once-per-team) effect prepares for its chosen
                // acting players in team-first APNAP order instead of every
                // seat; the whole unit follows that ordering so commit order
                // matches the pre-unit behavior.
                let unit_shared_order: Option<&Vec<PlayerId>> = unit
                    .iter()
                    .find_map(|effect_index| shared_action_players.get(effect_index));
                let unit_players: Vec<PlayerId> = match unit_shared_order {
                    Some(acting) => acting.clone(),
                    None => players.clone(),
                };
                for &player_id in &unit_players {
                    let player_index = players
                        .iter()
                        .position(|candidate| *candidate == player_id)
                        .expect("acting player is in the iteration set");
                    ctx.with_temp_iterated_player(Some(player_id), |ctx| {
                        for &effect_index in &unit {
                            let effect = &self.effects[effect_index];
                            if effect.0.is_read_only_simultaneous_player_action() {
                                let outcome = execute_effect(game, effect, ctx)?;
                                outcomes_by_player[player_index].push(outcome.clone());
                                outcomes.push(outcome);
                            } else if effect.0.supports_simultaneous_player_action() {
                                let proposal =
                                    effect.0.prepare_simultaneous_player_action(game, ctx)?;
                                prepared.push((player_index, proposal));
                            } else {
                                return Err(ExecutionError::Impossible(
                                    "generic each-player action lacks simultaneous proposal support"
                                        .to_string(),
                                ));
                            }
                        }
                        Ok::<(), ExecutionError>(())
                    })?;
                }

                let game_checkpoint = game.clone();
                let mut batch_outcomes = Vec::with_capacity(prepared.len());
                for (player_index, proposal) in prepared {
                    match proposal.commit(game, ctx) {
                        Ok(outcome) => batch_outcomes.push((player_index, outcome)),
                        Err(error) => {
                            *game = game_checkpoint;
                            return Err(error);
                        }
                    }
                }
                for (player_index, outcome) in batch_outcomes {
                    outcomes_by_player[player_index].push(outcome.clone());
                    outcomes.push(outcome);
                }
            }
        }

        let mut player_counts = Vec::new();
        let mut player_affected_memory = Vec::new();
        for (&player_id, player_outcomes) in players.iter().zip(&outcomes_by_player) {
            if player_outcomes.is_empty() {
                continue;
            }
            let iteration_outcome =
                EffectOutcome::aggregate_summing_counts(player_outcomes.iter().cloned());
            player_counts.push((player_id, iteration_outcome.as_count().unwrap_or(0)));
            if let Some(memory) = iteration_outcome.affected_object_memory()
                && !memory.is_empty()
            {
                player_affected_memory.push((player_id, memory.to_vec()));
            }
        }

        Ok(EffectOutcome::aggregate_summing_counts(outcomes)
            .with_player_counts(player_counts)
            .with_player_affected_object_memory(player_affected_memory))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct RecordIteratedPlayerChoice(&'static str);

    #[derive(Debug)]
    struct ReadOnlyChoiceProposal;

    impl crate::effects::SimultaneousEffectProposal for ReadOnlyChoiceProposal {
        fn commit(
            self: Box<Self>,
            _game: &mut GameState,
            _ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            Ok(EffectOutcome::count(0))
        }
    }

    impl EffectExecutor for RecordIteratedPlayerChoice {
        fn execute(
            &self,
            game: &mut GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            let player = ctx
                .iteration
                .iterated_player
                .expect("ForPlayers must set the iterated player");
            let prompt =
                crate::decisions::context::BooleanContext::new(player, Some(ctx.source), self.0);
            ctx.decision_maker.decide_boolean(game, &prompt);
            Ok(EffectOutcome::count(0))
        }

        fn supports_simultaneous_player_action(&self) -> bool {
            true
        }

        fn prepare_simultaneous_player_action(
            &self,
            game: &GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
            let player = ctx
                .iteration
                .iterated_player
                .expect("ForPlayers must set the iterated player");
            let prompt =
                crate::decisions::context::BooleanContext::new(player, Some(ctx.source), self.0);
            ctx.decision_maker.decide_boolean(game, &prompt);
            Ok(Box::new(ReadOnlyChoiceProposal))
        }
    }

    #[derive(Default)]
    struct RecordChoiceOrder {
        prompts: Vec<(PlayerId, String)>,
    }

    impl crate::decision::DecisionMaker for RecordChoiceOrder {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.prompts.push((ctx.player, ctx.description.clone()));
            false
        }
    }

    #[derive(Debug, Clone)]
    struct AtomicBatchProbe;

    #[derive(Debug, Clone)]
    struct UnsupportedMutationProbe;

    impl EffectExecutor for UnsupportedMutationProbe {
        fn execute(
            &self,
            game: &mut GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            let player = ctx.iteration.iterated_player.expect("iterated player");
            game.player_mut(player).expect("probe player").lose_life(1);
            Ok(EffectOutcome::count(1))
        }
    }

    #[derive(Debug)]
    struct AtomicBatchProposal {
        player: PlayerId,
        fail: bool,
    }

    impl crate::effects::SimultaneousEffectProposal for AtomicBatchProposal {
        fn commit(
            self: Box<Self>,
            game: &mut GameState,
            _ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            if self.fail {
                return Err(ExecutionError::Impossible("probe failure".to_string()));
            }
            game.player_mut(self.player)
                .expect("probe player")
                .lose_life(1);
            Ok(EffectOutcome::count(1))
        }
    }

    impl EffectExecutor for AtomicBatchProbe {
        fn execute(
            &self,
            _game: &mut GameState,
            _ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            unreachable!("generic each-player execution must use the proposal hook")
        }

        fn supports_simultaneous_player_action(&self) -> bool {
            true
        }

        fn prepare_simultaneous_player_action(
            &self,
            _game: &GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
            let player = ctx.iteration.iterated_player.expect("iterated player");
            Ok(Box::new(AtomicBatchProposal {
                player,
                fail: player == PlayerId::from_index(1),
            }))
        }
    }

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn i004_generic_each_player_choices_use_apnap_order() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        game.turn.active_player = cara;
        game.turn_store.turn_order = vec![alice, bob, cara];

        let source = game.new_object_id();
        let mut decisions = RecordChoiceOrder::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
        ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::new(RecordIteratedPlayerChoice("choose"))],
        )
        .execute(&mut game, &mut ctx)
        .expect("each-player effect should resolve");

        assert_eq!(
            decisions.prompts,
            vec![
                (cara, "choose".to_string()),
                (alice, "choose".to_string()),
                (bob, "choose".to_string()),
            ]
        );
    }

    #[test]
    fn i004_generic_each_player_clauses_are_action_major() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        game.turn.active_player = bob;
        game.turn_store.turn_order = vec![alice, bob, cara];

        let source = game.new_object_id();
        let mut decisions = RecordChoiceOrder::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
        ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![
                Effect::new(RecordIteratedPlayerChoice("first action")),
                Effect::new(RecordIteratedPlayerChoice("second action")),
            ],
        )
        .execute(&mut game, &mut ctx)
        .expect("each-player effect should resolve");

        assert_eq!(
            decisions.prompts,
            vec![
                (bob, "first action".to_string()),
                (cara, "first action".to_string()),
                (alice, "first action".to_string()),
                (bob, "second action".to_string()),
                (cara, "second action".to_string()),
                (alice, "second action".to_string()),
            ]
        );
    }

    #[test]
    fn i004_generic_each_player_action_uses_one_immutable_proposal_state() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::lose_life_player(
                crate::effect::Value::LifeTotal(PlayerFilter::You),
                PlayerFilter::IteratedPlayer,
            )],
        )
        .execute(&mut game, &mut ctx)
        .expect("simultaneous each-player life loss should resolve");

        assert_eq!(game.player(alice).expect("alice").life, 0);
        assert_eq!(
            game.player(bob).expect("bob").life,
            0,
            "Bob's proposal must use Alice's pre-action life total"
        );
        assert_eq!(
            result.player_counts(),
            Some([(alice, 20), (bob, 20)].as_slice())
        );
        assert_eq!(result.events.len(), 2);
        assert!(
            result
                .events
                .iter()
                .all(|event| event.provenance() == ctx.provenance)
        );
        assert!(
            game.player(alice).expect("alice").is_in_game()
                && game.player(bob).expect("bob").is_in_game(),
            "state-based actions are checked only after the whole batch resolves"
        );
    }

    #[test]
    fn i004_simultaneous_proposal_commit_is_atomic_on_error() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let error = ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(AtomicBatchProbe)])
            .execute(&mut game, &mut ctx)
            .expect_err("second proposal should fail");

        assert_eq!(
            error,
            ExecutionError::Impossible("probe failure".to_string())
        );
        assert_eq!(game.player(alice).expect("alice").life, 20);
        assert_eq!(game.player(bob).expect("bob").life, 20);
    }

    #[test]
    fn i004_unsupported_generic_mutation_fails_closed() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let error = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![
                Effect::lose_life_player(1, PlayerFilter::IteratedPlayer),
                Effect::new(UnsupportedMutationProbe),
            ],
        )
        .execute(&mut game, &mut ctx)
        .expect_err("unsupported mutation must not run sequentially");

        let ExecutionError::Impossible(message) = &error else {
            panic!("expected Impossible error, got {error:?}");
        };
        assert!(
            message.starts_with("generic each-player action lacks simultaneous proposal support"),
            "unexpected gate message: {message}"
        );
        assert!(
            message.contains("UnsupportedMutationProbe"),
            "gate message should name the offending effect: {message}"
        );
        assert_eq!(game.player(alice).expect("alice").life, 20);
        assert_eq!(game.player(bob).expect("bob").life, 20);
    }

    #[test]
    fn for_players_sums_count_results_across_players() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::lose_life_player(1, PlayerFilter::IteratedPlayer)],
        );
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).expect("alice").life, 19);
        assert_eq!(game.player(PlayerId::from_index(1)).expect("bob").life, 19);
    }

    #[test]
    fn for_players_records_per_player_count_partitions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::lose_life_player(
                crate::effect::Value::Fixed(1),
                PlayerFilter::IteratedPlayer,
            )],
        );
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(
            result.player_counts(),
            Some([(alice, 1), (bob, 1)].as_slice())
        );
    }

    #[test]
    fn for_players_records_per_player_affected_object_memory_partitions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let alice_card = game.new_object_id();
        let bob_card = game.new_object_id();
        let alice_memory = crate::effect::OutcomeObjectMemory {
            object_id: alice_card,
            stable_id: crate::ids::StableId::from(alice_card),
            controller: alice,
            owner: alice,
            zone: crate::zone::Zone::Library,
            power: None,
            toughness: None,
            mana_value: 1,
            card_types: vec![crate::types::CardType::Creature],
            colors: crate::color::ColorSet::COLORLESS,
            subtypes: Vec::new(),
            is_token: false,
        };
        let bob_memory = crate::effect::OutcomeObjectMemory {
            object_id: bob_card,
            stable_id: crate::ids::StableId::from(bob_card),
            controller: bob,
            owner: bob,
            zone: crate::zone::Zone::Library,
            power: None,
            toughness: None,
            mana_value: 2,
            card_types: vec![crate::types::CardType::Instant],
            colors: crate::color::ColorSet::COLORLESS,
            subtypes: Vec::new(),
            is_token: false,
        };

        let result = EffectOutcome::aggregate_summing_counts(vec![
            EffectOutcome::count(1)
                .with_affected_object_memory(vec![alice_memory.clone()])
                .with_player_affected_object_memory(vec![(alice, vec![alice_memory])]),
            EffectOutcome::count(1)
                .with_affected_object_memory(vec![bob_memory.clone()])
                .with_player_affected_object_memory(vec![(bob, vec![bob_memory])]),
        ]);

        let partitions = result
            .player_affected_object_memory()
            .expect("per-player affected memory");
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].0, alice);
        assert_eq!(partitions[0].1[0].controller, alice);
        assert_eq!(partitions[1].0, bob);
        assert_eq!(partitions[1].1[0].controller, bob);

        let effect = ForPlayersEffect::new(PlayerFilter::Any, Vec::new());
        let empty_result = effect
            .execute(&mut game, &mut ctx)
            .expect("empty per-player effect should resolve");
        assert!(empty_result.player_affected_object_memory().is_none());
    }

    #[test]
    fn for_each_opponent_reveal_keeps_each_opponents_revealed_card_partitioned() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let bob_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1001), "Bob Top")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let cara_card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1002), "Cara Top")
                .card_types(vec![crate::types::CardType::Instant])
                .build();
        let bob_id = game.create_object_from_card(&bob_card, bob, crate::zone::Zone::Library);
        let cara_id = game.create_object_from_card(&cara_card, cara, crate::zone::Zone::Library);

        let effect = ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![Effect::reveal_top_cards(
                PlayerFilter::IteratedPlayer,
                crate::effect::Value::Fixed(1),
                crate::tag::TagKey::from("revealed"),
            )],
        );
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("for each opponent reveal should resolve");

        assert_eq!(result.events.len(), 2);
        assert_eq!(
            result.affected_object_memory().map(|memory| memory.len()),
            Some(2)
        );
        let partitions = result
            .player_affected_object_memory()
            .expect("per-player reveal partitions");
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].0, bob);
        assert_eq!(partitions[0].1.len(), 1);
        assert_eq!(partitions[0].1[0].object_id, bob_id);
        assert_eq!(partitions[1].0, cara);
        assert_eq!(partitions[1].1.len(), 1);
        assert_eq!(partitions[1].1[0].object_id, cara_id);
    }
}

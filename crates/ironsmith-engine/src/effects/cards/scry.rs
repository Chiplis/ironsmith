//! Scry and fateseal effect implementation.

use crate::decisions::ask_choose_one;
use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{
    CompletedEffectOutputs, EffectExecutor, SimultaneousEffectCommit, SimultaneousEffectProposal,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{
    PreparedReplacementProgram, ReplacementEventContext, TraitEventResult,
};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::static_abilities::misc::event_amount_replacement::may_have_keyword_action_replacements;
use crate::target::PlayerFilter;

fn players_in_turn_order(game: &GameState) -> Vec<PlayerId> {
    game.team_apnap_player_order()
}

fn top_library_cards_top_to_bottom(
    game: &GameState,
    player_id: PlayerId,
    count: usize,
) -> Vec<ObjectId> {
    game.player(player_id)
        .map(|player| player.library.iter().rev().take(count).copied().collect())
        .unwrap_or_default()
}

fn choose_fateseal_opponent(
    game: &GameState,
    ctx: &mut ExecutionContext,
    fatesealer: PlayerId,
) -> Option<PlayerId> {
    let opponents: Vec<PlayerId> = players_in_turn_order(game)
        .into_iter()
        .filter(|player_id| {
            game.are_opponents(fatesealer, *player_id)
                && game.player_is_within_range(fatesealer, *player_id)
                && game
                    .player(*player_id)
                    .is_some_and(|player| player.is_in_game())
        })
        .collect();
    match opponents.len() {
        0 => None,
        1 => opponents.first().copied(),
        _ => {
            let options: Vec<(String, PlayerId)> = opponents
                .iter()
                .filter_map(|player_id| {
                    game.player(*player_id)
                        .map(|player| (player.name.to_string(), *player_id))
                })
                .collect();
            ask_choose_one(
                game,
                &mut ctx.decision_maker,
                fatesealer,
                ctx.source,
                &options,
            )
        }
    }
}

#[derive(Debug)]
struct ScryArrangement {
    viewer: PlayerId,
    player_id: PlayerId,
    top_cards_top_to_bottom: Vec<ObjectId>,
    bottom_cards_top_to_bottom: Vec<ObjectId>,
    observation: Option<CompletedEffectOutputs>,
}

fn look_for_arrangement(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    viewer: PlayerId,
    player_id: PlayerId,
    count: usize,
    action: KeywordActionKind,
) -> Result<ScryArrangement, ExecutionError> {
    let cards = top_library_cards_top_to_bottom(game, player_id, count);
    let observation = if cards.is_empty() {
        None
    } else {
        game.remember_library_group(viewer, player_id, &cards, false);
        Some(super::look_at_cards_with_outputs(
            game,
            ctx,
            viewer,
            player_id,
            crate::zone::Zone::Library,
            &cards,
            format!("{action:?} {} card(s)", cards.len()),
        )?)
    };
    Ok(ScryArrangement {
        viewer,
        player_id,
        top_cards_top_to_bottom: cards,
        bottom_cards_top_to_bottom: Vec::new(),
        observation,
    })
}

fn choose_arrangement(
    game: &GameState,
    ctx: &mut ExecutionContext,
    arrangement: &mut ScryArrangement,
) {
    let viewer = arrangement.viewer;
    let cards = &mut arrangement.top_cards_top_to_bottom;
    let Some(bottom) = super::choose_library_bottom_partition(game, ctx, viewer, cards) else {
        return;
    };
    cards.retain(|card| !bottom.contains(card));
    *cards = super::order_library_cards_top_to_bottom(
        game,
        ctx,
        viewer,
        "Reorder cards to keep on top of your library",
        cards,
    );
    arrangement.bottom_cards_top_to_bottom = super::order_library_cards_top_to_bottom(
        game,
        ctx,
        viewer,
        "Reorder cards to put on the bottom of your library",
        &bottom,
    );
}

fn apply_scry_arrangement(game: &mut GameState, arrangement: &ScryArrangement) {
    if arrangement.top_cards_top_to_bottom.is_empty()
        && arrangement.bottom_cards_top_to_bottom.is_empty()
    {
        return;
    }
    super::arrange_library_cards(
        game,
        arrangement.player_id,
        &arrangement.top_cards_top_to_bottom,
        &arrangement.bottom_cards_top_to_bottom,
        "scry or fateseal arranged library cards",
    );
    for cards in [
        &arrangement.top_cards_top_to_bottom,
        &arrangement.bottom_cards_top_to_bottom,
    ] {
        game.remember_library_group(arrangement.viewer, arrangement.player_id, cards, false);
    }
}

/// Effect that lets a player scry N cards.
///
/// Per Rule 701.22, look at the top N cards, then put any number on the bottom
/// of the library in any order and the rest on top in any order.
///
/// # Fields
///
/// * `count` - Number of cards to scry
/// * `player` - The player who scries
///
/// # Example
///
/// ```ignore
/// // Scry 2
/// let effect = ScryEffect::new(2, PlayerFilter::You);
///
/// // Scry 1
/// let effect = ScryEffect::you(1);
/// ```
pub type ScryEffect = ironsmith_core::ScryEffect;

impl EffectExecutor for ScryEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(ScryProposal {
            player: resolve_player_filter(game, &self.player, ctx)?,
            count: resolve_value(game, &self.count, ctx)?.max(0) as u32,
            processed: None,
            arrangement: None,
            sealed: false,
        }))
    }

    #[crate::effects::project_effect_outcome]
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        if may_have_keyword_action_replacements(game) {
            let proposal = self.prepare_simultaneous_player_action(game, ctx)?;
            return crate::effects::composition::complete_prepared_original_with_outputs(
                proposal, game, ctx, false,
            );
        }
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player = resolve_player_filter(game, &self.player, ctx)?;
                let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
                complete_individual_arrangement(
                    game,
                    ctx,
                    player,
                    player,
                    count,
                    KeywordActionKind::Scry,
                )
            },
        )
    }
}

#[derive(Debug)]
struct ScryProposal {
    player: PlayerId,
    count: u32,
    processed: Option<TraitEventResult>,
    arrangement: Option<ScryArrangement>,
    sealed: bool,
}

impl SimultaneousEffectProposal for ScryProposal {
    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.count == 0 || self.processed.is_some() {
            return Ok(());
        }
        let event = crate::events::Event::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Scry, self.player, ctx.source, self.count),
            ctx.provenance,
        );
        self.processed = Some(if may_have_keyword_action_replacements(game) {
            crate::events::processing::process_trait_event_with_execution_context(game, event, ctx)?
        } else {
            TraitEventResult::Proceed(event)
        });
        Ok(())
    }

    // CR 701.22c: all participants look before anyone partitions their cards.
    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.prepare_selection(game, ctx)?;
        if self.arrangement.is_none()
            && let Some(event) = self
                .processed
                .as_ref()
                .and_then(TraitEventResult::resolved_event)
        {
            let action = crate::events::downcast_event::<KeywordActionEvent>(event.inner())
                .filter(|action| action.action == KeywordActionKind::Scry)
                .ok_or_else(|| {
                    ExecutionError::InternalError("scry replacement lost its action".into())
                })?;
            if action.amount > 0 {
                self.arrangement = Some(look_for_arrangement(
                    game,
                    ctx,
                    action.player,
                    action.player,
                    action.amount as usize,
                    KeywordActionKind::Scry,
                )?);
            }
        }
        Ok(())
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if !self.sealed {
            self.prepare_original(game, ctx)?;
            if let Some(arrangement) = &mut self.arrangement {
                choose_arrangement(game, ctx, arrangement);
            }
            self.sealed = !ctx.decision_maker.awaiting_choice();
        }
        Ok(())
    }

    fn commit_original_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.seal_original(game, ctx)?;
        let zero = || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0));
        let Some(processed) = self
            .processed
            .take()
            .filter(|_| !ctx.decision_maker.awaiting_choice())
        else {
            return Ok(SimultaneousEffectCommit::finished(zero()));
        };
        let (original, additions) = processed.into_expansion();
        let committed = match original {
            TraitEventResult::Proceed(_) | TraitEventResult::Modified(_) => {
                let outcome = match self.arrangement {
                    Some(arrangement) => {
                        complete_arrangement(game, ctx, arrangement, KeywordActionKind::Scry)?
                    }
                    None => zero(), // CR 701.22b: scry 0 produces no event.
                };
                SimultaneousEffectCommit::finished(outcome)
            }
            TraitEventResult::Replaced {
                context,
                effects,
                source,
                controller,
                ..
            } => {
                let bindings = scry_replacement_bindings(&context)?;
                crate::effects::replacement::commit_bound_replacement_program_original_with_outputs(
                    game,
                    ctx,
                    PreparedReplacementProgram {
                        context,
                        effects,
                        source,
                        controller,
                        source_snapshot: None,
                    },
                    bindings,
                )?
            }
            TraitEventResult::Prevented => {
                let mut outcome = EffectOutcome::prevented();
                outcome.set_value(crate::effect::OutcomeValue::Count(0));
                SimultaneousEffectCommit::finished(CompletedEffectOutputs::aggregate_only(outcome))
            }
            _ => {
                return Err(ExecutionError::InternalError(
                    "scry replacement did not finish selection".into(),
                ));
            }
        };
        Ok(
            crate::effects::replacement::defer_replacement_programs_with_outputs(
                committed,
                additions,
                scry_replacement_bindings,
            ),
        )
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(SimultaneousEffectCommit::into_aggregate)
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::complete_prepared_original_with_outputs(self, game, ctx, false)
            .map(CompletedEffectOutputs::into_outcome)
    }
}

fn scry_replacement_bindings(
    context: &ReplacementEventContext,
) -> Result<crate::effects::replacement::ReplacementProgramBindings, ExecutionError> {
    let action = crate::events::downcast_event::<KeywordActionEvent>(context.event.inner())
        .filter(|action| action.action == KeywordActionKind::Scry)
        .ok_or_else(|| ExecutionError::InternalError("scry replacement lost its action".into()))?;
    Ok(crate::effects::replacement::ReplacementProgramBindings {
        targets: None,
        object_tags: crate::effects::composition::keyword_action_object_bindings(
            action.object_tags.clone(),
            action.snapshot.as_ref(),
        ),
    })
}

fn complete_individual_arrangement(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    viewer: PlayerId,
    player: PlayerId,
    count: usize,
    action: KeywordActionKind,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    if count == 0 {
        return Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let mut arrangement = look_for_arrangement(game, ctx, viewer, player, count, action)?;
    choose_arrangement(game, ctx, &mut arrangement);
    if ctx.decision_maker.awaiting_choice() {
        return Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    complete_arrangement(game, ctx, arrangement, action)
}

fn complete_arrangement(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    arrangement: ScryArrangement,
    action: KeywordActionKind,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    apply_scry_arrangement(game, &arrangement);
    let bottom = arrangement.bottom_cards_top_to_bottom.len();
    let count = arrangement.top_cards_top_to_bottom.len() + bottom;
    let mut outputs = CompletedEffectOutputs::with_primary_result(
        EffectOutcome::count(count as i64),
        arrangement.observation,
    );
    // CR 701.22d: a nonzero instruction still completes on an empty library.
    outputs = crate::effects::composition::complete_keyword_action_with_outputs(
        game,
        ctx,
        outputs,
        KeywordActionEvent::new(action, arrangement.viewer, ctx.source, count as u32),
    )?;
    if action == KeywordActionKind::Scry && bottom > 0 {
        outputs = crate::effects::composition::complete_keyword_action_with_outputs(
            game,
            ctx,
            outputs,
            KeywordActionEvent::new(
                KeywordActionKind::ScryToBottom,
                arrangement.viewer,
                ctx.source,
                bottom as u32,
            ),
        )?;
    }
    Ok(outputs)
}

/// Effect that lets a player fateseal N cards.
///
/// Per rule 701.29a, the player looks at the top N cards of an opponent's
/// library, then puts any number of them on the bottom of that library and the
/// rest on top in any order.
#[derive(Debug, Clone, PartialEq)]
pub struct FatesealEffect {
    /// Number of cards to fateseal.
    pub count: Value,
    /// The player who fateseals.
    pub player: PlayerFilter,
}

impl FatesealEffect {
    /// Create a new fateseal effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// The controller fateseals N.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for FatesealEffect {
    #[crate::effects::project_effect_outcome]
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let fatesealer = resolve_player_filter(game, &self.player, ctx)?;
                let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

                if count == 0 {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let Some(opponent) = choose_fateseal_opponent(game, ctx, fatesealer) else {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                complete_individual_arrangement(
                    game,
                    ctx,
                    fatesealer,
                    opponent,
                    count,
                    KeywordActionKind::Fateseal,
                )
            },
        )
    }
}

/// Effect that makes multiple players scry at once.
pub type EachPlayerScryEffect = ironsmith_core::EachPlayerScryEffect;

impl EffectExecutor for EachPlayerScryEffect {
    #[crate::effects::project_effect_outcome]
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let count = resolve_value(game, &self.count, ctx)?;
        if count <= 0 {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        crate::effects::ForPlayersEffect::new(
            self.player_filter.clone(),
            vec![crate::effect::Effect::new(ScryEffect::new(
                count,
                PlayerFilter::IteratedPlayer,
            ))],
        )
        .execute_with_outputs(game, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(ironsmith_runtime_parser_tests)]
    use crate::cards::CardDefinitionBuilder;
    use crate::decision::DecisionMaker;
    use crate::effects::ExecutionContext;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn setup_three_player_game() -> GameState {
        GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        )
    }

    fn add_library_card(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let id = game.new_object_id();
        let card = crate::card::CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Instant])
            .build();
        let object = Object::from_card(id, &card, owner, Zone::Library);
        game.add_object(object);
        id
    }

    fn library_names_bottom_to_top(game: &GameState, player: PlayerId) -> Vec<String> {
        game.player(player)
            .expect("player should exist")
            .library
            .iter()
            .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
            .collect()
    }

    struct ScriptedScryDecisionMaker {
        partitions: std::collections::HashMap<PlayerId, Vec<ObjectId>>,
        top_orders: std::collections::HashMap<PlayerId, Vec<ObjectId>>,
        bottom_orders: std::collections::HashMap<PlayerId, Vec<ObjectId>>,
        option_choices: std::collections::HashMap<PlayerId, Vec<usize>>,
        partition_calls: Vec<PlayerId>,
    }

    impl ScriptedScryDecisionMaker {
        fn new() -> Self {
            Self {
                partitions: std::collections::HashMap::new(),
                top_orders: std::collections::HashMap::new(),
                bottom_orders: std::collections::HashMap::new(),
                option_choices: std::collections::HashMap::new(),
                partition_calls: Vec::new(),
            }
        }
    }

    impl DecisionMaker for ScriptedScryDecisionMaker {
        fn decide_partition(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::PartitionContext,
        ) -> Vec<ObjectId> {
            self.partition_calls.push(ctx.player);
            self.partitions
                .get(&ctx.player)
                .cloned()
                .unwrap_or_default()
        }

        fn decide_order(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::OrderContext,
        ) -> Vec<ObjectId> {
            let description = ctx.description.to_ascii_lowercase();
            if description.contains("bottom") {
                return self
                    .bottom_orders
                    .get(&ctx.player)
                    .cloned()
                    .unwrap_or_else(|| ctx.items.iter().map(|(id, _)| *id).collect());
            }
            self.top_orders
                .get(&ctx.player)
                .cloned()
                .unwrap_or_else(|| ctx.items.iter().map(|(id, _)| *id).collect())
        }

        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.option_choices
                .get(&ctx.player)
                .cloned()
                .unwrap_or_else(|| {
                    ctx.options
                        .iter()
                        .filter(|option| option.legal)
                        .map(|option| option.index)
                        .take(ctx.min)
                        .collect()
                })
        }
    }

    #[test]
    fn scry_zero_emits_no_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let a = add_library_card(&mut game, alice, "A");
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ScryEffect::you(0)
            .execute(&mut game, &mut ctx)
            .expect("scry 0 should resolve");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(0));
        assert!(outcome.events.is_empty());
        assert_eq!(
            library_names_bottom_to_top(&game, alice),
            vec!["A".to_string()]
        );
        assert!(game.player(alice).expect("alice").library.contains(&a));
    }

    #[test]
    fn scry_can_reorder_cards_kept_on_top() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let a = add_library_card(&mut game, alice, "A");
        let b = add_library_card(&mut game, alice, "B");
        let c = add_library_card(&mut game, alice, "C");
        let source = game.new_object_id();
        let mut decision_maker = ScriptedScryDecisionMaker::new();
        decision_maker.top_orders.insert(alice, vec![b, c]);
        let outcome = {
            let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
            ScryEffect::you(2)
                .execute(&mut game, &mut ctx)
                .expect("scry should resolve")
        };

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            library_names_bottom_to_top(&game, alice),
            vec!["A".to_string(), "C".to_string(), "B".to_string()]
        );
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(
            top_library_cards_top_to_bottom(&game, alice, 3),
            vec![b, c, a]
        );
    }

    #[test]
    fn scry_can_reorder_cards_moved_to_bottom() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let a = add_library_card(&mut game, alice, "A");
        let b = add_library_card(&mut game, alice, "B");
        let c = add_library_card(&mut game, alice, "C");
        let source = game.new_object_id();
        let mut decision_maker = ScriptedScryDecisionMaker::new();
        decision_maker.partitions.insert(alice, vec![c, b]);
        decision_maker.bottom_orders.insert(alice, vec![c, b]);
        {
            let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
            ScryEffect::you(2)
                .execute(&mut game, &mut ctx)
                .expect("scry should resolve");
        }

        assert_eq!(
            library_names_bottom_to_top(&game, alice),
            vec!["B".to_string(), "C".to_string(), "A".to_string()]
        );
        assert_eq!(
            top_library_cards_top_to_bottom(&game, alice, 3),
            vec![a, c, b]
        );
    }

    #[test]
    fn fateseal_reorders_opponents_library_and_emits_fateseal_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let a = add_library_card(&mut game, bob, "A");
        let b = add_library_card(&mut game, bob, "B");
        let c = add_library_card(&mut game, bob, "C");
        let source = game.new_object_id();
        let mut decision_maker = ScriptedScryDecisionMaker::new();
        decision_maker.partitions.insert(alice, vec![c]);
        decision_maker.top_orders.insert(alice, vec![b]);
        decision_maker.bottom_orders.insert(alice, vec![c]);

        let outcome = {
            let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
            FatesealEffect::you(2)
                .execute(&mut game, &mut ctx)
                .expect("fateseal should resolve")
        };

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            library_names_bottom_to_top(&game, bob),
            vec!["C".to_string(), "A".to_string(), "B".to_string()]
        );
        assert_eq!(
            top_library_cards_top_to_bottom(&game, bob, 3),
            vec![b, a, c]
        );
        assert_eq!(outcome.events.len(), 1);
        let event = outcome.events[0]
            .downcast::<KeywordActionEvent>()
            .expect("expected keyword action event");
        assert_eq!(event.action, KeywordActionKind::Fateseal);
        assert_eq!(event.player, alice);
        assert_eq!(event.source, source);
        assert_eq!(event.amount, 2);
        assert!(game.player(bob).expect("bob").library.contains(&a));
    }

    #[test]
    fn fateseal_multiplayer_chooses_an_opponent_without_targeting() {
        let mut game = setup_three_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let _b1 = add_library_card(&mut game, bob, "B1");
        let b2 = add_library_card(&mut game, bob, "B2");
        let c1 = add_library_card(&mut game, charlie, "C1");
        let _c2 = add_library_card(&mut game, charlie, "C2");
        let source = game.new_object_id();
        let mut decision_maker = ScriptedScryDecisionMaker::new();
        decision_maker.option_choices.insert(alice, vec![1]);
        decision_maker.partitions.insert(alice, vec![c1]);
        decision_maker.bottom_orders.insert(alice, vec![c1]);

        let outcome = {
            let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
            FatesealEffect::you(2)
                .execute(&mut game, &mut ctx)
                .expect("fateseal should resolve in multiplayer")
        };

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            library_names_bottom_to_top(&game, bob),
            vec!["B1".to_string(), "B2".to_string()]
        );
        assert_eq!(
            library_names_bottom_to_top(&game, charlie),
            vec!["C1".to_string(), "C2".to_string()]
        );
        assert_eq!(
            top_library_cards_top_to_bottom(&game, bob, 2),
            vec![b2, _b1]
        );
        assert_eq!(
            top_library_cards_top_to_bottom(&game, charlie, 2),
            vec![_c2, c1]
        );
        assert_eq!(decision_maker.partition_calls, vec![alice]);
    }

    #[test]
    fn each_player_scry_uses_apnap_choice_order_and_moves_after_all_choices() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;

        let _a1 = add_library_card(&mut game, alice, "A1");
        let a2 = add_library_card(&mut game, alice, "A2");
        let b1 = add_library_card(&mut game, bob, "B1");
        let _b2 = add_library_card(&mut game, bob, "B2");

        let source = game.new_object_id();
        let mut decision_maker = ScriptedScryDecisionMaker::new();
        decision_maker.partitions.insert(alice, vec![a2]);
        decision_maker.bottom_orders.insert(alice, vec![a2]);
        decision_maker.partitions.insert(bob, vec![b1]);
        decision_maker.bottom_orders.insert(bob, vec![b1]);
        let outcome = {
            let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
            EachPlayerScryEffect::new(2, PlayerFilter::Any)
                .execute(&mut game, &mut ctx)
                .expect("each-player scry should resolve")
        };
        let partition_calls = decision_maker.partition_calls.clone();

        assert_eq!(partition_calls, vec![alice, bob]);
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(4));
        let events: Vec<_> = outcome
            .events
            .iter()
            .filter_map(|event| event.downcast::<KeywordActionEvent>())
            .map(|event| (event.action, event.player, event.amount))
            .collect();
        assert_eq!(
            events,
            [
                (KeywordActionKind::Scry, alice, 2),
                (KeywordActionKind::ScryToBottom, alice, 1),
                (KeywordActionKind::Scry, bob, 2),
                (KeywordActionKind::ScryToBottom, bob, 1)
            ]
        );
        assert_eq!(
            library_names_bottom_to_top(&game, alice),
            vec!["A2".to_string(), "A1".to_string()]
        );
        assert_eq!(
            library_names_bottom_to_top(&game, bob),
            vec!["B1".to_string(), "B2".to_string()]
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_each_player_scries_uses_simultaneous_scry_effect() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Shared Visions")
            .card_types(vec![CardType::Sorcery])
            .parse_text("Each player scries 1.")
            .expect("each-player scry should parse");

        let debug = format!("{:?}", definition.spell_effect);
        let rendered = crate::runtime_display::unprocessed_compiled_lines(&definition).join(" ");
        assert!(
            debug.contains("EachPlayerScryEffect"),
            "expected each-player scry lowering, got {debug}"
        );
        assert!(
            !debug.contains("ForPlayersEffect"),
            "each-player scry should not lower through generic per-player sequencing, got {debug}"
        );
        assert!(
            rendered.contains("Each player scries 1"),
            "expected rendered text to preserve each-player scry wording, got {rendered}"
        );
    }
}

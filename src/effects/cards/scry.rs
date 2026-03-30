//! Scry effect implementation.

use crate::decisions::{ScrySpec, make_decision};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::filter::FilterContext;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;

fn players_in_turn_order(game: &GameState) -> Vec<PlayerId> {
    if game.turn_order.is_empty() {
        return Vec::new();
    }

    let start = game
        .turn_order
        .iter()
        .position(|&player_id| player_id == game.turn.active_player)
        .unwrap_or(0);

    (0..game.turn_order.len())
        .filter_map(|offset| {
            let player_id = game.turn_order[(start + offset) % game.turn_order.len()];
            game.player(player_id)
                .filter(|player| player.is_in_game())
                .map(|_| player_id)
        })
        .collect()
}

fn normalize_order_response(response: Vec<ObjectId>, original: &[ObjectId]) -> Vec<ObjectId> {
    let mut remaining = original.to_vec();
    let mut out = Vec::with_capacity(original.len());
    for id in response {
        if let Some(pos) = remaining.iter().position(|candidate| *candidate == id) {
            out.push(id);
            remaining.remove(pos);
        }
    }
    out.extend(remaining);
    out
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

fn reorder_cards_top_to_bottom(
    game: &GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    description: &str,
    cards_top_to_bottom: &[ObjectId],
) -> Vec<ObjectId> {
    if cards_top_to_bottom.len() <= 1 {
        return cards_top_to_bottom.to_vec();
    }

    let items: Vec<(ObjectId, String)> = cards_top_to_bottom
        .iter()
        .map(|&id| {
            let name = game
                .object(id)
                .map(|object| object.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            (id, name)
        })
        .collect();
    let order_ctx =
        crate::decisions::context::OrderContext::new(player_id, Some(ctx.source), description, items);
    normalize_order_response(
        ctx.decision_maker.decide_order(game, &order_ctx),
        cards_top_to_bottom,
    )
}

#[derive(Debug, Clone, PartialEq)]
struct ScryArrangement {
    player_id: PlayerId,
    total_looked: usize,
    top_cards_top_to_bottom: Vec<ObjectId>,
    bottom_cards_top_to_bottom: Vec<ObjectId>,
}

fn choose_scry_arrangement(
    game: &GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    count: usize,
) -> ScryArrangement {
    let top_cards_top_to_bottom = top_library_cards_top_to_bottom(game, player_id, count);
    if top_cards_top_to_bottom.is_empty() {
        return ScryArrangement {
            player_id,
            total_looked: 0,
            top_cards_top_to_bottom: Vec::new(),
            bottom_cards_top_to_bottom: Vec::new(),
        };
    }

    let spec = ScrySpec::new(ctx.source, top_cards_top_to_bottom.clone());
    let bottom_cards_top_to_bottom: Vec<ObjectId> = make_decision(
        game,
        &mut ctx.decision_maker,
        player_id,
        Some(ctx.source),
        spec,
    )
    .into_iter()
    .filter(|card| top_cards_top_to_bottom.contains(card))
    .collect();

    let kept_on_top: Vec<ObjectId> = top_cards_top_to_bottom
        .iter()
        .filter(|card| !bottom_cards_top_to_bottom.contains(card))
        .copied()
        .collect();
    let ordered_top_cards = reorder_cards_top_to_bottom(
        game,
        ctx,
        player_id,
        "Reorder cards to keep on top of your library",
        &kept_on_top,
    );
    let ordered_bottom_cards = reorder_cards_top_to_bottom(
        game,
        ctx,
        player_id,
        "Reorder cards to put on the bottom of your library",
        &bottom_cards_top_to_bottom,
    );

    ScryArrangement {
        player_id,
        total_looked: top_cards_top_to_bottom.len(),
        top_cards_top_to_bottom: ordered_top_cards,
        bottom_cards_top_to_bottom: ordered_bottom_cards,
    }
}

fn apply_scry_arrangement(game: &mut GameState, arrangement: &ScryArrangement) {
    if arrangement.total_looked == 0 {
        return;
    }

    let looked_set: std::collections::HashSet<_> = arrangement
        .top_cards_top_to_bottom
        .iter()
        .chain(arrangement.bottom_cards_top_to_bottom.iter())
        .copied()
        .collect();

    let Some(player) = game.player_mut(arrangement.player_id) else {
        return;
    };

    player.library.retain(|id| !looked_set.contains(id));

    for id in &arrangement.bottom_cards_top_to_bottom {
        player.library.insert(0, *id);
    }
    for id in arrangement.top_cards_top_to_bottom.iter().rev() {
        player.library.push(*id);
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
#[derive(Debug, Clone, PartialEq)]
pub struct ScryEffect {
    /// Number of cards to scry.
    pub count: Value,
    /// The player who scries.
    pub player: PlayerFilter,
}

impl ScryEffect {
    /// Create a new scry effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// The controller scries N.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for ScryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        if count == 0 {
            return Ok(EffectOutcome::count(0));
        }
        let arrangement = choose_scry_arrangement(game, ctx, player_id, count);
        if arrangement.total_looked == 0 {
            return Ok(EffectOutcome::count(0));
        }
        apply_scry_arrangement(game, &arrangement);

        Ok(
            EffectOutcome::count(arrangement.total_looked as i32).with_event(
                TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::Scry,
                    player_id,
                    ctx.source,
                    arrangement.total_looked as u32,
                ),
                ctx.provenance,
            )),
        )
    }
}

/// Effect that makes multiple players scry at once.
#[derive(Debug, Clone, PartialEq)]
pub struct EachPlayerScryEffect {
    pub count: Value,
    pub player_filter: PlayerFilter,
}

impl EachPlayerScryEffect {
    pub fn new(count: impl Into<Value>, player_filter: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player_filter,
        }
    }
}

impl EffectExecutor for EachPlayerScryEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        if count == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let filter_ctx = FilterContext::new(ctx.controller).with_source(ctx.source);
        let players: Vec<PlayerId> = players_in_turn_order(game)
            .into_iter()
            .filter(|player_id| self.player_filter.matches_player(*player_id, &filter_ctx))
            .collect();
        if players.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut arrangements = Vec::new();
        for player_id in players {
            let arrangement = choose_scry_arrangement(game, ctx, player_id, count);
            if arrangement.total_looked > 0 {
                arrangements.push(arrangement);
            }
        }
        if arrangements.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        for arrangement in &arrangements {
            apply_scry_arrangement(game, arrangement);
        }

        let total = arrangements.iter().map(|a| a.total_looked as i32).sum();
        let events = arrangements.into_iter().map(|arrangement| {
            TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::Scry,
                    arrangement.player_id,
                    ctx.source,
                    arrangement.total_looked as u32,
                ),
                ctx.provenance,
            )
        });

        Ok(EffectOutcome::count(total).with_events(events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::CardDefinitionBuilder;
    use crate::decision::DecisionMaker;
    use crate::executor::ExecutionContext;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_library_card(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
    ) -> ObjectId {
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
            .filter_map(|id| game.object(*id).map(|object| object.name.clone()))
            .collect()
    }

    struct ScriptedScryDecisionMaker {
        partitions: std::collections::HashMap<PlayerId, Vec<ObjectId>>,
        top_orders: std::collections::HashMap<PlayerId, Vec<ObjectId>>,
        bottom_orders: std::collections::HashMap<PlayerId, Vec<ObjectId>>,
        partition_calls: Vec<PlayerId>,
    }

    impl ScriptedScryDecisionMaker {
        fn new() -> Self {
            Self {
                partitions: std::collections::HashMap::new(),
                top_orders: std::collections::HashMap::new(),
                bottom_orders: std::collections::HashMap::new(),
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
        assert_eq!(library_names_bottom_to_top(&game, alice), vec!["A".to_string()]);
        assert!(
            game.player(alice)
                .expect("alice")
                .library
                .contains(&a)
        );
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
        assert_eq!(outcome.events.len(), 2);
        assert_eq!(
            library_names_bottom_to_top(&game, alice),
            vec!["A2".to_string(), "A1".to_string()]
        );
        assert_eq!(
            library_names_bottom_to_top(&game, bob),
            vec!["B1".to_string(), "B2".to_string()]
        );
    }

    #[test]
    fn parse_each_player_scries_uses_simultaneous_scry_effect() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Shared Visions")
            .card_types(vec![CardType::Sorcery])
            .parse_text("Each player scries 1.")
            .expect("each-player scry should parse");

        let debug = format!("{:?}", definition.spell_effect);
        let rendered = crate::compiled_text::compiled_lines(&definition).join(" ");
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

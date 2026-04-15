//! Surveil effect implementation.

use crate::decisions::{SurveilSpec, make_decision};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

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
    let order_ctx = crate::decisions::context::OrderContext::new(
        player_id,
        Some(ctx.source),
        description,
        items,
    );
    normalize_order_response(
        ctx.decision_maker.decide_order(game, &order_ctx),
        cards_top_to_bottom,
    )
}

/// Effect that lets a player surveil N cards.
///
/// Per Rule 701.25, look at the top N cards, then put any number into your
/// graveyard and the rest on top of your library in any order.
///
/// # Fields
///
/// * `count` - Number of cards to surveil
/// * `player` - The player who surveils
///
/// # Example
///
/// ```ignore
/// // Surveil 2
/// let effect = SurveilEffect::new(2, PlayerFilter::You);
///
/// // Surveil 1
/// let effect = SurveilEffect::you(1);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SurveilEffect {
    /// Number of cards to surveil.
    pub count: Value,
    /// The player who surveils.
    pub player: PlayerFilter,
}

impl SurveilEffect {
    /// Create a new surveil effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// The controller surveils N.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for SurveilEffect {
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

        // Get the top N cards (they're at the end of the library vec)
        let top_cards_top_to_bottom: Vec<ObjectId> = game
            .player(player_id)
            .map(|p| p.library.iter().rev().take(count).copied().collect())
            .unwrap_or_default();

        if top_cards_top_to_bottom.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let surveil_count = top_cards_top_to_bottom.len();

        // Ask player which cards to put in graveyard using the new spec-based system
        let spec = SurveilSpec::new(ctx.source, top_cards_top_to_bottom.clone());
        let cards_to_graveyard: Vec<ObjectId> = make_decision(
            game,
            &mut ctx.decision_maker,
            player_id,
            Some(ctx.source),
            spec,
        )
        .into_iter()
        .filter(|c| top_cards_top_to_bottom.contains(c))
        .collect();

        let kept_on_top_top_to_bottom: Vec<ObjectId> = top_cards_top_to_bottom
            .iter()
            .filter(|c| !cards_to_graveyard.contains(c))
            .copied()
            .collect();
        let ordered_top_cards = reorder_cards_top_to_bottom(
            game,
            ctx,
            player_id,
            "Reorder cards to keep on top of your library",
            &kept_on_top_top_to_bottom,
        );

        // Put cards going to graveyard
        for &card_id in &cards_to_graveyard {
            game.move_object_by_effect(card_id, Zone::Graveyard);
        }

        // Put the rest back on top
        if let Some(p) = game.player_mut(player_id) {
            p.library.retain(|id| !ordered_top_cards.contains(id));
            for id in ordered_top_cards.iter().rev() {
                p.library.push(*id);
            }
        }

        Ok(EffectOutcome::count(surveil_count as i32).with_event(
            TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::Surveil,
                    player_id,
                    ctx.source,
                    surveil_count as u32,
                ),
                ctx.provenance,
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            .filter_map(|id| game.object(*id).map(|object| object.name.clone()))
            .collect()
    }

    fn graveyard_names_top_to_bottom(game: &GameState, player: PlayerId) -> Vec<String> {
        game.player(player)
            .expect("player should exist")
            .graveyard
            .iter()
            .rev()
            .filter_map(|id| game.object(*id).map(|object| object.name.clone()))
            .collect()
    }

    struct ScriptedSurveilDecisionMaker {
        partition: Vec<ObjectId>,
        top_order: Vec<ObjectId>,
    }

    impl DecisionMaker for ScriptedSurveilDecisionMaker {
        fn decide_partition(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::PartitionContext,
        ) -> Vec<ObjectId> {
            self.partition.clone()
        }

        fn decide_order(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::OrderContext,
        ) -> Vec<ObjectId> {
            if ctx.description.contains("keep on top") {
                return self.top_order.clone();
            }
            ctx.items.iter().map(|(id, _)| *id).collect()
        }
    }

    #[test]
    fn surveil_zero_emits_no_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let a = add_library_card(&mut game, alice, "A");
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = SurveilEffect::you(0)
            .execute(&mut game, &mut ctx)
            .expect("surveil 0 should resolve");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(0));
        assert!(outcome.events.is_empty());
        assert_eq!(
            library_names_bottom_to_top(&game, alice),
            vec!["A".to_string()]
        );
        assert!(game.player(alice).expect("alice").library.contains(&a));
    }

    #[test]
    fn surveil_can_move_selected_cards_to_graveyard_and_reorder_the_rest() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let _a = add_library_card(&mut game, alice, "A");
        let b = add_library_card(&mut game, alice, "B");
        let c = add_library_card(&mut game, alice, "C");
        let d = add_library_card(&mut game, alice, "D");
        let source = game.new_object_id();
        let mut decision_maker = ScriptedSurveilDecisionMaker {
            partition: vec![c],
            top_order: vec![b, d],
        };

        let outcome = {
            let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
            SurveilEffect::you(3)
                .execute(&mut game, &mut ctx)
                .expect("surveil should resolve")
        };

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(3));
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(
            library_names_bottom_to_top(&game, alice),
            vec!["A".to_string(), "D".to_string(), "B".to_string()]
        );
        assert_eq!(
            graveyard_names_top_to_bottom(&game, alice),
            vec!["C".to_string()]
        );
    }

    #[test]
    fn surveil_order_normalization_ignores_invalid_ids_and_keeps_unspecified_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let _a = add_library_card(&mut game, alice, "A");
        let b = add_library_card(&mut game, alice, "B");
        let _c = add_library_card(&mut game, alice, "C");
        let _d = add_library_card(&mut game, alice, "D");
        let bogus = ObjectId::from_raw(999_999);
        let source = game.new_object_id();
        let mut decision_maker = ScriptedSurveilDecisionMaker {
            partition: vec![],
            top_order: vec![b, bogus],
        };

        {
            let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
            SurveilEffect::you(3)
                .execute(&mut game, &mut ctx)
                .expect("surveil should resolve");
        }

        assert_eq!(
            library_names_bottom_to_top(&game, alice),
            vec![
                "A".to_string(),
                "C".to_string(),
                "D".to_string(),
                "B".to_string()
            ]
        );
    }
}

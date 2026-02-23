//! Cascade keyword effect implementation.
//!
//! Exiles cards from the top of your library until a nonland card with lesser
//! mana value is exiled, lets you cast it without paying its mana cost, then
//! puts all other exiled cards on the bottom of your library in random order.

use rand::seq::SliceRandom;

use crate::alternative_cast::CastingMethod;
use crate::cost::OptionalCostsPaid;
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::mana::{ManaCost, ManaSymbol};
use crate::zone::Zone;

/// Effect that resolves a single cascade trigger.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CascadeEffect;

impl CascadeEffect {
    /// Create a new cascade effect.
    pub fn new() -> Self {
        Self
    }
}

fn mana_value_on_stack(cost: Option<&ManaCost>, x_value: Option<u32>) -> u32 {
    let Some(cost) = cost else {
        return 0;
    };
    let x = x_value.unwrap_or(0);
    let x_pips = cost
        .pips()
        .iter()
        .filter(|pip| pip.iter().any(|symbol| matches!(symbol, ManaSymbol::X)))
        .count() as u32;
    cost.mana_value() + x_pips.saturating_mul(x)
}

impl EffectExecutor for CascadeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let (source_mana_value, source_name) = if let Some(source_obj) = game.object(ctx.source) {
            (
                mana_value_on_stack(
                    source_obj.mana_cost.as_ref(),
                    ctx.x_value.or(source_obj.x_value),
                ),
                source_obj.name.clone(),
            )
        } else if let Some(snapshot) = ctx.source_snapshot.as_ref() {
            (
                mana_value_on_stack(snapshot.mana_cost.as_ref(), ctx.x_value.or(snapshot.x_value)),
                snapshot.name.clone(),
            )
        } else {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        };

        let mut exiled = Vec::new();
        let mut candidate = None;

        loop {
            let top_card = game
                .player(ctx.controller)
                .and_then(|player| player.library.last().copied());
            let Some(top_card_id) = top_card else {
                break;
            };

            let Some(exiled_id) = game.move_object(top_card_id, Zone::Exile) else {
                break;
            };
            exiled.push(exiled_id);

            let Some(card) = game.object(exiled_id) else {
                continue;
            };
            if card.is_land() {
                continue;
            }
            let card_mana_value = card.mana_cost.as_ref().map_or(0, ManaCost::mana_value);
            if card_mana_value < source_mana_value {
                candidate = Some(exiled_id);
                break;
            }
        }

        let mut casted_card = None;
        if let Some(candidate_id) = candidate {
            let Some(candidate_obj) = game.object(candidate_id) else {
                return Ok(EffectOutcome::count(exiled.len() as i32));
            };
            let candidate_name = candidate_obj.name.clone();

            let choice_ctx = crate::decisions::context::BooleanContext::new(
                ctx.controller,
                Some(candidate_id),
                format!("Cast {candidate_name} without paying its mana cost?"),
            )
            .with_source_name(&source_name);
            let should_cast = ctx.decision_maker.decide_boolean(game, &choice_ctx);

            if should_cast {
                let from_zone = candidate_obj.zone;
                let mana_cost = candidate_obj.mana_cost.clone();
                let stable_id = candidate_obj.stable_id;
                let x_value = mana_cost
                    .as_ref()
                    .and_then(|cost| if cost.has_x() { Some(0u32) } else { None });

                if let Some(new_id) = game.move_object(candidate_id, Zone::Stack) {
                    if let Some(obj) = game.object_mut(new_id) {
                        obj.x_value = x_value;
                    }

                    let stack_entry = StackEntry {
                        object_id: new_id,
                        controller: ctx.controller,
                        targets: vec![],
                        x_value,
                        ability_effects: None,
                        is_ability: false,
                        casting_method: CastingMethod::PlayFrom {
                            source: ctx.source,
                            zone: from_zone,
                            use_alternative: None,
                        },
                        optional_costs_paid: OptionalCostsPaid::default(),
                        defending_player: None,
                        saga_final_chapter_source: None,
                        source_stable_id: Some(stable_id),
                        source_snapshot: None,
                        source_name: Some(candidate_name),
                        triggering_event: None,
                        intervening_if: None,
                        keyword_payment_contributions: vec![],
                        crew_contributors: vec![],
                        saddle_contributors: vec![],
                        chosen_modes: None,
                    };

                    game.push_to_stack(stack_entry);
                    casted_card = Some((candidate_id, new_id));
                }
            }
        }

        let mut to_bottom = exiled;
        if let Some((casted_from_exile, _)) = casted_card {
            to_bottom.retain(|id| *id != casted_from_exile);
        }
        to_bottom.shuffle(&mut rand::rng());

        for exiled_id in to_bottom {
            if let Some(new_id) = game.move_object(exiled_id, Zone::Library) {
                let owner = game.object(new_id).map(|obj| obj.owner);
                if let Some(owner) = owner
                    && let Some(player) = game.player_mut(owner)
                    && let Some(pos) = player.library.iter().position(|id| *id == new_id)
                {
                    player.library.remove(pos);
                    player.library.insert(0, new_id);
                }
            }
        }

        if let Some((_, casted_id)) = casted_card {
            Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
                casted_id,
            ])))
        } else {
            Ok(EffectOutcome::count(0))
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_spell(card_id: u32, name: &str, mana_value: u8) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(mana_value)]))
            .card_types(vec![CardType::Sorcery])
            .build()
    }

    fn make_land(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .card_types(vec![CardType::Land])
            .build()
    }

    #[test]
    fn cascade_exiles_until_lesser_and_casts() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = make_spell(1, "Cascade Source", 3);
        let source_id = game.create_object_from_card(&source, alice, Zone::Stack);

        let lesser = make_spell(2, "Lesser Spell", 2);
        let top_land = make_land(3, "Top Land");
        game.create_object_from_card(&lesser, alice, Zone::Library);
        game.create_object_from_card(&top_land, alice, Zone::Library);

        let effect = CascadeEffect::new();
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        let result = effect.execute(&mut game, &mut ctx).expect("execute cascade");
        assert!(
            matches!(result.result, EffectResult::Objects(ref ids) if ids.len() == 1),
            "expected cascade to cast one card, got {:?}",
            result.result
        );

        let stack_names: Vec<String> = game
            .stack
            .iter()
            .filter_map(|entry| game.object(entry.object_id).map(|obj| obj.name.clone()))
            .collect();
        assert!(
            stack_names.iter().any(|name| name == "Lesser Spell"),
            "expected Lesser Spell on stack, got {:?}",
            stack_names
        );

        let library_names: Vec<String> = game
            .player(alice)
            .expect("alice")
            .library
            .iter()
            .filter_map(|id| game.object(*id).map(|obj| obj.name.clone()))
            .collect();
        assert_eq!(library_names.len(), 1);
        assert_eq!(library_names[0], "Top Land");
        assert_eq!(game.exile.len(), 0);
    }

    struct DeclineDecisionMaker;
    impl DecisionMaker for DeclineDecisionMaker {}

    #[test]
    fn cascade_decline_keeps_exiled_cards_in_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = make_spell(10, "Cascade Source", 3);
        let source_id = game.create_object_from_card(&source, alice, Zone::Stack);

        let lesser = make_spell(11, "Lesser Spell", 2);
        let top_land = make_land(12, "Top Land");
        game.create_object_from_card(&lesser, alice, Zone::Library);
        game.create_object_from_card(&top_land, alice, Zone::Library);

        let effect = CascadeEffect::new();
        let mut dm = DeclineDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        effect.execute(&mut game, &mut ctx).expect("execute cascade");

        assert!(
            game.stack.is_empty(),
            "declining cascade cast should not put cards on stack"
        );
        assert_eq!(game.exile.len(), 0);
        let library_names: Vec<String> = game
            .player(alice)
            .expect("alice")
            .library
            .iter()
            .filter_map(|id| game.object(*id).map(|obj| obj.name.clone()))
            .collect();
        assert_eq!(library_names.len(), 2);
        assert!(library_names.iter().any(|name| name == "Lesser Spell"));
        assert!(library_names.iter().any(|name| name == "Top Land"));
    }
}

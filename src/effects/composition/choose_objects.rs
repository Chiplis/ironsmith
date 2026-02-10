//! ChooseObjects effect implementation.
//!
//! This effect allows a player to choose objects matching a filter and tag them
//! for reference by subsequent effects in the same spell/ability.

use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{ChoiceCount, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

/// Effect that prompts a player to choose objects matching a filter and tags them.
///
/// This enables patterns like sacrifice costs and interactive selections:
/// - "Sacrifice a creature" → ChooseObjectsEffect + SacrificeEffect
/// - "Choose a creature you control" → ChooseObjectsEffect (for later reference)
///
/// # Fields
///
/// * `filter` - Filter for which objects can be chosen
/// * `count` - Number of objects to choose
/// * `chooser` - Which player makes the choice
/// * `zone` - Zone to search for objects (default: Battlefield)
/// * `tag` - Tag name to store chosen objects under
/// * `description` - Human-readable description for the UI
///
/// # Result
///
/// Returns `EffectResult::Objects(chosen_ids)` with the chosen object IDs.
/// If no valid objects exist, returns `EffectResult::Count(0)`.
///
/// # Example
///
/// ```ignore
/// // "Sacrifice a creature" as composed effects:
/// vec![
///     Effect::choose_objects(
///         ObjectFilter::creature().you_control(),
///         1,
///         PlayerFilter::You,
///         "sacrificed",
///     ),
///     Effect::sacrifice(ChooseSpec::tagged("sacrificed")),
/// ]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseObjectsEffect {
    /// Filter for which objects can be chosen.
    pub filter: ObjectFilter,
    /// Number of objects to choose.
    pub count: ChoiceCount,
    /// Which player makes the choice.
    pub chooser: PlayerFilter,
    /// Zone to search for objects.
    pub zone: Zone,
    /// Tag name to store chosen objects under.
    pub tag: TagKey,
    /// Human-readable description for the decision prompt.
    pub description: &'static str,
    /// Whether this choice represents a library search.
    pub is_search: bool,
    /// Restrict selection to only the top-most matching object in ordered zones.
    pub top_only: bool,
}

impl ChooseObjectsEffect {
    /// Create a new ChooseObjectsEffect.
    pub fn new(
        filter: ObjectFilter,
        count: impl Into<ChoiceCount>,
        chooser: PlayerFilter,
        tag: impl Into<TagKey>,
    ) -> Self {
        Self {
            filter,
            count: count.into(),
            chooser,
            zone: Zone::Battlefield,
            tag: tag.into(),
            description: "Choose",
            is_search: false,
            top_only: false,
        }
    }

    /// Set the zone to search for objects.
    pub fn in_zone(mut self, zone: Zone) -> Self {
        self.zone = zone;
        self
    }

    /// Set a custom description for the decision prompt.
    pub fn with_description(mut self, description: &'static str) -> Self {
        self.description = description;
        self
    }

    /// Mark this choice as a library search (respects search restrictions).
    pub fn as_search(mut self) -> Self {
        self.is_search = true;
        self
    }

    /// Restrict selection to the top-most matching object in ordered zones.
    pub fn top_only(mut self) -> Self {
        self.top_only = true;
        self
    }
}

impl EffectExecutor for ChooseObjectsEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        use crate::effects::CostValidationError;
        use crate::filter::FilterContext;

        if self.count.min == 0 {
            return Ok(());
        }

        // Create a filter context for checking
        let filter_ctx = FilterContext::new(controller).with_source(source);

        // Resolve the chooser (for cost validation, usually "you")
        let chooser_id = match self.chooser {
            PlayerFilter::You => controller,
            _ => controller, // Default to controller for validation
        };

        // Find candidates based on the zone - check the filter's zone if set
        let search_zone = self.filter.zone.unwrap_or(self.zone);

        let mut candidate_count = match search_zone {
            Zone::Battlefield => game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| {
                    // Apply "other" filter - exclude source
                    if self.filter.other && obj.id == source {
                        return false;
                    }
                    self.filter.matches(obj, &filter_ctx, game)
                })
                .count(),
            Zone::Hand => {
                if let Some(player) = game.player(chooser_id) {
                    player
                        .hand
                        .iter()
                        .filter_map(|&id| game.object(id))
                        .filter(|obj| {
                            // Apply "other" filter - exclude source
                            if self.filter.other && obj.id == source {
                                return false;
                            }
                            self.filter.matches(obj, &filter_ctx, game)
                        })
                        .count()
                } else {
                    0
                }
            }
            Zone::Graveyard => {
                if let Some(player) = game.player(chooser_id) {
                    if self.top_only {
                        player
                            .graveyard
                            .iter()
                            .rev()
                            .filter_map(|&id| game.object(id))
                            .find(|obj| {
                                if self.filter.other && obj.id == source {
                                    return false;
                                }
                                self.filter.matches(obj, &filter_ctx, game)
                            })
                            .map(|_| 1usize)
                            .unwrap_or(0usize)
                    } else {
                        player
                            .graveyard
                            .iter()
                            .filter_map(|&id| game.object(id))
                            .filter(|obj| {
                                if self.filter.other && obj.id == source {
                                    return false;
                                }
                                self.filter.matches(obj, &filter_ctx, game)
                            })
                            .count()
                    }
                } else {
                    0
                }
            }
            _ => {
                // For other zones, check generic
                game.objects_in_zone(search_zone)
                    .into_iter()
                    .filter_map(|id| game.object(id))
                    .filter(|obj| {
                        if self.filter.other && obj.id == source {
                            return false;
                        }
                        self.filter.matches(obj, &filter_ctx, game)
                    })
                    .count()
            }
        };

        if self.top_only && candidate_count > 1 {
            candidate_count = 1;
        }

        if candidate_count < self.count.min {
            return Err(CostValidationError::Other(format!(
                "Not enough objects to choose ({} needed, {} available)",
                self.count.min, candidate_count
            )));
        }

        Ok(())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Resolve the chooser
        let chooser_id = resolve_player_filter(game, &self.chooser, ctx)?;

        // If this is a library search, honor search restrictions and track it.
        if self.is_search && !game.can_search_library(chooser_id) {
            return Ok(EffectOutcome::from_result(EffectResult::Prevented));
        }
        if self.is_search {
            game.library_searches_this_turn.insert(chooser_id);
        }

        // Get filter context - use the controller's perspective for "you control" filters
        let filter_ctx = ctx.filter_context(game);

        // Determine the zone to search - prefer filter's zone if set, otherwise use effect's zone
        let search_zone = self.filter.zone.unwrap_or(self.zone);

        // Find all valid objects in the specified zone
        let candidates: Vec<ObjectId> = match search_zone {
            Zone::Battlefield => game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                .filter(|(_, obj)| self.filter.matches(obj, &filter_ctx, game))
                .map(|(id, _)| id)
                .collect(),
            Zone::Hand => {
                // For hand, we need to filter by the chooser's hand
                let player = game
                    .player(chooser_id)
                    .ok_or(ExecutionError::PlayerNotFound(chooser_id))?;
                player
                    .hand
                    .iter()
                    .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                    .filter(|(_, obj)| self.filter.matches(obj, &filter_ctx, game))
                    .map(|(id, _)| id)
                    .collect()
            }
            Zone::Graveyard => {
                let player = game
                    .player(chooser_id)
                    .ok_or(ExecutionError::PlayerNotFound(chooser_id))?;
                if self.top_only {
                    player
                        .graveyard
                        .iter()
                        .rev()
                        .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                        .find(|(_, obj)| self.filter.matches(obj, &filter_ctx, game))
                        .map(|(id, _)| vec![id])
                        .unwrap_or_default()
                } else {
                    player
                        .graveyard
                        .iter()
                        .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                        .filter(|(_, obj)| self.filter.matches(obj, &filter_ctx, game))
                        .map(|(id, _)| id)
                        .collect()
                }
            }
            _ => {
                // For other zones, use the generic objects_in_zone method
                game.objects_in_zone(search_zone)
                    .into_iter()
                    .filter_map(|id| game.object(id).map(|obj| (id, obj)))
                    .filter(|(_, obj)| self.filter.matches(obj, &filter_ctx, game))
                    .map(|(id, _)| id)
                    .collect()
            }
        };

        if candidates.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let min = self.count.min.min(candidates.len());
        let max = self
            .count
            .max
            .unwrap_or(candidates.len())
            .min(candidates.len());

        if max == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let spec = ChooseObjectsSpec::new(
            ctx.source,
            self.description.to_string(),
            candidates.clone(),
            min,
            Some(max),
        );
        let mut chosen: Vec<ObjectId> =
            make_decision(game, ctx.decision_maker, chooser_id, Some(ctx.source), spec);

        // Clamp to max and ensure uniqueness.
        chosen.truncate(max);
        chosen.sort();
        chosen.dedup();

        if chosen.len() < min {
            for id in &candidates {
                if chosen.len() >= min {
                    break;
                }
                if !chosen.contains(id) {
                    chosen.push(*id);
                }
            }
        }

        // Create snapshots and tag them
        let snapshots: Vec<ObjectSnapshot> = chosen
            .iter()
            .filter_map(|&id| {
                game.object(id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
            })
            .collect();

        if !snapshots.is_empty() {
            ctx.tag_objects(self.tag.clone(), snapshots);
        }

        Ok(EffectOutcome::from_result(EffectResult::Objects(chosen)))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn cost_description(&self) -> Option<String> {
        use crate::color::Color;

        // Generate a human-readable description
        let count_str = match (self.count.min, self.count.max) {
            (0, Some(1)) => "up to one".to_string(),
            (0, Some(n)) => format!("up to {}", n),
            (min, Some(max)) if min == max => match min {
                1 => "a".to_string(),
                n => format!("{}", n),
            },
            (min, Some(max)) => format!("{} to {}", min, max),
            (min, None) if min == 1 => "one or more".to_string(),
            (min, None) => format!("{} or more", min),
        };

        // Describe the color requirement if present
        let color_desc = if let Some(colors) = &self.filter.colors {
            if colors.count() == 1 {
                // Find which color it is
                let color_name = Color::ALL
                    .iter()
                    .find(|&&c| colors.contains(c))
                    .map(|c| format!("{:?}", c).to_lowercase())
                    .unwrap_or_default();
                if !color_name.is_empty() {
                    format!("{} ", color_name)
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        // Determine the base type
        let type_desc = if !self.filter.card_types.is_empty() {
            self.filter
                .card_types
                .iter()
                .map(|t| format!("{:?}", t).to_lowercase())
                .collect::<Vec<_>>()
                .join(" or ")
        } else {
            "card".to_string()
        };

        // Describe the zone
        let zone_desc = match self.filter.zone.unwrap_or(self.zone) {
            Zone::Hand => "from your hand",
            Zone::Graveyard => "from your graveyard",
            Zone::Battlefield => "",
            _ => "",
        };

        Some(format!(
            "Exile {} {}{}{}",
            count_str,
            color_desc,
            type_desc,
            if zone_desc.is_empty() {
                String::new()
            } else {
                format!(" {}", zone_desc)
            }
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_choose_objects_no_candidates() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        // No creatures on battlefield
        let effect =
            ChooseObjectsEffect::new(ObjectFilter::creature(), 1, PlayerFilter::You, "selected");
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        assert!(ctx.get_tagged("selected").is_none());
    }

    #[test]
    fn test_choose_objects_single() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature1 = create_creature(&mut game, "Bear 1", alice);
        let _creature2 = create_creature(&mut game, "Bear 2", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect =
            ChooseObjectsEffect::new(ObjectFilter::creature(), 1, PlayerFilter::You, "selected");
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should have chosen one creature (SelectFirstDecisionMaker picks first)
        if let EffectResult::Objects(chosen) = result.result {
            assert_eq!(chosen.len(), 1);
            assert_eq!(chosen[0], creature1);
        } else {
            panic!("Expected Objects result");
        }

        // Should be tagged
        let tagged = ctx.get_tagged("selected");
        assert!(tagged.is_some());
        assert_eq!(tagged.unwrap().name, "Bear 1");
    }

    #[test]
    fn test_choose_objects_filtered() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create creatures for both players
        let _alice_creature = create_creature(&mut game, "Alice Bear", alice);
        let bob_creature = create_creature(&mut game, "Bob Bear", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        // Choose creature you don't control (opponent's)
        let effect = ChooseObjectsEffect::new(
            ObjectFilter::creature().opponent_controls(),
            1,
            PlayerFilter::You,
            "target",
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let EffectResult::Objects(chosen) = result.result {
            assert_eq!(chosen.len(), 1);
            assert_eq!(chosen[0], bob_creature);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_choose_objects_zero_count() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let _creature = create_creature(&mut game, "Bear", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect =
            ChooseObjectsEffect::new(ObjectFilter::creature(), 0, PlayerFilter::You, "selected");
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
    }

    #[test]
    fn test_choose_objects_clone_box() {
        let effect =
            ChooseObjectsEffect::new(ObjectFilter::creature(), 1, PlayerFilter::You, "target");
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ChooseObjectsEffect"));
    }

    #[test]
    fn test_choose_objects_with_zone() {
        let effect =
            ChooseObjectsEffect::new(ObjectFilter::creature(), 1, PlayerFilter::You, "target")
                .in_zone(Zone::Graveyard);

        assert_eq!(effect.zone, Zone::Graveyard);
    }
}

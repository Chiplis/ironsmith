//! ChooseObjects effect implementation.
//!
//! This effect allows a player to choose objects matching a filter and tag them
//! for reference by subsequent effects in the same spell/ability.

use crate::effect::{ChoiceCount, EffectOutcome, SearchSelectionMode, Value};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::filter::Comparison;
use crate::game_state::GameState;
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
/// * `zone` - Optional fallback zone to search when the filter itself is zone-less
/// * `tag` - Tag name to store chosen objects under
/// * `description` - Human-readable description for the UI
/// * `reveal` - Whether chosen cards are revealed before moving them
///
/// # Result
///
/// Returns `crate::effect::OutcomeValue::Objects(chosen_ids)` with the chosen object IDs.
/// If no valid objects exist, returns `crate::effect::OutcomeValue::Count(0)`.
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
    /// Optional runtime-resolved count used when the printed amount is defined by a Value.
    pub count_value: Option<Value>,
    /// Which player makes the choice.
    pub chooser: PlayerFilter,
    /// Fallback zone to search for objects when the filter has no explicit zone.
    pub zone: Option<Zone>,
    /// Additional zones to search for objects.
    pub additional_zones: Vec<Zone>,
    /// Tag name to store chosen objects under.
    pub tag: TagKey,
    /// Human-readable description for the decision prompt.
    pub description: &'static str,
    /// Whether this choice represents a library search.
    pub is_search: bool,
    /// Whether chosen cards should be revealed.
    pub reveal: bool,
    /// Search phrasing that determines fail-to-find behavior.
    pub search_mode: SearchSelectionMode,
    /// Restrict selection to top-most matching objects in ordered zones.
    pub top_only: bool,
    /// Replace any prior snapshots stored under `tag` instead of accumulating.
    pub replace_tagged_objects: bool,
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
            count_value: None,
            chooser,
            zone: None,
            additional_zones: Vec::new(),
            tag: tag.into(),
            description: "Choose",
            is_search: false,
            reveal: false,
            search_mode: SearchSelectionMode::Exact,
            top_only: false,
            replace_tagged_objects: false,
        }
    }

    /// Set the zone to search for objects.
    pub fn in_zone(mut self, zone: Zone) -> Self {
        self.zone = Some(zone);
        self.additional_zones.clear();
        self
    }

    /// Set the zones to search for objects.
    pub fn in_zones(mut self, zones: Vec<Zone>) -> Self {
        let mut iter = zones.into_iter();
        if let Some(first) = iter.next() {
            self.zone = Some(first);
            self.additional_zones = iter.collect();
        } else {
            self.zone = None;
            self.additional_zones.clear();
        }
        self
    }

    /// Set a custom description for the decision prompt.
    pub fn with_description(mut self, description: &'static str) -> Self {
        self.description = description;
        self
    }

    /// Use a runtime-resolved value for the choice count.
    pub fn with_count_value(mut self, count_value: Value) -> Self {
        self.count_value = Some(count_value);
        self
    }

    pub fn with_count_value_opt(mut self, count_value: Option<Value>) -> Self {
        self.count_value = count_value;
        self
    }

    /// Mark this choice as a library search (respects search restrictions).
    pub fn as_search(mut self) -> Self {
        self.is_search = true;
        self.search_mode = SearchSelectionMode::Exact;
        self
    }

    /// Mark this choice as an optional search ("up to", "any number").
    pub fn as_optional_search(mut self) -> Self {
        self.is_search = true;
        self.search_mode = SearchSelectionMode::Optional;
        self
    }

    /// Mark this choice as an "all matching" search.
    pub fn as_all_matching_search(mut self) -> Self {
        self.is_search = true;
        self.search_mode = SearchSelectionMode::AllMatching;
        self
    }

    /// Mark chosen cards as revealed.
    pub fn reveal(mut self) -> Self {
        self.reveal = true;
        self
    }

    /// Restrict selection to top-most matching objects in ordered zones.
    pub fn top_only(mut self) -> Self {
        self.top_only = true;
        self
    }

    /// Replace previously tagged objects instead of accumulating with them.
    pub fn replace_tagged_objects(mut self) -> Self {
        self.replace_tagged_objects = true;
        self
    }

    /// Number of top-most matches considered when `top_only` is set.
    pub(crate) fn top_only_selection_limit(&self, x_value: Option<u32>) -> usize {
        if !self.top_only {
            return usize::MAX;
        }
        if self.count.dynamic_x {
            return x_value
                .and_then(|x| usize::try_from(x).ok())
                .filter(|x| *x > 0)
                .unwrap_or(1);
        }
        self.count.max.unwrap_or(self.count.min).max(1)
    }

    pub(crate) fn search_zones(&self) -> Result<Vec<Zone>, ExecutionError> {
        let Some(primary_zone) = self.filter.zone.or(self.zone) else {
            return Err(ExecutionError::UnresolvableValue(
                "ChooseObjectsEffect requires an explicit search zone".to_string(),
            ));
        };

        let mut zones = vec![primary_zone];
        for zone in &self.additional_zones {
            if !zones.contains(zone) {
                zones.push(*zone);
            }
        }
        Ok(zones)
    }
}

impl EffectExecutor for ChooseObjectsEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::choose_objects_runtime::run_choose_objects(self, game, ctx)
    }

    fn cost_description(&self) -> Option<String> {
        use crate::color::Color;

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

        let color_desc = if let Some(colors) = &self.filter.colors {
            if colors.count() == 1 {
                let color_name = Color::ALL
                    .iter()
                    .find(|&&c| colors.contains(c))
                    .map(|c| c.name().to_string())
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

        let type_desc = if !self.filter.card_types.is_empty() {
            self.filter
                .card_types
                .iter()
                .map(|t| t.name().to_string())
                .collect::<Vec<_>>()
                .join(" or ")
        } else if !self.filter.subtypes.is_empty() {
            self.filter
                .subtypes
                .iter()
                .map(|s| s.to_string().to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join(" or ")
        } else {
            "card".to_string()
        };

        let zone_desc = match self.filter.zone.or(self.zone) {
            Some(Zone::Hand) => "from your hand",
            Some(Zone::Graveyard) => "from your graveyard",
            Some(Zone::Battlefield) | None => "",
            _ => "",
        };

        let mana_value_desc = match &self.filter.mana_value {
            Some(Comparison::Equal(value)) => format!(" with mana value {}", value),
            Some(Comparison::LessThan(value)) => format!(" with mana value less than {}", value),
            Some(Comparison::LessThanOrEqual(value)) => {
                format!(" with mana value {} or less", value)
            }
            Some(Comparison::GreaterThan(value)) => {
                format!(" with mana value greater than {}", value)
            }
            Some(Comparison::GreaterThanOrEqual(value)) => {
                format!(" with mana value {} or greater", value)
            }
            Some(Comparison::NotEqual(value)) => {
                format!(" with mana value not equal to {}", value)
            }
            Some(Comparison::OneOf(values)) => {
                let joined = values
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(" with mana value {}", joined)
            }
            Some(Comparison::EqualExpr(_))
            | Some(Comparison::NotEqualExpr(_))
            | Some(Comparison::LessThanExpr(_))
            | Some(Comparison::LessThanOrEqualExpr(_))
            | Some(Comparison::GreaterThanExpr(_))
            | Some(Comparison::GreaterThanOrEqualExpr(_)) => {
                " with a constrained mana value".to_string()
            }
            None => String::new(),
        };

        Some(format!(
            "Exile {} {}{}{}{}",
            count_str,
            color_desc,
            type_desc,
            mana_value_desc,
            if zone_desc.is_empty() {
                String::new()
            } else {
                format!(" {}", zone_desc)
            }
        ))
    }
}

impl CostExecutableEffect for ChooseObjectsEffect {
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
        let Some(search_zone) = self.filter.zone.or(self.zone) else {
            return Err(CostValidationError::Other(
                "ChooseObjectsEffect requires an explicit search zone".to_string(),
            ));
        };
        let top_only_limit = self.top_only_selection_limit(None);

        let candidate_count = match search_zone {
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
                            .filter(|obj| {
                                if self.filter.other && obj.id == source {
                                    return false;
                                }
                                self.filter.matches(obj, &filter_ctx, game)
                            })
                            .take(top_only_limit)
                            .count()
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
            Zone::Library => {
                let owner_ids: Vec<_> = if let Some(owner_filter) = &self.filter.owner {
                    game.players
                        .iter()
                        .map(|player| player.id)
                        .filter(|player_id| owner_filter.matches_player(*player_id, &filter_ctx))
                        .collect()
                } else {
                    vec![chooser_id]
                };
                if self.top_only {
                    let mut total = 0usize;
                    'owners: for owner_id in owner_ids {
                        let Some(player) = game.player(owner_id) else {
                            continue;
                        };
                        for obj in player
                            .library
                            .iter()
                            .rev()
                            .filter_map(|&id| game.object(id))
                        {
                            if self.filter.other && obj.id == source {
                                continue;
                            }
                            if self.filter.matches(obj, &filter_ctx, game) {
                                total += 1;
                                if total >= top_only_limit {
                                    break 'owners;
                                }
                            }
                        }
                    }
                    total
                } else {
                    owner_ids
                        .into_iter()
                        .filter_map(|owner_id| game.player(owner_id))
                        .flat_map(|player| player.library.iter())
                        .filter_map(|&id| game.object(id))
                        .filter(|obj| {
                            if self.filter.other && obj.id == source {
                                return false;
                            }
                            self.filter.matches(obj, &filter_ctx, game)
                        })
                        .count()
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

        if candidate_count < self.count.min {
            return Err(CostValidationError::Other(format!(
                "Not enough objects to choose ({} needed, {} available)",
                self.count.min, candidate_count
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
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
        if let crate::effect::OutcomeValue::Objects(chosen) = result.value {
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

        if let crate::effect::OutcomeValue::Objects(chosen) = result.value {
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
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

        assert_eq!(effect.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn test_choose_objects_requires_explicit_search_zone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect =
            ChooseObjectsEffect::new(ObjectFilter::default(), 1, PlayerFilter::You, "selected");
        let err = effect
            .execute(&mut game, &mut ctx)
            .expect_err("zone-less choose effect should fail explicitly");

        assert!(matches!(
            err,
            ExecutionError::UnresolvableValue(message)
                if message.contains("explicit search zone")
        ));
    }
}

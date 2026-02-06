//! Object selection decision specifications.
//!
//! These specs are for decisions where the player selects one or more objects
//! (permanents, cards, etc.) from a list of candidates.

use crate::decision::FallbackStrategy;
use crate::decisions::context::{
    DecisionContext, ProliferateContext, SelectObjectsContext, SelectableObject,
};
use crate::decisions::spec::{DecisionPrimitive, DecisionSpec};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

// ============================================================================
// SacrificeSpec - Choose permanent(s) to sacrifice
// ============================================================================

/// Specification for choosing a permanent to sacrifice.
#[derive(Debug, Clone)]
pub struct SacrificeSpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// Description of what kind of permanent to sacrifice (e.g., "a creature").
    pub description: String,
    /// The candidates (objects that can be sacrificed).
    /// This is stored here so specs can be fully self-contained.
    pub candidates: Vec<ObjectId>,
}

impl SacrificeSpec {
    /// Create a new SacrificeSpec.
    pub fn new(
        source: ObjectId,
        description: impl Into<String>,
        candidates: Vec<ObjectId>,
    ) -> Self {
        Self {
            source,
            description: description.into(),
            candidates,
        }
    }
}

impl DecisionSpec for SacrificeSpec {
    type Response = ObjectId;

    fn description(&self) -> String {
        format!("Choose {} to sacrifice", self.description)
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectObjects {
            min: 1,
            max: Some(1),
        }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> ObjectId {
        // For mandatory sacrifice, default to first candidate
        self.candidates
            .first()
            .copied()
            .expect("SacrificeSpec requires at least one candidate")
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let candidates: Vec<SelectableObject> = self
            .candidates
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                SelectableObject::new(id, name)
            })
            .collect();

        DecisionContext::SelectObjects(SelectObjectsContext::new(
            player,
            Some(self.source),
            format!("Choose {} to sacrifice", self.description),
            candidates,
            1,
            Some(1),
        ))
    }
}

// ============================================================================
// ChooseObjectsSpec - Choose one or more objects from candidates
// ============================================================================

/// Specification for choosing one or more objects from a list of candidates.
#[derive(Debug, Clone)]
pub struct ChooseObjectsSpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// Description shown to the player.
    pub description: String,
    /// Objects that can be chosen.
    pub candidates: Vec<ObjectId>,
    /// Minimum number of objects to choose.
    pub min: usize,
    /// Maximum number of objects to choose (None = unlimited).
    pub max: Option<usize>,
}

impl ChooseObjectsSpec {
    /// Create a new ChooseObjectsSpec.
    pub fn new(
        source: ObjectId,
        description: impl Into<String>,
        candidates: Vec<ObjectId>,
        min: usize,
        max: Option<usize>,
    ) -> Self {
        Self {
            source,
            description: description.into(),
            candidates,
            min,
            max,
        }
    }
}

impl DecisionSpec for ChooseObjectsSpec {
    type Response = Vec<ObjectId>;

    fn description(&self) -> String {
        self.description.clone()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectObjects {
            min: self.min,
            max: self.max,
        }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> Vec<ObjectId> {
        if self.min == 0 {
            return Vec::new();
        }
        self.candidates.iter().copied().take(self.min).collect()
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let candidates: Vec<SelectableObject> = self
            .candidates
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                SelectableObject::new(id, name)
            })
            .collect();

        DecisionContext::SelectObjects(SelectObjectsContext::new(
            player,
            Some(self.source),
            self.description.clone(),
            candidates,
            self.min,
            self.max,
        ))
    }
}

// ============================================================================
// DiscardSpec - Choose card(s) to discard
// ============================================================================

/// Specification for choosing a card to discard from hand.
#[derive(Debug, Clone)]
pub struct DiscardSpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// Description of what to discard.
    pub description: String,
    /// Cards that can be discarded.
    pub candidates: Vec<ObjectId>,
    /// Whether this is optional (can choose to not discard).
    pub optional: bool,
}

impl DiscardSpec {
    /// Create a mandatory discard spec.
    pub fn new(
        source: ObjectId,
        description: impl Into<String>,
        candidates: Vec<ObjectId>,
    ) -> Self {
        Self {
            source,
            description: description.into(),
            candidates,
            optional: false,
        }
    }

    /// Create an optional discard spec.
    pub fn optional(
        source: ObjectId,
        description: impl Into<String>,
        candidates: Vec<ObjectId>,
    ) -> Self {
        Self {
            source,
            description: description.into(),
            candidates,
            optional: true,
        }
    }
}

impl DecisionSpec for DiscardSpec {
    type Response = Option<ObjectId>;

    fn description(&self) -> String {
        if self.optional {
            format!("You may {}", self.description)
        } else {
            self.description.clone()
        }
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectObjects {
            min: if self.optional { 0 } else { 1 },
            max: Some(1),
        }
    }

    fn default_response(&self, strategy: FallbackStrategy) -> Option<ObjectId> {
        if self.optional {
            match strategy {
                FallbackStrategy::Accept => self.candidates.first().copied(),
                _ => None,
            }
        } else {
            self.candidates.first().copied()
        }
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let candidates: Vec<SelectableObject> = self
            .candidates
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                SelectableObject::new(id, name)
            })
            .collect();

        let description = if self.optional {
            format!("You may {}", self.description)
        } else {
            self.description.clone()
        };

        DecisionContext::SelectObjects(SelectObjectsContext::new(
            player,
            Some(self.source),
            description,
            candidates,
            if self.optional { 0 } else { 1 },
            Some(1),
        ))
    }
}

// ============================================================================
// ExileSpec - Choose card(s) to exile from hand
// ============================================================================

/// Specification for choosing a card to exile from hand.
/// Used for costs like Force of Will's alternative cost.
#[derive(Debug, Clone)]
pub struct ExileSpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// Description of what to exile.
    pub description: String,
    /// Cards that can be exiled.
    pub candidates: Vec<ObjectId>,
}

impl ExileSpec {
    /// Create a new ExileSpec.
    pub fn new(
        source: ObjectId,
        description: impl Into<String>,
        candidates: Vec<ObjectId>,
    ) -> Self {
        Self {
            source,
            description: description.into(),
            candidates,
        }
    }
}

impl DecisionSpec for ExileSpec {
    type Response = ObjectId;

    fn description(&self) -> String {
        self.description.clone()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectObjects {
            min: 1,
            max: Some(1),
        }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> ObjectId {
        self.candidates
            .first()
            .copied()
            .expect("ExileSpec requires at least one candidate")
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let candidates: Vec<SelectableObject> = self
            .candidates
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                SelectableObject::new(id, name)
            })
            .collect();

        DecisionContext::SelectObjects(SelectObjectsContext::new(
            player,
            Some(self.source),
            self.description.clone(),
            candidates,
            1,
            Some(1),
        ))
    }
}

// ============================================================================
// SearchSpec - Choose card(s) from library search
// ============================================================================

/// Specification for choosing a card from a library search.
#[derive(Debug, Clone)]
pub struct SearchSpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// Cards matching the search criteria.
    pub matching_cards: Vec<ObjectId>,
    /// Whether the search is revealed to all players.
    pub reveal: bool,
    /// Whether the player can "fail to find" (choose nothing).
    pub may_fail_to_find: bool,
}

impl SearchSpec {
    /// Create a new SearchSpec.
    pub fn new(source: ObjectId, matching_cards: Vec<ObjectId>, reveal: bool) -> Self {
        Self {
            source,
            matching_cards,
            reveal,
            may_fail_to_find: true, // Most searches can fail to find
        }
    }

    /// Create a mandatory search (must find a card if possible).
    pub fn mandatory(source: ObjectId, matching_cards: Vec<ObjectId>, reveal: bool) -> Self {
        Self {
            source,
            matching_cards,
            reveal,
            may_fail_to_find: false,
        }
    }
}

impl DecisionSpec for SearchSpec {
    type Response = Option<ObjectId>;

    fn description(&self) -> String {
        if self.reveal {
            "Search library (revealed)".to_string()
        } else {
            "Search library".to_string()
        }
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectObjects {
            min: if self.may_fail_to_find { 0 } else { 1 },
            max: Some(1),
        }
    }

    fn default_response(&self, strategy: FallbackStrategy) -> Option<ObjectId> {
        match strategy {
            FallbackStrategy::Decline if self.may_fail_to_find => None,
            _ => self.matching_cards.first().copied(),
        }
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let candidates: Vec<SelectableObject> = self
            .matching_cards
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                SelectableObject::new(id, name)
            })
            .collect();

        let description = if self.reveal {
            "Search library (revealed)".to_string()
        } else {
            "Search library".to_string()
        };

        DecisionContext::SelectObjects(SelectObjectsContext::new(
            player,
            Some(self.source),
            description,
            candidates,
            if self.may_fail_to_find { 0 } else { 1 },
            Some(1),
        ))
    }
}

// ============================================================================
// DiscardToHandSizeSpec - Discard down to maximum hand size
// ============================================================================

/// Specification for discarding cards to reach maximum hand size.
#[derive(Debug, Clone)]
pub struct DiscardToHandSizeSpec {
    /// Number of cards that must be discarded.
    pub count: usize,
    /// Cards in hand that can be discarded.
    pub hand: Vec<ObjectId>,
}

impl DiscardToHandSizeSpec {
    /// Create a new DiscardToHandSizeSpec.
    pub fn new(count: usize, hand: Vec<ObjectId>) -> Self {
        Self { count, hand }
    }
}

impl DecisionSpec for DiscardToHandSizeSpec {
    type Response = Vec<ObjectId>;

    fn description(&self) -> String {
        format!("Discard {} card(s) to reach maximum hand size", self.count)
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectObjects {
            min: self.count,
            max: Some(self.count),
        }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> Vec<ObjectId> {
        // Default: discard the last N cards in hand
        self.hand.iter().rev().take(self.count).copied().collect()
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let candidates: Vec<SelectableObject> = self
            .hand
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                SelectableObject::new(id, name)
            })
            .collect();

        DecisionContext::SelectObjects(SelectObjectsContext::new(
            player,
            None,
            format!("Discard {} card(s) to reach maximum hand size", self.count),
            candidates,
            self.count,
            Some(self.count),
        ))
    }
}

// ============================================================================
// LegendRuleSpec - Choose which legend to keep
// ============================================================================

/// Specification for legend rule - choose which legendary permanent to keep.
#[derive(Debug, Clone)]
pub struct LegendRuleSpec {
    /// Name of the legendary permanent.
    pub name: String,
    /// The legendary permanents with this name.
    pub legends: Vec<ObjectId>,
}

impl LegendRuleSpec {
    /// Create a new LegendRuleSpec.
    pub fn new(name: impl Into<String>, legends: Vec<ObjectId>) -> Self {
        Self {
            name: name.into(),
            legends,
        }
    }
}

impl DecisionSpec for LegendRuleSpec {
    type Response = ObjectId;

    fn description(&self) -> String {
        format!("Choose which {} to keep (legend rule)", self.name)
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectObjects {
            min: 1,
            max: Some(1),
        }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> ObjectId {
        // Default: keep the first one (usually the one that was already on the battlefield)
        self.legends
            .first()
            .copied()
            .expect("LegendRuleSpec requires at least two legends")
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let candidates: Vec<SelectableObject> = self
            .legends
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| self.name.clone());
                SelectableObject::new(id, name)
            })
            .collect();

        DecisionContext::SelectObjects(SelectObjectsContext::new(
            player,
            None,
            format!("Choose which {} to keep (legend rule)", self.name),
            candidates,
            1,
            Some(1),
        ))
    }
}

// ============================================================================
// ProliferateSpec - Choose permanents and players to proliferate
// ============================================================================

/// Specification for proliferate - choose any number of permanents and players with counters.
#[derive(Debug, Clone)]
pub struct ProliferateSpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// Permanents with counters that can be proliferated.
    pub eligible_permanents: Vec<ObjectId>,
    /// Players with counters that can be proliferated.
    pub eligible_players: Vec<PlayerId>,
}

impl ProliferateSpec {
    /// Create a new ProliferateSpec.
    pub fn new(
        source: ObjectId,
        eligible_permanents: Vec<ObjectId>,
        eligible_players: Vec<PlayerId>,
    ) -> Self {
        Self {
            source,
            eligible_permanents,
            eligible_players,
        }
    }
}

/// Response for proliferate: selected permanents and players.
#[derive(Debug, Clone, Default)]
pub struct ProliferateResponse {
    pub permanents: Vec<ObjectId>,
    pub players: Vec<PlayerId>,
}

impl DecisionSpec for ProliferateSpec {
    type Response = ProliferateResponse;

    fn description(&self) -> String {
        "Choose permanents and players to proliferate".to_string()
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectMixed
    }

    fn default_response(&self, strategy: FallbackStrategy) -> ProliferateResponse {
        match strategy {
            FallbackStrategy::Maximum => ProliferateResponse {
                permanents: self.eligible_permanents.clone(),
                players: self.eligible_players.clone(),
            },
            _ => ProliferateResponse::default(),
        }
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let eligible_permanents: Vec<(ObjectId, String)> = self
            .eligible_permanents
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                (id, name)
            })
            .collect();

        let eligible_players: Vec<(PlayerId, String)> = self
            .eligible_players
            .iter()
            .map(|&id| {
                let name = game
                    .player(id)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                (id, name)
            })
            .collect();

        DecisionContext::Proliferate(ProliferateContext::new(
            player,
            Some(self.source),
            eligible_permanents,
            eligible_players,
        ))
    }
}

// ============================================================================
// MayChooseCardSpec - Optional card selection from hand
// ============================================================================

/// Specification for optionally choosing a card from hand.
/// Used for effects like Chrome Mox's imprint.
#[derive(Debug, Clone)]
pub struct MayChooseCardSpec {
    /// The source of the effect.
    pub source: ObjectId,
    /// Description of what kind of card can be chosen.
    pub description: String,
    /// Cards that can be chosen.
    pub candidates: Vec<ObjectId>,
}

impl MayChooseCardSpec {
    /// Create a new MayChooseCardSpec.
    pub fn new(
        source: ObjectId,
        description: impl Into<String>,
        candidates: Vec<ObjectId>,
    ) -> Self {
        Self {
            source,
            description: description.into(),
            candidates,
        }
    }
}

impl DecisionSpec for MayChooseCardSpec {
    type Response = Option<ObjectId>;

    fn description(&self) -> String {
        format!("You may {}", self.description)
    }

    fn primitive(&self) -> DecisionPrimitive {
        DecisionPrimitive::SelectObjects {
            min: 0,
            max: Some(1),
        }
    }

    fn default_response(&self, strategy: FallbackStrategy) -> Option<ObjectId> {
        match strategy {
            FallbackStrategy::Accept => self.candidates.first().copied(),
            _ => None,
        }
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        let candidates: Vec<SelectableObject> = self
            .candidates
            .iter()
            .map(|&id| {
                let name = game
                    .object(id)
                    .map(|o| o.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                SelectableObject::new(id, name)
            })
            .collect();

        DecisionContext::SelectObjects(SelectObjectsContext::new(
            player,
            Some(self.source),
            format!("You may {}", self.description),
            candidates,
            0,
            Some(1),
        ))
    }
}

// ============================================================================
// TargetsSpec - Choose targets for a spell or ability
// ============================================================================

/// A targeting requirement.
#[derive(Debug, Clone)]
pub struct TargetRequirement {
    /// Description of what's being targeted.
    pub description: String,
    /// Legal targets that match this specification.
    pub legal_targets: Vec<crate::game_state::Target>,
    /// Minimum number of targets to choose.
    pub min_targets: usize,
    /// Maximum number of targets to choose (None = unlimited).
    pub max_targets: Option<usize>,
}

impl TargetRequirement {
    /// Create a requirement for exactly one target.
    pub fn single(
        description: impl Into<String>,
        legal_targets: Vec<crate::game_state::Target>,
    ) -> Self {
        Self {
            description: description.into(),
            legal_targets,
            min_targets: 1,
            max_targets: Some(1),
        }
    }

    /// Create a requirement for any number of targets.
    pub fn any_number(
        description: impl Into<String>,
        legal_targets: Vec<crate::game_state::Target>,
    ) -> Self {
        Self {
            description: description.into(),
            legal_targets,
            min_targets: 0,
            max_targets: None,
        }
    }
}

/// Specification for choosing targets for a spell or ability.
#[derive(Debug, Clone)]
pub struct TargetsSpec {
    /// The source spell or ability.
    pub source: ObjectId,
    /// Description of what is being targeted.
    pub context: String,
    /// The targeting requirements.
    pub requirements: Vec<TargetRequirement>,
}

impl TargetsSpec {
    /// Create a new TargetsSpec.
    pub fn new(
        source: ObjectId,
        context: impl Into<String>,
        requirements: Vec<TargetRequirement>,
    ) -> Self {
        Self {
            source,
            context: context.into(),
            requirements,
        }
    }
}

impl DecisionSpec for TargetsSpec {
    type Response = Vec<crate::game_state::Target>;

    fn description(&self) -> String {
        format!("Choose targets for {}", self.context)
    }

    fn primitive(&self) -> DecisionPrimitive {
        // Calculate total min/max from all requirements
        let min: usize = self.requirements.iter().map(|r| r.min_targets).sum();
        let max: Option<usize> = {
            let maxes: Vec<_> = self
                .requirements
                .iter()
                .filter_map(|r| r.max_targets)
                .collect();
            if maxes.len() == self.requirements.len() {
                Some(maxes.iter().sum())
            } else {
                None // At least one requirement has unlimited targets
            }
        };

        DecisionPrimitive::SelectObjects { min, max }
    }

    fn default_response(&self, _strategy: FallbackStrategy) -> Vec<crate::game_state::Target> {
        // Default: pick first legal target for each requirement
        self.requirements
            .iter()
            .filter_map(|r| r.legal_targets.first().cloned())
            .collect()
    }

    fn build_context(
        &self,
        player: PlayerId,
        _source: Option<ObjectId>,
        game: &GameState,
    ) -> DecisionContext {
        // Flatten all legal targets from all requirements into a single list
        // Note: This simplification works for most cases, but complex multi-requirement
        // targeting might need a more sophisticated approach
        let mut candidates: Vec<SelectableObject> = Vec::new();

        for req in &self.requirements {
            for target in &req.legal_targets {
                let (id, name) = match target {
                    crate::game_state::Target::Object(id) => {
                        let name = game
                            .object(*id)
                            .map(|o| o.name.clone())
                            .unwrap_or_else(|| "Unknown".to_string());
                        (*id, name)
                    }
                    crate::game_state::Target::Player(pid) => {
                        let name = game
                            .player(*pid)
                            .map(|p| p.name.clone())
                            .unwrap_or_else(|| "Unknown player".to_string());
                        // Use a synthetic ObjectId for players in the selection context
                        // This is a workaround since SelectObjectsContext uses ObjectId
                        (ObjectId::from_raw(pid.index() as u64 + 0x8000_0000), name)
                    }
                };
                // Avoid duplicates
                if !candidates.iter().any(|c| c.id == id) {
                    candidates.push(SelectableObject::new(id, name));
                }
            }
        }

        let min: usize = self.requirements.iter().map(|r| r.min_targets).sum();
        let max: Option<usize> = {
            let maxes: Vec<_> = self
                .requirements
                .iter()
                .filter_map(|r| r.max_targets)
                .collect();
            if maxes.len() == self.requirements.len() {
                Some(maxes.iter().sum())
            } else {
                None
            }
        };

        DecisionContext::SelectObjects(SelectObjectsContext::new(
            player,
            Some(self.source),
            format!("Choose targets for {}", self.context),
            candidates,
            min,
            max,
        ))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sacrifice_spec() {
        let source = ObjectId::from_raw(1);
        let candidates = vec![ObjectId::from_raw(2), ObjectId::from_raw(3)];
        let spec = SacrificeSpec::new(source, "a creature", candidates.clone());

        assert!(spec.description().contains("sacrifice"));
        assert!(matches!(
            spec.primitive(),
            DecisionPrimitive::SelectObjects {
                min: 1,
                max: Some(1)
            }
        ));
        assert_eq!(
            spec.default_response(FallbackStrategy::FirstOption),
            candidates[0]
        );
    }

    #[test]
    fn test_discard_spec_optional() {
        let source = ObjectId::from_raw(1);
        let candidates = vec![ObjectId::from_raw(2)];
        let spec = DiscardSpec::optional(source, "discard a card", candidates);

        assert!(spec.description().contains("may"));
        assert!(matches!(
            spec.primitive(),
            DecisionPrimitive::SelectObjects {
                min: 0,
                max: Some(1)
            }
        ));
        assert!(spec.default_response(FallbackStrategy::Decline).is_none());
    }

    #[test]
    fn test_search_spec() {
        let source = ObjectId::from_raw(1);
        let cards = vec![ObjectId::from_raw(2)];
        let spec = SearchSpec::new(source, cards.clone(), true);

        assert!(spec.description().contains("revealed"));
        assert!(spec.default_response(FallbackStrategy::Decline).is_none());
        assert_eq!(
            spec.default_response(FallbackStrategy::FirstOption),
            Some(cards[0])
        );
    }

    #[test]
    fn test_discard_to_hand_size_spec() {
        let hand = vec![
            ObjectId::from_raw(1),
            ObjectId::from_raw(2),
            ObjectId::from_raw(3),
        ];
        let spec = DiscardToHandSizeSpec::new(2, hand);

        let response = spec.default_response(FallbackStrategy::FirstOption);
        assert_eq!(response.len(), 2);
    }

    #[test]
    fn test_proliferate_spec() {
        let source = ObjectId::from_raw(1);
        let perms = vec![ObjectId::from_raw(2)];
        let players = vec![PlayerId::from_index(0)];
        let spec = ProliferateSpec::new(source, perms.clone(), players.clone());

        let max_response = spec.default_response(FallbackStrategy::Maximum);
        assert_eq!(max_response.permanents, perms);
        assert_eq!(max_response.players, players);
    }
}

use winnow::Parser;
use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;

use crate::cards::builders::{DamageBySpec, PlayerAst};
use crate::color::ColorSet;
use crate::effect::{Comparison, Value, ValueComparisonOperator};
use crate::static_abilities::AnthemCountExpression;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

use super::super::lexer::{LexStream, LexedClause, OwnedLexToken};
use super::super::util::{
    comparison_to_at_least_threshold, comparison_to_strict_at_least_threshold,
    comparison_to_strict_at_most_threshold, comparison_to_value_comparison_operator,
    parse_card_type, parse_color, parse_greater_than_or_equal_quantity_prefix,
    parse_quantity_comparison_prefix, parse_quantity_comparison_prefix_words,
    parse_subtype_flexible, trim_edge_punctuation_tokens,
};
use super::filters::parse_object_filter_with_grammar_entrypoint;
use super::leaf::{
    LeafPlayerReference, LeafPlayerReferenceMode, parse_leaf_player_reference_tokens,
    parse_leaf_player_reference_words,
};
use super::primitives;
use crate::object_filters::parse_object_filter_words;

#[path = "conditions/counter_shapes.rs"]
mod counter_shapes;
#[path = "conditions/event_shapes.rs"]
mod event_shapes;
#[path = "conditions/life_tie_shapes.rs"]
mod life_tie_shapes;
#[path = "conditions/relation_shapes.rs"]
mod relation_shapes;
#[path = "conditions/status_shapes.rs"]
mod status_shapes;
#[path = "conditions/zone_change_shapes.rs"]
mod zone_change_shapes;

pub use counter_shapes::parse_player_counter_condition;

#[derive(Debug, Clone, PartialEq)]
enum LifeRelationShape {
    MoreThanYou,
    MoreThanEachOtherPlayer,
    MoreThanEachOpponent,
    MoreThanPlayer(PlayerFilter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CardsInHandRelationShape {
    MoreThanYou,
    MoreThanEachOtherPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlConditionFilterSuffix {
    DifferentPowers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlConditionOptions {
    pub allow_that_player: bool,
    pub allow_opponent_players: bool,
    pub allow_defending_player: bool,
    pub bind_filter_controller_to_subject: bool,
    pub allow_different_powers_tail: bool,
    pub default_filter_zone: Option<Zone>,
}

impl Default for ControlConditionOptions {
    fn default() -> Self {
        Self {
            allow_that_player: true,
            allow_opponent_players: false,
            allow_defending_player: false,
            bind_filter_controller_to_subject: false,
            allow_different_powers_tail: false,
            default_filter_zone: Some(Zone::Battlefield),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlConditionAst {
    pub player: PlayerAst,
    pub player_filter: Option<PlayerFilter>,
    pub comparison: Comparison,
    pub quantity_token_count: usize,
    pub quantity_words: Vec<String>,
    pub object_words: Vec<String>,
    pub filter: ObjectFilter,
    pub requires_different_powers: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OwnershipConditionOptions {
    pub allow_opponent_players: bool,
    pub bind_filter_owner_to_subject: bool,
    pub default_filter_zone: Option<Zone>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OwnershipConditionAst {
    pub player: PlayerAst,
    pub player_filter: Option<PlayerFilter>,
    pub comparison: Comparison,
    pub quantity_token_count: usize,
    pub quantity_words: Vec<String>,
    pub object_words: Vec<String>,
    pub filter: ObjectFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusConditionSubjectAst {
    Source,
    EquippedCreature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusConditionStateAst {
    Equipped,
    Enchanted,
    Tapped,
    Untapped,
    Attacking,
    AttackingAlone,
    Monstrous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubjectStatusConditionAst {
    pub subject: StatusConditionSubjectAst,
    pub state: StatusConditionStateAst,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectDescriptorAst {
    Color(ColorSet),
    CardType(CardType),
    Subtype(Subtype),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubjectDescriptorConditionSubjectAst {
    EnchantedPermanent,
    AttachedObject,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubjectDescriptorConditionAst {
    pub subject: SubjectDescriptorConditionSubjectAst,
    pub filter: ObjectFilter,
    pub descriptor: ObjectDescriptorAst,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectAttachedToObjectConditionAst {
    pub attachment_filter: ObjectFilter,
    pub attached_to_filter: ObjectFilter,
    pub comparison: Comparison,
    pub display: String,
}

/// A characteristic test against the public cards a player removed from a
/// draft with a named card group.
#[derive(Debug, Clone, PartialEq)]
pub struct RemovedFromDraftConditionAst {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub with_cards_named: String,
}

/// Parse `you removed <card filter> from the draft with cards named <name>`.
///
/// Keeping the removed card as an ordinary object filter lets conditional
/// grants ask for any printed characteristic or ability without introducing a
/// mechanic-specific flag for each one.
pub fn parse_removed_from_draft_condition(
    tokens: &[OwnedLexToken],
) -> Option<RemovedFromDraftConditionAst> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    let (_, rest) = primitives::parse_prefix(tokens, primitives::phrase(&["you", "removed"]))?;
    let separator =
        primitives::find_phrase_start(rest, &["from", "the", "draft", "with", "cards", "named"])?;
    let filter_tokens = trim_edge_punctuation_tokens(rest.get(..separator)?);
    let (_, name_tokens) = primitives::parse_prefix(
        rest.get(separator..)?,
        primitives::phrase(&["from", "the", "draft", "with", "cards", "named"]),
    )?;
    let name_tokens = trim_edge_punctuation_tokens(name_tokens);
    if filter_tokens.is_empty() || name_tokens.is_empty() {
        return None;
    }
    let filter_words = crate::lexer::token_word_refs(filter_tokens);
    let has_authored_zone = filter_words
        .iter()
        .any(|word| crate::util::parse_zone_word(word).is_some());
    let mut filter = parse_object_filter_with_grammar_entrypoint(filter_tokens, false)
        .or_else(|_| parse_object_filter_words(&filter_words, false))
        .ok()?;
    if has_authored_zone {
        return None;
    }
    // Draft provenance is carried by the condition itself, not by an object
    // zone. The permissive word parser may default an otherwise zone-free
    // card descriptor to the battlefield; remove only that unauthored default.
    filter.zone = None;
    let mut with_cards_named = crate::lexer::token_word_refs(name_tokens).join(" ");
    if let Some(source_name) = crate::util::current_source_reference_name()
        && source_name.eq_ignore_ascii_case(&with_cards_named)
    {
        with_cards_named = source_name;
    }
    if with_cards_named.is_empty() {
        return None;
    }
    Some(RemovedFromDraftConditionAst {
        player: PlayerFilter::You,
        filter,
        with_cards_named,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStatusAst {
    Monarch,
    Initiative,
    MaxSpeed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerStatusConditionAst {
    pub player: PlayerFilter,
    pub status: PlayerStatusAst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerAchievementAst {
    CitysBlessing,
    CompletedDungeon { dungeon_name: Option<String> },
    FullParty,
    VisitedAttractionThisTurn,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerAchievementConditionAst {
    pub player: PlayerFilter,
    pub achievement: PlayerAchievementAst,
    pub negated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCardsInHandConditionAst {
    pub player: PlayerFilter,
    pub comparison: Comparison,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerLifeTotalConditionAst {
    pub player: PlayerFilter,
    pub comparison: Comparison,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerLifeTieConditionAst {
    pub minimum_players: u32,
    pub tied_players: PlayerFilter,
}

#[derive(Debug, Clone)]
pub struct PlayerLifeTieChoiceConditionAst<'a> {
    pub minimum_players: u32,
    pub tied_players: PlayerFilter,
    pub consequence_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerHasQuantityObjectConditionAst {
    pub player: PlayerFilter,
    pub comparison: Comparison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerLifeRelationAst {
    HasMoreLifeThanYou,
    HasLessLifeThanYou,
    HasNoOpponentWithMoreLifeThan,
    HasMoreLifeThanEachOtherPlayer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerLifeRelationConditionAst {
    pub player: PlayerFilter,
    pub relation: PlayerLifeRelationAst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCardsInHandRelationAst {
    HasMoreCardsInHandThanYou,
    HasMoreCardsInHandThanEachOtherPlayer,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCardsInHandRelationConditionAst {
    pub player: PlayerFilter,
    pub relation: PlayerCardsInHandRelationAst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetSetPredicateAst {
    DifferentColorSets,
}

/// Parse a predicate that relates the object targets selected by the enclosing
/// spell or ability. These predicates intentionally carry no card identity or
/// effect-specific behavior; they lower to reusable resolution conditions.
pub fn parse_target_set_predicate(tokens: &[OwnedLexToken]) -> Option<TargetSetPredicateAst> {
    relation_shapes::parse_target_set_predicate(tokens)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerTurnEventAst {
    CardsDrawn,
    LandsEnteredBattlefieldUnderControl,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerTurnEventConditionAst {
    pub player: PlayerFilter,
    pub event: PlayerTurnEventAst,
    pub comparison: Comparison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellContextReferenceAst {
    TargetSpell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellContextConditionAst {
    ControllerIsPoisoned { spell: SpellContextReferenceAst },
    NoManaSpentToCast { spell: SpellContextReferenceAst },
    YouControlMoreCreaturesThanController { spell: SpellContextReferenceAst },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerSpellCastThisTurnConditionAst {
    MatchingFilters {
        player: PlayerFilter,
        filters: Vec<ObjectFilter>,
        negated: bool,
    },
    CountAtLeast {
        player: PlayerFilter,
        count: u32,
    },
    MatchingFilterCountAtLeast {
        player: PlayerFilter,
        filter: ObjectFilter,
        count: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerLifeChangeDirectionAst {
    Gained,
    Lost,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerLifeChangeThisTurnConditionAst {
    pub player: PlayerFilter,
    pub direction: PlayerLifeChangeDirectionAst,
    pub comparison: Comparison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerWouldActionAst {
    DrawCard,
    Proliferate,
    BeginExtraTurn,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerWouldActionConditionAst {
    pub player: PlayerFilter,
    pub action: PlayerWouldActionAst,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BattlefieldChangeThisTurnConditionAst {
    PermanentLeftBattlefield {
        negated: bool,
    },
    NonlandPermanentLeftBattlefieldOrSpellWarped,
    PermanentLeftBattlefieldUnderYourControl {
        surface: crate::PermanentLeftBattlefieldControlSurface,
    },
    ObjectPutIntoGraveyardFromBattlefield {
        filter: ObjectFilter,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectDeathThisTurnEventAst {
    Died,
    PutIntoYourGraveyardFromAnywhere,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectDeathThisTurnConditionAst {
    pub event: ObjectDeathThisTurnEventAst,
    pub filter: ObjectFilter,
    pub comparison: Comparison,
    pub under_controller: Option<PlayerFilter>,
    pub damaged_by: Option<DamageBySpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattlefieldEntryTurnWindowAst {
    ThisTurn,
    LastTurn,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BattlefieldEntryConditionAst {
    ObjectEntered {
        filter: ObjectFilter,
        window: BattlefieldEntryTurnWindowAst,
        min_count: Option<u32>,
    },
    LandEnteredUnderYourControlThisTurn {
        player: PlayerAst,
    },
}

impl ControlConditionAst {
    pub fn has_explicit_quantity(&self) -> bool {
        self.quantity_token_count > 0
    }

    pub fn exact_count(&self) -> Option<u32> {
        match self.comparison {
            Comparison::Equal(count) if count >= 0 => Some(count as u32),
            _ => None,
        }
    }

    pub fn at_least_count(&self) -> Option<u32> {
        comparison_to_at_least_threshold(&self.comparison)
    }

    pub fn strict_at_least_count(&self) -> Option<u32> {
        comparison_to_strict_at_least_threshold(&self.comparison)
    }

    pub fn quantity_text(&self) -> String {
        self.quantity_words.join(" ")
    }

    pub fn object_text(&self) -> String {
        self.object_words.join(" ")
    }
}

impl SubjectStatusConditionAst {
    pub fn condition_expr(self) -> Option<crate::ConditionExpr> {
        match (self.subject, self.state) {
            (StatusConditionSubjectAst::Source, StatusConditionStateAst::Equipped) => {
                Some(crate::ConditionExpr::SourceIsEquipped)
            }
            (StatusConditionSubjectAst::Source, StatusConditionStateAst::Enchanted) => {
                Some(crate::ConditionExpr::SourceIsEnchanted)
            }
            (StatusConditionSubjectAst::Source, StatusConditionStateAst::Tapped) => {
                Some(crate::ConditionExpr::SourceIsTapped)
            }
            (StatusConditionSubjectAst::Source, StatusConditionStateAst::Untapped) => {
                Some(crate::ConditionExpr::SourceIsUntapped)
            }
            (StatusConditionSubjectAst::Source, StatusConditionStateAst::Attacking) => {
                Some(crate::ConditionExpr::SourceIsAttacking)
            }
            (StatusConditionSubjectAst::Source, StatusConditionStateAst::AttackingAlone) => {
                let mut attacking_creatures = ObjectFilter::creature();
                attacking_creatures.attacking = true;
                Some(crate::ConditionExpr::And(
                    Box::new(crate::ConditionExpr::SourceIsAttacking),
                    Box::new(crate::ConditionExpr::CountComparison {
                        count: AnthemCountExpression::MatchingFilter(attacking_creatures),
                        comparison: Comparison::Equal(1),
                        display: Some("no other creatures are attacking".to_string()),
                    }),
                ))
            }
            (StatusConditionSubjectAst::Source, StatusConditionStateAst::Monstrous) => {
                Some(crate::ConditionExpr::SourceIsMonstrous)
            }
            (StatusConditionSubjectAst::EquippedCreature, StatusConditionStateAst::Tapped) => {
                Some(crate::ConditionExpr::EquippedCreatureTapped)
            }
            (StatusConditionSubjectAst::EquippedCreature, StatusConditionStateAst::Untapped) => {
                Some(crate::ConditionExpr::EquippedCreatureUntapped)
            }
            (StatusConditionSubjectAst::EquippedCreature, StatusConditionStateAst::Attacking) => {
                Some(crate::ConditionExpr::EquippedCreatureAttacking)
            }
            _ => None,
        }
    }
}

impl SubjectDescriptorConditionAst {
    pub fn condition_expr(self, display: String) -> crate::ConditionExpr {
        if self.subject == SubjectDescriptorConditionSubjectAst::AttachedObject {
            let mut descriptor_filter = ObjectFilter::default();
            apply_object_descriptor_to_filter(&mut descriptor_filter, self.descriptor);
            return crate::ConditionExpr::AttachedToSourceMatches(descriptor_filter);
        }

        if self.subject == SubjectDescriptorConditionSubjectAst::EnchantedPermanent {
            match self.descriptor {
                ObjectDescriptorAst::CardType(CardType::Creature) => {
                    return crate::ConditionExpr::EnchantedPermanentIsCreature;
                }
                ObjectDescriptorAst::CardType(CardType::Land) => {
                    return crate::ConditionExpr::EnchantedPermanentIsLand;
                }
                ObjectDescriptorAst::Subtype(Subtype::Equipment) => {
                    return crate::ConditionExpr::EnchantedPermanentIsEquipment;
                }
                ObjectDescriptorAst::Subtype(Subtype::Vehicle) => {
                    return crate::ConditionExpr::EnchantedPermanentIsVehicle;
                }
                _ => {}
            }
        }

        let mut filter = self.filter;
        apply_object_descriptor_to_filter(&mut filter, self.descriptor);
        crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison: Comparison::GreaterThanOrEqual(1),
            display: Some(display),
        }
    }
}

impl PlayerStatusConditionAst {
    pub fn condition_expr(self) -> crate::ConditionExpr {
        match self.status {
            PlayerStatusAst::Monarch => crate::ConditionExpr::PlayerIsMonarch {
                player: self.player,
            },
            PlayerStatusAst::Initiative => crate::ConditionExpr::PlayerHasInitiative {
                player: self.player,
            },
            PlayerStatusAst::MaxSpeed => crate::ConditionExpr::ValueComparison {
                left: Value::Speed(self.player),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(4),
            },
        }
    }
}

impl PlayerAchievementConditionAst {
    pub fn condition_expr(self) -> crate::ConditionExpr {
        let condition = match self.achievement {
            PlayerAchievementAst::CitysBlessing => crate::ConditionExpr::PlayerHasCitysBlessing {
                player: self.player,
            },
            PlayerAchievementAst::CompletedDungeon { dungeon_name } => {
                crate::ConditionExpr::PlayerCompletedDungeon {
                    player: self.player,
                    dungeon_name,
                }
            }
            PlayerAchievementAst::FullParty => crate::ConditionExpr::YouHaveFullParty,
            PlayerAchievementAst::VisitedAttractionThisTurn => crate::ConditionExpr::TurnHistory(
                ironsmith_core::TurnHistoryCondition::PlayerVisitedAttractionThisTurn(self.player),
            ),
        };
        if self.negated {
            crate::ConditionExpr::Not(Box::new(condition))
        } else {
            condition
        }
    }
}

impl PlayerCardsInHandConditionAst {
    pub fn condition_expr(self) -> Option<crate::ConditionExpr> {
        if let Some(count) = comparison_to_strict_at_least_threshold(&self.comparison) {
            return Some(crate::ConditionExpr::PlayerCardsInHandOrMore {
                player: self.player,
                count: count as i32,
            });
        }
        if let Some(count) = comparison_to_strict_at_most_threshold(&self.comparison) {
            return Some(crate::ConditionExpr::PlayerCardsInHandOrFewer {
                player: self.player,
                count: count as i32,
            });
        }
        None
    }

    pub fn is_no_cards_in_hand(&self) -> bool {
        comparison_to_strict_at_most_threshold(&self.comparison) == Some(0)
    }
}

impl PlayerLifeTotalConditionAst {
    pub fn condition_expr(self) -> Option<crate::ConditionExpr> {
        let (operator, right) = comparison_to_value_comparison_operator(self.comparison)?;
        Some(crate::ConditionExpr::ValueComparison {
            left: Value::LifeTotal(self.player),
            operator,
            right: Value::Fixed(right),
        })
    }
}

pub fn parse_control_condition(
    tokens: &[OwnedLexToken],
    options: ControlConditionOptions,
) -> Option<ControlConditionAst> {
    parse_control_condition_shape(tokens, options)
}

pub fn parse_control_relation_tail_clause(
    tokens: &[OwnedLexToken],
    options: ControlConditionOptions,
) -> Option<LexedClause<'_>> {
    let captured = parse_control_relation_clauses(tokens, options.allow_different_powers_tail)?;
    parse_control_condition_subject_clause(captured.subject_clause, options)?;
    Some(captured.tail_clause)
}

pub fn parse_control_condition_words(
    words: &[&str],
    options: ControlConditionOptions,
) -> Option<ControlConditionAst> {
    let shape =
        relation_shapes::parse_control_relation_words(words, options.allow_different_powers_tail)?;
    let (player, player_filter) =
        parse_control_condition_subject_words(shape.subject_words, options)?;
    finish_control_condition_words(
        player,
        player_filter,
        shape.prefix_words,
        shape.tail_words,
        shape.has_different_powers_modifier,
        options,
    )
}

struct PossessionRelationCapture<'a> {
    subject_clause: LexedClause<'a>,
    prefix_tokens: &'a [OwnedLexToken],
    tail_tokens: &'a [OwnedLexToken],
    has_modifier: bool,
}

pub struct ControlRelationClauses<'a> {
    pub subject_clause: LexedClause<'a>,
    pub tail_clause: LexedClause<'a>,
}

pub struct NegatedControlRelationClauses<'a> {
    pub subject_clause: LexedClause<'a>,
    pub negation_clause: LexedClause<'a>,
    pub tail_clause: LexedClause<'a>,
}

pub struct HasRelationClauses<'a> {
    pub subject_clause: LexedClause<'a>,
    pub tail_clause: LexedClause<'a>,
}

pub struct CopulaRelationClauses<'a> {
    pub subject_clause: LexedClause<'a>,
    pub tail_clause: LexedClause<'a>,
}

pub struct PrepositionalCopulaRelationClauses<'a> {
    pub subject_clause: LexedClause<'a>,
    pub preposition_clause: LexedClause<'a>,
    pub tail_clause: LexedClause<'a>,
}

pub fn parse_copula_relation_clauses(
    tokens: &[OwnedLexToken],
) -> Option<CopulaRelationClauses<'_>> {
    let captured =
        match_possession_relation_shape(tokens, relation_shapes::PossessionAction::Copula, false)?;
    Some(CopulaRelationClauses {
        subject_clause: captured.subject_clause,
        tail_clause: LexedClause::new(captured.tail_tokens).trimmed(),
    })
}

pub fn parse_prepositional_copula_relation_clauses<'a>(
    tokens: &'a [OwnedLexToken],
    preposition_words: &[&str],
) -> Option<PrepositionalCopulaRelationClauses<'a>> {
    let shape = relation_shapes::parse_prepositional_copula(tokens, preposition_words)?;
    Some(PrepositionalCopulaRelationClauses {
        subject_clause: LexedClause::new(shape.subject_tokens),
        preposition_clause: LexedClause::new(shape.preposition_tokens),
        tail_clause: LexedClause::new(shape.tail_tokens).trimmed(),
    })
}

pub fn parse_existential_object_clause(tokens: &[OwnedLexToken]) -> Option<LexedClause<'_>> {
    relation_shapes::parse_existential_object(tokens)
        .map(LexedClause::new)
        .map(LexedClause::trimmed)
}

pub fn parse_has_relation_clauses(tokens: &[OwnedLexToken]) -> Option<HasRelationClauses<'_>> {
    let captured =
        match_possession_relation_shape(tokens, relation_shapes::PossessionAction::Has, false)?;
    Some(HasRelationClauses {
        subject_clause: captured.subject_clause,
        tail_clause: LexedClause::new(captured.tail_tokens).trimmed(),
    })
}

pub fn parse_control_relation_clauses(
    tokens: &[OwnedLexToken],
    allow_different_powers_tail: bool,
) -> Option<ControlRelationClauses<'_>> {
    let captured = match_possession_relation_shape(
        tokens,
        relation_shapes::PossessionAction::Control,
        allow_different_powers_tail,
    )?;
    Some(ControlRelationClauses {
        subject_clause: captured.subject_clause,
        tail_clause: LexedClause::new(captured.tail_tokens).trimmed(),
    })
}

pub fn parse_control_or_controlled_relation_clauses(
    tokens: &[OwnedLexToken],
) -> Option<ControlRelationClauses<'_>> {
    let captured = match_possession_relation_shape(
        tokens,
        relation_shapes::PossessionAction::ControlOrControlled,
        false,
    )?;
    Some(ControlRelationClauses {
        subject_clause: captured.subject_clause,
        tail_clause: LexedClause::new(captured.tail_tokens).trimmed(),
    })
}

pub fn parse_negated_control_relation_clauses(
    tokens: &[OwnedLexToken],
) -> Option<NegatedControlRelationClauses<'_>> {
    let shape = relation_shapes::parse_negated_control(tokens)?;
    Some(NegatedControlRelationClauses {
        subject_clause: LexedClause::new(shape.subject_tokens),
        negation_clause: LexedClause::new(shape.negation_tokens),
        tail_clause: LexedClause::new(shape.tail_tokens).trimmed(),
    })
}

fn match_possession_relation_shape(
    tokens: &[OwnedLexToken],
    action: relation_shapes::PossessionAction,
    allow_different_powers: bool,
) -> Option<PossessionRelationCapture<'_>> {
    let shape = relation_shapes::parse_possession_relation(tokens, action, allow_different_powers)?;
    Some(PossessionRelationCapture {
        subject_clause: LexedClause::new(shape.subject_tokens),
        prefix_tokens: shape.prefix_tokens,
        tail_tokens: shape.tail_tokens,
        has_modifier: shape.has_different_powers_modifier,
    })
}

fn parse_control_condition_shape(
    tokens: &[OwnedLexToken],
    options: ControlConditionOptions,
) -> Option<ControlConditionAst> {
    let captured = match_possession_relation_shape(
        tokens,
        relation_shapes::PossessionAction::Control,
        options.allow_different_powers_tail,
    )?;

    let (player, player_filter) =
        parse_control_condition_subject_clause(captured.subject_clause, options)?;

    finish_control_condition(
        player,
        player_filter,
        captured.prefix_tokens,
        captured.tail_tokens,
        captured.has_modifier,
        options,
    )
}

fn parse_control_condition_subject_clause(
    clause: LexedClause<'_>,
    options: ControlConditionOptions,
) -> Option<(PlayerAst, Option<PlayerFilter>)> {
    let reference = parse_leaf_player_reference_tokens(
        clause.tokens(),
        control_subject_reference_mode(options),
    )?;
    lower_control_subject_reference(reference)
}

fn parse_control_condition_subject_words(
    words: &[&str],
    options: ControlConditionOptions,
) -> Option<(PlayerAst, Option<PlayerFilter>)> {
    let reference =
        parse_leaf_player_reference_words(words, control_subject_reference_mode(options))?;
    lower_control_subject_reference(reference)
}

fn control_subject_reference_mode(options: ControlConditionOptions) -> LeafPlayerReferenceMode {
    LeafPlayerReferenceMode::ControlSubject {
        allow_that_player: options.allow_that_player,
        allow_opponent_players: options.allow_opponent_players,
        allow_defending_player: options.allow_defending_player,
    }
}

fn lower_control_subject_reference(
    reference: LeafPlayerReference,
) -> Option<(PlayerAst, Option<PlayerFilter>)> {
    match reference {
        LeafPlayerReference::You => Some((PlayerAst::You, Some(PlayerFilter::You))),
        LeafPlayerReference::ThatPlayer => Some((PlayerAst::That, None)),
        LeafPlayerReference::Opponent => Some((PlayerAst::Opponent, Some(PlayerFilter::Opponent))),
        LeafPlayerReference::DefendingPlayer => {
            Some((PlayerAst::Defending, Some(PlayerFilter::Defending)))
        }
        _ => None,
    }
}

fn finish_control_condition(
    player: PlayerAst,
    player_filter: Option<PlayerFilter>,
    prefix_tokens: &[OwnedLexToken],
    tail_tokens: &[OwnedLexToken],
    captured_requires_different_powers: bool,
    options: ControlConditionOptions,
) -> Option<ControlConditionAst> {
    let tail_tokens = trim_edge_punctuation_tokens(tail_tokens);
    let (comparison, quantity_len) =
        parse_quantity_comparison_prefix(tail_tokens, true, true, "control condition").ok()?;
    let quantity_words = tail_tokens
        .get(..quantity_len)?
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut filter_tokens = trim_edge_punctuation_tokens(tail_tokens.get(quantity_len..)?);
    if filter_tokens.is_empty() {
        return None;
    }
    let different_powers_suffix = split_control_condition_filter_suffix(filter_tokens);
    let requires_different_powers = captured_requires_different_powers
        || options.allow_different_powers_tail && different_powers_suffix.is_some();
    if requires_different_powers {
        filter_tokens = trim_edge_punctuation_tokens(different_powers_suffix?.0);
        if filter_tokens.is_empty() {
            return None;
        }
    }
    let object_words = filter_tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .map(str::to_string)
        .collect::<Vec<_>>();

    let mut filter = match parse_object_filter_with_grammar_entrypoint(filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => {
            let prefixed_filter_tokens = prefix_tokens
                .iter()
                .chain(filter_tokens.iter())
                .cloned()
                .collect::<Vec<_>>();
            parse_object_filter_with_grammar_entrypoint(&prefixed_filter_tokens, false).ok()?
        }
    };
    if filter.zone.is_none() {
        filter.zone = options.default_filter_zone;
    }
    if tail_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "another")
    {
        filter.other = true;
    }
    if options.bind_filter_controller_to_subject && filter.controller.is_none() {
        filter.controller = player_filter.clone();
    }

    Some(ControlConditionAst {
        player,
        player_filter,
        comparison,
        quantity_token_count: quantity_len,
        quantity_words,
        object_words,
        filter,
        requires_different_powers,
    })
}

fn finish_control_condition_words(
    player: PlayerAst,
    player_filter: Option<PlayerFilter>,
    prefix_words: &[&str],
    tail_words: &[&str],
    captured_requires_different_powers: bool,
    options: ControlConditionOptions,
) -> Option<ControlConditionAst> {
    let (comparison, quantity_word_count) =
        parse_quantity_comparison_prefix_words(tail_words, true, true, "control condition").ok()?;
    let quantity_words = tail_words
        .get(..quantity_word_count)?
        .iter()
        .map(|word| (*word).to_string())
        .collect::<Vec<_>>();
    let mut filter_words = tail_words.get(quantity_word_count..)?;
    if filter_words.is_empty() {
        return None;
    }
    let different_powers_suffix = split_control_condition_filter_suffix_words(filter_words);
    let requires_different_powers = captured_requires_different_powers
        || options.allow_different_powers_tail && different_powers_suffix.is_some();
    if requires_different_powers {
        filter_words = different_powers_suffix?.0;
        if filter_words.is_empty() {
            return None;
        }
    }
    let object_words = filter_words
        .iter()
        .map(|word| (*word).to_string())
        .collect::<Vec<_>>();

    let mut filter = match parse_object_filter_words(filter_words, false) {
        Ok(filter) => filter,
        Err(_) => {
            let prefixed_filter_words = prefix_words
                .iter()
                .chain(filter_words.iter())
                .copied()
                .collect::<Vec<_>>();
            parse_object_filter_words(&prefixed_filter_words, false).ok()?
        }
    };
    if filter.zone.is_none() {
        filter.zone = options.default_filter_zone;
    }
    if tail_words.first().is_some_and(|word| *word == "another") {
        filter.other = true;
    }
    if options.bind_filter_controller_to_subject && filter.controller.is_none() {
        filter.controller = player_filter.clone();
    }

    Some(ControlConditionAst {
        player,
        player_filter,
        comparison,
        quantity_token_count: quantity_word_count,
        quantity_words,
        object_words,
        filter,
        requires_different_powers,
    })
}

pub fn parse_ownership_condition(
    tokens: &[OwnedLexToken],
    options: OwnershipConditionOptions,
) -> Option<OwnershipConditionAst> {
    parse_ownership_condition_shape(tokens, options)
}

fn parse_ownership_condition_shape(
    tokens: &[OwnedLexToken],
    options: OwnershipConditionOptions,
) -> Option<OwnershipConditionAst> {
    let captured =
        match_possession_relation_shape(tokens, relation_shapes::PossessionAction::Own, false)?;
    let (player, player_filter) =
        parse_ownership_condition_subject_clause(captured.subject_clause, options)?;

    finish_ownership_condition(player, player_filter, captured.tail_tokens, options)
}

fn parse_ownership_condition_subject_clause(
    clause: LexedClause<'_>,
    options: OwnershipConditionOptions,
) -> Option<(PlayerAst, Option<PlayerFilter>)> {
    let reference = parse_leaf_player_reference_tokens(
        clause.tokens(),
        LeafPlayerReferenceMode::OwnershipSubject {
            allow_opponent_players: options.allow_opponent_players,
        },
    )?;
    match reference {
        LeafPlayerReference::You => Some((PlayerAst::You, Some(PlayerFilter::You))),
        LeafPlayerReference::Opponent => Some((PlayerAst::Opponent, Some(PlayerFilter::Opponent))),
        _ => None,
    }
}

fn finish_ownership_condition(
    player: PlayerAst,
    player_filter: Option<PlayerFilter>,
    tail_tokens: &[OwnedLexToken],
    options: OwnershipConditionOptions,
) -> Option<OwnershipConditionAst> {
    let tail_tokens = trim_edge_punctuation_tokens(tail_tokens);
    let (comparison, quantity_len) =
        parse_quantity_comparison_prefix(tail_tokens, true, true, "ownership condition").ok()?;
    let quantity_words = tail_tokens
        .get(..quantity_len)?
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let filter_tokens = trim_edge_punctuation_tokens(tail_tokens.get(quantity_len..)?);
    if filter_tokens.is_empty() {
        return None;
    }
    let object_words = filter_tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .map(str::to_string)
        .collect::<Vec<_>>();

    let Ok(mut filter) = parse_object_filter_with_grammar_entrypoint(filter_tokens, false) else {
        return None;
    };
    if filter.zone.is_none() {
        filter.zone = options.default_filter_zone;
    }
    if options.bind_filter_owner_to_subject && filter.owner.is_none() {
        filter.owner = player_filter.clone();
    }

    Some(OwnershipConditionAst {
        player,
        player_filter,
        comparison,
        quantity_token_count: quantity_len,
        quantity_words,
        object_words,
        filter,
    })
}

pub fn parse_subject_status_condition(
    tokens: &[OwnedLexToken],
) -> Option<SubjectStatusConditionAst> {
    parse_subject_status_shape(tokens)
}

fn parse_subject_status_shape(tokens: &[OwnedLexToken]) -> Option<SubjectStatusConditionAst> {
    status_shapes::parse_subject_status(tokens)
}

pub fn parse_subject_descriptor_condition(
    tokens: &[OwnedLexToken],
) -> Option<SubjectDescriptorConditionAst> {
    parse_subject_descriptor_shape(tokens)
}

pub fn parse_object_attached_to_object_condition(
    tokens: &[OwnedLexToken],
) -> Option<ObjectAttachedToObjectConditionAst> {
    let relation = parse_copula_relation_clauses(tokens)?;
    let (_, attached_to_tokens) = primitives::parse_prefix(
        relation.tail_clause.tokens(),
        primitives::phrase(&["attached", "to"]),
    )?;
    let subject_tokens = relation.subject_clause.tokens();
    let (comparison, quantity_tokens) =
        parse_quantity_comparison_prefix(subject_tokens, true, true, "attachment condition")
            .ok()?;
    let attachment_tokens = subject_tokens.get(quantity_tokens..)?;
    let mut attachment_filter =
        parse_object_filter_with_grammar_entrypoint(attachment_tokens, false).ok()?;
    let mut attached_to_filter =
        parse_object_filter_with_grammar_entrypoint(attached_to_tokens, false).ok()?;
    attachment_filter.zone.get_or_insert(Zone::Battlefield);
    attached_to_filter.zone.get_or_insert(Zone::Battlefield);
    Some(ObjectAttachedToObjectConditionAst {
        attachment_filter,
        attached_to_filter,
        comparison,
        display: crate::lexer::token_word_refs(tokens).join(" "),
    })
}

fn parse_subject_descriptor_shape(
    tokens: &[OwnedLexToken],
) -> Option<SubjectDescriptorConditionAst> {
    let relation = parse_copula_relation_clauses(tokens)?;
    let subject = parse_subject_descriptor_subject_clause(relation.subject_clause)?;
    let descriptor = parse_object_descriptor_clause(relation.tail_clause)?;
    let filter =
        parse_object_filter_with_grammar_entrypoint(relation.subject_clause.tokens(), false)
            .ok()?;

    Some(SubjectDescriptorConditionAst {
        subject,
        filter,
        descriptor,
    })
}

fn parse_subject_descriptor_subject_clause(
    clause: LexedClause<'_>,
) -> Option<SubjectDescriptorConditionSubjectAst> {
    status_shapes::parse_subject_descriptor_subject(clause.tokens())
}

fn parse_object_descriptor_clause(clause: LexedClause<'_>) -> Option<ObjectDescriptorAst> {
    let (_, tokens) = primitives::parse_prefix(
        clause.trimmed().tokens(),
        opt(alt((
            primitives::kw("a"),
            primitives::kw("an"),
            primitives::kw("the"),
        )))
        .void(),
    )?;
    let [descriptor] = tokens else {
        return None;
    };
    parse_object_descriptor_word(descriptor.as_word()?)
}

pub fn parse_player_status_condition(tokens: &[OwnedLexToken]) -> Option<PlayerStatusConditionAst> {
    parse_player_status_shape(tokens)
}

fn parse_player_status_shape(tokens: &[OwnedLexToken]) -> Option<PlayerStatusConditionAst> {
    let shape = status_shapes::parse_player_status_tokens(tokens)?;
    let player = match shape.subject_tokens {
        Some(subject) => parse_player_status_subject_clause(LexedClause::new(subject))?,
        None => PlayerFilter::You,
    };
    Some(PlayerStatusConditionAst {
        player,
        status: shape.status,
    })
}

pub fn parse_player_achievement_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerAchievementConditionAst> {
    parse_player_achievement_shape(tokens)
}

fn parse_player_achievement_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerAchievementConditionAst> {
    status_shapes::parse_player_achievement(tokens)
}

pub fn parse_player_cards_in_hand_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerCardsInHandConditionAst> {
    let cards_in_hand_phrases: &[&[&str]] = &[
        &["card", "in", "hand"],
        &["cards", "in", "hand"],
        &["card", "in", "your", "hand"],
        &["cards", "in", "your", "hand"],
        &["card", "in", "their", "hand"],
        &["cards", "in", "their", "hand"],
    ];
    if let Some(condition) = parse_player_has_quantity_object_condition(
        tokens,
        cards_in_hand_phrases,
        "cards-in-hand condition",
    ) {
        Some(PlayerCardsInHandConditionAst {
            player: condition.player,
            comparison: condition.comparison,
        })
    } else {
        None
    }
}

pub fn parse_player_life_total_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeTotalConditionAst> {
    let life_phrases: &[&[&str]] = &[&["life"]];
    if let Some(condition) =
        parse_player_has_quantity_object_condition(tokens, life_phrases, "life-total condition")
    {
        Some(PlayerLifeTotalConditionAst {
            player: condition.player,
            comparison: condition.comparison,
        })
    } else {
        None
    }
}

pub fn parse_player_life_tie_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeTieConditionAst> {
    let shape = life_tie_shapes::parse_player_life_tie_condition_tokens(tokens)?;
    Some(PlayerLifeTieConditionAst {
        minimum_players: shape.minimum_players,
        tied_players: shape.tied_players,
    })
}

pub fn parse_player_life_tie_choice_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeTieChoiceConditionAst<'_>> {
    let shape = life_tie_shapes::parse_player_life_tie_choice_conditional_tokens(tokens)?;
    Some(PlayerLifeTieChoiceConditionAst {
        minimum_players: shape.minimum_players,
        tied_players: shape.tied_players,
        consequence_tokens: shape.consequence_tokens,
    })
}

pub fn parse_player_has_quantity_object_condition(
    tokens: &[OwnedLexToken],
    object_phrases: &[&[&str]],
    context: &str,
) -> Option<PlayerHasQuantityObjectConditionAst> {
    let relation = parse_has_relation_clauses(tokens)?;
    let player = parse_player_has_quantity_subject_clause(relation.subject_clause)?;
    let shape =
        event_shapes::parse_quantity_object_tail(relation.tail_clause.tokens(), object_phrases)?;
    let (comparison, used) =
        parse_quantity_comparison_prefix(shape.amount_tokens, false, false, context).ok()?;
    (used == shape.amount_tokens.len())
        .then_some(PlayerHasQuantityObjectConditionAst { player, comparison })
}

pub fn parse_player_life_relation_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeRelationConditionAst> {
    parse_player_life_relation_shape(tokens)
}

fn parse_player_life_relation_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeRelationConditionAst> {
    let words = crate::lexer::token_word_refs(tokens);
    if words
        == [
            "you", "have", "the", "most", "life", "or", "are", "tied", "for", "most", "life",
        ]
    {
        return Some(PlayerLifeRelationConditionAst {
            player: PlayerFilter::You,
            relation: PlayerLifeRelationAst::HasNoOpponentWithMoreLifeThan,
        });
    }
    if let Some(condition) = parse_no_opponent_more_life_than_shape(tokens) {
        return Some(condition);
    }

    let relation = parse_has_relation_clauses(tokens)?;
    let subject = parse_life_relation_player_subject_clause(relation.subject_clause)?;
    let relation_clause = relation.tail_clause;

    match parse_life_relation_shape(relation_clause)? {
        LifeRelationShape::MoreThanYou => Some(PlayerLifeRelationConditionAst {
            player: subject,
            relation: PlayerLifeRelationAst::HasMoreLifeThanYou,
        }),
        LifeRelationShape::MoreThanEachOtherPlayer => Some(PlayerLifeRelationConditionAst {
            player: subject,
            relation: PlayerLifeRelationAst::HasMoreLifeThanEachOtherPlayer,
        }),
        LifeRelationShape::MoreThanEachOpponent if subject == PlayerFilter::You => {
            Some(PlayerLifeRelationConditionAst {
                player: subject,
                relation: PlayerLifeRelationAst::HasMoreLifeThanEachOtherPlayer,
            })
        }
        LifeRelationShape::MoreThanPlayer(player) if subject == PlayerFilter::You => {
            Some(PlayerLifeRelationConditionAst {
                player,
                relation: PlayerLifeRelationAst::HasLessLifeThanYou,
            })
        }
        _ => None,
    }
}

fn parse_life_relation_shape(relation_clause: LexedClause<'_>) -> Option<LifeRelationShape> {
    let (kind, player_tokens) = event_shapes::parse_life_relation(relation_clause.tokens())?;
    if let Some(player_tokens) = player_tokens {
        let player = parse_life_relation_player_subject_clause(LexedClause::new(player_tokens))?;
        Some(LifeRelationShape::MoreThanPlayer(player))
    } else {
        Some(kind)
    }
}

fn parse_no_opponent_more_life_than_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeRelationConditionAst> {
    let shape = event_shapes::parse_no_opponent_more_life_than(tokens)?;
    let player = parse_life_relation_player_subject_clause(LexedClause::new(shape.player_tokens))?;
    Some(PlayerLifeRelationConditionAst {
        player,
        relation: PlayerLifeRelationAst::HasNoOpponentWithMoreLifeThan,
    })
}

pub fn parse_player_cards_in_hand_relation_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerCardsInHandRelationConditionAst> {
    parse_player_cards_in_hand_relation_shape(tokens)
}

fn parse_player_cards_in_hand_relation_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerCardsInHandRelationConditionAst> {
    let relation = parse_has_relation_clauses(tokens)?;
    let subject = parse_life_relation_player_subject_clause(relation.subject_clause)?;
    let relation_clause = relation.tail_clause;

    match parse_cards_in_hand_relation_shape(relation_clause)? {
        CardsInHandRelationShape::MoreThanYou => Some(PlayerCardsInHandRelationConditionAst {
            player: subject,
            relation: PlayerCardsInHandRelationAst::HasMoreCardsInHandThanYou,
        }),
        CardsInHandRelationShape::MoreThanEachOtherPlayer => {
            Some(PlayerCardsInHandRelationConditionAst {
                player: subject,
                relation: PlayerCardsInHandRelationAst::HasMoreCardsInHandThanEachOtherPlayer,
            })
        }
    }
}

fn parse_cards_in_hand_relation_shape(
    relation_clause: LexedClause<'_>,
) -> Option<CardsInHandRelationShape> {
    event_shapes::parse_cards_in_hand_relation(relation_clause.tokens())
}

pub fn parse_player_turn_event_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerTurnEventConditionAst> {
    parse_player_turn_event_shape(tokens)
}

fn parse_player_turn_event_shape(tokens: &[OwnedLexToken]) -> Option<PlayerTurnEventConditionAst> {
    parse_cards_drawn_this_turn_shape(tokens)
        .or_else(|| parse_lands_entered_this_turn_shape(tokens))
}

fn parse_cards_drawn_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerTurnEventConditionAst> {
    let shape = event_shapes::parse_cards_drawn_this_turn(tokens)?;
    let player = parse_life_relation_player_subject_clause(LexedClause::new(shape.subject_tokens))?;
    let (comparison, used) = parse_quantity_comparison_prefix(
        shape.amount_tokens,
        false,
        false,
        "cards-drawn condition",
    )
    .ok()?;
    (used == shape.amount_tokens.len()).then_some(PlayerTurnEventConditionAst {
        player,
        event: PlayerTurnEventAst::CardsDrawn,
        comparison,
    })
}

fn parse_lands_entered_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerTurnEventConditionAst> {
    let shape = event_shapes::parse_lands_entered_this_turn(tokens)?;
    let player = parse_life_relation_player_subject_clause(LexedClause::new(shape.subject_tokens))?;
    let comparison =
        super::leaf::parse_leaf_another_event_count_comparison_tokens(shape.amount_tokens)
            .ok()
            .flatten()
            .or_else(|| {
                let (comparison, used) = parse_quantity_comparison_prefix(
                    shape.amount_tokens,
                    false,
                    false,
                    "lands-entered condition",
                )
                .ok()?;
                (used == shape.amount_tokens.len()).then_some(comparison)
            })?;
    Some(PlayerTurnEventConditionAst {
        player,
        event: PlayerTurnEventAst::LandsEnteredBattlefieldUnderControl,
        comparison,
    })
}

pub fn parse_spell_context_condition(tokens: &[OwnedLexToken]) -> Option<SpellContextConditionAst> {
    parse_spell_context_condition_shape(tokens)
}

fn parse_spell_context_condition_shape(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextConditionAst> {
    parse_target_spell_controller_poisoned_shape(tokens)
        .or_else(|| parse_no_mana_spent_to_cast_target_spell_shape(tokens))
        .or_else(|| parse_you_control_more_creatures_than_spell_controller_shape(tokens))
}

fn parse_target_spell_controller_poisoned_shape(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextConditionAst> {
    let shape = event_shapes::parse_target_spell_controller_poisoned(tokens)?;
    let spell = event_shapes::parse_target_spell_controller(shape.controller_tokens)?;
    Some(SpellContextConditionAst::ControllerIsPoisoned { spell })
}

fn parse_no_mana_spent_to_cast_target_spell_shape(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextConditionAst> {
    let shape = event_shapes::parse_no_mana_spent_to_cast(tokens)?;
    let spell = event_shapes::parse_target_spell_reference(shape.spell_tokens)?;
    Some(SpellContextConditionAst::NoManaSpentToCast { spell })
}

fn parse_you_control_more_creatures_than_spell_controller_shape(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextConditionAst> {
    let relation = parse_control_relation_clauses(tokens, false)?;
    if !status_shapes::is_you_subject(relation.subject_clause.tokens()) {
        return None;
    }
    let shape = event_shapes::parse_more_creatures_than_controller(relation.tail_clause.tokens())?;
    let spell = event_shapes::parse_target_spell_controller(shape.controller_tokens)?;
    Some(SpellContextConditionAst::YouControlMoreCreaturesThanController { spell })
}

pub fn parse_player_spell_cast_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerSpellCastThisTurnConditionAst> {
    parse_player_spell_cast_this_turn_shape(tokens)
}

fn parse_player_spell_cast_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerSpellCastThisTurnConditionAst> {
    let shape = event_shapes::parse_spell_cast_this_turn(tokens)?;
    let player = parse_spell_cast_this_turn_subject_clause(LexedClause::new(shape.subject_tokens))?;
    if !shape.negated
        && player == PlayerFilter::You
        && event_shapes::is_another_spell(shape.object_tokens)
    {
        return Some(PlayerSpellCastThisTurnConditionAst::CountAtLeast { player, count: 2 });
    }
    if !shape.negated
        && let Some((count, used)) = parse_greater_than_or_equal_quantity_prefix(
            shape.object_tokens,
            false,
            false,
            "spell-cast condition",
        )
        .ok()
        .flatten()
    {
        if shape
            .object_tokens
            .get(used)
            .is_some_and(|token| token.is_word("spell") || token.is_word("spells"))
            && used + 1 == shape.object_tokens.len()
        {
            return Some(PlayerSpellCastThisTurnConditionAst::CountAtLeast { player, count });
        }
        if let Some(filters) = parse_spell_cast_filter_tokens(&shape.object_tokens[used..])
            && let [filter] = filters.as_slice()
        {
            return Some(
                PlayerSpellCastThisTurnConditionAst::MatchingFilterCountAtLeast {
                    player,
                    filter: filter.clone(),
                    count,
                },
            );
        }
    }
    let filters = parse_spell_cast_filter_tokens(shape.object_tokens)?;
    if filters.is_empty() {
        return None;
    }
    Some(PlayerSpellCastThisTurnConditionAst::MatchingFilters {
        player,
        filters,
        negated: shape.negated,
    })
}

pub fn parse_player_life_change_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeChangeThisTurnConditionAst> {
    parse_player_life_change_this_turn_shape(tokens)
}

fn parse_player_life_change_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeChangeThisTurnConditionAst> {
    let shape = event_shapes::parse_life_change_this_turn(tokens)?;
    let player = parse_life_change_subject_clause(LexedClause::new(shape.subject_tokens))?;
    let comparison = if shape.amount_tokens.is_empty() {
        Comparison::GreaterThanOrEqual(1)
    } else {
        let (comparison, used) = parse_quantity_comparison_prefix(
            shape.amount_tokens,
            false,
            false,
            "life-change condition",
        )
        .ok()?;
        if used != shape.amount_tokens.len() {
            return None;
        }
        comparison
    };

    Some(PlayerLifeChangeThisTurnConditionAst {
        player,
        direction: shape.direction,
        comparison,
    })
}

pub fn parse_player_would_action_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerWouldActionConditionAst> {
    parse_player_would_action_shape(tokens)
}

fn parse_player_would_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerWouldActionConditionAst> {
    let shape = event_shapes::parse_player_would(tokens)?;
    let player = parse_player_would_subject_clause(LexedClause::new(shape.subject_tokens))?;
    Some(PlayerWouldActionConditionAst {
        player,
        action: shape.action,
    })
}

pub fn parse_battlefield_change_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    parse_battlefield_change_this_turn_shape(tokens)
}

fn parse_battlefield_change_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    match zone_change_shapes::parse_battlefield_change(tokens)? {
        zone_change_shapes::BattlefieldChangeShape::NoPermanentLeft => {
            Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: true })
        }
        zone_change_shapes::BattlefieldChangeShape::PermanentLeft => {
            Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: false })
        }
        zone_change_shapes::BattlefieldChangeShape::PermanentLeftUnderYourControl { surface } => {
            Some(
                BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefieldUnderYourControl {
                    surface,
                },
            )
        }
        zone_change_shapes::BattlefieldChangeShape::LandPutIntoGraveyardFromBattlefield => Some(
            BattlefieldChangeThisTurnConditionAst::ObjectPutIntoGraveyardFromBattlefield {
                filter: ObjectFilter::land().controlled_by(PlayerFilter::You),
            },
        ),
        zone_change_shapes::BattlefieldChangeShape::NonlandPermanentLeftOrSpellWarped => Some(
            BattlefieldChangeThisTurnConditionAst::NonlandPermanentLeftBattlefieldOrSpellWarped,
        ),
    }
}

pub fn parse_object_death_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<ObjectDeathThisTurnConditionAst> {
    parse_object_death_this_turn_shape(tokens)
}

fn parse_object_death_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<ObjectDeathThisTurnConditionAst> {
    match zone_change_shapes::parse_death(tokens)? {
        zone_change_shapes::DeathShape::Died(shape) => {
            let comparison = parse_object_death_amount(shape.amount_tokens)?;
            let damaged_by = shape.damaged_by.map(|damager| match damager {
                zone_change_shapes::DamagerShape::ThisCreature => DamageBySpec::ThisCreature,
                zone_change_shapes::DamagerShape::EquippedCreature => {
                    DamageBySpec::EquippedCreature
                }
                zone_change_shapes::DamagerShape::EnchantedCreature => {
                    DamageBySpec::EnchantedCreature
                }
            });
            Some(ObjectDeathThisTurnConditionAst {
                event: ObjectDeathThisTurnEventAst::Died,
                filter: ObjectFilter::creature(),
                comparison,
                under_controller: shape.under_your_control.then_some(PlayerFilter::You),
                damaged_by,
            })
        }
        zone_change_shapes::DeathShape::CreatureCardPutIntoYourGraveyard => {
            Some(ObjectDeathThisTurnConditionAst {
                event: ObjectDeathThisTurnEventAst::PutIntoYourGraveyardFromAnywhere,
                filter: ObjectFilter::creature(),
                comparison: Comparison::GreaterThanOrEqual(1),
                under_controller: None,
                damaged_by: None,
            })
        }
    }
}

fn parse_object_death_amount(tokens: &[OwnedLexToken]) -> Option<Comparison> {
    if tokens.is_empty()
        || primitives::parse_all(tokens, primitives::kw("a").void(), "death article").is_ok()
    {
        return Some(Comparison::GreaterThanOrEqual(1));
    }
    if primitives::parse_all(
        tokens,
        primitives::phrase(&["one", "or", "more"]).void(),
        "death minimum-one quantity",
    )
    .is_ok()
    {
        return Some(Comparison::GreaterThanOrEqual(1));
    }
    let (comparison, used) =
        parse_quantity_comparison_prefix(tokens, false, false, "object-death condition").ok()?;
    (used == tokens.len()).then_some(comparison)
}

pub fn parse_battlefield_entry_condition(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldEntryConditionAst> {
    parse_battlefield_entry_shape(tokens)
}

fn parse_battlefield_entry_shape(tokens: &[OwnedLexToken]) -> Option<BattlefieldEntryConditionAst> {
    match zone_change_shapes::parse_entry(tokens)? {
        zone_change_shapes::EntryShape::LandThisTurn => Some(
            BattlefieldEntryConditionAst::LandEnteredUnderYourControlThisTurn {
                player: PlayerAst::You,
            },
        ),
        zone_change_shapes::EntryShape::Object {
            object_tokens,
            window,
            other,
            you_had_surface,
        } => {
            let mut object_tokens = object_tokens;
            let mut min_count = None;
            let leading_words = crate::lexer::token_word_refs(object_tokens);
            let counted = match leading_words.as_slice() {
                ["two", "or", "more", ..] => Some(2u32),
                ["three", "or", "more", ..] => Some(3u32),
                ["four", "or", "more", ..] => Some(4u32),
                _ => None,
            };
            if let Some(count) = counted {
                // Drop the three count words at the token level.
                let mut seen = 0usize;
                let mut cut = 0usize;
                for (token_idx, token) in object_tokens.iter().enumerate() {
                    if token.as_word().is_some() {
                        seen += 1;
                    }
                    if seen == 3 {
                        cut = token_idx + 1;
                        break;
                    }
                }
                if cut > 0 {
                    object_tokens = &object_tokens[cut..];
                    min_count = Some(count);
                }
            }
            let mut filter =
                parse_object_filter_with_grammar_entrypoint(object_tokens, false).ok()?;
            filter.controller = Some(PlayerFilter::You);
            if other {
                filter.other = true;
            }
            filter.set_you_had_entry_surface(you_had_surface);
            Some(BattlefieldEntryConditionAst::ObjectEntered {
                filter,
                min_count,
                window: match window {
                    zone_change_shapes::EntryWindowShape::ThisTurn => {
                        BattlefieldEntryTurnWindowAst::ThisTurn
                    }
                    zone_change_shapes::EntryWindowShape::LastTurn => {
                        BattlefieldEntryTurnWindowAst::LastTurn
                    }
                },
            })
        }
    }
}

fn parse_player_status_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    let reference = parse_leaf_player_reference_tokens(
        clause.tokens(),
        LeafPlayerReferenceMode::PlayerStatusSubject,
    )?;
    lower_player_status_subject_reference(reference)
}

fn lower_player_status_subject_reference(reference: LeafPlayerReference) -> Option<PlayerFilter> {
    match reference {
        LeafPlayerReference::You => Some(PlayerFilter::You),
        LeafPlayerReference::DefendingPlayer => Some(PlayerFilter::Defending),
        LeafPlayerReference::AttackingPlayer => Some(PlayerFilter::Attacking),
        LeafPlayerReference::ThatPlayer => Some(PlayerFilter::IteratedPlayer),
        LeafPlayerReference::Opponent => Some(PlayerFilter::Opponent),
        LeafPlayerReference::AnyPlayer => Some(PlayerFilter::Any),
        _ => None,
    }
}

fn parse_player_has_quantity_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    let reference = parse_leaf_player_reference_tokens(
        clause.tokens(),
        LeafPlayerReferenceMode::PlayerHasQuantitySubject,
    )?;
    match reference {
        LeafPlayerReference::You => Some(PlayerFilter::You),
        LeafPlayerReference::Opponent => Some(PlayerFilter::Opponent),
        LeafPlayerReference::AnyPlayer => Some(PlayerFilter::Any),
        LeafPlayerReference::ThatPlayer => Some(PlayerFilter::IteratedPlayer),
        LeafPlayerReference::AttackingPlayer => Some(PlayerFilter::Attacking),
        LeafPlayerReference::DefendingPlayer => Some(PlayerFilter::Defending),
        _ => None,
    }
}

fn parse_life_relation_player_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    let reference = parse_leaf_player_reference_tokens(
        clause.tokens(),
        LeafPlayerReferenceMode::LifeRelationSubject,
    )?;
    match reference {
        LeafPlayerReference::You => Some(PlayerFilter::You),
        LeafPlayerReference::ThatPlayer => Some(PlayerFilter::IteratedPlayer),
        LeafPlayerReference::TargetPlayer => Some(PlayerFilter::target_player()),
        LeafPlayerReference::TargetOpponent => Some(PlayerFilter::target_opponent()),
        LeafPlayerReference::Opponent | LeafPlayerReference::EachOpponent => {
            Some(PlayerFilter::Opponent)
        }
        LeafPlayerReference::AnyPlayer => Some(PlayerFilter::Any),
        LeafPlayerReference::DefendingPlayer => Some(PlayerFilter::Defending),
        LeafPlayerReference::AttackingPlayer => Some(PlayerFilter::Attacking),
        #[cfg(test)]
        LeafPlayerReference::EachPlayer | LeafPlayerReference::ItsController => None,
    }
}

fn parse_spell_cast_this_turn_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    let reference = parse_leaf_player_reference_tokens(
        clause.tokens(),
        LeafPlayerReferenceMode::SpellCastThisTurnSubject,
    )?;
    match reference {
        LeafPlayerReference::ThatPlayer => Some(PlayerFilter::Active),
        LeafPlayerReference::You => Some(PlayerFilter::You),
        LeafPlayerReference::Opponent => Some(PlayerFilter::Opponent),
        _ => None,
    }
}

fn parse_spell_cast_filter_tokens(tokens: &[OwnedLexToken]) -> Option<Vec<ObjectFilter>> {
    if let Some((left, right)) = split_both_spell_cast_filter_tokens(tokens) {
        return Some(vec![
            parse_spell_cast_filter_tokens_single(left)?,
            parse_spell_cast_filter_tokens_single(right)?,
        ]);
    }
    Some(vec![parse_spell_cast_filter_tokens_single(tokens)?])
}

fn split_both_spell_cast_filter_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let shape = event_shapes::parse_spell_cast_filter_pair(tokens)?;
    Some((shape.left_tokens, shape.right_tokens))
}

fn parse_spell_cast_filter_tokens_single(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    parse_object_filter_with_grammar_entrypoint(tokens, false).ok()
}

fn parse_life_change_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    let reference = parse_leaf_player_reference_tokens(
        clause.tokens(),
        LeafPlayerReferenceMode::LifeChangeSubject,
    )?;
    match reference {
        LeafPlayerReference::You => Some(PlayerFilter::You),
        LeafPlayerReference::Opponent => Some(PlayerFilter::Opponent),
        LeafPlayerReference::AnyPlayer => Some(PlayerFilter::Any),
        _ => None,
    }
}

fn parse_player_would_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    let reference = parse_leaf_player_reference_tokens(
        clause.tokens(),
        LeafPlayerReferenceMode::PlayerWouldSubject,
    )?;
    match reference {
        LeafPlayerReference::You => Some(PlayerFilter::You),
        LeafPlayerReference::Opponent => Some(PlayerFilter::Opponent),
        _ => None,
    }
}

fn parse_object_descriptor_word(word: &str) -> Option<ObjectDescriptorAst> {
    parse_color(word)
        .map(ObjectDescriptorAst::Color)
        .or_else(|| parse_card_type(word).map(ObjectDescriptorAst::CardType))
        .or_else(|| parse_subtype_flexible(word).map(ObjectDescriptorAst::Subtype))
}

fn apply_object_descriptor_to_filter(filter: &mut ObjectFilter, descriptor: ObjectDescriptorAst) {
    match descriptor {
        ObjectDescriptorAst::Color(color) => filter.colors = Some(color),
        ObjectDescriptorAst::CardType(card_type) => filter.card_types.push(card_type),
        ObjectDescriptorAst::Subtype(subtype) => {
            *filter = std::mem::take(filter).with_subtype(subtype);
        }
    }
}

fn parse_control_condition_filter_suffix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ControlConditionFilterSuffix> {
    alt((
        primitives::phrase(&["with", "different", "powers"]),
        primitives::phrase(&["with", "different", "power"]),
    ))
    .value(ControlConditionFilterSuffix::DifferentPowers)
    .parse_next(input)
}

fn split_control_condition_filter_suffix(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], ControlConditionFilterSuffix)> {
    primitives::split_lexed_once_before_suffix(tokens, 1, || {
        parse_control_condition_filter_suffix_lexed
    })
}

fn parse_control_condition_filter_suffix_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<ControlConditionFilterSuffix> {
    alt((
        (
            primitives::word_slice_exact("with"),
            primitives::word_slice_exact("different"),
            primitives::word_slice_exact("powers"),
        ),
        (
            primitives::word_slice_exact("with"),
            primitives::word_slice_exact("different"),
            primitives::word_slice_exact("power"),
        ),
    ))
    .value(ControlConditionFilterSuffix::DifferentPowers)
    .parse_next(input)
}

fn split_control_condition_filter_suffix_words<'a>(
    words: &'a [&'a str],
) -> Option<(&'a [&'a str], ControlConditionFilterSuffix)> {
    let suffix_start = words.len().checked_sub(3)?;
    let suffix = primitives::parse_full_word_slice(
        &words[suffix_start..],
        parse_control_condition_filter_suffix_word_slice,
    )?;
    Some((&words[..suffix_start], suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_removed_from_draft_characteristic_condition_with_exact_group_name() {
        let tokens = lex_line(
            "you removed a creature card with flying from the draft with cards named Draft Mimic",
            0,
        )
        .expect("lex removed-from-draft condition");
        let parsed = parse_removed_from_draft_condition(&tokens)
            .expect("typed removed-from-draft condition");

        assert_eq!(parsed.player, PlayerFilter::You);
        assert!(parsed.filter.card_types.contains(&CardType::Creature));
        assert!(parsed.filter.has_explicit_card_noun());
        assert_eq!(
            parsed.filter.static_abilities,
            vec![crate::static_abilities::StaticAbilityId::Flying]
        );
        assert_eq!(parsed.with_cards_named, "Draft Mimic");

        let lowercase_source_name = lex_line(
            "you removed a creature card with flying from the draft with cards named animus of predation",
            0,
        )
        .expect("lex lowercase removed-from-draft condition");
        let lowercase_source_name = parse_removed_from_draft_condition(&lowercase_source_name)
            .expect("lowercase named group must remain characteristic data");
        assert_eq!(lowercase_source_name.filter.zone, None);
        assert!(
            lowercase_source_name
                .with_cards_named
                .eq_ignore_ascii_case("animus of predation"),
            "the named draft group may restore the current source's authored casing: {lowercase_source_name:#?}"
        );

        let near_miss = lex_line(
            "you removed a creature card with flying from your graveyard with cards named Draft Mimic",
            0,
        )
        .expect("lex near miss");
        assert!(parse_removed_from_draft_condition(&near_miss).is_none());
    }
    use crate::lexer::lex_line;

    #[test]
    fn life_change_condition_accepts_any_player_subject() {
        let tokens = lex_line("a player lost 4 or more life this turn", 0).expect("lex");
        assert_eq!(
            parse_player_life_change_this_turn_condition(&tokens),
            Some(PlayerLifeChangeThisTurnConditionAst {
                player: PlayerFilter::Any,
                direction: PlayerLifeChangeDirectionAst::Lost,
                comparison: Comparison::GreaterThanOrEqual(4),
            })
        );
    }

    #[test]
    fn parse_subject_status_condition_uses_shared_capture_shape() {
        let cases = [
            (
                "this creature is untapped",
                SubjectStatusConditionAst {
                    subject: StatusConditionSubjectAst::Source,
                    state: StatusConditionStateAst::Untapped,
                },
            ),
            (
                "this tapped",
                SubjectStatusConditionAst {
                    subject: StatusConditionSubjectAst::Source,
                    state: StatusConditionStateAst::Tapped,
                },
            ),
            (
                "equipped creature attacking",
                SubjectStatusConditionAst {
                    subject: StatusConditionSubjectAst::EquippedCreature,
                    state: StatusConditionStateAst::Attacking,
                },
            ),
            (
                "it is attacking alone",
                SubjectStatusConditionAst {
                    subject: StatusConditionSubjectAst::Source,
                    state: StatusConditionStateAst::AttackingAlone,
                },
            ),
        ];

        for (text, expected) in cases {
            let tokens = lex_line(text, 0).expect("lex");

            let parsed = parse_subject_status_condition(&tokens).expect(text);

            assert_eq!(parsed, expected, "{text}");
        }
    }

    #[test]
    fn parse_subject_descriptor_condition_uses_shared_capture_shape() {
        let cases = [
            (
                "enchanted permanent is a creature",
                SubjectDescriptorConditionSubjectAst::EnchantedPermanent,
                ObjectDescriptorAst::CardType(CardType::Creature),
            ),
            (
                "equipped creature is a human",
                SubjectDescriptorConditionSubjectAst::AttachedObject,
                ObjectDescriptorAst::Subtype(Subtype::Human),
            ),
        ];

        for (text, expected_subject, expected_descriptor) in cases {
            let tokens = lex_line(text, 0).expect("lex");

            let parsed = parse_subject_descriptor_condition(&tokens).expect(text);

            assert_eq!(parsed.subject, expected_subject, "{text}");
            assert_eq!(parsed.descriptor, expected_descriptor, "{text}");
            assert!(!parsed.filter.tagged_constraints.is_empty(), "{text}");
        }
    }

    #[test]
    fn parses_object_attachment_relationship_condition() {
        let tokens = lex_line(
            "an Equipment named Groom's Finery is attached to a creature you control",
            0,
        )
        .expect("lex attachment condition");
        let parsed =
            parse_object_attached_to_object_condition(&tokens).expect("typed attachment condition");
        assert_eq!(
            parsed.attachment_filter.name.as_deref(),
            Some("grooms finery")
        );
        assert!(
            parsed
                .attached_to_filter
                .card_types
                .contains(&CardType::Creature)
        );
        assert_eq!(
            parsed.attached_to_filter.controller,
            Some(PlayerFilter::You)
        );
        assert_eq!(parsed.comparison, Comparison::GreaterThanOrEqual(1));
        assert_eq!(
            parsed.display,
            "an Equipment named Groom's Finery is attached to a creature you control"
        );

        let tokens = lex_line("two or more Equipment are attached to it", 0)
            .expect("lex counted attachment condition");
        let parsed = parse_object_attached_to_object_condition(&tokens)
            .expect("counted attachment condition");
        assert_eq!(parsed.comparison, Comparison::GreaterThanOrEqual(2));
        assert!(
            parsed
                .attachment_filter
                .subtypes
                .contains(&Subtype::Equipment)
        );
        assert!(
            parsed
                .attached_to_filter
                .tagged_constraints
                .iter()
                .any(|constraint| {
                    constraint.tag.as_str() == "__it__"
                        && matches!(
                            constraint.relation,
                            crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        )
                })
        );

        let tokens = lex_line("another Aura is attached to enchanted creature", 0)
            .expect("lex other-Aura attachment condition");
        let parsed = parse_object_attached_to_object_condition(&tokens)
            .expect("other-Aura attachment condition");
        assert_eq!(parsed.comparison, Comparison::GreaterThanOrEqual(1));
        assert!(parsed.attachment_filter.other);
        assert!(parsed.attachment_filter.subtypes.contains(&Subtype::Aura));
    }

    #[test]
    fn parse_ownership_condition_uses_shared_capture_shape() {
        let cases = [
            (
                "you own three or more artifacts",
                OwnershipConditionAst {
                    player: PlayerAst::You,
                    player_filter: Some(PlayerFilter::You),
                    comparison: Comparison::GreaterThanOrEqual(3),
                    quantity_token_count: 3,
                    quantity_words: vec!["three".to_string(), "or".to_string(), "more".to_string()],
                    object_words: vec!["artifacts".to_string()],
                    filter: ObjectFilter::artifact().owned_by(PlayerFilter::You),
                },
            ),
            (
                "an opponent owns exactly two lands",
                OwnershipConditionAst {
                    player: PlayerAst::Opponent,
                    player_filter: Some(PlayerFilter::Opponent),
                    comparison: Comparison::Equal(2),
                    quantity_token_count: 2,
                    quantity_words: vec!["exactly".to_string(), "two".to_string()],
                    object_words: vec!["lands".to_string()],
                    filter: ObjectFilter::land().owned_by(PlayerFilter::Opponent),
                },
            ),
        ];

        for (text, expected) in cases {
            let tokens = lex_line(text, 0).expect("lex");

            let parsed = parse_ownership_condition(
                &tokens,
                OwnershipConditionOptions {
                    allow_opponent_players: true,
                    bind_filter_owner_to_subject: true,
                    default_filter_zone: None,
                },
            )
            .expect(text);

            assert_eq!(parsed, expected, "{text}");
        }
    }

    #[test]
    fn parse_control_condition_preserves_another_as_object_modifier() {
        let tokens = lex_line("you control another artifact", 0).expect("lex");

        let parsed = parse_control_condition(
            &tokens,
            ControlConditionOptions {
                bind_filter_controller_to_subject: true,
                ..ControlConditionOptions::default()
            },
        )
        .expect("control condition should parse");

        assert_eq!(parsed.at_least_count(), Some(1));
        assert_eq!(parsed.filter.card_types, vec![CardType::Artifact]);
        assert_eq!(parsed.filter.controller, Some(PlayerFilter::You));
        assert!(parsed.filter.other, "{parsed:?}");
    }

    #[test]
    fn parse_control_condition_supports_opt_in_defending_player_subject() {
        let tokens = lex_line("defending player controls a snow land", 0).expect("lex");

        let parsed = parse_control_condition(
            &tokens,
            ControlConditionOptions {
                allow_defending_player: true,
                bind_filter_controller_to_subject: false,
                ..ControlConditionOptions::default()
            },
        )
        .expect("defending-player control condition should parse");

        assert_eq!(parsed.player, PlayerAst::Defending);
        assert_eq!(parsed.player_filter, Some(PlayerFilter::Defending));
        assert_eq!(parsed.at_least_count(), Some(1));
        assert_eq!(parsed.filter.card_types, vec![CardType::Land]);
        assert!(
            parsed
                .filter
                .supertypes
                .contains(&crate::types::Supertype::Snow)
        );
    }

    #[test]
    fn player_status_subjects_lower_typed_contextual_references() {
        let tokens = lex_line("that player is the monarch", 0).expect("lex");
        let parsed = parse_player_status_condition(&tokens).expect("that-player status");
        assert_eq!(parsed.player, PlayerFilter::IteratedPlayer);
        assert_eq!(parsed.status, PlayerStatusAst::Monarch);
    }

    #[test]
    fn deferred_player_subject_modes_keep_contextual_lowering_distinct() {
        let that_player = lex_line("that player", 0).expect("lex");
        let that_player = LexedClause::new(&that_player);
        assert_eq!(
            parse_player_has_quantity_subject_clause(that_player),
            Some(PlayerFilter::IteratedPlayer)
        );
        assert_eq!(
            parse_life_relation_player_subject_clause(that_player),
            Some(PlayerFilter::IteratedPlayer)
        );
        assert_eq!(
            parse_spell_cast_this_turn_subject_clause(that_player),
            Some(PlayerFilter::Active)
        );

        let source_contraction = lex_line("you've", 0).expect("lex");
        assert_eq!(
            parse_spell_cast_this_turn_subject_clause(LexedClause::new(&source_contraction)),
            Some(PlayerFilter::You)
        );

        let odd_each = lex_line("each opponents", 0).expect("lex");
        assert_eq!(
            parse_life_relation_player_subject_clause(LexedClause::new(&odd_each)),
            Some(PlayerFilter::Opponent)
        );
    }

    #[test]
    fn parse_player_has_quantity_object_condition_uses_shared_capture_shape() {
        let opponents = lex_line("you have two or more opponents", 0).expect("lex");
        let parsed = parse_player_has_quantity_object_condition(
            &opponents,
            &[&["opponents"]],
            "opponents condition",
        )
        .expect("player has opponents condition should parse");

        assert_eq!(parsed.player, PlayerFilter::You);
        assert_eq!(
            comparison_to_strict_at_least_threshold(&parsed.comparison),
            Some(2)
        );

        let life = lex_line("a player has 13 or less life", 0).expect("lex");
        let parsed =
            parse_player_has_quantity_object_condition(&life, &[&["life"]], "life condition")
                .expect("player has life condition should parse");

        assert_eq!(parsed.player, PlayerFilter::Any);
        assert_eq!(parsed.comparison, Comparison::LessThanOrEqual(13));
    }

    #[test]
    fn typed_zone_change_shapes_preserve_condition_semantics() {
        let death = lex_line("Two creatures died under your control this turn.", 0).expect("lex");
        let death = parse_object_death_this_turn_condition(&death).expect("death condition");
        assert_eq!(death.event, ObjectDeathThisTurnEventAst::Died);
        assert_eq!(death.comparison, Comparison::Equal(2));
        assert_eq!(death.under_controller, Some(PlayerFilter::You));

        let entry = lex_line(
            "Another creature entered the battlefield under your control this turn.",
            0,
        )
        .expect("lex");
        let BattlefieldEntryConditionAst::ObjectEntered {
            filter,
            window,
            min_count: _,
        } = parse_battlefield_entry_condition(&entry).expect("entry condition")
        else {
            panic!("expected object entry condition");
        };
        assert_eq!(window, BattlefieldEntryTurnWindowAst::ThisTurn);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert!(filter.other);

        let left = lex_line("No permanents left the battlefield this turn.", 0).expect("lex");
        assert_eq!(
            parse_battlefield_change_this_turn_condition(&left),
            Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: true })
        );
    }

    #[test]
    fn typed_zone_change_shapes_parse_one_or_more_creatures_died() {
        let death = lex_line("one or more creatures died this turn", 0).expect("lex");
        let death = parse_object_death_this_turn_condition(&death).expect("death condition");
        assert_eq!(death.comparison, Comparison::GreaterThanOrEqual(1));
    }

    #[test]
    fn typed_event_references_and_actions_preserve_condition_semantics() {
        let spell = lex_line("No mana was spent to cast it.", 0).expect("lex");
        assert_eq!(
            parse_spell_context_condition(&spell),
            Some(SpellContextConditionAst::NoManaSpentToCast {
                spell: SpellContextReferenceAst::TargetSpell,
            })
        );

        let action = lex_line("You would begin an extra turn.", 0).expect("lex");
        assert_eq!(
            parse_player_would_action_condition(&action),
            Some(PlayerWouldActionConditionAst {
                player: PlayerFilter::You,
                action: PlayerWouldActionAst::BeginExtraTurn,
            })
        );
    }
}

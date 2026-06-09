use crate::cards::builders::PlayerAst;
use crate::color::ColorSet;
use crate::effect::{Comparison, Value, ValueComparisonOperator};
use crate::static_abilities::AnthemCountExpression;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

use super::super::lex_patterns::{
    LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom, LexPatternMatch,
};
use super::super::lexer::{
    LexedClause, OwnedLexToken, TokenWordView, render_token_slice, token_slice_first_is_any,
};
use super::super::util::{
    comparison_to_at_least_threshold, comparison_to_strict_at_least_threshold,
    comparison_to_strict_at_most_threshold, comparison_to_value_comparison_operator,
    parse_card_type, parse_color, parse_quantity_comparison_prefix,
    parse_quantity_comparison_prefix_words, parse_subtype_flexible, trim_edge_punctuation_tokens,
};
use super::filters::parse_object_filter_with_grammar_entrypoint;
use crate::runtime_backend::object_filters::parse_object_filter_words;

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

const CONTROL_ACTION_PHRASES: &[&[&str]] = &[&["control"], &["controls"]];
const CONTROL_ACTION_WORDS: &[&str] = &["control", "controls"];
const CONTROL_OR_CONTROLLED_ACTION_PHRASES: &[&[&str]] = &[&["control"], &["controlled"]];
const CONTROL_OR_CONTROLLED_ACTION_WORDS: &[&str] = &["control", "controlled"];
const HAS_ACTION_PHRASES: &[&[&str]] = &[&["has"], &["have"]];
const HAS_ACTION_WORDS: &[&str] = &["has", "have"];
const COPULA_ACTION_PHRASES: &[&[&str]] = &[&["is"], &["are"]];
const COPULA_ACTION_WORDS: &[&str] = &["is", "are"];
const CONTROL_DIFFERENT_POWERS_TAILS: &[&[&str]] = &[
    &["with", "different", "powers"],
    &["with", "different", "power"],
];
const CONTROL_NEGATION_PHRASES: &[&[&str]] = &[&["dont"], &["don't"], &["do", "not"]];

fn clause_matches_phrase(clause: LexedClause<'_>, phrase: &[&str]) -> bool {
    LexPattern::new(&[LexPattern::phrase(phrase)]).matches_clause(clause)
}

fn clause_matches_any_phrase(clause: LexedClause<'_>, phrases: &[&[&str]]) -> bool {
    LexPattern::new(&[LexPattern::any_phrase(phrases)]).matches_clause(clause)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ControlConditionOptions {
    pub(crate) allow_that_player: bool,
    pub(crate) allow_opponent_players: bool,
    pub(crate) allow_defending_player: bool,
    pub(crate) bind_filter_controller_to_subject: bool,
    pub(crate) allow_different_powers_tail: bool,
    pub(crate) default_filter_zone: Option<Zone>,
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
pub(crate) struct ControlConditionAst {
    pub(crate) player: PlayerAst,
    pub(crate) player_filter: Option<PlayerFilter>,
    pub(crate) comparison: Comparison,
    pub(crate) quantity_token_count: usize,
    pub(crate) quantity_words: Vec<String>,
    pub(crate) object_words: Vec<String>,
    pub(crate) filter: ObjectFilter,
    pub(crate) requires_different_powers: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OwnershipConditionOptions {
    pub(crate) allow_opponent_players: bool,
    pub(crate) bind_filter_owner_to_subject: bool,
    pub(crate) default_filter_zone: Option<Zone>,
}

impl Default for OwnershipConditionOptions {
    fn default() -> Self {
        Self {
            allow_opponent_players: false,
            bind_filter_owner_to_subject: false,
            default_filter_zone: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OwnershipConditionAst {
    pub(crate) player: PlayerAst,
    pub(crate) player_filter: Option<PlayerFilter>,
    pub(crate) comparison: Comparison,
    pub(crate) quantity_token_count: usize,
    pub(crate) quantity_words: Vec<String>,
    pub(crate) object_words: Vec<String>,
    pub(crate) filter: ObjectFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatusConditionSubjectAst {
    Source,
    EquippedCreature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatusConditionStateAst {
    Equipped,
    Enchanted,
    Tapped,
    Untapped,
    Attacking,
    Monstrous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SubjectStatusConditionAst {
    pub(crate) subject: StatusConditionSubjectAst,
    pub(crate) state: StatusConditionStateAst,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ObjectDescriptorAst {
    Color(ColorSet),
    CardType(CardType),
    Subtype(Subtype),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubjectDescriptorConditionSubjectAst {
    EnchantedPermanent,
    AttachedObject,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SubjectDescriptorConditionAst {
    pub(crate) subject: SubjectDescriptorConditionSubjectAst,
    pub(crate) filter: ObjectFilter,
    pub(crate) descriptor: ObjectDescriptorAst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerStatusAst {
    Monarch,
    Initiative,
    MaxSpeed,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerStatusConditionAst {
    pub(crate) player: PlayerFilter,
    pub(crate) status: PlayerStatusAst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlayerAchievementAst {
    CitysBlessing,
    CompletedDungeon { dungeon_name: Option<String> },
    FullParty,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerAchievementConditionAst {
    pub(crate) player: PlayerFilter,
    pub(crate) achievement: PlayerAchievementAst,
    pub(crate) negated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerCardsInHandConditionAst {
    pub(crate) player: PlayerFilter,
    pub(crate) comparison: Comparison,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerLifeTotalConditionAst {
    pub(crate) player: PlayerFilter,
    pub(crate) comparison: Comparison,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerHasQuantityObjectConditionAst {
    pub(crate) player: PlayerFilter,
    pub(crate) comparison: Comparison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerLifeRelationAst {
    HasMoreLifeThanYou,
    HasLessLifeThanYou,
    HasNoOpponentWithMoreLifeThan,
    HasMoreLifeThanEachOtherPlayer,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerLifeRelationConditionAst {
    pub(crate) player: PlayerFilter,
    pub(crate) relation: PlayerLifeRelationAst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerCardsInHandRelationAst {
    HasMoreCardsInHandThanYou,
    HasMoreCardsInHandThanEachOtherPlayer,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerCardsInHandRelationConditionAst {
    pub(crate) player: PlayerFilter,
    pub(crate) relation: PlayerCardsInHandRelationAst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerTurnEventAst {
    CardsDrawn,
    LandsEnteredBattlefieldUnderControl,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerTurnEventConditionAst {
    pub(crate) player: PlayerFilter,
    pub(crate) event: PlayerTurnEventAst,
    pub(crate) comparison: Comparison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpellContextReferenceAst {
    TargetSpell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpellContextConditionAst {
    ControllerIsPoisoned { spell: SpellContextReferenceAst },
    NoManaSpentToCast { spell: SpellContextReferenceAst },
    YouControlMoreCreaturesThanController { spell: SpellContextReferenceAst },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PlayerSpellCastThisTurnConditionAst {
    MatchingFilters {
        player: PlayerFilter,
        filters: Vec<ObjectFilter>,
        negated: bool,
    },
    CountAtLeast {
        player: PlayerFilter,
        count: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerLifeChangeDirectionAst {
    Gained,
    Lost,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerLifeChangeThisTurnConditionAst {
    pub(crate) player: PlayerFilter,
    pub(crate) direction: PlayerLifeChangeDirectionAst,
    pub(crate) comparison: Comparison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerWouldActionAst {
    DrawCard,
    Proliferate,
    BeginExtraTurn,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlayerWouldActionConditionAst {
    pub(crate) player: PlayerFilter,
    pub(crate) action: PlayerWouldActionAst,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BattlefieldChangeThisTurnConditionAst {
    PermanentLeftBattlefield { negated: bool },
    PermanentLeftBattlefieldUnderYourControl,
    ObjectPutIntoGraveyardFromBattlefield { filter: ObjectFilter },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ObjectDeathThisTurnEventAst {
    Died,
    PutIntoYourGraveyardFromAnywhere,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ObjectDeathThisTurnConditionAst {
    pub(crate) event: ObjectDeathThisTurnEventAst,
    pub(crate) filter: ObjectFilter,
    pub(crate) comparison: Comparison,
    pub(crate) under_controller: Option<PlayerFilter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlefieldEntryTurnWindowAst {
    ThisTurn,
    LastTurn,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BattlefieldEntryConditionAst {
    ObjectEntered {
        filter: ObjectFilter,
        window: BattlefieldEntryTurnWindowAst,
    },
    LandEnteredUnderYourControlThisTurn {
        player: PlayerAst,
    },
}

impl ControlConditionAst {
    pub(crate) fn has_explicit_quantity(&self) -> bool {
        self.quantity_token_count > 0
    }

    pub(crate) fn exact_count(&self) -> Option<u32> {
        match self.comparison {
            Comparison::Equal(count) if count >= 0 => Some(count as u32),
            _ => None,
        }
    }

    pub(crate) fn at_least_count(&self) -> Option<u32> {
        comparison_to_at_least_threshold(&self.comparison)
    }

    pub(crate) fn strict_at_least_count(&self) -> Option<u32> {
        comparison_to_strict_at_least_threshold(&self.comparison)
    }

    pub(crate) fn quantity_text(&self) -> String {
        self.quantity_words.join(" ")
    }

    pub(crate) fn object_text(&self) -> String {
        self.object_words.join(" ")
    }
}

impl SubjectStatusConditionAst {
    pub(crate) fn condition_expr(self) -> Option<crate::ConditionExpr> {
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
    pub(crate) fn condition_expr(self, display: String) -> crate::ConditionExpr {
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
    pub(crate) fn condition_expr(self) -> crate::ConditionExpr {
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
    pub(crate) fn condition_expr(self) -> crate::ConditionExpr {
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
        };
        if self.negated {
            crate::ConditionExpr::Not(Box::new(condition))
        } else {
            condition
        }
    }
}

impl PlayerCardsInHandConditionAst {
    pub(crate) fn condition_expr(self) -> Option<crate::ConditionExpr> {
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

    pub(crate) fn is_no_cards_in_hand(&self) -> bool {
        comparison_to_strict_at_most_threshold(&self.comparison) == Some(0)
    }
}

impl PlayerLifeTotalConditionAst {
    pub(crate) fn condition_expr(self) -> Option<crate::ConditionExpr> {
        let (operator, right) = comparison_to_value_comparison_operator(self.comparison)?;
        Some(crate::ConditionExpr::ValueComparison {
            left: Value::LifeTotal(self.player),
            operator,
            right: Value::Fixed(right),
        })
    }
}

pub(crate) fn parse_control_condition(
    tokens: &[OwnedLexToken],
    options: ControlConditionOptions,
) -> Option<ControlConditionAst> {
    parse_control_condition_shape(tokens, options)
}

pub(crate) fn parse_control_relation_tail_clause(
    tokens: &[OwnedLexToken],
    options: ControlConditionOptions,
) -> Option<LexedClause<'_>> {
    let captured = parse_control_relation_clauses(tokens, options.allow_different_powers_tail)?;
    parse_control_condition_subject_clause(captured.subject_clause, options)?;
    Some(captured.tail_clause)
}

pub(crate) fn parse_control_condition_words(
    words: &[&str],
    options: ControlConditionOptions,
) -> Option<ControlConditionAst> {
    let basic_atoms = [
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilAnyPhrase(CONTROL_ACTION_PHRASES),
        ),
        LexPattern::action("action", LexCaptureKind::OneOf(CONTROL_ACTION_WORDS)),
        LexPattern::tail("amount_and_object", LexCaptureKind::OneOrMoreWords),
    ];
    let modifier_atoms = [
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilAnyPhrase(CONTROL_ACTION_PHRASES),
        ),
        LexPattern::action("action", LexCaptureKind::OneOf(CONTROL_ACTION_WORDS)),
        LexPattern::tail(
            "amount_and_object",
            LexCaptureKind::UntilLastAnyPhrase(CONTROL_DIFFERENT_POWERS_TAILS),
        ),
        LexPattern::modifier("modifier", LexCaptureKind::OneOf(&["with"])),
        LexPattern::phrase(&["different"]),
        LexPattern::any_word(&["powers", "power"]),
    ];
    let matched = if options.allow_different_powers_tail {
        LexPattern::new(&modifier_atoms)
            .match_word_refs(words)
            .or_else(|| LexPattern::new(&basic_atoms).match_word_refs(words))?
    } else {
        LexPattern::new(&basic_atoms).match_word_refs(words)?
    };
    let subject_range = matched
        .capture_by_role(LexCaptureRole::Subject)?
        .word_range
        .clone();
    let tail_range = matched
        .capture_by_role(LexCaptureRole::Tail)?
        .word_range
        .clone();
    let (player, player_filter) =
        parse_control_condition_subject_words(&words[subject_range], options)?;
    finish_control_condition_words(
        player,
        player_filter,
        &words[..tail_range.start],
        &words[tail_range],
        matched.capture_by_role(LexCaptureRole::Modifier).is_some(),
        options,
    )
}

struct PossessionRelationCapture<'a> {
    subject_clause: LexedClause<'a>,
    prefix_tokens: &'a [OwnedLexToken],
    tail_tokens: &'a [OwnedLexToken],
    has_modifier: bool,
}

pub(crate) struct ControlRelationClauses<'a> {
    pub(crate) subject_clause: LexedClause<'a>,
    pub(crate) tail_clause: LexedClause<'a>,
}

pub(crate) struct NegatedControlRelationClauses<'a> {
    pub(crate) subject_clause: LexedClause<'a>,
    pub(crate) negation_clause: LexedClause<'a>,
    pub(crate) tail_clause: LexedClause<'a>,
}

pub(crate) struct HasRelationClauses<'a> {
    pub(crate) subject_clause: LexedClause<'a>,
    pub(crate) tail_clause: LexedClause<'a>,
}

pub(crate) struct CopulaRelationClauses<'a> {
    pub(crate) subject_clause: LexedClause<'a>,
    pub(crate) tail_clause: LexedClause<'a>,
}

pub(crate) struct PrepositionalCopulaRelationClauses<'a> {
    pub(crate) subject_clause: LexedClause<'a>,
    pub(crate) preposition_clause: LexedClause<'a>,
    pub(crate) tail_clause: LexedClause<'a>,
}

pub(crate) fn parse_copula_relation_clauses(
    tokens: &[OwnedLexToken],
) -> Option<CopulaRelationClauses<'_>> {
    let captured =
        match_possession_relation_shape(tokens, COPULA_ACTION_PHRASES, COPULA_ACTION_WORDS, None)?;
    Some(CopulaRelationClauses {
        subject_clause: captured.subject_clause,
        tail_clause: LexedClause::new(captured.tail_tokens).trimmed(),
    })
}

pub(crate) fn parse_prepositional_copula_relation_clauses<'a>(
    tokens: &'a [OwnedLexToken],
    preposition_words: &[&str],
) -> Option<PrepositionalCopulaRelationClauses<'a>> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilAnyPhrase(COPULA_ACTION_PHRASES),
        ),
        LexPattern::action("copula", LexCaptureKind::OneOf(COPULA_ACTION_WORDS)),
        LexPattern::modifier("preposition", LexCaptureKind::OneOf(preposition_words)),
        LexPattern::tail("tail", LexCaptureKind::OneOrMoreWords),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    Some(PrepositionalCopulaRelationClauses {
        subject_clause: matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?,
        preposition_clause: matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?,
        tail_clause: matched
            .capture_clause_by_role(LexCaptureRole::Tail, clause)?
            .trimmed(),
    })
}

pub(crate) fn parse_existential_object_clause(tokens: &[OwnedLexToken]) -> Option<LexedClause<'_>> {
    let clause = LexedClause::new(tokens);
    let optional_copula = [LexPattern::action(
        "copula",
        LexCaptureKind::OneOf(COPULA_ACTION_WORDS),
    )];
    let atoms = [
        LexPattern::subject("existential", LexCaptureKind::OneOf(&["there"])),
        LexPattern::optional(&optional_copula),
        LexPattern::object("object", LexCaptureKind::OneOrMoreWords),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .map(LexedClause::trimmed)
}

pub(crate) fn parse_has_relation_clauses(
    tokens: &[OwnedLexToken],
) -> Option<HasRelationClauses<'_>> {
    let captured =
        match_possession_relation_shape(tokens, HAS_ACTION_PHRASES, HAS_ACTION_WORDS, None)?;
    Some(HasRelationClauses {
        subject_clause: captured.subject_clause,
        tail_clause: LexedClause::new(captured.tail_tokens).trimmed(),
    })
}

pub(crate) fn parse_control_relation_clauses(
    tokens: &[OwnedLexToken],
    allow_different_powers_tail: bool,
) -> Option<ControlRelationClauses<'_>> {
    let captured = match_possession_relation_shape(
        tokens,
        CONTROL_ACTION_PHRASES,
        CONTROL_ACTION_WORDS,
        if allow_different_powers_tail {
            Some(CONTROL_DIFFERENT_POWERS_TAILS)
        } else {
            None
        },
    )?;
    Some(ControlRelationClauses {
        subject_clause: captured.subject_clause,
        tail_clause: LexedClause::new(captured.tail_tokens).trimmed(),
    })
}

pub(crate) fn parse_control_or_controlled_relation_clauses(
    tokens: &[OwnedLexToken],
) -> Option<ControlRelationClauses<'_>> {
    let captured = match_possession_relation_shape(
        tokens,
        CONTROL_OR_CONTROLLED_ACTION_PHRASES,
        CONTROL_OR_CONTROLLED_ACTION_WORDS,
        None,
    )?;
    Some(ControlRelationClauses {
        subject_clause: captured.subject_clause,
        tail_clause: LexedClause::new(captured.tail_tokens).trimmed(),
    })
}

pub(crate) fn parse_negated_control_relation_clauses(
    tokens: &[OwnedLexToken],
) -> Option<NegatedControlRelationClauses<'_>> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilAnyPhrase(CONTROL_NEGATION_PHRASES),
        ),
        LexPattern::modifier(
            "negation",
            LexCaptureKind::OneOfPhrase(CONTROL_NEGATION_PHRASES),
        ),
        LexPattern::action("action", LexCaptureKind::OneOf(CONTROL_ACTION_WORDS)),
        LexPattern::tail("amount_and_object", LexCaptureKind::OneOrMoreWords),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    Some(NegatedControlRelationClauses {
        subject_clause: matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?,
        negation_clause: matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?,
        tail_clause: matched
            .capture_clause_by_role(LexCaptureRole::Tail, clause)?
            .trimmed(),
    })
}

fn match_possession_relation_shape<'a>(
    tokens: &'a [OwnedLexToken],
    action_phrases: &[&[&str]],
    action_words: &[&str],
    modifier_tail_phrases: Option<&[&[&str]]>,
) -> Option<PossessionRelationCapture<'a>> {
    let clause = LexedClause::new(tokens);
    let basic_atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(action_words)),
        LexPattern::tail("amount_and_object", LexCaptureKind::OneOrMoreWords),
    ];
    let build_capture = |matched: LexPatternMatch<'_>| -> Option<PossessionRelationCapture<'a>> {
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        let tail_capture = matched.capture_by_role(LexCaptureRole::Tail)?;
        let tail_range = clause.words().token_range_for_word_range(
            tail_capture.word_range.start,
            tail_capture.word_range.end,
        )?;
        let prefix_range = clause
            .words()
            .token_range_for_word_range(0, tail_capture.word_range.start)?;

        Some(PossessionRelationCapture {
            subject_clause,
            prefix_tokens: tokens.get(prefix_range)?,
            tail_tokens: tokens.get(tail_range)?,
            has_modifier: matched.capture_by_role(LexCaptureRole::Modifier).is_some(),
        })
    };

    if let Some(modifier_tail_phrases) = modifier_tail_phrases {
        let modifier_atoms = [
            LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
            LexPattern::action("action", LexCaptureKind::OneOf(action_words)),
            LexPattern::tail(
                "amount_and_object",
                LexCaptureKind::UntilLastAnyPhrase(modifier_tail_phrases),
            ),
            LexPattern::modifier("modifier", LexCaptureKind::OneOf(&["with"])),
            LexPattern::phrase(&["different"]),
            LexPattern::any_word(&["powers", "power"]),
        ];
        let matched = LexPattern::new(&modifier_atoms)
            .match_clause(clause)
            .or_else(|| LexPattern::new(&basic_atoms).match_clause(clause))?;
        return build_capture(matched);
    } else {
        let matched = LexPattern::new(&basic_atoms).match_clause(clause)?;
        build_capture(matched)
    }
}

fn parse_control_condition_shape(
    tokens: &[OwnedLexToken],
    options: ControlConditionOptions,
) -> Option<ControlConditionAst> {
    let captured = match_possession_relation_shape(
        tokens,
        CONTROL_ACTION_PHRASES,
        CONTROL_ACTION_WORDS,
        if options.allow_different_powers_tail {
            Some(CONTROL_DIFFERENT_POWERS_TAILS)
        } else {
            None
        },
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
    if clause_matches_phrase(clause, &["you"]) {
        return Some((PlayerAst::You, Some(PlayerFilter::You)));
    }
    if options.allow_that_player && clause_matches_phrase(clause, &["that", "player"]) {
        return Some((PlayerAst::That, None));
    }
    if options.allow_opponent_players
        && clause_matches_any_phrase(
            clause,
            &[
                &["opponent"],
                &["opponents"],
                &["an", "opponent"],
                &["your", "opponents"],
            ],
        )
    {
        return Some((PlayerAst::Opponent, Some(PlayerFilter::Opponent)));
    }
    if options.allow_defending_player && clause_matches_phrase(clause, &["defending", "player"]) {
        return Some((PlayerAst::Defending, Some(PlayerFilter::Defending)));
    }
    None
}

fn parse_control_condition_subject_words(
    words: &[&str],
    options: ControlConditionOptions,
) -> Option<(PlayerAst, Option<PlayerFilter>)> {
    if words == ["you"] {
        return Some((PlayerAst::You, Some(PlayerFilter::You)));
    }
    if options.allow_that_player && words == ["that", "player"] {
        return Some((PlayerAst::That, None));
    }
    if options.allow_opponent_players
        && [
            &["opponent"][..],
            &["opponents"][..],
            &["an", "opponent"][..],
            &["your", "opponents"][..],
        ]
        .contains(&words)
    {
        return Some((PlayerAst::Opponent, Some(PlayerFilter::Opponent)));
    }
    if options.allow_defending_player && words == ["defending", "player"] {
        return Some((PlayerAst::Defending, Some(PlayerFilter::Defending)));
    }
    None
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
    let requires_different_powers = captured_requires_different_powers
        || options.allow_different_powers_tail
            && (token_words_end_with(filter_tokens, &["with", "different", "powers"])
                || token_words_end_with(filter_tokens, &["with", "different", "power"]));
    if requires_different_powers {
        filter_tokens = trim_edge_punctuation_tokens(
            filter_tokens.get(..filter_tokens.len().saturating_sub(3))?,
        );
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
    let requires_different_powers = captured_requires_different_powers
        || options.allow_different_powers_tail
            && (filter_words.ends_with(&["with", "different", "powers"])
                || filter_words.ends_with(&["with", "different", "power"]));
    if requires_different_powers {
        filter_words = filter_words.get(..filter_words.len().saturating_sub(3))?;
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

pub(crate) fn parse_ownership_condition(
    tokens: &[OwnedLexToken],
    options: OwnershipConditionOptions,
) -> Option<OwnershipConditionAst> {
    parse_ownership_condition_shape(tokens, options)
}

fn parse_ownership_condition_shape(
    tokens: &[OwnedLexToken],
    options: OwnershipConditionOptions,
) -> Option<OwnershipConditionAst> {
    let action_phrases: &[&[&str]] = &[&["own"], &["owns"]];
    let action_words = &["own", "owns"];
    let captured = match_possession_relation_shape(tokens, action_phrases, action_words, None)?;
    let (player, player_filter) =
        parse_ownership_condition_subject_clause(captured.subject_clause, options)?;

    finish_ownership_condition(player, player_filter, captured.tail_tokens, options)
}

fn parse_ownership_condition_subject_clause(
    clause: LexedClause<'_>,
    options: OwnershipConditionOptions,
) -> Option<(PlayerAst, Option<PlayerFilter>)> {
    if clause_matches_phrase(clause, &["you"]) {
        return Some((PlayerAst::You, Some(PlayerFilter::You)));
    }
    if options.allow_opponent_players
        && clause_matches_any_phrase(
            clause,
            &[
                &["opponent"],
                &["opponents"],
                &["an", "opponent"],
                &["your", "opponents"],
            ],
        )
    {
        return Some((PlayerAst::Opponent, Some(PlayerFilter::Opponent)));
    }
    None
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

pub(crate) fn parse_subject_status_condition(
    tokens: &[OwnedLexToken],
) -> Option<SubjectStatusConditionAst> {
    parse_subject_status_shape(tokens)
}

fn parse_subject_status_shape(tokens: &[OwnedLexToken]) -> Option<SubjectStatusConditionAst> {
    let state_words = &[
        "attacking",
        "enchanted",
        "equipped",
        "monstrous",
        "tapped",
        "untapped",
    ];
    parse_subject_status_shape_with_copula(tokens, state_words)
        .or_else(|| parse_subject_status_shape_without_copula(tokens, state_words))
}

fn parse_subject_status_shape_with_copula(
    tokens: &[OwnedLexToken],
    state_words: &[&str],
) -> Option<SubjectStatusConditionAst> {
    let relation = parse_copula_relation_clauses(tokens)?;
    let state_atoms = [LexPattern::object(
        "state",
        LexCaptureKind::OneOf(state_words),
    )];
    let state_match = LexPattern::new(&state_atoms).match_clause(relation.tail_clause)?;
    let subject = parse_subject_status_subject_clause(relation.subject_clause)?;
    let state_clause =
        state_match.capture_clause_by_role(LexCaptureRole::Object, relation.tail_clause)?;
    let state = parse_subject_status_state_clause(state_clause)?;

    Some(SubjectStatusConditionAst { subject, state })
}

fn parse_subject_status_shape_without_copula(
    tokens: &[OwnedLexToken],
    state_words: &[&str],
) -> Option<SubjectStatusConditionAst> {
    if let Some(parsed) = parse_subject_status_shape_without_copula_rightmost(tokens, state_words) {
        return Some(parsed);
    }
    let clause = LexedClause::new(tokens);
    let state_phrases: &[&[&str]] = &[
        &["attacking"],
        &["enchanted"],
        &["equipped"],
        &["monstrous"],
        &["tapped"],
        &["untapped"],
    ];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(state_phrases)),
        LexPattern::object("state", LexCaptureKind::OneOf(state_words)),
    ];
    parse_subject_status_match(
        tokens,
        LexPattern::new(&atoms).match_clause(clause)?,
        clause,
    )
}

fn parse_subject_status_shape_without_copula_rightmost(
    tokens: &[OwnedLexToken],
    state_words: &[&str],
) -> Option<SubjectStatusConditionAst> {
    let words = TokenWordView::new(tokens);
    let state_word_idx = (1..words.len())
        .rev()
        .find(|idx| state_words.iter().any(|state| words.at_is(*idx, state)))?;
    let subject_range = words.token_range_for_word_range(0, state_word_idx)?;
    let state_range = words.token_range_for_word_range(state_word_idx, state_word_idx + 1)?;
    let subject = parse_subject_status_subject_clause(LexedClause::new(&tokens[subject_range]))?;
    let state = parse_subject_status_state_clause(LexedClause::new(&tokens[state_range]))?;
    Some(SubjectStatusConditionAst { subject, state })
}

fn parse_subject_status_match(
    _tokens: &[OwnedLexToken],
    matched: LexPatternMatch<'_>,
    clause: LexedClause<'_>,
) -> Option<SubjectStatusConditionAst> {
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let subject = parse_subject_status_subject_clause(subject_clause)?;
    let state_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let state = parse_subject_status_state_clause(state_clause)?;

    Some(SubjectStatusConditionAst { subject, state })
}

fn parse_subject_status_subject_clause(
    clause: LexedClause<'_>,
) -> Option<StatusConditionSubjectAst> {
    if clause_matches_any_phrase(
        clause,
        &[
            &["this", "creature"],
            &["this", "permanent"],
            &["this"],
            &["it"],
            &["its"],
        ],
    ) {
        return Some(StatusConditionSubjectAst::Source);
    }
    if clause_matches_phrase(clause, &["equipped", "creature"]) {
        return Some(StatusConditionSubjectAst::EquippedCreature);
    }
    None
}

fn parse_subject_status_state_clause(clause: LexedClause<'_>) -> Option<StatusConditionStateAst> {
    if clause_matches_phrase(clause, &["equipped"]) {
        return Some(StatusConditionStateAst::Equipped);
    }
    if clause_matches_phrase(clause, &["enchanted"]) {
        return Some(StatusConditionStateAst::Enchanted);
    }
    if clause_matches_phrase(clause, &["tapped"]) {
        return Some(StatusConditionStateAst::Tapped);
    }
    if clause_matches_phrase(clause, &["untapped"]) {
        return Some(StatusConditionStateAst::Untapped);
    }
    if clause_matches_phrase(clause, &["attacking"]) {
        return Some(StatusConditionStateAst::Attacking);
    }
    if clause_matches_phrase(clause, &["monstrous"]) {
        return Some(StatusConditionStateAst::Monstrous);
    }
    None
}

pub(crate) fn parse_subject_descriptor_condition(
    tokens: &[OwnedLexToken],
) -> Option<SubjectDescriptorConditionAst> {
    parse_subject_descriptor_shape(tokens)
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
    if clause_matches_phrase(clause, &["enchanted", "permanent"]) {
        return Some(SubjectDescriptorConditionSubjectAst::EnchantedPermanent);
    }
    if clause_matches_any_phrase(
        clause,
        &[
            &["equipped", "creature"],
            &["equipped", "permanent"],
            &["enchanted", "artifact"],
            &["enchanted", "creature"],
            &["enchanted", "land"],
        ],
    ) {
        return Some(SubjectDescriptorConditionSubjectAst::AttachedObject);
    }
    None
}

fn parse_object_descriptor_clause(clause: LexedClause<'_>) -> Option<ObjectDescriptorAst> {
    let mut tokens = clause.trimmed().tokens();
    if token_slice_first_is_any(tokens, &["a", "an", "the"]) {
        tokens = &tokens[1..];
    }
    let [descriptor] = tokens else {
        return None;
    };
    parse_object_descriptor_word(descriptor.as_word()?)
}

pub(crate) fn parse_player_status_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerStatusConditionAst> {
    parse_player_status_shape(tokens)
}

pub(crate) fn parse_player_status_condition_words(
    words: &[&str],
) -> Option<PlayerStatusConditionAst> {
    let shortcut_atoms = [
        LexPattern::subject("subject", LexCaptureKind::OneOf(&["youre"])),
        LexPattern::object("status", LexCaptureKind::Rest),
    ];
    if let Some(matched) = LexPattern::new(&shortcut_atoms).match_word_refs(words) {
        let status_range = matched
            .capture_by_role(LexCaptureRole::Object)?
            .word_range
            .clone();
        let status = parse_player_status_tail_words(&words[status_range])?;
        return Some(PlayerStatusConditionAst {
            player: PlayerFilter::You,
            status,
        });
    }

    let action_words = &["are", "have", "has", "is"];
    let action_phrases: &[&[&str]] = &[&["are"], &["have"], &["has"], &["is"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(action_words)),
        LexPattern::object("status", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_word_refs(words)?;
    let subject_range = matched
        .capture_by_role(LexCaptureRole::Subject)?
        .word_range
        .clone();
    let player = parse_player_status_subject_words(&words[subject_range])?;
    let status_range = matched
        .capture_by_role(LexCaptureRole::Object)?
        .word_range
        .clone();
    let status = parse_player_status_tail_words(&words[status_range])?;

    Some(PlayerStatusConditionAst { player, status })
}

fn parse_player_status_shape(tokens: &[OwnedLexToken]) -> Option<PlayerStatusConditionAst> {
    let clause = LexedClause::new(tokens);
    let shortcut_atoms = [
        LexPattern::subject("subject", LexCaptureKind::OneOf(&["youre"])),
        LexPattern::object("status", LexCaptureKind::Rest),
    ];
    if let Some(matched) = LexPattern::new(&shortcut_atoms).match_clause(clause) {
        let status_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
        let status = parse_player_status_tail_clause(status_clause)?;
        return Some(PlayerStatusConditionAst {
            player: PlayerFilter::You,
            status,
        });
    }

    let action_words = &["are", "have", "has", "is"];
    let action_phrases: &[&[&str]] = &[&["are"], &["have"], &["has"], &["is"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(action_words)),
        LexPattern::object("status", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = parse_player_status_subject_clause(subject_clause)?;
    let status_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let status = parse_player_status_tail_clause(status_clause)?;

    Some(PlayerStatusConditionAst { player, status })
}

fn parse_player_status_tail_clause(clause: LexedClause<'_>) -> Option<PlayerStatusAst> {
    parse_player_status_tail_clause_lexed(clause)
}

pub(crate) fn parse_player_achievement_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerAchievementConditionAst> {
    parse_player_achievement_shape(tokens)
}

fn parse_player_achievement_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerAchievementConditionAst> {
    let clause = LexedClause::new(tokens);
    let action_shapes: &[(&[&str], bool)] = &[
        (&["have", "not"], true),
        (&["havent"], true),
        (&["have"], false),
    ];
    for (action_phrase, negated) in action_shapes {
        let atoms = [
            LexPattern::subject("subject", LexCaptureKind::UntilPhrase(action_phrase)),
            LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
            LexPattern::object("achievement", LexCaptureKind::Rest),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        if !subject_clause_matches_you(subject_clause) {
            continue;
        }
        let achievement_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
        let achievement = parse_player_achievement_tail_clause(achievement_clause)?;
        return Some(PlayerAchievementConditionAst {
            player: PlayerFilter::You,
            achievement,
            negated: *negated,
        });
    }

    let shortcut_atoms = [
        LexPattern::subject("subject", LexCaptureKind::OneOf(&["youve"])),
        LexPattern::object("achievement", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&shortcut_atoms).match_clause(clause)?;
    let achievement_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let achievement = parse_player_achievement_tail_clause(achievement_clause)?;
    Some(PlayerAchievementConditionAst {
        player: PlayerFilter::You,
        achievement,
        negated: false,
    })
}

pub(crate) fn parse_player_cards_in_hand_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerCardsInHandConditionAst> {
    let cards_in_hand_phrases: &[&[&str]] = &[
        &["card", "in", "hand"],
        &["cards", "in", "hand"],
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

pub(crate) fn parse_player_life_total_condition(
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

pub(crate) fn parse_player_has_quantity_object_condition(
    tokens: &[OwnedLexToken],
    object_phrases: &[&[&str]],
    context: &str,
) -> Option<PlayerHasQuantityObjectConditionAst> {
    let atoms = [
        LexPattern::amount("amount", LexCaptureKind::UntilAnyPhrase(object_phrases)),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let relation = parse_has_relation_clauses(tokens)?;
    let player = parse_player_has_quantity_subject_clause(relation.subject_clause)?;
    let matched = LexPattern::new(&atoms).match_clause(relation.tail_clause)?;
    let amount_capture = matched.capture_by_role(LexCaptureRole::Amount)?;
    if amount_capture.word_range.is_empty() {
        return None;
    }
    let amount_range = relation.tail_clause.words().token_range_for_word_range(
        amount_capture.word_range.start,
        amount_capture.word_range.end,
    )?;
    let amount_tokens = relation.tail_clause.tokens().get(amount_range)?;
    let (comparison, used) =
        parse_quantity_comparison_prefix(amount_tokens, false, false, context).ok()?;
    if used != amount_tokens.len() {
        return None;
    }
    let object_clause =
        matched.capture_clause_by_role(LexCaptureRole::Object, relation.tail_clause)?;
    if !object_clause.matches_any_words(object_phrases) {
        return None;
    }

    Some(PlayerHasQuantityObjectConditionAst { player, comparison })
}

fn parse_amount_capture_comparison(
    tokens: &[OwnedLexToken],
    clause: LexedClause<'_>,
    matched: &LexPatternMatch<'_>,
    context: &str,
) -> Option<Comparison> {
    let amount_capture = matched.capture_by_role(LexCaptureRole::Amount)?;
    if amount_capture.word_range.is_empty() {
        return None;
    }
    let amount_range = clause.words().token_range_for_word_range(
        amount_capture.word_range.start,
        amount_capture.word_range.end,
    )?;
    let amount_tokens = tokens.get(amount_range)?;
    let (comparison, used) =
        parse_quantity_comparison_prefix(amount_tokens, false, false, context).ok()?;
    if used == amount_tokens.len() {
        Some(comparison)
    } else {
        None
    }
}

pub(crate) fn parse_player_life_relation_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeRelationConditionAst> {
    parse_player_life_relation_shape(tokens)
}

fn parse_player_life_relation_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeRelationConditionAst> {
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

const MORE_LIFE_THAN_YOU_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["more", "life", "than"]),
    LexPattern::subject(
        "player",
        LexCaptureKind::OneOfPhrase(&[&["you", "do"], &["you"]]),
    ),
]);
const MORE_LIFE_THAN_EACH_OTHER_PLAYER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["more", "life", "than"]),
    LexPattern::subject(
        "player",
        LexCaptureKind::OneOfPhrase(&[
            &["each", "other", "player"],
            &["each", "other", "players"],
        ]),
    ),
]);
const MORE_LIFE_THAN_EACH_OPPONENT_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["more", "life", "than"]),
    LexPattern::subject(
        "player",
        LexCaptureKind::OneOfPhrase(&[&["each", "opponent"], &["each", "opponents"]]),
    ),
]);

fn parse_life_relation_shape(relation_clause: LexedClause<'_>) -> Option<LifeRelationShape> {
    const MORE_LIFE_THAN_PLAYER_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["more", "life", "than"]),
        LexPattern::subject("player", LexCaptureKind::Rest),
    ]);

    let relation_clause = relation_clause.trimmed();
    if MORE_LIFE_THAN_YOU_PATTERN.matches(relation_clause) {
        return Some(LifeRelationShape::MoreThanYou);
    }
    if MORE_LIFE_THAN_EACH_OTHER_PLAYER_PATTERN.matches(relation_clause) {
        return Some(LifeRelationShape::MoreThanEachOtherPlayer);
    }
    if MORE_LIFE_THAN_EACH_OPPONENT_PATTERN.matches(relation_clause) {
        return Some(LifeRelationShape::MoreThanEachOpponent);
    }
    let matched = MORE_LIFE_THAN_PLAYER_PATTERN.match_clause(relation_clause)?;
    let player_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, relation_clause)?;
    let player = parse_life_relation_player_subject_clause(player_clause)?;
    Some(LifeRelationShape::MoreThanPlayer(player))
}

fn parse_no_opponent_more_life_than_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeRelationConditionAst> {
    let clause = LexedClause::new(tokens);
    let tail_phrase = &["more", "life", "than"];
    let atoms = [
        LexPattern::phrase(&["no"]),
        LexPattern::subject("subject", LexCaptureKind::OneOf(&["opponent", "opponents"])),
        LexPattern::word("has"),
        LexPattern::phrase(tail_phrase),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let player = parse_life_relation_player_subject_clause(object_clause)?;
    Some(PlayerLifeRelationConditionAst {
        player,
        relation: PlayerLifeRelationAst::HasNoOpponentWithMoreLifeThan,
    })
}

pub(crate) fn parse_player_cards_in_hand_relation_condition(
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

const MORE_CARDS_IN_HAND_THAN_PREFIXES: &[&[&str]] = &[
    &["more", "card"],
    &["more", "cards"],
    &["more", "card", "in", "hand"],
    &["more", "cards", "in", "hand"],
    &["more", "card", "in", "your", "hand"],
    &["more", "cards", "in", "your", "hand"],
    &["more", "card", "in", "their", "hand"],
    &["more", "cards", "in", "their", "hand"],
];
fn parse_cards_in_hand_relation_shape(
    relation_clause: LexedClause<'_>,
) -> Option<CardsInHandRelationShape> {
    const MORE_CARDS_IN_HAND_THAN_YOU_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(MORE_CARDS_IN_HAND_THAN_PREFIXES),
        LexPattern::word("than"),
        LexPattern::subject(
            "player",
            LexCaptureKind::OneOfPhrase(&[&["you", "do"], &["you"]]),
        ),
    ]);
    const MORE_CARDS_IN_HAND_THAN_EACH_OTHER_PLAYER_PATTERN: LexPattern<'static> =
        LexPattern::new(&[
            LexPattern::any_phrase(MORE_CARDS_IN_HAND_THAN_PREFIXES),
            LexPattern::word("than"),
            LexPattern::subject(
                "player",
                LexCaptureKind::OneOfPhrase(&[
                    &["each", "other", "player"],
                    &["each", "other", "players"],
                ]),
            ),
        ]);

    let relation_clause = relation_clause.trimmed();
    if MORE_CARDS_IN_HAND_THAN_YOU_PATTERN.matches(relation_clause) {
        return Some(CardsInHandRelationShape::MoreThanYou);
    }
    if MORE_CARDS_IN_HAND_THAN_EACH_OTHER_PLAYER_PATTERN.matches(relation_clause) {
        return Some(CardsInHandRelationShape::MoreThanEachOtherPlayer);
    }
    None
}

pub(crate) fn parse_player_turn_event_condition(
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
    let clause = LexedClause::new(tokens);
    let action_shapes: &[&[&str]] = &[&["has", "drawn"], &["have", "drawn"], &["drew"]];
    let card_phrases: &[&[&str]] = &[&["card"], &["cards"]];
    for action_phrase in action_shapes {
        let atoms = [
            LexPattern::subject("subject", LexCaptureKind::UntilPhrase(action_phrase)),
            LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
            LexPattern::amount("amount", LexCaptureKind::UntilAnyPhrase(card_phrases)),
            LexPattern::object("object", LexCaptureKind::OneOf(&["card", "cards"])),
            LexPattern::phrase(&["this", "turn"]),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        let player = parse_life_relation_player_subject_clause(subject_clause)?;
        let comparison =
            parse_amount_capture_comparison(tokens, clause, &matched, "cards-drawn condition")?;
        return Some(PlayerTurnEventConditionAst {
            player,
            event: PlayerTurnEventAst::CardsDrawn,
            comparison,
        });
    }
    None
}

fn parse_lands_entered_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerTurnEventConditionAst> {
    let clause = LexedClause::new(tokens);
    let land_phrases: &[&[&str]] = &[&["land"], &["lands"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(&["had"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["had"])),
        LexPattern::amount("amount", LexCaptureKind::UntilAnyPhrase(land_phrases)),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = parse_life_relation_player_subject_clause(subject_clause)?;
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !parse_lands_entered_this_turn_object_clause(object_clause) {
        return None;
    }
    let comparison =
        parse_amount_capture_comparison(tokens, clause, &matched, "lands-entered condition")?;
    Some(PlayerTurnEventConditionAst {
        player,
        event: PlayerTurnEventAst::LandsEnteredBattlefieldUnderControl,
        comparison,
    })
}

fn parse_lands_entered_this_turn_object_clause(clause: LexedClause<'_>) -> bool {
    const OPTIONAL_THE: &[LexPatternAtom<'static>] = &[LexPattern::word("the")];
    const LANDS_ENTERED_THIS_TURN_OBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object("object", LexCaptureKind::OneOf(&["land", "lands"])),
        LexPattern::any_word(&["enter", "entered"]),
        LexPattern::optional(OPTIONAL_THE),
        LexPattern::phrase(&["battlefield", "under"]),
        LexPattern::any_word(&["your", "their", "that", "its"]),
        LexPattern::phrase(&["control", "this", "turn"]),
    ]);

    LANDS_ENTERED_THIS_TURN_OBJECT_PATTERN.matches_clause(clause)
}

pub(crate) fn parse_spell_context_condition(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextConditionAst> {
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
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("controller", LexCaptureKind::UntilPhrase(&["poisoned"])),
        LexPattern::object("status", LexCaptureKind::OneOf(&["poisoned"])),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let controller_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let spell = parse_target_spell_controller_clause(controller_clause)?;
    Some(SpellContextConditionAst::ControllerIsPoisoned { spell })
}

fn parse_no_mana_spent_to_cast_target_spell_shape(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextConditionAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::phrase(&["no", "mana"]),
        LexPattern::action("spent_action", LexCaptureKind::OneOf(&["was", "were"])),
        LexPattern::phrase(&["spent", "to", "cast"]),
        LexPattern::object("spell", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let spell_clause = matched.capture_clause("spell", clause)?;
    let spell = parse_target_spell_reference_clause(spell_clause)?;
    Some(SpellContextConditionAst::NoManaSpentToCast { spell })
}

fn parse_you_control_more_creatures_than_spell_controller_shape(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextConditionAst> {
    let atoms = [
        LexPattern::phrase(&["more"]),
        LexPattern::object(
            "controlled_object",
            LexCaptureKind::OneOf(&["creature", "creatures"]),
        ),
        LexPattern::word("than"),
        LexPattern::object("controller", LexCaptureKind::Rest),
    ];
    let relation = parse_control_relation_clauses(tokens, false)?;
    if !clause_matches_phrase(relation.subject_clause, &["you"]) {
        return None;
    }
    let matched = LexPattern::new(&atoms).match_clause(relation.tail_clause)?;
    let controller_clause = matched.capture_clause("controller", relation.tail_clause)?;
    let spell = parse_target_spell_controller_clause(controller_clause)?;
    Some(SpellContextConditionAst::YouControlMoreCreaturesThanController { spell })
}

pub(crate) fn parse_player_spell_cast_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerSpellCastThisTurnConditionAst> {
    parse_player_spell_cast_this_turn_shape(tokens)
}

fn parse_player_spell_cast_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerSpellCastThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let action_shapes: &[(&[&str], bool)] = &[
        (&["did", "not", "cast"], true),
        (&["didnt", "cast"], true),
        (&["have", "cast"], false),
        (&["has", "cast"], false),
        (&["cast"], false),
    ];
    for (action_phrase, negated) in action_shapes {
        let atoms = [
            LexPattern::subject("subject", LexCaptureKind::UntilPhrase(action_phrase)),
            LexPattern::action("action", LexCaptureKind::WordCount(action_phrase.len())),
            LexPattern::object("object", LexCaptureKind::UntilPhrase(&["this", "turn"])),
            LexPattern::phrase(&["this", "turn"]),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        let Some(player) = parse_spell_cast_this_turn_subject_clause(subject_clause) else {
            continue;
        };
        let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
        if !*negated && player == PlayerFilter::You && is_another_spell_filter(object_clause) {
            return Some(PlayerSpellCastThisTurnConditionAst::CountAtLeast { player, count: 2 });
        }
        let filters = parse_spell_cast_filter_tokens(object_clause.tokens())?;
        if filters.is_empty() {
            return None;
        }
        return Some(PlayerSpellCastThisTurnConditionAst::MatchingFilters {
            player,
            filters,
            negated: *negated,
        });
    }
    None
}

fn is_another_spell_filter(clause: LexedClause<'_>) -> bool {
    const ANOTHER_SPELL_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::phrase(&["another", "spell"])]);

    ANOTHER_SPELL_PATTERN.matches_clause(clause)
}

pub(crate) fn parse_player_life_change_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeChangeThisTurnConditionAst> {
    parse_player_life_change_this_turn_shape(tokens)
}

fn parse_player_life_change_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let action_words = &["gained", "lost"];
    let action_phrases: &[&[&str]] = &[&["gained"], &["lost"]];
    let object_phrases: &[&[&str]] = &[&["life"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(action_words)),
        LexPattern::amount("amount", LexCaptureKind::UntilAnyPhrase(object_phrases)),
        LexPattern::object("object", LexCaptureKind::OneOf(&["life"])),
        LexPattern::phrase(&["this", "turn"]),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = parse_life_change_subject_clause(subject_clause)?;
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    let direction = parse_life_change_direction_clause(action_clause)?;
    let amount_capture = matched.capture_by_role(LexCaptureRole::Amount)?;
    let comparison = if amount_capture.word_range.is_empty() {
        Comparison::GreaterThanOrEqual(1)
    } else {
        let amount_range = clause.words().token_range_for_word_range(
            amount_capture.word_range.start,
            amount_capture.word_range.end,
        )?;
        let amount_tokens = tokens.get(amount_range)?;
        let (comparison, used) =
            parse_quantity_comparison_prefix(amount_tokens, false, false, "life-change condition")
                .ok()?;
        if used != amount_tokens.len() {
            return None;
        }
        comparison
    };

    Some(PlayerLifeChangeThisTurnConditionAst {
        player,
        direction,
        comparison,
    })
}

pub(crate) fn parse_player_would_action_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerWouldActionConditionAst> {
    parse_player_would_action_shape(tokens)
}

fn parse_player_would_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerWouldActionConditionAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["would"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::word("would"),
        LexPattern::action("action", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = parse_player_would_subject_clause(subject_clause)?;
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    let action = parse_player_would_action_clause(action_clause)?;
    Some(PlayerWouldActionConditionAst { player, action })
}

pub(crate) fn parse_battlefield_change_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    parse_battlefield_change_this_turn_shape(tokens)
}

fn parse_battlefield_change_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    parse_no_permanent_left_battlefield_shape(tokens)
        .or_else(|| parse_permanent_left_battlefield_under_your_control_shape(tokens))
        .or_else(|| parse_object_put_into_graveyard_from_battlefield_shape(tokens))
        .or_else(|| parse_nonland_permanent_or_spell_warped_this_turn_shape(tokens))
        .or_else(|| parse_permanent_left_battlefield_shape(tokens))
}

fn parse_no_permanent_left_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let battlefield_tail: &[&[&str]] = &[
        &["battlefield", "this", "turn"],
        &["the", "battlefield", "this", "turn"],
    ];
    let atoms = [
        LexPattern::word("no"),
        LexPattern::subject(
            "subject",
            LexCaptureKind::OneOf(&["permanent", "permanents"]),
        ),
        LexPattern::action("action", LexCaptureKind::OneOf(&["left"])),
        LexPattern::any_phrase(battlefield_tail),
    ];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: true })
}

fn parse_permanent_left_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let optional_article = [LexPattern::any_word(&["a", "an", "the"])];
    let battlefield_tail: &[&[&str]] = &[
        &["battlefield", "this", "turn"],
        &["the", "battlefield", "this", "turn"],
    ];
    let atoms = [
        LexPattern::optional(&optional_article),
        LexPattern::subject(
            "subject",
            LexCaptureKind::OneOf(&["permanent", "permanents"]),
        ),
        LexPattern::action("action", LexCaptureKind::OneOf(&["left"])),
        LexPattern::any_phrase(battlefield_tail),
    ];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: false })
}

fn parse_permanent_left_battlefield_under_your_control_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let optional_article = [LexPattern::any_word(&["a", "an", "the"])];
    let controlled_battlefield_tail: &[&[&str]] = &[
        &["battlefield", "under", "your", "control", "this", "turn"],
        &[
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ],
    ];
    let battlefield_tail: &[&[&str]] = &[
        &["battlefield", "this", "turn"],
        &["the", "battlefield", "this", "turn"],
    ];
    let controlled_tail = [
        LexPattern::optional(&optional_article),
        LexPattern::subject(
            "subject",
            LexCaptureKind::OneOf(&["permanent", "permanents", "creature", "creatures"]),
        ),
        LexPattern::word("left"),
        LexPattern::any_phrase(controlled_battlefield_tail),
    ];
    let you_controlled_tail = [
        LexPattern::optional(&optional_article),
        LexPattern::subject(
            "subject",
            LexCaptureKind::OneOf(&["permanent", "permanents"]),
        ),
        LexPattern::phrase(&["you", "controlled"]),
        LexPattern::word("left"),
        LexPattern::any_phrase(battlefield_tail),
    ];
    let alternatives: &[&[LexPatternAtom<'_>]] = &[&controlled_tail, &you_controlled_tail];
    let atoms = [LexPattern::any_sequence(alternatives)];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefieldUnderYourControl)
}

fn parse_object_put_into_graveyard_from_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let optional_article = [LexPattern::any_word(&["a", "an", "the"])];
    let graveyard_tail: &[&[&str]] = &[
        &[
            "put",
            "into",
            "graveyard",
            "from",
            "battlefield",
            "this",
            "turn",
        ],
        &[
            "put",
            "into",
            "graveyard",
            "from",
            "the",
            "battlefield",
            "this",
            "turn",
        ],
        &[
            "put",
            "into",
            "a",
            "graveyard",
            "from",
            "battlefield",
            "this",
            "turn",
        ],
        &[
            "put",
            "into",
            "a",
            "graveyard",
            "from",
            "the",
            "battlefield",
            "this",
            "turn",
        ],
    ];
    let atoms = [
        LexPattern::optional(&optional_article),
        LexPattern::object("object", LexCaptureKind::OneOf(&["land", "lands"])),
        LexPattern::phrase(&["you", "controlled"]),
        LexPattern::action("action", LexCaptureKind::OneOf(&["was", "were"])),
        LexPattern::any_phrase(graveyard_tail),
    ];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(
        BattlefieldChangeThisTurnConditionAst::ObjectPutIntoGraveyardFromBattlefield {
            filter: ObjectFilter::land().controlled_by(PlayerFilter::You),
        },
    )
}

fn parse_nonland_permanent_or_spell_warped_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let left_battlefield_phrases: &[&[&str]] = &[
        &[
            "a",
            "nonland",
            "permanent",
            "left",
            "battlefield",
            "this",
            "turn",
        ],
        &[
            "a",
            "nonland",
            "permanent",
            "left",
            "the",
            "battlefield",
            "this",
            "turn",
        ],
        &[
            "nonland",
            "permanent",
            "left",
            "battlefield",
            "this",
            "turn",
        ],
        &[
            "nonland",
            "permanent",
            "left",
            "the",
            "battlefield",
            "this",
            "turn",
        ],
    ];
    let spell_warped_phrases: &[&[&str]] = &[
        &["a", "spell", "was", "warped", "this", "turn"],
        &["spell", "was", "warped", "this", "turn"],
    ];
    let atoms = [
        LexPattern::any_phrase(left_battlefield_phrases),
        LexPattern::word("or"),
        LexPattern::any_phrase(spell_warped_phrases),
    ];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: false })
}

pub(crate) fn parse_object_death_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<ObjectDeathThisTurnConditionAst> {
    parse_object_death_this_turn_shape(tokens)
}

fn parse_object_death_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<ObjectDeathThisTurnConditionAst> {
    parse_object_died_this_turn_shape(tokens)
        .or_else(|| parse_object_put_into_your_graveyard_from_anywhere_shape(tokens))
}

fn parse_object_died_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<ObjectDeathThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let object_phrases: &[&[&str]] = &[&["creature"], &["creatures"]];
    if let Some(matched) = LexPattern::new(&[
        LexPattern::amount("amount", LexCaptureKind::UntilAnyPhrase(object_phrases)),
        LexPattern::object("object", LexCaptureKind::OneOf(&["creature", "creatures"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["died"])),
        LexPattern::phrase(&["under", "your", "control"]),
        LexPattern::phrase(&["this", "turn"]),
    ])
    .match_clause(clause)
    {
        let amount_capture = matched.capture_by_role(LexCaptureRole::Amount)?;
        let comparison = if amount_capture.word_range.is_empty() {
            Comparison::GreaterThanOrEqual(1)
        } else {
            let amount_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
            if clause_matches_phrase(amount_clause, &["a"]) {
                Comparison::GreaterThanOrEqual(1)
            } else {
                parse_amount_capture_comparison(tokens, clause, &matched, "object-death condition")?
            }
        };

        return Some(ObjectDeathThisTurnConditionAst {
            event: ObjectDeathThisTurnEventAst::Died,
            filter: ObjectFilter::creature(),
            comparison,
            under_controller: Some(PlayerFilter::You),
        });
    }

    let atoms = [
        LexPattern::amount("amount", LexCaptureKind::UntilAnyPhrase(object_phrases)),
        LexPattern::object("object", LexCaptureKind::OneOf(&["creature", "creatures"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["died"])),
        LexPattern::phrase(&["this", "turn"]),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let amount_capture = matched.capture_by_role(LexCaptureRole::Amount)?;
    let comparison = if amount_capture.word_range.is_empty() {
        Comparison::GreaterThanOrEqual(1)
    } else {
        let amount_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
        if clause_matches_phrase(amount_clause, &["a"]) {
            Comparison::GreaterThanOrEqual(1)
        } else {
            parse_amount_capture_comparison(tokens, clause, &matched, "object-death condition")?
        }
    };

    Some(ObjectDeathThisTurnConditionAst {
        event: ObjectDeathThisTurnEventAst::Died,
        filter: ObjectFilter::creature(),
        comparison,
        under_controller: None,
    })
}

fn parse_object_put_into_your_graveyard_from_anywhere_shape(
    tokens: &[OwnedLexToken],
) -> Option<ObjectDeathThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let optional_article = [LexPattern::word("a")];
    let atoms = [
        LexPattern::optional(&optional_article),
        LexPattern::phrase(&["creature", "card"]),
        LexPattern::action("action", LexCaptureKind::OneOf(&["was"])),
        LexPattern::phrase(&[
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "anywhere",
            "this",
            "turn",
        ]),
    ];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(ObjectDeathThisTurnConditionAst {
        event: ObjectDeathThisTurnEventAst::PutIntoYourGraveyardFromAnywhere,
        filter: ObjectFilter::creature(),
        comparison: Comparison::GreaterThanOrEqual(1),
        under_controller: None,
    })
}

pub(crate) fn parse_battlefield_entry_condition(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldEntryConditionAst> {
    parse_battlefield_entry_shape(tokens)
}

fn parse_battlefield_entry_shape(tokens: &[OwnedLexToken]) -> Option<BattlefieldEntryConditionAst> {
    parse_you_had_land_entered_battlefield_this_turn_shape(tokens)
        .or_else(|| parse_you_had_object_entered_battlefield_last_turn_shape(tokens))
        .or_else(|| parse_object_entered_battlefield_this_turn_shape(tokens))
}

fn subject_clause_matches_you(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["you"])
}

fn parse_player_status_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    if clause_matches_phrase(clause, &["you"]) {
        return Some(PlayerFilter::You);
    }
    if clause_matches_phrase(clause, &["defending", "player"]) {
        return Some(PlayerFilter::Defending);
    }
    if clause_matches_phrase(clause, &["attacking", "player"]) {
        return Some(PlayerFilter::Attacking);
    }
    if clause_matches_phrase(clause, &["that", "player"]) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    if clause_matches_any_phrase(clause, &[&["an", "opponent"], &["opponent"]]) {
        return Some(PlayerFilter::Opponent);
    }
    if clause_matches_any_phrase(clause, &[&["a", "player"], &["player"]]) {
        return Some(PlayerFilter::Any);
    }
    None
}

fn parse_player_status_subject_words(words: &[&str]) -> Option<PlayerFilter> {
    if words == ["you"] {
        return Some(PlayerFilter::You);
    }
    if words == ["defending", "player"] {
        return Some(PlayerFilter::Defending);
    }
    if words == ["attacking", "player"] {
        return Some(PlayerFilter::Attacking);
    }
    if words == ["that", "player"] {
        return Some(PlayerFilter::IteratedPlayer);
    }
    if [&["an", "opponent"][..], &["opponent"][..]].contains(&words) {
        return Some(PlayerFilter::Opponent);
    }
    if [&["a", "player"][..], &["player"][..]].contains(&words) {
        return Some(PlayerFilter::Any);
    }
    None
}

fn parse_player_status_tail_clause_lexed(clause: LexedClause<'_>) -> Option<PlayerStatusAst> {
    const MONARCH_PHRASES: &[&[&str]] = &[&["monarch"], &["a", "monarch"], &["the", "monarch"]];
    const INITIATIVE_PHRASES: &[&[&str]] = &[
        &["initiative"],
        &["a", "initiative"],
        &["an", "initiative"],
        &["the", "initiative"],
    ];
    const MAX_SPEED_PHRASES: &[&[&str]] = &[
        &["max", "speed"],
        &["maximum", "speed"],
        &["a", "max", "speed"],
        &["a", "maximum", "speed"],
        &["an", "max", "speed"],
        &["an", "maximum", "speed"],
        &["the", "max", "speed"],
        &["the", "maximum", "speed"],
    ];

    if clause_matches_any_phrase(clause, MONARCH_PHRASES) {
        return Some(PlayerStatusAst::Monarch);
    }
    if clause_matches_any_phrase(clause, INITIATIVE_PHRASES) {
        return Some(PlayerStatusAst::Initiative);
    }
    if clause_matches_any_phrase(clause, MAX_SPEED_PHRASES) {
        return Some(PlayerStatusAst::MaxSpeed);
    }
    None
}

fn parse_player_status_tail_words(words: &[&str]) -> Option<PlayerStatusAst> {
    const MONARCH_PHRASES: &[&[&str]] = &[&["monarch"], &["a", "monarch"], &["the", "monarch"]];
    const INITIATIVE_PHRASES: &[&[&str]] = &[
        &["initiative"],
        &["a", "initiative"],
        &["an", "initiative"],
        &["the", "initiative"],
    ];
    const MAX_SPEED_PHRASES: &[&[&str]] = &[
        &["max", "speed"],
        &["maximum", "speed"],
        &["a", "max", "speed"],
        &["a", "maximum", "speed"],
        &["an", "max", "speed"],
        &["an", "maximum", "speed"],
        &["the", "max", "speed"],
        &["the", "maximum", "speed"],
    ];

    if MONARCH_PHRASES.contains(&words) {
        return Some(PlayerStatusAst::Monarch);
    }
    if INITIATIVE_PHRASES.contains(&words) {
        return Some(PlayerStatusAst::Initiative);
    }
    if MAX_SPEED_PHRASES.contains(&words) {
        return Some(PlayerStatusAst::MaxSpeed);
    }
    None
}

fn parse_player_has_quantity_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    if clause_matches_phrase(clause, &["you"]) {
        return Some(PlayerFilter::You);
    }
    if clause_matches_any_phrase(
        clause,
        &[&["a", "opponent"], &["an", "opponent"], &["opponent"]],
    ) {
        return Some(PlayerFilter::Opponent);
    }
    if clause_matches_any_phrase(clause, &[&["a", "player"], &["player"]]) {
        return Some(PlayerFilter::Any);
    }
    if clause_matches_phrase(clause, &["that", "player"]) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    if clause_matches_phrase(clause, &["attacking", "player"]) {
        return Some(PlayerFilter::Attacking);
    }
    if clause_matches_phrase(clause, &["defending", "player"]) {
        return Some(PlayerFilter::Defending);
    }
    None
}

fn parse_life_relation_player_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    if clause_matches_phrase(clause, &["you"]) {
        return Some(PlayerFilter::You);
    }
    if clause_matches_any_phrase(clause, &[&["that", "player"], &["player", "who"]]) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    if clause_matches_phrase(clause, &["target", "player"]) {
        return Some(PlayerFilter::target_player());
    }
    if clause_matches_phrase(clause, &["target", "opponent"]) {
        return Some(PlayerFilter::target_opponent());
    }
    if clause_matches_any_phrase(
        clause,
        &[
            &["each", "opponent"],
            &["each", "opponents"],
            &["a", "opponent"],
            &["an", "opponent"],
            &["opponent"],
            &["opponents"],
        ],
    ) {
        return Some(PlayerFilter::Opponent);
    }
    if clause_matches_any_phrase(clause, &[&["a", "player"], &["any", "player"], &["player"]]) {
        return Some(PlayerFilter::Any);
    }
    if clause_matches_phrase(clause, &["defending", "player"]) {
        return Some(PlayerFilter::Defending);
    }
    if clause_matches_phrase(clause, &["attacking", "player"]) {
        return Some(PlayerFilter::Attacking);
    }
    None
}

fn parse_you_had_land_entered_battlefield_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldEntryConditionAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::WordCount(1)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["had"])),
        LexPattern::object("object", LexCaptureKind::OneOf(&["land", "lands"])),
        LexPattern::any_word(&["enter", "entered"]),
        LexPattern::phrase(&["battlefield", "under", "your", "control", "this", "turn"]),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !subject_clause_matches_you(subject_clause) {
        return None;
    }
    Some(
        BattlefieldEntryConditionAst::LandEnteredUnderYourControlThisTurn {
            player: PlayerAst::You,
        },
    )
}

fn parse_you_had_object_entered_battlefield_last_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldEntryConditionAst> {
    let clause = LexedClause::new(tokens);
    let enter_phrases: &[&[&str]] = &[&["enter"], &["entered"]];
    let optional_the = [LexPattern::word("the")];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::WordCount(1)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["had"])),
        LexPattern::object("object", LexCaptureKind::UntilAnyPhrase(enter_phrases)),
        LexPattern::any_word(&["enter", "entered"]),
        LexPattern::optional(&optional_the),
        LexPattern::phrase(&["battlefield", "under", "your", "control", "last", "turn"]),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !subject_clause_matches_you(subject_clause) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let mut filter =
        parse_object_filter_with_grammar_entrypoint(object_clause.tokens(), false).ok()?;
    filter.controller = Some(PlayerFilter::You);
    if token_slice_first_is_any(object_clause.trimmed().tokens(), &["another", "other"]) {
        filter.other = true;
    }
    Some(BattlefieldEntryConditionAst::ObjectEntered {
        filter,
        window: BattlefieldEntryTurnWindowAst::LastTurn,
    })
}

fn parse_object_entered_battlefield_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldEntryConditionAst> {
    let clause = LexedClause::new(tokens);
    let enter_phrases: &[&[&str]] = &[&["enter"], &["entered"]];
    let optional_the = [LexPattern::word("the")];
    let atoms = [
        LexPattern::object("object", LexCaptureKind::UntilAnyPhrase(enter_phrases)),
        LexPattern::any_word(&["enter", "entered"]),
        LexPattern::optional(&optional_the),
        LexPattern::phrase(&["battlefield", "under", "your", "control", "this", "turn"]),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let mut filter =
        parse_object_filter_with_grammar_entrypoint(object_clause.tokens(), false).ok()?;
    filter.controller = Some(PlayerFilter::You);
    if token_slice_first_is_any(object_clause.trimmed().tokens(), &["another", "other"]) {
        filter.other = true;
    }
    Some(BattlefieldEntryConditionAst::ObjectEntered {
        filter,
        window: BattlefieldEntryTurnWindowAst::ThisTurn,
    })
}

fn parse_target_spell_controller_clause(
    clause: LexedClause<'_>,
) -> Option<SpellContextReferenceAst> {
    if clause_matches_any_phrase(
        clause,
        &[
            &["its", "controller"],
            &["that", "spells", "controller"],
            &["that", "spell", "controller"],
        ],
    ) {
        return Some(SpellContextReferenceAst::TargetSpell);
    }
    None
}

fn parse_target_spell_reference_clause(
    clause: LexedClause<'_>,
) -> Option<SpellContextReferenceAst> {
    if clause_matches_any_phrase(clause, &[&["it"], &["that", "spell"]]) {
        return Some(SpellContextReferenceAst::TargetSpell);
    }
    None
}

fn parse_spell_cast_this_turn_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    if clause_matches_phrase(clause, &["that", "player"]) {
        return Some(PlayerFilter::Active);
    }
    if clause_matches_any_phrase(clause, &[&["you"], &["youve"]]) {
        return Some(PlayerFilter::You);
    }
    if clause_matches_any_phrase(
        clause,
        &[&["opponent"], &["opponents"], &["an", "opponent"]],
    ) {
        return Some(PlayerFilter::Opponent);
    }
    None
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

fn split_both_spell_cast_filter_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    parse_both_spell_cast_filter_pair_tokens(tokens)
        .or_else(|| parse_named_spell_cast_filter_pair_tokens(tokens))
}

fn parse_both_spell_cast_filter_pair_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    const BOTH_SPELL_CAST_FILTER_PAIR_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("both"),
        LexPattern::object("left", LexCaptureKind::UntilPhrase(&["and"])),
        LexPattern::word("and"),
        LexPattern::object("right", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    spell_cast_filter_pair_captures(BOTH_SPELL_CAST_FILTER_PAIR_PATTERN, clause)
}

fn parse_named_spell_cast_filter_pair_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    const NAMED_SPELL_CAST_FILTER_PAIR_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object("left", LexCaptureKind::UntilPhrase(&["and"])),
        LexPattern::word("and"),
        LexPattern::object("right", LexCaptureKind::OneOrMoreWords),
    ]);

    let clause = LexedClause::new(tokens);
    let (left, right) =
        spell_cast_filter_pair_captures(NAMED_SPELL_CAST_FILTER_PAIR_PATTERN, clause)?;
    if !spell_named_prefix_matches_tokens(left) || !spell_named_prefix_matches_tokens(right) {
        return None;
    }
    Some((left, right))
}

fn spell_cast_filter_pair_captures<'a>(
    pattern: LexPattern<'static>,
    clause: LexedClause<'a>,
) -> Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    let matched = pattern.match_clause(clause)?;
    let left = matched.capture_clause("left", clause)?;
    let right = matched.capture_clause("right", clause)?;
    let left_tokens = left.tokens();
    let right_tokens = right.tokens();
    (!left_tokens.is_empty() && !right_tokens.is_empty()).then_some((left_tokens, right_tokens))
}

fn spell_named_prefix_matches_tokens(tokens: &[OwnedLexToken]) -> bool {
    const A_SPELL_NAMED_PREFIX_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::phrase(&["a", "spell", "named"])]);
    const SPELL_NAMED_PREFIX_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::phrase(&["spell", "named"])]);

    let clause = LexedClause::new(tokens);
    A_SPELL_NAMED_PREFIX_PATTERN.matches_prefix(clause)
        || SPELL_NAMED_PREFIX_PATTERN.matches_prefix(clause)
}

fn parse_spell_cast_filter_tokens_single(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    parse_object_filter_with_grammar_entrypoint(tokens, false).ok()
}

fn parse_life_change_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    if clause_matches_phrase(clause, &["you"]) {
        return Some(PlayerFilter::You);
    }
    if clause_matches_any_phrase(
        clause,
        &[
            &["opponent"],
            &["opponents"],
            &["an", "opponent"],
            &["one", "or", "more", "opponents"],
        ],
    ) {
        return Some(PlayerFilter::Opponent);
    }
    None
}

fn parse_life_change_direction_clause(
    clause: LexedClause<'_>,
) -> Option<PlayerLifeChangeDirectionAst> {
    if clause_matches_phrase(clause, &["gained"]) {
        return Some(PlayerLifeChangeDirectionAst::Gained);
    }
    if clause_matches_phrase(clause, &["lost"]) {
        return Some(PlayerLifeChangeDirectionAst::Lost);
    }
    None
}

fn parse_player_would_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    if clause_matches_phrase(clause, &["you"]) {
        return Some(PlayerFilter::You);
    }
    if clause_matches_any_phrase(
        clause,
        &[&["an", "opponent"], &["opponent"], &["opponents"]],
    ) {
        return Some(PlayerFilter::Opponent);
    }
    None
}

fn parse_player_would_action_clause(clause: LexedClause<'_>) -> Option<PlayerWouldActionAst> {
    if clause_matches_any_phrase(clause, &[&["draw", "a", "card"], &["draw", "card"]]) {
        return Some(PlayerWouldActionAst::DrawCard);
    }
    if clause_matches_phrase(clause, &["proliferate"]) {
        return Some(PlayerWouldActionAst::Proliferate);
    }
    if clause_matches_any_phrase(
        clause,
        &[
            &["begin", "extra", "turn"],
            &["begin", "an", "extra", "turn"],
        ],
    ) {
        return Some(PlayerWouldActionAst::BeginExtraTurn);
    }
    None
}

fn parse_player_achievement_tail_clause(clause: LexedClause<'_>) -> Option<PlayerAchievementAst> {
    const CITYS_BLESSING_PHRASES: &[&[&str]] = &[
        &["citys", "blessing"],
        &["city", "blessing"],
        &["a", "citys", "blessing"],
        &["a", "city", "blessing"],
        &["the", "citys", "blessing"],
        &["the", "city", "blessing"],
    ];
    const FULL_PARTY_PHRASES: &[&[&str]] = &[
        &["full", "party"],
        &["a", "full", "party"],
        &["the", "full", "party"],
    ];
    const COMPLETED_DUNGEON_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::optional(&[LexPattern::any_word(&["a", "an", "the"])]),
        LexPattern::word("completed"),
        LexPattern::object("dungeon", LexCaptureKind::Rest),
    ]);
    const CITYS_BLESSING_FOR_EACH_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::optional(&[LexPattern::any_word(&["a", "an", "the"])]),
        LexPattern::any_phrase(&[&["citys", "blessing"], &["city", "blessing"]]),
        LexPattern::phrase(&["for", "each"]),
        LexPattern::tail("tail", LexCaptureKind::Rest),
    ]);

    if clause_matches_any_phrase(clause, CITYS_BLESSING_PHRASES)
        || CITYS_BLESSING_FOR_EACH_PATTERN.matches_clause(clause)
    {
        return Some(PlayerAchievementAst::CitysBlessing);
    }
    if clause_matches_any_phrase(clause, FULL_PARTY_PHRASES) {
        return Some(PlayerAchievementAst::FullParty);
    }
    let matched = COMPLETED_DUNGEON_PATTERN.match_clause(clause)?;
    let dungeon_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    parse_completed_dungeon_achievement_clause(dungeon_clause)
}

fn parse_completed_dungeon_achievement_clause(
    clause: LexedClause<'_>,
) -> Option<PlayerAchievementAst> {
    const DUNGEON_PHRASES: &[&[&str]] = &[
        &["dungeon"],
        &["a", "dungeon"],
        &["an", "dungeon"],
        &["the", "dungeon"],
    ];
    const NAMED_DUNGEON_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::optional(&[LexPattern::any_word(&["a", "an", "the"])]),
        LexPattern::object("dungeon_name", LexCaptureKind::Rest),
    ]);

    if clause_matches_any_phrase(clause, DUNGEON_PHRASES) {
        return Some(PlayerAchievementAst::CompletedDungeon { dungeon_name: None });
    }
    let matched = NAMED_DUNGEON_PATTERN.match_clause(clause)?;
    let dungeon_name_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let dungeon_name_tokens = dungeon_name_clause.trimmed().tokens();
    if dungeon_name_tokens.is_empty() {
        return None;
    }
    Some(PlayerAchievementAst::CompletedDungeon {
        dungeon_name: Some(render_token_slice(dungeon_name_tokens).trim().to_string()),
    })
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

fn token_words_end_with(tokens: &[OwnedLexToken], suffix: &[&str]) -> bool {
    if suffix.len() > tokens.len() {
        return false;
    }
    let start = tokens.len() - suffix.len();
    tokens[start..]
        .iter()
        .zip(suffix.iter())
        .all(|(token, expected)| {
            token
                .as_word()
                .is_some_and(|_| token.parser_text() == *expected)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

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
}

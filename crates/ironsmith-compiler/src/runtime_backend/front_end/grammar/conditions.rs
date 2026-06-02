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
use super::super::lexer::{LexedClause, OwnedLexToken, TokenWordView};
use super::super::util::{
    comparison_to_at_least_threshold, comparison_to_strict_at_least_threshold,
    comparison_to_strict_at_most_threshold, comparison_to_value_comparison_operator,
    parse_card_type, parse_color, parse_quantity_comparison_prefix, parse_subtype_flexible,
    trim_edge_punctuation_tokens,
};
use super::filters::parse_object_filter_with_grammar_entrypoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ControlConditionOptions {
    pub(crate) allow_that_player: bool,
    pub(crate) allow_opponent_players: bool,
    pub(crate) bind_filter_controller_to_subject: bool,
    pub(crate) allow_different_powers_tail: bool,
    pub(crate) default_filter_zone: Option<Zone>,
}

impl Default for ControlConditionOptions {
    fn default() -> Self {
        Self {
            allow_that_player: true,
            allow_opponent_players: false,
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
    pub(crate) fn at_least_count(&self) -> Option<u32> {
        comparison_to_at_least_threshold(&self.comparison)
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
    if let Some(condition) = parse_control_condition_shape(tokens, options) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (player, player_filter, prefix_len) = match word_refs.as_slice() {
        ["you", control_word, ..] if matches!(*control_word, "control" | "controls") => {
            (PlayerAst::You, Some(PlayerFilter::You), 2usize)
        }
        ["that", "player", control_word, ..]
            if options.allow_that_player && matches!(*control_word, "control" | "controls") =>
        {
            (PlayerAst::That, None, 3usize)
        }
        ["opponent" | "opponents", control_word, ..]
            if options.allow_opponent_players
                && matches!(*control_word, "control" | "controls") =>
        {
            (PlayerAst::Opponent, Some(PlayerFilter::Opponent), 2usize)
        }
        ["an", "opponent", control_word, ..] | ["your", "opponents", control_word, ..]
            if options.allow_opponent_players
                && matches!(*control_word, "control" | "controls") =>
        {
            (PlayerAst::Opponent, Some(PlayerFilter::Opponent), 3usize)
        }
        _ => return None,
    };

    finish_control_condition(
        player,
        player_filter,
        tokens.get(..prefix_len)?,
        tokens.get(prefix_len..)?,
        false,
        options,
    )
}

fn parse_control_condition_shape(
    tokens: &[OwnedLexToken],
    options: ControlConditionOptions,
) -> Option<ControlConditionAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["control"], &["controls"]];
    let action_words = &["control", "controls"];
    let different_powers_tails: &[&[&str]] = &[
        &["with", "different", "powers"],
        &["with", "different", "power"],
    ];
    let modifier_atoms = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::UntilAnyPhrase(action_phrases),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(action_words),
        ),
        LexPattern::role_capture(
            "amount_and_object",
            LexCaptureRole::Tail,
            LexCaptureKind::UntilLastAnyPhrase(different_powers_tails),
        ),
        LexPattern::role_capture(
            "modifier",
            LexCaptureRole::Modifier,
            LexCaptureKind::OneOf(&["with"]),
        ),
        LexPattern::phrase(&["different"]),
        LexPattern::any_word(&["powers", "power"]),
    ];
    let basic_atoms = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::UntilAnyPhrase(action_phrases),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(action_words),
        ),
        LexPattern::role_capture(
            "amount_and_object",
            LexCaptureRole::Tail,
            LexCaptureKind::OneOrMoreWords,
        ),
    ];
    let matched = if options.allow_different_powers_tail {
        LexPattern::new(&modifier_atoms)
            .match_clause(clause)
            .or_else(|| LexPattern::new(&basic_atoms).match_clause(clause))?
    } else {
        LexPattern::new(&basic_atoms).match_clause(clause)?
    };

    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let subject_words = subject_clause.word_refs();
    let (player, player_filter) = parse_control_condition_subject(&subject_words, options)?;
    let tail_capture = matched.capture_by_role(LexCaptureRole::Tail)?;
    let tail_range = clause
        .words()
        .token_range_for_word_range(tail_capture.word_range.start, tail_capture.word_range.end)?;
    let prefix_range = clause
        .words()
        .token_range_for_word_range(0, tail_capture.word_range.start)?;

    finish_control_condition(
        player,
        player_filter,
        tokens.get(prefix_range)?,
        tokens.get(tail_range)?,
        matched.capture_by_role(LexCaptureRole::Modifier).is_some(),
        options,
    )
}

fn parse_control_condition_subject(
    words: &[&str],
    options: ControlConditionOptions,
) -> Option<(PlayerAst, Option<PlayerFilter>)> {
    match words {
        ["you"] => Some((PlayerAst::You, Some(PlayerFilter::You))),
        ["that", "player"] if options.allow_that_player => Some((PlayerAst::That, None)),
        ["opponent" | "opponents"] if options.allow_opponent_players => {
            Some((PlayerAst::Opponent, Some(PlayerFilter::Opponent)))
        }
        ["an", "opponent"] | ["your", "opponents"] if options.allow_opponent_players => {
            Some((PlayerAst::Opponent, Some(PlayerFilter::Opponent)))
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
    if options.bind_filter_controller_to_subject && filter.controller.is_none() {
        filter.controller = player_filter.clone();
    }

    Some(ControlConditionAst {
        player,
        player_filter,
        comparison,
        quantity_token_count: quantity_len,
        filter,
        requires_different_powers,
    })
}

pub(crate) fn parse_ownership_condition(
    tokens: &[OwnedLexToken],
    options: OwnershipConditionOptions,
) -> Option<OwnershipConditionAst> {
    if let Some(condition) = parse_ownership_condition_shape(tokens, options) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (player, player_filter, prefix_len) = match word_refs.as_slice() {
        ["you", own_word, ..] if matches!(*own_word, "own" | "owns") => {
            (PlayerAst::You, Some(PlayerFilter::You), 2usize)
        }
        ["opponent" | "opponents", own_word, ..]
            if options.allow_opponent_players && matches!(*own_word, "own" | "owns") =>
        {
            (PlayerAst::Opponent, Some(PlayerFilter::Opponent), 2usize)
        }
        ["an", "opponent", own_word, ..] | ["your", "opponents", own_word, ..]
            if options.allow_opponent_players && matches!(*own_word, "own" | "owns") =>
        {
            (PlayerAst::Opponent, Some(PlayerFilter::Opponent), 3usize)
        }
        _ => return None,
    };

    finish_ownership_condition(player, player_filter, tokens.get(prefix_len..)?, options)
}

fn parse_ownership_condition_shape(
    tokens: &[OwnedLexToken],
    options: OwnershipConditionOptions,
) -> Option<OwnershipConditionAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["own"], &["owns"]];
    let action_words = &["own", "owns"];
    let atoms = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::UntilAnyPhrase(action_phrases),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(action_words),
        ),
        LexPattern::role_capture(
            "amount_and_object",
            LexCaptureRole::Tail,
            LexCaptureKind::OneOrMoreWords,
        ),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let subject_words = subject_clause.word_refs();
    let (player, player_filter) = parse_ownership_condition_subject(&subject_words, options)?;
    let tail_capture = matched.capture_by_role(LexCaptureRole::Tail)?;
    let tail_range = clause
        .words()
        .token_range_for_word_range(tail_capture.word_range.start, tail_capture.word_range.end)?;

    finish_ownership_condition(player, player_filter, tokens.get(tail_range)?, options)
}

fn parse_ownership_condition_subject(
    words: &[&str],
    options: OwnershipConditionOptions,
) -> Option<(PlayerAst, Option<PlayerFilter>)> {
    match words {
        ["you"] => Some((PlayerAst::You, Some(PlayerFilter::You))),
        ["opponent" | "opponents"] if options.allow_opponent_players => {
            Some((PlayerAst::Opponent, Some(PlayerFilter::Opponent)))
        }
        ["an", "opponent"] | ["your", "opponents"] if options.allow_opponent_players => {
            Some((PlayerAst::Opponent, Some(PlayerFilter::Opponent)))
        }
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
    let filter_tokens = trim_edge_punctuation_tokens(tail_tokens.get(quantity_len..)?);
    if filter_tokens.is_empty() {
        return None;
    }

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
        filter,
    })
}

pub(crate) fn parse_subject_status_condition(
    tokens: &[OwnedLexToken],
) -> Option<SubjectStatusConditionAst> {
    if let Some(condition) = parse_subject_status_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (subject, rest) = match word_refs.as_slice() {
        ["this", "creature", rest @ ..] | ["this", "permanent", rest @ ..] => {
            (StatusConditionSubjectAst::Source, rest)
        }
        ["this", rest @ ..] | ["it", rest @ ..] | ["its", rest @ ..] => {
            (StatusConditionSubjectAst::Source, rest)
        }
        ["equipped", "creature", rest @ ..] => (StatusConditionSubjectAst::EquippedCreature, rest),
        _ => return None,
    };
    let rest = match rest {
        ["is", rest @ ..] | ["are", rest @ ..] => rest,
        rest => rest,
    };
    let state = match rest {
        ["equipped"] => StatusConditionStateAst::Equipped,
        ["enchanted"] => StatusConditionStateAst::Enchanted,
        ["tapped"] => StatusConditionStateAst::Tapped,
        ["untapped"] => StatusConditionStateAst::Untapped,
        ["attacking"] => StatusConditionStateAst::Attacking,
        ["monstrous"] => StatusConditionStateAst::Monstrous,
        _ => return None,
    };

    Some(SubjectStatusConditionAst { subject, state })
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
    let clause = LexedClause::new(tokens);
    let copula_phrases: &[&[&str]] = &[&["is"], &["are"]];
    let atoms = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::UntilAnyPhrase(copula_phrases),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(&["is", "are"]),
        ),
        LexPattern::role_capture(
            "state",
            LexCaptureRole::Object,
            LexCaptureKind::OneOf(state_words),
        ),
    ];
    parse_subject_status_match(
        tokens,
        LexPattern::new(&atoms).match_clause(clause)?,
        clause,
    )
}

fn parse_subject_status_shape_without_copula(
    tokens: &[OwnedLexToken],
    state_words: &[&str],
) -> Option<SubjectStatusConditionAst> {
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
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::UntilAnyPhrase(state_phrases),
        ),
        LexPattern::role_capture(
            "state",
            LexCaptureRole::Object,
            LexCaptureKind::OneOf(state_words),
        ),
    ];
    parse_subject_status_match(
        tokens,
        LexPattern::new(&atoms).match_clause(clause)?,
        clause,
    )
}

fn parse_subject_status_match(
    _tokens: &[OwnedLexToken],
    matched: LexPatternMatch<'_>,
    clause: LexedClause<'_>,
) -> Option<SubjectStatusConditionAst> {
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let subject = parse_subject_status_subject_words(&subject_clause.word_refs())?;
    let state_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let state = match state_clause.word_refs().as_slice() {
        ["equipped"] => StatusConditionStateAst::Equipped,
        ["enchanted"] => StatusConditionStateAst::Enchanted,
        ["tapped"] => StatusConditionStateAst::Tapped,
        ["untapped"] => StatusConditionStateAst::Untapped,
        ["attacking"] => StatusConditionStateAst::Attacking,
        ["monstrous"] => StatusConditionStateAst::Monstrous,
        _ => return None,
    };

    Some(SubjectStatusConditionAst { subject, state })
}

fn parse_subject_status_subject_words(words: &[&str]) -> Option<StatusConditionSubjectAst> {
    match words {
        ["this", "creature"] | ["this", "permanent"] | ["this"] | ["it"] | ["its"] => {
            Some(StatusConditionSubjectAst::Source)
        }
        ["equipped", "creature"] => Some(StatusConditionSubjectAst::EquippedCreature),
        _ => None,
    }
}

pub(crate) fn parse_subject_descriptor_condition(
    tokens: &[OwnedLexToken],
) -> Option<SubjectDescriptorConditionAst> {
    if let Some(condition) = parse_subject_descriptor_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let be_idx = word_refs
        .iter()
        .position(|word| matches!(*word, "is" | "are"))?;
    let subject_words = word_refs.get(..be_idx)?;
    let subject = match subject_words {
        ["enchanted", "permanent"] => SubjectDescriptorConditionSubjectAst::EnchantedPermanent,
        ["equipped", "creature" | "permanent"]
        | ["enchanted", "artifact" | "creature" | "land"] => {
            SubjectDescriptorConditionSubjectAst::AttachedObject
        }
        _ => return None,
    };

    let descriptor_words = strip_optional_article(word_refs.get(be_idx + 1..)?);
    let [descriptor_word] = descriptor_words else {
        return None;
    };
    let descriptor = parse_object_descriptor_word(descriptor_word)?;

    let subject_end = words.token_index_for_word_index(be_idx)?;
    let filter = parse_object_filter_with_grammar_entrypoint(&tokens[..subject_end], false).ok()?;

    Some(SubjectDescriptorConditionAst {
        subject,
        filter,
        descriptor,
    })
}

fn parse_subject_descriptor_shape(
    tokens: &[OwnedLexToken],
) -> Option<SubjectDescriptorConditionAst> {
    let clause = LexedClause::new(tokens);
    let copula_phrases: &[&[&str]] = &[&["is"], &["are"]];
    let atoms = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::UntilAnyPhrase(copula_phrases),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(&["is", "are"]),
        ),
        LexPattern::role_capture("descriptor", LexCaptureRole::Object, LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let subject = parse_subject_descriptor_subject_words(&subject_clause.word_refs())?;
    let descriptor_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let descriptor_word_refs = descriptor_clause.word_refs();
    let descriptor_words = strip_optional_article(&descriptor_word_refs);
    let [descriptor_word] = descriptor_words else {
        return None;
    };
    let descriptor = parse_object_descriptor_word(descriptor_word)?;
    let filter =
        parse_object_filter_with_grammar_entrypoint(subject_clause.tokens(), false).ok()?;

    Some(SubjectDescriptorConditionAst {
        subject,
        filter,
        descriptor,
    })
}

fn parse_subject_descriptor_subject_words(
    words: &[&str],
) -> Option<SubjectDescriptorConditionSubjectAst> {
    match words {
        ["enchanted", "permanent"] => {
            Some(SubjectDescriptorConditionSubjectAst::EnchantedPermanent)
        }
        ["equipped", "creature" | "permanent"]
        | ["enchanted", "artifact" | "creature" | "land"] => {
            Some(SubjectDescriptorConditionSubjectAst::AttachedObject)
        }
        _ => None,
    }
}

pub(crate) fn parse_player_status_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerStatusConditionAst> {
    if let Some(condition) = parse_player_status_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (player, rest) = parse_player_status_subject_and_tail(&word_refs)?;
    let status = parse_player_status_tail(rest)?;

    Some(PlayerStatusConditionAst { player, status })
}

fn parse_player_status_shape(tokens: &[OwnedLexToken]) -> Option<PlayerStatusConditionAst> {
    let clause = LexedClause::new(tokens);
    let shortcut_atoms = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::OneOf(&["youre"]),
        ),
        LexPattern::role_capture("status", LexCaptureRole::Object, LexCaptureKind::Rest),
    ];
    if let Some(matched) = LexPattern::new(&shortcut_atoms).match_clause(clause) {
        let status_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
        let status = parse_player_status_tail(&status_clause.word_refs())?;
        return Some(PlayerStatusConditionAst {
            player: PlayerFilter::You,
            status,
        });
    }

    let action_words = &["are", "have", "has", "is"];
    let action_phrases: &[&[&str]] = &[&["are"], &["have"], &["has"], &["is"]];
    let atoms = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::UntilAnyPhrase(action_phrases),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(action_words),
        ),
        LexPattern::role_capture("status", LexCaptureRole::Object, LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = parse_player_status_subject_words(&subject_clause.word_refs())?;
    let status_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let status = parse_player_status_tail(&status_clause.word_refs())?;

    Some(PlayerStatusConditionAst { player, status })
}

pub(crate) fn parse_player_achievement_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerAchievementConditionAst> {
    if let Some(condition) = parse_player_achievement_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (player, negated, rest) = match word_refs.as_slice() {
        ["you", "havent", rest @ ..] | ["you", "have", "not", rest @ ..] => {
            (PlayerFilter::You, true, rest)
        }
        ["you", "have", rest @ ..] | ["youve", rest @ ..] => (PlayerFilter::You, false, rest),
        _ => return None,
    };

    let achievement = parse_player_achievement_tail(rest)?;

    Some(PlayerAchievementConditionAst {
        player,
        achievement,
        negated,
    })
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
            LexPattern::role_capture(
                "subject",
                LexCaptureRole::Subject,
                LexCaptureKind::UntilPhrase(action_phrase),
            ),
            LexPattern::role_capture(
                "action",
                LexCaptureRole::Action,
                LexCaptureKind::WordCount(action_phrase.len()),
            ),
            LexPattern::role_capture("achievement", LexCaptureRole::Object, LexCaptureKind::Rest),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        if subject_clause.word_refs().as_slice() != ["you"] {
            continue;
        }
        let achievement_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
        let achievement = parse_player_achievement_tail(&achievement_clause.word_refs())?;
        return Some(PlayerAchievementConditionAst {
            player: PlayerFilter::You,
            achievement,
            negated: *negated,
        });
    }

    let shortcut_atoms = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::OneOf(&["youve"]),
        ),
        LexPattern::role_capture("achievement", LexCaptureRole::Object, LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&shortcut_atoms).match_clause(clause)?;
    let achievement_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let achievement = parse_player_achievement_tail(&achievement_clause.word_refs())?;
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
    if let Some((player, comparison)) = parse_player_has_quantity_object_shape(
        tokens,
        cards_in_hand_phrases,
        "cards-in-hand condition",
    ) {
        return Some(PlayerCardsInHandConditionAst { player, comparison });
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (player, quantity_word_idx) = parse_player_has_quantity_subject(&word_refs)?;
    let quantity_token_idx = words.token_index_for_word_index(quantity_word_idx)?;
    let quantity_tokens = tokens.get(quantity_token_idx..)?;
    let (comparison, used) =
        parse_quantity_comparison_prefix(quantity_tokens, false, false, "cards-in-hand condition")
            .ok()?;
    let tail_words = TokenWordView::new(quantity_tokens.get(used..)?).to_word_refs();
    if !matches!(
        tail_words.as_slice(),
        ["card" | "cards", "in", "hand"] | ["card" | "cards", "in", "their", "hand"]
    ) {
        return None;
    }

    Some(PlayerCardsInHandConditionAst { player, comparison })
}

pub(crate) fn parse_player_life_total_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeTotalConditionAst> {
    let life_phrases: &[&[&str]] = &[&["life"]];
    if let Some((player, comparison)) =
        parse_player_has_quantity_object_shape(tokens, life_phrases, "life-total condition")
    {
        return Some(PlayerLifeTotalConditionAst { player, comparison });
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (player, quantity_word_idx) = parse_player_has_quantity_subject(&word_refs)?;
    let quantity_token_idx = words.token_index_for_word_index(quantity_word_idx)?;
    let quantity_tokens = tokens.get(quantity_token_idx..)?;
    let (comparison, used) =
        parse_quantity_comparison_prefix(quantity_tokens, false, false, "life-total condition")
            .ok()?;
    let tail_words = TokenWordView::new(quantity_tokens.get(used..)?).to_word_refs();
    if !matches!(tail_words.as_slice(), ["life"]) {
        return None;
    }

    Some(PlayerLifeTotalConditionAst { player, comparison })
}

fn parse_player_has_quantity_object_shape(
    tokens: &[OwnedLexToken],
    object_phrases: &[&[&str]],
    context: &str,
) -> Option<(PlayerFilter, Comparison)> {
    let clause = LexedClause::new(tokens);
    let action_words = &["has", "have"];
    let action_phrases: &[&[&str]] = &[&["has"], &["have"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(action_words)),
        LexPattern::amount("amount", LexCaptureKind::UntilAnyPhrase(object_phrases)),
        LexPattern::object("object", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = parse_player_has_quantity_subject_words(&subject_clause.word_refs())?;
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
    if used != amount_tokens.len() {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !object_clause.matches_any_words(object_phrases) {
        return None;
    }

    Some((player, comparison))
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
    if let Some(condition) = parse_player_life_relation_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    if let [
        "no",
        "opponent" | "opponents",
        "has",
        "more",
        "life",
        "than",
        rest @ ..,
    ] = word_refs.as_slice()
    {
        let (player, used) = parse_life_relation_player_subject(rest)?;
        if used == rest.len() {
            return Some(PlayerLifeRelationConditionAst {
                player,
                relation: PlayerLifeRelationAst::HasNoOpponentWithMoreLifeThan,
            });
        }
    }

    let (subject, subject_len) = parse_life_relation_player_subject(&word_refs)?;
    if !matches!(word_refs.get(subject_len), Some(&"has" | &"have")) {
        return None;
    }
    let tail = &word_refs[subject_len + 1..];

    match tail {
        ["more", "life", "than", "you"] | ["more", "life", "than", "you", "do"] => {
            Some(PlayerLifeRelationConditionAst {
                player: subject,
                relation: PlayerLifeRelationAst::HasMoreLifeThanYou,
            })
        }
        [
            "more",
            "life",
            "than",
            "each",
            "other",
            "player" | "players",
        ] => Some(PlayerLifeRelationConditionAst {
            player: subject,
            relation: PlayerLifeRelationAst::HasMoreLifeThanEachOtherPlayer,
        }),
        ["more", "life", "than", "each", "opponent" | "opponents"]
            if subject == PlayerFilter::You =>
        {
            Some(PlayerLifeRelationConditionAst {
                player: subject,
                relation: PlayerLifeRelationAst::HasMoreLifeThanEachOtherPlayer,
            })
        }
        ["more", "life", "than", rest @ ..] if subject == PlayerFilter::You => {
            let (player, used) = parse_life_relation_player_subject(rest)?;
            if used == rest.len() {
                Some(PlayerLifeRelationConditionAst {
                    player,
                    relation: PlayerLifeRelationAst::HasLessLifeThanYou,
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

fn parse_player_life_relation_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeRelationConditionAst> {
    if let Some(condition) = parse_no_opponent_more_life_than_shape(tokens) {
        return Some(condition);
    }

    let clause = LexedClause::new(tokens);
    let action_words = &["has", "have"];
    let action_phrases: &[&[&str]] = &[&["has"], &["have"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(action_words)),
        LexPattern::tail("relation", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let subject = parse_life_relation_player_subject_words(&subject_clause.word_refs())?;
    let relation_clause = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)?;
    let relation_words = relation_clause.word_refs();

    match relation_words.as_slice() {
        ["more", "life", "than", "you"] | ["more", "life", "than", "you", "do"] => {
            Some(PlayerLifeRelationConditionAst {
                player: subject,
                relation: PlayerLifeRelationAst::HasMoreLifeThanYou,
            })
        }
        [
            "more",
            "life",
            "than",
            "each",
            "other",
            "player" | "players",
        ] => Some(PlayerLifeRelationConditionAst {
            player: subject,
            relation: PlayerLifeRelationAst::HasMoreLifeThanEachOtherPlayer,
        }),
        ["more", "life", "than", "each", "opponent" | "opponents"]
            if subject == PlayerFilter::You =>
        {
            Some(PlayerLifeRelationConditionAst {
                player: subject,
                relation: PlayerLifeRelationAst::HasMoreLifeThanEachOtherPlayer,
            })
        }
        ["more", "life", "than", rest @ ..] if subject == PlayerFilter::You => {
            let player = parse_life_relation_player_subject_words(rest)?;
            Some(PlayerLifeRelationConditionAst {
                player,
                relation: PlayerLifeRelationAst::HasLessLifeThanYou,
            })
        }
        _ => None,
    }
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
    let player = parse_life_relation_player_subject_words(&object_clause.word_refs())?;
    Some(PlayerLifeRelationConditionAst {
        player,
        relation: PlayerLifeRelationAst::HasNoOpponentWithMoreLifeThan,
    })
}

pub(crate) fn parse_player_cards_in_hand_relation_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerCardsInHandRelationConditionAst> {
    if let Some(condition) = parse_player_cards_in_hand_relation_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (subject, subject_len) = parse_life_relation_player_subject(&word_refs)?;
    if !matches!(word_refs.get(subject_len), Some(&"has" | &"have")) {
        return None;
    }
    let tail = &word_refs[subject_len + 1..];

    match tail {
        ["more", "card" | "cards", "in", "hand", "than", "you"]
        | ["more", "card" | "cards", "in", "hand", "than", "you", "do"]
        | [
            "more",
            "card" | "cards",
            "in",
            "their",
            "hand",
            "than",
            "you",
        ]
        | [
            "more",
            "card" | "cards",
            "in",
            "their",
            "hand",
            "than",
            "you",
            "do",
        ] => Some(PlayerCardsInHandRelationConditionAst {
            player: subject,
            relation: PlayerCardsInHandRelationAst::HasMoreCardsInHandThanYou,
        }),
        [
            "more",
            "card" | "cards",
            "in",
            "hand",
            "than",
            "each",
            "other",
            "player" | "players",
        ]
        | [
            "more",
            "card" | "cards",
            "in",
            "their",
            "hand",
            "than",
            "each",
            "other",
            "player" | "players",
        ] => Some(PlayerCardsInHandRelationConditionAst {
            player: subject,
            relation: PlayerCardsInHandRelationAst::HasMoreCardsInHandThanEachOtherPlayer,
        }),
        _ => None,
    }
}

fn parse_player_cards_in_hand_relation_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerCardsInHandRelationConditionAst> {
    let clause = LexedClause::new(tokens);
    let action_words = &["has", "have"];
    let action_phrases: &[&[&str]] = &[&["has"], &["have"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(action_words)),
        LexPattern::tail("relation", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let subject = parse_life_relation_player_subject_words(&subject_clause.word_refs())?;
    let relation_clause = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)?;
    let relation_words = relation_clause.word_refs();

    match relation_words.as_slice() {
        ["more", "card" | "cards", "in", "hand", "than", "you"]
        | ["more", "card" | "cards", "in", "hand", "than", "you", "do"]
        | [
            "more",
            "card" | "cards",
            "in",
            "their",
            "hand",
            "than",
            "you",
        ]
        | [
            "more",
            "card" | "cards",
            "in",
            "their",
            "hand",
            "than",
            "you",
            "do",
        ] => Some(PlayerCardsInHandRelationConditionAst {
            player: subject,
            relation: PlayerCardsInHandRelationAst::HasMoreCardsInHandThanYou,
        }),
        [
            "more",
            "card" | "cards",
            "in",
            "hand",
            "than",
            "each",
            "other",
            "player" | "players",
        ]
        | [
            "more",
            "card" | "cards",
            "in",
            "their",
            "hand",
            "than",
            "each",
            "other",
            "player" | "players",
        ] => Some(PlayerCardsInHandRelationConditionAst {
            player: subject,
            relation: PlayerCardsInHandRelationAst::HasMoreCardsInHandThanEachOtherPlayer,
        }),
        _ => None,
    }
}

pub(crate) fn parse_player_turn_event_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerTurnEventConditionAst> {
    if let Some(condition) = parse_player_turn_event_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (player, subject_len) = parse_life_relation_player_subject(&word_refs)?;

    if let Some((comparison, event)) =
        parse_cards_drawn_this_turn_tail(tokens, &words, &word_refs, subject_len)
    {
        return Some(PlayerTurnEventConditionAst {
            player,
            event,
            comparison,
        });
    }

    let (comparison, event) =
        parse_lands_entered_this_turn_tail(tokens, &words, &word_refs, subject_len)?;
    Some(PlayerTurnEventConditionAst {
        player,
        event,
        comparison,
    })
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
            LexPattern::role_capture(
                "subject",
                LexCaptureRole::Subject,
                LexCaptureKind::UntilPhrase(action_phrase),
            ),
            LexPattern::role_capture(
                "action",
                LexCaptureRole::Action,
                LexCaptureKind::WordCount(action_phrase.len()),
            ),
            LexPattern::role_capture(
                "amount",
                LexCaptureRole::Amount,
                LexCaptureKind::UntilAnyPhrase(card_phrases),
            ),
            LexPattern::role_capture(
                "object",
                LexCaptureRole::Object,
                LexCaptureKind::OneOf(&["card", "cards"]),
            ),
            LexPattern::phrase(&["this", "turn"]),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        let player = parse_life_relation_player_subject_words(&subject_clause.word_refs())?;
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
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::UntilPhrase(&["had"]),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(&["had"]),
        ),
        LexPattern::role_capture(
            "amount",
            LexCaptureRole::Amount,
            LexCaptureKind::UntilAnyPhrase(land_phrases),
        ),
        LexPattern::role_capture("object", LexCaptureRole::Object, LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = parse_life_relation_player_subject_words(&subject_clause.word_refs())?;
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if !matches!(
        object_clause.word_refs().as_slice(),
        [
            "land" | "lands",
            "enter" | "entered",
            "battlefield",
            "under",
            "your" | "their" | "that" | "its",
            "control",
            "this",
            "turn",
        ]
    ) {
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

pub(crate) fn parse_spell_context_condition(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextConditionAst> {
    if let Some(condition) = parse_spell_context_condition_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();

    if let Some(spell) = parse_target_spell_controller_poisoned(&word_refs) {
        return Some(SpellContextConditionAst::ControllerIsPoisoned { spell });
    }
    if let Some(spell) = parse_no_mana_spent_to_cast_target_spell(&word_refs) {
        return Some(SpellContextConditionAst::NoManaSpentToCast { spell });
    }
    if let Some(spell) = parse_you_control_more_creatures_than_spell_controller(&word_refs) {
        return Some(SpellContextConditionAst::YouControlMoreCreaturesThanController { spell });
    }

    None
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
        LexPattern::object("status", LexCaptureKind::WordCount(1)),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let status_clause = matched.capture_clause("status", clause)?;
    if !matches!(status_clause.word_refs().as_slice(), ["poisoned"]) {
        return None;
    }
    let controller_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let spell = parse_target_spell_controller_capture(&controller_clause.word_refs())?;
    Some(SpellContextConditionAst::ControllerIsPoisoned { spell })
}

fn parse_no_mana_spent_to_cast_target_spell_shape(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextConditionAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::amount("amount", LexCaptureKind::WordCount(2)),
        LexPattern::action("action", LexCaptureKind::WordCount(4)),
        LexPattern::object("spell", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let amount_clause = matched.capture_clause("amount", clause)?;
    if !matches!(amount_clause.word_refs().as_slice(), ["no", "mana"]) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(
        action_clause.word_refs().as_slice(),
        ["was" | "were", "spent", "to", "cast"]
    ) {
        return None;
    }
    let spell_clause = matched.capture_clause("spell", clause)?;
    let spell = parse_target_spell_reference(&spell_clause.word_refs())?;
    Some(SpellContextConditionAst::NoManaSpentToCast { spell })
}

fn parse_you_control_more_creatures_than_spell_controller_shape(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextConditionAst> {
    let clause = LexedClause::new(tokens);
    let control_phrases: &[&[&str]] = &[&["control"], &["controls"]];
    let atoms = [
        LexPattern::subject("player", LexCaptureKind::UntilAnyPhrase(control_phrases)),
        LexPattern::action("action", LexCaptureKind::WordCount(1)),
        LexPattern::object("controlled_object", LexCaptureKind::UntilPhrase(&["than"])),
        LexPattern::word("than"),
        LexPattern::object("controller", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let player_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if !matches!(player_clause.word_refs().as_slice(), ["you"]) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if !matches!(
        action_clause.word_refs().as_slice(),
        ["control" | "controls"]
    ) {
        return None;
    }
    let controlled_object_clause = matched.capture_clause("controlled_object", clause)?;
    if !matches!(
        controlled_object_clause.word_refs().as_slice(),
        ["more", "creature" | "creatures"]
    ) {
        return None;
    }
    let controller_clause = matched.capture_clause("controller", clause)?;
    let spell = parse_target_spell_controller_capture(&controller_clause.word_refs())?;
    Some(SpellContextConditionAst::YouControlMoreCreaturesThanController { spell })
}

pub(crate) fn parse_player_spell_cast_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerSpellCastThisTurnConditionAst> {
    if let Some(condition) = parse_player_spell_cast_this_turn_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (player, prefix_len, negated) = parse_spell_cast_this_turn_subject(&word_refs)?;
    if word_refs.len() <= prefix_len + 2
        || !matches!(
            word_refs.get(word_refs.len().saturating_sub(2)..),
            Some(["this", "turn"])
        )
    {
        return None;
    }

    let filter_words = word_refs.get(prefix_len..word_refs.len() - 2)?;
    if !negated && player == PlayerFilter::You && matches!(filter_words, ["another", "spell"]) {
        return Some(PlayerSpellCastThisTurnConditionAst::CountAtLeast { player, count: 2 });
    }

    let filters = parse_spell_cast_filter_words(filter_words)?;
    if filters.is_empty() {
        return None;
    }
    Some(PlayerSpellCastThisTurnConditionAst::MatchingFilters {
        player,
        filters,
        negated,
    })
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
            LexPattern::role_capture(
                "subject",
                LexCaptureRole::Subject,
                LexCaptureKind::UntilPhrase(action_phrase),
            ),
            LexPattern::role_capture(
                "action",
                LexCaptureRole::Action,
                LexCaptureKind::WordCount(action_phrase.len()),
            ),
            LexPattern::role_capture(
                "object",
                LexCaptureRole::Object,
                LexCaptureKind::UntilPhrase(&["this", "turn"]),
            ),
            LexPattern::phrase(&["this", "turn"]),
        ];
        let Some(matched) = LexPattern::new(&atoms).match_clause(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        let Some(player) = parse_spell_cast_this_turn_subject_words(&subject_clause.word_refs())
        else {
            continue;
        };
        let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
        let filter_words = object_clause.word_refs();
        if !*negated
            && player == PlayerFilter::You
            && matches!(filter_words.as_slice(), ["another", "spell"])
        {
            return Some(PlayerSpellCastThisTurnConditionAst::CountAtLeast { player, count: 2 });
        }
        let filters = parse_spell_cast_filter_words(&filter_words)?;
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

pub(crate) fn parse_player_life_change_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeChangeThisTurnConditionAst> {
    if let Some(condition) = parse_player_life_change_this_turn_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (player, direction, quantity_word_idx) = parse_life_change_subject(&word_refs)?;
    let quantity_token_idx = words.token_index_for_word_index(quantity_word_idx)?;
    let quantity_tokens = tokens.get(quantity_token_idx..)?;

    let (comparison, used) = if matches!(
        TokenWordView::new(quantity_tokens)
            .to_word_refs()
            .as_slice(),
        ["life", "this", "turn"]
    ) {
        (Comparison::GreaterThanOrEqual(1), 0)
    } else {
        parse_quantity_comparison_prefix(quantity_tokens, false, false, "life-change condition")
            .ok()?
    };

    let tail_words = TokenWordView::new(quantity_tokens.get(used..)?).to_word_refs();
    if matches!(tail_words.as_slice(), ["life", "this", "turn"]) {
        Some(PlayerLifeChangeThisTurnConditionAst {
            player,
            direction,
            comparison,
        })
    } else {
        None
    }
}

fn parse_player_life_change_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let action_words = &["gained", "lost"];
    let action_phrases: &[&[&str]] = &[&["gained"], &["lost"]];
    let object_phrases: &[&[&str]] = &[&["life"]];
    let atoms = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::UntilAnyPhrase(action_phrases),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(action_words),
        ),
        LexPattern::role_capture(
            "amount",
            LexCaptureRole::Amount,
            LexCaptureKind::UntilAnyPhrase(object_phrases),
        ),
        LexPattern::role_capture(
            "object",
            LexCaptureRole::Object,
            LexCaptureKind::OneOf(&["life"]),
        ),
        LexPattern::phrase(&["this", "turn"]),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = parse_life_change_subject_words(&subject_clause.word_refs())?;
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    let direction = match action_clause.word_refs().as_slice() {
        ["gained"] => PlayerLifeChangeDirectionAst::Gained,
        ["lost"] => PlayerLifeChangeDirectionAst::Lost,
        _ => return None,
    };
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
    if let Some(condition) = parse_player_would_action_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (player, rest) = parse_player_would_subject(&word_refs)?;
    let action = parse_player_would_action(rest)?;
    Some(PlayerWouldActionConditionAst { player, action })
}

fn parse_player_would_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<PlayerWouldActionConditionAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["would"]];
    let atoms = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::UntilAnyPhrase(action_phrases),
        ),
        LexPattern::word("would"),
        LexPattern::role_capture("action", LexCaptureRole::Action, LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let player = parse_player_would_subject_words(&subject_clause.word_refs())?;
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    let action = parse_player_would_action(&action_clause.word_refs())?;
    Some(PlayerWouldActionConditionAst { player, action })
}

pub(crate) fn parse_battlefield_change_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    if let Some(condition) = parse_battlefield_change_this_turn_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();

    if matches!(
        word_refs.as_slice(),
        [
            "no",
            "permanent" | "permanents",
            "left",
            "battlefield",
            "this",
            "turn"
        ]
    ) {
        return Some(
            BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: true },
        );
    }

    if matches!(
        word_refs.as_slice(),
        ["a", "permanent", "left", "battlefield", "this", "turn"]
            | [
                "permanent" | "permanents",
                "left",
                "battlefield",
                "this",
                "turn"
            ]
            | [
                "nonland",
                "permanent",
                "left",
                "battlefield",
                "this",
                "turn",
                "or",
                "spell",
                "was",
                "warped",
                "this",
                "turn",
            ]
    ) {
        return Some(
            BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: false },
        );
    }

    if matches!(
        word_refs.as_slice(),
        [
            "permanent" | "permanents" | "creature" | "creatures",
            "left",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "permanent" | "permanents",
            "you",
            "controlled",
            "left",
            "battlefield",
            "this",
            "turn"
        ]
    ) {
        return Some(
            BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefieldUnderYourControl,
        );
    }

    if matches!(
        word_refs.as_slice(),
        [
            "land" | "lands",
            "you",
            "controlled",
            "was" | "were",
            "put",
            "into",
            "graveyard",
            "from",
            "battlefield",
            "this",
            "turn"
        ]
    ) {
        return Some(
            BattlefieldChangeThisTurnConditionAst::ObjectPutIntoGraveyardFromBattlefield {
                filter: ObjectFilter::land().controlled_by(PlayerFilter::You),
            },
        );
    }

    None
}

fn parse_battlefield_change_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    parse_no_permanent_left_battlefield_shape(tokens)
        .or_else(|| parse_permanent_left_battlefield_under_your_control_shape(tokens))
        .or_else(|| parse_permanent_left_battlefield_shape(tokens))
}

fn parse_no_permanent_left_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::word("no"),
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::OneOf(&["permanent", "permanents"]),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(&["left"]),
        ),
        LexPattern::phrase(&["battlefield", "this", "turn"]),
    ];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: true })
}

fn parse_permanent_left_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let optional_article = [LexPattern::word("a")];
    let atoms = [
        LexPattern::optional(&optional_article),
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::OneOf(&["permanent", "permanents"]),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(&["left"]),
        ),
        LexPattern::phrase(&["battlefield", "this", "turn"]),
    ];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: false })
}

fn parse_permanent_left_battlefield_under_your_control_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let controlled_tail = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::OneOf(&["permanent", "permanents", "creature", "creatures"]),
        ),
        LexPattern::word("left"),
        LexPattern::phrase(&["battlefield", "under", "your", "control", "this", "turn"]),
    ];
    let you_controlled_tail = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::OneOf(&["permanent", "permanents"]),
        ),
        LexPattern::phrase(&["you", "controlled"]),
        LexPattern::word("left"),
        LexPattern::phrase(&["battlefield", "this", "turn"]),
    ];
    let alternatives: &[&[LexPatternAtom<'_>]] = &[&controlled_tail, &you_controlled_tail];
    let atoms = [LexPattern::any_sequence(alternatives)];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefieldUnderYourControl)
}

pub(crate) fn parse_object_death_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<ObjectDeathThisTurnConditionAst> {
    if let Some(condition) = parse_object_death_this_turn_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();

    if matches!(
        word_refs.as_slice(),
        ["a", "creature", "died", "this", "turn"]
            | ["creature" | "creatures", "died", "this", "turn"]
    ) {
        return Some(ObjectDeathThisTurnConditionAst {
            event: ObjectDeathThisTurnEventAst::Died,
            filter: ObjectFilter::creature(),
            comparison: Comparison::GreaterThanOrEqual(1),
        });
    }

    if matches!(
        word_refs.as_slice(),
        [
            "a",
            "creature",
            "card",
            "was",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "anywhere",
            "this",
            "turn"
        ] | [
            "creature",
            "card",
            "was",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "anywhere",
            "this",
            "turn"
        ]
    ) {
        return Some(ObjectDeathThisTurnConditionAst {
            event: ObjectDeathThisTurnEventAst::PutIntoYourGraveyardFromAnywhere,
            filter: ObjectFilter::creature(),
            comparison: Comparison::GreaterThanOrEqual(1),
        });
    }

    if let Ok((comparison, used)) =
        parse_quantity_comparison_prefix(tokens, false, false, "object-death condition")
    {
        let tail_words = TokenWordView::new(tokens.get(used..)?).to_word_refs();
        if matches!(
            tail_words.as_slice(),
            ["creature" | "creatures", "died", "this", "turn"]
        ) {
            return Some(ObjectDeathThisTurnConditionAst {
                event: ObjectDeathThisTurnEventAst::Died,
                filter: ObjectFilter::creature(),
                comparison,
            });
        }
    }

    None
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
    let atoms = [
        LexPattern::role_capture(
            "amount",
            LexCaptureRole::Amount,
            LexCaptureKind::UntilAnyPhrase(object_phrases),
        ),
        LexPattern::role_capture(
            "object",
            LexCaptureRole::Object,
            LexCaptureKind::OneOf(&["creature", "creatures"]),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(&["died"]),
        ),
        LexPattern::phrase(&["this", "turn"]),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let amount_capture = matched.capture_by_role(LexCaptureRole::Amount)?;
    let comparison = if amount_capture.word_range.is_empty() {
        Comparison::GreaterThanOrEqual(1)
    } else {
        let amount_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
        if matches!(amount_clause.word_refs().as_slice(), ["a"]) {
            Comparison::GreaterThanOrEqual(1)
        } else {
            parse_amount_capture_comparison(tokens, clause, &matched, "object-death condition")?
        }
    };

    Some(ObjectDeathThisTurnConditionAst {
        event: ObjectDeathThisTurnEventAst::Died,
        filter: ObjectFilter::creature(),
        comparison,
    })
}

fn parse_object_put_into_your_graveyard_from_anywhere_shape(
    tokens: &[OwnedLexToken],
) -> Option<ObjectDeathThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let optional_article = [LexPattern::word("a")];
    let atoms = [
        LexPattern::optional(&optional_article),
        LexPattern::role_capture(
            "object",
            LexCaptureRole::Object,
            LexCaptureKind::WordCount(2),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(&["was"]),
        ),
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
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    if object_clause.word_refs().as_slice() != ["creature", "card"] {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    if action_clause.word_refs().as_slice() != ["was"] {
        return None;
    }
    Some(ObjectDeathThisTurnConditionAst {
        event: ObjectDeathThisTurnEventAst::PutIntoYourGraveyardFromAnywhere,
        filter: ObjectFilter::creature(),
        comparison: Comparison::GreaterThanOrEqual(1),
    })
}

pub(crate) fn parse_battlefield_entry_condition(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldEntryConditionAst> {
    if let Some(condition) = parse_battlefield_entry_shape(tokens) {
        return Some(condition);
    }

    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();

    if let Some(condition) = parse_you_had_land_entered_battlefield_this_turn(&word_refs) {
        return Some(condition);
    }

    if let Some(condition) = parse_you_had_object_entered_battlefield_last_turn(tokens, &words) {
        return Some(condition);
    }

    parse_object_entered_battlefield_this_turn(tokens, &words)
}

fn parse_battlefield_entry_shape(tokens: &[OwnedLexToken]) -> Option<BattlefieldEntryConditionAst> {
    parse_you_had_land_entered_battlefield_this_turn_shape(tokens)
        .or_else(|| parse_you_had_object_entered_battlefield_last_turn_shape(tokens))
        .or_else(|| parse_object_entered_battlefield_this_turn_shape(tokens))
}

fn parse_player_status_subject_and_tail<'a>(
    words: &'a [&'a str],
) -> Option<(PlayerFilter, &'a [&'a str])> {
    match words {
        ["youre", rest @ ..] => Some((PlayerFilter::You, rest)),
        ["you", "are" | "have", rest @ ..] => Some((PlayerFilter::You, rest)),
        ["defending", "player", "is" | "has", rest @ ..] => Some((PlayerFilter::Defending, rest)),
        ["attacking", "player", "is" | "has", rest @ ..] => Some((PlayerFilter::Attacking, rest)),
        ["that", "player", "is" | "has", rest @ ..] => Some((PlayerFilter::IteratedPlayer, rest)),
        ["an", "opponent", "is" | "has", rest @ ..] | ["opponent", "is" | "has", rest @ ..] => {
            Some((PlayerFilter::Opponent, rest))
        }
        ["a", "player", "is" | "has", rest @ ..] | ["player", "is" | "has", rest @ ..] => {
            Some((PlayerFilter::Any, rest))
        }
        _ => None,
    }
}

fn parse_player_status_subject_words(words: &[&str]) -> Option<PlayerFilter> {
    match words {
        ["you"] => Some(PlayerFilter::You),
        ["defending", "player"] => Some(PlayerFilter::Defending),
        ["attacking", "player"] => Some(PlayerFilter::Attacking),
        ["that", "player"] => Some(PlayerFilter::IteratedPlayer),
        ["an", "opponent"] | ["opponent"] => Some(PlayerFilter::Opponent),
        ["a", "player"] | ["player"] => Some(PlayerFilter::Any),
        _ => None,
    }
}

fn parse_player_has_quantity_subject(words: &[&str]) -> Option<(PlayerFilter, usize)> {
    match words {
        ["you", "have", ..] => Some((PlayerFilter::You, 2)),
        ["a" | "an", "opponent", "has", ..] => Some((PlayerFilter::Opponent, 3)),
        ["opponent", "has", ..] => Some((PlayerFilter::Opponent, 2)),
        ["a", "player", "has", ..] => Some((PlayerFilter::Any, 3)),
        ["player", "has", ..] => Some((PlayerFilter::Any, 2)),
        ["that", "player", "has", ..] => Some((PlayerFilter::IteratedPlayer, 3)),
        ["attacking", "player", "has", ..] => Some((PlayerFilter::Attacking, 3)),
        ["defending", "player", "has", ..] => Some((PlayerFilter::Defending, 3)),
        _ => None,
    }
}

fn parse_player_has_quantity_subject_words(words: &[&str]) -> Option<PlayerFilter> {
    match words {
        ["you"] => Some(PlayerFilter::You),
        ["a" | "an", "opponent"] | ["opponent"] => Some(PlayerFilter::Opponent),
        ["a", "player"] | ["player"] => Some(PlayerFilter::Any),
        ["that", "player"] => Some(PlayerFilter::IteratedPlayer),
        ["attacking", "player"] => Some(PlayerFilter::Attacking),
        ["defending", "player"] => Some(PlayerFilter::Defending),
        _ => None,
    }
}

fn parse_life_relation_player_subject(words: &[&str]) -> Option<(PlayerFilter, usize)> {
    match words {
        ["you", ..] => Some((PlayerFilter::You, 1)),
        ["that", "player", ..] | ["player", "who", ..] => Some((PlayerFilter::IteratedPlayer, 2)),
        ["target", "player", ..] => Some((PlayerFilter::target_player(), 2)),
        ["target", "opponent", ..] => Some((PlayerFilter::target_opponent(), 2)),
        ["each", "opponent" | "opponents", ..] => Some((PlayerFilter::Opponent, 2)),
        ["a" | "an", "opponent", ..] => Some((PlayerFilter::Opponent, 2)),
        ["opponent" | "opponents", ..] => Some((PlayerFilter::Opponent, 1)),
        ["a" | "any", "player", ..] => Some((PlayerFilter::Any, 2)),
        ["player", ..] => Some((PlayerFilter::Any, 1)),
        ["defending", "player", ..] => Some((PlayerFilter::Defending, 2)),
        ["attacking", "player", ..] => Some((PlayerFilter::Attacking, 2)),
        _ => None,
    }
}

fn parse_life_relation_player_subject_words(words: &[&str]) -> Option<PlayerFilter> {
    match words {
        ["you"] => Some(PlayerFilter::You),
        ["that", "player"] | ["player", "who"] => Some(PlayerFilter::IteratedPlayer),
        ["target", "player"] => Some(PlayerFilter::target_player()),
        ["target", "opponent"] => Some(PlayerFilter::target_opponent()),
        ["each", "opponent" | "opponents"] => Some(PlayerFilter::Opponent),
        ["a" | "an", "opponent"] | ["opponent" | "opponents"] => Some(PlayerFilter::Opponent),
        ["a" | "any", "player"] | ["player"] => Some(PlayerFilter::Any),
        ["defending", "player"] => Some(PlayerFilter::Defending),
        ["attacking", "player"] => Some(PlayerFilter::Attacking),
        _ => None,
    }
}

fn parse_cards_drawn_this_turn_tail(
    tokens: &[OwnedLexToken],
    words: &TokenWordView,
    word_refs: &[&str],
    subject_len: usize,
) -> Option<(Comparison, PlayerTurnEventAst)> {
    let quantity_word_idx = match word_refs.get(subject_len..) {
        Some(["drew", ..]) => subject_len + 1,
        Some(["has" | "have", "drawn", ..]) => subject_len + 2,
        _ => return None,
    };
    let quantity_token_idx = words.token_index_for_word_index(quantity_word_idx)?;
    let quantity_tokens = tokens.get(quantity_token_idx..)?;
    let (comparison, used) =
        parse_quantity_comparison_prefix(quantity_tokens, false, false, "cards-drawn condition")
            .ok()?;
    let tail_words = TokenWordView::new(quantity_tokens.get(used..)?).to_word_refs();
    if matches!(tail_words.as_slice(), ["card" | "cards", "this", "turn"]) {
        Some((comparison, PlayerTurnEventAst::CardsDrawn))
    } else {
        None
    }
}

fn parse_lands_entered_this_turn_tail(
    tokens: &[OwnedLexToken],
    words: &TokenWordView,
    word_refs: &[&str],
    subject_len: usize,
) -> Option<(Comparison, PlayerTurnEventAst)> {
    if !matches!(word_refs.get(subject_len), Some(&"had")) {
        return None;
    }
    let quantity_word_idx = subject_len + 1;
    let quantity_token_idx = words.token_index_for_word_index(quantity_word_idx)?;
    let quantity_tokens = tokens.get(quantity_token_idx..)?;
    let (comparison, used) =
        parse_quantity_comparison_prefix(quantity_tokens, false, false, "lands-entered condition")
            .ok()?;
    let tail_words = TokenWordView::new(quantity_tokens.get(used..)?).to_word_refs();
    if matches!(
        tail_words.as_slice(),
        [
            "land" | "lands",
            "enter" | "entered",
            "battlefield",
            "under",
            "your" | "their" | "that" | "its",
            "control",
            "this",
            "turn",
        ]
    ) {
        Some((
            comparison,
            PlayerTurnEventAst::LandsEnteredBattlefieldUnderControl,
        ))
    } else {
        None
    }
}

fn parse_you_had_land_entered_battlefield_this_turn(
    words: &[&str],
) -> Option<BattlefieldEntryConditionAst> {
    if matches!(
        words,
        [
            "you",
            "had",
            "land" | "lands",
            "enter" | "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ]
    ) {
        Some(
            BattlefieldEntryConditionAst::LandEnteredUnderYourControlThisTurn {
                player: PlayerAst::You,
            },
        )
    } else {
        None
    }
}

fn parse_you_had_land_entered_battlefield_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldEntryConditionAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::WordCount(1),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(&["had"]),
        ),
        LexPattern::role_capture(
            "object",
            LexCaptureRole::Object,
            LexCaptureKind::OneOf(&["land", "lands"]),
        ),
        LexPattern::any_word(&["enter", "entered"]),
        LexPattern::phrase(&["battlefield", "under", "your", "control", "this", "turn"]),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if subject_clause.word_refs().as_slice() != ["you"] {
        return None;
    }
    Some(
        BattlefieldEntryConditionAst::LandEnteredUnderYourControlThisTurn {
            player: PlayerAst::You,
        },
    )
}

fn parse_you_had_object_entered_battlefield_last_turn(
    tokens: &[OwnedLexToken],
    words: &TokenWordView,
) -> Option<BattlefieldEntryConditionAst> {
    let word_refs = words.to_word_refs();
    if !matches!(word_refs.get(..2), Some(["you", "had"])) {
        return None;
    }

    let enter_idx = word_refs
        .iter()
        .position(|word| matches!(*word, "enter" | "entered"))?;
    if enter_idx <= 2 {
        return None;
    }
    let battlefield_idx = enter_idx + 1;
    let tail_start = if matches!(word_refs.get(battlefield_idx), Some(&"the")) {
        battlefield_idx + 1
    } else {
        battlefield_idx
    };
    if !matches!(
        word_refs.get(tail_start..),
        Some(["battlefield", "under", "your", "control", "last", "turn"])
    ) {
        return None;
    }

    let object_start = words.token_index_for_word_index(2)?;
    let object_end = words.token_index_for_word_index(enter_idx)?;
    let mut filter =
        parse_object_filter_with_grammar_entrypoint(tokens.get(object_start..object_end)?, false)
            .ok()?;
    filter.controller = Some(PlayerFilter::You);
    if matches!(word_refs.get(2), Some(&"another" | &"other")) {
        filter.other = true;
    }
    Some(BattlefieldEntryConditionAst::ObjectEntered {
        filter,
        window: BattlefieldEntryTurnWindowAst::LastTurn,
    })
}

fn parse_you_had_object_entered_battlefield_last_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldEntryConditionAst> {
    let clause = LexedClause::new(tokens);
    let enter_phrases: &[&[&str]] = &[&["enter"], &["entered"]];
    let optional_the = [LexPattern::word("the")];
    let atoms = [
        LexPattern::role_capture(
            "subject",
            LexCaptureRole::Subject,
            LexCaptureKind::WordCount(1),
        ),
        LexPattern::role_capture(
            "action",
            LexCaptureRole::Action,
            LexCaptureKind::OneOf(&["had"]),
        ),
        LexPattern::role_capture(
            "object",
            LexCaptureRole::Object,
            LexCaptureKind::UntilAnyPhrase(enter_phrases),
        ),
        LexPattern::any_word(&["enter", "entered"]),
        LexPattern::optional(&optional_the),
        LexPattern::phrase(&["battlefield", "under", "your", "control", "last", "turn"]),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    if subject_clause.word_refs().as_slice() != ["you"] {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let mut filter =
        parse_object_filter_with_grammar_entrypoint(object_clause.tokens(), false).ok()?;
    filter.controller = Some(PlayerFilter::You);
    if matches!(
        object_clause.word_refs().first(),
        Some(&"another" | &"other")
    ) {
        filter.other = true;
    }
    Some(BattlefieldEntryConditionAst::ObjectEntered {
        filter,
        window: BattlefieldEntryTurnWindowAst::LastTurn,
    })
}

fn parse_object_entered_battlefield_this_turn(
    tokens: &[OwnedLexToken],
    words: &TokenWordView,
) -> Option<BattlefieldEntryConditionAst> {
    let word_refs = words.to_word_refs();
    let enter_idx = word_refs
        .iter()
        .position(|word| matches!(*word, "enter" | "entered"))?;
    if enter_idx == 0 {
        return None;
    }
    let battlefield_idx = enter_idx + 1;
    let tail_start = if matches!(word_refs.get(battlefield_idx), Some(&"the")) {
        battlefield_idx + 1
    } else {
        battlefield_idx
    };
    if !matches!(
        word_refs.get(tail_start..),
        Some(["battlefield", "under", "your", "control", "this", "turn"])
    ) {
        return None;
    }

    let object_end = words.token_index_for_word_index(enter_idx)?;
    let mut filter =
        parse_object_filter_with_grammar_entrypoint(tokens.get(..object_end)?, false).ok()?;
    filter.controller = Some(PlayerFilter::You);
    if matches!(word_refs.first(), Some(&"another" | &"other")) {
        filter.other = true;
    }
    Some(BattlefieldEntryConditionAst::ObjectEntered {
        filter,
        window: BattlefieldEntryTurnWindowAst::ThisTurn,
    })
}

fn parse_object_entered_battlefield_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldEntryConditionAst> {
    let clause = LexedClause::new(tokens);
    let enter_phrases: &[&[&str]] = &[&["enter"], &["entered"]];
    let optional_the = [LexPattern::word("the")];
    let atoms = [
        LexPattern::role_capture(
            "object",
            LexCaptureRole::Object,
            LexCaptureKind::UntilAnyPhrase(enter_phrases),
        ),
        LexPattern::any_word(&["enter", "entered"]),
        LexPattern::optional(&optional_the),
        LexPattern::phrase(&["battlefield", "under", "your", "control", "this", "turn"]),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let mut filter =
        parse_object_filter_with_grammar_entrypoint(object_clause.tokens(), false).ok()?;
    filter.controller = Some(PlayerFilter::You);
    if matches!(
        object_clause.word_refs().first(),
        Some(&"another" | &"other")
    ) {
        filter.other = true;
    }
    Some(BattlefieldEntryConditionAst::ObjectEntered {
        filter,
        window: BattlefieldEntryTurnWindowAst::ThisTurn,
    })
}

fn parse_target_spell_controller_poisoned(words: &[&str]) -> Option<SpellContextReferenceAst> {
    let rest = parse_target_spell_controller_subject(words)?;
    if rest == ["poisoned"] {
        Some(SpellContextReferenceAst::TargetSpell)
    } else {
        None
    }
}

fn parse_no_mana_spent_to_cast_target_spell(words: &[&str]) -> Option<SpellContextReferenceAst> {
    let [
        "no",
        "mana",
        "was" | "were",
        "spent",
        "to",
        "cast",
        rest @ ..,
    ] = words
    else {
        return None;
    };
    if parse_target_spell_reference(rest).is_some() {
        Some(SpellContextReferenceAst::TargetSpell)
    } else {
        None
    }
}

fn parse_you_control_more_creatures_than_spell_controller(
    words: &[&str],
) -> Option<SpellContextReferenceAst> {
    let [
        "you",
        "control" | "controls",
        "more",
        "creature" | "creatures",
        "than",
        rest @ ..,
    ] = words
    else {
        return None;
    };
    let controller_tail = parse_target_spell_controller_subject(rest)?;
    if controller_tail.is_empty() {
        Some(SpellContextReferenceAst::TargetSpell)
    } else {
        None
    }
}

fn parse_target_spell_controller_subject<'a>(words: &'a [&'a str]) -> Option<&'a [&'a str]> {
    match words {
        ["its", "controller", rest @ ..]
        | ["that", "spells", "controller", rest @ ..]
        | ["that", "spell", "controller", rest @ ..] => Some(rest),
        _ => None,
    }
}

fn parse_target_spell_controller_capture(words: &[&str]) -> Option<SpellContextReferenceAst> {
    match words {
        ["its", "controller"]
        | ["that", "spells", "controller"]
        | ["that", "spell", "controller"] => Some(SpellContextReferenceAst::TargetSpell),
        _ => None,
    }
}

fn parse_target_spell_reference(words: &[&str]) -> Option<SpellContextReferenceAst> {
    match words {
        ["it"] | ["that", "spell"] => Some(SpellContextReferenceAst::TargetSpell),
        _ => None,
    }
}

fn parse_spell_cast_this_turn_subject(words: &[&str]) -> Option<(PlayerFilter, usize, bool)> {
    match words {
        ["that", "player", "didnt", "cast", ..] => Some((PlayerFilter::Active, 4, true)),
        ["that", "player", "did", "not", "cast", ..] => Some((PlayerFilter::Active, 5, true)),
        ["you", "didnt", "cast", ..] => Some((PlayerFilter::You, 3, true)),
        ["you", "did", "not", "cast", ..] => Some((PlayerFilter::You, 4, true)),
        ["opponent", "has", "cast", ..] | ["opponents", "have", "cast", ..] => {
            Some((PlayerFilter::Opponent, 3, false))
        }
        ["youve", "cast", ..] => Some((PlayerFilter::You, 2, false)),
        ["you", "have", "cast", ..] => Some((PlayerFilter::You, 3, false)),
        ["you", "cast", ..] => Some((PlayerFilter::You, 2, false)),
        _ => None,
    }
}

fn parse_spell_cast_this_turn_subject_words(words: &[&str]) -> Option<PlayerFilter> {
    match words {
        ["that", "player"] => Some(PlayerFilter::Active),
        ["you"] | ["youve"] => Some(PlayerFilter::You),
        ["opponent"] | ["opponents"] => Some(PlayerFilter::Opponent),
        _ => None,
    }
}

fn parse_spell_cast_filter_words(words: &[&str]) -> Option<Vec<ObjectFilter>> {
    if let Some((left, right)) = split_both_spell_cast_filter_words(words) {
        return Some(vec![
            parse_spell_cast_filter(left)?,
            parse_spell_cast_filter(right)?,
        ]);
    }
    Some(vec![parse_spell_cast_filter(words)?])
}

fn split_both_spell_cast_filter_words<'a>(
    words: &'a [&'a str],
) -> Option<(&'a [&'a str], &'a [&'a str])> {
    let stripped = match words {
        ["both", rest @ ..] => rest,
        _ => words,
    };
    let and_idx = stripped.iter().position(|word| *word == "and")?;
    let (left, right_with_and) = stripped.split_at(and_idx);
    let right = right_with_and.get(1..)?;
    if left.is_empty() || right.is_empty() {
        return None;
    }
    if !matches!(words, ["both", ..])
        && (!spell_named_prefix_matches(left) || !spell_named_prefix_matches(right))
    {
        return None;
    }
    Some((left, right))
}

fn spell_named_prefix_matches(words: &[&str]) -> bool {
    matches!(words, ["a", "spell", "named", ..] | ["spell", "named", ..])
}

fn parse_spell_cast_filter(words: &[&str]) -> Option<ObjectFilter> {
    let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_object_filter_with_grammar_entrypoint(&filter_tokens, false).ok()
}

fn parse_life_change_subject(
    words: &[&str],
) -> Option<(PlayerFilter, PlayerLifeChangeDirectionAst, usize)> {
    match words {
        ["you", "gained", ..] => Some((PlayerFilter::You, PlayerLifeChangeDirectionAst::Gained, 2)),
        ["you", "lost", ..] => Some((PlayerFilter::You, PlayerLifeChangeDirectionAst::Lost, 2)),
        ["opponent" | "opponents", "lost", ..] => Some((
            PlayerFilter::Opponent,
            PlayerLifeChangeDirectionAst::Lost,
            2,
        )),
        ["an", "opponent", "lost", ..] => Some((
            PlayerFilter::Opponent,
            PlayerLifeChangeDirectionAst::Lost,
            3,
        )),
        ["one", "or", "more", "opponents", "lost", ..] => Some((
            PlayerFilter::Opponent,
            PlayerLifeChangeDirectionAst::Lost,
            5,
        )),
        _ => None,
    }
}

fn parse_life_change_subject_words(words: &[&str]) -> Option<PlayerFilter> {
    match words {
        ["you"] => Some(PlayerFilter::You),
        ["opponent" | "opponents"] | ["an", "opponent"] | ["one", "or", "more", "opponents"] => {
            Some(PlayerFilter::Opponent)
        }
        _ => None,
    }
}

fn parse_player_would_subject<'a>(words: &'a [&'a str]) -> Option<(PlayerFilter, &'a [&'a str])> {
    match words {
        ["you", "would", rest @ ..] => Some((PlayerFilter::You, rest)),
        ["an", "opponent", "would", rest @ ..] | ["opponent", "would", rest @ ..] => {
            Some((PlayerFilter::Opponent, rest))
        }
        ["opponents", "would", rest @ ..] => Some((PlayerFilter::Opponent, rest)),
        _ => None,
    }
}

fn parse_player_would_subject_words(words: &[&str]) -> Option<PlayerFilter> {
    match words {
        ["you"] => Some(PlayerFilter::You),
        ["an", "opponent"] | ["opponent"] | ["opponents"] => Some(PlayerFilter::Opponent),
        _ => None,
    }
}

fn parse_player_would_action(words: &[&str]) -> Option<PlayerWouldActionAst> {
    match words {
        ["draw", "a", "card"] | ["draw", "card"] => Some(PlayerWouldActionAst::DrawCard),
        ["proliferate"] => Some(PlayerWouldActionAst::Proliferate),
        ["begin", "extra", "turn"] | ["begin", "an", "extra", "turn"] => {
            Some(PlayerWouldActionAst::BeginExtraTurn)
        }
        _ => None,
    }
}

fn parse_player_achievement_tail(words: &[&str]) -> Option<PlayerAchievementAst> {
    let words = strip_optional_article(words);
    match words {
        ["citys", "blessing"] | ["city", "blessing"] => Some(PlayerAchievementAst::CitysBlessing),
        ["citys", "blessing", "for", "each", ..] | ["city", "blessing", "for", "each", ..] => {
            Some(PlayerAchievementAst::CitysBlessing)
        }
        ["completed", rest @ ..] => parse_completed_dungeon_achievement(rest),
        ["full", "party"] => Some(PlayerAchievementAst::FullParty),
        _ => None,
    }
}

fn parse_completed_dungeon_achievement(words: &[&str]) -> Option<PlayerAchievementAst> {
    let words = strip_optional_article(words);
    if words == ["dungeon"] {
        return Some(PlayerAchievementAst::CompletedDungeon { dungeon_name: None });
    }
    if words.is_empty() {
        return None;
    }
    Some(PlayerAchievementAst::CompletedDungeon {
        dungeon_name: Some(words.join(" ")),
    })
}

fn parse_player_status_tail(words: &[&str]) -> Option<PlayerStatusAst> {
    match strip_optional_article(words) {
        ["monarch"] => Some(PlayerStatusAst::Monarch),
        ["initiative"] => Some(PlayerStatusAst::Initiative),
        ["max" | "maximum", "speed"] => Some(PlayerStatusAst::MaxSpeed),
        _ => None,
    }
}

fn strip_optional_article<'a>(words: &'a [&'a str]) -> &'a [&'a str] {
    match words {
        ["a" | "an" | "the", rest @ ..] => rest,
        _ => words,
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
}

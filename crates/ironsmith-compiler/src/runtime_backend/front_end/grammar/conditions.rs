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
    parse_card_type, parse_color, parse_quantity_comparison_prefix, parse_subtype_flexible,
    trim_edge_punctuation_tokens,
};
use super::filters::parse_object_filter_with_grammar_entrypoint;
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};

const MORE_LIFE_THAN_PLAYER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["more", "life", "than"]),
    LexPattern::subject("player", LexCaptureKind::Rest),
]);
const MORE_LIFE_THAN_YOU_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["more", "life", "than", "you"],
            &["more", "life", "than", "you", "do"]
        ]
);
const MORE_LIFE_THAN_EACH_OTHER_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["more", "life", "than", "each", "other", "player"],
            &["more", "life", "than", "each", "other", "players"],
        ]
);
const MORE_LIFE_THAN_EACH_OPPONENT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["more", "life", "than", "each", "opponent"],
            &["more", "life", "than", "each", "opponents"],
        ]
);
const MORE_CARDS_IN_HAND_THAN_YOU_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["more", "card", "in", "hand", "than", "you"],
            &["more", "cards", "in", "hand", "than", "you"],
            &["more", "card", "in", "hand", "than", "you", "do"],
            &["more", "cards", "in", "hand", "than", "you", "do"],
            &["more", "card", "in", "their", "hand", "than", "you"],
            &["more", "cards", "in", "their", "hand", "than", "you"],
            &["more", "card", "in", "their", "hand", "than", "you", "do"],
            &["more", "cards", "in", "their", "hand", "than", "you", "do"],
        ]
);
const MORE_CARDS_IN_HAND_THAN_EACH_OTHER_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "more", "card", "in", "hand", "than", "each", "other", "player"
            ],
            &[
                "more", "cards", "in", "hand", "than", "each", "other", "player"
            ],
            &[
                "more", "card", "in", "hand", "than", "each", "other", "players"
            ],
            &[
                "more", "cards", "in", "hand", "than", "each", "other", "players"
            ],
            &[
                "more", "card", "in", "their", "hand", "than", "each", "other", "player",
            ],
            &[
                "more", "cards", "in", "their", "hand", "than", "each", "other", "player",
            ],
            &[
                "more", "card", "in", "their", "hand", "than", "each", "other", "players",
            ],
            &[
                "more", "cards", "in", "their", "hand", "than", "each", "other", "players",
            ],
        ]
);

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
    ObjectLeftBattlefield { filter: ObjectFilter },
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
    parse_control_condition_shape(tokens, options)
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
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(action_words)),
        LexPattern::tail(
            "amount_and_object",
            LexCaptureKind::UntilLastAnyPhrase(different_powers_tails),
        ),
        LexPattern::modifier("modifier", LexCaptureKind::OneOf(&["with"])),
        LexPattern::phrase(&["different"]),
        LexPattern::any_word(&["powers", "power"]),
    ];
    let basic_atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(action_words)),
        LexPattern::tail("amount_and_object", LexCaptureKind::OneOrMoreWords),
    ];
    let matched = if options.allow_different_powers_tail {
        LexPattern::new(&modifier_atoms)
            .match_clause(clause)
            .or_else(|| LexPattern::new(&basic_atoms).match_clause(clause))?
    } else {
        LexPattern::new(&basic_atoms).match_clause(clause)?
    };

    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let (player, player_filter) = parse_control_condition_subject_clause(subject_clause, options)?;
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
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["own"], &["owns"]];
    let action_words = &["own", "owns"];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(action_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(action_words)),
        LexPattern::tail("amount_and_object", LexCaptureKind::OneOrMoreWords),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let (player, player_filter) =
        parse_ownership_condition_subject_clause(subject_clause, options)?;
    let tail_capture = matched.capture_by_role(LexCaptureRole::Tail)?;
    let tail_range = clause
        .words()
        .token_range_for_word_range(tail_capture.word_range.start, tail_capture.word_range.end)?;

    finish_ownership_condition(player, player_filter, tokens.get(tail_range)?, options)
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
    let clause = LexedClause::new(tokens);
    let copula_phrases: &[&[&str]] = &[&["is"], &["are"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(copula_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["is", "are"])),
        LexPattern::object("state", LexCaptureKind::OneOf(state_words)),
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
    let clause = LexedClause::new(tokens);
    let copula_phrases: &[&[&str]] = &[&["is"], &["are"]];
    let atoms = [
        LexPattern::subject("subject", LexCaptureKind::UntilAnyPhrase(copula_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(&["is", "are"])),
        LexPattern::object("descriptor", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
    let subject = parse_subject_descriptor_subject_clause(subject_clause)?;
    let descriptor_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let descriptor = parse_object_descriptor_clause(descriptor_clause)?;
    let filter =
        parse_object_filter_with_grammar_entrypoint(subject_clause.tokens(), false).ok()?;

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

fn parse_player_status_subject_clause(clause: LexedClause<'_>) -> Option<PlayerFilter> {
    parse_player_status_subject_clause_shape(clause)
}

fn parse_player_status_tail_clause(clause: LexedClause<'_>) -> Option<PlayerStatusAst> {
    parse_player_status_tail_clause_shape(clause)
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
    let player = parse_player_has_quantity_subject_clause(subject_clause)?;
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
    let subject = parse_life_relation_player_subject_clause(subject_clause)?;
    let relation_clause = matched
        .capture_clause_by_role(LexCaptureRole::Tail, clause)?
        .trimmed();

    if MORE_LIFE_THAN_YOU_PATTERN.matches(relation_clause) {
        return Some(PlayerLifeRelationConditionAst {
            player: subject,
            relation: PlayerLifeRelationAst::HasMoreLifeThanYou,
        });
    }
    if MORE_LIFE_THAN_EACH_OTHER_PLAYER_PATTERN.matches(relation_clause) {
        return Some(PlayerLifeRelationConditionAst {
            player: subject,
            relation: PlayerLifeRelationAst::HasMoreLifeThanEachOtherPlayer,
        });
    }
    if subject == PlayerFilter::You && MORE_LIFE_THAN_EACH_OPPONENT_PATTERN.matches(relation_clause)
    {
        return Some(PlayerLifeRelationConditionAst {
            player: subject,
            relation: PlayerLifeRelationAst::HasMoreLifeThanEachOtherPlayer,
        });
    }
    if subject == PlayerFilter::You
        && let Some(matched) = MORE_LIFE_THAN_PLAYER_PATTERN.match_clause(relation_clause)
    {
        let player_clause =
            matched.capture_clause_by_role(LexCaptureRole::Subject, relation_clause)?;
        let player = parse_life_relation_player_subject_clause(player_clause)?;
        return Some(PlayerLifeRelationConditionAst {
            player,
            relation: PlayerLifeRelationAst::HasLessLifeThanYou,
        });
    }
    None
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
    let subject = parse_life_relation_player_subject_clause(subject_clause)?;
    let relation_clause = matched
        .capture_clause_by_role(LexCaptureRole::Tail, clause)?
        .trimmed();

    if MORE_CARDS_IN_HAND_THAN_YOU_PATTERN.matches(relation_clause) {
        return Some(PlayerCardsInHandRelationConditionAst {
            player: subject,
            relation: PlayerCardsInHandRelationAst::HasMoreCardsInHandThanYou,
        });
    }
    if MORE_CARDS_IN_HAND_THAN_EACH_OTHER_PLAYER_PATTERN.matches(relation_clause) {
        return Some(PlayerCardsInHandRelationConditionAst {
            player: subject,
            relation: PlayerCardsInHandRelationAst::HasMoreCardsInHandThanEachOtherPlayer,
        });
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
    const LANDS_ENTERED_THIS_TURN_OBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object("object", LexCaptureKind::OneOf(&["land", "lands"])),
        LexPattern::any_word(&["enter", "entered"]),
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
    let clause = LexedClause::new(tokens);
    let atoms = [
        LexPattern::subject("player", LexCaptureKind::OneOf(&["you"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["control", "controls"])),
        LexPattern::phrase(&["more"]),
        LexPattern::object(
            "controlled_object",
            LexCaptureKind::OneOf(&["creature", "creatures"]),
        ),
        LexPattern::word("than"),
        LexPattern::object("controller", LexCaptureKind::Rest),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let controller_clause = matched.capture_clause("controller", clause)?;
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
        .or_else(|| parse_permanent_left_battlefield_shape(tokens))
        .or_else(|| parse_object_left_battlefield_shape(tokens))
}

fn parse_no_permanent_left_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let optional_the = [LexPattern::word("the")];
    let atoms = [
        LexPattern::word("no"),
        LexPattern::subject(
            "subject",
            LexCaptureKind::OneOf(&["permanent", "permanents"]),
        ),
        LexPattern::action("action", LexCaptureKind::OneOf(&["left"])),
        LexPattern::optional(&optional_the),
        LexPattern::phrase(&["battlefield", "this", "turn"]),
    ];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: true })
}

fn parse_permanent_left_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let optional_article = [LexPattern::any_word(&["a", "an"])];
    let optional_the = [LexPattern::word("the")];
    let atoms = [
        LexPattern::optional(&optional_article),
        LexPattern::subject(
            "subject",
            LexCaptureKind::OneOf(&["permanent", "permanents"]),
        ),
        LexPattern::action("action", LexCaptureKind::OneOf(&["left"])),
        LexPattern::optional(&optional_the),
        LexPattern::phrase(&["battlefield", "this", "turn"]),
    ];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield { negated: false })
}

fn parse_permanent_left_battlefield_under_your_control_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let optional_article = [LexPattern::any_word(&[
        "a", "an",
    ])];
    let optional_the = [LexPattern::word("the")];
    let controlled_tail = [
        LexPattern::optional(&optional_article),
        LexPattern::subject(
            "subject",
            LexCaptureKind::OneOf(&["permanent", "permanents", "creature", "creatures"]),
        ),
        LexPattern::word("left"),
        LexPattern::optional(&optional_the),
        LexPattern::phrase(&["battlefield", "under", "your", "control", "this", "turn"]),
    ];
    let you_controlled_tail = [
        LexPattern::optional(&optional_article),
        LexPattern::subject(
            "subject",
            LexCaptureKind::OneOf(&["permanent", "permanents"]),
        ),
        LexPattern::phrase(&["you", "controlled"]),
        LexPattern::word("left"),
        LexPattern::optional(&optional_the),
        LexPattern::phrase(&["battlefield", "this", "turn"]),
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
    let optional_article = [LexPattern::any_word(&[
        "a", "an",
    ])];
    let optional_graveyard_article = [LexPattern::word("a")];
    let optional_the = [LexPattern::word("the")];
    let atoms = [
        LexPattern::optional(&optional_article),
        LexPattern::object("object", LexCaptureKind::OneOf(&["land", "lands"])),
        LexPattern::phrase(&["you", "controlled"]),
        LexPattern::action("action", LexCaptureKind::OneOf(&["was", "were"])),
        LexPattern::phrase(&["put", "into"]),
        LexPattern::optional(&optional_graveyard_article),
        LexPattern::word("graveyard"),
        LexPattern::word("from"),
        LexPattern::optional(&optional_the),
        LexPattern::phrase(&["battlefield", "this", "turn"]),
    ];
    LexPattern::new(&atoms).match_clause(clause)?;
    Some(
        BattlefieldChangeThisTurnConditionAst::ObjectPutIntoGraveyardFromBattlefield {
            filter: ObjectFilter::land().controlled_by(PlayerFilter::You),
        },
    )
}

fn parse_object_left_battlefield_shape(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
    let clause = LexedClause::new(tokens);
    let optional_article = [LexPattern::any_word(&[
        "a", "an",
    ])];
    let optional_the = [LexPattern::word("the")];
    let atoms = [
        LexPattern::optional(&optional_article),
        LexPattern::object("object", LexCaptureKind::UntilPhrase(&["left"])),
        LexPattern::action("action", LexCaptureKind::OneOf(&["left"])),
        LexPattern::optional(&optional_the),
        LexPattern::phrase(&["battlefield", "this", "turn"]),
    ];
    let matched = LexPattern::new(&atoms).match_clause(clause)?;
    let object_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let mut filter =
        parse_object_filter_with_grammar_entrypoint(object_clause.tokens(), false).ok()?;
    filter.zone = Some(Zone::Battlefield);
    Some(BattlefieldChangeThisTurnConditionAst::ObjectLeftBattlefield {
        filter,
    })
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

fn parse_player_status_subject_clause_shape(clause: LexedClause<'_>) -> Option<PlayerFilter> {
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

fn parse_player_status_tail_clause_shape(clause: LexedClause<'_>) -> Option<PlayerStatusAst> {
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
    if clause_matches_any_phrase(clause, &[&["opponent"], &["opponents"]]) {
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

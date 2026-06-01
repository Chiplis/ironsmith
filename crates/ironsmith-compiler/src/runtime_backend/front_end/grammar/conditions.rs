use crate::cards::builders::PlayerAst;
use crate::color::ColorSet;
use crate::effect::{Comparison, Value, ValueComparisonOperator};
use crate::static_abilities::AnthemCountExpression;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

use super::super::lexer::{OwnedLexToken, TokenWordView};
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

    let tail_tokens = trim_edge_punctuation_tokens(tokens.get(prefix_len..)?);
    let (comparison, quantity_len) =
        parse_quantity_comparison_prefix(tail_tokens, true, true, "control condition").ok()?;
    let mut filter_tokens = trim_edge_punctuation_tokens(tail_tokens.get(quantity_len..)?);
    if filter_tokens.is_empty() {
        return None;
    }
    let requires_different_powers = options.allow_different_powers_tail
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
            let prefixed_filter_tokens = tokens
                .get(..prefix_len)?
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

    let tail_tokens = trim_edge_punctuation_tokens(tokens.get(prefix_len..)?);
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

pub(crate) fn parse_subject_descriptor_condition(
    tokens: &[OwnedLexToken],
) -> Option<SubjectDescriptorConditionAst> {
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

pub(crate) fn parse_player_status_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerStatusConditionAst> {
    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (player, rest) = parse_player_status_subject_and_tail(&word_refs)?;
    let status = parse_player_status_tail(rest)?;

    Some(PlayerStatusConditionAst { player, status })
}

pub(crate) fn parse_player_achievement_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerAchievementConditionAst> {
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

pub(crate) fn parse_player_cards_in_hand_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerCardsInHandConditionAst> {
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

pub(crate) fn parse_player_life_relation_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeRelationConditionAst> {
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

pub(crate) fn parse_player_cards_in_hand_relation_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerCardsInHandRelationConditionAst> {
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

pub(crate) fn parse_player_turn_event_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerTurnEventConditionAst> {
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

pub(crate) fn parse_spell_context_condition(
    tokens: &[OwnedLexToken],
) -> Option<SpellContextConditionAst> {
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

pub(crate) fn parse_player_spell_cast_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerSpellCastThisTurnConditionAst> {
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

pub(crate) fn parse_player_life_change_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerLifeChangeThisTurnConditionAst> {
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

pub(crate) fn parse_player_would_action_condition(
    tokens: &[OwnedLexToken],
) -> Option<PlayerWouldActionConditionAst> {
    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let (player, rest) = parse_player_would_subject(&word_refs)?;
    let action = parse_player_would_action(rest)?;
    Some(PlayerWouldActionConditionAst { player, action })
}

pub(crate) fn parse_battlefield_change_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldChangeThisTurnConditionAst> {
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

pub(crate) fn parse_object_death_this_turn_condition(
    tokens: &[OwnedLexToken],
) -> Option<ObjectDeathThisTurnConditionAst> {
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

pub(crate) fn parse_battlefield_entry_condition(
    tokens: &[OwnedLexToken],
) -> Option<BattlefieldEntryConditionAst> {
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

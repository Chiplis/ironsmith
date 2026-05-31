use super::super::grammar::primitives as grammar;
use super::super::keyword_static::parse_pt_modifier_values;
use super::super::lexer::{LexedClause, OwnedLexToken};
use super::super::object_filters::parse_object_filter_lexed;
use super::super::rule_engine::{LexClauseView, LexRuleDef, LexRuleIndex, RULE_SHAPE_STARTS_IF};
use super::sentence_helpers::target_ast_to_object_filter;
use super::{parse_object_filter, parse_target_phrase as parse_target_phrase_lexed};
use crate::cards::builders::{CardTextError, ChoiceCount, EffectAst};
use crate::cards::builders::{IT_TAG, PlayerAst, TagKey, TargetAst, Value};
use crate::effect::{EventValueSpec, Until};
use crate::object::CounterType;
use crate::runtime_backend::contains_until_end_of_turn;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use crate::runtime_backend::model::ast::{SubjectVerbActionAst, SubjectVerbRoleAst};
use crate::runtime_backend::util::parse_choice_count_token_prefix_consumed;
use crate::static_abilities::StaticAbilityId;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::CardType;

const KEYWORD_BUNDLE_IF_IT_HAS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["if", "it", "has"]);
const UNTIL_END_OF_TURN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["until", "end", "of", "turn"]);
const UNTIL_YOUR_NEXT_TURN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["until", "your", "next", "turn"]);
const UNTIL_END_OF_COMBAT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["until", "end", "of", "combat"]);
const AND_SO_ON_FOR_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["and", "so", "on", "for"]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const DOUBLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["double"]);
const LOSES_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["loses"]);
const LIFE_TOTAL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["life", "total"]);
const THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const DOUBLE_UNSPENT_MANA_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "double", "the", "amount", "of", "each", "type", "of", "unspent", "mana",
        ]
);
const EMPTY_MANA_POOL_PATTERNS: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "player", "loses", "all", "unspent", "mana"],
            &["target", "player", "loses", "all", "unspent", "mana"],
            &["target", "opponent", "loses", "all", "unspent", "mana"],
            &["you", "lose", "all", "unspent", "mana"],
        ]
);
const THAT_PLAYER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["that", "player"]);
const TARGET_OPPONENT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "opponent"]);
const YOU_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you"]);
const EACH_OR_ALL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["each"], &["all"]]);
const GET_OR_GETS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["get"], &["gets"]]);
const TOKEN_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["token"]);
const ARTIFACT_OR_ARTIFACTS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["artifact"], &["artifacts"]]);
const ENCHANTMENT_OR_ENCHANTMENTS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enchantment"], &["enchantments"]]);
const DRAW_THAT_MANY_CARDS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["draw", "that", "many", "cards"]);
const POWER_AND_TOUGHNESS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["power", "and", "toughness"]);
const POWER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["power"]);
const TOUGHNESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["toughness"]);
const SCALED_TARGET_POWER_VERBS: &[(&str, i32)] = &[("double", 1), ("triple", 2)];

fn scaled_target_power_verb(word: &str) -> Option<(&'static str, i32)> {
    SCALED_TARGET_POWER_VERBS
        .iter()
        .find_map(|(verb, multiplier)| (*verb == word).then_some((*verb, *multiplier)))
}

fn parse_keyword_bundle_static_ability(words: &[&str]) -> Option<(StaticAbilityId, usize)> {
    const KEYWORD_PHRASES: &[(&[&str], StaticAbilityId)] = &[
        (&["first", "strike"], StaticAbilityId::FirstStrike),
        (&["double", "strike"], StaticAbilityId::DoubleStrike),
        (&["flying"], StaticAbilityId::Flying),
        (&["deathtouch"], StaticAbilityId::Deathtouch),
        (&["haste"], StaticAbilityId::Haste),
        (&["hexproof"], StaticAbilityId::Hexproof),
        (&["indestructible"], StaticAbilityId::Indestructible),
        (&["lifelink"], StaticAbilityId::Lifelink),
        (&["menace"], StaticAbilityId::Menace),
        (&["protection"], StaticAbilityId::Protection),
        (&["reach"], StaticAbilityId::Reach),
        (&["trample"], StaticAbilityId::Trample),
        (&["vigilance"], StaticAbilityId::Vigilance),
        (&["partner"], StaticAbilityId::Partner),
    ];

    KEYWORD_PHRASES.iter().find_map(|(phrase, ability_id)| {
        words
            .starts_with(phrase)
            .then_some((*ability_id, phrase.len()))
    })
}

fn parse_keyword_bundle_pump_clause(
    words: &[&str],
    start: usize,
) -> Result<Option<((Value, Value), StaticAbilityId, usize)>, CardTextError> {
    let Some(modifier) = words.get(start).copied() else {
        return Ok(None);
    };
    let Ok((power, toughness)) = parse_pt_modifier_values(modifier) else {
        return Ok(None);
    };
    let ability_start = start + 4;
    if !KEYWORD_BUNDLE_IF_IT_HAS_PREFIX_PATTERN.matches_words(&words[start + 1..]) {
        return Ok(None);
    }
    let Some((ability_id, consumed)) = parse_keyword_bundle_static_ability(&words[ability_start..])
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported keyword-bundle ability in gets clause: '{}'",
            words.join(" ")
        )));
    };
    Ok(Some((
        (power, toughness),
        ability_id,
        ability_start + consumed,
    )))
}

pub(crate) fn parse_keyword_bundle_pump_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    if clause.is_empty() {
        return Ok(None);
    }

    let (subject_start_word_idx, duration) =
        if UNTIL_END_OF_TURN_PREFIX_PATTERN.matches_words(&words) {
            (4usize, Until::EndOfTurn)
        } else if UNTIL_YOUR_NEXT_TURN_PREFIX_PATTERN.matches_words(&words) {
            (4usize, Until::YourNextTurn)
        } else if UNTIL_END_OF_COMBAT_PREFIX_PATTERN.matches_words(&words) {
            (4usize, Until::EndOfCombat)
        } else {
            return Ok(None);
        };

    let Some(get_word_idx) = words
        .iter()
        .position(|word| GET_OR_GETS_WORD_PATTERN.matches_word(word))
    else {
        return Ok(None);
    };
    if get_word_idx <= subject_start_word_idx {
        return Ok(None);
    }

    let subject_clause = clause.between_words_trimmed(subject_start_word_idx, get_word_idx);
    if subject_clause.is_empty() {
        return Ok(None);
    }

    let filter_clause = if subject_clause
        .first_word()
        .is_some_and(|word| EACH_OR_ALL_WORD_PATTERN.matches_word(word))
    {
        subject_clause
            .after_words(1)
            .unwrap_or_else(|| subject_clause.from(subject_clause.len()))
    } else {
        subject_clause
    };
    if filter_clause.is_empty() {
        return Ok(None);
    }

    let base_filter = parse_object_filter(filter_clause.tokens(), false)?;

    let Some(((power, toughness), first_ability, mut cursor)) =
        parse_keyword_bundle_pump_clause(&words, get_word_idx + 1)?
    else {
        return Ok(None);
    };

    let mut ability_ids = vec![first_ability];
    while let Some(((next_power, next_toughness), next_ability, next_cursor)) =
        parse_keyword_bundle_pump_clause(&words, cursor)?
    {
        if next_power != power || next_toughness != toughness {
            return Err(CardTextError::ParseError(format!(
                "keyword-bundle gets clause changes modifier mid-sequence: '{}'",
                words.join(" ")
            )));
        }
        ability_ids.push(next_ability);
        cursor = next_cursor;
    }

    if !AND_SO_ON_FOR_PREFIX_PATTERN.matches_words(&words[cursor..]) {
        return Ok(None);
    }
    cursor += 4;

    while cursor < words.len() {
        if AND_WORD_PATTERN.matches_word(words[cursor]) {
            cursor += 1;
            continue;
        }
        let Some((ability_id, consumed)) = parse_keyword_bundle_static_ability(&words[cursor..])
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing keyword-bundle list in gets clause: '{}'",
                words.join(" ")
            )));
        };
        ability_ids.push(ability_id);
        cursor += consumed;
    }

    let effects = ability_ids
        .into_iter()
        .map(|ability_id| {
            EffectAst::subject_verb_pump_all(
                base_filter.clone().with_static_ability(ability_id),
                power.clone(),
                toughness.clone(),
                duration.clone(),
            )
        })
        .collect::<Vec<_>>();

    Ok(Some(effects))
}

pub(crate) fn parse_scaled_target_power_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();
    let words = clause.word_refs();
    let Some((verb, multiplier)) = clause.first_word().and_then(scaled_target_power_verb) else {
        return Ok(None);
    };

    let scaled_stat = |value: Value| {
        if multiplier == 1 {
            value
        } else {
            Value::Scaled(Box::new(value), multiplier)
        }
    };

    let scale_pt_from_value_spec =
        |target: &TargetAst, include_power: bool, include_toughness: bool| {
            let amount_source_filter =
                target_ast_to_object_filter(target.clone()).unwrap_or_else(|| {
                    let mut fallback = ObjectFilter::default();
                    fallback.card_types.push(CardType::Creature);
                    fallback
                });
            let value_spec = Box::new(ChooseSpec::target(ChooseSpec::Object(amount_source_filter)));
            EffectAst::subject_verb_pump(
                if include_power {
                    scaled_stat(Value::PowerOf(value_spec.clone()))
                } else {
                    Value::Fixed(0)
                },
                if include_toughness {
                    scaled_stat(Value::ToughnessOf(value_spec))
                } else {
                    Value::Fixed(0)
                },
                target.clone(),
                Until::EndOfTurn,
                None,
            )
        };
    let scale_pt_all = |filter: ObjectFilter, include_power: bool, include_toughness: bool| {
        EffectAst::subject_verb_scale_power_toughness_all(
            filter,
            include_power,
            include_toughness,
            multiplier,
            Until::EndOfTurn,
        )
    };
    let parse_double_life_total_subject =
        |subject_words: &[&str]| -> Option<(PlayerAst, PlayerFilter)> {
            match subject_words {
                ["your"] => Some((PlayerAst::You, PlayerFilter::You)),
                ["target", "player"] | ["target", "players"] => {
                    Some((PlayerAst::Target, PlayerFilter::target_player()))
                }
                ["target", "opponent"] | ["target", "opponents"] => {
                    Some((PlayerAst::TargetOpponent, PlayerFilter::target_opponent()))
                }
                ["opponent"] | ["opponents"] | ["an", "opponent"] | ["an", "opponents"] => {
                    Some((PlayerAst::Opponent, PlayerFilter::Opponent))
                }
                _ => None,
            }
        };
    let parse_double_mana_pool_subject = |subject_words: &[&str]| -> Option<PlayerAst> {
        match subject_words {
            ["you", "have"] => Some(PlayerAst::You),
            ["target", "player", "has"] | ["target", "player", "have"] => Some(PlayerAst::Target),
            ["target", "opponent", "has"] | ["target", "opponent", "have"] => {
                Some(PlayerAst::TargetOpponent)
            }
            ["opponent", "has"] | ["opponents", "have"] => Some(PlayerAst::Opponent),
            _ => None,
        }
    };

    if DOUBLE_WORD_PATTERN.matches_word(verb)
        && let Some(life_total_idx) = LIFE_TOTAL_PATTERN.find_exact_window(&words, 2)
        && let Some((player, player_filter)) =
            parse_double_life_total_subject(&words[1..life_total_idx])
        && life_total_idx + 2 == words.len()
    {
        return Ok(Some(vec![EffectAst::subject_verb_set_life_total(
            player,
            Value::Scaled(Box::new(Value::LifeTotal(player_filter)), 2),
        )]));
    }

    if DOUBLE_WORD_PATTERN.matches_word(verb)
        && let Some(mana_prefix_len) = DOUBLE_UNSPENT_MANA_PREFIX_PATTERN.matched_prefix_len(&words)
        && let Some(player) = parse_double_mana_pool_subject(&words[mana_prefix_len..])
    {
        return Ok(Some(vec![EffectAst::subject_verb_double_mana_pool(player)]));
    }
    if LOSES_WORD_PATTERN.matches_word(verb) && EMPTY_MANA_POOL_PATTERNS.matches_words(&words) {
        let player = if THAT_PLAYER_PREFIX_PATTERN.matches_words(&words) {
            PlayerAst::That
        } else if TARGET_OPPONENT_PREFIX_PATTERN.matches_words(&words) {
            PlayerAst::TargetOpponent
        } else if YOU_PREFIX_PATTERN.matches_words(&words) {
            PlayerAst::You
        } else {
            PlayerAst::Target
        };
        return Ok(Some(vec![EffectAst::subject_verb_empty_mana_pool(player)]));
    }

    let duration_start =
        if words.len() >= 4 && contains_until_end_of_turn(&words[words.len() - 4..]) {
            words.len() - 4
        } else {
            words.len()
        };
    let subject_end = duration_start;

    if words
        .first()
        .is_some_and(|word| word.eq_ignore_ascii_case(verb))
        && THE_WORD_PATTERN.matches_word_at(&words, 1)
    {
        let (include_power, include_toughness, subject_start) = match words.get(2..) {
            Some(["power", "of", ..]) => (true, false, 4),
            Some(["toughness", "of", ..]) => (false, true, 4),
            Some(["power", "and", "toughness", "of", ..]) => (true, true, 6),
            _ => (false, false, 0),
        };
        if subject_start != 0 && subject_start < subject_end {
            let subject_clause = clause.between_words_trimmed(subject_start, subject_end);
            if subject_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing subject in {verb} clause (clause: '{}')",
                    clause_text
                )));
            }

            let subject_words = &words[subject_start..subject_end];
            if subject_words
                .first()
                .is_some_and(|word| EACH_OR_ALL_WORD_PATTERN.matches_word(word))
            {
                let filter_clause = subject_clause
                    .after_words(1)
                    .unwrap_or_else(|| subject_clause.from(subject_clause.len()));
                if filter_clause.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing filter in {verb} clause (clause: '{}')",
                        clause_text
                    )));
                }
                let filter = parse_object_filter(filter_clause.tokens(), false)?;
                return Ok(Some(vec![scale_pt_all(
                    filter,
                    include_power,
                    include_toughness,
                )]));
            }

            let target = parse_target_phrase_lexed(subject_clause.tokens())?;
            return Ok(Some(vec![scale_pt_from_value_spec(
                &target,
                include_power,
                include_toughness,
            )]));
        }
    }

    let (include_power, include_toughness, characteristic_start) = if subject_end >= 4
        && POWER_AND_TOUGHNESS_PATTERN.matches_words(&words[subject_end - 3..subject_end])
    {
        (true, true, subject_end - 3)
    } else if subject_end >= 1 && POWER_WORD_PATTERN.matches_word(words[subject_end - 1]) {
        (true, false, subject_end - 1)
    } else if subject_end >= 1 && TOUGHNESS_WORD_PATTERN.matches_word(words[subject_end - 1]) {
        (false, true, subject_end - 1)
    } else {
        return Ok(None);
    };
    if characteristic_start <= 1 {
        return Ok(None);
    }

    let target_clause = clause.between_words_trimmed(1, characteristic_start);
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in {verb} clause (clause: '{}')",
            clause_text
        )));
    }

    if words
        .get(1)
        .is_some_and(|word| EACH_OR_ALL_WORD_PATTERN.matches_word(word))
    {
        let filter_clause = target_clause
            .after_words(1)
            .unwrap_or_else(|| target_clause.from(target_clause.len()));
        if filter_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter in {verb} clause (clause: '{}')",
                clause_text
            )));
        }
        let filter = parse_object_filter(filter_clause.tokens(), false)?;
        return Ok(Some(vec![scale_pt_all(
            filter,
            include_power,
            include_toughness,
        )]));
    }

    let target = parse_target_phrase_lexed(target_clause.tokens())?;
    Ok(Some(vec![scale_pt_from_value_spec(
        &target,
        include_power,
        include_toughness,
    )]))
}

pub(super) fn parse_redirect_next_damage_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::clause_pattern_helpers::parse_redirect_next_damage_sentence(view.tokens)
}

pub(super) fn parse_prevent_next_time_damage_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::clause_pattern_helpers::parse_prevent_next_time_damage_sentence(view.tokens)
}

pub(super) fn parse_scaled_target_power_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_scaled_target_power_sentence(view.tokens)
}

pub(super) fn parse_keyword_bundle_pump_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_keyword_bundle_pump_sentence(view.tokens)
}

pub(super) fn parse_spell_this_way_pay_life_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if grammar::words_match_prefix(
        view.tokens,
        &["if", "you", "cast", "a", "spell", "this", "way"],
    )
    .is_some()
        && grammar::contains_word(view.tokens, "rather")
        && grammar::contains_word(view.tokens, "mana")
        && grammar::contains_word(view.tokens, "cost")
    {
        return Ok(Some(vec![
            EffectAst::subject_verb_grant_tagged_spell_alternative_cost_pay_life_by_mana_value_until_end_of_turn(TagKey::from(IT_TAG), PlayerAst::You),
        ]));
    }
    Ok(None)
}

pub(super) fn parse_sacrifice_any_number_then_draw_that_many_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause = LexedClause::new(view.tokens);
    if !clause.first_is_word("sacrifice") {
        return Ok(None);
    }
    let Some((before_then, after_then)) = clause.split_once_on_then_trimmed() else {
        return Ok(None);
    };
    if !DRAW_THAT_MANY_CARDS_PATTERN.matches(after_then) {
        return Ok(None);
    }

    if !before_then.first_is_word("sacrifice") {
        return Ok(None);
    }
    let Some((count, used)) = parse_choice_count_token_prefix_consumed(&before_then.tokens()[1..])
    else {
        return Ok(None);
    };
    if count != ChoiceCount::any_number() {
        return Ok(None);
    };
    let filter_clause = before_then.from(1 + used).trimmed();
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object after 'any number of' (clause: '{}')",
            view.display_text()
        )));
    }
    let filter_words = filter_clause.word_refs();
    let filter = if TOKEN_MARKER_PATTERN.matches_words(&filter_words)
        && filter_words
            .iter()
            .any(|word| ARTIFACT_OR_ARTIFACTS_WORD_PATTERN.matches_word(word))
        && filter_words
            .iter()
            .any(|word| ENCHANTMENT_OR_ENCHANTMENTS_WORD_PATTERN.matches_word(word))
    {
        let mut filter = ObjectFilter::default();
        filter.any_of = vec![
            ObjectFilter::artifact().you_control(),
            ObjectFilter::enchantment().you_control(),
            ObjectFilter::default().token().you_control(),
        ];
        filter
    } else {
        parse_object_filter_lexed(filter_clause.tokens(), false)?
    };
    let tag = TagKey::from("sacrificed_0");

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::any_number(),
            count_value: None,
            player: PlayerAst::You,
            tag: tag.clone(),
        },
        EffectAst::subject_verb_sacrifice_all(PlayerAst::You, ObjectFilter::tagged(tag)),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: Value::EventValue(EventValueSpec::Amount),
            },
        ),
    ]))
}

pub(super) const SUBJECT_VERB_PRE_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 6] = [
    LexRuleDef {
        id: "redirect-next-damage",
        priority: 100,
        heads: &["the", "all"],
        shape_mask: 0,
        run: parse_redirect_next_damage_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "prevent-next-time-damage",
        priority: 110,
        heads: &["the"],
        shape_mask: 0,
        run: parse_prevent_next_time_damage_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "scale-target-power",
        priority: 120,
        heads: &["double", "triple"],
        shape_mask: 0,
        run: parse_scaled_target_power_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "keyword-bundle-pump",
        priority: 125,
        heads: &["until"],
        shape_mask: 0,
        run: parse_keyword_bundle_pump_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "spell-this-way-pay-life",
        priority: 130,
        heads: &["if"],
        shape_mask: RULE_SHAPE_STARTS_IF,
        run: parse_spell_this_way_pay_life_rule_lexed,
    },
    LexRuleDef {
        id: "sacrifice-any-number-then-draw-that-many",
        priority: 140,
        heads: &["sacrifice"],
        shape_mask: 0,
        run: parse_sacrifice_any_number_then_draw_that_many_rule_lexed,
    },
];

pub(super) const SUBJECT_VERB_PRE_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&SUBJECT_VERB_PRE_DIAGNOSTIC_RULES_LEXED);

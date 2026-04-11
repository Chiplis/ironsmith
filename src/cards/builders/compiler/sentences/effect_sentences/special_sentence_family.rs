use super::super::grammar::primitives as grammar;
use super::super::lexer::{OwnedLexToken, TokenWordView};
use super::super::rule_engine::{LexClauseView, LexRuleDef, LexRuleIndex, RULE_SHAPE_STARTS_IF};
use super::super::util::trim_commas;
use super::sentence_helpers::target_ast_to_object_filter;
use super::{parse_object_filter, parse_target_phrase as parse_target_phrase_lexed};
use crate::cards::builders::compiler::contains_until_end_of_turn;
use crate::cards::builders::{CardTextError, EffectAst};
use crate::cards::builders::{IT_TAG, PlayerAst, TagKey, TargetAst, Value};
use crate::effect::Until;
use crate::object::CounterType;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::CardType;

pub(crate) fn parse_exile_then_meld_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["exile", "them"]).is_none() {
        return Ok(None);
    }
    let Some(meld_idx) = crate::cards::builders::compiler::grammar::primitives::find_phrase_start(
        tokens,
        &["then", "meld", "them", "into"],
    ) else {
        return Ok(None);
    };
    let result_words = &clause_words[meld_idx + 4..];
    if result_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing meld result name (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    Ok(Some(EffectAst::Meld {
        result_name: result_words.join(" "),
        enters_tapped: false,
        enters_attacking: false,
    }))
}

pub(crate) fn parse_if_damage_would_be_dealt_put_counters_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["if", "damage", "would", "be", "dealt", "to"])
        .is_none()
    {
        return Ok(None);
    }

    let Some(this_turn_rel) =
        crate::cards::builders::compiler::grammar::primitives::find_phrase_start(
            &tokens[6..],
            &["this", "turn"],
        )
    else {
        return Ok(None);
    };
    let this_turn_idx = 6 + this_turn_rel;
    let tail = &clause_words[this_turn_idx + 2..];
    let valid_tail = matches!(
        tail,
        [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counters", "on",
            "it"
        ] | [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counters", "on",
            "that", "creature"
        ] | [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counter", "on",
            "it"
        ] | [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counter", "on",
            "that", "creature"
        ]
    );
    if !valid_tail {
        return Ok(None);
    }

    let target_tokens = &tokens[6..this_turn_idx];
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase_lexed(target_tokens)?;

    Ok(Some(EffectAst::PreventDamageToTargetPutCounters {
        amount: None,
        target,
        duration: Until::EndOfTurn,
        counter_type: CounterType::PlusOnePlusOne,
    }))
}

pub(crate) fn parse_scaled_target_power_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let word_storage = TokenWordView::new(tokens);
    let words = word_storage.word_refs();
    let Some((verb, multiplier)) = word_storage.first().and_then(|word| match word {
        "double" => Some(("double", 1)),
        "triple" => Some(("triple", 2)),
        _ => None,
    }) else {
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
            EffectAst::Pump {
                power: if include_power {
                    scaled_stat(Value::PowerOf(value_spec.clone()))
                } else {
                    Value::Fixed(0)
                },
                toughness: if include_toughness {
                    scaled_stat(Value::ToughnessOf(value_spec))
                } else {
                    Value::Fixed(0)
                },
                target: target.clone(),
                duration: Until::EndOfTurn,
                condition: None,
            }
        };
    let scale_pt_all = |filter: ObjectFilter, include_power: bool, include_toughness: bool| {
        EffectAst::ScalePowerToughnessAll {
            filter,
            power: include_power,
            toughness: include_toughness,
            multiplier,
            duration: Until::EndOfTurn,
        }
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

    if verb == "double"
        && let Some(life_total_idx) = words.iter().position(|word| *word == "life")
        && words.get(life_total_idx + 1) == Some(&"total")
        && let Some((player, player_filter)) =
            parse_double_life_total_subject(&words[1..life_total_idx])
        && life_total_idx + 2 == words.len()
    {
        return Ok(Some(vec![EffectAst::SetLifeTotal {
            amount: Value::Scaled(Box::new(Value::LifeTotal(player_filter)), 2),
            player,
        }]));
    }

    let mana_prefix = [
        "double", "the", "amount", "of", "each", "type", "of", "unspent", "mana",
    ];
    if verb == "double"
        && words.starts_with(&mana_prefix)
        && let Some(player) = parse_double_mana_pool_subject(&words[mana_prefix.len()..])
    {
        return Ok(Some(vec![EffectAst::DoubleManaPool { player }]));
    }

    let duration_start =
        if words.len() >= 4 && contains_until_end_of_turn(&words[words.len() - 4..]) {
            words.len() - 4
        } else {
            words.len()
        };
    let subject_end = duration_start;

    if words.first().copied() == Some(verb) && words.get(1).copied() == Some("the") {
        let (include_power, include_toughness, subject_start) = match words.get(2..) {
            Some(["power", "of", ..]) => (true, false, 4),
            Some(["toughness", "of", ..]) => (false, true, 4),
            Some(["power", "and", "toughness", "of", ..]) => (true, true, 6),
            _ => (false, false, 0),
        };
        if subject_start != 0 && subject_start < subject_end {
            let subject_tokens = trim_commas(&tokens[subject_start..subject_end]);
            if subject_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing subject in {verb} clause (clause: '{}')",
                    words.join(" ")
                )));
            }

            let subject_words = &words[subject_start..subject_end];
            if subject_words
                .first()
                .is_some_and(|word| *word == "each" || *word == "all")
            {
                let filter_tokens = &subject_tokens[1..];
                if filter_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing filter in {verb} clause (clause: '{}')",
                        words.join(" ")
                    )));
                }
                let filter = parse_object_filter(filter_tokens, false)?;
                return Ok(Some(vec![scale_pt_all(
                    filter,
                    include_power,
                    include_toughness,
                )]));
            }

            let target = parse_target_phrase_lexed(&subject_tokens)?;
            return Ok(Some(vec![scale_pt_from_value_spec(
                &target,
                include_power,
                include_toughness,
            )]));
        }
    }

    let (include_power, include_toughness, characteristic_start) = if subject_end >= 4
        && words[subject_end - 3..subject_end] == ["power", "and", "toughness"]
    {
        (true, true, subject_end - 3)
    } else if subject_end >= 1 && words[subject_end - 1] == "power" {
        (true, false, subject_end - 1)
    } else if subject_end >= 1 && words[subject_end - 1] == "toughness" {
        (false, true, subject_end - 1)
    } else {
        return Ok(None);
    };
    if characteristic_start <= 1 {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..characteristic_start]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in {verb} clause (clause: '{}')",
            words.join(" ")
        )));
    }

    if words
        .get(1)
        .is_some_and(|word| *word == "each" || *word == "all")
    {
        let filter_tokens = &target_tokens[1..];
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter in {verb} clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(Some(vec![scale_pt_all(
            filter,
            include_power,
            include_toughness,
        )]));
    }

    let target = parse_target_phrase_lexed(&target_tokens)?;
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
            EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag: TagKey::from(IT_TAG),
                player: PlayerAst::You,
            },
        ]));
    }
    Ok(None)
}

pub(super) const SPECIAL_PRE_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 4] = [
    LexRuleDef {
        id: "redirect-next-damage",
        priority: 100,
        heads: &["the"],
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
        id: "spell-this-way-pay-life",
        priority: 130,
        heads: &["if"],
        shape_mask: RULE_SHAPE_STARTS_IF,
        run: parse_spell_this_way_pay_life_rule_lexed,
    },
];

pub(super) const SPECIAL_PRE_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&SPECIAL_PRE_DIAGNOSTIC_RULES_LEXED);

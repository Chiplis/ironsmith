use super::super::grammar::primitives as grammar;
use super::super::keyword_static::parse_pt_modifier_values;
use super::super::lexer::{OwnedLexToken, TokenWordView};
use super::super::object_filters::parse_object_filter_lexed;
use super::super::rule_engine::{LexClauseView, LexRuleDef, LexRuleIndex, RULE_SHAPE_STARTS_IF};
use super::super::token_primitives::find_window_index as find_word_sequence_index;
use super::super::util::trim_commas;
use super::sentence_helpers::target_ast_to_object_filter;
use super::{parse_object_filter, parse_target_phrase as parse_target_phrase_lexed};
use crate::cards::builders::{CardTextError, ChoiceCount, EffectAst};
use crate::cards::builders::{IT_TAG, PlayerAst, TagKey, TargetAst, Value};
use crate::effect::{EventValueSpec, Until};
use crate::object::CounterType;
use crate::runtime_backend::contains_until_end_of_turn;
use crate::runtime_backend::model::ast::{SubjectVerbActionAst, SubjectVerbRoleAst};
use crate::runtime_backend::token_index_for_word_index;
use crate::static_abilities::StaticAbilityId;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::CardType;

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
    if words.get(start + 1..ability_start) != Some(&["if", "it", "has"][..]) {
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
    let word_storage = TokenWordView::new(tokens);
    let words = word_storage.word_refs();
    if words.is_empty() {
        return Ok(None);
    }

    let (subject_start_word_idx, duration) = if words.starts_with(&["until", "end", "of", "turn"]) {
        (4usize, Until::EndOfTurn)
    } else if words.starts_with(&["until", "your", "next", "turn"]) {
        (4usize, Until::YourNextTurn)
    } else if words.starts_with(&["until", "end", "of", "combat"]) {
        (4usize, Until::EndOfCombat)
    } else {
        return Ok(None);
    };

    let Some(get_word_idx) = words
        .iter()
        .position(|word| matches!(*word, "get" | "gets"))
    else {
        return Ok(None);
    };
    if get_word_idx <= subject_start_word_idx {
        return Ok(None);
    }

    let subject_start_token_idx =
        token_index_for_word_index(tokens, subject_start_word_idx).unwrap_or(tokens.len());
    let subject_end_token_idx =
        token_index_for_word_index(tokens, get_word_idx).unwrap_or(tokens.len());
    if subject_start_token_idx >= subject_end_token_idx {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[subject_start_token_idx..subject_end_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let filter_tokens = if subject_tokens
        .first()
        .is_some_and(|token| token.is_word("each") || token.is_word("all"))
    {
        &subject_tokens[1..]
    } else {
        subject_tokens.as_slice()
    };
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let base_filter = parse_object_filter(filter_tokens, false)?;

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

    if words.get(cursor..cursor + 4) != Some(&["and", "so", "on", "for"][..]) {
        return Ok(None);
    }
    cursor += 4;

    while cursor < words.len() {
        if words[cursor] == "and" {
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

    if verb == "double"
        && let Some(life_total_idx) = words.iter().position(|word| *word == "life")
        && words.get(life_total_idx + 1) == Some(&"total")
        && let Some((player, player_filter)) =
            parse_double_life_total_subject(&words[1..life_total_idx])
        && life_total_idx + 2 == words.len()
    {
        return Ok(Some(vec![EffectAst::subject_verb_set_life_total(
            player,
            Value::Scaled(Box::new(Value::LifeTotal(player_filter)), 2),
        )]));
    }

    let mana_prefix = [
        "double", "the", "amount", "of", "each", "type", "of", "unspent", "mana",
    ];
    if verb == "double"
        && words.starts_with(&mana_prefix)
        && let Some(player) = parse_double_mana_pool_subject(&words[mana_prefix.len()..])
    {
        return Ok(Some(vec![EffectAst::subject_verb_double_mana_pool(player)]));
    }
    if verb == "loses"
        && matches!(
            words.as_slice(),
            ["that", "player", "loses", "all", "unspent", "mana"]
                | ["target", "player", "loses", "all", "unspent", "mana"]
                | ["target", "opponent", "loses", "all", "unspent", "mana"]
                | ["you", "lose", "all", "unspent", "mana"]
        )
    {
        let player = match words.as_slice() {
            ["that", "player", ..] => PlayerAst::That,
            ["target", "opponent", ..] => PlayerAst::TargetOpponent,
            ["you", ..] => PlayerAst::You,
            _ => PlayerAst::Target,
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
    if grammar::words_match_prefix(view.tokens, &["sacrifice", "any", "number", "of"]).is_none() {
        return Ok(None);
    }
    let words = crate::runtime_backend::token_word_refs(view.tokens);
    let Some(then_idx) = words.iter().position(|word| *word == "then") else {
        return Ok(None);
    };
    if words.get(then_idx + 1..) != Some(&["draw", "that", "many", "cards"][..]) {
        return Ok(None);
    }

    let Some(then_token_idx) =
        crate::runtime_backend::token_index_for_word_index(view.tokens, then_idx)
    else {
        return Ok(None);
    };
    let filter_tokens = trim_commas(&view.tokens[4..then_token_idx]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object after 'any number of' (clause: '{}')",
            view.display_text()
        )));
    }
    let filter_words = crate::runtime_backend::token_word_refs(&filter_tokens);
    let filter_text = view.display_text().to_ascii_lowercase();
    let filter = if filter_text.contains("token")
        && filter_words
            .iter()
            .any(|word| matches!(*word, "artifact" | "artifacts"))
        && filter_words
            .iter()
            .any(|word| matches!(*word, "enchantment" | "enchantments"))
    {
        let mut filter = ObjectFilter::default();
        filter.any_of = vec![
            ObjectFilter::artifact().you_control(),
            ObjectFilter::enchantment().you_control(),
            ObjectFilter::default().token().you_control(),
        ];
        filter
    } else {
        parse_object_filter_lexed(&filter_tokens, false)?
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

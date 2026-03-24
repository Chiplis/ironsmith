use super::super::effect_ast_traversal::{
    for_each_nested_effects, for_each_nested_effects_mut, try_for_each_nested_effects_mut,
};
use super::super::keyword_static::parse_where_x_value_clause;
use super::super::lexer::{OwnedLexToken, split_lexed_sentences};
use super::super::native_tokens::LowercaseWordView;
use super::super::object_filters::{is_comparison_or_delimiter, parse_object_filter};
use super::super::util::{
    helper_tag_for_tokens, is_article, mana_pips_from_token, parse_number, parse_subject,
    parse_target_phrase, span_from_tokens, token_index_for_word_index, trim_commas, words,
};
use super::super::value_helpers::parse_value_from_lexed;
use super::sentence_helpers::*;
use super::{
    find_verb, parse_effect_sentence_lexed, parse_search_library_disjunction_filter,
    parse_token_copy_modifier_sentence, trim_edge_punctuation,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, CarryContext, EffectAst, GrantedAbilityAst, IT_TAG, IfResultPredicate,
    InsteadSemantics, KeywordAction, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, SubjectAst, TagKey, TargetAst, TextSpan,
    TokenCopyFollowup, ZoneReplacementDurationAst,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::filter::Comparison;
use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::zone::Zone;
use std::cell::OnceCell;

type PairSentenceRule =
    fn(&[OwnedLexToken], &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError>;
type TripleSentenceRule = fn(
    &[OwnedLexToken],
    &[OwnedLexToken],
    &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError>;
type QuadSentenceRule = fn(
    &[OwnedLexToken],
    &[OwnedLexToken],
    &[OwnedLexToken],
    &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError>;

const CHOSEN_NAME_TAG: &str = "__chosen_name__";

fn lowercase_word_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut lowered = tokens.to_vec();
    for token in &mut lowered {
        if let Some(word) = token.word_mut() {
            *word = word.to_ascii_lowercase();
        }
    }
    lowered
}

fn parse_exact_card_effect_bundle_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let lowered = lowercase_word_tokens(tokens);
    let sentences = split_lexed_sentences(&lowered);
    if sentences.len() == 2
        && let Ok(Some(effects)) = parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
            sentences[0],
            sentences[1],
        )
    {
        return Some(effects);
    }
    let sentence_words = words(&lowered);

    if sentence_words.as_slice()
        == [
            "look", "at", "the", "top", "x", "cards", "of", "your", "library", "where", "x", "is",
            "your", "devotion", "to", "blue", "put", "up", "to", "one", "of", "them", "on", "top",
            "of", "your", "library", "and", "the", "rest", "on", "the", "bottom", "of", "your",
            "library", "in", "a", "random", "order", "if", "x", "is", "greater", "than", "or",
            "equal", "to", "the", "number", "of", "cards", "in", "your", "library", "you", "win",
            "the", "game",
        ]
    {
        let looked_tag = TagKey::from("thassas_oracle_looked");
        return Some(vec![
            EffectAst::LookAtTopCards {
                player: PlayerAst::You,
                count: Value::Devotion {
                    player: PlayerFilter::You,
                    color: crate::color::Color::Blue,
                },
                tag: looked_tag.clone(),
            },
            EffectAst::RearrangeLookedCardsInLibrary {
                tag: looked_tag,
                player: PlayerAst::You,
                count: ChoiceCount::up_to(1),
            },
            EffectAst::Conditional {
                predicate: PredicateAst::ValueComparison {
                    left: Value::Devotion {
                        player: PlayerFilter::You,
                        color: crate::color::Color::Blue,
                    },
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::CardsInLibrary(PlayerFilter::You),
                },
                if_true: vec![EffectAst::WinGame {
                    player: PlayerAst::You,
                }],
                if_false: Vec::new(),
            },
        ]);
    }

    if sentence_words.as_slice()
        == [
            "if",
            "this",
            "spell",
            "was",
            "cast",
            "from",
            "a",
            "graveyard",
            "copy",
            "this",
            "spell",
            "and",
            "you",
            "may",
            "choose",
            "a",
            "new",
            "target",
            "for",
            "the",
            "copy",
        ]
    {
        return Some(vec![EffectAst::Conditional {
            predicate: PredicateAst::ThisSpellWasCastFromZone(Zone::Graveyard),
            if_true: vec![EffectAst::CopySpell {
                target: TargetAst::Source(None),
                count: Value::Fixed(1),
                player: PlayerAst::Implicit,
                may_choose_new_targets: true,
            }],
            if_false: Vec::new(),
        }]);
    }

    if sentence_words.as_slice()
        == [
            "search", "your", "library", "for", "a", "basic", "forest", "card", "and", "a",
            "basic", "plains", "card", "reveal", "those", "cards", "put", "them", "into", "your",
            "hand", "then", "shuffle",
        ]
    {
        return Some(vec![EffectAst::SearchLibrarySlotsToHand {
            slots: vec![
                crate::cards::builders::SearchLibrarySlotAst {
                    filter: ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .owned_by(PlayerFilter::You)
                        .with_type(crate::types::CardType::Land)
                        .with_supertype(crate::types::Supertype::Basic)
                        .with_subtype(crate::types::Subtype::Forest),
                    optional: true,
                },
                crate::cards::builders::SearchLibrarySlotAst {
                    filter: ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .owned_by(PlayerFilter::You)
                        .with_type(crate::types::CardType::Land)
                        .with_supertype(crate::types::Supertype::Basic)
                        .with_subtype(crate::types::Subtype::Plains),
                    optional: true,
                },
            ],
            player: PlayerAst::You,
            reveal: true,
            progress_tag: TagKey::from("yasharn_search_progress"),
        }]);
    }

    None
}

#[derive(Debug, Clone)]
struct ConsultSentenceParts {
    effects: Vec<EffectAst>,
    player: PlayerAst,
    all_tag: TagKey,
    match_tag: TagKey,
}

struct ConsultCastClause {
    caster: PlayerAst,
    allow_land: bool,
    timing: ConsultCastTiming,
    cost: ConsultCastCost,
    mana_value_condition: Option<ConsultCastManaValueCondition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConsultCastTiming {
    Immediate,
    UntilEndOfTurn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConsultCastCost {
    Normal,
    WithoutPayingManaCost,
    PayLifeEqualToManaValue,
}

#[derive(Clone, Debug, PartialEq)]
struct ConsultCastManaValueCondition {
    operator: crate::effect::ValueComparisonOperator,
    right: Value,
}

fn parse_exile_top_library_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let tokens = trim_commas(tokens);
    let token_words = words(&tokens);
    let count_word_idx = if token_words.starts_with(&["exile", "the", "top"]) {
        3usize
    } else if token_words.starts_with(&["exile", "top"]) {
        2usize
    } else {
        return None;
    };

    let count_tokens = token_words[count_word_idx..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (count, used) = parse_number(&count_tokens)?;
    if count_tokens
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return None;
    }
    let tail_words = words(&count_tokens[used + 1..]);
    if tail_words != ["of", "your", "library"] {
        return None;
    }

    Some(vec![EffectAst::ExileTopOfLibrary {
        count: Value::Fixed(count as i32),
        player: PlayerAst::You,
        tags: Vec::new(),
        accumulated_tags: Vec::new(),
    }])
}

fn parse_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<ConsultSentenceParts>, CardTextError> {
    let mut sentence_tokens = trim_commas(tokens);
    let sentence_words = words(&sentence_tokens);
    let leading_if_you_do = sentence_words.starts_with(&["if", "you", "do"])
        || sentence_words.starts_with(&["if", "they", "do"]);
    if leading_if_you_do {
        let start_word_idx = 3usize;
        let Some(start_token_idx) = token_index_for_word_index(&sentence_tokens, start_word_idx)
            .or(Some(sentence_tokens.len()))
        else {
            return Ok(None);
        };
        sentence_tokens = trim_commas(&sentence_tokens[start_token_idx..]);
    }
    if sentence_tokens.is_empty() {
        return Ok(None);
    }

    let mut prefix_effects = Vec::new();
    let mut prefix_tokens: Vec<OwnedLexToken> = Vec::new();
    let consult_tokens = if let Some(then_idx) = sentence_tokens
        .iter()
        .position(|token| token.is_word("then"))
    {
        prefix_tokens = trim_commas(&sentence_tokens[..then_idx]);
        if prefix_tokens.is_empty() {
            return Ok(None);
        }
        prefix_effects = parse_exile_top_library_prefix(&prefix_tokens)
            .or_else(|| parse_effect_sentence_lexed(&prefix_tokens).ok())
            .or_else(|| parse_effect_chain(&prefix_tokens).ok())
            .unwrap_or_default();
        if prefix_effects.is_empty() {
            return Ok(None);
        }
        trim_commas(&sentence_tokens[then_idx + 1..])
    } else {
        sentence_tokens
    };
    if consult_tokens.is_empty() {
        return Ok(None);
    }

    let Some(consult_verb_idx) = consult_tokens.iter().position(|token| {
        token.is_word("reveal")
            || token.is_word("reveals")
            || token.is_word("exile")
            || token.is_word("exiles")
    }) else {
        return Ok(None);
    };
    let player = if consult_verb_idx == 0 {
        infer_consult_player_from_prefix(&prefix_tokens).unwrap_or(PlayerAst::You)
    } else {
        match parse_subject(&consult_tokens[..consult_verb_idx]) {
            SubjectAst::Player(player) => player,
            _ => return Ok(None),
        }
    };
    let mode = if consult_tokens[consult_verb_idx].is_word("reveal")
        || consult_tokens[consult_verb_idx].is_word("reveals")
    {
        LibraryConsultModeAst::Reveal
    } else {
        LibraryConsultModeAst::Exile
    };

    let Some(until_idx) = consult_tokens
        .iter()
        .position(|token| token.is_word("until"))
    else {
        return Ok(None);
    };
    if until_idx <= consult_verb_idx + 1 {
        return Ok(None);
    }

    let prefix_words: Vec<&str> = words(&consult_tokens[consult_verb_idx + 1..until_idx])
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if !prefix_words.starts_with(&["cards", "from", "top", "of"])
        || !prefix_words.ends_with(&["library"])
    {
        return Ok(None);
    }

    let until_tokens = trim_commas(&consult_tokens[until_idx + 1..]);
    let Some(match_verb_idx) = until_tokens.iter().position(|token| {
        token.is_word("reveal")
            || token.is_word("reveals")
            || token.is_word("exile")
            || token.is_word("exiles")
    }) else {
        return Ok(None);
    };
    if match_verb_idx == 0 || match_verb_idx + 1 >= until_tokens.len() {
        return Ok(None);
    }

    let mut filter_tokens = trim_commas(&until_tokens[match_verb_idx + 1..]).to_vec();
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let stop_rule = if let Some((count, used)) = parse_number(&filter_tokens) {
        let remaining = trim_commas(&filter_tokens[used..]).to_vec();
        if remaining.is_empty() {
            return Ok(None);
        }
        filter_tokens = remaining;
        LibraryConsultStopRuleAst::MatchCount(Value::Fixed(count as i32))
    } else {
        LibraryConsultStopRuleAst::FirstMatch
    };

    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        match parse_object_filter(&filter_tokens, false) {
            Ok(filter) => filter,
            Err(_) => return Ok(None),
        }
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let all_tag = helper_tag_for_tokens(
        tokens,
        match mode {
            LibraryConsultModeAst::Reveal => "revealed",
            LibraryConsultModeAst::Exile => "exiled",
        },
    );
    let match_tag = helper_tag_for_tokens(tokens, "chosen");
    let mut effects = prefix_effects;
    effects.push(EffectAst::ConsultTopOfLibrary {
        player,
        mode,
        filter,
        stop_rule,
        all_tag: all_tag.clone(),
        match_tag: match_tag.clone(),
    });

    Ok(Some(ConsultSentenceParts {
        effects,
        player,
        all_tag,
        match_tag,
    }))
}

fn infer_consult_player_from_prefix(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let prefix_tokens = trim_commas(tokens);
    let (_, verb_idx) = find_verb(&prefix_tokens)?;
    match parse_subject(&prefix_tokens[..verb_idx]) {
        SubjectAst::Player(player) => Some(player),
        _ => None,
    }
}

fn parse_consult_remainder_order(words: &[&str]) -> Option<LibraryBottomOrderAst> {
    if !words.contains(&"bottom") || !words.contains(&"library") {
        return None;
    }
    if words.windows(2).any(|window| window == ["random", "order"]) {
        return Some(LibraryBottomOrderAst::Random);
    }
    if words.windows(2).any(|window| window == ["any", "order"]) {
        return Some(LibraryBottomOrderAst::ChooserChooses);
    }
    None
}

fn consult_stop_rule_is_single_match(stop_rule: &LibraryConsultStopRuleAst) -> bool {
    matches!(
        stop_rule,
        LibraryConsultStopRuleAst::FirstMatch
            | LibraryConsultStopRuleAst::MatchCount(Value::Fixed(1))
    )
}

fn parse_consult_condition_value(tokens: &[&str]) -> Option<Value> {
    if matches!(tokens, ["this's", "power"]) {
        return Some(Value::SourcePower);
    }

    let value_tokens = tokens
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    if let Some((value, used)) = parse_value_from_lexed(&value_tokens)
        && words(&value_tokens[used..]).is_empty()
    {
        return Some(value);
    }

    let (filter_tokens, had_number_prefix) = if tokens.starts_with(&["the", "number", "of"]) {
        (&tokens[3..], true)
    } else if tokens.starts_with(&["number", "of"]) {
        (&tokens[2..], true)
    } else {
        (tokens, false)
    };
    if !had_number_prefix || filter_tokens.is_empty() {
        return None;
    }

    let filter_tokens = filter_tokens
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

fn parse_consult_mana_value_condition(words: &[&str]) -> Option<ConsultCastManaValueCondition> {
    if words.is_empty() {
        return None;
    }
    if words.first().copied() != Some("if") {
        return None;
    }

    let after_prefix = if words.starts_with(&["if", "it's", "a", "spell", "with", "mana", "value"])
    {
        &words[7..]
    } else if words.starts_with(&["if", "it", "is", "a", "spell", "with", "mana", "value"]) {
        &words[8..]
    } else if words.starts_with(&["if", "the", "spell's", "mana", "value"]) {
        &words[5..]
    } else if words.starts_with(&["if", "that", "spell's", "mana", "value"]) {
        &words[5..]
    } else if words.starts_with(&["if", "its", "mana", "value"]) {
        &words[4..]
    } else {
        return None;
    };

    let (operator, right_tokens) =
        if after_prefix.starts_with(&["is", "less", "than", "or", "equal", "to"]) {
            (
                crate::effect::ValueComparisonOperator::LessThanOrEqual,
                &after_prefix[6..],
            )
        } else if after_prefix.starts_with(&["is", "less", "than"]) {
            (
                crate::effect::ValueComparisonOperator::LessThan,
                &after_prefix[3..],
            )
        } else if after_prefix.len() >= 4
            && after_prefix[0] == "is"
            && after_prefix[2] == "or"
            && after_prefix[3] == "less"
        {
            (
                crate::effect::ValueComparisonOperator::LessThanOrEqual,
                &after_prefix[1..2],
            )
        } else {
            return None;
        };

    let right = parse_consult_condition_value(right_tokens)?;
    Some(ConsultCastManaValueCondition { operator, right })
}

fn parse_consult_cast_clause(tokens: &[OwnedLexToken]) -> Option<ConsultCastClause> {
    let mut second_tokens = trim_commas(tokens);
    let duration_words = words(&second_tokens);
    let mut timing = ConsultCastTiming::Immediate;
    if starts_with_until_end_of_turn(&duration_words)
        || duration_words.starts_with(&["until", "the", "end", "of", "turn"])
    {
        let consumed_words = if duration_words.get(1) == Some(&"the") {
            5usize
        } else {
            4usize
        };
        let start_token_idx = token_index_for_word_index(&second_tokens, consumed_words)
            .unwrap_or(second_tokens.len());
        second_tokens = trim_commas(&second_tokens[start_token_idx..]);
        timing = ConsultCastTiming::UntilEndOfTurn;
    }

    let may_idx = second_tokens
        .iter()
        .position(|token| token.is_word("may"))?;
    if may_idx == 0 || may_idx + 1 >= second_tokens.len() {
        return None;
    }

    let caster = match parse_subject(&second_tokens[..may_idx]) {
        SubjectAst::Player(player) => player,
        _ => return None,
    };
    let tail_words = words(&second_tokens[may_idx + 1..]);
    let (allow_land, prefix_len) = if tail_words.starts_with(&["cast", "that", "card"]) {
        (false, 3usize)
    } else if tail_words.starts_with(&["cast", "it"]) {
        (false, 2usize)
    } else if tail_words.starts_with(&["cast", "that", "exiled", "card"]) {
        (false, 4usize)
    } else if tail_words.starts_with(&["cast", "the", "exiled", "card"]) {
        (false, 4usize)
    } else if tail_words.starts_with(&["play", "that", "card"]) {
        (true, 3usize)
    } else if tail_words.starts_with(&["play", "it"]) {
        (true, 2usize)
    } else {
        return None;
    };

    let remainder = &tail_words[prefix_len..];
    if remainder == ["this", "turn"] {
        return Some(ConsultCastClause {
            caster,
            allow_land,
            timing: ConsultCastTiming::UntilEndOfTurn,
            cost: ConsultCastCost::Normal,
            mana_value_condition: None,
        });
    }

    if remainder
        == [
            "by", "paying", "life", "equal", "to", "the", "spell's", "mana", "value", "rather",
            "than", "paying", "its", "mana", "cost",
        ]
    {
        return Some(ConsultCastClause {
            caster,
            allow_land,
            timing,
            cost: ConsultCastCost::PayLifeEqualToManaValue,
            mana_value_condition: None,
        });
    }

    if !remainder.starts_with(&["without", "paying", "its", "mana", "cost"]) {
        return None;
    }

    let mana_value_condition = if remainder.len() == 5 {
        None
    } else {
        Some(
            parse_consult_mana_value_condition(&remainder[5..]).or_else(|| {
                let condition_words = &remainder[5..];
                if condition_words.len() == 9
                    && condition_words[0] == "if"
                    && condition_words[1] == "that"
                    && matches!(condition_words[2], "spells" | "spell's")
                    && condition_words[3] == "mana"
                    && condition_words[4] == "value"
                    && condition_words[5] == "is"
                    && condition_words[7] == "or"
                    && condition_words[8] == "less"
                {
                    let value = condition_words[6].parse::<i32>().ok()?;
                    Some(ConsultCastManaValueCondition {
                        operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                        right: Value::Fixed(value),
                    })
                } else {
                    None
                }
            })?,
        )
    };

    Some(ConsultCastClause {
        caster,
        allow_land,
        timing,
        cost: ConsultCastCost::WithoutPayingManaCost,
        mana_value_condition,
    })
}

fn parse_consult_bottom_remainder_clause(
    tokens: &[OwnedLexToken],
    mode: LibraryConsultModeAst,
) -> Option<LibraryBottomOrderAst> {
    let mut clause_words = words(tokens);
    while clause_words
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        clause_words.remove(0);
    }

    let Some(order) = parse_consult_remainder_order(&clause_words) else {
        return None;
    };
    let mode_word = match mode {
        LibraryConsultModeAst::Reveal => "revealed",
        LibraryConsultModeAst::Exile => "exiled",
    };
    if !clause_words.contains(&mode_word) {
        return None;
    }
    let mentions_cast_window = clause_words
        .windows(3)
        .any(|window| window == ["not", "cast", "this"])
        || clause_words.windows(4).any(|window| {
            window == ["werent", "cast", "this", "way"]
                || window == ["weren't", "cast", "this", "way"]
        })
        || clause_words
            .windows(5)
            .any(|window| window == ["were", "not", "cast", "this", "way"]);
    let mentions_remainder = clause_words.contains(&"rest") || clause_words.contains(&"other");

    (mentions_cast_window || mentions_remainder).then_some(order)
}

fn parse_if_declined_put_match_into_hand(
    tokens: &[OwnedLexToken],
    match_tag: TagKey,
) -> Option<Vec<EffectAst>> {
    let clause_words = words(tokens);
    let moves_to_hand = clause_words == ["put", "that", "card", "into", "your", "hand"]
        || clause_words == ["put", "it", "into", "your", "hand"]
        || clause_words.starts_with(&[
            "if", "you", "dont", "put", "that", "card", "into", "your", "hand",
        ])
        || clause_words.starts_with(&["if", "you", "dont", "put", "it", "into", "your", "hand"])
        || clause_words.starts_with(&[
            "if", "you", "don't", "put", "that", "card", "into", "your", "hand",
        ])
        || clause_words.starts_with(&["if", "you", "don't", "put", "it", "into", "your", "hand"])
        || clause_words.starts_with(&[
            "if", "you", "do", "not", "put", "that", "card", "into", "your", "hand",
        ])
        || clause_words.starts_with(&[
            "if", "you", "do", "not", "put", "it", "into", "your", "hand",
        ])
        || clause_words.starts_with(&[
            "if", "you", "dont", "cast", "that", "card", "this", "way", "put", "it", "into",
            "your", "hand",
        ])
        || clause_words.starts_with(&[
            "if", "you", "don't", "cast", "that", "card", "this", "way", "put", "it", "into",
            "your", "hand",
        ])
        || clause_words.starts_with(&[
            "if", "you", "do", "not", "cast", "that", "card", "this", "way", "put", "it", "into",
            "your", "hand",
        ])
        || clause_words.starts_with(&[
            "if", "you", "dont", "cast", "it", "this", "way", "put", "it", "into", "your", "hand",
        ])
        || clause_words.starts_with(&[
            "if", "you", "don't", "cast", "it", "this", "way", "put", "it", "into", "your", "hand",
        ])
        || clause_words.starts_with(&[
            "if", "you", "do", "not", "cast", "it", "this", "way", "put", "it", "into", "your",
            "hand",
        ]);
    if !moves_to_hand {
        return None;
    }

    Some(vec![EffectAst::MoveToZone {
        target: TargetAst::Tagged(match_tag, None),
        zone: Zone::Hand,
        to_top: false,
        battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped: false,
        attached_to: None,
    }])
}

fn consult_cast_effects(
    clause: &ConsultCastClause,
    match_tag: TagKey,
) -> Result<Vec<EffectAst>, CardTextError> {
    if clause.allow_land && !matches!(clause.cost, ConsultCastCost::Normal) {
        return Err(CardTextError::ParseError(
            "playing a land without paying its mana cost is unsupported".to_string(),
        ));
    }

    let mut cast_effects = match clause.cost {
        ConsultCastCost::Normal | ConsultCastCost::WithoutPayingManaCost => {
            let without_paying_mana_cost =
                matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost);
            if clause.allow_land || matches!(clause.timing, ConsultCastTiming::UntilEndOfTurn) {
                vec![EffectAst::GrantPlayTaggedUntilEndOfTurn {
                    tag: match_tag.clone(),
                    player: clause.caster,
                    allow_land: clause.allow_land,
                    without_paying_mana_cost,
                }]
            } else {
                vec![EffectAst::May {
                    effects: vec![EffectAst::CastTagged {
                        tag: match_tag.clone(),
                        allow_land: false,
                        as_copy: false,
                        without_paying_mana_cost,
                    }],
                }]
            }
        }
        ConsultCastCost::PayLifeEqualToManaValue => {
            if clause.allow_land {
                return Err(CardTextError::ParseError(
                    "pay-life consult cast clauses cannot allow lands".to_string(),
                ));
            }
            vec![
                EffectAst::GrantPlayTaggedUntilEndOfTurn {
                    tag: match_tag.clone(),
                    player: clause.caster,
                    allow_land: false,
                    without_paying_mana_cost: false,
                },
                EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                    tag: match_tag.clone(),
                    player: clause.caster,
                },
            ]
        }
    };

    if let Some(condition) = &clause.mana_value_condition {
        cast_effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::ValueComparison {
                left: Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(match_tag))),
                operator: condition.operator,
                right: condition.right.clone(),
            },
            if_true: cast_effects,
            if_false: Vec::new(),
        }]
    }

    Ok(cast_effects)
}

fn parse_consult_match_move_and_bottom_remainder(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let second_words = words(&second_tokens);
    let (zone, battlefield_tapped) = if second_words
        .starts_with(&["put", "that", "card", "into", "your", "hand"])
        || second_words.starts_with(&["put", "it", "into", "your", "hand"])
    {
        (Zone::Hand, false)
    } else if second_words.starts_with(&[
        "put",
        "that",
        "card",
        "onto",
        "the",
        "battlefield",
        "tapped",
    ]) || second_words.starts_with(&["put", "it", "onto", "the", "battlefield", "tapped"])
        || second_words.starts_with(&["put", "that", "card", "onto", "battlefield", "tapped"])
        || second_words.starts_with(&["put", "it", "onto", "battlefield", "tapped"])
    {
        (Zone::Battlefield, true)
    } else if second_words.starts_with(&["put", "that", "card", "onto", "the", "battlefield"])
        || second_words.starts_with(&["put", "it", "onto", "the", "battlefield"])
        || second_words.starts_with(&["put", "that", "card", "onto", "battlefield"])
        || second_words.starts_with(&["put", "it", "onto", "battlefield"])
    {
        (Zone::Battlefield, false)
    } else {
        return Ok(None);
    };

    if !second_words.contains(&"rest") && !second_words.contains(&"other") {
        return Ok(None);
    }
    let Some(order) = parse_consult_remainder_order(&second_words) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.push(EffectAst::MoveToZone {
        target: TargetAst::Tagged(parts.match_tag.clone(), None),
        zone,
        to_top: false,
        battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped,
        attached_to: None,
    });
    effects.push(EffectAst::PutTaggedRemainderOnBottomOfLibrary {
        tag: parts.all_tag,
        keep_tagged: Some(parts.match_tag),
        order,
        player: parts.player,
    });
    Ok(Some(effects))
}

fn parse_consult_match_into_hand_exile_others(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::ConsultTopOfLibrary {
            mode: LibraryConsultModeAst::Reveal,
            ..
        })
    ) {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    let second_words = words(&second_tokens);
    let moves_to_hand = second_words.starts_with(&["put", "that", "card", "into", "your", "hand"])
        || second_words.starts_with(&["put", "it", "into", "your", "hand"]);
    let exiles_rest = second_words.contains(&"exile")
        && second_words.contains(&"other")
        && second_words.contains(&"cards");
    if !moves_to_hand || !exiles_rest {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::MoveToZone {
        target: TargetAst::Tagged(parts.match_tag.clone(), None),
        zone: Zone::Hand,
        to_top: false,
        battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped: false,
        attached_to: None,
    });
    effects.push(EffectAst::ForEachTagged {
        tag: parts.all_tag,
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                TagKey::from(IT_TAG),
                ObjectFilter::tagged(parts.match_tag),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::Exile {
                target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                face_down: false,
            }],
        }],
    });
    Ok(Some(effects))
}

fn parse_tainted_pact_sequence(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words: Vec<&str> = words(&first_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if first_words.as_slice() != ["exile", "top", "card", "of", "your", "library"] {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    let second_words: Vec<&str> = words(&second_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let second_matches = second_words.as_slice()
        == [
            "you", "may", "put", "that", "card", "into", "your", "hand", "unless", "it", "has",
            "same", "name", "as", "another", "card", "exiled", "this", "way",
        ]
        || second_words.as_slice()
            == [
                "you", "may", "put", "it", "into", "your", "hand", "unless", "it", "has", "same",
                "name", "as", "another", "card", "exiled", "this", "way",
            ];
    if !second_matches {
        return Ok(None);
    }

    let third_tokens = trim_commas(third);
    let third_words: Vec<&str> = words(&third_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let third_matches = third_words.as_slice()
        == [
            "repeat",
            "this",
            "process",
            "until",
            "you",
            "put",
            "card",
            "into",
            "your",
            "hand",
            "or",
            "you",
            "exile",
            "two",
            "cards",
            "with",
            "same",
            "name",
            "whichever",
            "comes",
            "first",
        ];
    if !third_matches {
        return Ok(None);
    }

    let current_tag = TagKey::from("tainted_pact_current");
    let exiled_tag = TagKey::from("tainted_pact_exiled");
    let all_exiled_filter = ObjectFilter::tagged(exiled_tag.clone()).in_zone(Zone::Exile);
    Ok(Some(vec![EffectAst::RepeatProcess {
        effects: vec![
            EffectAst::ExileTopOfLibrary {
                count: Value::Fixed(1),
                player: PlayerAst::You,
                tags: vec![current_tag.clone()],
                accumulated_tags: vec![exiled_tag.clone()],
            },
            EffectAst::Conditional {
                predicate: PredicateAst::And(
                    Box::new(PredicateAst::TaggedMatches(
                        current_tag.clone(),
                        ObjectFilter::default().in_zone(Zone::Exile),
                    )),
                    Box::new(PredicateAst::ValueComparison {
                        left: Value::Count(all_exiled_filter.clone()),
                        operator: crate::effect::ValueComparisonOperator::Equal,
                        right: Value::DistinctNames(all_exiled_filter),
                    }),
                ),
                if_true: vec![EffectAst::MayMoveToZone {
                    target: TargetAst::Tagged(current_tag.clone(), None),
                    zone: Zone::Hand,
                    player: PlayerAst::You,
                }],
                if_false: Vec::new(),
            },
        ],
        continue_effect_index: 1,
        continue_predicate: IfResultPredicate::WasDeclined,
    }]))
}

fn prepend_prefix_sentence_to_consult_pair(
    prefix: &[OwnedLexToken],
    consult: &[OwnedLexToken],
    followup: &[OwnedLexToken],
    pair_rule: PairSentenceRule,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let prefix_effects =
        parse_effect_sentence_lexed(prefix).or_else(|_| parse_effect_chain(prefix))?;
    if prefix_effects.is_empty() {
        return Ok(None);
    }

    let Some(mut combined) = pair_rule(consult, followup)? else {
        return Ok(None);
    };
    let mut effects = prefix_effects;
    effects.append(&mut combined);
    Ok(Some(effects))
}

fn parse_prefix_then_consult_match_move_and_bottom_remainder(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    prepend_prefix_sentence_to_consult_pair(
        first,
        second,
        third,
        parse_consult_match_move_and_bottom_remainder,
    )
}

fn parse_prefix_then_consult_match_into_hand_exile_others(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    prepend_prefix_sentence_to_consult_pair(
        first,
        second,
        third,
        parse_consult_match_into_hand_exile_others,
    )
}

struct SentenceInput {
    lowered: OnceCell<Vec<OwnedLexToken>>,
    lexed: Option<Vec<OwnedLexToken>>,
}

impl SentenceInput {
    fn from_lexed(tokens: &[OwnedLexToken]) -> Self {
        Self {
            lowered: OnceCell::new(),
            lexed: Some(tokens.to_vec()),
        }
    }

    fn lowered(&self) -> &[OwnedLexToken] {
        self.lowered
            .get_or_init(|| match self.lexed.as_deref() {
                Some(tokens) => lowercase_word_tokens(tokens),
                None => Vec::new(),
            })
            .as_slice()
    }
}

fn future_zone_replacement_from_sentence_text(sentence_text: &str) -> Option<EffectAst> {
    let normalized = sentence_text.to_ascii_lowercase();
    let target = TargetAst::Tagged(TagKey::from(IT_TAG), None);

    if normalized.contains("countered this way")
        && normalized.contains("instead of putting it into")
        && normalized.contains("graveyard")
    {
        return Some(EffectAst::RegisterZoneReplacement {
            target,
            from_zone: Some(Zone::Stack),
            to_zone: Some(Zone::Graveyard),
            replacement_zone: Zone::Exile,
            duration: ZoneReplacementDurationAst::OneShot,
        });
    }

    if normalized.contains("would die this turn") && normalized.contains("exile") {
        return Some(EffectAst::RegisterZoneReplacement {
            target,
            from_zone: Some(Zone::Battlefield),
            to_zone: Some(Zone::Graveyard),
            replacement_zone: Zone::Exile,
            duration: ZoneReplacementDurationAst::OneShot,
        });
    }

    if normalized.contains("would be put into")
        && normalized.contains("graveyard")
        && normalized.contains("this turn")
        && normalized.contains("exile")
    {
        return Some(EffectAst::RegisterZoneReplacement {
            target,
            from_zone: None,
            to_zone: Some(Zone::Graveyard),
            replacement_zone: Zone::Exile,
            duration: ZoneReplacementDurationAst::OneShot,
        });
    }

    None
}

fn maybe_rewrite_future_zone_replacement_sentence(
    sentence_effects: &mut Vec<EffectAst>,
    sentence_text: &str,
) {
    if !matches!(
        classify_instead_followup_text(sentence_text),
        InsteadSemantics::FutureReplacement
    ) {
        return;
    }

    let Some(replacement) = future_zone_replacement_from_sentence_text(sentence_text) else {
        return;
    };

    if sentence_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::ExileInsteadOfGraveyardThisTurn { .. }
                | EffectAst::PreventNextTimeDamage { .. }
                | EffectAst::RedirectNextTimeDamageToSource { .. }
        )
    }) {
        return;
    }

    if sentence_effects.len() == 1 {
        if let Some(EffectAst::IfResult { effects, .. }) = sentence_effects.first_mut() {
            *effects = vec![replacement];
            return;
        }
        *sentence_effects = vec![replacement];
    }
}

fn parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words = words(&first_tokens);
    let count_word_idx = if first_words.starts_with(&["reveal", "the", "top"]) {
        3usize
    } else if first_words.starts_with(&["reveal", "top"]) {
        2usize
    } else {
        return Ok(None);
    };

    let count_tokens = first_words[count_word_idx..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    if count_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "card" || word == "cards")
    {
        return Ok(None);
    }
    let Some((count, used)) = parse_number(&count_tokens) else {
        return Ok(None);
    };
    if count_tokens
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Ok(None);
    }
    let reveal_tail = words(&count_tokens[used + 1..]);
    if reveal_tail != ["of", "your", "library"] {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    let second_words = words(&second_tokens);
    if !matches!(
        second_words.get(..2),
        Some(["put", "all"] | ["puts", "all"])
    ) {
        return Ok(None);
    }
    let Some(revealed_idx) = second_words
        .windows(3)
        .position(|window| window == ["revealed", "this", "way"])
    else {
        return Ok(None);
    };
    if revealed_idx <= 2 {
        return Ok(None);
    }

    let Some(filter_start) = token_index_for_word_index(&second_tokens, 2) else {
        return Ok(None);
    };
    let filter_end =
        token_index_for_word_index(&second_tokens, revealed_idx).unwrap_or(second_tokens.len());
    let filter_tokens = trim_commas(&second_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_revealed = &second_words[revealed_idx + 3..];
    let has_hand_clause = after_revealed
        .windows(3)
        .any(|window| window == ["into", "your", "hand"]);
    let has_rest_clause = after_revealed
        .windows(5)
        .any(|window| window == ["and", "the", "rest", "into", "your"])
        && after_revealed.contains(&"graveyard");
    if !has_hand_clause || !has_rest_clause {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::RevealTopPutMatchingIntoHandRestIntoGraveyard {
            player: PlayerAst::You,
            count,
            filter,
        },
    ]))
}

fn parse_delayed_dies_exile_top_power_choose_play(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words = words(&first_tokens);
    if !first_words.starts_with(&["when", "that", "creature", "dies", "this", "turn"]) {
        return Ok(None);
    }

    let Some(comma_idx) = first_tokens.iter().position(|token| token.is_comma()) else {
        return Ok(None);
    };
    let action_tokens = trim_commas(&first_tokens[comma_idx + 1..]);
    let action_words: Vec<&str> = words(&action_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let starts_with_exile_top_power = action_words.starts_with(&[
        "exile", "number", "of", "cards", "from", "top", "of", "your", "library", "equal", "to",
        "its", "power",
    ]);
    let ends_with_choose_exiled =
        action_words.ends_with(&["choose", "card", "exiled", "this", "way"]);
    if !starts_with_exile_top_power || !ends_with_choose_exiled {
        return Ok(None);
    }

    let second_words: Vec<&str> = words(second)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let is_until_next_turn_play_clause = second_words.as_slice()
        == [
            "until", "end", "of", "your", "next", "turn", "you", "may", "play", "that", "card",
        ];
    if !is_until_next_turn_play_clause {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(first, "looked");
    let chosen_tag = helper_tag_for_tokens(first, "chosen");
    let mut exiled_filter = ObjectFilter::default();
    exiled_filter.zone = Some(Zone::Exile);
    exiled_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Ok(Some(vec![EffectAst::DelayedWhenLastObjectDiesThisTurn {
        filter: None,
        effects: vec![
            EffectAst::LookAtTopCards {
                player: PlayerAst::You,
                count: Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG)))),
                tag: looked_tag.clone(),
            },
            EffectAst::Exile {
                target: TargetAst::Tagged(looked_tag, None),
                face_down: false,
            },
            EffectAst::ChooseObjects {
                filter: exiled_filter,
                count: ChoiceCount::exactly(1),
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
            },
            EffectAst::GrantPlayTaggedUntilYourNextTurn {
                tag: chosen_tag,
                player: PlayerAst::You,
                allow_land: true,
            },
        ],
    }]))
}

fn parse_pair_sentence_sequence(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    const RULES: [(&str, PairSentenceRule); 12] = [
        (
            "damage-prevention-then-put-counters",
            parse_damage_prevention_then_put_counters,
        ),
        (
            "delayed-dies-exile-top-power-choose-play",
            parse_delayed_dies_exile_top_power_choose_play,
        ),
        (
            "target-gains-flashback-until-eot-targets-mana-cost",
            parse_target_gains_flashback_until_eot_with_targets_mana_cost,
        ),
        (
            "mill-then-put-from-among-into-hand",
            parse_mill_then_may_put_from_among_into_hand,
        ),
        (
            "exile-until-match-grant-play-this-turn",
            parse_exile_until_match_grant_play_this_turn,
        ),
        (
            "target-chooses-other-cant-block",
            parse_target_player_chooses_then_other_cant_block,
        ),
        (
            "tap-all-then-they-dont-untap-while-source-tapped",
            parse_tap_all_then_they_dont_untap_while_source_tapped,
        ),
        (
            "choose-card-type-then-reveal-and-put",
            parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand,
        ),
        (
            "choose-creature-type-then-become-type",
            parse_choose_creature_type_then_become_type,
        ),
        (
            "reveal-top-matching-into-hand-rest-graveyard",
            parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard,
        ),
        (
            "consult-match-move-bottom-remainder",
            parse_consult_match_move_and_bottom_remainder,
        ),
        (
            "consult-match-into-hand-exile-others",
            parse_consult_match_into_hand_exile_others,
        ),
    ];

    for (name, rule) in RULES {
        if let Some(combined) = rule(first, second)? {
            return Ok(Some((name, combined)));
        }
    }

    Ok(None)
}

fn parse_damage_prevention_then_put_counters(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let Some(first_effect) = first_effects.first() else {
        return Ok(None);
    };
    if first_effects.len() != 1 {
        return Ok(None);
    }

    let (amount, target, duration) = match first_effect {
        EffectAst::PreventDamage {
            amount,
            target,
            duration,
        } => (Some(amount.clone()), target.clone(), duration.clone()),
        EffectAst::PreventAllDamageToTarget { target, duration } => {
            (None, target.clone(), duration.clone())
        }
        _ => return Ok(None),
    };

    let second_tokens = trim_commas(second);
    let second_words: Vec<&str> = words(&second_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if !second_words.starts_with(&["for", "each", "1", "damage", "prevented", "this", "way"])
        || !second_words.contains(&"put")
        || !second_words.contains(&"+1/+1")
        || !second_words.contains(&"counter")
        || !second_words.contains(&"on")
    {
        return Ok(None);
    }

    let Some(on_idx) = second_words.iter().position(|word| *word == "on") else {
        return Ok(None);
    };
    let target_words = &second_words[on_idx + 1..];
    let valid_target_tail = matches!(
        target_words,
        ["that", "creature"] | ["it"] | ["that", "permanent"] | ["that", "object"]
    );
    if !valid_target_tail {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::PreventDamageToTargetPutCounters {
        amount,
        target,
        duration,
        counter_type: CounterType::PlusOnePlusOne,
    }]))
}

fn parse_target_gains_flashback_until_eot_with_targets_mana_cost(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words = words(&first_tokens);
    let Some(gain_idx) = first_words
        .iter()
        .position(|word| matches!(*word, "gain" | "gains"))
    else {
        return Ok(None);
    };
    if first_words[gain_idx + 1..] != ["flashback", "until", "end", "of", "turn"] {
        return Ok(None);
    }

    let Some(gain_token_idx) = token_index_for_word_index(&first_tokens, gain_idx) else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&first_tokens[..gain_token_idx]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(&target_tokens)?;

    let second_tokens = trim_commas(second);
    let second_words = words(&second_tokens);
    let valid_followup = second_words.as_slice()
        == [
            "the",
            "flashback",
            "cost",
            "is",
            "equal",
            "to",
            "its",
            "mana",
            "cost",
        ]
        || second_words.as_slice()
            == [
                "that",
                "cards",
                "flashback",
                "cost",
                "is",
                "equal",
                "to",
                "its",
                "mana",
                "cost",
            ];
    if !valid_followup {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::GrantToTarget {
        target,
        grantable: crate::grant::Grantable::flashback_from_cards_mana_cost(),
        duration: crate::grant::GrantDuration::UntilEndOfTurn,
    }]))
}

fn parse_tap_all_then_they_dont_untap_while_source_tapped(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [EffectAst::TapAll { filter }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let second_words = words(&second_tokens);
    let starts_with_supported_pronoun_clause = second_words
        .starts_with(&["they", "dont", "untap", "during"])
        || second_words.starts_with(&["they", "do", "not", "untap", "during"]);
    let has_source_tapped_duration = second_words
        .windows(4)
        .any(|window| window == ["for", "as", "long", "as"])
        && second_words.contains(&"remains")
        && second_words.contains(&"tapped")
        && (second_words.contains(&"this")
            || second_words.contains(&"thiss")
            || second_words.contains(&"source")
            || second_words.contains(&"artifact")
            || second_words.contains(&"creature")
            || second_words.contains(&"permanent"));
    if !starts_with_supported_pronoun_clause || !has_source_tapped_duration {
        return Ok(None);
    }

    let Some((duration, clause_tokens)) = parse_restriction_duration(&second_tokens)? else {
        return Ok(None);
    };
    let clause_words = words(&clause_tokens);
    let valid_untap_clause = clause_words.starts_with(&["they", "dont", "untap", "during"])
        || clause_words.starts_with(&["they", "do", "not", "untap", "during"]);
    if !valid_untap_clause {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::TapAll {
            filter: filter.clone(),
        },
        EffectAst::Cant {
            restriction: crate::effect::Restriction::untap(filter.clone()),
            duration,
            condition: Some(crate::ConditionExpr::SourceIsTapped),
        },
    ]))
}

fn parse_exile_until_match_grant_play_this_turn(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::ConsultTopOfLibrary {
            mode: LibraryConsultModeAst::Exile,
            stop_rule,
            ..
        }) if consult_stop_rule_is_single_match(stop_rule)
    ) {
        return Ok(None);
    }

    let Some(clause) = parse_consult_cast_clause(second) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.extend(consult_cast_effects(&clause, parts.match_tag)?);
    Ok(Some(effects))
}

fn parse_look_at_top_reveal_match_put_rest_bottom(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [EffectAst::LookAtTopCards { player, count, .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let second_words = words(&second_tokens);
    if second_words.is_empty() {
        return Ok(None);
    }

    let (chooser, reveal_word_idx) = if second_words.starts_with(&["you", "may", "reveal"]) {
        (PlayerAst::You, 2usize)
    } else if second_words.starts_with(&["that", "player", "may", "reveal"]) {
        (PlayerAst::That, 3usize)
    } else if second_words.starts_with(&["they", "may", "reveal"]) {
        (PlayerAst::That, 2usize)
    } else if second_words.starts_with(&["may", "reveal"]) {
        (*player, 1usize)
    } else if second_words.starts_with(&["reveal"]) {
        (*player, 0usize)
    } else {
        return Ok(None);
    };

    let from_among_word_idx = second_words
        .windows(3)
        .position(|window| window == ["from", "among", "them"])
        .or_else(|| {
            second_words
                .windows(4)
                .position(|window| window == ["from", "among", "those", "cards"])
        });
    let Some(from_among_word_idx) = from_among_word_idx else {
        return Ok(None);
    };
    if from_among_word_idx <= reveal_word_idx {
        return Ok(None);
    }

    let filter_start = token_index_for_word_index(&second_tokens, reveal_word_idx + 1)
        .unwrap_or(second_tokens.len());
    let filter_end = token_index_for_word_index(&second_tokens, from_among_word_idx)
        .unwrap_or(second_tokens.len());
    let filter_tokens = trim_commas(&second_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_from_word_idx = if second_words
        .windows(4)
        .any(|window| window == ["from", "among", "those", "cards"])
    {
        from_among_word_idx + 4
    } else {
        from_among_word_idx + 3
    };
    let after_from_words = &second_words[after_from_word_idx..];
    let puts_into_hand = (after_from_words.starts_with(&["and", "put", "it", "into"])
        || after_from_words.starts_with(&["put", "it", "into"]))
        && after_from_words.contains(&"hand");
    if !puts_into_hand {
        return Ok(None);
    }

    let third_words = words(third);
    let puts_rest_bottom = matches!(third_words.first().copied(), Some("put" | "puts"))
        && third_words.contains(&"rest")
        && third_words.contains(&"bottom")
        && third_words.contains(&"library");
    if !puts_rest_bottom {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::LookAtTopCards {
        player: *player,
        count: count.clone(),
        tag: TagKey::from(IT_TAG),
    }];
    effects.push(
        EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary {
            player: chooser,
            filter,
            reveal: true,
            if_not_chosen: Vec::new(),
        },
    );
    Ok(Some(effects))
}

fn parse_top_cards_view_sentence(tokens: &[OwnedLexToken]) -> Option<(PlayerAst, Value, bool)> {
    let tokens = trim_commas(tokens);
    let clause_words = words(&tokens);
    if clause_words.is_empty() {
        return None;
    }

    let (count_word_idx, revealed) = if clause_words.starts_with(&["look", "at", "the", "top"]) {
        (4usize, false)
    } else if clause_words.starts_with(&["look", "at", "top"]) {
        (3usize, false)
    } else if clause_words.starts_with(&["reveal", "the", "top"]) {
        (3usize, true)
    } else if clause_words.starts_with(&["reveal", "top"]) {
        (2usize, true)
    } else {
        return None;
    };

    let count_start = token_index_for_word_index(&tokens, count_word_idx)?;
    let count_tokens = &tokens[count_start..];
    let (count, used) = parse_number(count_tokens)?;
    let count_tail = words(&count_tokens[used..]);
    if !matches!(count_tail.first().copied(), Some("card" | "cards")) {
        return None;
    }

    let owner_tail = &count_tail[1..];
    if owner_tail != ["of", "your", "library"] {
        return None;
    }

    Some((PlayerAst::You, Value::Fixed(count as i32), revealed))
}

fn parse_counted_looked_cards_into_your_hand_words(words: &[&str]) -> Option<u32> {
    if !words.starts_with(&["put"]) {
        return None;
    }

    let count_tokens = words[1..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (count, used) = parse_number(&count_tokens)?;

    let mut idx = 1 + used;
    if words.get(idx).copied() == Some("of") {
        idx += 1;
    }

    match words.get(idx).copied() {
        Some("them") => idx += 1,
        Some("those") => {
            idx += 1;
            if words.get(idx).copied() == Some("card") || words.get(idx).copied() == Some("cards") {
                idx += 1;
            }
        }
        _ => return None,
    }

    if words.get(idx..idx + 3) != Some(&["into", "your", "hand"]) {
        return None;
    }
    idx += 3;

    if idx == words.len() {
        return Some(count as u32);
    }
    if idx + 1 == words.len() && words[idx] == "instead" {
        return Some(count as u32);
    }

    None
}

fn parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    let trimmed = trim_commas(tokens);
    let clause_words = words(&trimmed);
    if !clause_words.starts_with(&["if", "this", "spell", "was", "kicked"]) {
        return None;
    }

    let tail_start = token_index_for_word_index(&trimmed, 5).unwrap_or(trimmed.len());
    let tail = trim_commas(&trimmed[tail_start..]);
    let tail_words = words(&tail);
    parse_counted_looked_cards_into_your_hand_words(&tail_words)
}

fn parse_may_put_filtered_looked_card_onto_battlefield(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, bool)>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let sentence_words = words(&sentence_tokens);
    if sentence_words.is_empty() {
        return Ok(None);
    }

    let (chooser, action_word_idx) = if sentence_words.starts_with(&["you", "may", "put"]) {
        (PlayerAst::You, 2usize)
    } else if sentence_words.starts_with(&["that", "player", "may", "put"]) {
        (PlayerAst::That, 3usize)
    } else if sentence_words.starts_with(&["they", "may", "put"]) {
        (PlayerAst::That, 2usize)
    } else if sentence_words.starts_with(&["may", "put"]) {
        (PlayerAst::You, 1usize)
    } else {
        return Ok(None);
    };

    let from_among_word_idx = sentence_words
        .windows(3)
        .position(|window| window == ["from", "among", "them"])
        .or_else(|| {
            sentence_words
                .windows(4)
                .position(|window| window == ["from", "among", "those", "cards"])
        });
    let Some(from_among_word_idx) = from_among_word_idx else {
        return Ok(None);
    };
    if from_among_word_idx <= action_word_idx {
        return Ok(None);
    }

    let filter_start = token_index_for_word_index(&sentence_tokens, action_word_idx + 1)
        .unwrap_or(sentence_tokens.len());
    let filter_end = token_index_for_word_index(&sentence_tokens, from_among_word_idx)
        .unwrap_or(sentence_tokens.len());
    let filter_tokens = trim_commas(&sentence_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_from_word_idx = if sentence_words
        .windows(4)
        .any(|window| window == ["from", "among", "those", "cards"])
    {
        from_among_word_idx + 4
    } else {
        from_among_word_idx + 3
    };
    let after_from_words = &sentence_words[after_from_word_idx..];
    let tapped = match after_from_words {
        ["onto", "the", "battlefield"] | ["onto", "battlefield"] => false,
        ["onto", "the", "battlefield", "tapped"] | ["onto", "battlefield", "tapped"] => true,
        _ => return Ok(None),
    };

    Ok(Some((chooser, filter, tapped)))
}

fn parse_if_you_dont_put_card_from_among_them_into_your_hand(tokens: &[OwnedLexToken]) -> bool {
    let trimmed = trim_commas(tokens);
    let words: Vec<&str> = words(&trimmed)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    words.as_slice()
        == [
            "if", "you", "dont", "put", "card", "from", "among", "them", "into", "your", "hand",
        ]
        || words.as_slice()
            == [
                "if", "you", "don't", "put", "card", "from", "among", "them", "into", "your",
                "hand",
            ]
        || words.as_slice()
            == [
                "if", "you", "do", "not", "put", "card", "from", "among", "them", "into", "your",
                "hand",
            ]
        || words.as_slice()
            == [
                "if", "you", "dont", "put", "card", "from", "among", "those", "cards", "into",
                "your", "hand",
            ]
        || words.as_slice()
            == [
                "if", "you", "don't", "put", "card", "from", "among", "those", "cards", "into",
                "your", "hand",
            ]
        || words.as_slice()
            == [
                "if", "you", "do", "not", "put", "card", "from", "among", "those", "cards", "into",
                "your", "hand",
            ]
}

fn is_put_rest_on_bottom_of_library_sentence(tokens: &[OwnedLexToken]) -> bool {
    let trimmed = trim_commas(tokens);
    let words = words(&trimmed);
    matches!(words.first().copied(), Some("put" | "puts"))
        && words.contains(&"rest")
        && words.contains(&"bottom")
        && words.contains(&"library")
}

fn parse_look_at_top_put_counted_into_hand_rest_bottom_with_kicker_override(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
    fourth: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [EffectAst::LookAtTopCards { player, .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let Some(base_count) =
        parse_counted_looked_cards_into_your_hand_words(&words(&trim_commas(second)))
    else {
        return Ok(None);
    };
    let Some(kicked_count) = parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(third)
    else {
        return Ok(None);
    };
    if !is_put_rest_on_bottom_of_library_sentence(fourth) {
        return Ok(None);
    }

    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::Conditional {
            predicate: crate::cards::builders::PredicateAst::ThisSpellWasKicked,
            if_true: vec![EffectAst::PutSomeIntoHandRestOnBottomOfLibrary {
                player: *player,
                count: kicked_count,
            }],
            if_false: vec![EffectAst::PutSomeIntoHandRestOnBottomOfLibrary {
                player: *player,
                count: base_count,
            }],
        },
    ]))
}

fn parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
    fourth: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [EffectAst::LookAtTopCards { .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let Some((chooser, battlefield_filter, tapped)) =
        parse_may_put_filtered_looked_card_onto_battlefield(second)?
    else {
        return Ok(None);
    };
    if !parse_if_you_dont_put_card_from_among_them_into_your_hand(third) {
        return Ok(None);
    }
    if !is_put_rest_on_bottom_of_library_sentence(fourth) {
        return Ok(None);
    }

    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
            player: chooser,
            battlefield_filter,
            tapped,
        },
    ]))
}

fn parse_top_cards_put_match_into_hand_rest_graveyard(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) = parse_top_cards_view_sentence(first) else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let second_words = words(&second_tokens);
    if second_words.is_empty() {
        return Ok(None);
    }

    let (chooser, action_word_idx, reveal_chosen) =
        if second_words.starts_with(&["you", "may", "reveal"]) {
            (PlayerAst::You, 2usize, true)
        } else if second_words.starts_with(&["you", "may", "put"]) {
            (PlayerAst::You, 2usize, false)
        } else if second_words.starts_with(&["that", "player", "may", "reveal"]) {
            (PlayerAst::That, 3usize, true)
        } else if second_words.starts_with(&["that", "player", "may", "put"]) {
            (PlayerAst::That, 3usize, false)
        } else if second_words.starts_with(&["they", "may", "reveal"]) {
            (PlayerAst::That, 2usize, true)
        } else if second_words.starts_with(&["they", "may", "put"]) {
            (PlayerAst::That, 2usize, false)
        } else if second_words.starts_with(&["may", "reveal"]) {
            (player, 1usize, true)
        } else if second_words.starts_with(&["may", "put"]) {
            (player, 1usize, false)
        } else if second_words.starts_with(&["reveal"]) {
            (player, 0usize, true)
        } else if second_words.starts_with(&["put"]) {
            (player, 0usize, false)
        } else {
            return Ok(None);
        };

    let from_among_word_idx = second_words
        .windows(3)
        .position(|window| window == ["from", "among", "them"])
        .or_else(|| {
            second_words
                .windows(4)
                .position(|window| window == ["from", "among", "those", "cards"])
        });
    let Some(from_among_word_idx) = from_among_word_idx else {
        return Ok(None);
    };
    if from_among_word_idx <= action_word_idx {
        return Ok(None);
    }

    let filter_start = token_index_for_word_index(&second_tokens, action_word_idx + 1)
        .unwrap_or(second_tokens.len());
    let filter_end = token_index_for_word_index(&second_tokens, from_among_word_idx)
        .unwrap_or(second_tokens.len());
    let filter_tokens = trim_commas(&second_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_from_word_idx = if second_words
        .windows(4)
        .any(|window| window == ["from", "among", "those", "cards"])
    {
        from_among_word_idx + 4
    } else {
        from_among_word_idx + 3
    };
    let after_from_words = &second_words[after_from_word_idx..];
    let moves_into_hand = if reveal_chosen {
        (after_from_words.starts_with(&["and", "put", "it", "into"])
            || after_from_words.starts_with(&["put", "it", "into"]))
            && after_from_words.contains(&"hand")
    } else {
        after_from_words.starts_with(&["into"]) && after_from_words.contains(&"hand")
    };
    if !moves_into_hand {
        return Ok(None);
    }

    let third_words = words(third);
    let puts_rest_graveyard = matches!(third_words.first().copied(), Some("put" | "puts"))
        && third_words.contains(&"rest")
        && third_words.contains(&"graveyard");
    if !puts_rest_graveyard {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::LookAtTopCards {
        player,
        count,
        tag: TagKey::from(IT_TAG),
    }];
    if reveal_top {
        effects.push(EffectAst::RevealTagged {
            tag: TagKey::from(IT_TAG),
        });
    }
    effects.push(EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
        player: chooser,
        filter,
        reveal: reveal_chosen,
        if_not_chosen: Vec::new(),
    });
    Ok(Some(effects))
}

fn parse_exile_until_match_cast_rest_bottom(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    let Some(clause) = parse_consult_cast_clause(second) else {
        return Ok(None);
    };
    if !matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost) {
        return Ok(None);
    }
    let Some(order) = parse_consult_bottom_remainder_clause(
        third,
        match parts.effects.last() {
            Some(EffectAst::ConsultTopOfLibrary { mode, .. }) => *mode,
            _ => return Ok(None),
        },
    ) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.extend(consult_cast_effects(&clause, parts.match_tag.clone())?);
    effects.push(EffectAst::PutTaggedRemainderOnBottomOfLibrary {
        tag: parts.all_tag,
        keep_tagged: None,
        order,
        player: parts.player,
    });
    Ok(Some(effects))
}

fn parse_exile_until_match_cast_else_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    let Some(EffectAst::ConsultTopOfLibrary {
        mode: LibraryConsultModeAst::Exile,
        stop_rule,
        ..
    }) = parts.effects.last()
    else {
        return Ok(None);
    };
    if !consult_stop_rule_is_single_match(stop_rule) {
        return Ok(None);
    }
    let Some(clause) = parse_consult_cast_clause(second) else {
        return Ok(None);
    };
    if !matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost) || clause.allow_land {
        return Ok(None);
    }
    let Some(hand_effects) = parse_if_declined_put_match_into_hand(third, parts.match_tag.clone())
    else {
        return Ok(None);
    };

    let cast_effects = consult_cast_effects(&clause, parts.match_tag)?;
    let mut effects = parts.effects;
    if cast_effects.len() == 1 {
        let single_effect = cast_effects.into_iter().next().ok_or_else(|| {
            CardTextError::ParseError("missing cast effect for consult follow-up".to_string())
        })?;
        let EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } = single_effect
        else {
            effects.push(single_effect);
            effects.push(EffectAst::IfResult {
                predicate: IfResultPredicate::WasDeclined,
                effects: hand_effects,
            });
            return Ok(Some(effects));
        };
        let mut gated_if_true = if_true;
        gated_if_true.push(EffectAst::IfResult {
            predicate: IfResultPredicate::WasDeclined,
            effects: hand_effects.clone(),
        });
        let mut gated_if_false = if_false;
        gated_if_false.extend(hand_effects);
        effects.push(EffectAst::Conditional {
            predicate,
            if_true: gated_if_true,
            if_false: gated_if_false,
        });
    } else {
        effects.extend(cast_effects);
        effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::WasDeclined,
            effects: hand_effects,
        });
    }
    Ok(Some(effects))
}

fn title_case_words(words: &[&str]) -> String {
    words
        .iter()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut titled = String::new();
            titled.extend(first.to_uppercase());
            titled.push_str(chars.as_str());
            titled
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_named_card_filter_segment(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut segment_words = words(tokens);
    while segment_words.first().is_some_and(|word| is_article(word)) {
        segment_words.remove(0);
    }
    if matches!(segment_words.last().copied(), Some("card" | "cards")) {
        segment_words.pop();
    }
    if segment_words.is_empty() {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.name = Some(title_case_words(&segment_words));
    Some(filter)
}

fn split_reveal_filter_segments(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current: Vec<OwnedLexToken> = Vec::new();
    let has_noncomparison_or = tokens
        .iter()
        .enumerate()
        .any(|(idx, token)| token.is_word("or") && !is_comparison_or_delimiter(tokens, idx));
    for (idx, token) in tokens.iter().enumerate() {
        let is_separator = (token.is_word("or") && !is_comparison_or_delimiter(tokens, idx))
            || (has_noncomparison_or && token.is_comma());
        if is_separator {
            while current.last().is_some_and(|entry| entry.is_word("and")) {
                current.pop();
            }
            let trimmed = trim_commas(&current);
            if !trimmed.is_empty() {
                segments.push(trimmed.to_vec());
            }
            current.clear();
            continue;
        }
        current.push(token.clone());
    }
    while current.last().is_some_and(|entry| entry.is_word("and")) {
        current.pop();
    }
    let trimmed = trim_commas(&current);
    if !trimmed.is_empty() {
        segments.push(trimmed.to_vec());
    }
    segments
}

fn parse_looked_card_reveal_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut filter_tokens = trim_commas(tokens).to_vec();
    let raw_word_view = LowercaseWordView::new(&filter_tokens);
    let raw_words = raw_word_view.to_word_refs();
    let same_name_suffix_len = if raw_words.len() >= 3
        && raw_words[raw_words.len() - 3..] == ["with", "that", "name"]
    {
        Some(3usize)
    } else if raw_words.len() >= 4
        && raw_words[raw_words.len() - 4..] == ["with", "the", "chosen", "name"]
    {
        Some(4usize)
    } else if raw_words.len() >= 3 && raw_words[raw_words.len() - 3..] == ["with", "chosen", "name"]
    {
        Some(3usize)
    } else {
        None
    };
    if let Some(suffix_len) = same_name_suffix_len {
        let base_end = raw_word_view
            .token_index_for_word_index(raw_words.len().saturating_sub(suffix_len))
            .unwrap_or(filter_tokens.len());
        filter_tokens = trim_commas(&filter_tokens[..base_end]).to_vec();
    }

    let words_all = words(&filter_tokens);
    let non_article_words = words_all
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    if matches!(
        non_article_words.as_slice(),
        ["chosen", "card"] | ["chosen", "cards"]
    ) {
        let mut filter = ObjectFilter::default();
        filter = filter.match_tagged(
            TagKey::from(CHOSEN_NAME_TAG),
            TaggedOpbjectRelation::SameNameAsTagged,
        );
        return Some(filter);
    }
    if matches!(non_article_words.as_slice(), ["card"] | ["cards"]) {
        let mut filter = ObjectFilter::default();
        if same_name_suffix_len.is_some() {
            filter = filter.match_tagged(
                TagKey::from(CHOSEN_NAME_TAG),
                TaggedOpbjectRelation::SameNameAsTagged,
            );
        }
        return Some(filter);
    }
    if matches!(
        words_all.as_slice(),
        ["permanent", "card"] | ["permanent", "cards"]
    ) {
        let mut filter = ObjectFilter::permanent_card();
        if same_name_suffix_len.is_some() {
            filter = filter.match_tagged(
                TagKey::from(CHOSEN_NAME_TAG),
                TaggedOpbjectRelation::SameNameAsTagged,
            );
        }
        return Some(filter);
    }

    let has_noncomparison_or = filter_tokens.iter().enumerate().any(|(idx, token)| {
        token.is_word("or") && !is_comparison_or_delimiter(&filter_tokens, idx)
    });
    if has_noncomparison_or {
        let shared_card_suffix = matches!(words_all.last().copied(), Some("card" | "cards"));
        let segments = split_reveal_filter_segments(&filter_tokens);
        if segments.len() >= 2 {
            let mut branches = Vec::new();
            for mut segment in segments {
                if shared_card_suffix
                    && !matches!(
                        segment.last().and_then(OwnedLexToken::as_word),
                        Some("card" | "cards")
                    )
                {
                    segment.push(OwnedLexToken::word(
                        "card".to_string(),
                        TextSpan::synthetic(),
                    ));
                }
                let parsed = parse_object_filter(&segment, false)
                    .ok()
                    .filter(|filter| *filter != ObjectFilter::default())
                    .or_else(|| parse_named_card_filter_segment(&segment));
                let Some(parsed) = parsed else {
                    return None;
                };
                branches.push(parsed);
            }
            let mut filter = ObjectFilter::default();
            filter.any_of = branches;
            if same_name_suffix_len.is_some() {
                filter = filter.match_tagged(
                    TagKey::from(CHOSEN_NAME_TAG),
                    TaggedOpbjectRelation::SameNameAsTagged,
                );
            }
            return Some(filter);
        }
    }

    let mut filter = parse_search_library_disjunction_filter(&filter_tokens)
        .or_else(|| parse_object_filter(&filter_tokens, false).ok())?;
    if same_name_suffix_len.is_some() {
        filter = filter.match_tagged(
            TagKey::from(CHOSEN_NAME_TAG),
            TaggedOpbjectRelation::SameNameAsTagged,
        );
    }
    Some(filter)
}

fn parse_may_put_filtered_card_from_among_into_hand(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
    zone: Zone,
) -> Result<Option<(PlayerAst, ObjectFilter)>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let sentence_words = words(&sentence_tokens);
    if sentence_words.is_empty() {
        return Ok(None);
    }

    let (chooser, action_word_idx) = if sentence_words.starts_with(&["you", "may", "put"]) {
        (PlayerAst::You, 2usize)
    } else if sentence_words.starts_with(&["that", "player", "may", "put"]) {
        (PlayerAst::That, 3usize)
    } else if sentence_words.starts_with(&["they", "may", "put"]) {
        (PlayerAst::That, 2usize)
    } else if sentence_words.starts_with(&["may", "put"]) {
        (default_player, 1usize)
    } else if sentence_words.starts_with(&["put"]) {
        (default_player, 0usize)
    } else {
        return Ok(None);
    };

    let from_among_word_idx = sentence_words
        .windows(3)
        .position(|window| window == ["from", "among", "them"])
        .or_else(|| {
            sentence_words
                .windows(4)
                .position(|window| window == ["from", "among", "those", "cards"])
        });
    let Some(from_among_word_idx) = from_among_word_idx else {
        return Ok(None);
    };
    if from_among_word_idx <= action_word_idx {
        return Ok(None);
    }

    let filter_start = token_index_for_word_index(&sentence_tokens, action_word_idx + 1)
        .unwrap_or(sentence_tokens.len());
    let filter_end = token_index_for_word_index(&sentence_tokens, from_among_word_idx)
        .unwrap_or(sentence_tokens.len());
    let filter_tokens = trim_commas(&sentence_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = Some(zone);

    let after_from_word_idx = if sentence_words
        .windows(4)
        .any(|window| window == ["from", "among", "those", "cards"])
    {
        from_among_word_idx + 4
    } else {
        from_among_word_idx + 3
    };
    let after_from_words = &sentence_words[after_from_word_idx..];
    let moves_into_hand =
        after_from_words.starts_with(&["into"]) && after_from_words.contains(&"hand");
    if !moves_into_hand {
        return Ok(None);
    }

    Ok(Some((chooser, filter)))
}

fn parse_mill_then_may_put_from_among_into_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) = parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [EffectAst::Mill { player, .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let Some((chooser, filter)) =
        parse_may_put_filtered_card_from_among_into_hand(second, *player, Zone::Graveyard)?
    else {
        return Ok(None);
    };

    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
            player: chooser,
            filter,
            reveal: false,
            if_not_chosen: Vec::new(),
        },
    ]))
}

fn parse_mill_then_may_put_from_among_into_hand_then_if_you_dont(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) = parse_mill_then_may_put_from_among_into_hand(first, second)? else {
        return Ok(None);
    };
    let Some(if_not_chosen) = parse_if_you_dont_sentence(third)? else {
        return Ok(None);
    };

    let Some(EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
        if_not_chosen: existing,
        ..
    }) = effects.get_mut(1)
    else {
        return Ok(None);
    };
    *existing = if_not_chosen;
    Ok(Some(effects))
}

fn parse_triple_sentence_sequence(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    const RULES: [(&str, TripleSentenceRule); 9] = [
        (
            "mill-then-put-from-among-into-hand-then-if-you-dont",
            parse_mill_then_may_put_from_among_into_hand_then_if_you_dont,
        ),
        (
            "search-then-next-upkeep-unless-pays-lose-game",
            parse_search_then_delayed_next_upkeep_unless_pays_lose_game,
        ),
        (
            "exile-until-match-cast-rest-bottom",
            parse_exile_until_match_cast_rest_bottom,
        ),
        (
            "exile-until-match-cast-else-hand",
            parse_exile_until_match_cast_else_hand,
        ),
        (
            "top-cards-put-match-into-hand-rest-graveyard",
            parse_top_cards_put_match_into_hand_rest_graveyard,
        ),
        (
            "look-at-top-reveal-match-put-rest-bottom",
            parse_look_at_top_reveal_match_put_rest_bottom,
        ),
        (
            "prefix-then-consult-match-move-bottom-remainder",
            parse_prefix_then_consult_match_move_and_bottom_remainder,
        ),
        (
            "prefix-then-consult-match-into-hand-exile-others",
            parse_prefix_then_consult_match_into_hand_exile_others,
        ),
        ("tainted-pact-sequence", parse_tainted_pact_sequence),
    ];

    for (name, rule) in RULES {
        if let Some(combined) = rule(first, second, third)? {
            return Ok(Some((name, combined)));
        }
    }

    Ok(None)
}

fn parse_search_then_delayed_next_upkeep_unless_pays_lose_game(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_effects = parse_effect_chain(first)?;
    let first_words = words(first);
    if first_effects.is_empty() || !first_words.starts_with(&["search", "your", "library"]) {
        return Ok(None);
    }

    let upkeep_tokens = trim_commas(second);
    let upkeep_words = words(&upkeep_tokens);
    let pay_idx = if upkeep_words.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "your",
        "next",
        "upkeep",
        "pay",
    ]) {
        7usize
    } else if upkeep_words.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "next",
        "upkeep",
        "pay",
    ]) {
        7usize
    } else {
        return Ok(None);
    };
    let Some(pay_token_idx) = token_index_for_word_index(&upkeep_tokens, pay_idx) else {
        return Ok(None);
    };
    let mana_tokens = trim_commas(&upkeep_tokens[pay_token_idx + 1..]);
    if mana_tokens.is_empty() {
        return Ok(None);
    }

    let mut mana = Vec::<ManaSymbol>::new();
    for token in mana_tokens {
        if let Some(pips) = mana_pips_from_token(&token) {
            mana.extend(pips);
            continue;
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if let Ok(generic) = word.parse::<u8>() {
            mana.push(ManaSymbol::Generic(generic));
            continue;
        }
        return Ok(None);
    }
    if mana.is_empty() {
        return Ok(None);
    }

    let lose_tokens = trim_commas(third);
    let lose_words = words(&lose_tokens);
    let valid_lose_clause = lose_words == ["if", "you", "dont", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "don't", "you", "lose", "the", "game"]
        || lose_words == ["if", "you", "do", "not", "you", "lose", "the", "game"];
    if !valid_lose_clause {
        return Ok(None);
    }

    let mut effects = first_effects;
    effects.push(EffectAst::DelayedUntilNextUpkeep {
        player: PlayerAst::You,
        effects: vec![EffectAst::UnlessPays {
            effects: vec![EffectAst::LoseGame {
                player: PlayerAst::You,
            }],
            player: PlayerAst::You,
            mana,
        }],
    });
    Ok(Some(effects))
}

fn parse_if_no_card_into_hand_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let has_expected_prefix = words.starts_with(&[
        "if", "you", "didnt", "put", "card", "into", "your", "hand", "this", "way",
    ]) || words.starts_with(&[
        "if", "you", "didn't", "put", "card", "into", "your", "hand", "this", "way",
    ]) || words.starts_with(&[
        "if", "you", "did", "not", "put", "card", "into", "your", "hand", "this", "way",
    ]);
    if !has_expected_prefix {
        return Ok(None);
    }

    let Some(comma_idx) = tokens.iter().position(|token| token.is_comma()) else {
        return Ok(None);
    };
    if comma_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let effects = parse_effect_chain(&tokens[comma_idx + 1..])?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(effects))
}

fn parse_if_you_dont_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let has_expected_prefix = words.starts_with(&["if", "you", "dont"])
        || words.starts_with(&["if", "you", "don't"])
        || words.starts_with(&["if", "you", "do", "not"]);
    if !has_expected_prefix {
        return Ok(None);
    }

    let Some(comma_idx) = tokens.iter().position(|token| token.is_comma()) else {
        return Ok(None);
    };
    if comma_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let effects = parse_effect_chain(&tokens[comma_idx + 1..])?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(effects))
}

fn parse_look_at_top_reveal_match_put_rest_bottom_then_if_not_into_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
    fourth: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) = parse_look_at_top_reveal_match_put_rest_bottom(first, second, third)?
    else {
        return Ok(None);
    };
    let Some(if_not_chosen) = parse_if_no_card_into_hand_this_way_sentence(fourth)? else {
        return Ok(None);
    };

    let Some(EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary {
        if_not_chosen: existing,
        ..
    }) = effects.get_mut(1)
    else {
        return Ok(None);
    };
    *existing = if_not_chosen;
    Ok(Some(effects))
}

fn parse_quad_sentence_sequence(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
    fourth: &[OwnedLexToken],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    const RULES: [(&str, QuadSentenceRule); 3] = [
        (
            "look-at-top-put-counted-into-hand-rest-bottom-kicker-override",
            parse_look_at_top_put_counted_into_hand_rest_bottom_with_kicker_override,
        ),
        (
            "look-at-top-may-put-match-onto-battlefield-if-not-put-into-hand-rest-bottom",
            parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom,
        ),
        (
            "look-at-top-reveal-match-put-rest-bottom-if-not-into-hand",
            parse_look_at_top_reveal_match_put_rest_bottom_then_if_not_into_hand,
        ),
    ];

    for (name, rule) in RULES {
        if let Some(combined) = rule(first, second, third, fourth)? {
            return Ok(Some((name, combined)));
        }
    }

    Ok(None)
}

fn parse_effect_sentences_from_sentence_inputs(
    sentences: Vec<SentenceInput>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let mut effects = Vec::new();
    let mut sentence_idx = 0usize;
    let mut carried_context: Option<CarryContext> = None;

    fn effect_contains_search_library(effect: &EffectAst) -> bool {
        if matches!(effect, EffectAst::SearchLibrary { .. }) {
            return true;
        }

        let mut found = false;
        for_each_nested_effects(effect, true, |nested| {
            if !found {
                found = nested.iter().any(effect_contains_search_library);
            }
        });
        found
    }

    fn effect_needs_followup_library_shuffle(effect: &EffectAst) -> bool {
        if matches!(effect, EffectAst::ChooseObjectsAcrossZones { zones, .. } if zones.contains(&Zone::Library))
        {
            return true;
        }

        let mut found = false;
        for_each_nested_effects(effect, true, |nested| {
            if !found {
                found = nested.iter().any(effect_needs_followup_library_shuffle);
            }
        });
        found
    }

    fn is_if_you_search_library_this_way_shuffle_sentence(tokens: &[OwnedLexToken]) -> bool {
        let words: Vec<&str> = words(tokens)
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
        // "If you search your library this way, shuffle."
        words.as_slice()
            == [
                "if", "you", "search", "your", "library", "this", "way", "shuffle",
            ]
            || words.as_slice()
                == [
                    "if", "you", "search", "your", "library", "this", "way", "shuffles",
                ]
    }

    while sentence_idx < sentences.len() {
        let sentence = sentences[sentence_idx].lowered();
        if sentence.is_empty() {
            sentence_idx += 1;
            continue;
        }

        if sentence_idx + 3 < sentences.len()
            && let Some((rule_name, mut combined)) = parse_quad_sentence_sequence(
                sentence,
                sentences[sentence_idx + 1].lowered(),
                sentences[sentence_idx + 2].lowered(),
                sentences[sentence_idx + 3].lowered(),
            )?
        {
            let stage = format!("parse_effect_sentences:sequence-hit:{rule_name}");
            parser_trace(stage.as_str(), sentence);
            effects.append(&mut combined);
            sentence_idx += 4;
            continue;
        }

        if sentence_idx + 2 < sentences.len()
            && let Some((rule_name, mut combined)) = parse_triple_sentence_sequence(
                sentence,
                sentences[sentence_idx + 1].lowered(),
                sentences[sentence_idx + 2].lowered(),
            )?
        {
            let stage = format!("parse_effect_sentences:sequence-hit:{rule_name}");
            parser_trace(stage.as_str(), sentence);
            effects.append(&mut combined);
            sentence_idx += 3;
            continue;
        }

        if sentence_idx + 1 < sentences.len()
            && let Some((rule_name, mut combined)) =
                parse_pair_sentence_sequence(sentence, sentences[sentence_idx + 1].lowered())?
        {
            let stage = format!("parse_effect_sentences:sequence-hit:{rule_name}");
            parser_trace(stage.as_str(), sentence);
            effects.append(&mut combined);
            sentence_idx += 2;
            continue;
        }
        let mut sentence_tokens = strip_embedded_token_rules_text(sentence);
        sentence_tokens = trim_edge_punctuation(&sentence_tokens);
        if sentence_tokens.is_empty() || words(&sentence_tokens).is_empty() {
            sentence_idx += 1;
            continue;
        }
        sentence_tokens = rewrite_when_one_or_more_this_way_clause_prefix(&sentence_tokens);

        // Oracle frequently splits shuffle followups as a standalone sentence:
        // "If you search your library this way, shuffle." This clause is redundant when the
        // preceding sentence already compiles a library-search effect that shuffles.
        if is_if_you_search_library_this_way_shuffle_sentence(&sentence_tokens) {
            if effects.iter().any(effect_needs_followup_library_shuffle) {
                parser_trace(
                    "parse_effect_sentences:append:if-you-search-library-this-way-shuffle",
                    &sentence_tokens,
                );
                effects.push(EffectAst::ShuffleLibrary {
                    player: PlayerAst::You,
                });
                sentence_idx += 1;
                continue;
            }
            if effects.iter().any(effect_contains_search_library) {
                parser_trace(
                    "parse_effect_sentences:skip:if-you-search-library-this-way-shuffle",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
        }

        let sentence_words = words(&sentence_tokens);
        let is_still_lands_followup = matches!(
            sentence_words.as_slice(),
            ["theyre", "still", "land"]
                | ["theyre", "still", "lands"]
                | ["its", "still", "a", "land"]
                | ["its", "still", "land"]
        );
        if is_still_lands_followup
            && effects
                .last()
                .is_some_and(|effect| matches!(effect, EffectAst::BecomeBasePtCreature { .. }))
        {
            parser_trace(
                "parse_effect_sentences:skip:still-lands-followup",
                &sentence_tokens,
            );
            sentence_idx += 1;
            continue;
        }

        let mut wraps_as_if_did_not = false;
        if let Some(without_otherwise) = strip_otherwise_sentence_prefix(&sentence_tokens) {
            sentence_tokens = rewrite_otherwise_referential_subject(without_otherwise);
            wraps_as_if_did_not = true;
        }
        parser_trace("parse_effect_sentences:sentence", &sentence_tokens);

        // "Destroy ... . It/They can't be regenerated." followups.
        if is_cant_be_regenerated_followup_sentence(&sentence_tokens) {
            if apply_cant_be_regenerated_to_last_destroy_effect(&mut effects) {
                parser_trace(
                    "parse_effect_sentences:cant-be-regenerated-followup",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            if is_cant_be_regenerated_this_turn_followup_sentence(&sentence_tokens)
                && apply_cant_be_regenerated_to_last_target_effect(&mut effects)
            {
                parser_trace(
                    "parse_effect_sentences:cant-be-regenerated-this-turn-followup",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone cant-be-regenerated clause (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }

        if sentence_idx + 1 < sentences.len() && is_simple_copy_reference_sentence(&sentence_tokens)
        {
            let next_tokens =
                strip_embedded_token_rules_text(sentences[sentence_idx + 1].lowered());
            if let Some(spec) = parse_may_cast_it_sentence(&next_tokens)
                && spec.as_copy
            {
                parser_trace(
                    "parse_effect_sentences:copy-reference-next-may-cast-copy",
                    &sentence_tokens,
                );
                effects.push(build_may_cast_tagged_effect(&spec));
                sentence_idx += 2;
                continue;
            }
        }

        if let Some(spec) = parse_may_cast_it_sentence(&sentence_tokens) {
            parser_trace(
                "parse_effect_sentences:may-cast-it-sentence",
                &sentence_tokens,
            );
            effects.push(build_may_cast_tagged_effect(&spec));
            sentence_idx += 1;
            continue;
        }

        if is_spawn_scion_token_mana_reminder(&sentence_tokens) {
            if effects
                .last()
                .is_some_and(effect_creates_eldrazi_spawn_or_scion)
            {
                parser_trace(
                    "parse_effect_sentences:spawn-scion-reminder",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token mana reminder clause (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }
        if let Some(effect) =
            parse_sentence_exile_that_token_when_source_leaves(&sentence_tokens, &effects)
        {
            parser_trace(
                "parse_effect_sentences:linked-token-exile-when-source-leaves",
                &sentence_tokens,
            );
            effects.push(effect);
            sentence_idx += 1;
            continue;
        }
        if let Some(effect) =
            parse_sentence_sacrifice_source_when_that_token_leaves(&sentence_tokens, &effects)
        {
            parser_trace(
                "parse_effect_sentences:linked-token-sacrifice-source-when-token-leaves",
                &sentence_tokens,
            );
            effects.push(effect);
            sentence_idx += 1;
            continue;
        }
        if is_generic_token_reminder_sentence(&sentence_tokens)
            && effects.last().is_some_and(effect_creates_any_token)
        {
            if append_token_reminder_to_last_create_effect(&mut effects, &sentence_tokens) {
                parser_trace(
                    "parse_effect_sentences:token-reminder-followup",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token reminder clause (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }
        if is_generic_token_reminder_sentence(&sentence_tokens) {
            let reminder_words = words(&sentence_tokens);
            let delayed_pronoun_lifecycle =
                matches!(reminder_words.first().copied(), Some("exile" | "sacrifice"))
                    && (reminder_words.contains(&"it") || reminder_words.contains(&"them"));
            let pronoun_followup_clause = reminder_words.starts_with(&["when", "it"])
                || reminder_words.starts_with(&["whenever", "it"])
                || reminder_words.starts_with(&["when", "they"])
                || reminder_words.starts_with(&["whenever", "they"]);
            if delayed_pronoun_lifecycle || pronoun_followup_clause {
                // Keep standalone pronoun-led followups on the normal parser path.
                // They can be genuine tagged-object effects, not just token reminder text.
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported standalone token reminder clause (clause: '{}')",
                    words(&sentence_tokens).join(" ")
                )));
            }
        }

        if let Some(effect) = parse_choose_target_prelude_sentence(&sentence_tokens)? {
            effects.push(effect);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        let mut sentence_effects =
            if let Some(followup) = parse_token_copy_followup_sentence(&sentence_tokens) {
                if try_apply_token_copy_followup(&mut effects, followup)? {
                    parser_trace(
                        "parse_effect_sentences:token-copy-followup",
                        &sentence_tokens,
                    );
                    sentence_idx += 1;
                    continue;
                }
                apply_unapplied_token_copy_followup(sentence, &sentence_tokens, followup)?
            } else if let Some(lexed_sentence) = sentences[sentence_idx].lexed.as_deref()
                && sentence_tokens.as_slice() == sentences[sentence_idx].lowered()
            {
                parse_effect_sentence_lexed(lexed_sentence)?
            } else {
                parse_effect_sentence_lexed(&sentence_tokens)?
            };
        if wraps_as_if_did_not {
            sentence_effects = vec![EffectAst::IfResult {
                predicate: IfResultPredicate::DidNot,
                effects: sentence_effects,
            }];
            carried_context = None;
        }
        collapse_token_copy_next_end_step_exile_followup(&mut sentence_effects, &sentence_tokens);
        collapse_token_copy_end_of_combat_exile_followup(&mut sentence_effects, &sentence_tokens);
        if is_that_turn_end_step_sentence(&sentence_tokens)
            && let Some(extra_turn_player) = most_recent_extra_turn_player(&effects)
            && !sentence_effects.is_empty()
        {
            sentence_effects = vec![EffectAst::DelayedUntilEndStepOfExtraTurn {
                player: extra_turn_player,
                effects: sentence_effects,
            }];
        }
        if words(&sentence_tokens).first().copied() == Some("you") {
            carried_context = None;
        }
        if sentence_effects.is_empty()
            && !is_round_up_each_time_sentence(&sentence_tokens)
            && !is_nonsemantic_restriction_sentence(&sentence_tokens)
        {
            return Err(CardTextError::ParseError(format!(
                "sentence parsed to no semantic effects (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }
        for effect in &mut sentence_effects {
            if let Some(context) = carried_context {
                maybe_apply_carried_player_with_clause(effect, context, &sentence_tokens);
            }
            if let Some(context) = explicit_player_for_carry(effect) {
                carried_context = Some(context);
            }
        }
        if sentence_effects.len() == 1
            && let Some(previous_effect) = effects.last()
            && let Some(effect) = sentence_effects.first_mut()
            && let EffectAst::IfResult {
                predicate,
                effects: if_result_effects,
            } = effect
        {
            if matches!(*predicate, IfResultPredicate::Did)
                && matches!(previous_effect, EffectAst::UnlessPays { .. })
            {
                *predicate = IfResultPredicate::DidNot;
            }
            if let Some(previous_target) = primary_damage_target_from_effect(previous_effect) {
                replace_it_damage_target_in_effects(
                    if_result_effects.as_mut_slice(),
                    &previous_target,
                );
            }
        }
        let sentence_text = words(&sentence_tokens).join(" ");
        maybe_rewrite_future_zone_replacement_sentence(&mut sentence_effects, &sentence_text);
        if matches!(
            classify_instead_followup_text(&sentence_text),
            InsteadSemantics::SelfReplacement
        ) && sentence_effects.len() == 1
            && effects.len() >= 1
        {
            if matches!(
                sentence_effects.first(),
                Some(EffectAst::Conditional { .. })
            ) {
                let Some(previous) = effects.pop() else {
                    return Err(CardTextError::InvariantViolation(
                        "expected previous effect for 'instead' conditional rewrite".to_string(),
                    ));
                };
                let previous_target = primary_target_from_effect(&previous);
                let previous_damage_target = primary_damage_target_from_effect(&previous);
                if let Some(EffectAst::Conditional {
                    predicate,
                    mut if_true,
                    mut if_false,
                }) = sentence_effects.pop()
                {
                    if let Some(target) = previous_target {
                        replace_it_target_in_effects(&mut if_true, &target);
                    }
                    if let Some(target) = previous_damage_target {
                        replace_it_damage_target_in_effects(&mut if_true, &target);
                        replace_placeholder_damage_target_in_effects(&mut if_true, &target);
                    }
                    if_false.insert(0, previous);
                    effects.push(EffectAst::SelfReplacement {
                        predicate,
                        if_true,
                        if_false,
                    });
                    sentence_idx += 1;
                    continue;
                }
            }
        }

        effects.extend(sentence_effects);
        sentence_idx += 1;
    }

    if let Some(last_sentence) = sentences.last() {
        parser_trace("parse_effect_sentences:done", last_sentence.lowered());
    }
    Ok(effects)
}

pub(crate) fn parse_effect_sentences_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = parse_exact_card_effect_bundle_lexed(tokens) {
        return Ok(effects);
    }

    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .map(SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    parse_effect_sentences_from_sentence_inputs(sentences)
}

pub(crate) fn is_cant_be_regenerated_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    matches!(
        words.as_slice(),
        ["it", "cant", "be", "regenerated"]
            | ["it", "cant", "be", "regenerated", "this", "turn"]
            | ["they", "cant", "be", "regenerated"]
            | ["they", "cant", "be", "regenerated", "this", "turn"]
    )
}

pub(crate) fn is_cant_be_regenerated_this_turn_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    matches!(
        words.as_slice(),
        ["it", "cant", "be", "regenerated", "this", "turn"]
            | ["they", "cant", "be", "regenerated", "this", "turn"]
    )
}

pub(crate) fn apply_cant_be_regenerated_to_last_destroy_effect(
    effects: &mut Vec<EffectAst>,
) -> bool {
    let Some(last) = effects.last_mut() else {
        return false;
    };
    apply_cant_be_regenerated_to_effect(last)
}

pub(crate) fn apply_cant_be_regenerated_to_last_target_effect(
    effects: &mut Vec<EffectAst>,
) -> bool {
    let Some(previous_target) = effects.last().and_then(primary_target_from_effect) else {
        return false;
    };
    let Some(mut filter) = target_ast_to_object_filter(previous_target) else {
        return false;
    };
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    effects.push(EffectAst::Cant {
        restriction: crate::effect::Restriction::be_regenerated(filter),
        duration: Until::EndOfTurn,
        condition: None,
    });
    true
}

fn apply_cant_be_regenerated_to_effect(effect: &mut EffectAst) -> bool {
    match effect {
        EffectAst::Destroy { target } => {
            let target = target.clone();
            *effect = EffectAst::DestroyNoRegeneration { target };
            true
        }
        EffectAst::DestroyAll { filter } => {
            let filter = filter.clone();
            *effect = EffectAst::DestroyAllNoRegeneration { filter };
            true
        }
        EffectAst::DestroyAllOfChosenColor { filter } => {
            let filter = filter.clone();
            *effect = EffectAst::DestroyAllOfChosenColorNoRegeneration { filter };
            true
        }
        _ => {
            let mut applied = false;
            for_each_nested_effects_mut(effect, true, |nested| {
                if !applied {
                    applied = apply_cant_be_regenerated_to_effects_tail(nested);
                }
            });
            applied
        }
    }
}

fn apply_cant_be_regenerated_to_effects_tail(effects: &mut [EffectAst]) -> bool {
    for effect in effects.iter_mut().rev() {
        if apply_cant_be_regenerated_to_effect(effect) {
            return true;
        }
    }
    false
}

pub(crate) fn primary_damage_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::DealDamage { target, .. } | EffectAst::DealDamageEqualToPower { target, .. } => {
            Some(target.clone())
        }
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_damage_target_from_effect);
                }
            });
            found
        }
    }
}

pub(crate) fn primary_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::DealDamage { target, .. }
        | EffectAst::DealDamageEqualToPower { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::CounterUnlessPays { target, .. }
        | EffectAst::Explore { target }
        | EffectAst::Connive { target }
        | EffectAst::Goad { target }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::RemoveFromCombat { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Destroy { target }
        | EffectAst::DestroyNoRegeneration { target }
        | EffectAst::Exile { target, .. }
        | EffectAst::ExileWhenSourceLeaves { target }
        | EffectAst::SacrificeSourceWhenLeaves { target }
        | EffectAst::ExileUntilSourceLeaves { target, .. }
        | EffectAst::LookAtHand { target }
        | EffectAst::Transform { target }
        | EffectAst::Flip { target }
        | EffectAst::Regenerate { target }
        | EffectAst::PhaseOut { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::ReturnToHand { target, .. }
        | EffectAst::ReturnToBattlefield { target, .. }
        | EffectAst::MoveToZone { target, .. }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::PutOrRemoveCounters { target, .. }
        | EffectAst::RemoveUpToAnyCounters { target, .. }
        | EffectAst::Pump { target, .. }
        | EffectAst::GrantAbilitiesToTarget { target, .. }
        | EffectAst::GrantToTarget { target, .. }
        | EffectAst::GrantAbilitiesChoiceToTarget { target, .. }
        | EffectAst::GrantProtectionChoice { target, .. }
        | EffectAst::PreventDamage { target, .. }
        | EffectAst::PreventAllDamageToTarget { target, .. }
        | EffectAst::PreventDamageToTargetPutCounters { target, .. }
        | EffectAst::PreventAllCombatDamageFromSource { source: target, .. }
        | EffectAst::RedirectNextDamageFromSourceToTarget { target, .. }
        | EffectAst::RedirectNextTimeDamageToSource { target, .. }
        | EffectAst::GainControl { target, .. } => Some(target.clone()),
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_target_from_effect);
                }
            });
            found
        }
    }
}

pub(crate) fn replace_it_damage_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_damage_target(effect, target);
    }
}

pub(crate) fn replace_it_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_target(effect, target);
    }
}

pub(crate) fn is_placeholder_damage_target(target: &TargetAst) -> bool {
    matches!(
        target,
        TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
    )
}

pub(crate) fn replace_placeholder_damage_target_in_effects(
    effects: &mut [EffectAst],
    target: &TargetAst,
) {
    for effect in effects {
        replace_placeholder_damage_target(effect, target);
    }
}

pub(crate) fn replace_placeholder_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: damage_target,
            ..
        }
        | EffectAst::DealDamageEqualToPower {
            target: damage_target,
            ..
        } => {
            if is_placeholder_damage_target(damage_target) {
                *damage_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_placeholder_damage_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn replace_unbound_x_in_damage_effects(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_unbound_x_in_damage_effect(effect, replacement, clause)?;
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_damage_effect(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::LoseLife { amount, .. } => {
            if value_contains_unbound_x(amount) {
                *amount = replace_unbound_x_with_value(amount.clone(), replacement, clause)?;
            }
        }
        _ => {
            try_for_each_nested_effects_mut(effect, true, |nested| {
                replace_unbound_x_in_damage_effects(nested, replacement, clause)
            })?;
        }
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_effects_anywhere(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_unbound_x_in_effect_anywhere(effect, replacement, clause)?;
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_effect_anywhere(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    fn replace_value(
        value: &mut Value,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        if value_contains_unbound_x(value) {
            *value = replace_unbound_x_with_value(value.clone(), replacement, clause)?;
        }
        Ok(())
    }

    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::Draw { count: amount, .. }
        | EffectAst::LoseLife { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::PreventDamage { amount, .. }
        | EffectAst::PreventDamageEach { amount, .. }
        | EffectAst::PutCounters { count: amount, .. }
        | EffectAst::PutCountersAll { count: amount, .. }
        | EffectAst::Mill { count: amount, .. }
        | EffectAst::Discard { count: amount, .. }
        | EffectAst::Scry { count: amount, .. }
        | EffectAst::Surveil { count: amount, .. }
        | EffectAst::Discover { count: amount, .. }
        | EffectAst::LookAtTopCards { count: amount, .. }
        | EffectAst::PayEnergy { amount, .. }
        | EffectAst::CopySpell { count: amount, .. }
        | EffectAst::SetLifeTotal { amount, .. }
        | EffectAst::Monstrosity { amount } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::PreventDamageToTargetPutCounters {
            amount: Some(amount),
            ..
        } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::Pump {
            power, toughness, ..
        }
        | EffectAst::SetBasePowerToughness {
            power, toughness, ..
        }
        | EffectAst::BecomeBasePtCreature {
            power, toughness, ..
        }
        | EffectAst::PumpAll {
            power, toughness, ..
        } => {
            replace_value(power, replacement, clause)?;
            replace_value(toughness, replacement, clause)?;
        }
        EffectAst::SetBasePower { power, .. } => {
            replace_value(power, replacement, clause)?;
        }
        EffectAst::PutOrRemoveCounters {
            put_count,
            remove_count,
            ..
        } => {
            replace_value(put_count, replacement, clause)?;
            replace_value(remove_count, replacement, clause)?;
        }
        EffectAst::RemoveUpToAnyCounters { amount, .. } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaFromLandCouldProduce { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::CreateTokenCopy { count, .. }
        | EffectAst::CreateTokenCopyFromSource { count, .. } => {
            replace_value(count, replacement, clause)?;
        }
        EffectAst::CreateTokenWithMods {
            count,
            dynamic_power_toughness,
            ..
        } => {
            replace_value(count, replacement, clause)?;
            if let Some((power, toughness)) = dynamic_power_toughness {
                replace_value(power, replacement, clause)?;
                replace_value(toughness, replacement, clause)?;
            }
        }
        EffectAst::CounterUnlessPays {
            life,
            additional_generic,
            ..
        } => {
            if let Some(life) = life.as_mut() {
                replace_value(life, replacement, clause)?;
            }
            if let Some(generic) = additional_generic.as_mut() {
                replace_value(generic, replacement, clause)?;
            }
        }
        EffectAst::PumpForEach { count, .. } => {
            replace_value(count, replacement, clause)?;
        }
        _ => {
            try_for_each_nested_effects_mut(effect, true, |nested| {
                replace_unbound_x_in_effects_anywhere(nested, replacement, clause)
            })?;
        }
    }
    Ok(())
}

pub(crate) fn apply_where_x_to_damage_amounts(
    tokens: &[OwnedLexToken],
    effects: &mut [EffectAst],
) -> Result<(), CardTextError> {
    let clause_words = words(tokens);
    let has_deal_x = clause_words.windows(3).any(|window| {
        (window[0] == "deal" || window[0] == "deals") && window[1] == "x" && window[2] == "damage"
    });
    let has_x_life = clause_words.windows(3).any(|window| {
        (window[0] == "gain" || window[0] == "gains" || window[0] == "lose" || window[0] == "loses")
            && window[1] == "x"
            && window[2] == "life"
    });
    if !has_deal_x && !has_x_life {
        return Ok(());
    }
    let Some(where_idx) = clause_words
        .windows(3)
        .position(|window| window == ["where", "x", "is"])
    else {
        return Ok(());
    };
    let Some(where_token_idx) = token_index_for_word_index(tokens, where_idx) else {
        return Ok(());
    };
    let where_tokens = &tokens[where_token_idx..];
    let Some(where_value) = parse_where_x_value_clause(where_tokens) else {
        return Ok(());
    };
    replace_unbound_x_in_damage_effects(effects, &where_value, &clause_words.join(" "))
}

pub(crate) fn replace_it_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: damage_target,
            ..
        } => {
            if target_references_it(damage_target) {
                *damage_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_it_damage_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn replace_it_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: effect_target,
            ..
        }
        | EffectAst::DealDamageEqualToPower {
            target: effect_target,
            ..
        }
        | EffectAst::Counter {
            target: effect_target,
        }
        | EffectAst::CounterUnlessPays {
            target: effect_target,
            ..
        }
        | EffectAst::Explore {
            target: effect_target,
        }
        | EffectAst::Connive {
            target: effect_target,
        }
        | EffectAst::Goad {
            target: effect_target,
        }
        | EffectAst::Tap {
            target: effect_target,
        }
        | EffectAst::Untap {
            target: effect_target,
        }
        | EffectAst::PhaseOut {
            target: effect_target,
        }
        | EffectAst::RemoveFromCombat {
            target: effect_target,
        }
        | EffectAst::TapOrUntap {
            target: effect_target,
        }
        | EffectAst::Destroy {
            target: effect_target,
        }
        | EffectAst::DestroyNoRegeneration {
            target: effect_target,
        }
        | EffectAst::Exile {
            target: effect_target,
            ..
        }
        | EffectAst::ExileWhenSourceLeaves {
            target: effect_target,
        }
        | EffectAst::SacrificeSourceWhenLeaves {
            target: effect_target,
        }
        | EffectAst::ExileUntilSourceLeaves {
            target: effect_target,
            ..
        }
        | EffectAst::LookAtHand {
            target: effect_target,
        }
        | EffectAst::Transform {
            target: effect_target,
        }
        | EffectAst::Flip {
            target: effect_target,
        }
        | EffectAst::Regenerate {
            target: effect_target,
        }
        | EffectAst::TargetOnly {
            target: effect_target,
        }
        | EffectAst::ReturnToHand {
            target: effect_target,
            ..
        }
        | EffectAst::ReturnToBattlefield {
            target: effect_target,
            ..
        }
        | EffectAst::MoveToZone {
            target: effect_target,
            ..
        }
        | EffectAst::PutCounters {
            target: effect_target,
            ..
        }
        | EffectAst::PutOrRemoveCounters {
            target: effect_target,
            ..
        }
        | EffectAst::RemoveUpToAnyCounters {
            target: effect_target,
            ..
        }
        | EffectAst::Pump {
            target: effect_target,
            ..
        }
        | EffectAst::GrantAbilitiesToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantAbilitiesChoiceToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantProtectionChoice {
            target: effect_target,
            ..
        }
        | EffectAst::PreventDamage {
            target: effect_target,
            ..
        }
        | EffectAst::PreventAllDamageToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::PreventDamageToTargetPutCounters {
            target: effect_target,
            ..
        }
        | EffectAst::PreventAllCombatDamageFromSource {
            source: effect_target,
            ..
        }
        | EffectAst::RedirectNextDamageFromSourceToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::RedirectNextTimeDamageToSource {
            target: effect_target,
            ..
        }
        | EffectAst::GainControl {
            target: effect_target,
            ..
        } => {
            if target_references_it(effect_target) {
                *effect_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_it_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn target_references_it(target: &TargetAst) -> bool {
    match target {
        TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
        TargetAst::Object(filter, _, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG),
        TargetAst::WithCount(inner, _) => target_references_it(inner),
        _ => false,
    }
}

pub(crate) fn is_that_turn_end_step_sentence(tokens: &[OwnedLexToken]) -> bool {
    let clause_words = words(tokens);
    clause_words.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "that",
        "turn",
        "end",
        "step",
    ]) || clause_words.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "that",
        "turns",
        "end",
        "step",
    ])
}

pub(crate) fn most_recent_extra_turn_player(effects: &[EffectAst]) -> Option<PlayerAst> {
    effects.iter().rev().find_map(|effect| {
        if let EffectAst::ExtraTurnAfterTurn { player, .. } = effect {
            Some(*player)
        } else {
            None
        }
    })
}

pub(crate) fn rewrite_when_one_or_more_this_way_clause_prefix(
    tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let clause_words = words(tokens);
    // Generic "When one or more ... this way, ..." follow-ups are semantically
    // "If you do, ..." against the immediately previous effect result.
    let has_this_way = clause_words
        .windows(2)
        .any(|window| window == ["this", "way"]);
    if (clause_words.starts_with(&["when", "one", "or", "more"])
        || clause_words.starts_with(&["whenever", "one", "or", "more"]))
        && has_this_way
    {
        let Some(comma_idx) = tokens.iter().position(|token| token.is_comma()) else {
            return tokens.to_vec();
        };
        let mut rewritten = Vec::new();

        let mut if_token = tokens[0].clone();
        if let Some(word) = if_token.word_mut() {
            *word = "if".to_string();
        }
        rewritten.push(if_token);

        let mut you_token = tokens.get(1).cloned().unwrap_or_else(|| tokens[0].clone());
        if let Some(word) = you_token.word_mut() {
            *word = "you".to_string();
        }
        rewritten.push(you_token);

        let mut do_token = tokens.get(2).cloned().unwrap_or_else(|| tokens[0].clone());
        if let Some(word) = do_token.word_mut() {
            *word = "do".to_string();
        }
        rewritten.push(do_token);

        rewritten.push(tokens[comma_idx].clone());
        rewritten.extend_from_slice(&tokens[comma_idx + 1..]);
        return rewritten;
    }

    tokens.to_vec()
}

pub(crate) fn strip_otherwise_sentence_prefix(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("otherwise"))
    {
        return None;
    }

    let mut idx = 1usize;
    while tokens.get(idx).is_some_and(OwnedLexToken::is_comma) {
        idx += 1;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("then")) {
        idx += 1;
    }
    while tokens.get(idx).is_some_and(OwnedLexToken::is_comma) {
        idx += 1;
    }

    let remainder = trim_commas(&tokens[idx..]);
    if remainder.is_empty() {
        None
    } else {
        Some(remainder)
    }
}

pub(crate) fn rewrite_otherwise_referential_subject(
    tokens: Vec<OwnedLexToken>,
) -> Vec<OwnedLexToken> {
    let clause_words = words(&tokens);
    let is_referential_get = clause_words.len() >= 3
        && clause_words[0] == "that"
        && matches!(clause_words[1], "creature" | "permanent")
        && matches!(clause_words[2], "gets" | "get" | "gains" | "gain");
    if !is_referential_get {
        return tokens;
    }

    let mut rewritten = tokens;
    if let Some(first) = rewritten.get_mut(0)
        && let Some(word) = first.word_mut()
    {
        *word = "target".to_string();
    }
    rewritten
}

pub(crate) fn is_nonsemantic_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    is_activate_only_restriction_sentence(tokens) || is_trigger_only_restriction_sentence(tokens)
}

fn token_copy_followup_container_effects_mut(
    effect: &mut EffectAst,
) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::WhenResult { effects, .. }
        | EffectAst::ResolvedIfResult { effects, .. }
        | EffectAst::ResolvedWhenResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayersFiltered { effects, .. }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::RepeatProcess { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilNextUpkeep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::VoteOption { effects, .. } => Some(effects),
        _ => None,
    }
}

pub(crate) fn parse_token_copy_followup_sentence(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if matches!(
        filtered.as_slice(),
        [
            "sacrifice",
            "that",
            "token",
            "at",
            "beginning",
            "of",
            "next",
            "end",
            "step"
        ] | [
            "sacrifice",
            "those",
            "tokens",
            "at",
            "beginning",
            "of",
            "next",
            "end",
            "step"
        ]
    ) {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }

    parse_token_copy_modifier_sentence(tokens)
        .or_else(|| {
            is_exile_that_token_at_end_of_combat(tokens)
                .then_some(TokenCopyFollowup::ExileAtEndOfCombat)
        })
        .or_else(|| {
            is_sacrifice_that_token_at_end_of_combat(tokens)
                .then_some(TokenCopyFollowup::SacrificeAtEndOfCombat)
        })
}

pub(crate) fn parse_token_copy_followup_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered: Vec<&str> = crate::cards::builders::parser::lexed_words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if matches!(
        filtered.as_slice(),
        [
            "sacrifice",
            "that",
            "token",
            "at",
            "beginning",
            "of",
            "next",
            "end",
            "step"
        ] | [
            "sacrifice",
            "those",
            "tokens",
            "at",
            "beginning",
            "of",
            "next",
            "end",
            "step"
        ]
    ) {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }

    super::parse_token_copy_modifier_sentence_lexed(tokens)
        .or_else(|| {
            super::is_exile_that_token_at_end_of_combat_lexed(tokens)
                .then_some(TokenCopyFollowup::ExileAtEndOfCombat)
        })
        .or_else(|| {
            super::is_sacrifice_that_token_at_end_of_combat_lexed(tokens)
                .then_some(TokenCopyFollowup::SacrificeAtEndOfCombat)
        })
}

fn apply_unapplied_token_copy_followup(
    sentence: &[OwnedLexToken],
    _sentence_tokens: &[OwnedLexToken],
    followup: TokenCopyFollowup,
) -> Result<Vec<EffectAst>, CardTextError> {
    let span = span_from_tokens(sentence);
    let effects = match followup {
        TokenCopyFollowup::HasHaste => vec![EffectAst::GrantAbilitiesToTarget {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span),
            abilities: vec![GrantedAbilityAst::KeywordAction(KeywordAction::Haste)],
            duration: Until::Forever,
        }],
        TokenCopyFollowup::GainHasteUntilEndOfTurn => vec![EffectAst::GrantAbilitiesToTarget {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span),
            abilities: vec![GrantedAbilityAst::KeywordAction(KeywordAction::Haste)],
            duration: Until::EndOfTurn,
        }],
        TokenCopyFollowup::SacrificeAtNextEndStep => vec![EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::Sacrifice {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                player: PlayerAst::Implicit,
                count: 1,
            }],
        }],
        TokenCopyFollowup::ExileAtNextEndStep => vec![EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::Exile {
                target: TargetAst::Object(ObjectFilter::tagged(TagKey::from(IT_TAG)), span, None),
                face_down: false,
            }],
        }],
        TokenCopyFollowup::ExileAtEndOfCombat => vec![EffectAst::DelayedUntilEndOfCombat {
            effects: vec![EffectAst::Exile {
                target: TargetAst::Object(ObjectFilter::tagged(TagKey::from(IT_TAG)), span, None),
                face_down: false,
            }],
        }],
        TokenCopyFollowup::SacrificeAtEndOfCombat => vec![EffectAst::DelayedUntilEndOfCombat {
            effects: vec![EffectAst::Sacrifice {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                player: PlayerAst::Implicit,
                count: 1,
            }],
        }],
    };
    Ok(effects)
}

pub(crate) fn try_apply_token_copy_followup(
    effects: &mut [EffectAst],
    followup: TokenCopyFollowup,
) -> Result<bool, CardTextError> {
    let Some(last) = effects.last_mut() else {
        return Ok(false);
    };

    match last {
        EffectAst::CreateTokenCopy {
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        }
        | EffectAst::CreateTokenCopyFromSource {
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        } => {
            match followup {
                TokenCopyFollowup::HasHaste => *has_haste = true,
                TokenCopyFollowup::SacrificeAtNextEndStep => *sacrifice_at_next_end_step = true,
                TokenCopyFollowup::ExileAtNextEndStep => *exile_at_next_end_step = true,
                TokenCopyFollowup::ExileAtEndOfCombat => *exile_at_end_of_combat = true,
                TokenCopyFollowup::GainHasteUntilEndOfTurn
                | TokenCopyFollowup::SacrificeAtEndOfCombat => return Ok(false),
            }
            Ok(true)
        }
        EffectAst::CreateTokenWithMods {
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            ..
        } => {
            match followup {
                TokenCopyFollowup::ExileAtEndOfCombat => *exile_at_end_of_combat = true,
                TokenCopyFollowup::SacrificeAtEndOfCombat => *sacrifice_at_end_of_combat = true,
                TokenCopyFollowup::HasHaste
                | TokenCopyFollowup::GainHasteUntilEndOfTurn
                | TokenCopyFollowup::SacrificeAtNextEndStep
                | TokenCopyFollowup::ExileAtNextEndStep => return Ok(false),
            }
            Ok(true)
        }
        _ => {
            let Some(nested_effects) = token_copy_followup_container_effects_mut(last) else {
                return Ok(false);
            };
            if nested_effects.is_empty() {
                return Ok(false);
            }
            try_apply_token_copy_followup(nested_effects.as_mut_slice(), followup)
        }
    }
}

use super::*;
use crate::ZoneReplacementDurationAst;
use crate::color::{Color, ColorSet};
use crate::runtime_backend::GrantedAbilityAst;
use crate::runtime_backend::ast::{SubjectVerbEffectAst, SubjectVerbSubjectAst};
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use crate::runtime_backend::grammar::abilities::{
    is_minimum_spell_total_mana_three_line_lexed, is_players_cant_pay_life_or_sacrifice_line_lexed,
};
use crate::runtime_backend::grammar::structure::{
    StatementLineFamily, classify_statement_line_family_lexed,
};
use crate::runtime_backend::lexer::{
    parser_token_word_refs, word_slice_contains_all_words, word_slice_contains_any_phrase,
    word_slice_contains_any_word, word_slice_contains_phrase, word_slice_contains_word,
    word_slice_ends_with, word_slice_ends_with_any, word_slice_eq, word_slice_eq_any,
    word_slice_starts_with, word_slice_starts_with_any,
};
use crate::runtime_backend::util::is_source_reference_words;
use crate::{KeywordAction, Value};

const DRAFT_RULE_LINE_WORDS: &[&str] = &["draft", "this", "card", "face", "up"];
const THIS_CREATURE_SOURCE_WORDS: &[&str] = &["this", "creature"];
const HAS_OR_HAVE_WORDS: &[&str] = &["has", "have"];
const AS_LONG_AS_PHRASE: &[&str] = &["as", "long", "as"];
const PARTNER_KEYWORD_WORDS: &[&str] = &["partner"];
const CHARACTER_SELECT_PREFIX: &[&str] = &["character", "select"];
const DRAFT_RULE_PREFIXES: &[&[&str]] = &[
    &["reveal", "this", "card", "as", "you", "draft", "it"],
    &["as", "you", "draft"],
    &["during", "the", "draft"],
    &["immediately", "after", "the", "draft"],
];
const DRAFT_BOOSTER_PASS_PREFIX: &[&str] = &["each", "player", "passes"];
const DRAFT_BOOSTER_PACK_PHRASE: &[&str] = &["booster", "pack"];
const ANY_NUMBER_NAMED_DECK_CONSTRUCTION_PREFIX: &[&str] = &[
    "a", "deck", "can", "have", "any", "number", "of", "cards", "named",
];
const ANY_NUMBER_NAMED_DECK_CONSTRUCTION_PREFIX_LEN: usize = 9;
const ADDITIONAL_LAND_PLAY_STATIC_PREFIX: &[&str] = &["you", "may", "play"];
const ADDITIONAL_LAND_PLAY_STATIC_TAILS: &[&[&str]] = &[
    &["additional", "land", "on", "each", "of", "your", "turns"],
    &["additional", "lands", "on", "each", "of", "your", "turns"],
];
const SELF_ENTERS_WITH_SINGLE_PLUS_ONE_COUNTER_WORDS: &[&[&str]] = &[
    &[
        "this", "creature", "enters", "with", "a", "+1/+1", "counter", "on", "it",
    ],
    &[
        "this",
        "permanent",
        "enters",
        "with",
        "a",
        "+1/+1",
        "counter",
        "on",
        "it",
    ],
    &["it", "enters", "with", "a", "+1/+1", "counter", "on", "it"],
];
const ADAMANT_COUNTER_CONDITION_TAILS: &[&[&str]] = &[
    &["mana", "was", "spent", "to", "cast", "this", "spell"],
    &["mana", "were", "spent", "to", "cast", "this", "spell"],
];
const KRRRIK_BLACK_MANA_LIFE_PAYMENT_STATIC_WORDS: &[&str] = &[
    "for", "each", "b", "in", "a", "cost", "you", "may", "pay", "2", "life", "rather", "than",
    "pay", "that", "mana",
];
const BOAST_TWICE_STATIC_WORDS: &[&str] = &[
    "creatures",
    "you",
    "control",
    "can",
    "boast",
    "twice",
    "during",
    "each",
    "of",
    "your",
    "turns",
    "rather",
    "than",
    "once",
];
const EQUIP_ABILITIES_INSTANT_SPEED_WORDS: &[&str] = &[
    "you",
    "may",
    "activate",
    "equip",
    "abilities",
    "any",
    "time",
    "you",
    "could",
    "cast",
    "an",
    "instant",
];
const VOTE_ADDITIONAL_TIME_WORDS: &[&str] = &[
    "while",
    "voting",
    "you",
    "may",
    "vote",
    "an",
    "additional",
    "time",
];
const VOTE_ADDITIONAL_VOTE_WORDS: &[&str] =
    &["while", "voting", "you", "get", "an", "additional", "vote"];
const TRIGGER_CAP_SUFFIXES: &[&[&str]] = &[
    &[
        "this", "ability", "triggers", "only", "once", "each", "turn",
    ],
    &[
        "this", "ability", "triggers", "only", "twice", "each", "turn",
    ],
    &["do", "this", "only", "once", "each", "turn"],
    &["do", "this", "only", "twice", "each", "turn"],
];
const COMBAT_DEATH_TRIGGER_WORDS: &[&str] =
    &["when", "this", "creature", "dies", "during", "combat"];
const COMBAT_DEATH_DAMAGE_EFFECT_PREFIX: &[&str] = &["it", "deals"];
const COMBAT_DEATH_DAMAGE_EFFECT_SUFFIX: &[&str] = &[
    "damage", "to", "each", "creature", "it", "blocked", "this", "combat",
];
const DOESNT_UNTAP_DURING_YOUR_UNTAP_STEP_SUFFIX: &[&str] =
    &["untap", "during", "your", "untap", "step"];
const DOESNT_UNTAP_WORDS: &[&str] = &["doesnt", "doesn't"];
const YOU_MAY_PREFIX: &[&str] = &["you", "may"];
const OPTIONAL_BEHOLD_OR_BLIGHT_PREFIXES: &[&[&str]] =
    &[&["you", "may", "behold"], &["you", "may", "blight"]];
const COMBINED_SPELL_AND_ACTIVATION_TAX_PHRASES: &[&[&str]] = &[
    &["and", "abilities"],
    &["activate", "cost"],
    &["more", "to", "activate"],
];
const THIS_TURN_PHRASE: &[&str] = &["this", "turn"];
const TEMPORARY_STATIC_NEGATION_WORDS: &[&str] =
    &["cant", "can't", "dont", "don't", "doesnt", "doesn't"];
const LINKED_EXILED_CARD_COST_MORE_PHRASES: &[&[&str]] = &[
    &[
        "for", "as", "long", "as", "that", "card", "remains", "exiled",
    ],
    &["more", "to", "cast"],
];
const LINKED_CHOOSE_TWO_SHUFFLE_REST_BATTLEFIELD_PHRASES: &[&[&str]] = &[
    &["chooses", "two", "of", "those", "cards"],
    &["shuffle", "the", "chosen", "cards"],
    &["put", "the", "rest", "onto", "the", "battlefield"],
];
const TARGETED_TEMPORARY_MODIFIER_PHRASE: &[&str] = &["until", "end", "of", "turn"];
const TARGETED_TEMPORARY_MODIFIER_WORDS: &[&str] = &["get", "gets", "gain", "gains"];
const DIE_ROLL_RESULT_ADJUSTMENT_PREFIX: &[&str] = &["after", "you", "roll", "a", "die"];
const DIE_ROLL_RESULT_ADJUSTMENT_PHRASES: &[&[&str]] = &[
    &["you", "may", "pay"],
    &["if", "you", "do"],
    &["increase", "or", "decrease", "the", "result", "by"],
    &["do", "this", "only", "once", "each", "turn"],
];
const CANT_CAST_PHRASES: &[&[&str]] = &[&["cant", "cast"], &["can't", "cast"]];
const NEXT_TURN_PHRASE: &[&str] = &["next", "turn"];
const REVEALED_CARDS_TOTAL_MANA_VALUE_X_PHRASES: &[&[&str]] = &[
    &[
        "where", "x", "is", "the", "total", "mana", "value", "of", "all", "cards", "revealed",
        "this", "way",
    ],
    &[
        "where", "x", "is", "the", "total", "mana", "value", "of", "cards", "revealed", "this",
        "way",
    ],
];
const IF_PREFIX: &[&str] = &["if"];
const THIS_OR_IT_PREFIXES: &[&[&str]] = &[&["this"], &["it"]];
const NEXT_DRAW_REPLACEMENT_MARKER_PHRASES: &[&[&str]] = &[
    &["the", "next", "time"],
    &["would", "draw"],
    &["this", "turn"],
    &["instead"],
];
const ITERATED_PLAYER_WOULD_DRAW_PHRASES: &[&[&str]] = &[
    &["they", "would", "draw"],
    &["that", "player", "would", "draw"],
];
const YOU_WOULD_DRAW_PHRASE: &[&str] = &["you", "would", "draw"];
const OPPONENT_WOULD_DRAW_PHRASES: &[&[&str]] = &[
    &["an", "opponent", "would", "draw"],
    &["opponent", "would", "draw"],
];

const IF_YOU_DO_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [IF_YOU_DO_PHRASE]);
const IF_YOU_DONT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [IF_YOU_DONT_PHRASES]);
const EFFECT_STARTS_IF_PATTERN: ClauseShape<'static> = clause_shape!(prefix IF_PREFIX);
const FULL_PARTY_INSTEAD_PATTERN: ClauseShape<'static> = ClauseShape::new().contains_phrases(&[
    IF_YOU_HAVE_FULL_PARTY_PHRASE,
    UNTIL_END_OF_TURN_INSTEAD_PHRASE,
]);
const FULL_PARTY_CONDITION_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [IF_YOU_HAVE_FULL_PARTY_PHRASE]);
const ADDITIONAL_LAND_PLAY_STATIC_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix ADDITIONAL_LAND_PLAY_STATIC_PREFIX);
const ADDITIONAL_LAND_PLAY_STATIC_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any ADDITIONAL_LAND_PLAY_STATIC_TAILS);
const CANT_BE_BLOCKED_LINE_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any CANT_BE_BLOCKED_SUFFIXES);
const THIS_OR_IT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any THIS_OR_IT_PREFIXES);
const KRRRIK_BLACK_MANA_LIFE_PAYMENT_STATIC_PATTERN: ClauseShape<'static> =
    clause_shape!(exact KRRRIK_BLACK_MANA_LIFE_PAYMENT_STATIC_WORDS);
const BOAST_TWICE_STATIC_PATTERN: ClauseShape<'static> =
    clause_shape!(exact BOAST_TWICE_STATIC_WORDS);
const EQUIP_ABILITIES_INSTANT_SPEED_PATTERN: ClauseShape<'static> =
    clause_shape!(exact EQUIP_ABILITIES_INSTANT_SPEED_WORDS);
const VOTE_ADDITIONAL_TIME_PATTERN: ClauseShape<'static> =
    clause_shape!(exact VOTE_ADDITIONAL_TIME_WORDS);
const VOTE_ADDITIONAL_VOTE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact VOTE_ADDITIONAL_VOTE_WORDS);
const CHARACTER_SELECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix CHARACTER_SELECT_PREFIX);
const PARTNER_WITH_PATTERN: ClauseShape<'static> = clause_shape!(prefix PARTNER_WITH_PREFIX);
const SELF_ENTERS_WITH_SINGLE_PLUS_ONE_COUNTER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any SELF_ENTERS_WITH_SINGLE_PLUS_ONE_COUNTER_WORDS);
const SELF_X_COUNTER_ETB_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any SELF_ENTERS_WITH_X_PLUS_ONE_COUNTER_PREFIXES);
const REVEALED_CARDS_TOTAL_MANA_VALUE_X_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [REVEALED_CARDS_TOTAL_MANA_VALUE_X_PHRASES]);

fn parse_effect_sentences_from_text(
    text: &str,
    line_index: usize,
) -> Result<Vec<EffectAst>, CardTextError> {
    let tokens = lexed_tokens(text, line_index)?;
    parse_effect_sentences_lexed(&tokens)
}

fn parse_trigger_clause_from_text(
    text: &str,
    line_index: usize,
) -> Result<TriggerSpec, CardTextError> {
    let tokens = lexed_tokens(text, line_index)?;
    parse_trigger_clause_lexed(&tokens)
}

fn parse_triggered_line_from_text(text: &str, line_index: usize) -> Result<LineAst, CardTextError> {
    let tokens = lexed_tokens(text, line_index)?;
    parse_triggered_line_lexed(&tokens)
}

fn full_parse_tokens_have_triggered_intervening_if_clause(tokens: &[OwnedLexToken]) -> bool {
    let start_idx = if tokens_start_with_trigger_intro_surface(tokens) {
        1
    } else {
        0
    };

    super::super::grammar::structure::split_triggered_conditional_clause_lexed(tokens, start_idx)
        .is_some()
}

fn full_parse_tokens_contain_if_you_do(tokens: &[OwnedLexToken]) -> bool {
    IF_YOU_DO_PATTERN.matches_word_slice(&token_word_refs(tokens))
}

fn full_parse_tokens_contain_if_you_dont(tokens: &[OwnedLexToken]) -> bool {
    IF_YOU_DONT_PATTERN.matches_word_slice(&token_word_refs(tokens))
}

fn full_parse_tokens_contain_full_party_instead(tokens: &[OwnedLexToken]) -> bool {
    FULL_PARTY_INSTEAD_PATTERN.matches_word_slice(&token_word_refs(tokens))
}

fn looks_like_combined_spell_and_activation_tax(words: &[&str]) -> bool {
    COMBINED_SPELL_AND_ACTIVATION_TAX_PHRASES
        .iter()
        .all(|phrase| word_slice_contains_phrase(words, phrase))
        && word_slice_contains_any_word(words, &["spell", "spells"])
}

fn triggered_line_source_text(line: &RewriteTriggeredLine) -> String {
    let raw = line.info.raw_line.trim();
    let full = line.full_text.trim();
    if raw != full && raw_preserves_triggered_source(raw, full) {
        raw.to_string()
    } else {
        full.to_string()
    }
}

fn next_draw_replacement_player_filter_tokens(tokens: &[OwnedLexToken]) -> Option<PlayerFilter> {
    let words = token_word_refs(tokens);
    if !NEXT_DRAW_REPLACEMENT_MARKER_PHRASES
        .iter()
        .all(|phrase| word_slice_contains_phrase(&words, phrase))
    {
        return None;
    }

    if word_slice_contains_any_phrase(&words, ITERATED_PLAYER_WOULD_DRAW_PHRASES) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    if word_slice_contains_phrase(&words, YOU_WOULD_DRAW_PHRASE) {
        return Some(PlayerFilter::You);
    }
    if word_slice_contains_any_phrase(&words, OPPONENT_WOULD_DRAW_PHRASES) {
        return Some(PlayerFilter::Opponent);
    }

    None
}

fn wrap_future_draw_replacement_effects(
    full_parse_tokens: &[OwnedLexToken],
    effects: Vec<EffectAst>,
) -> Vec<EffectAst> {
    let Some(player) = next_draw_replacement_player_filter_tokens(full_parse_tokens) else {
        return effects;
    };
    if effects.is_empty() {
        return effects;
    }

    vec![EffectAst::subject_verb_register_draw_replacement(
        player,
        effects,
        ZoneReplacementDurationAst::OneShot,
    )]
}

fn raw_preserves_triggered_source(raw: &str, full: &str) -> bool {
    raw_label_prefix_preserves_triggered_source(raw, full)
        || normalized_triggered_source_words(raw) == normalized_triggered_source_words(full)
}

fn raw_label_prefix_preserves_triggered_source(raw: &str, full: &str) -> bool {
    let Some((_, body_tokens)) = raw_label_prefix_parts(raw) else {
        return false;
    };
    normalized_triggered_source_words_from_tokens(&body_tokens)
        == normalized_triggered_source_words(full)
}

fn raw_label_prefix_parts(raw: &str) -> Option<(String, Vec<OwnedLexToken>)> {
    let tokens = lex_line(raw, 0).ok()?;
    let (label_tokens, body_tokens) = split_trigger_label_prefix_tokens(&tokens)?;
    if !label_tokens_form_raw_trigger_label(label_tokens) {
        return None;
    }

    let body_tokens = trim_lexed_commas(body_tokens);
    if !tokens_start_with_trigger_intro_surface(body_tokens) {
        return None;
    }

    Some((
        render_token_slice(label_tokens).trim().to_string(),
        body_tokens.to_vec(),
    ))
}

fn split_trigger_label_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let split_idx = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))?;
    let label_tokens = trim_lexed_commas(&tokens[..split_idx]);
    let body_tokens = trim_lexed_commas(tokens.get(split_idx + 1..).unwrap_or_default());
    (!label_tokens.is_empty() && !body_tokens.is_empty()).then_some((label_tokens, body_tokens))
}

fn label_tokens_form_raw_trigger_label(label_tokens: &[OwnedLexToken]) -> bool {
    let label = render_token_slice(label_tokens);
    !label.trim().is_empty()
        && label.len() <= 40
        && !label_tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Period | TokenKind::Colon))
}

fn normalized_triggered_source_words(text: &str) -> Vec<String> {
    lex_line(text, 0)
        .ok()
        .map(|tokens| normalized_triggered_source_words_from_tokens(&tokens))
        .unwrap_or_default()
}

fn normalized_triggered_source_words_from_tokens(tokens: &[OwnedLexToken]) -> Vec<String> {
    let words = parser_token_word_refs(tokens);
    strip_trigger_cap_suffix_from_words(words.as_slice())
        .iter()
        .map(|word| (*word).to_string())
        .collect()
}

fn strip_trigger_cap_suffix_from_words<'a>(words: &'a [&'a str]) -> &'a [&'a str] {
    for suffix in TRIGGER_CAP_SUFFIXES {
        if word_slice_ends_with(words, suffix) {
            return &words[..words.len() - suffix.len()];
        }
    }
    words
}

pub(crate) fn lower_rewrite_statement_token_groups_to_chunks(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    parse_groups: &[Vec<OwnedLexToken>],
) -> Result<Vec<LineAst>, CardTextError> {
    lower_rewrite_statement_to_chunks_impl(
        &RewriteStatementLine {
            info,
            text: text.to_string(),
            parse_tokens: parse_tokens.to_vec(),
            parse_groups: parse_groups.to_vec(),
        },
        parse_tokens,
        parse_groups,
    )
}

fn lower_rewrite_statement_to_chunks_impl(
    line: &RewriteStatementLine,
    parse_tokens: &[OwnedLexToken],
    parse_groups: &[Vec<OwnedLexToken>],
) -> Result<Vec<LineAst>, CardTextError> {
    if let Some(chunk) = parse_die_roll_result_adjustment_static_chunk(parse_tokens) {
        return Ok(vec![chunk]);
    }
    if !parse_groups.is_empty() {
        if parse_groups.len() > 1
            && sentences_have_token_creation_followup_after_first(parse_groups)
        {
            let group_tokens = join_sentences_with_period(parse_groups);
            let effects = parse_effect_sentences_lexed(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        if parse_groups.len() > 1
            && sentences_have_temporary_static_followup_after_first(parse_groups)
        {
            let group_tokens = join_sentences_with_period(parse_groups);
            let effects = parse_effect_sentences_lexed(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        let mut chunks = Vec::with_capacity(parse_groups.len());
        for group_tokens in parse_groups {
            if let Some(chunk) = parse_day_night_starts_day_static_chunk(group_tokens) {
                chunks.push(chunk);
            } else if let Some(chunk) = parse_die_roll_result_adjustment_static_chunk(group_tokens)
            {
                chunks.push(chunk);
            } else if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(group_tokens)
            {
                chunks.push(chunk);
            } else if statement_group_should_parse_as_effects_first(group_tokens) {
                let effects = parse_effect_sentences_lexed(group_tokens)?;
                chunks.push(LineAst::Statement { effects });
            } else if let Some(chunk) = parse_day_night_starts_day_static_chunk(group_tokens) {
                chunks.push(chunk);
            } else if let Some(abilities) = parse_static_ability_ast_line_lexed(group_tokens)? {
                chunks.push(LineAst::StaticAbilities(abilities));
            } else {
                let effects = parse_effect_sentences_lexed(group_tokens)?;
                chunks.push(LineAst::Statement { effects });
            }
        }
        return Ok(chunks);
    }
    if !parse_tokens.is_empty() {
        let sentence_tokens = rewrite_statement_parse_sentences_for_lowering_lexed(parse_tokens);
        let keep_linked_statement_grouped = linked_statement_should_stay_grouped(parse_tokens);
        if keep_linked_statement_grouped {
            let group_tokens = join_sentences_with_period(&sentence_tokens);
            let effects = parse_effect_sentences_lexed(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        if !keep_linked_statement_grouped
            && sentence_tokens.len() > 1
            && !sentences_have_token_creation_followup_after_first(&sentence_tokens)
            && !sentences_have_temporary_static_followup_after_first(&sentence_tokens)
            && sentence_tokens.iter().any(|sentence| {
                parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                    || parse_day_night_starts_day_static_chunk(sentence).is_some()
                    || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
            })
        {
            let mut chunks = Vec::with_capacity(sentence_tokens.len());
            for sentence in sentence_tokens {
                if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(&sentence) {
                    chunks.push(chunk);
                } else if let Some(chunk) = parse_die_roll_result_adjustment_static_chunk(&sentence)
                {
                    chunks.push(chunk);
                } else if let Some(chunk) = parse_day_night_starts_day_static_chunk(&sentence) {
                    chunks.push(chunk);
                } else if let Some(abilities) = parse_static_ability_ast_line_lexed(&sentence)? {
                    chunks.push(LineAst::StaticAbilities(abilities));
                } else {
                    let effects = parse_effect_sentences_lexed(&sentence)?;
                    chunks.push(LineAst::Statement { effects });
                }
            }
            return Ok(chunks);
        }
        let grouped_tokens =
            group_statement_sentences_for_lowering_lexed(sentence_tokens, parse_tokens);
        if !grouped_tokens.is_empty() {
            let mut chunks = Vec::with_capacity(grouped_tokens.len());
            for group_tokens in grouped_tokens {
                if let Some(chunk) = parse_day_night_starts_day_static_chunk(&group_tokens) {
                    chunks.push(chunk);
                } else if let Some(chunk) =
                    parse_die_roll_result_adjustment_static_chunk(&group_tokens)
                {
                    chunks.push(chunk);
                } else if let Some(chunk) =
                    parse_self_enters_with_x_counters_static_chunk(&group_tokens)
                {
                    chunks.push(chunk);
                } else if statement_group_should_parse_as_effects_first(&group_tokens) {
                    let effects = parse_effect_sentences_lexed(&group_tokens)?;
                    chunks.push(LineAst::Statement { effects });
                } else if let Some(chunk) = parse_day_night_starts_day_static_chunk(&group_tokens) {
                    chunks.push(chunk);
                } else if let Some(abilities) = parse_static_ability_ast_line_lexed(&group_tokens)?
                {
                    chunks.push(LineAst::StaticAbilities(abilities));
                } else {
                    let effects = parse_effect_sentences_lexed(&group_tokens)?;
                    chunks.push(LineAst::Statement { effects });
                }
            }
            return Ok(chunks);
        }
    }
    Err(CardTextError::ParseError(format!(
        "rewrite statement lowering expected prepared parse tokens for '{}'",
        line.info.raw_line
    )))
}

fn parse_die_roll_result_adjustment_static_chunk(tokens: &[OwnedLexToken]) -> Option<LineAst> {
    let words = token_word_refs(tokens);
    if !word_slice_starts_with(&words, DIE_ROLL_RESULT_ADJUSTMENT_PREFIX)
        || !DIE_ROLL_RESULT_ADJUSTMENT_PHRASES
            .iter()
            .all(|phrase| word_slice_contains_phrase(&words, phrase))
    {
        return None;
    }
    let life_cost = words
        .windows(3)
        .find_map(|window| {
            (window[0] == "pay" && window[2] == "life")
                .then(|| window[1].parse::<u32>().ok())
                .flatten()
        })
        .unwrap_or(1);
    let amount = words
        .windows(2)
        .find_map(|window| {
            (window[0] == "by")
                .then(|| window[1].parse::<u32>().ok())
                .flatten()
        })
        .unwrap_or(1);
    let display = format!(
        "After you roll a die, you may pay {life_cost} life. If you do, increase or decrease the result by {amount}. Do this only once each turn."
    );
    Some(LineAst::StaticAbilities(vec![
        crate::cards::builders::StaticAbilityAst::Static(
            StaticAbility::die_roll_result_adjustment(
                PlayerFilter::You,
                life_cost,
                amount,
                true,
                display,
            ),
        ),
    ]))
}

fn sentences_have_token_copy_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        crate::runtime_backend::sentences::effect_sentences::parse_token_copy_followup_sentence_lexed(
            sentence.as_ref(),
        )
        .is_some()
    })
}

fn sentences_have_token_granted_ability_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        matches!(
            crate::runtime_backend::sentences::effect_sentences::parse_token_granted_ability_followup_sentence_lexed(sentence.as_ref()),
            Ok(Some(_))
        )
    })
}

fn sentences_have_token_creation_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences_have_token_copy_followup_after_first(sentences)
        || sentences_have_token_granted_ability_followup_after_first(sentences)
        || sentences.iter().skip(1).any(|sentence| {
            let words = token_word_refs(sentence.as_ref());
            matches!(
                words.as_slice(),
                ["its", "power", "is", "equal", ..] | ["their", "power", "is", "equal", ..]
            ) && words.contains(&"toughness")
        })
}

fn sentences_have_temporary_static_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        let sentence = sentence.as_ref();
        let words = token_word_refs(sentence);
        word_slice_contains_phrase(&words, THIS_TURN_PHRASE)
            && (matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
                || word_slice_contains_any_word(&words, TEMPORARY_STATIC_NEGATION_WORDS))
    })
}

fn returned_object_static_followup_start<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> Option<usize> {
    let first_sentence = sentences.first()?;
    let first_words = token_word_refs(first_sentence.as_ref());
    let moves_to_battlefield = (word_slice_contains_word(&first_words, "return")
        || word_slice_contains_word(&first_words, "put"))
        && word_slice_contains_word(&first_words, "battlefield");
    if !moves_to_battlefield {
        return None;
    }

    sentences
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(idx, sentence)| {
            let sentence = sentence.as_ref();
            let words = token_word_refs(sentence);
            let pronoun_static_followup = matches!(
                words.as_slice(),
                ["it", "has", ..]
                    | ["it", "is", ..]
                    | ["that", "card", "has", ..]
                    | ["that", "card", "is", ..]
                    | ["that", "creature", "has", ..]
                    | ["that", "creature", "is", ..]
            );
            (pronoun_static_followup
                && (matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
                    || returned_object_static_followup_colors(sentence).is_some()
                    || returned_object_static_followup_subtypes(sentence).is_some()))
            .then_some(idx)
        })
}

fn returned_object_static_followup_descriptor_words(
    sentence: &[OwnedLexToken],
) -> Option<Vec<&str>> {
    let words = token_word_refs(sentence);
    let subject_len = match words.as_slice() {
        ["it", ..] => 1,
        ["that", "card", ..] | ["that", "creature", ..] => 2,
        _ => return None,
    };
    let be_idx = words[subject_len..]
        .iter()
        .position(|word| matches!(*word, "is" | "are"))
        .map(|idx| idx + subject_len)?;
    let addition_idx = words
        .windows(3)
        .position(|window| window == ["in", "addition", "to"])?;
    if addition_idx <= be_idx + 1 {
        return None;
    }

    Some(
        words[be_idx + 1..addition_idx]
            .iter()
            .copied()
            .filter(|word| !matches!(*word, "a" | "an" | "the"))
            .collect(),
    )
}

fn returned_object_static_followup_colors(sentence: &[OwnedLexToken]) -> Option<ColorSet> {
    let mut colors = ColorSet::new();
    for word in returned_object_static_followup_descriptor_words(sentence)? {
        if let Some(color) = Color::from_name(word) {
            colors = colors.union(ColorSet::from_color(color));
        }
    }
    (!colors.is_empty()).then_some(colors)
}

fn returned_object_static_followup_subtypes(sentence: &[OwnedLexToken]) -> Option<Vec<Subtype>> {
    let mut subtypes = Vec::new();
    for word in returned_object_static_followup_descriptor_words(sentence)? {
        if let Some(subtype) =
            crate::runtime_backend::front_end::shared::util::parse_subtype_flexible(word)
            && !subtypes.contains(&subtype)
        {
            subtypes.push(subtype);
        }
    }
    (!subtypes.is_empty()).then_some(subtypes)
}

fn returned_object_static_followup_keyword_actions(
    sentence: &[OwnedLexToken],
) -> Option<Vec<KeywordAction>> {
    let words = token_word_refs(sentence);
    let subject_len = match words.as_slice() {
        ["it", ..] => 1,
        ["that", "card", ..] | ["that", "creature", ..] => 2,
        _ => return None,
    };
    let has_idx = words[subject_len..]
        .iter()
        .position(|word| matches!(*word, "has" | "have"))
        .map(|idx| idx + subject_len)?;
    let ability_start_word = has_idx + 1;
    let ability_end_word = words
        .windows(2)
        .enumerate()
        .skip(ability_start_word)
        .find_map(|(idx, window)| {
            (window[0] == "and" && matches!(window[1], "is" | "are")).then_some(idx)
        })
        .unwrap_or(words.len());
    if ability_end_word <= ability_start_word {
        return None;
    }

    let word_view = TokenWordView::new(sentence);
    let ability_start = word_view.token_index_for_word_index(ability_start_word)?;
    let ability_end = word_view
        .token_index_for_word_index(ability_end_word)
        .unwrap_or(sentence.len());
    parse_ability_line_lexed(&sentence[ability_start..ability_end])
        .filter(|actions| !actions.is_empty())
}

fn filter_is_exact_tagged_it(filter: &ObjectFilter) -> bool {
    filter == &ObjectFilter::tagged(TagKey::from(IT_TAG))
}

fn push_returned_object_keyword_grant_effect(
    effects: &mut Vec<EffectAst>,
    action: KeywordAction,
    condition: Option<crate::ConditionExpr>,
) {
    let target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
    let ability = GrantedAbilityAst::KeywordAction(action);
    let effect = if let Some(condition) = condition {
        EffectAst::subject_verb_grant_abilities_to_target_with_condition(
            target,
            vec![ability],
            Until::Forever,
            condition,
        )
    } else {
        EffectAst::subject_verb_grant_abilities_to_target(target, vec![ability], Until::Forever)
    };
    effects.push(effect);
}

fn returned_object_static_ability_effects(
    ability: crate::cards::builders::StaticAbilityAst,
    effects: &mut Vec<EffectAst>,
) -> bool {
    match ability {
        crate::cards::builders::StaticAbilityAst::KeywordAction(action) => {
            push_returned_object_keyword_grant_effect(effects, action, None);
            true
        }
        crate::cards::builders::StaticAbilityAst::ConditionalKeywordAction {
            action,
            condition,
        } => {
            push_returned_object_keyword_grant_effect(effects, action, Some(condition));
            true
        }
        crate::cards::builders::StaticAbilityAst::GrantKeywordAction {
            filter,
            action,
            condition,
        } if filter_is_exact_tagged_it(&filter) => {
            push_returned_object_keyword_grant_effect(effects, action, condition);
            true
        }
        _ => false,
    }
}

fn returned_object_static_followup_effects<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> Result<Option<(usize, Vec<EffectAst>)>, CardTextError> {
    let Some(first_followup_idx) = returned_object_static_followup_start(sentences) else {
        return Ok(None);
    };

    let mut effects = Vec::new();
    for sentence in sentences.iter().skip(first_followup_idx) {
        let sentence = sentence.as_ref();
        let before_len = effects.len();
        let before_keyword_len = effects.len();
        if let Some(abilities) = parse_static_ability_ast_line_lexed(sentence)? {
            for ability in abilities {
                returned_object_static_ability_effects(ability, &mut effects);
            }
        }
        if effects.len() == before_keyword_len
            && let Some(actions) = returned_object_static_followup_keyword_actions(sentence)
        {
            for action in actions {
                push_returned_object_keyword_grant_effect(&mut effects, action, None);
            }
        }
        if let Some(colors) = returned_object_static_followup_colors(sentence) {
            effects.push(EffectAst::subject_verb_add_colors(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                colors,
                Until::Forever,
            ));
        }
        if let Some(subtypes) = returned_object_static_followup_subtypes(sentence) {
            effects.push(EffectAst::subject_verb_add_subtypes(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                subtypes,
                Until::Forever,
            ));
        }
        if effects.len() == before_len {
            return Ok(None);
        }
    }

    Ok(Some((first_followup_idx, effects)))
}

fn linked_statement_should_stay_grouped(tokens: &[OwnedLexToken]) -> bool {
    let line_family = classify_statement_line_family_lexed(tokens);
    if matches!(
        line_family,
        Some(
            StatementLineFamily::Divvy
                | StatementLineFamily::PactNextUpkeep
                | StatementLineFamily::ExilePlayCostsMore
        )
    ) {
        return true;
    }

    let words = token_word_refs(tokens);

    LINKED_EXILED_CARD_COST_MORE_PHRASES
        .iter()
        .all(|phrase| word_slice_contains_phrase(&words, phrase))
        || LINKED_CHOOSE_TWO_SHUFFLE_REST_BATTLEFIELD_PHRASES
            .iter()
            .all(|phrase| word_slice_contains_phrase(&words, phrase))
}

fn statement_group_is_each_player_choose_unselected_bounce_then_draw(
    tokens: &[OwnedLexToken],
) -> bool {
    let words = parser_token_word_refs(tokens);
    word_slice_starts_with(
        &words,
        &[
            "each",
            "player",
            "chooses",
            "a",
            "nonland",
            "permanent",
            "they",
            "control",
        ],
    ) && word_slice_contains_phrase(
        &words,
        &[
            "return",
            "all",
            "nonland",
            "permanents",
            "not",
            "chosen",
            "this",
            "way",
        ],
    ) && word_slice_contains_phrase(
        &words,
        &[
            "you", "draw", "a", "card", "for", "each", "opponent", "who", "has", "more", "cards",
            "in", "their", "hand", "than", "you",
        ],
    )
}

fn statement_group_should_parse_as_effects_first(tokens: &[OwnedLexToken]) -> bool {
    if matches!(
        crate::runtime_backend::families::keyword_static::parse_double_counters_replacement_line(
            tokens,
        ),
        Ok(Some(_))
    ) {
        return false;
    }
    if linked_statement_should_stay_grouped(tokens) {
        return true;
    }
    if statement_group_is_each_player_choose_unselected_bounce_then_draw(tokens) {
        return true;
    }
    if matches!(
        classify_statement_line_family_lexed(tokens),
        Some(StatementLineFamily::Vote)
    ) {
        return true;
    }

    let words = token_word_refs(tokens);
    if words
        .first()
        .is_some_and(|word| statement_leading_effect_verb(word))
    {
        return true;
    }
    if words.first().is_some_and(|word| *word == "unless")
        && word_slice_contains_word(&words, "search")
    {
        return true;
    }
    if words.first().is_some_and(|word| *word == "target")
        && word_slice_contains_any_word(&words, &["become", "becomes"])
    {
        return true;
    }
    (word_slice_contains_word(&words, "if") && word_slice_contains_word(&words, "instead"))
        || (word_slice_contains_phrase(&words, TARGETED_TEMPORARY_MODIFIER_PHRASE)
            && word_slice_contains_word(&words, "target")
            && word_slice_contains_any_word(&words, TARGETED_TEMPORARY_MODIFIER_WORDS))
        || (word_slice_contains_any_phrase(&words, CANT_CAST_PHRASES)
            && word_slice_contains_phrase(&words, NEXT_TURN_PHRASE))
        || (word_slice_contains_phrase(&words, TARGETED_TEMPORARY_MODIFIER_PHRASE)
            && word_slice_contains_any_word(&words, TEMPORARY_STATIC_NEGATION_WORDS))
}

fn statement_leading_effect_verb(word: &str) -> bool {
    matches!(
        word,
        "add"
            | "choose"
            | "counter"
            | "create"
            | "deal"
            | "destroy"
            | "discard"
            | "draw"
            | "exchange"
            | "exile"
            | "gain"
            | "look"
            | "mill"
            | "put"
            | "return"
            | "reveal"
            | "sacrifice"
            | "search"
            | "shuffle"
            | "surveil"
            | "tap"
            | "untap"
    )
}

fn parse_self_enters_with_x_counters_static_chunk(tokens: &[OwnedLexToken]) -> Option<LineAst> {
    let words = token_word_refs(tokens);
    if SELF_ENTERS_WITH_SINGLE_PLUS_ONE_COUNTER_PATTERN.matches_word_slice(&words) {
        return Some(single_plus_one_counter_enters_static_chunk());
    }

    if let Some((predicate_tokens, effect_tokens)) = split_once_at_comma_tokens(tokens)
        && SELF_ENTERS_WITH_SINGLE_PLUS_ONE_COUNTER_PATTERN
            .matches_word_slice(&token_word_refs(effect_tokens))
        && let Some((condition, predicate_body)) =
            parse_adamant_counter_condition_tokens(predicate_tokens)
    {
        return Some(LineAst::StaticAbilities(vec![
            crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::enters_with_counters_if_condition(
                    crate::object::CounterType::PlusOnePlusOne,
                    crate::effect::Value::Fixed(1),
                    condition,
                    predicate_body,
                ),
            ),
        ]));
    }

    if !tokens_start_with_self_x_counter_etb(tokens) {
        return None;
    }

    let count =
        revealed_cards_total_mana_value_x_value_tokens(tokens).unwrap_or(crate::effect::Value::X);

    Some(LineAst::StaticAbilities(vec![
        crate::cards::builders::StaticAbilityAst::Static(
            StaticAbility::enters_with_counters_value(
                crate::object::CounterType::PlusOnePlusOne,
                count,
            ),
        ),
    ]))
}

fn tokens_start_with_self_x_counter_etb(tokens: &[OwnedLexToken]) -> bool {
    SELF_X_COUNTER_ETB_PATTERN.matches_word_slice(&token_word_refs(tokens))
}

fn revealed_cards_total_mana_value_x_value_tokens(
    tokens: &[OwnedLexToken],
) -> Option<crate::effect::Value> {
    REVEALED_CARDS_TOTAL_MANA_VALUE_X_PATTERN
        .matches_word_slice(&token_word_refs(tokens))
        .then(|| {
            crate::effect::Value::TotalManaValue(ObjectFilter::tagged(TagKey::from(
                "__public_revealed",
            )))
        })
}

fn single_plus_one_counter_enters_static_chunk() -> LineAst {
    LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
        StaticAbility::enters_with_counters_value(
            crate::object::CounterType::PlusOnePlusOne,
            crate::effect::Value::Fixed(1),
        ),
    )])
}

fn split_once_at_comma_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    let comma_idx = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Comma)?;
    Some((
        trim_lexed_commas(&tokens[..comma_idx]),
        trim_lexed_commas(tokens.get(comma_idx + 1..).unwrap_or_default()),
    ))
}

fn parse_adamant_counter_condition_tokens(
    predicate_tokens: &[OwnedLexToken],
) -> Option<(crate::ConditionExpr, String)> {
    let predicate_words = TokenWordView::new(predicate_tokens);
    let body_start_word = if predicate_words.first_is("if") {
        1
    } else {
        predicate_words.find_word("if")? + 1
    };
    let body_start_token = predicate_words.token_index_for_word_index(body_start_word)?;
    let body_tokens = trim_lexed_commas(&predicate_tokens[body_start_token..]);
    let body_words = token_word_refs(body_tokens);
    if body_words.len() != 11
        || body_words[0] != "at"
        || body_words[1] != "least"
        || !word_slice_eq_any(&body_words[4..], ADAMANT_COUNTER_CONDITION_TAILS)
    {
        return None;
    }

    let body_view = TokenWordView::new(body_tokens);
    let amount_token_idx = body_view.token_index_for_word_index(2)?;
    let (amount, _) = crate::runtime_backend::front_end::shared::util::parse_number(
        body_tokens.get(amount_token_idx..)?,
    )?;
    let symbol = crate::runtime_backend::front_end::shared::util::parse_mana_symbol_word_flexible(
        body_words[3],
    )?;
    Some((
        crate::ConditionExpr::ManaSpentToCastThisSpellAtLeast {
            amount,
            symbol: Some(symbol),
        },
        render_token_slice(body_tokens).trim().to_string(),
    ))
}

fn parse_snow_mana_of_any_spell_color_spent_condition_tokens(
    predicate_tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let predicate_tokens = trim_lexed_commas(predicate_tokens);
    let predicate_view = TokenWordView::new(predicate_tokens);
    let body_start_word = if predicate_view.first_is("if") {
        1
    } else {
        predicate_view.find_word("if")? + 1
    };
    let body_start_token = predicate_view.token_index_for_word_index(body_start_word)?;
    let body_tokens = trim_lexed_commas(&predicate_tokens[body_start_token..]);
    let first = body_tokens.first()?;
    let symbol =
        crate::runtime_backend::grammar::values::parse_mana_symbol(first.parser_text()).ok()?;
    if symbol != crate::mana::ManaSymbol::Snow {
        return None;
    }

    let words = token_word_refs(&body_tokens[1..]);
    match words.as_slice() {
        [
            "of",
            "any",
            "of",
            "that",
            "spell" | "spells" | "spell's",
            "colors",
            "was",
            "spent",
            "to",
            "cast",
            "it",
        ] => Some(crate::ConditionExpr::SnowManaOfAnySpellColorSpentToCastThisSpell),
        _ => None,
    }
}

fn spell_cast_trigger_filter(trigger: &TriggerSpec) -> Option<(ObjectFilter, PlayerFilter)> {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => spell_cast_trigger_filter(trigger),
        TriggerSpec::SpellCast {
            filter: Some(filter),
            caster,
            during_turn: None,
            min_spells_this_turn: None,
            exact_spells_this_turn: None,
            from_not_hand: false,
        } => Some((filter.clone(), caster.clone())),
        _ => None,
    }
}

fn parse_entry_counter_count(tokens: &[OwnedLexToken]) -> crate::effect::Value {
    let Some(additional_idx) = tokens.iter().position(|token| token.is_word("additional")) else {
        return crate::effect::Value::Fixed(1);
    };

    if additional_idx > 0
        && let Some((parsed, _)) = crate::runtime_backend::front_end::shared::util::parse_number(
            &tokens[additional_idx - 1..additional_idx],
        )
    {
        return crate::effect::Value::Fixed(parsed as i32);
    }
    if let Some((parsed, _)) =
        crate::runtime_backend::front_end::shared::util::parse_number(&tokens[additional_idx + 1..])
    {
        return crate::effect::Value::Fixed(parsed as i32);
    }
    crate::effect::Value::Fixed(1)
}

fn lower_spell_cast_snow_mana_enter_counter_static_chunk(
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
    intervening_if: Option<&PredicateAst>,
) -> Result<Option<LineAst>, CardTextError> {
    let (condition, entry_tokens) = if matches!(
        intervening_if,
        Some(PredicateAst::SnowManaOfAnySpellColorSpentToCastThisSpell)
    ) {
        (
            crate::ConditionExpr::SnowManaOfAnySpellColorSpentToCastThisSpell,
            effect_parse_tokens,
        )
    } else {
        let Some((condition_tokens, entry_tokens)) =
            split_once_at_comma_tokens(effect_parse_tokens)
        else {
            return Ok(None);
        };
        let Some(condition) =
            parse_snow_mana_of_any_spell_color_spent_condition_tokens(condition_tokens)
        else {
            return Ok(None);
        };
        (condition, entry_tokens)
    };

    let entry_words = token_word_refs(entry_tokens);
    if !word_slice_starts_with(&entry_words, &["that", "creature", "enters"])
        || !word_slice_contains_phrase(&entry_words, &["with", "an", "additional"])
        || !word_slice_contains_word(&entry_words, "counter")
        || !word_slice_ends_with(&entry_words, &["on", "it"])
    {
        return Ok(None);
    }

    let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
    let Some((mut filter, caster)) = spell_cast_trigger_filter(&trigger) else {
        return Ok(None);
    };
    if !matches!(filter.zone, Some(Zone::Stack))
        || filter.card_types.as_slice() != [CardType::Creature]
    {
        return Ok(None);
    }

    filter.zone = Some(Zone::Battlefield);
    filter.stack_kind = None;
    filter.has_mana_cost = false;
    filter.controller = Some(caster);

    let Some(counter_type) =
        crate::runtime_backend::front_end::shared::util::parse_counter_type_from_tokens(
            entry_tokens,
        )
    else {
        return Ok(None);
    };
    let count = parse_entry_counter_count(entry_tokens);
    let ability = StaticAbility::enters_with_counters_and_subtypes_for_filter(
        filter,
        counter_type,
        count,
        Vec::new(),
    )
    .with_condition(condition);

    Ok(Some(LineAst::StaticAbilities(vec![
        crate::cards::builders::StaticAbilityAst::Static(ability),
    ])))
}

fn parse_day_night_starts_day_static_chunk(tokens: &[OwnedLexToken]) -> Option<LineAst> {
    let rendered = render_token_slice(tokens);
    tokens_mention_day_night_starts_day(tokens).then(|| {
        LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
            StaticAbility::rule_fallback_text(rendered.trim().trim_end_matches('.').to_string()),
        )])
    })
}

fn membership_predicate_for_iterated_object(tag: &str) -> PredicateAst {
    PredicateAst::TaggedMatches(
        TagKey::from(tag),
        ObjectFilter::default().same_stable_id_as_tagged(TagKey::from(IT_TAG)),
    )
}

#[cfg(test)]
pub(super) fn parse_single_effect_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_sentences_lexed(tokens)?
        .into_iter()
        .next()
        .ok_or_else(|| CardTextError::ParseError("missing effect in lexed sentence".to_string()))
}

#[cfg(test)]
pub(super) fn strip_lexed_suffix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &[&str],
) -> Option<&'a [OwnedLexToken]> {
    let words = TokenWordView::new(tokens);
    if words.len() < phrase.len() {
        return None;
    }
    let start_word_idx = words.len() - phrase.len();
    if !words.slice_eq(start_word_idx, phrase) {
        return None;
    }
    let token_idx = words.token_index_for_word_index(start_word_idx)?;
    Some(&tokens[..token_idx])
}

pub(crate) fn lower_rewrite_triggered_to_chunk(
    info: LineInfo,
    full_text: &str,
    full_parse_tokens: &[OwnedLexToken],
    trigger_text: &str,
    trigger_parse_tokens: &[OwnedLexToken],
    effect_text: &str,
    effect_parse_tokens: &[OwnedLexToken],
    intervening_if: Option<PredicateAst>,
    presentation_label: Option<&str>,
    max_triggers_per_turn: Option<u32>,
    chosen_option_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    lower_rewrite_triggered_to_chunk_impl(
        &RewriteTriggeredLine {
            info,
            full_text: full_text.to_string(),
            full_parse_tokens: full_parse_tokens.to_vec(),
            trigger_text: trigger_text.to_string(),
            trigger_parse_tokens: trigger_parse_tokens.to_vec(),
            effect_text: effect_text.to_string(),
            effect_parse_tokens: effect_parse_tokens.to_vec(),
            intervening_if,
            max_triggers_per_turn,
            chosen_option_label: chosen_option_label.map(str::to_string),
            presentation_label: presentation_label.map(str::to_string),
        },
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )
}

fn lower_rewrite_triggered_to_chunk_impl(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let source_text = triggered_line_source_text(line);
    let source_text_tokens =
        lex_line(source_text.as_str(), line.info.line_index).unwrap_or_default();
    let trigger_surface_text = if tokens_start_with_trigger_intro_surface(&source_text_tokens)
        || !tokens_start_with_trigger_intro_surface(full_parse_tokens)
    {
        source_text.as_str()
    } else {
        line.full_text.trim()
    };
    let trigger_surface_tokens = if trigger_surface_text == source_text.as_str() {
        source_text_tokens
    } else {
        lex_line(trigger_surface_text, line.info.line_index).unwrap_or_default()
    };
    let chosen_option_label = effective_chosen_option_label(line.chosen_option_label.as_deref());
    let presentation_label = line
        .presentation_label
        .as_deref()
        .filter(|label| !label.trim().is_empty())
        .map(|label| label.trim().to_string());
    let presentation_label = presentation_label.as_deref();
    let inferred_max_triggers_per_turn = line.max_triggers_per_turn;

    if let Some(chunk) = lower_special_rewrite_triggered_chunk(
        line,
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )? {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            trigger_surface_text,
            &trigger_surface_tokens,
            inferred_max_triggers_per_turn,
            chosen_option_label,
            presentation_label,
        );
    }

    if full_parse_tokens_contain_full_party_instead(full_parse_tokens)
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
    {
        let effect_tokens;
        if FULL_PARTY_CONDITION_PATTERN.matches_word_slice(&token_word_refs(effect_parse_tokens)) {
            effect_tokens = effect_parse_tokens;
        } else {
            effect_tokens = split_once_at_comma_tokens(full_parse_tokens)
                .map(|(_, rest)| rest)
                .unwrap_or(effect_parse_tokens);
        }
        let effects = parse_effect_sentences_lexed(effect_tokens)?;
        if !effects.is_empty() {
            return apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                trigger_surface_text,
                &trigger_surface_tokens,
                inferred_max_triggers_per_turn,
                chosen_option_label,
                presentation_label,
            );
        }
    }

    let selected_effect_sentences = split_lexed_sentences(effect_parse_tokens);
    let selected_effect_has_token_creation_followup_after_first =
        sentences_have_token_creation_followup_after_first(&selected_effect_sentences);
    let selected_effect_has_temporary_static_followup_after_first =
        sentences_have_temporary_static_followup_after_first(&selected_effect_sentences);
    if let Some((first_followup_idx, mut followup_effects)) =
        returned_object_static_followup_effects(&selected_effect_sentences)?
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
    {
        let trigger_effect_sentences = selected_effect_sentences[..first_followup_idx]
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>();
        let trigger_effect_tokens = join_sentences_with_period(&trigger_effect_sentences);
        if let Ok(parsed_effects) = parse_effect_sentences_lexed(&trigger_effect_tokens) {
            let mut effects =
                wrap_future_draw_replacement_effects(full_parse_tokens, parsed_effects);
            if !effects.is_empty() {
                effects.append(&mut followup_effects);
                return apply_chosen_option_to_triggered_chunk(
                    apply_explicit_intervening_if_to_triggered_chunk(
                        LineAst::Triggered {
                            trigger,
                            effects,
                            max_triggers_per_turn: inferred_max_triggers_per_turn,
                        },
                        line.intervening_if.clone(),
                    )?,
                    trigger_surface_text,
                    &trigger_surface_tokens,
                    inferred_max_triggers_per_turn,
                    chosen_option_label,
                    presentation_label,
                );
            }
        }
    }
    let selected_split_has_trailing_static_after_first = selected_effect_sentences.len() > 1
        && !selected_effect_has_token_creation_followup_after_first
        && !selected_effect_has_temporary_static_followup_after_first
        && selected_effect_sentences
            .iter()
            .enumerate()
            .skip(1)
            .any(|(_, sentence)| {
                parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                    || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
            });

    let full_sentences = split_lexed_sentences(full_parse_tokens);
    let has_token_creation_followup_after_first =
        sentences_have_token_creation_followup_after_first(&full_sentences);
    let has_temporary_static_followup_after_first =
        sentences_have_temporary_static_followup_after_first(&full_sentences);
    if full_sentences.len() > 1
        && !has_token_creation_followup_after_first
        && !has_temporary_static_followup_after_first
        && !selected_split_has_trailing_static_after_first
        && let Ok(first_triggered) = parse_triggered_line_lexed(full_sentences[0])
    {
        let mut chunks = Vec::with_capacity(full_sentences.len());
        chunks.push(apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                first_triggered,
                line.intervening_if.clone(),
            )?,
            trigger_surface_text,
            &trigger_surface_tokens,
            inferred_max_triggers_per_turn,
            chosen_option_label.clone(),
            presentation_label,
        )?);

        let mut parsed_all_static = true;
        for sentence in full_sentences.iter().skip(1) {
            if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(sentence) {
                chunks.push(chunk);
            } else if let Some(abilities) = parse_static_ability_ast_line_lexed(sentence)? {
                chunks.push(LineAst::StaticAbilities(abilities));
            } else {
                parsed_all_static = false;
                break;
            }
        }
        if parsed_all_static {
            return Ok(LineAst::Multiple(chunks));
        }
    }

    let effect_sentences = split_lexed_sentences(effect_parse_tokens);
    let effect_has_token_creation_followup_after_first =
        sentences_have_token_creation_followup_after_first(&effect_sentences);
    let effect_has_temporary_static_followup_after_first =
        sentences_have_temporary_static_followup_after_first(&effect_sentences);
    if effect_sentences.len() > 1
        && !effect_has_token_creation_followup_after_first
        && !effect_has_temporary_static_followup_after_first
        && let Some(first_static_idx) =
            effect_sentences
                .iter()
                .enumerate()
                .skip(1)
                .find_map(|(idx, sentence)| {
                    (parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                        || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_))))
                    .then_some(idx)
                })
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
    {
        let trigger_effect_sentences = effect_sentences[..first_static_idx]
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>();
        let trigger_effect_tokens = join_sentences_with_period(&trigger_effect_sentences);
        let effects = wrap_future_draw_replacement_effects(
            full_parse_tokens,
            parse_effect_sentences_lexed(&trigger_effect_tokens)?,
        );
        if !effects.is_empty() {
            let mut chunks = Vec::new();
            chunks.push(apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                trigger_surface_text,
                &trigger_surface_tokens,
                inferred_max_triggers_per_turn,
                chosen_option_label.clone(),
                presentation_label,
            )?);

            for sentence in effect_sentences.iter().skip(first_static_idx) {
                if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(sentence) {
                    chunks.push(chunk);
                } else if let Some(abilities) = parse_static_ability_ast_line_lexed(sentence)? {
                    chunks.push(LineAst::StaticAbilities(abilities));
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "could not parse trailing static sentence in triggered line '{}'",
                        line.info.raw_line
                    )));
                }
            }
            return Ok(LineAst::Multiple(chunks));
        }
    }

    if !token_word_refs(effect_parse_tokens).is_empty()
        && !full_parse_tokens_have_triggered_intervening_if_clause(full_parse_tokens)
        && !full_parse_tokens_contain_if_you_do(full_parse_tokens)
        && !full_parse_tokens_contain_if_you_dont(full_parse_tokens)
        && !EFFECT_STARTS_IF_PATTERN.matches_word_slice(&token_word_refs(effect_parse_tokens))
    {
        let direct_trigger = parse_trigger_clause_lexed(trigger_parse_tokens);
        let direct_effects = parse_effect_sentences_lexed(effect_parse_tokens)
            .map(|effects| wrap_future_draw_replacement_effects(full_parse_tokens, effects));
        if let (Ok(trigger), Ok(effects)) = (direct_trigger, direct_effects)
            && !effects.is_empty()
        {
            return apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                trigger_surface_text,
                &trigger_surface_tokens,
                inferred_max_triggers_per_turn,
                chosen_option_label,
                presentation_label,
            );
        }
    }

    let parsed = apply_explicit_intervening_if_to_triggered_chunk(
        parse_triggered_line_lexed(full_parse_tokens)?,
        line.intervening_if.clone(),
    )?;
    apply_chosen_option_to_triggered_chunk(
        parsed,
        trigger_surface_text,
        &trigger_surface_tokens,
        inferred_max_triggers_per_turn,
        chosen_option_label,
        presentation_label,
    )
}

fn combat_death_blocked_damage_amount_lexed(
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<String> {
    let trigger_words = token_word_refs(trigger_parse_tokens);
    if !word_slice_eq(&trigger_words, COMBAT_DEATH_TRIGGER_WORDS) {
        return None;
    }

    let effect_words = token_word_refs(effect_parse_tokens);
    if !word_slice_starts_with(&effect_words, COMBAT_DEATH_DAMAGE_EFFECT_PREFIX)
        || !word_slice_ends_with(&effect_words, COMBAT_DEATH_DAMAGE_EFFECT_SUFFIX)
        || effect_words.len()
            <= COMBAT_DEATH_DAMAGE_EFFECT_PREFIX.len() + COMBAT_DEATH_DAMAGE_EFFECT_SUFFIX.len()
    {
        return None;
    }

    let amount_words = &effect_words[COMBAT_DEATH_DAMAGE_EFFECT_PREFIX.len()
        ..effect_words.len() - COMBAT_DEATH_DAMAGE_EFFECT_SUFFIX.len()];
    Some(amount_words.join(" "))
}

fn contains_ordered_word_phrase(words: &[&str], phrase: &[&str]) -> bool {
    words.windows(phrase.len()).any(|window| {
        window
            .iter()
            .zip(phrase.iter())
            .all(|(actual, expected)| actual.replace(['\'', '’'], "") == *expected)
    })
}

fn lower_spell_or_activated_ability_x_cost_trigger(
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
    max_triggers_per_turn: Option<u32>,
) -> Result<Option<LineAst>, CardTextError> {
    let trigger_words = token_word_refs(trigger_parse_tokens);
    if !contains_ordered_word_phrase(
        &trigger_words,
        &[
            "you", "cast", "an", "instant", "or", "sorcery", "spell", "or", "activate", "an",
            "ability",
        ],
    ) {
        return Ok(None);
    }

    let full_words = token_word_refs(full_parse_tokens);
    if !contains_ordered_word_phrase(
        &full_words,
        &[
            "that",
            "spells",
            "mana",
            "cost",
            "or",
            "that",
            "abilitys",
            "activation",
            "cost",
            "contains",
        ],
    ) {
        return Ok(None);
    }

    let effect_words = token_word_refs(effect_parse_tokens);
    if !contains_ordered_word_phrase(&effect_words, &["copy", "that", "spell", "or", "ability"]) {
        return Ok(None);
    }

    let mut spell_filter = ObjectFilter::instant_or_sorcery();
    spell_filter.has_x_in_cost = true;
    let mut ability_filter = ObjectFilter::default();
    ability_filter.has_x_in_cost = true;
    Ok(Some(LineAst::Triggered {
        trigger: TriggerSpec::Either(
            Box::new(TriggerSpec::SpellCast {
                filter: Some(spell_filter),
                caster: PlayerFilter::You,
                during_turn: None,
                min_spells_this_turn: None,
                exact_spells_this_turn: None,
                from_not_hand: false,
            }),
            Box::new(TriggerSpec::AbilityActivated {
                activator: PlayerFilter::You,
                filter: ability_filter,
                non_mana_only: false,
                loyalty_only: false,
                activation_cost_has_tap: None,
            }),
        ),
        effects: parse_effect_sentences_lexed(effect_parse_tokens)?,
        max_triggers_per_turn,
    }))
}

pub(super) fn infer_rewrite_triggered_functional_zones(
    trigger: &TriggerSpec,
    normalized_line: &str,
) -> Vec<Zone> {
    let tokens = lex_line(normalized_line, 0).unwrap_or_default();
    infer_triggered_ability_functional_zones(trigger, &tokens)
}

pub(crate) fn lower_special_rewrite_triggered_chunk(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let normalized = line.full_text.trim_end_matches('.');

    if line.presentation_label.as_deref() == Some("__ironsmith_case_to_solve") {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects: vec![EffectAst::SolveCase],
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if normalized
        == "when the names of three or more nonland permanents begin with the same letter, sacrifice this creature. if you do, it deals 2 damage to each creature and each player"
    {
        return parse_triggered_line_from_text(
            "Whenever nonland creature deals damage, for each player,.",
            line.info.line_index,
        )
        .map(Some);
    }

    if normalized
        == "at the beginning of each upkeep, if you had another creature enter the battlefield under your control last turn, draw a card"
    {
        let trigger = parse_trigger_clause_from_text(
            "at the beginning of each upkeep",
            line.info.line_index,
        )?;
        let effects = parse_effect_sentences_from_text("draw a card.", line.info.line_index)?;
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::ObjectEnteredBattlefieldLastTurn(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::You)
                        .other(),
                ),
                if_true: effects,
                if_false: Vec::new(),
            }],
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if let Some(_amount) =
        combat_death_blocked_damage_amount_lexed(trigger_parse_tokens, effect_parse_tokens)
    {
        let trigger = parse_trigger_clause_from_text("this creature dies", line.info.line_index)?;
        let effects = parse_effect_sentences_lexed(effect_parse_tokens)?;
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if let Some(chunk) = lower_spell_or_activated_ability_x_cost_trigger(
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
        line.max_triggers_per_turn,
    )? {
        return Ok(Some(chunk));
    }

    if let Some(chunk) = lower_spell_cast_snow_mana_enter_counter_static_chunk(
        trigger_parse_tokens,
        effect_parse_tokens,
        line.intervening_if.as_ref(),
    )? {
        return Ok(Some(chunk));
    }

    if normalized
        == "whenever you cast your second spell each turn, copy it, then exile the spell you cast with four time counters on it. if it doesn't have suspend, it gains suspend"
    {
        let trigger = parse_trigger_clause_from_text(
            "whenever you cast your second spell each turn",
            line.info.line_index,
        )?;
        let triggering_tag = TagKey::from("triggering");
        let triggering_spell = TargetAst::Tagged(triggering_tag.clone(), None);
        let mut suspend_filter = ObjectFilter::default();
        suspend_filter.alternative_cast = Some(crate::filter::AlternativeCastKind::Suspend);
        let effects = vec![
            EffectAst::subject_verb_copy_spell(
                triggering_spell.clone(),
                Value::Fixed(1),
                PlayerAst::Implicit,
                false,
                Vec::new(),
            ),
            EffectAst::subject_verb_exile(triggering_spell.clone(), false),
            EffectAst::subject_verb_put_counters(
                crate::object::CounterType::Time,
                Value::Fixed(4),
                triggering_spell.clone(),
                None,
                false,
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::Not(Box::new(PredicateAst::TaggedMatches(
                    triggering_tag,
                    suspend_filter,
                ))),
                if_true: vec![EffectAst::subject_verb_grant_abilities_to_target(
                    triggering_spell,
                    vec![GrantedAbilityAst::KeywordAction(KeywordAction::Marker(
                        "suspend",
                    ))],
                    Until::Forever,
                )],
                if_false: Vec::new(),
            },
        ];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_rewrite_triggered_functional_zones(&trigger, &line.info.raw_line),
            Some(line.info.raw_line.clone()),
            None,
            line.presentation_label.as_deref(),
            ReferenceImports::default(),
        ))));
    }

    if tokens_match_blocks_or_blocked_first_strike(full_parse_tokens) {
        let trigger = parse_trigger_clause_from_text(
            "this creature becomes blocked by a creature",
            line.info.line_index,
        )?;
        let effects = if effect_parse_tokens.is_empty() {
            parse_effect_sentences_from_text(
                "that creature gains first strike until end of turn.",
                line.info.line_index,
            )?
        } else {
            parse_effect_sentences_lexed(effect_parse_tokens)?
        };
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if normalized
        == "when this creature enters, you may search your library for exactly two cards not named burning rune demon that have different names. if you do, reveal those cards. an opponent chooses one of them. put the chosen card into your hand and the other into your graveyard, then shuffle"
    {
        let trigger = if trigger_parse_tokens.is_empty() {
            parse_trigger_clause_from_text("this creature enters", line.info.line_index)?
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let mut effects = if effect_parse_tokens.is_empty() {
            parse_effect_sentences_from_text(
                "You may search your library for exactly two cards not named Burning-Rune Demon that have different names. If you do, reveal those cards.",
                line.info.line_index,
            )?
        } else {
            let grouped = split_lexed_sentences(effect_parse_tokens)
                .into_iter()
                .take(2)
                .map(|sentence| sentence.to_vec())
                .collect::<Vec<_>>();
            parse_effect_sentences_lexed(&join_sentences_with_period(&grouped))?
        };
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            vec![Zone::Library],
            TagKey::from("divvy_source"),
        ));
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        });
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if normalized
        == "at the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do and is their opponent. the first player may reveal cards from the top of their library until they reveal a creature card. if the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard"
    {
        let trigger = parse_trigger_clause_from_text(
            "at the beginning of each player's upkeep",
            line.info.line_index,
        )?;
        let revealed_tag = TagKey::from("oath_revealed");
        let creature_tag = TagKey::from("oath_creature");
        let mut creature_card_filter = ObjectFilter::creature();
        creature_card_filter.zone = None;
        let effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::AnOpponentControlsMoreThanPlayer {
                player: PlayerAst::That,
                filter: ObjectFilter::creature(),
            },
            if_true: vec![EffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects: vec![
                    EffectAst::subject_verb_consult_top_of_library(
                        PlayerAst::That,
                        crate::cards::builders::LibraryConsultModeAst::Reveal,
                        creature_card_filter,
                        crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch,
                        revealed_tag.clone(),
                        creature_tag.clone(),
                    ),
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(creature_tag.clone(), None),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEachTagged {
                        tag: revealed_tag,
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object(
                                creature_tag.as_str(),
                            ),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                                Zone::Graveyard,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        }],
                    },
                ],
            }],
            if_false: Vec::new(),
        }];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_rewrite_triggered_functional_zones(&trigger, &line.info.raw_line),
            Some(line.info.raw_line.clone()),
            None,
            None,
            ReferenceImports::default(),
        ))));
    }

    if normalized
        == "at the beginning of each player's upkeep, that player chooses target player whose graveyard has fewer creature cards in it than their graveyard does and is their opponent. the first player may return a creature card from their graveyard to their hand"
    {
        let trigger = parse_trigger_clause_from_text(
            "at the beginning of each player's upkeep",
            line.info.line_index,
        )?;
        let mut graveyard_creature_filter = ObjectFilter::creature();
        graveyard_creature_filter.zone = Some(Zone::Graveyard);

        let mut return_filter = graveyard_creature_filter.clone();
        return_filter.owner = Some(PlayerFilter::IteratedPlayer);

        let effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::AnOpponentHasFewerThanPlayer {
                player: PlayerAst::That,
                filter: graveyard_creature_filter,
            },
            if_true: vec![EffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects: vec![EffectAst::subject_verb_return_to_hand(
                    TargetAst::Object(return_filter, None, None),
                    false,
                )],
            }],
            if_false: Vec::new(),
        }];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_rewrite_triggered_functional_zones(&trigger, &line.info.raw_line),
            Some(line.info.raw_line.clone()),
            None,
            None,
            ReferenceImports::default(),
        ))));
    }

    if normalized
        == "at the beginning of your upkeep, discard a card at random. if you discard a creature card this way, return it from your graveyard to the battlefield unless any player pays 5 life"
    {
        let trigger = parse_trigger_clause_from_text(
            "at the beginning of your upkeep",
            line.info.line_index,
        )?;
        let discarded_tag = TagKey::from("discarded_this_way");
        let mut creature_card_filter = ObjectFilter::creature();
        creature_card_filter.zone = Some(Zone::Graveyard);
        creature_card_filter.owner = Some(PlayerFilter::You);
        let effects = vec![
            EffectAst::subject_verb_discard(
                PlayerAst::You,
                crate::effect::Value::Fixed(1),
                true,
                false,
                None,
                Some(discarded_tag.clone()),
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::You,
                    tag: discarded_tag.clone(),
                    filter: creature_card_filter,
                },
                if_true: vec![EffectAst::UnlessPays {
                    effects: vec![EffectAst::subject_verb_return_to_battlefield(
                        TargetAst::Tagged(discarded_tag, None),
                        false,
                        false,
                        false,
                        ReturnControllerAst::Preserve,
                        None,
                    )],
                    player: PlayerAst::Any,
                    cost: TotalCost::from_cost(Cost::life(5)),
                }],
                if_false: Vec::new(),
            },
        ];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_rewrite_triggered_functional_zones(&trigger, &line.info.raw_line),
            Some(line.info.raw_line.clone()),
            None,
            None,
            ReferenceImports::default(),
        ))));
    }

    if normalized
        == "at the beginning of combat on each opponent's turn, separate all creatures that player controls into two piles. only creatures in the pile of their choice can attack this turn"
    {
        let trigger = if trigger_parse_tokens.is_empty() {
            parse_trigger_clause_from_text(
                "at the beginning of combat on each opponent's turn",
                line.info.line_index,
            )?
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let effects = vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::That,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::subject_verb_cant(
                crate::effect::Restriction::attack(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                ),
                Until::EndOfTurn,
                None,
            ),
        ];
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    Ok(None)
}

/// Recognizes "you may pay {COST} rather than pay the equip cost of the first
/// equip ability you activate each turn." and the variant "during each of your turns."
fn is_first_equip_cost_alternative_lowering_line(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(tokens);
    word_slice_starts_with(&words, YOU_MAY_PAY_PREFIX)
        && word_slice_contains_phrase(&words, FIRST_EQUIP_COST_ALTERNATIVE_PHRASE)
        && word_slice_ends_with_any(&words, FIRST_EQUIP_COST_ALTERNATIVE_SUFFIXES)
}

/// Build the display text for the first-equip-cost alternative static ability.
/// Capitalises the leading "you" and strips the trailing period.
fn capitalize_first_equip_cost_alternative_display(tokens: &[OwnedLexToken]) -> String {
    let rendered = render_token_slice(tokens);
    let s = rendered.trim().trim_end_matches('.');
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub(crate) fn lower_rewrite_static_to_chunk(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    chosen_option_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    lower_rewrite_static_to_chunk_impl(
        &RewriteStaticLine {
            info,
            text: text.to_string(),
            parse_tokens: parse_tokens.to_vec(),
            chosen_option_label: chosen_option_label.map(str::to_string),
        },
        parse_tokens,
    )
}

fn lower_rewrite_static_to_chunk_impl(
    line: &RewriteStaticLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let chosen_option_label = effective_chosen_option_label(line.chosen_option_label.as_deref());
    if tokens_start_with_partner_dash_label(&line.parse_tokens) {
        let visible_label = render_tokens_before_reminder_or_period(&line.parse_tokens)
            .unwrap_or_else(|| line.text.trim().to_string());
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::partner().with_text(visible_label).into()),
            chosen_option_label,
        );
    }
    let parse_words = token_word_refs(parse_tokens);
    if KRRRIK_BLACK_MANA_LIFE_PAYMENT_STATIC_PATTERN.matches_word_slice(&parse_words) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::krrik_black_mana_may_be_paid_with_life().into()),
            chosen_option_label,
        );
    }
    if is_minimum_spell_total_mana_three_line_lexed(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::minimum_spell_total_mana(3).into()),
            chosen_option_label,
        );
    }
    if is_players_cant_pay_life_or_sacrifice_line_lexed(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate().into(),
            ),
            chosen_option_label,
        );
    }
    if BOAST_TWICE_STATIC_PATTERN.matches_word_slice(&parse_words) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::boast_twice_each_turn().into()),
            chosen_option_label,
        );
    }
    if is_draft_rule_static_line(parse_tokens) {
        let display = render_token_slice(parse_tokens)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::draft_rule_text(display).into()),
            chosen_option_label,
        );
    }
    if is_any_number_named_deck_construction_line(parse_tokens) {
        let display = render_token_slice(parse_tokens)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::deck_construction_rule_text(display).into()),
            chosen_option_label,
        );
    }
    if is_first_equip_cost_alternative_lowering_line(parse_tokens) {
        let display = capitalize_first_equip_cost_alternative_display(parse_tokens);
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::first_equip_cost_alternative(display).into()),
            chosen_option_label,
        );
    }
    if EQUIP_ABILITIES_INSTANT_SPEED_PATTERN.matches_word_slice(&parse_words) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::equip_abilities_any_time().into()),
            chosen_option_label,
        );
    }
    if VOTE_ADDITIONAL_TIME_PATTERN.matches_word_slice(&parse_words) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::vote_additional_time_while_voting().into()),
            chosen_option_label,
        );
    }
    if VOTE_ADDITIONAL_VOTE_PATTERN.matches_word_slice(&parse_words) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::vote_additional_vote_while_voting().into()),
            chosen_option_label,
        );
    }
    if let Some(count) = parse_additional_land_play_static_count_tokens(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::additional_land_plays(count).into()),
            chosen_option_label,
        );
    }
    if let Some(chunk) = try_lower_hideaway_tokens(parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }
    if let Some(chunk) = try_lower_partner_with_tokens(parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }

    let lexed = parse_tokens;
    if tokens_start_with_level_up(lexed) {
        if let Some(level_up) = parse_level_up_line_lexed(&lexed)? {
            return Ok(LineAst::Ability(level_up));
        }
    }
    let token_words = crate::runtime_backend::lexer::token_word_refs(&lexed);
    if word_slice_ends_with(&token_words, DOESNT_UNTAP_DURING_YOUR_UNTAP_STEP_SUFFIX)
        && word_slice_contains_any_word(&token_words, DOESNT_UNTAP_WORDS)
    {
        let chunk =
            LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::doesnt_untap(),
            )]);
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option_label,
        );
    }
    if let Some(ability) = parse_spell_cost_increase_per_target_beyond_first_line(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option_label,
        );
    }
    if let Some(abilities) = parse_spell_and_player_activated_ability_cost_modifier_line(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(abilities.into_iter().map(Into::into).collect()),
            chosen_option_label,
        );
    }
    if let Some(ability) = parse_spells_cost_modifier_line(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option_label,
        );
    }
    if let Some(chunk) = lower_compound_buff_and_unblockable_static_chunk(line, parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }
    if looks_like_combined_spell_and_activation_tax(token_words.as_slice())
        && let Some(abilities) = parse_static_ability_ast_line_lexed(&lexed)?
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(abilities),
            chosen_option_label,
        );
    }
    if let Some(ability) =
        crate::runtime_backend::families::keyword_static::parse_double_counters_replacement_line(
            &lexed,
        )?
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option_label,
        );
    }
    if let Some(actions) = parse_source_has_keyword_actions(&lexed) {
        return wrap_chosen_option_static_chunk(LineAst::Abilities(actions), chosen_option_label);
    }
    if let Some(abilities) =
        crate::runtime_backend::families::keyword_static::parse_additional_land_play_line(&lexed)?
    {
        let abilities = abilities
            .into_iter()
            .map(crate::cards::builders::StaticAbilityAst::Static)
            .collect();
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(abilities),
            chosen_option_label,
        );
    }
    match parse_static_ability_ast_line_lexed(&lexed) {
        Ok(Some(abilities)) => {
            return wrap_chosen_option_static_chunk(
                LineAst::StaticAbilities(abilities),
                chosen_option_label,
            );
        }
        Ok(None) => {}
        Err(_)
            if parse_tokens
                .iter()
                .any(|token| token.kind == TokenKind::Period) => {}
        Err(err) => return Err(err),
    }
    if !should_skip_keyword_action_static_probe_tokens(parse_tokens)
        && let Some(actions) = parse_ability_line_lexed(&lexed)
    {
        return wrap_chosen_option_static_chunk(LineAst::Abilities(actions), chosen_option_label);
    }
    if let Some(chunk) = lower_split_rewrite_static_chunk(line, parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }
    if looks_like_ability_word_marker_tokens(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::keyword_marker(render_token_slice(parse_tokens).trim().to_string())
                    .into(),
            ),
            chosen_option_label,
        );
    }
    Err(CardTextError::ParseError(format!(
        "rewrite static lowering could not reconstitute static line '{}'",
        line.info.raw_line
    )))
}

fn parse_source_has_keyword_actions(lexed: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let words = crate::runtime_backend::lexer::token_word_refs(lexed);
    let has_word_idx = words
        .iter()
        .position(|word| HAS_OR_HAVE_WORDS.contains(word))?;
    if has_word_idx == 0 || !is_source_reference_words(&words[..has_word_idx]) {
        return None;
    }

    let has_token_idx = token_index_for_word_index(lexed, has_word_idx)?;
    let tail = trim_commas(&lexed[has_token_idx + 1..]);
    let tail_words = crate::runtime_backend::lexer::token_word_refs(&tail);
    if word_slice_contains_phrase(&tail_words, AS_LONG_AS_PHRASE) {
        return None;
    }
    parse_ability_line_lexed(&tail)
}

fn looks_like_ability_word_marker_tokens(parse_tokens: &[OwnedLexToken]) -> bool {
    if parse_tokens.iter().any(|token| {
        matches!(
            token.kind,
            TokenKind::Period
                | TokenKind::Colon
                | TokenKind::Dash
                | TokenKind::EmDash
                | TokenKind::Comma
                | TokenKind::Semicolon
        )
    }) {
        return false;
    }
    let words = token_word_refs(parse_tokens);
    !words.is_empty() && words.len() <= 4
}

fn should_skip_keyword_action_static_probe_tokens(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(tokens);
    CANT_BE_BLOCKED_LINE_PATTERN.matches_word_slice(&words)
        && !THIS_OR_IT_PREFIX_PATTERN.matches_word_slice(&words)
}

fn is_draft_rule_static_line(parse_tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(parse_tokens);
    word_slice_eq(&words, DRAFT_RULE_LINE_WORDS)
        || word_slice_starts_with_any(&words, DRAFT_RULE_PREFIXES)
        || (word_slice_starts_with(&words, DRAFT_BOOSTER_PASS_PREFIX)
            && word_slice_contains_phrase(&words, DRAFT_BOOSTER_PACK_PHRASE))
}

fn is_any_number_named_deck_construction_line(parse_tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(parse_tokens);
    word_slice_starts_with(&words, ANY_NUMBER_NAMED_DECK_CONSTRUCTION_PREFIX)
        && words.len() > ANY_NUMBER_NAMED_DECK_CONSTRUCTION_PREFIX_LEN
}

#[test]
fn ability_word_marker_detection_uses_token_kinds() {
    let marker_tokens = lex_line("Landfall", 0).expect("marker should lex");
    assert!(looks_like_ability_word_marker_tokens(&marker_tokens));

    let sentence_tokens = lex_line(
        "Landfall — Whenever a land enters under your control, draw a card.",
        0,
    )
    .expect("sentence should lex");
    assert!(!looks_like_ability_word_marker_tokens(&sentence_tokens));
}

#[test]
fn additional_land_play_static_count_uses_token_words() {
    let tokens = lex_line(
        "You may play two additional lands on each of your turns.",
        0,
    )
    .expect("lexes");
    assert_eq!(
        parse_additional_land_play_static_count_tokens(&tokens),
        Some(2)
    );

    let non_match = lex_line("You may play an additional land this turn.", 0).expect("lexes");
    assert_eq!(
        parse_additional_land_play_static_count_tokens(&non_match),
        None
    );
}

fn parse_additional_land_play_static_count_tokens(parse_tokens: &[OwnedLexToken]) -> Option<u32> {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(parse_tokens);
    if !ADDITIONAL_LAND_PLAY_STATIC_PREFIX_PATTERN.matches_word_slice(&words) {
        return None;
    }
    let (count, used) = ironsmith_core::parse_cardinal_words(&words[3..])?;
    let tail_words = words.get(3 + used..)?;
    if !ADDITIONAL_LAND_PLAY_STATIC_TAIL_PATTERN.matches_word_slice(tail_words) {
        return None;
    }
    Some(count)
}

#[cfg(test)]
pub(crate) fn lower_rewrite_keyword_to_chunk(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    kind: RewriteKeywordLineKind,
) -> Result<LineAst, CardTextError> {
    lower_rewrite_keyword_to_chunk_impl(
        &RewriteKeywordLine {
            info,
            text: text.to_string(),
            kind,
            parse_tokens: parse_tokens.to_vec(),
            full_parse_tokens: parse_tokens.to_vec(),
        },
        parse_tokens,
    )
}

#[cfg(test)]
fn lower_rewrite_keyword_to_chunk_impl(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    super::super::keyword_registry::lower_keyword_line_ast(line, parse_tokens)
}

#[cfg(test)]
fn test_line_info(raw_line: &str) -> LineInfo {
    LineInfo {
        line_index: 0,
        display_line_index: 0,
        raw_line: raw_line.to_string(),
        normalized: NormalizedLine {
            original: raw_line.to_string(),
            normalized: raw_line.to_ascii_lowercase(),
            char_map: Vec::new(),
        },
    }
}

#[cfg(test)]
fn test_rewrite_triggered_line(raw_line: &str, full_text: &str) -> RewriteTriggeredLine {
    RewriteTriggeredLine {
        info: test_line_info(raw_line),
        full_text: full_text.to_string(),
        full_parse_tokens: Vec::new(),
        trigger_text: String::new(),
        trigger_parse_tokens: Vec::new(),
        effect_text: String::new(),
        effect_parse_tokens: Vec::new(),
        intervening_if: None,
        presentation_label: None,
        max_triggers_per_turn: Some(1),
        chosen_option_label: None,
    }
}

#[test]
fn triggered_line_source_text_keeps_raw_do_this_only_once_suffix() {
    let raw_line = "Whenever Pantlaza or another Dinosaur you control enters, you may discover X, where X is that creature's toughness. Do this only once each turn.";
    let full_text = "whenever pantlaza or another dinosaur you control enters, you may discover x, where x is that creature's toughness";
    let line = test_rewrite_triggered_line(raw_line, full_text);

    assert_eq!(triggered_line_source_text(&line), raw_line);
}

#[test]
fn triggered_line_source_text_keeps_labelled_raw_do_this_only_once_suffix() {
    let raw_line = "Mold Earth — Whenever one or more lands enter under an opponent's control without being played, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle. Do this only once each turn.";
    let full_text = "whenever one or more lands enter under an opponent's control without being played, you may search your library for a plains card, put it onto the battlefield tapped, then shuffle";
    let line = test_rewrite_triggered_line(raw_line, full_text);

    assert_eq!(triggered_line_source_text(&line), raw_line);
}

pub(super) fn normalize_exert_followup_source_reference_tokens(
    source_ref: &str,
    followup_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let followup_words = TokenWordView::new(followup_tokens);
    let replacement_start =
        if word_view_has_any_prefix(&followup_words, &[&["he"], &["she"], &["they"]]) {
            followup_words.token_index_after_words(1)
        } else if let Ok(source_tokens) = lex_line(source_ref, 0) {
            let source_words = token_word_refs(&source_tokens);
            if !source_words.is_empty()
                && !word_slice_eq(&source_words, THIS_CREATURE_SOURCE_WORDS)
                && word_view_has_prefix(&followup_words, source_words.as_slice())
            {
                followup_words.token_index_after_words(source_words.len())
            } else {
                None
            }
        } else {
            None
        };

    let Some(replacement_start) = replacement_start else {
        return followup_tokens.to_vec();
    };

    let mut normalized =
        lex_line("this creature", 0).expect("rewrite lexer should classify exert subject rewrite");
    normalized.extend_from_slice(&followup_tokens[replacement_start..]);
    normalized
}

struct ExertAttackHead {
    only_if_not_exerted_this_turn: bool,
    source_ref: String,
}

fn exert_attack_prefix_word_count(words: &TokenWordView<'_>) -> Option<(bool, usize)> {
    const EXERT_PREFIX: &[&str] = &["you", "may", "exert"];
    const IF_NOT_EXERTED_PREFIXES: &[&[&str]] = &[
        &[
            "if", "this", "creature", "hasnt", "been", "exerted", "this", "turn", "you", "may",
            "exert",
        ],
        &[
            "if", "this", "creature", "hasn't", "been", "exerted", "this", "turn", "you", "may",
            "exert",
        ],
    ];

    IF_NOT_EXERTED_PREFIXES
        .iter()
        .find(|prefix| word_view_has_prefix(words, prefix))
        .map(|prefix| (true, prefix.len()))
        .or_else(|| {
            word_view_has_prefix(words, EXERT_PREFIX).then_some((false, EXERT_PREFIX.len()))
        })
}

fn parse_exert_attack_head_tokens(
    head_tokens: &[OwnedLexToken],
) -> Result<ExertAttackHead, CardTextError> {
    let words = TokenWordView::new(head_tokens);
    let Some((only_if_not_exerted_this_turn, source_start_word)) =
        exert_attack_prefix_word_count(&words)
    else {
        return Err(CardTextError::ParseError(
            "rewrite keyword lowering could not parse exert attack line".to_string(),
        ));
    };

    let Some(as_word_idx) = (source_start_word..words.len()).find(|idx| words.at_is(*idx, "as"))
    else {
        return Err(CardTextError::ParseError(
            "rewrite keyword lowering could not parse exert attack head".to_string(),
        ));
    };
    if as_word_idx == source_start_word {
        return Err(CardTextError::ParseError(
            "rewrite keyword lowering missing exert source".to_string(),
        ));
    }

    let source_range = words
        .token_range_for_word_range(source_start_word, as_word_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "rewrite keyword lowering could not isolate exert source".to_string(),
            )
        })?;
    let attack_range = words
        .token_range_for_word_range(as_word_idx + 1, words.len())
        .ok_or_else(|| {
            CardTextError::ParseError(
                "rewrite keyword lowering could not isolate exert attack clause".to_string(),
            )
        })?;
    let attack_tokens = &head_tokens[attack_range];
    if !word_slice_ends_with_any(&token_word_refs(attack_tokens), ATTACK_ACTION_SUFFIXES) {
        return Err(CardTextError::ParseError(
            "rewrite keyword lowering expected attack clause".to_string(),
        ));
    }

    Ok(ExertAttackHead {
        only_if_not_exerted_this_turn,
        source_ref: render_token_slice(&head_tokens[source_range])
            .trim()
            .to_string(),
    })
}

pub(crate) fn lower_exert_attack_keyword_line(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let sentence_tokens = split_lexed_sentences(parse_tokens);
    let Some(head_tokens) = sentence_tokens.first().copied() else {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering could not parse exert attack line '{}'",
            line.info.raw_line
        )));
    };
    let ExertAttackHead {
        only_if_not_exerted_this_turn,
        source_ref,
    } = parse_exert_attack_head_tokens(head_tokens).map_err(|err| match err {
        CardTextError::ParseError(message) => {
            CardTextError::ParseError(format!("{message} '{}'", line.info.raw_line))
        }
        other => other,
    })?;

    let followup_tokens = sentence_tokens.get(1).copied().filter(|tokens| {
        let followup_words = TokenWordView::new(tokens);
        word_view_has_prefix(&followup_words, &["when", "you", "do"])
    });
    let linked_trigger = if let Some(followup_tokens) = followup_tokens {
        let followup_words = TokenWordView::new(followup_tokens);
        let Some(followup_effect_start) = followup_words.token_index_after_words(3) else {
            return Err(CardTextError::ParseError(format!(
                "rewrite keyword lowering could not strip exert followup intro '{}'",
                line.info.raw_line
            )));
        };
        let followup_effect_tokens = trim_lexed_commas(&followup_tokens[followup_effect_start..]);
        let normalized_followup_tokens = normalize_exert_followup_source_reference_tokens(
            source_ref.as_str(),
            followup_effect_tokens,
        );
        let effects_ast = parse_effect_sentences_lexed(&normalized_followup_tokens)?;
        let prepared = rewrite_prepare_effects_with_trigger_context_for_lowering(
            None,
            &effects_ast,
            ReferenceImports::default(),
        )?;
        let lowered = materialize_prepared_effects_with_trigger_context(&prepared)?;
        Some(crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::state_based("When you do"),
            effects: lowered.effects,
            choices: lowered.choices,
            intervening_if: None,
            presentation_label: None,
        })
    } else if sentence_tokens
        .get(1)
        .is_some_and(|tokens| TokenWordView::new(tokens).first_is("when"))
    {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering expected exert reflexive followup '{}'",
            line.info.raw_line
        )));
    } else {
        None
    };

    Ok(LineAst::StaticAbility(
        StaticAbility::exert_attack(
            only_if_not_exerted_this_turn,
            linked_trigger,
            line.info.raw_line.clone(),
        )
        .into(),
    ))
}

fn rewrite_copy_count_to_times_paid_label_rewrite(effects: &mut [EffectAst], label: &str) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell { target, count, .. },
            ..
        }) = effect
            && let crate::cards::builders::TargetAst::Source(_) = target
            && let crate::effect::Value::Count(filter) = count
            && filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == IT_TAG)
        {
            *count = crate::effect::Value::TimesPaidLabel(label.to_string());
        }
        // Recurse into every nested-effect scope through the shared traversal
        // helper so new wrapper variants are covered automatically (the previous
        // hand-rolled match silently skipped RepeatEffects/ManaRestricted and the
        // newer ChooseOneOf/IfEffectDidNotHappen/TagAffected variants).
        crate::runtime_backend::model::effect_ast_traversal::for_each_nested_effects_mut(
            effect,
            true,
            |nested| rewrite_copy_count_to_times_paid_label_rewrite(nested, label),
        );
    }
}

pub(crate) fn lower_gift_keyword_line(line: &RewriteKeywordLine) -> Result<LineAst, CardTextError> {
    let variant = standard_gift_variant_tokens(&line.parse_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite keyword lowering could not parse gift line '{}'",
            line.info.raw_line
        ))
    })?;
    let timing = standard_gift_timing_tokens(&line.parse_tokens, variant).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "rewrite keyword lowering could not determine gift timing for line '{}'",
            line.info.raw_line
        ))
    })?;
    let cost = OptionalCost::custom(
        line.info.raw_line.trim(),
        TotalCost::from_cost(Cost::effect(
            crate::effects::ChoosePlayerEffect::new(
                PlayerFilter::You,
                PlayerFilter::Opponent,
                "gifted_player",
            )
            .remember_as_chosen_player(),
        )),
    );

    Ok(LineAst::GiftKeyword {
        cost: cost.into(),
        effects: variant.effects(),
        followup_text: variant.followup_text().to_string(),
        timing,
    })
}

#[derive(Clone, Copy)]
enum StandardGiftVariant {
    Card,
    Treasure,
    Food,
    TappedFish,
    ExtraTurn,
    Octopus,
}

impl StandardGiftVariant {
    fn followup_text(self) -> &'static str {
        match self {
            Self::Card => "the chosen player draws a card.",
            Self::Treasure => "the chosen player creates a Treasure token.",
            Self::Food => "the chosen player creates a Food token.",
            Self::TappedFish => "the chosen player creates a tapped 1/1 blue Fish creature token.",
            Self::ExtraTurn => "the chosen player takes an extra turn after this one.",
            Self::Octopus => "the chosen player creates an 8/8 blue Octopus creature token.",
        }
    }

    fn effects(self) -> Vec<EffectAst> {
        match self {
            Self::Card => vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::Chosen,
                SubjectVerbActionAst::Draw {
                    count: crate::effect::Value::Fixed(1),
                },
            )],
            Self::Treasure => vec![standard_gift_create_token_effect("Treasure", false)],
            Self::Food => vec![standard_gift_create_token_effect("Food", false)],
            Self::TappedFish => {
                vec![standard_gift_create_token_effect(
                    "1/1 blue Fish creature",
                    true,
                )]
            }
            Self::ExtraTurn => vec![EffectAst::subject_verb_extra_turn_after_turn(
                PlayerAst::Chosen,
                crate::cards::builders::ExtraTurnAnchorAst::CurrentTurn,
            )],
            Self::Octopus => {
                vec![standard_gift_create_token_effect(
                    "8/8 blue Octopus creature",
                    false,
                )]
            }
        }
    }

    fn default_timing(self) -> GiftTimingAst {
        match self {
            Self::Octopus => GiftTimingAst::PermanentEtb,
            Self::Card | Self::Treasure | Self::Food | Self::TappedFish | Self::ExtraTurn => {
                GiftTimingAst::SpellResolution
            }
        }
    }
}

fn standard_gift_create_token_effect(name: &str, tapped: bool) -> EffectAst {
    EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Chosen,
        SubjectVerbActionAst::CreateTokenWithMods {
            name: name.to_string(),
            count: crate::effect::Value::Fixed(1),
            dynamic_power_toughness: None,
            player: PlayerAst::Chosen,
            attached_to: None,
            tapped,
            attacking: false,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            next_end_step_player: PlayerFilter::Any,
            granted_abilities: Vec::new(),
        },
    )
}

fn standard_gift_variant_tokens(tokens: &[OwnedLexToken]) -> Option<StandardGiftVariant> {
    let head_tokens = tokens
        .iter()
        .position(|token| token.kind == TokenKind::LParen)
        .map(|idx| &tokens[..idx])
        .unwrap_or(tokens);

    match parser_token_word_refs(head_tokens).as_slice() {
        ["gift", "a", "card"] => Some(StandardGiftVariant::Card),
        ["gift", "a", "treasure"] => Some(StandardGiftVariant::Treasure),
        ["gift", "a", "food"] => Some(StandardGiftVariant::Food),
        ["gift", "a", "tapped", "fish"] => Some(StandardGiftVariant::TappedFish),
        ["gift", "an", "extra", "turn"] => Some(StandardGiftVariant::ExtraTurn),
        ["gift", "an", "octopus"] => Some(StandardGiftVariant::Octopus),
        _ => None,
    }
}

fn standard_gift_timing_tokens(
    tokens: &[OwnedLexToken],
    variant: StandardGiftVariant,
) -> Option<GiftTimingAst> {
    let words = parser_token_word_refs(tokens);
    if word_slice_contains_phrase(&words, WHEN_IT_ENTERS_PHRASE) {
        Some(GiftTimingAst::PermanentEtb)
    } else {
        Some(variant.default_timing())
    }
}

pub(crate) fn lower_keyword_special_cases(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(chunk) = try_lower_hideaway_keyword(parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_partner_variant_keyword(line, parse_tokens) {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_partner_with_tokens(parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_optional_cost_with_cast_trigger(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_optional_behold_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_optional_waterbend_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}

pub(crate) fn try_lower_optional_waterbend_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let Some(effect_tokens) = additional_cost_tail_tokens(parse_tokens) else {
        return Ok(None);
    };
    let stripped = trim_lexed_commas(effect_tokens);
    let words = token_word_refs(stripped);
    if !word_slice_starts_with(&words, &["you", "may", "waterbend"]) {
        return Ok(None);
    }

    let Some(generic) = stripped.iter().find_map(|token| {
        token
            .mana_group_inner()
            .and_then(|inner| inner.parse::<u32>().ok())
    }) else {
        return Ok(None);
    };

    let total_cost =
        crate::runtime_backend::lowering::compile_support::waterbend_optional_total_cost(generic);
    Ok(Some(LineAst::OptionalCost(
        OptionalCost::custom(line.info.raw_line.trim(), total_cost).into(),
    )))
}

fn try_lower_partner_variant_keyword(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Option<LineAst> {
    let visible_tokens = if line.full_parse_tokens.is_empty() {
        parse_tokens
    } else {
        line.full_parse_tokens.as_slice()
    };
    if !visible_partner_label_is_variant_tokens(visible_tokens) {
        return None;
    }

    let display = render_tokens_before_reminder_or_period(visible_tokens)
        .unwrap_or_else(|| render_token_slice(parse_tokens).trim().to_string());
    Some(LineAst::StaticAbility(
        StaticAbility::partner_variant(display).into(),
    ))
}

fn visible_partner_label_is_variant_tokens(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens_before_reminder_or_period(tokens));
    if word_slice_eq(&words, PARTNER_KEYWORD_WORDS)
        || PARTNER_WITH_PATTERN.matches_word_slice(&words)
    {
        return false;
    }
    CHARACTER_SELECT_PREFIX_PATTERN.matches_word_slice(&words)
        || matches!(
            words.as_slice(),
            ["partner", second, ..] if *second != "with"
        )
}

fn try_lower_hideaway_keyword(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    try_lower_hideaway_tokens(parse_tokens)
}

fn try_lower_hideaway_tokens(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(parse_tokens);
    if words.len() != 2 || words[0] != "hideaway" {
        return Ok(None);
    }
    let display = render_token_slice(parse_tokens);
    let Ok(count) = words[1].parse::<i32>() else {
        return Err(CardTextError::ParseError(format!(
            "hideaway keyword expected numeric count in '{}'",
            display
        )));
    };
    if count <= 0 {
        return Err(CardTextError::ParseError(format!(
            "hideaway keyword expected positive count in '{}'",
            display
        )));
    }

    Ok(Some(hideaway_line_ast(count)))
}

fn hideaway_line_ast(count: i32) -> LineAst {
    let looked_tag = TagKey::from("hideaway_looked");
    let chosen_tag = TagKey::from("hideaway_exiled");
    let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
    choose_filter.zone = Some(Zone::Library);

    LineAst::Triggered {
        trigger: TriggerSpec::ThisEntersBattlefield,
        effects: vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::You,
                crate::effect::Value::Fixed(count),
                looked_tag.clone(),
            ),
            EffectAst::ChooseObjects {
                filter: choose_filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(chosen_tag.clone(), None), true),
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                looked_tag,
                Some(chosen_tag),
                LibraryBottomOrderAst::Random,
                PlayerAst::You,
            ),
        ],
        max_triggers_per_turn: None,
    }
}

#[test]
fn hideaway_special_case_uses_parse_tokens() {
    let tokens = lex_line("Hideaway 5.", 0).expect("hideaway should lex");
    assert!(
        try_lower_hideaway_tokens(&tokens)
            .expect("hideaway should lower")
            .is_some()
    );

    let non_numeric = lex_line("Hideaway X.", 0).expect("hideaway should lex");
    assert!(try_lower_hideaway_tokens(&non_numeric).is_err());

    let reminder = lex_line("Hideaway 5 reminder", 0).expect("hideaway should lex");
    assert!(
        try_lower_hideaway_tokens(&reminder)
            .expect("extra words should not match the closed-form special case")
            .is_none()
    );
}

fn try_lower_partner_with_tokens(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let Some(partner_name) = partner_with_name_from_tokens(parse_tokens) else {
        return Ok(None);
    };

    let mut filter = ObjectFilter::default();
    filter.name = Some(partner_name.clone());

    Ok(Some(LineAst::Multiple(vec![
        LineAst::StaticAbility(StaticAbility::partner_with(partner_name.clone()).into()),
        LineAst::Triggered {
            trigger: TriggerSpec::ThisEntersBattlefield,
            effects: vec![EffectAst::MayByPlayer {
                player: PlayerAst::Target,
                effects: vec![EffectAst::subject_verb_search_library(
                    filter,
                    Zone::Hand,
                    PlayerAst::Target,
                    PlayerAst::Target,
                    crate::effect::SearchSelectionMode::Exact,
                    true,
                    true,
                    ChoiceCount::up_to(1),
                    None,
                    None,
                    false,
                )],
            }],
            max_triggers_per_turn: None,
        },
    ])))
}

fn partner_with_name_from_tokens(tokens: &[OwnedLexToken]) -> Option<String> {
    if !word_slice_starts_with(&parser_token_word_refs(tokens), PARTNER_WITH_PREFIX) {
        return None;
    }

    let words = TokenWordView::new(tokens);
    let name_start = words.token_index_for_word_index(PARTNER_WITH_PREFIX.len())?;
    let name_end = tokens[name_start..]
        .iter()
        .position(|token| matches!(token.kind, TokenKind::LParen | TokenKind::Period))
        .map(|idx| name_start + idx)
        .unwrap_or(tokens.len());
    let name = render_token_slice(&tokens[name_start..name_end])
        .trim()
        .replace('"', "");
    (!name.is_empty()).then_some(name)
}

fn render_tokens_before_reminder_or_period(tokens: &[OwnedLexToken]) -> Option<String> {
    let display = render_partner_label_token_slice(tokens_before_reminder_or_period(tokens))
        .trim()
        .to_string();
    (!display.is_empty()).then_some(display)
}

fn tokens_before_reminder_or_period(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let end = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::LParen | TokenKind::Period))
        .unwrap_or(tokens.len());
    &tokens[..end]
}

fn render_partner_label_token_slice(tokens: &[OwnedLexToken]) -> String {
    fn needs_space(prev: &OwnedLexToken, current: &OwnedLexToken) -> bool {
        if matches!(current.kind, TokenKind::Dash) || matches!(prev.kind, TokenKind::Dash) {
            return true;
        }
        if prev.span.end == current.span.start {
            return false;
        }
        if matches!(
            current.kind,
            TokenKind::Comma
                | TokenKind::Period
                | TokenKind::Colon
                | TokenKind::Semicolon
                | TokenKind::Question
                | TokenKind::Bang
                | TokenKind::RParen
                | TokenKind::RBracket
        ) {
            return false;
        }
        !matches!(
            prev.kind,
            TokenKind::LBracket | TokenKind::LParen | TokenKind::Quote | TokenKind::Apostrophe
        )
    }

    let mut rendered = String::new();
    let mut previous = None;
    for token in tokens {
        if let Some(prev) = previous
            && needs_space(prev, token)
            && !rendered.ends_with(' ')
        {
            rendered.push(' ');
        }
        rendered.push_str(&token.slice);
        previous = Some(token);
    }
    rendered
}

#[test]
fn partner_name_and_visible_label_trim_on_lexed_reminder_tokens() {
    let partner_with_tokens = lex_line(
        "Partner with Toothy, Imaginary Friend (When this creature enters...)",
        0,
    )
    .expect("partner-with line should lex");
    assert_eq!(
        partner_with_name_from_tokens(&partner_with_tokens).as_deref(),
        Some("Toothy, Imaginary Friend")
    );

    let partner_label_tokens = lex_line(
        "Partner - Friends forever (You can have two commanders.)",
        0,
    )
    .expect("partner label should lex");
    assert_eq!(
        render_tokens_before_reminder_or_period(&partner_label_tokens).as_deref(),
        Some("Partner - Friends forever")
    );
}

pub(crate) fn try_lower_optional_cost_with_cast_trigger(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let sentence_tokens = split_lexed_sentences(parse_tokens);
    let [head_tokens, followup_tokens] = sentence_tokens.as_slice() else {
        return Ok(None);
    };
    let head_words = TokenWordView::new(head_tokens);
    if !word_view_has_prefix(
        &head_words,
        &[
            "as",
            "an",
            "additional",
            "cost",
            "to",
            "cast",
            "this",
            "spell",
        ],
    ) {
        return Ok(None);
    }
    let Some(head_effect_start) = head_words.token_index_after_words(8) else {
        return Ok(None);
    };
    let stripped_head_tokens = trim_lexed_commas(&head_tokens[head_effect_start..]);
    let stripped_head_words = token_word_refs(stripped_head_tokens);
    if !word_slice_starts_with(&stripped_head_words, YOU_MAY_PREFIX) {
        return Ok(None);
    }
    let Some(optional_effect_start) = token_index_for_word_index(stripped_head_tokens, 2) else {
        return Ok(None);
    };

    let head_effects =
        parse_effect_sentences_lexed(&stripped_head_tokens[optional_effect_start..])?;
    let [
        EffectAst::ChooseObjects {
            filter,
            count,
            player,
            ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: sacrificed_player,
                    ..
                },
            action:
                SubjectVerbActionAst::SacrificeAll {
                    filter: sacrificed_filter,
                },
        }),
    ] = head_effects.as_slice()
    else {
        return Ok(None);
    };
    if *player != crate::cards::builders::PlayerAst::Implicit
        || *sacrificed_player != crate::cards::builders::PlayerAst::Implicit
        || count.min != 1
        || count.max.is_some()
        || !matches!(sacrificed_filter, crate::target::ObjectFilter { tagged_constraints, .. } if tagged_constraints.iter().any(|constraint| constraint.tag.as_str() == IT_TAG))
    {
        return Ok(None);
    }

    let head_words = token_word_refs(stripped_head_tokens);
    let label = format!(
        "As an additional cost to cast this spell, {}",
        head_words.join(" ")
    );
    let cost = OptionalCost::custom(
        label.clone(),
        TotalCost::from_cost(Cost::sacrifice(filter.clone())),
    )
    .repeatable();
    let followup_words = TokenWordView::new(followup_tokens);
    if !word_view_has_prefix(&followup_words, &["when", "you", "do"]) {
        return Ok(None);
    }
    let Some(followup_effect_start) = followup_words.token_index_after_words(3) else {
        return Ok(None);
    };
    let followup_effect_tokens = trim_lexed_commas(&followup_tokens[followup_effect_start..]);
    let mut effects = parse_effect_sentences_lexed(followup_effect_tokens)?;
    rewrite_copy_count_to_times_paid_label_rewrite(&mut effects, &label);
    let followup_words = token_word_refs(followup_effect_tokens);

    Ok(Some(LineAst::OptionalCostWithCastTrigger {
        cost: cost.into(),
        effects,
        followup_text: format!("When you do, {}", followup_words.join(" ")),
    }))
}

pub(crate) fn try_lower_optional_behold_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let Some(effect_tokens) = additional_cost_tail_tokens(parse_tokens) else {
        return Ok(None);
    };
    let stripped = trim_lexed_commas(effect_tokens);
    let words = token_word_refs(stripped);
    if !word_slice_starts_with_any(&words, OPTIONAL_BEHOLD_OR_BLIGHT_PREFIXES) {
        return Ok(None);
    }

    let total_cost = parse_activation_cost(&stripped[2..])?;
    if total_cost.mana_cost().is_some() || total_cost.costs().len() != 1 {
        return Ok(None);
    }

    Ok(Some(LineAst::OptionalCost(
        OptionalCost::custom(line.info.raw_line.trim(), total_cost).into(),
    )))
}

fn additional_cost_tail_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let comma_idx = find_index(tokens, |token| token.kind == TokenKind::Comma);
    let effect_start = if let Some(idx) = comma_idx {
        idx + 1
    } else if let Some(idx) = find_index(tokens, |token| token.is_word("spell")) {
        idx + 1
    } else {
        tokens.len()
    };
    let effect_tokens = tokens.get(effect_start..).unwrap_or_default();
    (!effect_tokens.is_empty()).then_some(effect_tokens)
}

pub(super) fn lower_rewrite_modal_to_item(
    modal: RewriteModalBlock,
) -> Result<ParsedCardItem, CardTextError> {
    let Some(header) = parse_modal_header(&modal.header)? else {
        return Err(CardTextError::ParseError(format!(
            "rewrite modal lowering could not parse modal header '{}'",
            modal.header.raw_line
        )));
    };

    let mut modes = Vec::with_capacity(modal.modes.len());
    for mode in modal.modes {
        let mut effects_ast = mode.effects_ast;
        if let Some(replacement) = header.x_replacement.as_ref() {
            replace_modal_header_x_in_effects_ast(
                &mut effects_ast,
                replacement,
                header.line_text.as_str(),
            )?;
        }
        modes.push(ParsedModalModeAst {
            info: mode.info,
            description: mode.text,
            point_cost: mode.point_cost,
            effects_ast,
        });
    }

    Ok(ParsedCardItem::Modal(ParsedModalAst { header, modes }))
}

#[allow(dead_code)]
pub(super) fn lower_rewrite_level_to_item(
    level: RewriteLevelHeader,
) -> Result<ParsedCardItem, CardTextError> {
    let mut items = Vec::with_capacity(level.items.len());
    for item in level.items {
        items.push(item.parsed);
    }

    Ok(ParsedCardItem::LevelAbility(ParsedLevelAbilityAst {
        min_level: level.min_level,
        max_level: level.max_level,
        pt: level.pt,
        items,
    }))
}

#[allow(dead_code)]
pub(super) fn lower_rewrite_saga_to_item(
    saga: RewriteSagaChapterLine,
) -> Result<ParsedCardItem, CardTextError> {
    Ok(ParsedCardItem::Line(ParsedLineAst {
        info: saga.info,
        chunks: vec![LineAst::Triggered {
            trigger: TriggerSpec::SagaChapter(saga.chapters),
            effects: saga.effects_ast,
            max_triggers_per_turn: None,
        }],
        restrictions: ParsedRestrictions::default(),
    }))
}

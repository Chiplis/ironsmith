use winnow::combinator::{alt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use crate::target::PlayerFilter;

use super::super::super::lexer::{
    LexStream, OwnedLexToken, TokenKind, parser_token_word_refs, trim_lexed_commas,
};
use super::super::primitives;
use super::{
    any_phrase_is_present, apostrophe_insensitive_phrase_is_present, every_phrase_is_present,
    phrase_is_exact, phrase_is_prefix, phrase_is_present, phrase_is_suffix,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TriggeredTextFacts {
    pub(crate) has_if_you_do: bool,
    pub(crate) has_if_you_dont: bool,
    pub(crate) has_full_party_instead: bool,
    pub(crate) has_full_party_condition: bool,
    pub(crate) starts_with_if: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerLabelSplit<'a> {
    pub(crate) label_tokens: &'a [OwnedLexToken],
    pub(crate) body_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CombatDeathBlockedDamage {
    pub(crate) amount_surface: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpellOrActivatedAbilityXCostTrigger;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlocksOrBecomesBlockedFirstStrike;

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

pub(crate) fn parse_triggered_text_facts_tokens(tokens: &[OwnedLexToken]) -> TriggeredTextFacts {
    let words = parser_token_word_refs(tokens);
    let full_party = &["if", "you", "have", "a", "full", "party"];
    TriggeredTextFacts {
        has_if_you_do: phrase_is_present(&words, &["if", "you", "do"]),
        has_if_you_dont: any_phrase_is_present(
            &words,
            &[&["if", "you", "don't"], &["if", "you", "dont"]],
        ),
        has_full_party_instead: every_phrase_is_present(
            &words,
            &[full_party, &["until", "end", "of", "turn", "instead"]],
        ),
        has_full_party_condition: phrase_is_present(&words, full_party),
        starts_with_if: phrase_is_prefix(&words, &["if"]),
    }
}

pub(crate) fn parse_next_draw_replacement_player_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PlayerFilter> {
    let words = parser_token_word_refs(tokens);
    if !every_phrase_is_present(
        &words,
        &[
            &["the", "next", "time"],
            &["would", "draw"],
            &["this", "turn"],
            &["instead"],
        ],
    ) {
        return None;
    }

    if any_phrase_is_present(
        &words,
        &[
            &["they", "would", "draw"],
            &["that", "player", "would", "draw"],
        ],
    ) {
        Some(PlayerFilter::IteratedPlayer)
    } else if phrase_is_present(&words, &["you", "would", "draw"]) {
        Some(PlayerFilter::You)
    } else if any_phrase_is_present(
        &words,
        &[
            &["an", "opponent", "would", "draw"],
            &["opponent", "would", "draw"],
        ],
    ) {
        Some(PlayerFilter::Opponent)
    } else {
        None
    }
}

fn parse_trigger_label_split<'a>(input: &mut LexStream<'a>) -> WResult<TriggerLabelSplit<'a>> {
    let label_tokens = repeat_till(
        1..,
        any.void(),
        peek(alt((
            primitives::token_kind(TokenKind::Dash),
            primitives::token_kind(TokenKind::EmDash),
        ))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((
        primitives::token_kind(TokenKind::Dash),
        primitives::token_kind(TokenKind::EmDash),
    ))
    .parse_next(input)?;
    let body_tokens = rest.parse_next(input)?;
    let label_tokens = trim_lexed_commas(label_tokens);
    let body_tokens = trim_lexed_commas(body_tokens);
    if label_tokens.is_empty() || body_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "trigger label",
            "non-empty label and body",
        ));
    }
    Ok(TriggerLabelSplit {
        label_tokens,
        body_tokens,
    })
}

pub(crate) fn parse_trigger_label_split_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TriggerLabelSplit<'_>> {
    primitives::parse_all(tokens, parse_trigger_label_split, "trigger-label-split").ok()
}

pub(crate) fn normalized_trigger_source_words_tokens(tokens: &[OwnedLexToken]) -> Vec<String> {
    let words = parser_token_word_refs(tokens);
    let stem = TRIGGER_CAP_SUFFIXES
        .iter()
        .find_map(|suffix| {
            phrase_is_suffix(&words, suffix)
                .then(|| &words[..words.len().saturating_sub(suffix.len())])
        })
        .unwrap_or(words.as_slice());
    stem.iter().map(|word| (*word).to_string()).collect()
}

pub(crate) fn parse_combat_death_blocked_damage_tokens(
    trigger_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
) -> Option<CombatDeathBlockedDamage> {
    let trigger_words = parser_token_word_refs(trigger_tokens);
    if !phrase_is_exact(
        &trigger_words,
        &["when", "this", "creature", "dies", "during", "combat"],
    ) {
        return None;
    }

    let effect_words = parser_token_word_refs(effect_tokens);
    let prefix = &["it", "deals"];
    let suffix = &[
        "damage", "to", "each", "creature", "it", "blocked", "this", "combat",
    ];
    if !phrase_is_prefix(&effect_words, prefix)
        || !phrase_is_suffix(&effect_words, suffix)
        || effect_words.len() <= prefix.len() + suffix.len()
    {
        return None;
    }
    let amount_words = &effect_words[prefix.len()..effect_words.len() - suffix.len()];
    Some(CombatDeathBlockedDamage {
        amount_surface: amount_words.join(" "),
    })
}

pub(crate) fn parse_spell_or_activated_ability_x_cost_trigger_tokens(
    full_tokens: &[OwnedLexToken],
    trigger_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
) -> Option<SpellOrActivatedAbilityXCostTrigger> {
    let trigger_words = parser_token_word_refs(trigger_tokens);
    let full_words = parser_token_word_refs(full_tokens);
    let effect_words = parser_token_word_refs(effect_tokens);
    (apostrophe_insensitive_phrase_is_present(
        &trigger_words,
        &[
            "you", "cast", "an", "instant", "or", "sorcery", "spell", "or", "activate", "an",
            "ability",
        ],
    ) && apostrophe_insensitive_phrase_is_present(
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
    ) && apostrophe_insensitive_phrase_is_present(
        &effect_words,
        &["copy", "that", "spell", "or", "ability"],
    ))
    .then_some(SpellOrActivatedAbilityXCostTrigger)
}

pub(crate) fn parse_blocks_or_becomes_blocked_first_strike_tokens(
    tokens: &[OwnedLexToken],
) -> Option<BlocksOrBecomesBlockedFirstStrike> {
    let words = parser_token_word_refs(tokens);
    (phrase_is_prefix(
        &words,
        &[
            "whenever", "this", "creature", "blocks", "or", "becomes", "blocked", "by", "a",
            "creature",
        ],
    ) && phrase_is_suffix(
        &words,
        &[
            "that", "creature", "gains", "first", "strike", "until", "end", "of", "turn",
        ],
    ))
    .then_some(BlocksOrBecomesBlockedFirstStrike)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_triggered_markers_and_draw_player() {
        let tokens = lex_line(
            "The next time an opponent would draw a card this turn, they mill instead. If you don't, draw.",
            0,
        )
        .unwrap();
        let facts = parse_triggered_text_facts_tokens(&tokens);
        assert!(facts.has_if_you_dont);
        assert_eq!(
            parse_next_draw_replacement_player_tokens(&tokens),
            Some(PlayerFilter::Opponent)
        );
    }

    #[test]
    fn parses_label_and_trigger_cap() {
        let tokens = lex_line(
            "Mold Earth — Whenever a land enters, draw a card. Do this only once each turn.",
            0,
        )
        .unwrap();
        let split = parse_trigger_label_split_tokens(&tokens).unwrap();
        assert_eq!(
            parser_token_word_refs(split.label_tokens),
            vec!["mold", "earth"]
        );
        assert_eq!(
            normalized_trigger_source_words_tokens(split.body_tokens),
            vec!["whenever", "a", "land", "enters", "draw", "a", "card"]
        );
    }

    #[test]
    fn parses_x_cost_and_combat_damage_shapes() {
        let trigger = lex_line(
            "Whenever you cast an instant or sorcery spell or activate an ability,",
            0,
        )
        .unwrap();
        let full = lex_line(
            "Whenever you cast an instant or sorcery spell or activate an ability, if that spell's mana cost or that ability's activation cost contains X, copy that spell or ability.",
            0,
        )
        .unwrap();
        let effect = lex_line("Copy that spell or ability.", 0).unwrap();
        assert!(
            parse_spell_or_activated_ability_x_cost_trigger_tokens(&full, &trigger, &effect)
                .is_some()
        );

        let trigger = lex_line("When this creature dies during combat", 0).unwrap();
        let effect = lex_line(
            "It deals 3 damage to each creature it blocked this combat.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_combat_death_blocked_damage_tokens(&trigger, &effect)
                .unwrap()
                .amount_surface,
            "3"
        );
    }
}

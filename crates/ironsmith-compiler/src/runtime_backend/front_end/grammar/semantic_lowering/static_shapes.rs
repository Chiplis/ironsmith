use crate::runtime_backend::front_end::shared::util::is_source_reference_words;

use super::super::super::lexer::{OwnedLexToken, TokenKind, TokenWordView, parser_token_word_refs};
use super::super::keyword_static_lines::{self, AdditionalVoteKind};
use super::super::leaf;
use super::{
    any_word_is_present, every_phrase_is_present, phrase_is_exact, phrase_is_prefix,
    phrase_is_present, phrase_is_suffix, phrase_location,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaticSpecialLineShape {
    BlackManaMayBePaidWithLife,
    BoastTwice,
    EquipAtInstantSpeed,
    AdditionalVoteTime,
    AdditionalVote,
    DoesntUntap,
    DraftRule,
    HiddenAgenda,
    DoubleAgenda,
    AnyNumberNamedDeckConstruction,
    FirstEquipCostAlternative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CombinedSpellAndActivationTax;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceKeywordTail<'a> {
    pub(crate) ability_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkipKeywordActionProbe;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AbilityWordMarker;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LevelUpIntro;

fn is_black_mana_life_payment(words: &[&str]) -> bool {
    phrase_is_exact(
        words,
        &[
            "for", "each", "b", "in", "a", "cost", "you", "may", "pay", "2", "life", "rather",
            "than", "pay", "that", "mana",
        ],
    )
}

fn is_boast_twice(words: &[&str]) -> bool {
    phrase_is_exact(
        words,
        &[
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
        ],
    )
}

fn is_equip_at_instant_speed(words: &[&str]) -> bool {
    phrase_is_exact(
        words,
        &[
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
        ],
    )
}

fn is_doesnt_untap(words: &[&str]) -> bool {
    phrase_is_suffix(words, &["untap", "during", "your", "untap", "step"])
        && any_word_is_present(words, &["doesnt", "doesn't"])
}

fn is_draft_rule(words: &[&str]) -> bool {
    phrase_is_exact(words, &["draft", "this", "card", "face", "up"])
        || [
            &["reveal", "this", "card", "as", "you", "draft", "it"][..],
            &["as", "you", "draft"],
            &["during", "the", "draft"],
            &["immediately", "after", "the", "draft"],
        ]
        .iter()
        .any(|prefix| phrase_is_prefix(words, prefix))
        || (phrase_is_prefix(words, &["each", "player", "passes"])
            && phrase_is_present(words, &["booster", "pack"]))
}

fn is_hidden_agenda(words: &[&str]) -> bool {
    phrase_is_exact(words, &["hidden", "agenda"])
}

fn is_double_agenda(words: &[&str]) -> bool {
    phrase_is_exact(words, &["double", "agenda"])
}

fn is_named_deck_construction(words: &[&str]) -> bool {
    let prefix = &[
        "a", "deck", "can", "have", "any", "number", "of", "cards", "named",
    ];
    phrase_is_prefix(words, prefix) && words.len() > prefix.len()
}

fn is_first_equip_alternative(words: &[&str]) -> bool {
    phrase_is_prefix(words, &["you", "may", "pay"])
        && phrase_is_present(
            words,
            &[
                "rather", "than", "pay", "the", "equip", "cost", "of", "the", "first", "equip",
                "ability", "you", "activate",
            ],
        )
        && (phrase_is_suffix(words, &["each", "turn"])
            || phrase_is_suffix(words, &["during", "each", "of", "your", "turns"]))
}

pub(crate) fn parse_static_special_line_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StaticSpecialLineShape> {
    let words = parser_token_word_refs(tokens);
    if is_black_mana_life_payment(&words) {
        Some(StaticSpecialLineShape::BlackManaMayBePaidWithLife)
    } else if is_boast_twice(&words) {
        Some(StaticSpecialLineShape::BoastTwice)
    } else if is_equip_at_instant_speed(&words) {
        Some(StaticSpecialLineShape::EquipAtInstantSpeed)
    } else if let Some(kind) = keyword_static_lines::parse_additional_vote_tokens(tokens) {
        Some(match kind {
            AdditionalVoteKind::OptionalTime => StaticSpecialLineShape::AdditionalVoteTime,
            AdditionalVoteKind::MandatoryVote => StaticSpecialLineShape::AdditionalVote,
        })
    } else if is_doesnt_untap(&words) {
        Some(StaticSpecialLineShape::DoesntUntap)
    } else if is_hidden_agenda(&words) {
        Some(StaticSpecialLineShape::HiddenAgenda)
    } else if is_double_agenda(&words) {
        Some(StaticSpecialLineShape::DoubleAgenda)
    } else if is_draft_rule(&words) {
        Some(StaticSpecialLineShape::DraftRule)
    } else if is_named_deck_construction(&words) {
        Some(StaticSpecialLineShape::AnyNumberNamedDeckConstruction)
    } else if is_first_equip_alternative(&words) {
        Some(StaticSpecialLineShape::FirstEquipCostAlternative)
    } else {
        None
    }
}

pub(crate) fn parse_combined_spell_and_activation_tax_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CombinedSpellAndActivationTax> {
    let words = parser_token_word_refs(tokens);
    (every_phrase_is_present(
        &words,
        &[
            &["and", "abilities"],
            &["activate", "cost"],
            &["more", "to", "activate"],
        ],
    ) && any_word_is_present(&words, &["spell", "spells"]))
    .then_some(CombinedSpellAndActivationTax)
}

pub(crate) fn parse_source_keyword_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SourceKeywordTail<'_>> {
    let words = parser_token_word_refs(tokens);
    let has_word =
        phrase_location(&words, &["has"]).or_else(|| phrase_location(&words, &["have"]))?;
    if has_word == 0 || !is_source_reference_words(&words[..has_word]) {
        return None;
    }
    let ability_start = has_word + 1;
    if phrase_is_present(&words[ability_start..], &["as", "long", "as"]) {
        return None;
    }
    let view = TokenWordView::new(tokens);
    let range = view.token_span_for_words(ability_start, words.len())?;
    let ability_tokens = super::super::super::lexer::trim_lexed_commas(&tokens[range]);
    (!ability_tokens.is_empty()).then_some(SourceKeywordTail { ability_tokens })
}

pub(crate) fn parse_skip_keyword_action_probe_tokens(
    tokens: &[OwnedLexToken],
) -> Option<SkipKeywordActionProbe> {
    let words = parser_token_word_refs(tokens);
    ((phrase_is_suffix(&words, &["can't", "be", "blocked"])
        || phrase_is_suffix(&words, &["cant", "be", "blocked"]))
        && !(phrase_is_prefix(&words, &["this"]) || phrase_is_prefix(&words, &["it"])))
    .then_some(SkipKeywordActionProbe)
}

pub(crate) fn parse_additional_land_play_count_tokens(tokens: &[OwnedLexToken]) -> Option<u32> {
    let words = parser_token_word_refs(tokens);
    if !phrase_is_prefix(&words, &["you", "may", "play"]) {
        return None;
    }
    let (number, used) = leaf::parse_leaf_number_prefix_words(words.get(3..)?)?.into_fixed()?;
    let tail = words.get(3 + used..)?;
    if phrase_is_exact(
        tail,
        &["additional", "land", "on", "each", "of", "your", "turns"],
    ) || phrase_is_exact(
        tail,
        &["additional", "lands", "on", "each", "of", "your", "turns"],
    ) {
        Some(number)
    } else {
        None
    }
}

pub(crate) fn parse_ability_word_marker_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AbilityWordMarker> {
    if tokens.iter().any(|token| {
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
        return None;
    }
    let word_count = TokenWordView::new(tokens).len();
    (word_count > 0 && word_count <= 4).then_some(AbilityWordMarker)
}

pub(crate) fn parse_level_up_intro_tokens(tokens: &[OwnedLexToken]) -> Option<LevelUpIntro> {
    phrase_is_prefix(&parser_token_word_refs(tokens), &["level", "up"]).then_some(LevelUpIntro)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_special_static_shapes() {
        let equip = lex_line(
            "You may activate equip abilities any time you could cast an instant.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_static_special_line_tokens(&equip),
            Some(StaticSpecialLineShape::EquipAtInstantSpeed)
        );

        let deck = lex_line(
            "A deck can have any number of cards named Relentless Rats.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_static_special_line_tokens(&deck),
            Some(StaticSpecialLineShape::AnyNumberNamedDeckConstruction)
        );
    }

    #[test]
    fn parses_source_tail_and_land_count() {
        let source = lex_line("This creature has flying and vigilance.", 0).unwrap();
        let tail = parse_source_keyword_tail_tokens(&source).unwrap();
        assert_eq!(
            parser_token_word_refs(tail.ability_tokens),
            vec!["flying", "and", "vigilance"]
        );

        let land = lex_line(
            "You may play two additional lands on each of your turns.",
            0,
        )
        .unwrap();
        assert_eq!(parse_additional_land_play_count_tokens(&land), Some(2));
    }
}

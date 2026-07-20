use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use super::super::{
    anthem_grants,
    effects::become_shapes,
    primitives::{self, WordSliceInput},
    structure,
};
use crate::runtime_backend::lexer::{OwnedLexToken, TokenWordView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnsupportedRewriteLineKind {
    FirstSpellCostModifier,
    StaticClause,
    TrailingPreventNextDamage,
    MarkerKeywordWithTail,
    SameNameDiscard,
    MixedEntersTappedUntap,
    PreventCombatDamageTail,
    DefendingPlayerChoice,
    SacrificeIslandThisWay,
    AuraCopyAttachment,
    LandwalkOverride,
    PowerOrToughnessUnblockable,
    DiscardQualifier,
    Predicate,
    EachPlayerExileSacrificeReturn,
    SaddledConditional,
    LookedCardFallback,
    AnthemSubject,
    AdditionalLandPermission,
    TargetOnlyRestriction,
    GenericLine,
    ChooseLeadingSpell,
    TemporaryLosesAbilitiesBecomes,
    StaticLosesAbilitiesBecomes,
    ForAsLongAsPermission,
    MultiStepEachPlayer,
    ArtifactCreaturePlayerTarget,
    CreatureTokenPlayerPlaneswalkerTarget,
    VillainousChoice,
    LegendaryCopyException,
}

impl UnsupportedRewriteLineKind {
    pub(crate) fn diagnostic(self) -> &'static str {
        match self {
            Self::FirstSpellCostModifier => "unsupported first-spell cost modifier mechanic",
            Self::StaticClause => "unsupported static clause",
            Self::TrailingPreventNextDamage => "unsupported trailing prevent-next damage clause",
            Self::MarkerKeywordWithTail => "unsupported marker keyword with non-keyword tail",
            Self::SameNameDiscard => "unsupported same-name-as-another-in-hand discard clause",
            Self::MixedEntersTappedUntap => {
                "unsupported mixed enters-tapped and negated-untap clause"
            }
            Self::PreventCombatDamageTail => "unsupported prevent-all-combat-damage clause tail",
            Self::DefendingPlayerChoice => "unsupported defending-players-choice clause",
            Self::SacrificeIslandThisWay => {
                "unsupported if-you-sacrifice-an-island-this-way clause"
            }
            Self::AuraCopyAttachment => "unsupported aura-copy attachment fanout clause",
            Self::LandwalkOverride => "unsupported landwalk override clause",
            Self::PowerOrToughnessUnblockable => {
                "unsupported power-or-toughness cant-be-blocked subject"
            }
            Self::DiscardQualifier => "unsupported discard qualifier clause",
            Self::Predicate => "unsupported predicate",
            Self::EachPlayerExileSacrificeReturn => {
                "unsupported each-player exile/sacrifice/return-this-way clause"
            }
            Self::SaddledConditional => "unsupported saddled conditional tail",
            Self::LookedCardFallback => "unsupported looked-card fallback tail",
            Self::AnthemSubject => "unsupported anthem subject",
            Self::AdditionalLandPermission => "unsupported additional-land-play permission clause",
            Self::TargetOnlyRestriction => "unsupported target-only restriction clause",
            Self::GenericLine => "unsupported line",
            Self::ChooseLeadingSpell => "unsupported choose-leading spell clause",
            Self::TemporaryLosesAbilitiesBecomes => {
                "unsupported loses-all-abilities with becomes clause"
            }
            Self::StaticLosesAbilitiesBecomes => {
                "unsupported lose-all-abilities static becomes clause"
            }
            Self::ForAsLongAsPermission => "unsupported for-as-long-as play/cast permission clause",
            Self::MultiStepEachPlayer => "unsupported multi-step each-player clause with 'then'",
            Self::ArtifactCreaturePlayerTarget => {
                "unsupported target artifact-creature-or-player clause"
            }
            Self::CreatureTokenPlayerPlaneswalkerTarget => {
                "unsupported creature-token/player/planeswalker target clause"
            }
            Self::VillainousChoice => "unsupported villainous-choice clause",
            Self::LegendaryCopyException => "unsupported copy-spell legendary-exception clause",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum UnsupportedRuleMatch {
    Prefix,
    Exact,
    Contains,
}

#[derive(Debug, Clone, Copy)]
struct UnsupportedRule {
    match_kind: UnsupportedRuleMatch,
    phrase: &'static [&'static str],
    kind: UnsupportedRewriteLineKind,
}

const RULES: &[UnsupportedRule] = &[
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Prefix,
        phrase: &[
            "the", "first", "creature", "spell", "you", "cast", "each", "turn", "costs",
        ],
        kind: UnsupportedRewriteLineKind::FirstSpellCostModifier,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Prefix,
        phrase: &[
            "once", "each", "turn", "you", "may", "play", "a", "card", "from", "exile",
        ],
        kind: UnsupportedRewriteLineKind::StaticClause,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Prefix,
        phrase: &[
            "prevent", "the", "next", "1", "damage", "that", "would", "be", "dealt", "to", "any",
            "target", "this", "turn", "by", "red", "sources",
        ],
        kind: UnsupportedRewriteLineKind::TrailingPreventNextDamage,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Prefix,
        phrase: &["ninjutsu", "abilities", "you", "activate", "cost"],
        kind: UnsupportedRewriteLineKind::MarkerKeywordWithTail,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Exact,
        phrase: &[
            "creatures",
            "you",
            "control",
            "have",
            "haste",
            "and",
            "attack",
            "each",
            "combat",
            "if",
            "able",
        ],
        kind: UnsupportedRewriteLineKind::AnthemSubject,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Exact,
        phrase: &[
            "you", "may", "play", "any", "number", "of", "lands", "on", "each", "of", "your",
            "turns",
        ],
        kind: UnsupportedRewriteLineKind::AdditionalLandPermission,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Exact,
        phrase: &[
            "target",
            "creature",
            "can",
            "block",
            "any",
            "number",
            "of",
            "creatures",
            "this",
            "turn",
        ],
        kind: UnsupportedRewriteLineKind::TargetOnlyRestriction,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Exact,
        phrase: &["unleash", "while"],
        kind: UnsupportedRewriteLineKind::GenericLine,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &[
            "same", "name", "as", "another", "card", "in", "their", "hand",
        ],
        kind: UnsupportedRewriteLineKind::SameNameDiscard,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &[
            "enters", "tapped", "and", "doesnt", "untap", "during", "your", "untap", "step",
        ],
        kind: UnsupportedRewriteLineKind::MixedEntersTappedUntap,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &[
            "prevent",
            "all",
            "combat",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "this",
            "turn",
            "by",
            "creatures",
            "with",
            "power",
        ],
        kind: UnsupportedRewriteLineKind::PreventCombatDamageTail,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &["of", "defending", "players", "choice"],
        kind: UnsupportedRewriteLineKind::DefendingPlayerChoice,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &["if", "you", "sacrifice", "an", "island", "this", "way"],
        kind: UnsupportedRewriteLineKind::SacrificeIslandThisWay,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &[
            "create", "a", "token", "thats", "a", "copy", "of", "that", "aura", "attached", "to",
            "that", "creature",
        ],
        kind: UnsupportedRewriteLineKind::AuraCopyAttachment,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &[
            "with",
            "islandwalk",
            "can",
            "be",
            "blocked",
            "as",
            "though",
            "they",
            "didnt",
            "have",
            "islandwalk",
        ],
        kind: UnsupportedRewriteLineKind::LandwalkOverride,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &[
            "with",
            "power",
            "or",
            "toughness",
            "1",
            "or",
            "less",
            "cant",
            "be",
            "blocked",
        ],
        kind: UnsupportedRewriteLineKind::PowerOrToughnessUnblockable,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &[
            "discard",
            "up",
            "to",
            "two",
            "permanents",
            "then",
            "draw",
            "that",
            "many",
            "cards",
        ],
        kind: UnsupportedRewriteLineKind::DiscardQualifier,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &[
            "if", "your", "life", "total", "is", "less", "than", "or", "equal", "to", "half",
            "your", "starting", "life", "total", "plus", "one",
        ],
        kind: UnsupportedRewriteLineKind::Predicate,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &[
            "then",
            "sacrifices",
            "all",
            "creatures",
            "they",
            "control",
            "then",
            "puts",
            "all",
            "cards",
            "they",
            "exiled",
            "this",
            "way",
            "onto",
            "the",
            "battlefield",
        ],
        kind: UnsupportedRewriteLineKind::EachPlayerExileSacrificeReturn,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &["if", "this", "creature", "isnt", "saddled", "this", "turn"],
        kind: UnsupportedRewriteLineKind::SaddledConditional,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &[
            "put", "a", "card", "from", "among", "them", "into", "your", "hand", "this", "turn",
        ],
        kind: UnsupportedRewriteLineKind::LookedCardFallback,
    },
    UnsupportedRule {
        match_kind: UnsupportedRuleMatch::Contains,
        phrase: &[
            "if",
            "the",
            "sacrificed",
            "creature",
            "was",
            "a",
            "hamster",
            "this",
            "turn",
        ],
        kind: UnsupportedRewriteLineKind::Predicate,
    },
];

pub(crate) fn parse_unsupported_rewrite_line_kind(
    tokens: &[OwnedLexToken],
) -> Option<UnsupportedRewriteLineKind> {
    if supported_static_loses_abilities_becomes_line(tokens) {
        return None;
    }
    let words = TokenWordView::new(tokens).word_refs();
    for rule in RULES {
        if let Some(kind) = parse_static_rule(&words, *rule) {
            return Some(kind);
        }
    }

    let mut input: WordSliceInput<'_> = &words;
    alt((
        parse_choose_leading_spell,
        parse_loses_abilities_becomes,
        parse_for_as_long_as_permission,
        parse_multi_step_each_player,
        parse_artifact_creature_player_target,
        parse_creature_token_player_planeswalker_target,
        parse_villainous_choice,
        parse_legendary_copy_exception,
    ))
    .parse_next(&mut input)
    .ok()
}

fn supported_static_loses_abilities_becomes_sentence(tokens: &[OwnedLexToken]) -> bool {
    let Some(shape) = anthem_grants::parse_lose_all_abilities_shape(tokens) else {
        return false;
    };
    if !shape.becomes {
        return false;
    }
    let words = TokenWordView::new(tokens).word_refs();
    let Some(becomes_word) = words.iter().position(|word| *word == "becomes") else {
        return false;
    };
    let Some(power_toughness) = become_shapes::parse_become_base_pt_words(
        words.get(becomes_word + 1..).unwrap_or_default(),
    ) else {
        return false;
    };
    become_shapes::parse_become_creature_descriptor_words(power_toughness.descriptor_words)
        .is_some()
}

fn supported_static_loses_abilities_becomes_line(tokens: &[OwnedLexToken]) -> bool {
    let sentences = structure::split_lexed_sentences(tokens);
    match sentences.as_slice() {
        [sentence] => supported_static_loses_abilities_becomes_sentence(sentence),
        [sentence, continuation] => {
            supported_static_loses_abilities_becomes_sentence(sentence)
                && super::parse_static_effect_continues_until_end_of_turn_surface(continuation)
                    .is_some()
        }
        _ => false,
    }
}

fn parse_static_rule<'a>(
    words: &'a [&'a str],
    rule: UnsupportedRule,
) -> Option<UnsupportedRewriteLineKind> {
    let mut input: WordSliceInput<'a> = words;
    match rule.match_kind {
        UnsupportedRuleMatch::Prefix => rule_phrase(rule).parse_next(&mut input).ok(),
        UnsupportedRuleMatch::Exact => (rule_phrase(rule), eof)
            .map(|(kind, _)| kind)
            .parse_next(&mut input)
            .ok(),
        UnsupportedRuleMatch::Contains => scan_rule(rule).parse_next(&mut input).ok(),
    }
}

fn rule_phrase<'a>(
    rule: UnsupportedRule,
) -> impl Parser<WordSliceInput<'a>, UnsupportedRewriteLineKind, ErrMode<ContextError>> {
    move |input: &mut WordSliceInput<'a>| {
        parse_word_phrase(input, rule.phrase)?;
        Ok(rule.kind)
    }
}

fn scan_rule<'a>(
    rule: UnsupportedRule,
) -> impl Parser<WordSliceInput<'a>, UnsupportedRewriteLineKind, ErrMode<ContextError>> {
    move |input: &mut WordSliceInput<'a>| {
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(rule_phrase(rule)))
            .parse_next(input)?;
        rule_phrase(rule).parse_next(input)
    }
}

fn word_sequence<'a>(
    expected: &'static [&'static str],
) -> impl Parser<WordSliceInput<'a>, (), ErrMode<ContextError>> {
    move |input: &mut WordSliceInput<'a>| parse_word_phrase(input, expected)
}

fn locate_word_sequence<'a>(
    expected: &'static [&'static str],
) -> impl Parser<WordSliceInput<'a>, (), ErrMode<ContextError>> {
    move |input: &mut WordSliceInput<'a>| {
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(word_sequence(expected)))
            .parse_next(input)?;
        word_sequence(expected).parse_next(input)
    }
}

fn parse_word_phrase(
    input: &mut WordSliceInput<'_>,
    expected: &'static [&'static str],
) -> WResult<()> {
    for expected_word in expected {
        primitives::word_slice_exact(expected_word)
            .void()
            .parse_next(input)?;
    }
    Ok(())
}

fn parse_choose_leading_spell(
    input: &mut WordSliceInput<'_>,
) -> WResult<UnsupportedRewriteLineKind> {
    word_sequence(&["choose", "target", "land"]).parse_next(input)?;
    locate_word_sequence(&[
        "create", "three", "tokens", "that", "are", "copies", "of", "it",
    ])
    .parse_next(input)?;
    Ok(UnsupportedRewriteLineKind::ChooseLeadingSpell)
}

fn parse_loses_abilities_becomes(
    input: &mut WordSliceInput<'_>,
) -> WResult<UnsupportedRewriteLineKind> {
    let original = *input;
    locate_word_sequence(&["loses", "all", "abilities", "and", "becomes"]).parse_next(input)?;
    let mut prefix = original;
    let temporary = word_sequence(&["until", "end", "of", "turn"])
        .parse_next(&mut prefix)
        .is_ok();
    Ok(if temporary {
        UnsupportedRewriteLineKind::TemporaryLosesAbilitiesBecomes
    } else {
        UnsupportedRewriteLineKind::StaticLosesAbilitiesBecomes
    })
}

fn parse_for_as_long_as_permission(
    input: &mut WordSliceInput<'_>,
) -> WResult<UnsupportedRewriteLineKind> {
    let original = *input;
    locate_word_sequence(&[
        "for", "as", "long", "as", "that", "card", "remains", "exiled", "its", "owner", "may",
        "play", "it",
    ])
    .parse_next(input)?;
    for excluded in [
        &[
            "a", "spell", "cast", "by", "an", "opponent", "this", "way", "costs",
        ][..],
        &["a", "spell", "cast", "this", "way", "costs"][..],
    ] {
        let mut probe = original;
        if locate_word_sequence(excluded)
            .parse_next(&mut probe)
            .is_ok()
        {
            return Err(primitives::backtrack_err(
                "unsupported permission",
                "permission without supported spell-cost modifier",
            ));
        }
    }
    Ok(UnsupportedRewriteLineKind::ForAsLongAsPermission)
}

fn parse_multi_step_each_player(
    input: &mut WordSliceInput<'_>,
) -> WResult<UnsupportedRewriteLineKind> {
    let original = *input;
    locate_word_sequence(&[
        "each",
        "player",
        "loses",
        "x",
        "life",
        "discards",
        "x",
        "cards",
        "sacrifices",
        "x",
        "creatures",
    ])
    .parse_next(input)?;
    let mut probe = original;
    locate_word_sequence(&["then", "sacrifices", "x", "lands"]).parse_next(&mut probe)?;
    Ok(UnsupportedRewriteLineKind::MultiStepEachPlayer)
}

fn parse_artifact_creature_player_target(
    input: &mut WordSliceInput<'_>,
) -> WResult<UnsupportedRewriteLineKind> {
    word_sequence(&["target", "artifact", "creature", "or", "player"]).parse_next(input)?;
    Ok(UnsupportedRewriteLineKind::ArtifactCreaturePlayerTarget)
}

fn parse_creature_token_player_planeswalker_target(
    input: &mut WordSliceInput<'_>,
) -> WResult<UnsupportedRewriteLineKind> {
    word_sequence(&[
        "target",
        "creature",
        "token",
        "player",
        "or",
        "planeswalker",
    ])
    .parse_next(input)?;
    Ok(UnsupportedRewriteLineKind::CreatureTokenPlayerPlaneswalkerTarget)
}

fn parse_villainous_choice(input: &mut WordSliceInput<'_>) -> WResult<UnsupportedRewriteLineKind> {
    word_sequence(&["villainous"]).parse_next(input)?;
    Ok(UnsupportedRewriteLineKind::VillainousChoice)
}

fn parse_legendary_copy_exception(
    input: &mut WordSliceInput<'_>,
) -> WResult<UnsupportedRewriteLineKind> {
    word_sequence(&["copy", "target", "spell"]).parse_next(input)?;
    locate_word_sequence(&["legendary"]).parse_next(input)?;
    Ok(UnsupportedRewriteLineKind::LegendaryCopyException)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn classifies_prefix_exact_contains_and_composed_unsupported_shapes() {
        let prefix = lex_line(
            "The first creature spell you cast each turn costs {1} less to cast.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_unsupported_rewrite_line_kind(&prefix),
            Some(UnsupportedRewriteLineKind::FirstSpellCostModifier)
        );

        let exact = lex_line("Unleash while", 0).unwrap();
        assert_eq!(
            parse_unsupported_rewrite_line_kind(&exact),
            Some(UnsupportedRewriteLineKind::GenericLine)
        );

        let composed = lex_line("Copy target spell, except the copy is legendary.", 0).unwrap();
        assert_eq!(
            parse_unsupported_rewrite_line_kind(&composed),
            Some(UnsupportedRewriteLineKind::LegendaryCopyException)
        );
    }
}

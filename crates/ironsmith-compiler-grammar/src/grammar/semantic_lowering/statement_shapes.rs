use winnow::combinator::{peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use crate::cards::builders::PredicateAst;
use crate::color::{Color, ColorSet};
use crate::effect::Value;
use crate::object::CounterType;
use crate::types::Subtype;

use super::super::super::lexer::{
    LexStream, OwnedLexToken, TokenWordView, parser_token_word_refs, render_token_slice,
    trim_lexed_commas,
};
use super::super::{leaf, primitives};
use super::{
    any_phrase_is_present, any_word_is_present, every_phrase_is_present, phrase_is_exact,
    phrase_is_prefix, phrase_is_present, phrase_is_suffix, phrase_location, word_is_present,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DieRollAdjustmentSpec {
    pub life_cost: u32,
    pub adjustment: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenCharacteristicFollowup;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemporaryStaticFollowup {
    pub has_negation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReturnedObjectMoveHead;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnedObjectSubject {
    It,
    ThatCard,
    ThatCreature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnedObjectFollowupFacts<'a> {
    pub subject: ReturnedObjectSubject,
    pub keyword_tokens: Option<&'a [OwnedLexToken]>,
    pub colors: Option<ColorSet>,
    pub subtypes: Vec<Subtype>,
    pub has_base_power_toughness: bool,
    pub has_keyword_gain: bool,
}

impl ReturnedObjectFollowupFacts<'_> {
    pub fn has_characteristic_changes(&self) -> bool {
        self.keyword_tokens.is_some()
            || self.colors.is_some()
            || !self.subtypes.is_empty()
            || self.has_base_power_toughness
            || self.has_keyword_gain
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkedStatementSurface {
    ExiledCardCostsMore,
    ChooseTwoShuffleRest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementEffectPreference {
    EachPlayerChooseBounceThenDraw,
    LeadingEffectVerb,
    TargetedEffectAction,
    UnlessSearch,
    TargetBecomes,
    ConditionalPriorResult,
    ConditionalInstead,
    TargetedTemporaryModifier,
    CantCastNextTurn,
    TemporaryNegation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelfCounterEntrySpec {
    Unconditional {
        count: Value,
    },
    Adamant {
        condition: PredicateAst,
        predicate_body: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommaSplit<'a> {
    pub before: &'a [OwnedLexToken],
    pub after: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub struct SnowManaCounterEntrySpec<'a> {
    pub condition: PredicateAst,
    pub entry_tokens: &'a [OwnedLexToken],
    pub counter_type: CounterType,
    pub count: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayNightStartsDay;

fn fixed_leaf_number(words: &[&str]) -> Option<u32> {
    leaf::parse_leaf_number_prefix_words(words)
        .and_then(leaf::LeafNumberPrefix::into_fixed)
        .map(|(value, _)| value)
}

fn fixed_leaf_mana_symbol(word: &str) -> Option<crate::mana::ManaSymbol> {
    crate::grammar::primitives::probe_shape(
        leaf::parse_leaf_bare_mana_symbol_complete(word)
            .or_else(|_| leaf::parse_leaf_spelled_mana_word_complete(word)),
    )
}

pub fn parse_die_roll_adjustment_tokens(tokens: &[OwnedLexToken]) -> Option<DieRollAdjustmentSpec> {
    let words = parser_token_word_refs(tokens);
    if !phrase_is_prefix(&words, &["after", "you", "roll", "a", "die"])
        || !every_phrase_is_present(
            &words,
            &[
                &["you", "may", "pay"],
                &["if", "you", "do"],
                &["increase", "or", "decrease", "the", "result", "by"],
                &["do", "this", "only", "once", "each", "turn"],
            ],
        )
    {
        return None;
    }

    let life_cost = phrase_location(&words, &["pay"])
        .and_then(|idx| words.get(idx + 1))
        .and_then(|word| fixed_leaf_number(&[*word]))
        .unwrap_or(1);
    let adjustment = phrase_location(&words, &["by"])
        .and_then(|idx| words.get(idx + 1))
        .and_then(|word| fixed_leaf_number(&[*word]))
        .unwrap_or(1);
    Some(DieRollAdjustmentSpec {
        life_cost,
        adjustment,
    })
}

pub fn parse_token_characteristic_followup_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TokenCharacteristicFollowup> {
    let words = parser_token_word_refs(tokens);
    ((phrase_is_prefix(&words, &["its", "power", "is", "equal"])
        || phrase_is_prefix(&words, &["their", "power", "is", "equal"]))
        && word_is_present(&words, "toughness"))
    .then_some(TokenCharacteristicFollowup)
}

pub fn parse_temporary_static_followup_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TemporaryStaticFollowup> {
    let words = parser_token_word_refs(tokens);
    let source_conditional_duration = matches!(
        leaf::parse_leaf_conditional_duration_kind_tokens(tokens),
        Some(
            leaf::LeafConditionalDurationKind::YouControlSource
                | leaf::LeafConditionalDurationKind::SourceRemainsTapped
                | leaf::LeafConditionalDurationKind::SourceRemainsOnBattlefield
        )
    );
    (phrase_is_present(&words, &["this", "turn"]) || source_conditional_duration).then(|| {
        TemporaryStaticFollowup {
            has_negation: any_word_is_present(
                &words,
                &["cant", "can't", "dont", "don't", "doesnt", "doesn't"],
            ),
        }
    })
}

pub fn parse_returned_object_move_head_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ReturnedObjectMoveHead> {
    let words = parser_token_word_refs(tokens);
    ((word_is_present(&words, "return") || word_is_present(&words, "put"))
        && word_is_present(&words, "battlefield"))
    .then_some(ReturnedObjectMoveHead)
}

fn returned_subject(words: &[&str]) -> Option<(ReturnedObjectSubject, usize)> {
    if phrase_is_prefix(words, &["it"]) {
        Some((ReturnedObjectSubject::It, 1))
    } else if phrase_is_prefix(words, &["that", "card"]) {
        Some((ReturnedObjectSubject::ThatCard, 2))
    } else if phrase_is_prefix(words, &["that", "creature"]) {
        Some((ReturnedObjectSubject::ThatCreature, 2))
    } else {
        None
    }
}

fn returned_descriptor_words<'a>(words: &'a [&'a str], subject_words: usize) -> &'a [&'a str] {
    let verb = phrase_location(&words[subject_words..], &["is"])
        .or_else(|| phrase_location(&words[subject_words..], &["are"]))
        .map(|offset| offset + subject_words);
    let addition = phrase_location(words, &["in", "addition", "to"]);
    match (verb, addition) {
        (Some(verb), Some(addition)) if addition > verb + 1 => &words[verb + 1..addition],
        _ => &[],
    }
}

fn returned_keyword_tokens<'a>(
    tokens: &'a [OwnedLexToken],
    words: &[&str],
    subject_words: usize,
) -> Option<&'a [OwnedLexToken]> {
    let has_word = phrase_location(&words[subject_words..], &["has"])
        .or_else(|| phrase_location(&words[subject_words..], &["have"]))?
        + subject_words;
    let ability_start = has_word + 1;
    let ability_end = phrase_location(&words[ability_start..], &["and", "is"])
        .or_else(|| phrase_location(&words[ability_start..], &["and", "are"]))
        .map(|offset| offset + ability_start)
        .unwrap_or(words.len());
    if ability_end <= ability_start {
        return None;
    }
    let view = TokenWordView::new(tokens);
    let range = view.token_span_for_words(ability_start, ability_end)?;
    Some(&tokens[range])
}

pub fn parse_returned_object_followup_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ReturnedObjectFollowupFacts<'_>> {
    let words = parser_token_word_refs(tokens);
    let (subject, subject_words) = returned_subject(&words)?;
    let descriptor = returned_descriptor_words(&words, subject_words);

    let mut colors = ColorSet::new();
    let mut subtypes = Vec::new();
    for word in descriptor {
        if let Some(color) = Color::from_name(word) {
            colors = colors.union(ColorSet::from_color(color));
        }
        if let Some(subtype) = crate::util::parse_subtype_flexible(word)
            && !subtypes.iter().any(|existing| existing == &subtype)
        {
            subtypes.push(subtype);
        }
    }

    Some(ReturnedObjectFollowupFacts {
        subject,
        keyword_tokens: returned_keyword_tokens(tokens, &words, subject_words),
        colors: (!colors.is_empty()).then_some(colors),
        subtypes,
        has_base_power_toughness: phrase_is_present(
            &words[subject_words..],
            &["base", "power", "and", "toughness"],
        ),
        has_keyword_gain: words.get(subject_words).is_some_and(|word| {
            word.eq_ignore_ascii_case("gain") || word.eq_ignore_ascii_case("gains")
        }),
    })
}

pub fn parse_linked_statement_surface_tokens(
    tokens: &[OwnedLexToken],
) -> Option<LinkedStatementSurface> {
    let words = parser_token_word_refs(tokens);
    if every_phrase_is_present(
        &words,
        &[
            &[
                "for", "as", "long", "as", "that", "card", "remains", "exiled",
            ],
            &["more", "to", "cast"],
        ],
    ) {
        Some(LinkedStatementSurface::ExiledCardCostsMore)
    } else if every_phrase_is_present(
        &words,
        &[
            &["chooses", "two", "of", "those", "cards"],
            &["shuffle", "the", "chosen", "cards"],
            &["put", "the", "rest", "onto", "the", "battlefield"],
        ],
    ) {
        Some(LinkedStatementSurface::ChooseTwoShuffleRest)
    } else {
        None
    }
}

fn each_player_choose_bounce_then_draw(words: &[&str]) -> bool {
    phrase_is_prefix(
        words,
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
    ) && every_phrase_is_present(
        words,
        &[
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
            &[
                "you", "draw", "a", "card", "for", "each", "opponent", "who", "has", "more",
                "cards", "in", "their", "hand", "than", "you",
            ],
        ],
    )
}

fn leading_effect_verb(words: &[&str]) -> bool {
    [
        "add",
        "choose",
        "counter",
        "create",
        "deal",
        "destroy",
        "discard",
        "draw",
        "exchange",
        "exile",
        "gain",
        "look",
        "mill",
        "put",
        "return",
        "reveal",
        "sacrifice",
        "search",
        "shuffle",
        "surveil",
        "tap",
        "untap",
    ]
    .iter()
    .any(|verb| phrase_is_prefix(words, &[*verb]))
}

pub fn parse_statement_effect_preference_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StatementEffectPreference> {
    let words = parser_token_word_refs(tokens);
    if each_player_choose_bounce_then_draw(&words) {
        Some(StatementEffectPreference::EachPlayerChooseBounceThenDraw)
    } else if leading_effect_verb(&words) {
        Some(StatementEffectPreference::LeadingEffectVerb)
    } else if phrase_is_prefix(&words, &["target"])
        && any_word_is_present(
            &words,
            &[
                "sacrifice",
                "sacrifices",
                "discard",
                "discards",
                "draw",
                "draws",
                "gain",
                "gains",
                "lose",
                "loses",
                "mill",
                "mills",
                "search",
                "searches",
                "shuffle",
                "shuffles",
            ],
        )
    {
        Some(StatementEffectPreference::TargetedEffectAction)
    } else if phrase_is_prefix(&words, &["unless"]) && word_is_present(&words, "search") {
        Some(StatementEffectPreference::UnlessSearch)
    } else if phrase_is_prefix(&words, &["target"])
        && any_word_is_present(&words, &["become", "becomes"])
    {
        Some(StatementEffectPreference::TargetBecomes)
    } else if phrase_is_prefix(&words, &["if"])
        && any_phrase_is_present(
            &words,
            &[
                &["that", "card"],
                &["that", "creature"],
                &["that", "object"],
                &["that", "permanent"],
                &["those", "cards"],
                &["those", "creatures"],
                &["those", "objects"],
                &["those", "permanents"],
            ],
        )
    {
        // On a resolving spell, an explicit demonstrative points back to an
        // object or set produced by the preceding instruction. Some of these
        // clauses also form valid battlefield static abilities in isolation,
        // so keep the prior-result surface typed as an effect preference.
        Some(StatementEffectPreference::ConditionalPriorResult)
    } else if word_is_present(&words, "if") && word_is_present(&words, "instead") {
        Some(StatementEffectPreference::ConditionalInstead)
    } else if phrase_is_present(&words, &["until", "end", "of", "turn"])
        && word_is_present(&words, "target")
        && any_word_is_present(&words, &["get", "gets", "gain", "gains"])
    {
        Some(StatementEffectPreference::TargetedTemporaryModifier)
    } else if any_phrase_is_present(&words, &[&["cant", "cast"], &["can't", "cast"]])
        && phrase_is_present(&words, &["next", "turn"])
    {
        Some(StatementEffectPreference::CantCastNextTurn)
    } else if phrase_is_present(&words, &["until", "end", "of", "turn"])
        && any_word_is_present(
            &words,
            &["cant", "can't", "dont", "don't", "doesnt", "doesn't"],
        )
    {
        Some(StatementEffectPreference::TemporaryNegation)
    } else {
        None
    }
}

fn parse_comma_split<'a>(input: &mut LexStream<'a>) -> WResult<CommaSplit<'a>> {
    let before = repeat_till(0.., any.void(), peek(primitives::comma()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    let after = rest.parse_next(input)?;
    let before = trim_lexed_commas(before);
    let after = trim_lexed_commas(after);
    if before.is_empty() || after.is_empty() {
        return Err(primitives::backtrack_err(
            "semantic comma split",
            "non-empty clauses",
        ));
    }
    Ok(CommaSplit { before, after })
}

pub fn parse_comma_split_tokens(tokens: &[OwnedLexToken]) -> Option<CommaSplit<'_>> {
    crate::grammar::primitives::probe_all(tokens, parse_comma_split, "semantic-comma-split")
}

fn parse_adamant_condition(tokens: &[OwnedLexToken]) -> Option<(PredicateAst, String)> {
    let words = parser_token_word_refs(tokens);
    let body_start = phrase_location(&words, &["if"])? + 1;
    let body_words = words.get(body_start..)?;
    if body_words.len() != 11
        || !phrase_is_prefix(body_words, &["at", "least"])
        || !any_phrase_is_present(
            body_words,
            &[
                &["mana", "was", "spent", "to", "cast", "this", "spell"],
                &["mana", "were", "spent", "to", "cast", "this", "spell"],
            ],
        )
    {
        return None;
    }
    let amount = fixed_leaf_number(&body_words[2..3])?;
    let symbol = fixed_leaf_mana_symbol(body_words[3])?;
    let view = TokenWordView::new(tokens);
    let body_range = view.token_span_for_words(body_start, words.len())?;
    Some((
        PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount,
            symbol: Some(symbol),
        },
        render_token_slice(&tokens[body_range]).trim().to_string(),
    ))
}

fn is_single_plus_one_entry(words: &[&str]) -> bool {
    [
        &[
            "this", "creature", "enters", "with", "a", "+1/+1", "counter", "on", "it",
        ][..],
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
    ]
    .iter()
    .any(|expected| phrase_is_exact(words, expected))
}

fn is_x_plus_one_entry(words: &[&str]) -> bool {
    [
        &[
            "this", "creature", "enters", "with", "x", "+1/+1", "counters", "on", "it",
        ][..],
        &[
            "this",
            "permanent",
            "enters",
            "with",
            "x",
            "+1/+1",
            "counters",
            "on",
            "it",
        ],
        &["it", "enters", "with", "x", "+1/+1", "counters", "on", "it"],
    ]
    .iter()
    .any(|expected| phrase_is_prefix(words, expected))
}

pub fn parse_self_counter_entry_tokens(tokens: &[OwnedLexToken]) -> Option<SelfCounterEntrySpec> {
    let words = parser_token_word_refs(tokens);
    if is_single_plus_one_entry(&words) {
        return Some(SelfCounterEntrySpec::Unconditional {
            count: Value::Fixed(1),
        });
    }

    if let Some(split) = parse_comma_split_tokens(tokens)
        && is_single_plus_one_entry(&parser_token_word_refs(split.after))
        && let Some((condition, predicate_body)) = parse_adamant_condition(split.before)
    {
        return Some(SelfCounterEntrySpec::Adamant {
            condition,
            predicate_body,
        });
    }

    if !is_x_plus_one_entry(&words) {
        return None;
    }
    let revealed_total = any_phrase_is_present(
        &words,
        &[
            &[
                "where", "x", "is", "the", "total", "mana", "value", "of", "all", "cards",
                "revealed", "this", "way",
            ],
            &[
                "where", "x", "is", "the", "total", "mana", "value", "of", "cards", "revealed",
                "this", "way",
            ],
        ],
    );
    let count = if revealed_total {
        Value::TotalManaValue(crate::target::ObjectFilter::tagged(
            crate::tag::CompilerReferenceTag::PublicRevealed.bind(),
        ))
    } else {
        Value::X
    };
    Some(SelfCounterEntrySpec::Unconditional { count })
}

fn parse_snow_condition(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let words = parser_token_word_refs(tokens);
    let body_start = phrase_location(&words, &["if"])? + 1;
    let body_words = words.get(body_start..)?;
    let expected_tail = &[
        "of", "any", "of", "that", "spell", "colors", "was", "spent", "to", "cast", "it",
    ];
    let plural_tail = &[
        "of", "any", "of", "that", "spells", "colors", "was", "spent", "to", "cast", "it",
    ];
    if body_words.len() != expected_tail.len() + 1
        || !(phrase_is_exact(&body_words[1..], expected_tail)
            || phrase_is_exact(&body_words[1..], plural_tail)
            || phrase_is_exact(
                &body_words[1..],
                &[
                    "of", "any", "of", "that", "spell's", "colors", "was", "spent", "to", "cast",
                    "it",
                ],
            ))
    {
        return None;
    }
    let symbol = crate::grammar::primitives::probe_shape(
        crate::grammar::values::parse_mana_symbol(body_words[0]),
    )?;
    (symbol == crate::mana::ManaSymbol::Snow)
        .then_some(PredicateAst::SnowManaOfAnySpellColorSpentToCastThisSpell)
}

fn parse_entry_counter_count(words: &[&str]) -> Value {
    let Some(additional) = phrase_location(words, &["additional"]) else {
        return Value::Fixed(1);
    };
    let before = additional
        .checked_sub(1)
        .and_then(|idx| words.get(idx))
        .and_then(|word| fixed_leaf_number(&[*word]));
    let after = words
        .get(additional + 1)
        .and_then(|word| fixed_leaf_number(&[*word]));
    Value::Fixed(before.or(after).unwrap_or(1) as i32)
}

pub fn parse_snow_mana_counter_entry_tokens(
    effect_tokens: &[OwnedLexToken],
    intervening_snow_condition: bool,
) -> Option<SnowManaCounterEntrySpec<'_>> {
    let (condition, entry_tokens) = if intervening_snow_condition {
        (
            PredicateAst::SnowManaOfAnySpellColorSpentToCastThisSpell,
            effect_tokens,
        )
    } else {
        let split = parse_comma_split_tokens(effect_tokens)?;
        (parse_snow_condition(split.before)?, split.after)
    };

    let words = parser_token_word_refs(entry_tokens);
    if !phrase_is_prefix(&words, &["that", "creature", "enters"])
        || !phrase_is_present(&words, &["with", "an", "additional"])
        || !word_is_present(&words, "counter")
        || !phrase_is_suffix(&words, &["on", "it"])
    {
        return None;
    }
    let counter_type = crate::util::parse_counter_type_from_tokens(entry_tokens)?;
    Some(SnowManaCounterEntrySpec {
        condition,
        entry_tokens,
        counter_type,
        count: parse_entry_counter_count(&words),
    })
}

pub fn parse_day_night_starts_day_tokens(tokens: &[OwnedLexToken]) -> Option<DayNightStartsDay> {
    let words = parser_token_word_refs(tokens);
    (phrase_is_present(&words, &["neither", "day", "nor", "night"])
        && phrase_is_present(&words, &["becomes", "day"])
        && any_phrase_is_present(
            &words,
            &[
                &["as", "this", "creature", "enters"],
                &["as", "this", "permanent", "enters"],
                &["as", "this", "object", "enters"],
            ],
        ))
    .then_some(DayNightStartsDay)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_die_roll_and_statement_preferences() {
        let tokens = lex_line(
            "After you roll a die, you may pay 2 life. If you do, increase or decrease the result by 3. Do this only once each turn.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_die_roll_adjustment_tokens(&tokens),
            Some(DieRollAdjustmentSpec {
                life_cost: 2,
                adjustment: 3,
            })
        );
        let effect = lex_line("Target creature gets +2/+2 until end of turn.", 0).unwrap();
        assert_eq!(
            parse_statement_effect_preference_tokens(&effect),
            Some(StatementEffectPreference::TargetedTemporaryModifier)
        );
        let targeted_action = lex_line(
            "Target player sacrifices a creature of their choice, then gains life equal to that creature's toughness.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_statement_effect_preference_tokens(&targeted_action),
            Some(StatementEffectPreference::TargetedEffectAction)
        );

        for text in [
            "If X is 6 or more, those permanents are 4/4 creatures in addition to their other types.",
            "If there are two or more instant and/or sorcery cards in your graveyard, that creature enters with two additional +1/+1 counters on it.",
        ] {
            let effect = lex_line(text, 0).unwrap();
            assert_eq!(
                parse_statement_effect_preference_tokens(&effect),
                Some(StatementEffectPreference::ConditionalPriorResult),
                "{text}"
            );
        }

        for text in [
            "If you control a Plains, creatures you control get +1/+1.",
            "If this creature entered this turn, it has haste.",
        ] {
            let static_ability = lex_line(text, 0).unwrap();
            assert_eq!(
                parse_statement_effect_preference_tokens(&static_ability),
                None,
                "{text}"
            );
        }
    }

    #[test]
    fn parses_returned_object_characteristics() {
        let tokens = lex_line(
            "That creature has flying and is a red Dragon in addition to its other colors and types.",
            0,
        )
        .unwrap();
        let facts = parse_returned_object_followup_tokens(&tokens).unwrap();
        assert_eq!(facts.subject, ReturnedObjectSubject::ThatCreature);
        assert!(facts.keyword_tokens.is_some());
        assert!(facts.colors.is_some());
        assert_eq!(facts.subtypes, vec![Subtype::Dragon]);
    }

    #[test]
    fn parses_counter_entry_shapes() {
        for text in [
            "If at least three {W} mana was spent to cast this spell, this creature enters with a +1/+1 counter on it.",
            "If at least three white mana was spent to cast this spell, this creature enters with a +1/+1 counter on it.",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            assert!(matches!(
                parse_self_counter_entry_tokens(&tokens),
                Some(SelfCounterEntrySpec::Adamant { .. })
            ));
        }

        let tokens = lex_line(
            "This creature enters with X +1/+1 counters on it, where X is the total mana value of cards revealed this way.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_self_counter_entry_tokens(&tokens),
            Some(SelfCounterEntrySpec::Unconditional {
                count: Value::TotalManaValue(_)
            })
        ));
    }
}

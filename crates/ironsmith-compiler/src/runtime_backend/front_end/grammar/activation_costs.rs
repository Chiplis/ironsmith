use winnow::combinator::repeat;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::ChoiceCount;
use crate::color::ColorSet;
use crate::effect::{Comparison, Value};
use crate::mana::ManaCost;
use crate::object::CounterType;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

use super::super::lexer::{LexStream, OwnedLexToken};
use super::primitives::TokenWordView;

#[path = "activation_costs/simple_segments.rs"]
mod simple_segments;
pub(crate) use simple_segments::*;

#[path = "activation_costs/cant_shapes.rs"]
pub(crate) mod cant_shapes;

#[path = "activation_costs/selectors.rs"]
mod selectors;
pub(crate) use selectors::*;

#[path = "activation_costs/counter_segments.rs"]
mod counter_segments;
pub(crate) use counter_segments::*;

#[path = "activation_costs/zone_segments.rs"]
mod zone_segments;
pub(crate) use zone_segments::*;

#[path = "activation_costs/object_segments.rs"]
mod object_segments;
pub(crate) use object_segments::*;

#[path = "activation_costs/exile_segments.rs"]
mod exile_segments;
pub(crate) use exile_segments::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActivationCostCst {
    pub(crate) raw: String,
    pub(crate) segments: Vec<ActivationCostSegmentCst>,
    pub(crate) alternative_branches: Vec<ActivationCostCst>,
    pub(crate) is_loyalty_shorthand: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub(crate) enum ActivationCostSegmentCst {
    Mana(ManaCost),
    Tap,
    TapChosen {
        count: u32,
        filter: ObjectFilter,
    },
    Untap,
    Life(Value),
    Energy(u32),
    DiscardSource,
    DiscardHand,
    DiscardCard(u32),
    DiscardFiltered {
        count: u32,
        card_types: Vec<CardType>,
        supertypes: Vec<Supertype>,
        filter: Option<ObjectFilter>,
        random: bool,
        name: Option<String>,
        other: bool,
    },
    Mill(u32),
    SacrificeSelf,
    SacrificeCreature,
    SacrificeChosen {
        count: crate::effect::ChoiceCount,
        filter: ObjectFilter,
    },
    UnattachChosen {
        count: u32,
        filter: ObjectFilter,
    },
    ExileSelf,
    ExileSelfFromGraveyard,
    ExileFromHand {
        count: u32,
        color_filter: Option<ColorSet>,
    },
    ExileFromGraveyard {
        count: u32,
        card_type: Option<CardType>,
    },
    ExileChosen {
        choice_count: ChoiceCount,
        filter: ObjectFilter,
    },
    ExileSelfAndNamedArtifacts {
        names: Vec<String>,
    },
    ExileTopLibrary {
        count: u32,
    },
    RevealSourceFromHand,
    RevealFromHand {
        count: Value,
        color_filter: Option<ColorSet>,
        card_type: Option<CardType>,
    },
    ReturnSelfToHand,
    ReturnChosenToHand {
        count: u32,
        filter: ObjectFilter,
    },
    MoveOpponentOwnedExiledCardToGraveyard,
    ExertSelf {
        display_text: String,
    },
    PutCounters {
        counter_type: CounterType,
        count: u32,
    },
    PutCountersChosen {
        counter_type: CounterType,
        count: u32,
        filter: ObjectFilter,
        source_equivalent: bool,
    },
    Blight {
        count: u32,
    },
    RemoveCounters {
        counter_type: CounterType,
        count: u32,
    },
    RemoveCountersAmong {
        counter_type: Option<CounterType>,
        count: u32,
        filter: ObjectFilter,
        display_x: bool,
        dynamic: bool,
    },
    RemoveCountersDynamic {
        counter_type: Option<CounterType>,
        display_x: bool,
        remove_all: bool,
    },
    Behold {
        subtype: Subtype,
        count: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivationCostSegmentKind {
    Pay,
    Discard,
    Mill,
    Sacrifice,
    Unattach,
    TapChosen,
    Behold,
    Blight,
    Exile,
    Reveal,
    Return,
    Exert,
    PutCounter,
    RemoveCounter,
    BareSymbol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActivationMinimumCount {
    pub(crate) count: u32,
    pub(crate) consumed_words: usize,
}

pub(crate) fn parse_activation_minimum_count_words(
    words: &[&str],
) -> Option<ActivationMinimumCount> {
    let parsed = super::shared_util::value_shapes::parse_quantity_comparison_prefix_words(
        words, false, true,
    )?;
    let count = match parsed.comparison {
        Comparison::GreaterThanOrEqual(value) if value >= 0 => value as u32,
        Comparison::GreaterThan(value) if value >= -1 => (value + 1) as u32,
        _ => return None,
    };
    Some(ActivationMinimumCount {
        count,
        consumed_words: parsed.consumed_words,
    })
}

/// A token span selected from the word projection of an activation-cost segment.
///
/// Keeping this mapping in the grammar layer lets typed activation parsers retain
/// original token boundaries even when a lexer token contributes multiple words.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ActivationCostTokenSpan<'a> {
    pub(crate) tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_activation_cost_word_suffix(
    tokens: &[OwnedLexToken],
    first_word: usize,
) -> Option<ActivationCostTokenSpan<'_>> {
    let view = TokenWordView::new(tokens);
    let first_token = if first_word == 0 {
        0
    } else {
        view.token_start_indices().get(first_word).copied()?
    };
    Some(ActivationCostTokenSpan {
        tokens: &tokens[first_token..],
    })
}

pub(crate) fn parse_activation_cost_segment_kind_tokens(
    tokens: &[OwnedLexToken],
) -> ActivationCostSegmentKind {
    let mut input = LexStream::new(tokens);
    parse_activation_cost_segment_kind_lexed
        .parse_next(&mut input)
        .unwrap_or(ActivationCostSegmentKind::BareSymbol)
}

pub(crate) fn parse_activation_cost_segment_kind_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivationCostSegmentKind> {
    let tokens: Vec<&OwnedLexToken> = repeat(0.., any).parse_next(input)?;
    let Some(first) = tokens.first().and_then(|token| token.as_word()) else {
        return Ok(ActivationCostSegmentKind::BareSymbol);
    };
    let normalized_first = first.to_ascii_lowercase();
    let kind = match normalized_first.as_str() {
        "pay" => ActivationCostSegmentKind::Pay,
        "discard" => ActivationCostSegmentKind::Discard,
        "mill" => ActivationCostSegmentKind::Mill,
        "sacrifice" => ActivationCostSegmentKind::Sacrifice,
        "unattach" => ActivationCostSegmentKind::Unattach,
        "behold" => ActivationCostSegmentKind::Behold,
        "blight" => ActivationCostSegmentKind::Blight,
        "exile" => ActivationCostSegmentKind::Exile,
        "reveal" => ActivationCostSegmentKind::Reveal,
        "return" => ActivationCostSegmentKind::Return,
        "exert" => ActivationCostSegmentKind::Exert,
        "put" => ActivationCostSegmentKind::PutCounter,
        "remove" => ActivationCostSegmentKind::RemoveCounter,
        "tap" if token_words_include(&tokens, "untapped") => ActivationCostSegmentKind::TapChosen,
        _ => ActivationCostSegmentKind::BareSymbol,
    };
    Ok(kind)
}

fn token_words_include(tokens: &[&OwnedLexToken], expected: &str) -> bool {
    for token in tokens {
        if token
            .as_word()
            .is_some_and(|word| word.eq_ignore_ascii_case(expected))
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn minimum_count_parser_preserves_threshold_surface() {
        for (words, expected) in [
            (&["at", "least", "three"][..], (3, 3)),
            (&["greater", "than", "two"][..], (3, 3)),
            (&["a"][..], (1, 1)),
            (&["four", "or", "more"][..], (4, 3)),
        ] {
            let parsed = parse_activation_minimum_count_words(words).unwrap();
            assert_eq!((parsed.count, parsed.consumed_words), expected);
        }
        assert!(parse_activation_minimum_count_words(&["exactly", "two"]).is_none());
    }

    #[test]
    fn segment_kind_parser_preserves_simple_and_tap_chosen_dispatch() {
        for (raw, expected) in [
            ("pay 2 life", ActivationCostSegmentKind::Pay),
            ("mill three cards", ActivationCostSegmentKind::Mill),
            (
                "tap an untapped creature",
                ActivationCostSegmentKind::TapChosen,
            ),
            ("{2}", ActivationCostSegmentKind::BareSymbol),
        ] {
            let tokens = lex_line(raw, 0).unwrap();
            assert_eq!(parse_activation_cost_segment_kind_tokens(&tokens), expected);
        }
    }
}

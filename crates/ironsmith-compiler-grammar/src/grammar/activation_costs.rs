use winnow::combinator::repeat;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::ChoiceCount;
use crate::color::ColorSet;
use crate::effect::Value;
use crate::mana::ManaCost;
use crate::object::CounterType;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype, Supertype};

use super::super::lexer::{LexStream, OwnedLexToken};

#[path = "activation_costs/simple_segments.rs"]
mod simple_segments;
pub use simple_segments::*;

#[path = "activation_costs/cant_shapes.rs"]
pub mod cant_shapes;

#[path = "activation_costs/selectors.rs"]
mod selectors;
pub use selectors::*;

#[path = "activation_costs/counter_segments.rs"]
mod counter_segments;
pub use counter_segments::*;

#[path = "activation_costs/zone_segments.rs"]
mod zone_segments;
pub use zone_segments::*;

#[path = "activation_costs/object_segments.rs"]
mod object_segments;
pub use object_segments::*;

#[path = "activation_costs/exile_segments.rs"]
mod exile_segments;
pub use exile_segments::*;

#[path = "activation_costs/components.rs"]
mod program;
pub use program::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ActivationCostCst {
    pub raw: String,
    pub segments: Vec<ActivationCostSegmentCst>,
    pub alternative_branches: Vec<ActivationCostCst>,
    pub is_loyalty_shorthand: bool,
    pub waterbend_generic: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActivationCostSegmentCst {
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
    SacrificeSelf {
        surface: Option<crate::target::SourceReferenceSurface>,
    },
    SacrificeCreature,
    SacrificeChosen {
        count: crate::effect::ChoiceCount,
        filter: ObjectFilter,
    },
    SacrificeAll {
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
    ExileChosen {
        choice_count: ChoiceCount,
        filter: ObjectFilter,
        top_only: bool,
        turn_face_up: bool,
    },
    /// A single authored exile cost containing both the source and a
    /// separately quantified set ("this Vehicle and four other ...").
    /// Keeping the two selectors distinct preserves both source identity and
    /// the exact cardinality of the other objects.
    ExileSourceAndChosen {
        source_filter: ObjectFilter,
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
    MoveChosenToLibraryTop {
        filter: ObjectFilter,
    },
    MoveSelfToLibraryBottom {
        surface: crate::target::SourceReferenceSurface,
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
        single_object: bool,
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
pub enum ActivationCostSegmentKind {
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

pub fn parse_activation_cost_segment_kind_tokens(
    tokens: &[OwnedLexToken],
) -> ActivationCostSegmentKind {
    let mut input = LexStream::new(tokens);
    parse_activation_cost_segment_kind_lexed
        .parse_next(&mut input)
        .unwrap_or(ActivationCostSegmentKind::BareSymbol)
}

pub fn parse_activation_cost_segment_kind_lexed<'a>(
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

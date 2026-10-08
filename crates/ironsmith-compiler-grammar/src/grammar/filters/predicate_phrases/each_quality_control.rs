//! "you control a land of each basic land type and a creature of each color"
//! (Coalition Victory): a conjunction of per-quality control requirements.
//! "A land of each basic land type" requires, for every basic land type, a
//! land you control with that type (one dual land can satisfy two types);
//! "a creature of each color" requires, for every color, a creature you
//! control of that color (CR 205.3i basic land types, CR 105.1 colors).
use super::*;

const BASIC_LAND_TYPES: [crate::types::Subtype; 5] = [
    crate::types::Subtype::Plains,
    crate::types::Subtype::Island,
    crate::types::Subtype::Swamp,
    crate::types::Subtype::Mountain,
    crate::types::Subtype::Forest,
];

const COLORS: [crate::color::ColorSet; 5] = [
    crate::color::ColorSet::WHITE,
    crate::color::ColorSet::BLUE,
    crate::color::ColorSet::BLACK,
    crate::color::ColorSet::RED,
    crate::color::ColorSet::GREEN,
];

#[derive(Clone, Copy)]
enum EachQuality {
    BasicLandType,
    Color,
}

pub(super) fn parse_each_quality_control_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let starts = view.token_start_indices();
    if words.get(..2) != Some(&["you", "control"][..]) {
        return Ok(None);
    }
    let mut predicates = Vec::new();
    let mut idx = 2usize;
    while idx < words.len() {
        if !matches!(words.get(idx), Some(&("a" | "an"))) {
            return Ok(None);
        }
        let noun_start = idx + 1;
        let Some(of_offset) = words[noun_start..].iter().position(|word| *word == "of") else {
            return Ok(None);
        };
        let of_idx = noun_start + of_offset;
        if of_idx == noun_start || words.get(of_idx + 1) != Some(&"each") {
            return Ok(None);
        }
        let (quality, consumed) = if words.get(of_idx + 2..of_idx + 5)
            == Some(&["basic", "land", "type"][..])
        {
            (EachQuality::BasicLandType, 5)
        } else if words.get(of_idx + 2) == Some(&"color") {
            (EachQuality::Color, 3)
        } else {
            return Ok(None);
        };
        let (Some(&noun_token), Some(&of_token)) = (starts.get(noun_start), starts.get(of_idx))
        else {
            return Ok(None);
        };
        let Some(mut base) =
            crate::grammar::primitives::probe_shape(parse_object_filter(&tokens[noun_token..of_token], false))
        else {
            return Ok(None);
        };
        base.controller = Some(PlayerFilter::You);
        let leaves = match quality {
            EachQuality::BasicLandType => BASIC_LAND_TYPES
                .iter()
                .map(|subtype| base.clone().with_subtype(*subtype))
                .collect::<Vec<_>>(),
            EachQuality::Color => COLORS
                .iter()
                .map(|color| base.clone().with_colors(*color))
                .collect::<Vec<_>>(),
        };
        predicates.extend(leaves.into_iter().map(|filter| {
            PredicateAst::Player(PlayerPredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter,
            })
        }));
        idx = of_idx + consumed;
        if idx < words.len() {
            if words[idx] != "and" {
                return Ok(None);
            }
            idx += 1;
            if idx == words.len() {
                return Ok(None);
            }
        }
    }
    let mut predicates = predicates.into_iter();
    let Some(first) = predicates.next() else {
        return Ok(None);
    };
    Ok(Some(predicates.fold(first, |left, right| {
        PredicateAst::And(Box::new(left), Box::new(right))
    })))
}

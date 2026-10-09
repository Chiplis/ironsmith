//! "Domain — For each basic land type among lands you control, this creature
//! has landwalk of that type." (Magnigoth Treefolk)
//!
//! Landwalk of a basic land type (CR 702.14) for each basic land type among
//! the controller's lands is the same as five conditional abilities: this
//! creature has plainswalk as long as you control a Plains, islandwalk as long
//! as you control an Island, and so on.

use super::*;

const DOMAIN_LANDWALK_WORDS: &[&str] = &[
    "for", "each", "basic", "land", "type", "among", "lands", "you", "control", "this",
    "creature", "has", "landwalk", "of", "that", "type",
];

pub fn parse_domain_landwalk_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let body = match words.first() {
        Some(&"domain") => &words[1..],
        _ => &words[..],
    };
    if !crate::word_primitives::parse_sequence_complete(body, DOMAIN_LANDWALK_WORDS) {
        return Ok(None);
    }
    use crate::types::Subtype;
    Ok(Some(
        [
            Subtype::Plains,
            Subtype::Island,
            Subtype::Swamp,
            Subtype::Mountain,
            Subtype::Forest,
        ]
        .into_iter()
        .map(|land_type| StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::landwalk(land_type))),
            condition: PredicateAst::YouControl(ObjectFilter::land().with_subtype(land_type)),
        })
        .collect(),
    ))
}

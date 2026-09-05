use super::*;

pub fn parse_where_x_value_shape_tokens(
    where_tokens: &[OwnedLexToken],
    stripped_references_target: bool,
) -> Option<WhereXValueShape> {
    // The typed "where X" values with a complete grammar each, in one table;
    // colors-among precedes the prior-effect metric because its surface
    // contains the generic prior-reference words.
    for parser in [
        parse_commander_choice_where_lexed
            as for<'a> fn(&mut LexStream<'a>) -> WResult<WhereXValueShape>,
        parse_tap_cost_power_where_lexed,
        parse_chosen_objects_power_difference_where_lexed,
        parse_colors_among_where_lexed,
        parse_prior_effect_where_lexed,
    ] {
        if let Ok(shape) = primitives::parse_all(where_tokens, parser, "typed where X value") {
            return Some(shape);
        }
    }
    if let Ok((metric, surface)) = primitives::parse_all(
        where_tokens,
        parse_reference_metric_where_lexed,
        "where X reference metric",
    ) {
        let reference = match surface {
            ReferenceSurface::Its => {
                if stripped_references_target {
                    WhereXReferenceShape::Target
                } else {
                    WhereXReferenceShape::Source
                }
            }
            ReferenceSurface::ThisCreature => WhereXReferenceShape::Source,
            ReferenceSurface::ThatSpell => WhereXReferenceShape::TaggedIt,
            ReferenceSurface::ThatCreature => {
                if stripped_references_target {
                    WhereXReferenceShape::Target
                } else {
                    WhereXReferenceShape::TaggedIt
                }
            }
        };
        return Some(WhereXValueShape::ReferenceMetric { reference, metric });
    }
    for parser in [
        parse_commander_cast_count_where_lexed
            as for<'a> fn(&mut LexStream<'a>) -> WResult<WhereXValueShape>,
        parse_card_types_in_your_graveyard_where_lexed,
        parse_sacrifice_cost_where_lexed,
        parse_two_plus_sacrificed_where_lexed,
        parse_counter_reference_where_lexed,
    ] {
        if let Ok(shape) = primitives::parse_all(where_tokens, parser, "typed where X value") {
            return Some(shape);
        }
    }
    None
}

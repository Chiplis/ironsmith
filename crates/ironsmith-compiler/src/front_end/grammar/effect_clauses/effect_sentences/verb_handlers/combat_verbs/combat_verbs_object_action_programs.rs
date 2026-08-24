use super::*;

pub fn parse_instead_if_control_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(shape) = combat_grammar::parse_combat_control_predicate_shape_lexed(tokens) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter(shape.filter_tokens, shape.other)?;
    if let Some(relation) = shape.power_toughness_relation {
        filter.power_toughness_relation = Some(relation);
    }
    if let Some(count) = shape.min_count {
        if shape.requires_different_powers {
            return Ok(Some(PredicateAst::PlayerHasAtLeastWithDifferentPowers {
                player: PlayerAst::You,
                filter,
                count,
            }));
        }
        Ok(Some(PredicateAst::PlayerHasAtLeast {
            player: PlayerAst::You,
            filter,
            count,
        }))
    } else {
        Ok(Some(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        }))
    }
}

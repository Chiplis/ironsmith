use super::{ActivationCostSegmentCst, parse_activation_choice_prefix_tokens};
use crate::cards::builders::CardTextError;
use crate::lexer::OwnedLexToken;
use crate::target::PlayerFilter;
use crate::zone::Zone;

pub(super) fn parse_grouped_hand_cost(
    tokens: &[OwnedLexToken],
    reveal: bool,
) -> Option<Result<ActivationCostSegmentCst, CardTextError>> {
    let suffixes: &[(&[&str], u8)] = &[
        (&["with", "the", "same", "name"], 0),
        (&["with", "different", "names"], 1),
        (&["that", "share", "a", "color"], 2),
    ];
    let (suffix, relation) = suffixes.iter().find(|(suffix, _)| {
        tokens.len() > suffix.len()
            && tokens[tokens.len() - suffix.len()..]
                .iter()
                .zip(*suffix)
                .all(|(token, word)| token.is_word(word))
    })?;
    Some((|| {
        let error = || {
            CardTextError::ParseError("grouped hand cost needs an exact hand-card selection".into())
        };
        if !tokens
            .first()
            .is_some_and(|token| token.is_word(if reveal { "reveal" } else { "discard" }))
        {
            return Err(error());
        }
        let prefix = parse_activation_choice_prefix_tokens(&tokens[1..tokens.len() - suffix.len()])
            .ok_or_else(error)?;
        if prefix.count.dynamic_x
            || prefix.count.max != Some(prefix.count.min)
            || prefix.count.min == 0
        {
            return Err(error());
        }
        if !prefix
            .rest
            .iter()
            .any(|token| token.is_word("card") || token.is_word("cards"))
        {
            return Err(error());
        }
        let mut filter = super::super::filters::parse_object_filter_with_grammar_entrypoint_lexed(
            prefix.rest,
            false,
        )?;
        if filter.zone.is_some_and(|zone| zone != Zone::Hand)
            || filter
                .owner
                .as_ref()
                .is_some_and(|owner| *owner != PlayerFilter::You)
        {
            return Err(error());
        }
        filter.zone = Some(Zone::Hand);
        filter.owner = Some(PlayerFilter::You);
        match relation {
            0 => filter.shares_name = true,
            1 => filter.distinct_names = true,
            _ => filter.shares_color = true,
        }
        Ok(ActivationCostSegmentCst::GroupedHandSelection {
            count: prefix.count.min as u32,
            filter,
            reveal,
            tag: crate::util::helper_tag_for_tokens(tokens, "grouped_hand_cost"),
        })
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;
    #[test]
    fn exact_grouped_costs_keep_relation_zone_owner_and_whole_card_filter() {
        for (text, reveal, relation) in [
            (
                "reveal two cards from your hand that share a color",
                true,
                0,
            ),
            ("discard three cards with different names", false, 1),
            ("discard two nonland cards with the same name", false, 2),
        ] {
            let parsed = parse_grouped_hand_cost(&lex_line(text, 0).unwrap(), reveal)
                .unwrap()
                .unwrap();
            let ActivationCostSegmentCst::GroupedHandSelection {
                count,
                filter,
                reveal: actual,
                ..
            } = parsed
            else {
                unreachable!()
            };
            assert_eq!(actual, reveal);
            assert_eq!(filter.zone, Some(Zone::Hand));
            assert_eq!(filter.owner, Some(PlayerFilter::You));
            assert!(match relation {
                0 => count == 2 && filter.shares_color,
                1 => count == 3 && filter.distinct_names,
                _ =>
                    count == 2
                        && filter.shares_name
                        && filter
                            .excluded_card_types
                            .contains(&crate::types::CardType::Land),
            });
        }
        for text in [
            "discard x cards with the same name",
            "discard two creature cards from your graveyard with the same name",
        ] {
            assert!(
                parse_grouped_hand_cost(&lex_line(text, 0).unwrap(), false)
                    .unwrap()
                    .is_err()
            );
        }
    }
}

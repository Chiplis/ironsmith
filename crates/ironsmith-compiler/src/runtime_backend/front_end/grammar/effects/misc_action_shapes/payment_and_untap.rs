use winnow::combinator::alt;
use winnow::prelude::*;

use crate::mana::ManaSymbol;
use crate::runtime_backend::lexer::{LexStream, OwnedLexToken};

use super::super::super::{permission_shapes, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UntapActionShape<'a> {
    All {
        filter_tokens: &'a [OwnedLexToken],
    },
    Tagged {
        filter_tokens: Option<&'a [OwnedLexToken]>,
    },
    Explicit {
        target_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConjoinedUntapAllShape<'a> {
    pub(crate) left_filter_tokens: &'a [OwnedLexToken],
    pub(crate) right_filter_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_conjoined_untap_all_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConjoinedUntapAllShape<'_>> {
    let after_all = primitives::strip_lexed_prefix_phrase(tokens, &["all"])?;
    let (left_filter_tokens, right_filter_tokens) =
        primitives::split_lexed_once_on_separator(after_all, || {
            primitives::phrase(&["and", "all"]).void()
        })?;
    if left_filter_tokens.is_empty() || right_filter_tokens.is_empty() {
        return None;
    }
    Some(ConjoinedUntapAllShape {
        left_filter_tokens,
        right_filter_tokens,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepeatedTaggedManaPayment {
    pub(crate) pip_groups: Vec<Vec<ManaSymbol>>,
}

pub(crate) fn parse_untap_action_tokens(tokens: &[OwnedLexToken]) -> UntapActionShape<'_> {
    if let Some((_, filter_tokens)) = primitives::parse_prefix(
        tokens,
        alt((primitives::kw("all"), primitives::kw("each"))).void(),
    ) && !filter_tokens.is_empty()
    {
        return UntapActionShape::All { filter_tokens };
    }
    if permission_shapes::exact_tokens(tokens, &["them"]) {
        UntapActionShape::Tagged {
            filter_tokens: None,
        }
    } else if let Some((_, filter_tokens)) =
        primitives::parse_prefix(tokens, primitives::kw("those").void())
        && !filter_tokens.is_empty()
    {
        UntapActionShape::Tagged {
            filter_tokens: Some(filter_tokens),
        }
    } else {
        UntapActionShape::Explicit {
            target_tokens: tokens,
        }
    }
}

pub(crate) fn parse_repeated_tagged_mana_payment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RepeatedTaggedManaPayment> {
    let repeats =
        primitives::find_prefix(tokens, || primitives::phrase(&["for", "each"])).is_some();
    let references_tagged_choice = primitives::find_prefix(tokens, || {
        alt((primitives::kw("those"), primitives::kw("them"))).void()
    })
    .is_some()
        || primitives::find_prefix(tokens, || primitives::phrase(&["chosen", "this", "way"]))
            .is_some();
    if !repeats || !references_tagged_choice {
        return None;
    }

    let mut stream = LexStream::new(tokens);
    let pip_groups = primitives::collect_mana_pip_groups
        .parse_next(&mut stream)
        .ok()?;
    Some(RepeatedTaggedManaPayment { pip_groups })
}

#[cfg(test)]
mod tests {
    use crate::runtime_backend::lexer::lex_line;

    use super::*;

    #[test]
    fn parses_untap_targets_and_repeated_tagged_mana() {
        let all = lex_line("each artifact", 0).unwrap();
        assert!(matches!(
            parse_untap_action_tokens(&all),
            UntapActionShape::All { .. }
        ));
        let conjoined = lex_line(
            "all nonland permanents you control and all nonland permanents that player controls",
            0,
        )
        .unwrap();
        let conjoined = parse_conjoined_untap_all_tokens(&conjoined)
            .expect("two quantified untap sets should parse");
        assert_eq!(
            crate::runtime_backend::token_word_refs(conjoined.left_filter_tokens),
            ["nonland", "permanents", "you", "control"]
        );
        assert_eq!(
            crate::runtime_backend::token_word_refs(conjoined.right_filter_tokens),
            ["nonland", "permanents", "that", "player", "controls"]
        );
        let tagged = lex_line("them", 0).unwrap();
        assert_eq!(
            parse_untap_action_tokens(&tagged),
            UntapActionShape::Tagged {
                filter_tokens: None
            }
        );

        let those_creatures = lex_line("those creatures", 0).unwrap();
        let UntapActionShape::Tagged {
            filter_tokens: Some(filter_tokens),
        } = parse_untap_action_tokens(&those_creatures)
        else {
            panic!("expected a typed tagged-set untap subject");
        };
        assert_eq!(
            filter_tokens
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .collect::<Vec<_>>(),
            ["creatures"]
        );

        let payment = lex_line("{w} for each of those chosen this way", 0).unwrap();
        assert_eq!(
            parse_repeated_tagged_mana_payment_tokens(&payment)
                .unwrap()
                .pip_groups
                .len(),
            1
        );
    }
}

use winnow::combinator::alt;
use winnow::prelude::*;

use crate::mana::ManaSymbol;
use crate::runtime_backend::lexer::{LexStream, OwnedLexToken};

use super::super::super::{permission_shapes, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UntapActionShape<'a> {
    All { filter_tokens: &'a [OwnedLexToken] },
    Tagged,
    Explicit { target_tokens: &'a [OwnedLexToken] },
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
        UntapActionShape::Tagged
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
        let tagged = lex_line("them", 0).unwrap();
        assert_eq!(parse_untap_action_tokens(&tagged), UntapActionShape::Tagged);

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

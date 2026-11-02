//! A complete top-card permission with a disjunction relative to this source's live colors.
use super::*;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<PermissionClauseSpec> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let rest = words.strip_prefix(&["you", "may", "play", "the", "top", "card", "of", "your", "library"])?;
    let source = rest.strip_prefix(&["if", "its", "colorless", "or", "it", "shares", "a", "color", "with"])
        .or_else(|| rest.strip_prefix(&["if", "it", "is", "colorless", "or", "it", "shares", "a", "color", "with"]))?;
    if !crate::util::is_source_reference_words(source) { return None; }
    let filter = ObjectFilter {
        owner: Some(PlayerFilter::You),
        any_of: vec![
            ObjectFilter::default().colorless(),
            ObjectFilter::default().sharing_characteristics_with(
                vec![ironsmith_core::ObjectCharacteristic::Color], ObjectFilter::source()),
        ],
        ..Default::default()
    };
    crate::word_claims::claim_tokens(tokens);
    Some(PermissionClauseSpec::GrantBySpec {
        player: PlayerAst::You,
        spec: crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::play_from(), filter, Zone::Library).with_top_card_only(),
        lifetime: PermissionLifetime::Static,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_disjunction_keeps_library_owner_top_and_live_source_relation() {
        for condition in ["it's", "it is"] {
            let text = format!("You may play the top card of your library if {condition} colorless or it shares a color with this permanent.");
            let tokens = crate::lexer::lex_line(&text, 0).unwrap();
            let Some(PermissionClauseSpec::GrantBySpec { spec, lifetime, .. }) = parse_permission_clause_spec_lexed(&tokens).unwrap() else { panic!("{text}"); };
            assert_eq!(lifetime, PermissionLifetime::Static);
            assert_eq!(spec.zone, Zone::Library);
            assert!(spec.top_card_only);
            assert_eq!(spec.filter.owner, Some(PlayerFilter::You));
            assert_eq!(spec.filter.any_of.len(), 2);
            assert_eq!(spec.filter.any_of[1].characteristic_relations.len(), 1);
        }
    }
    #[test]
    fn incomplete_or_different_permissions_are_not_consumed() {
        for text in [
            "You may play the top card of your library if it's colorless or it shares a color with another permanent",
            "You may play the top card of an opponent's library if it's colorless or it shares a color with this permanent",
            "You may play the top card of your library if it's colorless or it shares a color with this permanent and you gain 1 life",
            "You may play the top card of your library if it's colorless or it shares a color with",
        ] { assert!(parse(&crate::lexer::lex_line(text, 0).unwrap()).is_none(), "{text}"); }
    }
}

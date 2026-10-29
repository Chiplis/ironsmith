//! Custom named tokens supplied by the caller for one compilation.
//!
//! A custom card names a user-defined token ("create two Bobble tokens"); the
//! authoring layer rewrites that reference to a minimal token shape named
//! Bobble, and lowering substitutes the token's own compiled definition here.
//! The definitions are scoped to the closure, never global registry state.

use std::cell::RefCell;

use crate::cards::CardDefinition;

thread_local! {
    static CUSTOM_TOKENS: RefCell<Vec<CardDefinition>> = const { RefCell::new(Vec::new()) };
}

/// Run `f` with `tokens` resolvable by name from token-creation lowering.
/// Nested scopes see only the innermost token list.
pub fn with_custom_tokens<R>(tokens: Vec<CardDefinition>, f: impl FnOnce() -> R) -> R {
    struct Restore(Vec<CardDefinition>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = std::mem::take(&mut self.0);
            CUSTOM_TOKENS.with(|cell| *cell.borrow_mut() = previous);
        }
    }
    let _restore = Restore(CUSTOM_TOKENS.with(|cell| cell.replace(tokens)));
    f()
}

/// The custom token an authored token shape refers to: a non-predefined
/// shape that names its token explicitly ("... tokens named Bobble"). A
/// predefined token ("a Treasure token") or a subtype-derived name ("a 1/1
/// Goblin creature token") never resolves to a custom token.
pub(crate) fn custom_token_for(
    shape: &crate::model::token_definition::TokenDefinitionSpec,
    lowered: &CardDefinition,
) -> Option<CardDefinition> {
    use crate::model::token_definition::TokenDefinitionSpec;
    if matches!(shape, TokenDefinitionSpec::Builtin(_) | TokenDefinitionSpec::ModifiedBuiltin(_)) {
        return None;
    }
    let explicit = shape
        .text_roles()
        .is_some_and(|roles| roles.name == ironsmith_core::TokenNameTextRole::Explicit);
    if !explicit {
        return None;
    }
    custom_token_named(&lowered.card.name)
}

/// The custom token named `name`, if one is in scope (names compare like
/// card names: case-insensitively, ignoring surrounding space).
fn custom_token_named(name: &str) -> Option<CardDefinition> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    CUSTOM_TOKENS.with(|cell| {
        cell.borrow()
            .iter()
            .find(|token| token.card.name.trim().eq_ignore_ascii_case(name))
            .cloned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::CardDefinitionBuilder;
    use crate::ids::CardId;

    #[test]
    fn scope_restores_previous_tokens() {
        let token = CardDefinitionBuilder::new(CardId::new(), "Bobble").token().build();
        assert!(custom_token_named("bobble").is_none());
        with_custom_tokens(vec![token], || {
            assert!(custom_token_named(" BOBBLE ").is_some());
            with_custom_tokens(Vec::new(), || assert!(custom_token_named("Bobble").is_none()));
            assert!(custom_token_named("Bobble").is_some());
        });
        assert!(custom_token_named("Bobble").is_none());
    }
}

use super::*;
use crate::lexer::{TokenWordView, lex_line};

#[test]
fn source_exiled_spell_fact_is_typed_and_preserves_tail() {
    let tokens = lex_line(
        "a creature spell from among cards exiled with this enchantment this turn",
        0,
    )
    .unwrap();
    let parsed = parse_spell_from_source_exiled_tokens(&tokens).unwrap();
    assert_eq!(parsed.kind, SourceExiledSpellKind::Creature);
    assert_eq!(
        TokenWordView::new(parsed.tail_tokens).word_refs(),
        ["this", "turn"]
    );
    assert_eq!(
        parsed.reference.surface,
        ironsmith_core::SourceReferenceSurface::ThisPermanentType("this enchantment".to_string())
    );
}

#[test]
fn plural_source_exiled_spell_fact_preserves_filter_owner_and_source_surface() {
    let tokens = lex_line(
        "Dinosaur creature spells from among cards you own exiled with this creature this turn",
        0,
    )
    .unwrap();
    let parsed = parse_spells_from_source_exiled_tokens(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(parsed.subject_tokens).word_refs(),
        ["dinosaur", "creature", "spells"]
    );
    assert!(parsed.owned_by_you);
    assert_eq!(
        TokenWordView::new(parsed.tail_tokens).word_refs(),
        ["this", "turn"]
    );
    assert_eq!(
        parsed.reference.surface,
        ironsmith_core::SourceReferenceSurface::ThisPermanentType("this creature".to_string())
    );
}

#[test]
fn static_land_and_spell_pool_requires_the_complete_source_linked_surface() {
    let line = "You may play lands and cast spells from among cards exiled with this creature.";
    let tokens = lex_line(line, 0).unwrap();
    let parsed = parse_play_lands_and_spells_from_source_exiled_tokens(&tokens).unwrap();
    assert_eq!(parsed.surface, ironsmith_core::SourceReferenceSurface::ThisPermanentType("this creature".into()));
    for line in [
        "You may play lands and cast spells from among cards exiled with this creature this turn.",
        "You may play lands and cast creature spells from among cards exiled with this creature.",
        "You may play lands and cast spells from among cards exiled with this creature without paying their mana costs.",
        "You may play lands and cast spells from among cards exiled with that creature.",
    ] {
        assert!(parse_play_lands_and_spells_from_source_exiled_tokens(&lex_line(line, 0).unwrap()).is_none());
    }
}

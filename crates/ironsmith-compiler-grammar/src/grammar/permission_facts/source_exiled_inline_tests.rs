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

#[test]
fn private_inspection_and_play_share_one_complete_source_antecedent() {
    for article in ["", "the "] {
        let line = format!("You may look at {article}cards exiled with this creature, and you may play lands and cast spells from among those cards.");
        assert!(parse_look_and_play_source_exiled_tokens(&lex_line(&line, 0).unwrap()).is_some());
    }
    for line in [
        "You may look at cards exiled with this creature.",
        "You may look at cards exiled with this creature, and you may play lands and cast spells from among those cards this turn.",
        "You may look at cards exiled with this creature, and you may play lands and cast spells from among cards in your graveyard.",
        "Each player may look at cards exiled with this creature, and you may play lands and cast spells from among those cards.",
    ] {
        assert!(parse_look_and_play_source_exiled_tokens(&lex_line(line, 0).unwrap()).is_none());
    }
}

#[test]
fn standalone_inspector_requires_its_own_complete_sentence() {
    assert!(parse_look_source_exiled_tokens(&lex_line("You may look at cards exiled with this creature.", 0).unwrap()).is_some());
    for line in [
        "You may look at cards exiled with this creature this turn.",
        "You may look at cards exiled with that creature.",
        "You may look at cards exiled with this creature, and you may play lands and cast spells from among those cards.",
    ] { assert!(parse_look_source_exiled_tokens(&lex_line(line, 0).unwrap()).is_none()); }
}

#[test]
fn conditional_mana_rider_belongs_to_one_complete_source_pool_permission() {
    let line = "You may play lands and cast spells from among cards exiled with this creature. If you cast a spell this way, you may spend mana as though it were mana of any color to cast it.";
    let (_, mode) = parse_play_source_exiled_with_mana_tokens(&lex_line(line, 0).unwrap()).unwrap();
    assert_eq!(mode, ironsmith_core::value_model::ManaSpendMode::AnyColor);
    for line in [
        "You may play lands and cast spells from among cards exiled with this creature. Each player may spend mana as though it were mana of any color to cast spells.",
        "You may play lands and cast spells from among cards exiled with this creature. If you cast a spell this way, you may spend mana as though it were mana of any color to cast it. Draw a card.",
        "You may play lands and cast spells from among cards exiled with that creature. If you cast a spell this way, you may spend mana as though it were mana of any color to cast it.",
    ] { assert!(parse_play_source_exiled_with_mana_tokens(&lex_line(line, 0).unwrap()).is_none()); }
}

#[test]
fn inline_class_pool_mana_rider_requires_the_complete_shared_spell_reference() {
    for permission in ["You may play cards exiled with this Class", "You may play lands and cast spells from among cards exiled with this Class"] {
        let line = format!("{permission}, and you may spend mana as though it were mana of any color to cast those spells.");
        let tokens = lex_line(&line, 0).unwrap(); let (reference, mode) = parse_play_source_exiled_inline_mana_tokens(&tokens).unwrap();
        assert_eq!(reference.surface, ironsmith_core::SourceReferenceSurface::ThisPermanentType("this Class".into()));
        assert_eq!(mode, ironsmith_core::value_model::ManaSpendMode::AnyColor);
    }
    for line in [
        "You may play cards exiled with this Class, and you may spend mana as though it were mana of any color to cast it.",
        "You may play cards exiled with this Class this turn, and you may spend mana as though it were mana of any color to cast those spells.",
        "You may play cards exiled with this Class, and you may spend mana as though it were mana of any color to cast those spells. Draw a card.",
        "You may play creature cards exiled with this Class, and you may spend mana as though it were mana of any color to cast those spells.",
    ] { assert!(parse_play_source_exiled_inline_mana_tokens(&lex_line(line, 0).unwrap()).is_none()); }
}

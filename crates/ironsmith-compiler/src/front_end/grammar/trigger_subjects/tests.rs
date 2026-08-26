use super::super::super::lexer::{TokenWordView, lex_line};
use super::*;

#[test]
fn typed_trigger_boundaries_preserve_word_and_token_spans() {
    let tokens = lex_line("a noncreature card, this turn", 0).unwrap();
    let envelope = parse_discard_trigger_envelope(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(envelope.qualifier).word_refs(),
        ["a", "noncreature"]
    );
    assert_eq!(
        TokenWordView::new(envelope.trailing).word_refs(),
        ["this", "turn"]
    );
    assert_eq!(parse_trigger_word_token(&tokens, &["card"]), Some(2));
}

#[test]
fn typed_trigger_shapes_find_connectors_and_spell_activity() {
    assert_eq!(
        parse_source_or_another_shape(&["this", "creature", "or", "another", "artifact"]),
        Some(SourceOrAnotherShape {
            source_word_end: 2,
            connector_word: 2,
            connector_words: 1,
            other_word: 3,
            one_or_more: false,
        })
    );
    assert_eq!(
        parse_source_or_another_shape(&[
            "Satoru",
            "and/or",
            "one",
            "or",
            "more",
            "other",
            "creatures",
        ]),
        Some(SourceOrAnotherShape {
            source_word_end: 1,
            connector_word: 1,
            connector_words: 1,
            other_word: 5,
            one_or_more: true,
        })
    );
    assert_eq!(
        parse_source_or_another_shape(&["Satoru", "and/or", "one", "or", "more", "creatures",]),
        None
    );
    assert_eq!(
        parse_source_or_filter_shape(&["this", "or", "an", "instant", "or", "sorcery", "spell",]),
        Some(SourceOrFilterShape {
            source_word_end: 1,
            connector_word: 1,
            connector_words: 1,
            filter_word: 2,
        })
    );
    assert_eq!(
        parse_source_or_filter_shape(&[
            "this",
            "creature",
            "and",
            "or",
            "one",
            "or",
            "more",
            "other",
            "creatures",
        ]),
        Some(SourceOrFilterShape {
            source_word_end: 2,
            connector_word: 2,
            connector_words: 2,
            filter_word: 4,
        })
    );
    let tokens = lex_line("you cast or copy a spell", 0).unwrap();
    let facts = parse_spell_activity_verb_facts(&tokens);
    assert_eq!(facts.cast, Some(1));
    assert_eq!(facts.copy, Some(3));
}

#[test]
fn typed_player_reference_normalizes_possessives() {
    assert_eq!(
        parse_possessive_player_reference(&["enchanted", "creature's", "controller"]),
        PossessivePlayerReference::AttachedController(AttachedControllerSubject::Enchanted)
    );
    assert_eq!(
        parse_possessive_player_reference(&["each", "player's", "opponent"]),
        PossessivePlayerReference::Any
    );
    assert_eq!(
        parse_possessive_player_reference(&["your", "opponent"]),
        PossessivePlayerReference::You
    );
    assert_eq!(
        parse_possessive_player_reference(&["the", "chosen", "player's", "upkeep"]),
        PossessivePlayerReference::ChosenPlayer
    );
    assert_eq!(
        parse_possessive_player_reference(&["enchanted", "opponent's", "end", "step"]),
        PossessivePlayerReference::EnchantedPlayer
    );
}

#[test]
fn typed_spell_filter_envelope_stops_at_semantic_boundaries() {
    let tokens = lex_line("a blue spell from anywhere during your turn", 0).unwrap();
    let envelope = parse_spell_filter_envelope(&tokens);
    assert_eq!(
        TokenWordView::new(&tokens[..envelope.end]).word_refs(),
        ["a", "blue", "spell"]
    );
}

#[test]
fn typed_spell_filter_envelope_preserves_serial_type_and_subtype_lists() {
    let mixed_union = lex_line("an instant, sorcery, or Wizard spell during your turn", 0).unwrap();
    let envelope = parse_spell_filter_envelope(&mixed_union);
    assert_eq!(
        TokenWordView::new(&mixed_union[..envelope.end]).word_refs(),
        ["an", "instant", "sorcery", "or", "wizard", "spell"]
    );

    let subtype_union = lex_line("a Pegasus, Unicorn, or Horse creature spell", 0).unwrap();
    let envelope = parse_spell_filter_envelope(&subtype_union);
    assert_eq!(
        TokenWordView::new(&subtype_union[..envelope.end]).word_refs(),
        [
            "a", "pegasus", "unicorn", "or", "horse", "creature", "spell"
        ]
    );

    let simple = lex_line("a creature spell, during combat", 0).unwrap();
    let envelope = parse_spell_filter_envelope(&simple);
    assert_eq!(
        TokenWordView::new(&simple[..envelope.end]).word_refs(),
        ["a", "creature", "spell"]
    );
}

#[test]
fn typed_controller_suffix_preserves_longest_tail_and_subject_boundary() {
    let suffix =
        parse_trigger_control_suffix(&["creatures", "an", "opponent", "controls"]).unwrap();
    assert_eq!(suffix.controller, TriggerControllerReference::Opponent);
    assert_eq!(suffix.subject_end, 1);

    let phrase =
        parse_trigger_control_phrase(&["creatures", "you", "control", "with", "flying"]).unwrap();
    assert_eq!(phrase.controller, TriggerControllerReference::You);
    assert_eq!((phrase.start, phrase.words), (1, 2));
}

#[test]
fn typed_spell_or_ability_controller_tail_preserves_reference_kind() {
    assert_eq!(
        parse_spell_or_ability_controller_tail(&[
            "a", "spell", "or", "ability", "an", "opponent", "controls",
        ]),
        Some(TriggerControllerReference::Opponent)
    );
    assert_eq!(
        parse_spell_or_ability_controller_tail(&[
            "spell", "or", "ability", "player", "who", "cast", "it", "controls",
        ]),
        Some(TriggerControllerReference::EffectController)
    );
    assert_eq!(
        parse_spell_controller_tail(&["a", "spell", "you", "control"]),
        Some(TriggerControllerReference::You)
    );
    assert_eq!(
        parse_spell_controller_tail(&["spell", "an", "opponent", "controls"]),
        Some(TriggerControllerReference::Opponent)
    );
}

#[test]
fn typed_damage_source_surface_distinguishes_generic_sources() {
    let source = lex_line("a source you control", 0).unwrap();
    assert_eq!(
        parse_damage_source_surface(&source),
        crate::triggers::DamageSourceSurface::Source
    );

    let qualified_source = lex_line("a red source you control", 0).unwrap();
    assert_eq!(
        parse_damage_source_surface(&qualified_source),
        crate::triggers::DamageSourceSurface::Source
    );

    let creature = lex_line("a creature you control", 0).unwrap();
    assert_eq!(
        parse_damage_source_surface(&creature),
        crate::triggers::DamageSourceSurface::Filter
    );
}

#[test]
fn bare_plural_players_is_an_any_player_trigger_subject() {
    assert_eq!(
        parse_trigger_subject_surface_facts(&["players"]).player,
        Some(TriggerControllerReference::AnyPlayer)
    );
}

#[test]
fn typed_copy_and_token_lifecycle_sentences_preserve_shapes() {
    assert_eq!(
        parse_trigger_source_subject_words(&["a", "source"]),
        Some(TriggerSourceSubject::AnySource)
    );

    let reduction = lex_line("That copy costs {2} less to cast.", 0).unwrap();
    let shape = parse_copy_reference_cost_reduction_shape_tokens(&reduction).unwrap();
    assert_eq!(reduction[shape.reduction_tokens].len(), 1);

    let copy = lex_line("Copy that card.", 0).unwrap();
    assert_eq!(
        parse_simple_copy_reference_tokens(&copy),
        Some(SimpleCopyReferenceKind::ThatCard)
    );

    let exile = lex_line("Exile that token when this leaves the battlefield.", 0).unwrap();
    assert_eq!(
        parse_token_lifecycle_sentence_tokens(&exile),
        Some(TokenLifecycleSentenceKind::ExileCreatedTokenWhenSourceLeaves)
    );
    let sacrifice = lex_line("Sacrifice this when that token leaves the battlefield.", 0).unwrap();
    assert_eq!(
        parse_token_lifecycle_sentence_tokens(&sacrifice),
        Some(TokenLifecycleSentenceKind::SacrificeSourceWhenCreatedTokenLeaves)
    );
}

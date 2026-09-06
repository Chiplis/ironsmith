use super::*;
#[cfg(test)]
use ironsmith_compiler::ParseCardText;

#[test]
fn delayed_copy_of_prior_exiled_card_keeps_cast_inside_trigger() {
    let lexed = crate::lexer::lex_line(
            "Exile target instant or sorcery card from your graveyard. Creatures you control get +X/+0 until end of turn, where X is that card's mana value. Whenever a creature you control deals combat damage to a player this turn, copy the exiled card. You may cast the copy without paying its mana cost.",
            0,
        )
        .expect("Surge to Victory text should lex");
    let parsed = parse_effect_sentences_lexed(&lexed).expect("Surge to Victory text should parse");

    assert_eq!(
        parsed.len(),
        3,
        "cast follow-up escaped delayed trigger: {parsed:#?}"
    );
    let EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn { effects, .. }) = &parsed[2] else {
        panic!("expected delayed combat-damage trigger: {parsed:#?}");
    };
    assert!(
        format!("{effects:#?}").contains("CastTagged"),
        "copy cast should remain inside delayed trigger: {parsed:#?}"
    );

    let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Surge Shape")
            .card_types(vec![crate::CardType::Sorcery])
            .parse_text(
                "Exile target instant or sorcery card from your graveyard. Creatures you control get +X/+0 until end of turn, where X is that card's mana value. Whenever a creature you control deals combat damage to a player this turn, copy the exiled card. You may cast the copy without paying its mana cost.",
            )
            .expect("Surge to Victory shape should compile");
    let debug = format!("{definition:#?}");
    let cast = debug
        .split_once("CastTaggedEffect")
        .map(|(_, tail)| &tail[..tail.len().min(500)])
        .expect("delayed trigger should contain a tagged cast");
    assert!(
        cast.contains(crate::tag::CompilerReferenceTag::PriorExiledCard.as_str()),
        "{debug}"
    );
    assert!(!cast.contains("triggering"), "{debug}");
    let mana_value = debug
        .split_once("ManaValueOf")
        .map(|(_, tail)| &tail[..tail.len().min(500)])
        .expect("pump should contain a mana-value reference");
    assert!(
        mana_value.contains(crate::tag::CompilerReferenceTag::PriorExiledCard.as_str()),
        "pump should use the exiled card's mana value: {debug}"
    );
}

#[test]
fn immediate_exiled_card_cast_keeps_its_may_scope() {
    let lexed = crate::lexer::lex_line("You may cast the exiled card.", 0)
        .expect("optional tagged cast should lex");
    let parsed = parse_effect_sentences_lexed(&lexed).expect("optional tagged cast should parse");

    assert!(
        matches!(
            parsed.as_slice(),
            [EffectAst::Permissions(PermissionEffectAst::May { effects })]
                if matches!(
                    effects.as_slice(),
                    [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::Stack(StackActionAst::CastTagged { .. }),
                        ..
                    })]
                )
        ),
        "expected immediate cast inside a may scope, got {parsed:#?}"
    );
}

#[test]
fn cross_ability_exiled_card_copy_uses_source_link() {
    let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Imprint Copy")
            .card_types(vec![crate::CardType::Artifact])
            .subtypes(vec![crate::Subtype::Equipment])
            .parse_text(
                "Imprint — When this Equipment enters, you may exile an instant card from your hand.\n\
                 Whenever equipped creature deals combat damage to a player, you may copy the exiled card. If you do, you may cast the copy without paying its mana cost.\n\
                 Equip {4}",
            )
            .expect("a linked Imprint copy ability should compile");
    let debug = format!("{definition:#?}");

    assert!(debug.contains("ImprintFromHandEffect"), "{debug}");
    assert!(
        debug.contains(crate::tag::CompilerReferenceTag::SourceExiled.as_str()),
        "{debug}"
    );
    assert!(
        !debug.contains("CopySpellEffect"),
        "an exiled card is not a stack spell and must be selected before the copy-cast: {debug}"
    );
    assert!(debug.contains("CastTaggedEffect"), "{debug}");
    let cast_debug = debug
        .split_once("CastTaggedEffect")
        .map(|(_, tail)| tail)
        .expect("combat-damage trigger should contain a tagged cast");
    assert!(
        cast_debug.contains(crate::tag::CompilerReferenceTag::It.as_str()),
        "{debug}"
    );
    assert!(cast_debug.contains("as_copy: true"), "{debug}");
    assert!(
        cast_debug.contains("without_paying_mana_cost: true"),
        "{debug}"
    );
}

#[test]
fn copy_card_then_may_cast_copy_uses_prior_moved_tag_without_copying_source() {
    let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Copy Variant")
            .card_types(vec![crate::CardType::Sorcery])
            .parse_text(
                "Exile target instant or sorcery card from your graveyard. Copy that card. You may cast the copy without paying its mana cost.",
            )
            .expect("copy-and-cast sequence should compile");
    let debug = format!("{definition:#?}");

    assert!(!debug.contains("CopySpellEffect"), "{debug}");
    let cast_debug = debug
        .split_once("CastTaggedEffect")
        .map(|(_, tail)| tail)
        .expect("sequence should lower to a tagged cast");
    assert!(cast_debug.contains("__sentence_helper_exiled"), "{debug}");
    assert!(cast_debug.contains("as_copy: true"), "{debug}");
    assert!(
        cast_debug.contains("without_paying_mana_cost: true"),
        "{debug}"
    );
}

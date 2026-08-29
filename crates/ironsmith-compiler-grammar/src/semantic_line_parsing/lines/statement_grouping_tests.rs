use super::*;

#[cfg(test)]
#[test]
fn typed_statement_replacement_surface_stays_grouped() {
    let tokens = crate::lexer::lex_line(
        "Clash with an opponent, then return target creature to its owner's hand. If you win, you may put that creature on top of its owner's library instead.",
        0,
    )
    .expect("lex Whirlpool Whelm");

    assert!(linked_statement_should_stay_grouped(&tokens));
}

#[cfg(test)]
#[test]
fn trailing_conditional_self_replacement_stays_grouped() {
    let tokens = crate::lexer::lex_line(
        "Target creature an opponent controls gets -1/-1 until end of turn. That creature gets -4/-4 instead if you control a creature named Bogbrew Witch.",
        0,
    )
    .expect("lex conditional pump replacement");

    assert!(linked_statement_should_stay_grouped(&tokens));
}

#[cfg(test)]
#[test]
fn self_replacement_with_a_common_resolution_tail_stays_grouped() {
    let text = "Choose target creature with mana value 3 or less. If this spell was kicked, instead choose target creature. Exile the chosen creature, then its controller gains life equal to its mana value.";
    let tokens = crate::lexer::lex_line(text, 0).expect("lex choice replacement with common tail");
    assert!(linked_statement_should_stay_grouped(&tokens));
    let groups = split_lexed_sentences(&tokens)
        .into_iter()
        .map(|group| group.to_vec())
        .collect::<Vec<_>>();
    let parsed = parse_statement_token_groups_to_chunks(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: text.to_string(),
            source_tokens: tokens.clone(),
            normalized: NormalizedLine::identity(text),
            semantic_facts: Default::default(),
        },
        &tokens,
        &groups,
    )
    .expect("public statement lowering should preserve the typed program");
    let [LineAst::Statement { effects }] = parsed.as_slice() else {
        panic!("expected one statement program: {parsed:#?}");
    };
    assert!(
        matches!(effects.as_slice(), [EffectAst::SelfReplacement { .. }]),
        "{effects:#?}"
    );
    assert!(format!("{effects:#?}").contains("GainLife"), "{effects:#?}");

    let parsed_without_precomputed_groups = parse_statement_token_groups_to_chunks(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: text.to_string(),
            source_tokens: tokens.clone(),
            normalized: NormalizedLine::identity(text),
            semantic_facts: Default::default(),
        },
        &tokens,
        &[],
    )
    .expect("ungrouped public statement lowering should preserve the typed program");
    let [LineAst::Statement { effects }] = parsed_without_precomputed_groups.as_slice() else {
        panic!("expected one ungrouped statement program: {parsed_without_precomputed_groups:#?}");
    };
    assert!(
        matches!(effects.as_slice(), [EffectAst::SelfReplacement { .. }])
            && format!("{effects:#?}").contains("GainLife"),
        "{effects:#?}"
    );

    let unrelated = crate::lexer::lex_line(
        "Choose target creature with mana value 3 or less. If this spell was kicked, choose target creature. Exile the chosen creature.",
        0,
    )
    .expect("lex nonreplacement near miss");
    assert!(!linked_statement_should_stay_grouped(&unrelated));
}

#[cfg(test)]
fn parse_public_statement_groups_for_test(text: &str) -> Vec<LineAst> {
    let tokens = crate::lexer::lex_line(text, 0).expect("statement should lex");
    let groups = split_lexed_sentences(&tokens)
        .into_iter()
        .map(|group| group.to_vec())
        .collect::<Vec<_>>();
    parse_statement_token_groups_to_chunks(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: text.to_string(),
            source_tokens: tokens.clone(),
            normalized: NormalizedLine::identity(text),
            semantic_facts: Default::default(),
        },
        &tokens,
        &groups,
    )
    .expect("public statement route should parse")
}

#[cfg(test)]
#[test]
fn destroy_no_regeneration_pair_preempts_statement_group_splitting() {
    let text = "Destroy target creature that isn't enchanted. It can't be regenerated.";
    let parsed = parse_public_statement_groups_for_test(text);
    let [LineAst::Statement { effects }] = parsed.as_slice() else {
        panic!("expected one linked statement: {parsed:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Destroy {
                    target: TargetAst::Object(filter, _, _),
                    no_regeneration: true,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one typed no-regeneration destroy: {effects:#?}");
    };
    let aura = filter
        .without_attached_object
        .as_deref()
        .expect("negative enchanted state should survive statement grouping");
    assert_eq!(aura.subtypes, [crate::types::Subtype::Aura]);
}

#[cfg(test)]
#[test]
fn hidden_partition_permission_preempts_statement_group_splitting() {
    let text = "Look at the top three cards of your library. Exile one face down and put the rest on the bottom of your library in any order. For as long as it remains exiled, you may cast it if it's a creature spell.";
    let parsed = parse_public_statement_groups_for_test(text);
    let [LineAst::Statement { effects }] = parsed.as_slice() else {
        panic!("expected one linked statement: {parsed:#?}");
    };
    let debug = format!("{effects:#?}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("face_down: true"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
    assert!(
        debug.contains("GrantPlayTaggedForAsLongAsExiled"),
        "{debug}"
    );
    assert!(debug.contains("Creature"), "{debug}");
}

#[cfg(test)]
#[test]
fn historical_target_return_preempts_statement_group_splitting() {
    let text = "Choose up to three target permanent cards in graveyards that were put there from the battlefield this turn. Return them to the battlefield tapped under their owners' control. You draw a card for each opponent who controls one or more of those permanents.";
    let parsed = parse_public_statement_groups_for_test(text);
    let [LineAst::Statement { effects }] = parsed.as_slice() else {
        panic!("expected one linked statement: {parsed:#?}");
    };
    let debug = format!("{effects:#?}");
    assert!(
        debug.contains("entered_graveyard_from_battlefield_this_turn: true"),
        "{debug}"
    );
    assert!(debug.contains("ReturnToBattlefield"), "{debug}");
    assert!(debug.contains("PlayerControls"), "{debug}");
}

#[cfg(test)]
#[test]
fn failed_comma_then_sequence_keeps_its_result_branch_in_the_next_source_group() {
    let text = "Mill three cards, then return a land card or Elf card from your graveyard to your hand. If you can't, draw a card.";
    let parsed = parse_public_statement_groups_for_test(text);
    let [LineAst::Statement { effects }] = parsed.as_slice() else {
        panic!("expected one linked statement: {parsed:#?}");
    };
    let [
        EffectAst::SourceSentence { effects: first, .. },
        EffectAst::SourceSentence {
            effects: fallback, ..
        },
    ] = effects.as_slice()
    else {
        panic!("expected two authored source groups: {effects:#?}");
    };
    assert!(matches!(first.as_slice(), [EffectAst::CommaThen { .. }]));
    assert!(matches!(
        fallback.as_slice(),
        [EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::DidNot,
            ..
        }]
    ));
}

#[cfg(test)]
#[test]
fn quoted_token_copy_replacement_stays_grouped_with_its_granted_ability() {
    let text = "Create a token that's a copy of target permanent. If {R}{G} was spent to cast this spell, instead create a token that's a copy of that permanent, except the token has \"When this token enters, if it's a creature, it fights up to one target creature you don't control.\"";
    let tokens = crate::lexer::lex_line(text, 0).expect("lex quoted token-copy replacement");
    assert!(linked_statement_should_stay_grouped(&tokens));
    let mut direct = parse_effect_sentences_lexed(&tokens).expect("parse direct replacement");
    assert!(
        crate::effect_sentences::attach_inline_token_granted_abilities_to_last_create(
            &mut direct,
            &tokens,
        ),
        "direct attachment failed: {direct:#?}"
    );
    let effects = parse_effect_sentences_preserving_source_boundaries(&tokens)
        .expect("parse grouped token-copy replacement");
    let [EffectAst::SelfReplacement { if_true, .. }] = effects.as_slice() else {
        panic!("expected one self-replacement: {effects:#?}");
    };
    let replacement_members = if_true
        .iter()
        .flat_map(|effect| match effect {
            EffectAst::Coordination(coordination) => coordination
                .members
                .iter()
                .flat_map(|member| member.effects.iter())
                .collect::<Vec<_>>(),
            effect => vec![effect],
        })
        .collect::<Vec<_>>();
    let granted_abilities = replacement_members
        .iter()
        .find_map(|effect| match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopyFromSource {
                        granted_abilities, ..
                    },
                ..
            }) => Some(granted_abilities),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected a source-relative token copy: {if_true:#?}"));
    assert_eq!(granted_abilities.len(), 1, "{if_true:#?}");
}

#[cfg(test)]
#[test]
fn revealed_hand_union_count_stays_linked_through_the_public_statement_route() {
    let text =
        "Target opponent reveals their hand. You draw a card for each Forest and green card in it.";
    let tokens = crate::lexer::lex_line(text, 0).expect("revealed-hand union statement should lex");
    let groups = split_lexed_sentences(&tokens)
        .into_iter()
        .map(|group| group.to_vec())
        .collect::<Vec<_>>();
    let parsed = parse_statement_token_groups_to_chunks(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: text.to_string(),
            source_tokens: tokens.clone(),
            normalized: NormalizedLine::identity(text),
            semantic_facts: Default::default(),
        },
        &tokens,
        &groups,
    )
    .expect("public statement route should preserve the revealed-hand pair");
    let [LineAst::Statement { effects }] = parsed.as_slice() else {
        panic!("expected one statement program: {parsed:#?}");
    };
    let [
        _,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { count },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected reveal plus typed draw: {effects:#?}");
    };
    let Value::Count(filter) = count.unhinted() else {
        panic!("expected a revealed-hand object count: {count:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(
        filter.owner,
        Some(PlayerFilter::AliasedTarget(Box::new(
            PlayerFilter::Opponent
        )))
    );
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
}

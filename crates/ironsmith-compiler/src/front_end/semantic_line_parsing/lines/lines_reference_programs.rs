use super::*;

pub(super) fn membership_predicate_for_iterated_object(tag: &str) -> PredicateAst {
    PredicateAst::TaggedMatches(
        TagKey::from(tag),
        ObjectFilter::default().same_stable_id_as_tagged(TagKey::from(IT_TAG)),
    )
}

/// Preserve a targeted graveyard card and the optional normal-cost cast in a
/// single typed trigger body. The broad grant-ability sentence parser can
/// otherwise read `you may cast target card ...` as an ability granted to the
/// triggering spell, losing both targeting and execution.
pub fn exact_target_same_name_graveyard_may_cast_bundle(
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let words = crate::lexer::parser_token_word_refs(effect_parse_tokens);
    const BODY: &[&str] = &[
        "you",
        "may",
        "cast",
        "target",
        "card",
        "with",
        "the",
        "same",
        "name",
        "as",
        "that",
        "spell",
        "from",
        "your",
        "graveyard",
    ];
    if !crate::word_primitives::parse_sequence_prefix(&words, BODY)
        || !(words.len() == BODY.len()
            || crate::word_primitives::parse_sequence_complete(
                &words[BODY.len()..],
                &["you", "still", "pay", "its", "costs"],
            ))
    {
        return None;
    }

    let target_tag =
        crate::util::helper_tag_for_tokens(effect_parse_tokens, "targeted_same_name_spell");
    let mut filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    filter
        .tagged_constraints
        .push(crate::target::TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::Triggering.key(),
            relation: crate::target::TaggedOpbjectRelation::SameNameAsTagged,
        });
    filter.set_same_name_antecedent_surface(Some(ironsmith_core::SameNameAntecedentSurface::Spell));
    let target = TargetAst::Object(filter, Some(TextSpan::synthetic()), None);
    Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_explicit_target_only(target)),
            tag: target_tag.clone(),
        },
        EffectAst::May {
            effects: vec![EffectAst::subject_verb_cast_tagged(
                target_tag,
                PlayerAst::You,
                false,
                false,
                false,
                None,
            )],
        },
    ])
}

/// Preserve the immediate targeted graveyard cast permission used by combat-
/// damage triggers such as `you may cast target ... from that player's
/// graveyard, and mana of any type can be spent to cast that spell`.
///
/// The ordinary permission grammar intentionally models durable play grants.
/// This wording is instead a one-shot resolution instruction: declare one
/// target, then optionally cast that exact tagged card with the stated mana
/// spending mode. Keeping the two operations typed also lets trigger-reference
/// resolution bind `that player` to the damaged player.
pub fn exact_target_graveyard_any_type_may_cast_bundle(
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    const PREFIX: &[&str] = &["you", "may", "cast", "target"];
    const SUFFIX: &[&str] = &[
        "and", "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "that", "spell",
    ];

    let words = crate::lexer::parser_token_word_refs(effect_parse_tokens);
    if words.len() <= PREFIX.len() + SUFFIX.len()
        || !crate::word_primitives::parse_sequence_prefix(&words, PREFIX)
        || !crate::word_primitives::parse_sequence_suffix(&words, SUFFIX)
    {
        return None;
    }

    let view = crate::lexer::TokenWordView::new(effect_parse_tokens);
    let target_start = view.token_index_after_words(PREFIX.len() - 1)?;
    let suffix_start_word = words.len() - SUFFIX.len();
    let target_end = view.map_word_to_token_start(suffix_start_word)?;
    let target_tokens = crate::util::trim_commas(&effect_parse_tokens[target_start..target_end]);
    let target = crate::util::parse_target_phrase(&target_tokens).ok()?;
    let TargetAst::Object(filter, ..) = &target else {
        return None;
    };
    if filter.zone != Some(Zone::Graveyard)
        || filter.owner != Some(PlayerFilter::IteratedPlayer)
        || !filter.excluded_card_types.contains(&CardType::Land)
    {
        return None;
    }

    let target_tag =
        crate::util::helper_tag_for_tokens(effect_parse_tokens, "targeted_graveyard_any_type_cast");
    Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_explicit_target_only(target)),
            tag: target_tag.clone(),
        },
        EffectAst::May {
            effects: vec![
                EffectAst::subject_verb_cast_tagged_with_additional_cost_and_mana_spend_mode(
                    target_tag,
                    PlayerAst::You,
                    false,
                    false,
                    false,
                    None,
                    None,
                    ironsmith_core::value_model::ManaSpendMode::AnyType,
                ),
            ],
        },
    ])
}

#[cfg(test)]
#[test]
pub(super) fn targeted_relative_graveyard_cast_keeps_target_player_and_any_type_mana() {
    let tokens = lex_line(
        "You may cast target nonland permanent card from that player's graveyard, and mana of any type can be spent to cast that spell.",
        0,
    )
    .expect("targeted graveyard cast should lex");
    let effects = exact_target_graveyard_any_type_may_cast_bundle(&tokens)
        .expect("the typed immediate graveyard cast bundle should match");
    let [
        EffectAst::TagAffected {
            effect: target_effect,
            tag: target_tag,
        },
        EffectAst::May {
            effects: may_effects,
        },
    ] = effects.as_slice()
    else {
        panic!("expected explicit target followed by an optional cast: {effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::TargetOnly {
                target: TargetAst::Object(filter, ..),
                explicit_declaration: true,
            },
        ..
    }) = target_effect.as_ref()
    else {
        panic!("expected an explicit graveyard object target: {target_effect:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
    assert!(filter.excluded_card_types.contains(&CardType::Land));
    assert!(matches!(
        may_effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CastTagged {
                tag,
                mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::AnyType,
                ..
            },
            ..
        })] if tag == target_tag
    ));

    let changed = lex_line(
        "You may cast target nonland permanent card from that player's graveyard.",
        0,
    )
    .expect("changed mana clause should lex");
    assert!(exact_target_graveyard_any_type_may_cast_bundle(&changed).is_none());
}

#[cfg(test)]
#[test]
pub(super) fn targeted_same_name_graveyard_cast_keeps_target_and_optional_normal_payment() {
    let effects = "you may cast target card with the same name as that spell from your graveyard.";
    let full = format!("Whenever you cast an instant or sorcery spell from your hand, {effects}");
    let parsed = parse_triggered_text_for_test(
        &full,
        "you cast an instant or sorcery spell from your hand",
        effects,
    )
    .expect("the targeted same-name graveyard cast should reach the public trigger route");
    let parsed_effects = semantic_effects_for_test(&parsed)
        .unwrap_or_else(|| panic!("expected one triggered line: {parsed:#?}"));
    let [
        EffectAst::TagAffected {
            effect: target_effect,
            tag: target_tag,
        },
        EffectAst::May {
            effects: may_effects,
        },
    ] = parsed_effects
    else {
        panic!("expected targeted card followed by optional cast: {parsed_effects:#?}");
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::TargetOnly {
                target: TargetAst::Object(filter, _, _),
                explicit_declaration: true,
            },
        ..
    }) = target_effect.as_ref()
    else {
        panic!("expected an explicit object target: {target_effect:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert!(matches!(
        filter.tagged_constraints.as_slice(),
        [crate::target::TaggedObjectConstraint {
            tag,
            relation: crate::target::TaggedOpbjectRelation::SameNameAsTagged,
        }] if tag.as_str() == "triggering"
    ));
    assert!(matches!(
        may_effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CastTagged {
                tag,
                player: PlayerAst::You,
                allow_land: false,
                as_copy: false,
                without_paying_mana_cost: false,
                additional_mana_cost: None,
                cost_reduction: None,
                mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::Normal,
                ..
            },
            ..
        })] if tag == target_tag
    ));

    let free_cast = lex_line(
        "you may cast target card with the same name as that spell from your graveyard without paying its mana cost.",
        0,
    )
    .expect("free-cast near miss should lex");
    assert!(exact_target_same_name_graveyard_may_cast_bundle(&free_cast).is_none());
}

#[cfg(test)]
#[test]
pub(super) fn atomic_return_as_aura_bundle_preempts_returned_object_static_split() {
    let effects = "return it to the battlefield. It's an Aura enchantment with enchant creature you control and \"{G}{W}: Enchanted creature gains indestructible until end of turn,\" and it loses all other abilities.";
    let tokens = lex_line(effects, 0).expect("atomic Aura return should lex");
    let bundled = exact_atomic_return_as_aura_bundle(&tokens)
        .expect("typed Aura return should remain one atomic resolution bundle");
    assert!(matches!(
        bundled.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ReturnToBattlefield {
                as_aura: Some(as_aura),
                ..
            },
            ..
        })] if as_aura.remove_all_abilities && as_aura.granted_abilities.len() == 1
    ));

    let near_miss = lex_line(
        "return it to the battlefield. It's an Aura enchantment with enchant creature you control.",
        0,
    )
    .expect("plain Aura return should lex");
    assert!(
        exact_atomic_return_as_aura_bundle(&near_miss).is_none(),
        "an Aura return without both the quoted grant and ability loss must stay on its ordinary route"
    );
}

#[test]
pub(super) fn source_sentence_boundaries_preserve_jointly_parsed_reference_flow() {
    let independent = lex_line(
        "Put a +1/+1 counter on this creature. Each opponent loses 1 life.",
        0,
    )
    .expect("Aatchik-style effects should lex");
    let independent = parse_effect_sentences_preserving_source_boundaries(&independent)
        .expect("Aatchik-style effects should parse");
    assert_eq!(independent.len(), 2, "{independent:#?}");
    assert!(
        independent
            .iter()
            .all(|effect| matches!(effect, EffectAst::SourceSentence { .. })),
        "independent direct sentences should retain their authored boundary: {independent:#?}"
    );
    assert!(
        independent.iter().all(|effect| matches!(
            effect,
            EffectAst::SourceSentence {
                leading_then: false,
                ..
            }
        )),
        "ordinary sentence boundaries must not acquire ordering provenance: {independent:#?}"
    );

    let explicit_then = lex_line(
        "Draw two cards. Then discard a card unless you attacked this turn.",
        0,
    )
    .expect("explicit-then effects should lex");
    let explicit_then = parse_effect_sentences_preserving_source_boundaries(&explicit_then)
        .expect("explicit-then effects should parse");
    let [
        EffectAst::SourceSentence {
            leading_then: false,
            ..
        },
        EffectAst::SourceSentence {
            leading_then: true, ..
        },
    ] = explicit_then.as_slice()
    else {
        panic!("leading Then should be preserved on only the second sentence: {explicit_then:#?}");
    };

    let ordered = lex_line(
        "Starting with you, each player chooses up to five permanents they control. All permanents other than this creature that weren't chosen this way phase out.",
        0,
    )
    .expect("Disciple-style ordered choices should lex");
    let ordered = parse_effect_sentences_preserving_source_boundaries(&ordered)
        .expect("Disciple-style ordered choices should parse");
    let [
        EffectAst::SourceSentence {
            starting_with_controller: true,
            ..
        },
        EffectAst::SourceSentence {
            starting_with_controller: false,
            ..
        },
    ] = ordered.as_slice()
    else {
        panic!("the explicit participant ordering must remain on the first sentence: {ordered:#?}");
    };
    let ordered_single = lex_line(
        "Starting with you, each player chooses up to five permanents they control.",
        0,
    )
    .expect("single-sentence ordered choices should lex");
    let ordered_single = parse_effect_sentences_preserving_source_boundaries(&ordered_single)
        .expect("single-sentence ordered choices should parse");
    assert!(matches!(
        ordered_single.as_slice(),
        [EffectAst::SourceSentence {
            starting_with_controller: true,
            ..
        }]
    ));
    let unordered_single = lex_line("Each player chooses up to five permanents they control.", 0)
        .expect("unordered participant choice should lex");
    let unordered_single = parse_effect_sentences_preserving_source_boundaries(&unordered_single)
        .expect("unordered participant choice should parse");
    assert!(
        !unordered_single.iter().any(|effect| matches!(
            effect,
            EffectAst::SourceSentence {
                starting_with_controller: true,
                ..
            }
        )),
        "an ordinary player loop must not acquire explicit participant ordering: \
         {unordered_single:#?}"
    );

    let full_trigger = lex_line(
        "When this creature enters, starting with you, each player chooses up to five permanents they control. All permanents other than this creature that weren't chosen this way phase out.",
        0,
    )
    .expect("Disciple-style trigger should lex");
    let trigger_effects = lex_line(
        "Each player chooses up to five permanents they control. All permanents other than this creature that weren't chosen this way phase out.",
        0,
    )
    .expect("Disciple-style trigger effects should lex");
    let trigger_clause = lex_line("This creature enters, starting with you", 0)
        .expect("Disciple-style trigger clause should lex");
    let surfaced_trigger = parse_triggered_line(
        test_line_info(
            "When this creature enters, starting with you, each player chooses up to five permanents they control. All permanents other than this creature that weren't chosen this way phase out.",
        ),
        "when this creature enters, starting with you, each player chooses up to five permanents they control. all permanents other than this creature that weren't chosen this way phase out.",
        &full_trigger,
        &trigger_clause,
        &trigger_effects,
        None,
        None,
        None,
        None,
    )
    .expect("Disciple-style trigger should parse through the semantic line path");
    let surfaced_effects = match &surfaced_trigger {
        LineAst::Triggered { effects, .. } => effects.as_slice(),
        LineAst::Ability(parsed) => parsed
            .effects_ast
            .as_deref()
            .expect("the parsed trigger must retain its semantic effects"),
        _ => panic!("Disciple-style line must remain a trigger: {surfaced_trigger:#?}"),
    };
    assert!(
        matches!(
            surfaced_effects,
            [
                EffectAst::SourceSentence {
                    starting_with_controller: true,
                    ..
                },
                EffectAst::SourceSentence {
                    starting_with_controller: false,
                    ..
                }
            ]
        ),
        "the trigger split must not swallow participant ordering: {surfaced_effects:#?}"
    );

    let linked = "Reveal the top card of your library and put that card into your hand. You lose life equal to its mana value.";
    let tokens = lex_line(linked, 0).expect("linked trigger effects should lex");
    let effects = parse_effect_sentences_preserving_source_boundaries(&tokens)
        .expect("linked trigger effects should keep their joint parse");
    assert_eq!(effects.len(), 2, "{effects:#?}");
    assert!(
        effects
            .iter()
            .all(|effect| matches!(effect, EffectAst::SourceSentence { .. })),
        "joint parsing should retain a stable boundary without losing reference flow: {effects:#?}"
    );
}

#[test]
pub(super) fn tagged_characteristic_addition_is_a_bound_effect_followup() {
    let tokens = lex_line(
        "Put target artifact onto the battlefield. That permanent is an enchantment in addition to its other types.",
        0,
    )
    .expect("bound characteristic fixture should lex");
    let sentences = split_lexed_sentences(&tokens);
    assert!(sentences_have_bound_characteristic_followup_after_first(
        &sentences
    ));

    let tokens = lex_line(
        "Draw a card. Creatures you control are artifacts in addition to their other types.",
        0,
    )
    .expect("independent static fixture should lex");
    let sentences = split_lexed_sentences(&tokens);
    assert!(!sentences_have_bound_characteristic_followup_after_first(
        &sentences
    ));
}

pub fn normalize_exert_followup_source_reference_tokens(
    source_ref: &str,
    followup_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    semantic_grammar::normalize_exert_followup_source_tokens(source_ref, followup_tokens)
}

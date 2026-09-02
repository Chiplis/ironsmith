use super::*;

#[test]
pub(super) fn additional_land_play_static_count_uses_token_words() {
    let tokens = lex_line(
        "You may play two additional lands on each of your turns.",
        0,
    )
    .expect("lexes");
    assert_eq!(
        semantic_grammar::parse_additional_land_play_count_tokens(&tokens),
        Some(2)
    );

    let non_match = lex_line("You may play an additional land this turn.", 0).expect("lexes");
    assert_eq!(
        semantic_grammar::parse_additional_land_play_count_tokens(&non_match),
        None
    );
}

#[cfg(any(test, feature = "test-support"))]
pub fn parse_keyword_line_with_full_tokens_for_test(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    full_parse_tokens: &[OwnedLexToken],
    kind: RewriteKeywordLineKind,
) -> Result<LineAst, CardTextError> {
    super::super::super::keyword_registry::parse_keyword_payload_for_kind(
        info,
        text,
        parse_tokens,
        full_parse_tokens,
        kind,
    )
}

#[test]
pub(super) fn graveyard_copy_cast_accepts_only_the_standard_copy_cast_reminder_suffix() {
    let full = lex_line(
        "Exile up to one target legendary or Rat card from your graveyard and copy it. You may cast the copy. (You still pay its costs. A copy of a permanent spell becomes a token.)",
        0,
    )
    .expect("standard copy-cast reminder should lex");
    let effects = exact_graveyard_card_copy_cast_sequence(&full).unwrap_or_else(|| {
        panic!(
            "the standard reminder suffix should preserve the typed copy-cast sequence: {:#?}",
            split_lexed_sentences(&full)
                .iter()
                .map(|sentence| crate::lexer::parser_token_word_refs(sentence))
                .collect::<Vec<_>>()
        )
    });
    let debug = format!("{effects:#?}");
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("as_copy: true"), "{debug}");
    assert!(
        debug.contains("copy_cast_reminder_surface: true"),
        "{debug}"
    );
    assert!(!debug.contains("CopySpell"), "{debug}");

    let unrelated = lex_line(
        "Exile up to one target legendary or Rat card from your graveyard and copy it. You may cast the copy. You gain 1 life.",
        0,
    )
    .expect("near-miss copy-cast suffix should lex");
    assert!(exact_graveyard_card_copy_cast_sequence(&unrelated).is_none());
}

#[test]
pub(super) fn graveyard_copy_cast_accepts_conditional_copy_and_one_cast_result_tail() {
    let conditional = lex_line(
        "Exile up to one target Assassin card or card with freerunning from your graveyard. If you do, copy it. You may cast the copy.",
        0,
    )
    .expect("conditional copy-cast sequence should lex");
    let conditional_effects = exact_graveyard_card_copy_cast_sequence(&conditional)
        .expect("the registered conditional copy-cast family should stay typed");
    let conditional_debug = format!("{conditional_effects:#?}");
    assert!(
        conditional_debug.contains("CastTagged"),
        "{conditional_debug}"
    );
    assert!(
        conditional_debug.contains("as_copy: true"),
        "{conditional_debug}"
    );
    assert!(
        conditional_debug.contains("IfResult"),
        "{conditional_debug}"
    );
    assert!(
        !conditional_debug.contains("CopySpell"),
        "{conditional_debug}"
    );

    let with_cast_result = lex_line(
        "Exile up to one target black card from your graveyard and copy it. You may cast the copy. If you do, you lose 2 life.",
        0,
    )
    .expect("copy-cast result sequence should lex");
    let result_effects = exact_graveyard_card_copy_cast_sequence(&with_cast_result)
        .expect("one exact cast-result tail should follow the typed copy-cast prefix");
    let result_debug = format!("{result_effects:#?}");
    assert!(result_debug.contains("CastTagged"), "{result_debug}");
    assert!(result_debug.contains("LoseLife"), "{result_debug}");
    assert!(result_debug.contains("IfResult"), "{result_debug}");
    assert!(!result_debug.contains("CopySpell"), "{result_debug}");

    let wrong_result = lex_line(
        "Exile up to one target black card from your graveyard and copy it. You may cast the copy. If you don't, you lose 2 life.",
        0,
    )
    .expect("wrong-result near miss should lex");
    assert!(exact_graveyard_card_copy_cast_sequence(&wrong_result).is_none());

    let unrelated_tail = lex_line(
        "Exile up to one target black card from your graveyard and copy it. You may cast the copy. You gain 2 life.",
        0,
    )
    .expect("unrelated-tail near miss should lex");
    assert!(exact_graveyard_card_copy_cast_sequence(&unrelated_tail).is_none());
}

pub(super) fn rewrite_copy_count_to_times_paid_label_rewrite(
    effects: &mut [EffectAst],
    label: &str,
) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell { target, count, .. },
            ..
        }) = effect
            && let crate::cards::builders::TargetAst::Source(_) = target
            && let crate::effect::Value::Count(filter) = count
            && filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            })
        {
            *count = crate::effect::Value::TimesPaidLabel(label.into());
        }
        // Recurse into every nested-effect scope through the shared traversal
        // helper so new wrapper variants are covered automatically (the previous
        // hand-rolled match silently skipped RepeatEffects/ManaRestricted and the
        // newer ChooseOneOf/IfEffectDidNotHappen/TagAffected variants).
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            rewrite_copy_count_to_times_paid_label_rewrite(nested, label)
        });
    }
}

pub(super) fn standard_gift_create_token_effect(
    name: &str,
    definition: crate::model::token_definition::TokenDefinitionSpec,
    tapped: bool,
) -> EffectAst {
    EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Chosen,
        SubjectVerbActionAst::CreateTokenWithMods {
            name: name.to_string(),
            definition,
            count: crate::effect::Value::Fixed(1),
            dynamic_power_toughness: None,
            player: PlayerAst::Chosen,
            actor_surface_explicit: false,
            attached_to: None,
            tapped,
            attacking: false,
            attack_target_player: None,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            next_end_step_player: PlayerFilter::Any,
            granted_abilities: Vec::new(),
            ability_presentation: None,
        },
    )
}

pub(super) fn try_lower_hideaway_tokens(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let Some(shape) = semantic_grammar::parse_hideaway_keyword_tokens(parse_tokens)? else {
        return Ok(None);
    };
    Ok(Some(hideaway_line_ast(shape.count)))
}

#[test]
pub(super) fn hideaway_special_case_uses_parse_tokens() {
    let tokens = lex_line("Hideaway 5.", 0).expect("hideaway should lex");
    assert!(
        try_lower_hideaway_tokens(&tokens)
            .expect("hideaway should lower")
            .is_some()
    );

    let non_numeric = lex_line("Hideaway X.", 0).expect("hideaway should lex");
    assert!(try_lower_hideaway_tokens(&non_numeric).is_err());

    let reminder = lex_line("Hideaway 5 reminder", 0).expect("hideaway should lex");
    assert!(
        try_lower_hideaway_tokens(&reminder)
            .expect("extra words should not match the closed-form special case")
            .is_none()
    );
}

pub(super) fn try_lower_partner_with_tokens(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let Some(partner_name) = partner_with_name_from_tokens(parse_tokens) else {
        return Ok(None);
    };

    let mut filter = ObjectFilter::default();
    filter.name = Some(partner_name.clone());

    Ok(Some(LineAst::Multiple(vec![
        LineAst::StaticAbility(StaticAbility::partner_with(partner_name.clone()).into()),
        LineAst::Triggered {
            trigger: TriggerSpec::ThisEntersBattlefield {
                origin_condition: None,
            },
            effects: vec![EffectAst::MayByPlayer {
                player: PlayerAst::Target,
                effects: vec![EffectAst::subject_verb_search_library(
                    filter,
                    Zone::Hand,
                    PlayerAst::Target,
                    PlayerAst::Target,
                    crate::effect::SearchSelectionMode::Exact,
                    true,
                    Some(crate::effect::SearchResultReferenceSurface::ThatCard),
                    true,
                    ChoiceCount::up_to(1),
                    None,
                    None,
                    crate::effect::SearchResultReferenceSurface::ThatCard,
                    false,
                    false,
                    false,
                )],
            }],
            max_triggers_per_turn: None,
        },
    ])))
}

pub(super) fn partner_with_name_from_tokens(tokens: &[OwnedLexToken]) -> Option<String> {
    keyword_special_grammar::parse_partner_with_name_tokens(tokens)
}

#[test]
pub(super) fn partner_name_and_visible_label_trim_on_lexed_reminder_tokens() {
    let partner_with_tokens = lex_line(
        "Partner with Toothy, Imaginary Friend (When this creature enters...)",
        0,
    )
    .expect("partner-with line should lex");
    assert_eq!(
        partner_with_name_from_tokens(&partner_with_tokens).as_deref(),
        Some("Toothy, Imaginary Friend")
    );

    let partner_label_tokens = lex_line(
        "Partner - Friends forever (You can have two commanders.)",
        0,
    )
    .expect("partner label should lex");
    assert_eq!(
        keyword_special_grammar::parse_partner_visible_label_tokens(&partner_label_tokens)
            .as_deref(),
        Some("Partner - Friends forever")
    );
}
